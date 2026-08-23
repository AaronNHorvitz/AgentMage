//! Exact qualified HTTP model-gateway bridge for Verified Chat.

use agentmage_kernel_contracts::{
    EndpointClass, EndpointProfileId, EndpointProtocol, ModelEndpointProfile, ModelProfileId,
    ModelRouteDecision, RouteDecisionId,
};
use agentmage_kernel_engine::model_gateway::{
    CanonicalGatewayRequest, EncodedGatewayRequest, JsonGatewayCodec, ModelGatewayCodec,
    verify_endpoint_profile, verify_route_decision,
};

use crate::engineering_runtime::{
    EngineeringModelError, EngineeringModelInput, EngineeringModelPort,
};

/// Transport-authored observation returned without exposing raw credentials.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GatewayTransportReceipt {
    /// Exact endpoint profile used by the transport.
    pub endpoint_profile_id: EndpointProfileId,
    /// Exact protocol used on the wire.
    pub protocol: EndpointProtocol,
    /// Exact connected host after resolution.
    pub connected_host: String,
    /// Exact model identity returned by the endpoint.
    pub model_id: String,
    /// Digest of the exact encoded request body sent.
    pub request_body_sha256: String,
    /// Number of redirects followed; must remain zero.
    pub redirect_count: u8,
    /// Whether any ambient proxy affected transport.
    pub ambient_proxy_used: bool,
    /// TLS peer-certificate digest for non-local HTTPS routes.
    pub tls_peer_sha256: Option<String>,
    /// Exact bounded complete response bytes.
    pub response_bytes: Vec<u8>,
}

/// Credential-owning transport boundary; raw credential values never cross this port.
pub trait QualifiedGatewayTransportPort {
    /// Executes one exact already-encoded request against one verified endpoint profile.
    fn execute(
        &mut self,
        profile: &ModelEndpointProfile,
        request: &EncodedGatewayRequest,
    ) -> Result<GatewayTransportReceipt, EngineeringModelError>;
}

/// One explicit qualified endpoint and route exposed through the narrow Engineering model port.
pub struct QualifiedGatewayEngineeringModel<T> {
    model_profile_id: ModelProfileId,
    endpoint: ModelEndpointProfile,
    route: ModelRouteDecision,
    codec: JsonGatewayCodec,
    transport: T,
}

impl<T: QualifiedGatewayTransportPort> QualifiedGatewayEngineeringModel<T> {
    /// Binds one model, one endpoint, one route, and one credential-owning transport.
    pub fn new(
        model_profile_id: ModelProfileId,
        endpoint: ModelEndpointProfile,
        route: ModelRouteDecision,
        transport: T,
    ) -> Result<Self, EngineeringModelError> {
        verify_endpoint_profile(&endpoint).map_err(|_| EngineeringModelError::Failed)?;
        verify_route_decision(&route).map_err(|_| EngineeringModelError::Failed)?;
        if !endpoint.qualified
            || model_profile_id != route.model_profile_id
            || endpoint.endpoint_profile_id != route.endpoint_profile_id
            || endpoint.class != route.class
            || route.fallback
            || matches!(
                endpoint.protocol,
                EndpointProtocol::LlamaCppNative | EndpointProtocol::DockerModelRunner
            )
        {
            return Err(EngineeringModelError::Failed);
        }
        let codec = JsonGatewayCodec::new(endpoint.protocol);
        Ok(Self {
            model_profile_id,
            endpoint,
            route,
            codec,
            transport,
        })
    }
}

impl<T: QualifiedGatewayTransportPort> EngineeringModelPort
    for QualifiedGatewayEngineeringModel<T>
{
    fn model_profile_id(&self) -> ModelProfileId {
        self.model_profile_id.clone()
    }

    fn endpoint_profile_id(&self) -> EndpointProfileId {
        self.endpoint.endpoint_profile_id.clone()
    }

    fn route_decision_id(&self) -> RouteDecisionId {
        self.route.route_decision_id.clone()
    }

    fn execute(&mut self, input: &EngineeringModelInput) -> Result<String, EngineeringModelError> {
        if input.context.model_profile_id != self.model_profile_id
            || input.context.endpoint_profile_id != self.endpoint.endpoint_profile_id
            || input.context.route_decision_id != self.route.route_decision_id
        {
            return Err(EngineeringModelError::Failed);
        }
        let request = self
            .codec
            .encode(&CanonicalGatewayRequest {
                model_id: self.endpoint.model_id.clone(),
                context: input.context_bytes.clone(),
                max_output_tokens: self.endpoint.max_output_tokens,
                stream: false,
            })
            .map_err(|_| EngineeringModelError::Failed)?;
        let receipt = self.transport.execute(&self.endpoint, &request)?;
        if receipt.endpoint_profile_id != self.endpoint.endpoint_profile_id
            || receipt.protocol != self.endpoint.protocol
            || !self
                .endpoint
                .allowed_hosts
                .iter()
                .any(|host| host == &receipt.connected_host)
            || receipt.model_id != self.endpoint.model_id
            || receipt.request_body_sha256 != request.body_sha256
            || receipt.redirect_count != 0
            || receipt.ambient_proxy_used
            || (self.endpoint.class != EndpointClass::StrictLocal
                && receipt
                    .tls_peer_sha256
                    .as_deref()
                    .is_none_or(|digest| !valid_sha256(digest)))
            || (self.endpoint.class == EndpointClass::StrictLocal
                && receipt.tls_peer_sha256.is_some())
        {
            return Err(EngineeringModelError::Failed);
        }
        self.codec
            .decode_text(&receipt.response_bytes)
            .map_err(|_| EngineeringModelError::Failed)
    }
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}

#[cfg(test)]
mod tests {
    use agentmage_kernel_contracts::{
        CONTRACT_SCHEMA_VERSION, ContextDeliveryReceipt, ContextPacketId, EndpointClass,
        EndpointProfileId, EndpointProtocol, ModelEndpointProfile, ModelProfileId, RouteDecisionId,
        RuntimeRunId, SessionId,
    };
    use agentmage_kernel_engine::model_gateway::{seal_endpoint_profile, select_explicit_route};

    use super::{
        GatewayTransportReceipt, QualifiedGatewayEngineeringModel, QualifiedGatewayTransportPort,
    };
    use crate::engineering_runtime::{
        EngineeringModelError, EngineeringModelInput, EngineeringModelPort,
    };

    #[derive(Clone, Copy)]
    enum TransportMutation {
        None,
        Endpoint,
        Protocol,
        Host,
        Model,
        Request,
        Redirect,
        Proxy,
        MissingTlsPeer,
        InvalidTlsPeer,
    }

    #[derive(Clone)]
    struct FixtureTransport {
        mutation: TransportMutation,
    }

    impl QualifiedGatewayTransportPort for FixtureTransport {
        fn execute(
            &mut self,
            profile: &ModelEndpointProfile,
            request: &agentmage_kernel_engine::model_gateway::EncodedGatewayRequest,
        ) -> Result<GatewayTransportReceipt, EngineeringModelError> {
            Ok(GatewayTransportReceipt {
                endpoint_profile_id: if matches!(self.mutation, TransportMutation::Endpoint) {
                    EndpointProfileId::from_raw("endpoint-substituted")
                } else {
                    profile.endpoint_profile_id.clone()
                },
                protocol: if matches!(self.mutation, TransportMutation::Protocol) {
                    EndpointProtocol::AnthropicMessages
                } else {
                    profile.protocol
                },
                connected_host: if matches!(self.mutation, TransportMutation::Host) {
                    "substituted.example.test".to_owned()
                } else {
                    profile.allowed_hosts[0].clone()
                },
                model_id: if matches!(self.mutation, TransportMutation::Model) {
                    "substituted-model@revision".to_owned()
                } else {
                    profile.model_id.clone()
                },
                request_body_sha256: if matches!(self.mutation, TransportMutation::Request) {
                    "f".repeat(64)
                } else {
                    request.body_sha256.clone()
                },
                redirect_count: u8::from(matches!(self.mutation, TransportMutation::Redirect)),
                ambient_proxy_used: matches!(self.mutation, TransportMutation::Proxy),
                tls_peer_sha256: match self.mutation {
                    TransportMutation::MissingTlsPeer => None,
                    TransportMutation::InvalidTlsPeer => Some("invalid".to_owned()),
                    _ => Some("d".repeat(64)),
                },
                response_bytes: br#"{"choices":[{"message":{"content":"verified"}}]}"#.to_vec(),
            })
        }
    }

    fn endpoint() -> ModelEndpointProfile {
        seal_endpoint_profile(ModelEndpointProfile {
            schema_version: CONTRACT_SCHEMA_VERSION,
            endpoint_profile_id: EndpointProfileId::from_raw("endpoint-private-fixture"),
            class: EndpointClass::RemotePrivate,
            protocol: EndpointProtocol::OpenAiChat,
            operator: "fixture-operator".to_owned(),
            deployment_owner: "fixture-owner".to_owned(),
            base_url: "https://private.example.test".to_owned(),
            allowed_hosts: vec!["private.example.test".to_owned()],
            redirects_denied: true,
            ambient_proxies_denied: true,
            model_id: "fixture-model@revision".to_owned(),
            credential_reference: Some("credential://fixture/private".to_owned()),
            region: "us-central".to_owned(),
            logging_policy: "no-prompt-logging".to_owned(),
            retention_policy: "zero-retention".to_owned(),
            training_use_policy: "prohibited".to_owned(),
            max_context_tokens: 8_192,
            max_output_tokens: 256,
            qualification_sha256: "a".repeat(64),
            qualified: true,
            profile_sha256: "0".repeat(64),
        })
        .unwrap()
    }

    fn input(model: &ModelProfileId, endpoint: &ModelEndpointProfile) -> EngineeringModelInput {
        let route = RouteDecisionId::from_raw("route-private-fixture");
        EngineeringModelInput {
            session_id: SessionId::from_raw("session-private-fixture"),
            mode: agentmage_kernel_contracts::EngineeringSessionMode::Ask,
            context: ContextDeliveryReceipt {
                schema_version: CONTRACT_SCHEMA_VERSION,
                context_packet_id: ContextPacketId::from_raw("context-private-fixture"),
                run_id: RuntimeRunId::from_raw("run-private-fixture"),
                model_profile_id: model.clone(),
                endpoint_profile_id: endpoint.endpoint_profile_id.clone(),
                route_decision_id: route,
                artifacts: Vec::new(),
                inline_bytes: 5,
                estimated_tokens: 5,
                token_limit: 8_192,
                context_sha256: "b".repeat(64),
                receipt_sha256: "c".repeat(64),
            },
            context_bytes: b"hello".to_vec(),
        }
    }

    #[test]
    fn exact_remote_route_executes_and_every_transport_substitution_fails_closed() {
        let endpoint = endpoint();
        let model = ModelProfileId::from_raw("model-private-fixture");
        let route = select_explicit_route(
            RouteDecisionId::from_raw("route-private-fixture"),
            SessionId::from_raw("session-private-fixture"),
            model.clone(),
            &endpoint,
            EndpointClass::RemotePrivate,
            true,
            "e".repeat(64),
        )
        .unwrap();
        let mut accepted = QualifiedGatewayEngineeringModel::new(
            model.clone(),
            endpoint.clone(),
            route.clone(),
            FixtureTransport {
                mutation: TransportMutation::None,
            },
        )
        .unwrap();
        assert_eq!(
            accepted.execute(&input(&model, &endpoint)).unwrap(),
            "verified"
        );

        for mutation in [
            TransportMutation::Endpoint,
            TransportMutation::Protocol,
            TransportMutation::Host,
            TransportMutation::Model,
            TransportMutation::Request,
            TransportMutation::Redirect,
            TransportMutation::Proxy,
            TransportMutation::MissingTlsPeer,
            TransportMutation::InvalidTlsPeer,
        ] {
            let mut substituted = QualifiedGatewayEngineeringModel::new(
                model.clone(),
                endpoint.clone(),
                route.clone(),
                FixtureTransport { mutation },
            )
            .unwrap();
            assert_eq!(
                substituted.execute(&input(&model, &endpoint)),
                Err(EngineeringModelError::Failed)
            );
        }
    }
}
