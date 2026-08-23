//! Deterministic endpoint validation, route selection, and provider protocol encoding.

use std::collections::BTreeSet;

use agentmage_kernel_contracts::{
    CONTRACT_SCHEMA_VERSION, EndpointClass, EndpointProtocol, ModelEndpointProfile, ModelProfileId,
    ModelRouteDecision, RouteDecisionId, SessionId,
};
use serde::Serialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

const ZERO_SHA256: &str = "0000000000000000000000000000000000000000000000000000000000000000";
const MAX_ENDPOINT_TEXT_BYTES: usize = 512;
const MAX_ENDPOINT_HOSTS: usize = 16;
const MAX_MODEL_REQUEST_BYTES: usize = 4 * 1024 * 1024;

/// Stable gateway-policy or protocol refusal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ModelGatewayError {
    /// The endpoint or route contract is malformed.
    InvalidProfile,
    /// The profile has no current qualification evidence.
    NotQualified,
    /// The endpoint class exceeds the requested disclosure ceiling.
    DisclosureDenied,
    /// Fallback is absent, implicit, or violates route policy.
    FallbackDenied,
    /// The endpoint protocol is unsupported by the selected codec.
    ProtocolUnsupported,
    /// The encoded request exceeded its bound or was malformed.
    RequestInvalid,
    /// The endpoint response violated the closed protocol boundary.
    ResponseInvalid,
}

impl ModelGatewayError {
    /// Returns one stable redacted error code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidProfile => "model-gateway.profile.invalid",
            Self::NotQualified => "model-gateway.profile.not-qualified",
            Self::DisclosureDenied => "model-gateway.disclosure.denied",
            Self::FallbackDenied => "model-gateway.fallback.denied",
            Self::ProtocolUnsupported => "model-gateway.protocol.unsupported",
            Self::RequestInvalid => "model-gateway.request.invalid",
            Self::ResponseInvalid => "model-gateway.response.invalid",
        }
    }
}

/// Exact bounded canonical request presented to a protocol codec.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CanonicalGatewayRequest {
    /// Exact selected model ID and revision understood by the endpoint.
    pub model_id: String,
    /// Exact already-admitted UTF-8 context bytes.
    pub context: Vec<u8>,
    /// Maximum output tokens.
    pub max_output_tokens: u64,
    /// Whether ordered streaming is required.
    pub stream: bool,
}

/// Encoded provider request without a credential or destination capability.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EncodedGatewayRequest {
    /// Provider-relative path.
    pub path: String,
    /// Stable content type.
    pub content_type: String,
    /// Exact bounded request body.
    pub body: Vec<u8>,
    /// Digest of the exact request body.
    pub body_sha256: String,
}

/// Versioned protocol codec that has no transport or credential authority.
pub trait ModelGatewayCodec {
    /// Returns the exact protocol family accepted by this codec.
    fn protocol(&self) -> EndpointProtocol;

    /// Encodes one already-admitted canonical request.
    fn encode(
        &self,
        request: &CanonicalGatewayRequest,
    ) -> Result<EncodedGatewayRequest, ModelGatewayError>;

    /// Extracts ordered text from one complete protocol response fixture.
    fn decode_text(&self, response: &[u8]) -> Result<String, ModelGatewayError>;
}

/// Stateless codec covering the declared JSON protocol families.
#[derive(Clone, Copy, Debug)]
pub struct JsonGatewayCodec {
    protocol: EndpointProtocol,
}

impl JsonGatewayCodec {
    /// Creates a codec for one declared JSON protocol family.
    #[must_use]
    pub const fn new(protocol: EndpointProtocol) -> Self {
        Self { protocol }
    }
}

impl ModelGatewayCodec for JsonGatewayCodec {
    fn protocol(&self) -> EndpointProtocol {
        self.protocol
    }

    fn encode(
        &self,
        request: &CanonicalGatewayRequest,
    ) -> Result<EncodedGatewayRequest, ModelGatewayError> {
        validate_request(request)?;
        let prompt =
            std::str::from_utf8(&request.context).map_err(|_| ModelGatewayError::RequestInvalid)?;
        let (path, body) = match self.protocol {
            EndpointProtocol::Ollama => (
                "/api/generate",
                json!({
                    "model": request.model_id,
                    "prompt": prompt,
                    "stream": request.stream,
                    "options": { "num_predict": request.max_output_tokens }
                }),
            ),
            EndpointProtocol::OpenAiChat
            | EndpointProtocol::Tgi
            | EndpointProtocol::Sglang
            | EndpointProtocol::RayServe
            | EndpointProtocol::Kserve => (
                "/v1/chat/completions",
                json!({
                    "model": request.model_id,
                    "messages": [{"role": "user", "content": prompt}],
                    "stream": request.stream,
                    "max_tokens": request.max_output_tokens
                }),
            ),
            EndpointProtocol::OpenAiResponses => (
                "/v1/responses",
                json!({
                    "model": request.model_id,
                    "input": prompt,
                    "stream": request.stream,
                    "max_output_tokens": request.max_output_tokens
                }),
            ),
            EndpointProtocol::AnthropicMessages => (
                "/v1/messages",
                json!({
                    "model": request.model_id,
                    "messages": [{"role": "user", "content": prompt}],
                    "stream": request.stream,
                    "max_tokens": request.max_output_tokens
                }),
            ),
            EndpointProtocol::LlamaCppNative | EndpointProtocol::DockerModelRunner => {
                return Err(ModelGatewayError::ProtocolUnsupported);
            }
        };
        let body = serde_json::to_vec(&body).map_err(|_| ModelGatewayError::RequestInvalid)?;
        if body.len() > MAX_MODEL_REQUEST_BYTES {
            return Err(ModelGatewayError::RequestInvalid);
        }
        Ok(EncodedGatewayRequest {
            path: path.to_owned(),
            content_type: "application/json".to_owned(),
            body_sha256: sha256(&body),
            body,
        })
    }

    fn decode_text(&self, response: &[u8]) -> Result<String, ModelGatewayError> {
        if response.is_empty() || response.len() > MAX_MODEL_REQUEST_BYTES {
            return Err(ModelGatewayError::ResponseInvalid);
        }
        let value: Value =
            serde_json::from_slice(response).map_err(|_| ModelGatewayError::ResponseInvalid)?;
        let text = match self.protocol {
            EndpointProtocol::Ollama => value.get("response").and_then(Value::as_str),
            EndpointProtocol::OpenAiChat
            | EndpointProtocol::Tgi
            | EndpointProtocol::Sglang
            | EndpointProtocol::RayServe
            | EndpointProtocol::Kserve => value
                .get("choices")
                .and_then(|choices| choices.get(0))
                .and_then(|choice| choice.get("message"))
                .and_then(|message| message.get("content"))
                .and_then(Value::as_str),
            EndpointProtocol::OpenAiResponses => value
                .get("output_text")
                .and_then(Value::as_str)
                .or_else(|| {
                    value
                        .get("output")
                        .and_then(|output| output.get(0))
                        .and_then(|output| output.get("content"))
                        .and_then(|content| content.get(0))
                        .and_then(|content| content.get("text"))
                        .and_then(Value::as_str)
                }),
            EndpointProtocol::AnthropicMessages => value
                .get("content")
                .and_then(|content| content.get(0))
                .and_then(|content| content.get("text"))
                .and_then(Value::as_str),
            EndpointProtocol::LlamaCppNative | EndpointProtocol::DockerModelRunner => None,
        }
        .ok_or(ModelGatewayError::ResponseInvalid)?;
        if text.contains('\0') || text.len() > MAX_MODEL_REQUEST_BYTES {
            return Err(ModelGatewayError::ResponseInvalid);
        }
        Ok(text.to_owned())
    }
}

/// Seals one otherwise valid endpoint profile.
pub fn seal_endpoint_profile(
    mut profile: ModelEndpointProfile,
) -> Result<ModelEndpointProfile, ModelGatewayError> {
    profile.profile_sha256 = ZERO_SHA256.to_owned();
    validate_profile_fields(&profile)?;
    profile.profile_sha256 = canonical_sha256(&profile)?;
    Ok(profile)
}

/// Verifies one endpoint profile and its exact immutable digest.
pub fn verify_endpoint_profile(profile: &ModelEndpointProfile) -> Result<(), ModelGatewayError> {
    validate_profile_fields(profile)?;
    let mut candidate = profile.clone();
    candidate.profile_sha256 = ZERO_SHA256.to_owned();
    if profile.profile_sha256 != canonical_sha256(&candidate)? {
        return Err(ModelGatewayError::InvalidProfile);
    }
    Ok(())
}

/// Selects one explicit currently qualified route without considering fallback.
pub fn select_explicit_route(
    route_decision_id: RouteDecisionId,
    session_id: SessionId,
    model_profile_id: ModelProfileId,
    endpoint: &ModelEndpointProfile,
    maximum_class: EndpointClass,
    disclosure_accepted: bool,
    policy_sha256: String,
) -> Result<ModelRouteDecision, ModelGatewayError> {
    verify_endpoint_profile(endpoint)?;
    if !endpoint.qualified {
        return Err(ModelGatewayError::NotQualified);
    }
    if disclosure_rank(endpoint.class) > disclosure_rank(maximum_class) {
        return Err(ModelGatewayError::DisclosureDenied);
    }
    if endpoint.class != EndpointClass::StrictLocal && !disclosure_accepted {
        return Err(ModelGatewayError::DisclosureDenied);
    }
    if !valid_sha256(&policy_sha256) {
        return Err(ModelGatewayError::InvalidProfile);
    }
    seal_route(ModelRouteDecision {
        schema_version: CONTRACT_SCHEMA_VERSION,
        route_decision_id,
        session_id,
        model_profile_id,
        endpoint_profile_id: endpoint.endpoint_profile_id.clone(),
        class: endpoint.class,
        reason_code: "model-gateway.route.explicit-qualified-selection".to_owned(),
        fallback: false,
        prior_endpoint_profile_id: None,
        disclosure_accepted,
        policy_sha256,
        decision_sha256: ZERO_SHA256.to_owned(),
    })
}

/// Constructs an explicit fallback only when both routes permit the same disclosure boundary.
#[allow(clippy::too_many_arguments)]
pub fn select_explicit_fallback(
    route_decision_id: RouteDecisionId,
    session_id: SessionId,
    model_profile_id: ModelProfileId,
    prior: &ModelEndpointProfile,
    proposed: &ModelEndpointProfile,
    maximum_class: EndpointClass,
    disclosure_accepted: bool,
    fallback_policy_enabled: bool,
    policy_sha256: String,
) -> Result<ModelRouteDecision, ModelGatewayError> {
    verify_endpoint_profile(prior)?;
    verify_endpoint_profile(proposed)?;
    if !fallback_policy_enabled
        || prior.class == EndpointClass::StrictLocal
        || !proposed.qualified
        || prior.endpoint_profile_id == proposed.endpoint_profile_id
        || disclosure_rank(proposed.class) > disclosure_rank(maximum_class)
        || !disclosure_accepted
    {
        return Err(ModelGatewayError::FallbackDenied);
    }
    seal_route(ModelRouteDecision {
        schema_version: CONTRACT_SCHEMA_VERSION,
        route_decision_id,
        session_id,
        model_profile_id,
        endpoint_profile_id: proposed.endpoint_profile_id.clone(),
        class: proposed.class,
        reason_code: "model-gateway.route.explicit-policy-fallback".to_owned(),
        fallback: true,
        prior_endpoint_profile_id: Some(prior.endpoint_profile_id.clone()),
        disclosure_accepted,
        policy_sha256,
        decision_sha256: ZERO_SHA256.to_owned(),
    })
}

/// Verifies one exact route decision and forbids malformed fallback records.
pub fn verify_route_decision(decision: &ModelRouteDecision) -> Result<(), ModelGatewayError> {
    validate_route_fields(decision)?;
    let mut candidate = decision.clone();
    candidate.decision_sha256 = ZERO_SHA256.to_owned();
    if decision.decision_sha256 != canonical_sha256(&candidate)? {
        return Err(ModelGatewayError::InvalidProfile);
    }
    Ok(())
}

fn seal_route(mut decision: ModelRouteDecision) -> Result<ModelRouteDecision, ModelGatewayError> {
    validate_route_fields(&decision)?;
    decision.decision_sha256 = canonical_sha256(&decision)?;
    Ok(decision)
}

fn validate_profile_fields(profile: &ModelEndpointProfile) -> Result<(), ModelGatewayError> {
    if profile.schema_version != CONTRACT_SCHEMA_VERSION
        || profile.endpoint_profile_id.as_str().is_empty()
        || !valid_text(&profile.operator)
        || !valid_text(&profile.deployment_owner)
        || !valid_text(&profile.model_id)
        || !valid_text(&profile.region)
        || !valid_text(&profile.logging_policy)
        || !valid_text(&profile.retention_policy)
        || !valid_text(&profile.training_use_policy)
        || profile.allowed_hosts.is_empty()
        || profile.allowed_hosts.len() > MAX_ENDPOINT_HOSTS
        || profile.max_context_tokens == 0
        || profile.max_output_tokens == 0
        || !valid_sha256(&profile.qualification_sha256)
        || !valid_sha256(&profile.profile_sha256)
    {
        return Err(ModelGatewayError::InvalidProfile);
    }
    let unique = profile.allowed_hosts.iter().collect::<BTreeSet<_>>();
    if unique.len() != profile.allowed_hosts.len()
        || profile.allowed_hosts.iter().any(|host| !valid_host(host))
    {
        return Err(ModelGatewayError::InvalidProfile);
    }
    let endpoint = ParsedEndpoint::parse(&profile.base_url)?;
    if !profile
        .allowed_hosts
        .iter()
        .any(|host| host == endpoint.host)
        || endpoint.has_userinfo
        || endpoint.has_query_or_fragment
    {
        return Err(ModelGatewayError::InvalidProfile);
    }
    match profile.class {
        EndpointClass::StrictLocal => {
            if profile.credential_reference.is_some()
                || !matches!(endpoint.host, "127.0.0.1" | "localhost" | "::1")
                || endpoint.scheme != "http"
                || !profile.redirects_denied
                || !profile.ambient_proxies_denied
            {
                return Err(ModelGatewayError::InvalidProfile);
            }
        }
        EndpointClass::LocalNetworkPrivate => {
            if !matches!(endpoint.scheme, "http" | "https")
                || !profile.redirects_denied
                || !profile.ambient_proxies_denied
            {
                return Err(ModelGatewayError::InvalidProfile);
            }
        }
        EndpointClass::RemotePrivate | EndpointClass::RemoteManaged => {
            if endpoint.scheme != "https"
                || profile
                    .credential_reference
                    .as_deref()
                    .is_none_or(|value| !valid_text(value))
                || !profile.redirects_denied
                || !profile.ambient_proxies_denied
                || profile.region == "local"
            {
                return Err(ModelGatewayError::InvalidProfile);
            }
        }
    }
    Ok(())
}

fn validate_route_fields(decision: &ModelRouteDecision) -> Result<(), ModelGatewayError> {
    if decision.schema_version != CONTRACT_SCHEMA_VERSION
        || decision.route_decision_id.as_str().is_empty()
        || decision.session_id.as_str().is_empty()
        || decision.model_profile_id.as_str().is_empty()
        || decision.endpoint_profile_id.as_str().is_empty()
        || !valid_text(&decision.reason_code)
        || !valid_sha256(&decision.policy_sha256)
        || !valid_sha256(&decision.decision_sha256)
        || decision.fallback != decision.prior_endpoint_profile_id.is_some()
        || (decision.class != EndpointClass::StrictLocal && !decision.disclosure_accepted)
    {
        return Err(ModelGatewayError::InvalidProfile);
    }
    Ok(())
}

fn validate_request(request: &CanonicalGatewayRequest) -> Result<(), ModelGatewayError> {
    if !valid_text(&request.model_id)
        || request.context.is_empty()
        || request.context.len() > MAX_MODEL_REQUEST_BYTES
        || request.max_output_tokens == 0
    {
        return Err(ModelGatewayError::RequestInvalid);
    }
    Ok(())
}

struct ParsedEndpoint<'a> {
    scheme: &'a str,
    host: &'a str,
    has_userinfo: bool,
    has_query_or_fragment: bool,
}

impl<'a> ParsedEndpoint<'a> {
    fn parse(value: &'a str) -> Result<Self, ModelGatewayError> {
        if value.is_empty() || value.len() > MAX_ENDPOINT_TEXT_BYTES || value.contains('\0') {
            return Err(ModelGatewayError::InvalidProfile);
        }
        let (scheme, remainder) = value
            .split_once("://")
            .ok_or(ModelGatewayError::InvalidProfile)?;
        let authority = remainder.split('/').next().unwrap_or(remainder);
        let has_userinfo = authority.contains('@');
        let host_port = authority.rsplit('@').next().unwrap_or(authority);
        let host = if let Some(stripped) = host_port.strip_prefix('[') {
            stripped
                .split_once(']')
                .map(|(host, _)| host)
                .ok_or(ModelGatewayError::InvalidProfile)?
        } else {
            host_port.split(':').next().unwrap_or(host_port)
        };
        if host.is_empty() || !valid_host(host) {
            return Err(ModelGatewayError::InvalidProfile);
        }
        Ok(Self {
            scheme,
            host,
            has_userinfo,
            has_query_or_fragment: value.contains('?') || value.contains('#'),
        })
    }
}

const fn disclosure_rank(class: EndpointClass) -> u8 {
    match class {
        EndpointClass::StrictLocal => 0,
        EndpointClass::LocalNetworkPrivate => 1,
        EndpointClass::RemotePrivate => 2,
        EndpointClass::RemoteManaged => 3,
    }
}

fn valid_host(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 253
        && !value.contains('\0')
        && !value.contains('/')
        && !value.contains('\\')
        && !value.contains('@')
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b':'))
}

fn valid_text(value: &str) -> bool {
    !value.is_empty() && value.len() <= MAX_ENDPOINT_TEXT_BYTES && !value.contains('\0')
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn canonical_sha256<T: Serialize>(value: &T) -> Result<String, ModelGatewayError> {
    serde_json::to_vec(value)
        .map(|bytes| sha256(&bytes))
        .map_err(|_| ModelGatewayError::InvalidProfile)
}

fn sha256(bytes: &[u8]) -> String {
    let mut digest = Sha256::new();
    digest.update(bytes);
    let digest = digest.finalize();
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(64);
    for byte in digest {
        output.push(char::from(HEX[usize::from(byte >> 4)]));
        output.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    output
}

#[cfg(test)]
mod tests {
    use super::{
        CanonicalGatewayRequest, JsonGatewayCodec, ModelGatewayCodec, ModelGatewayError,
        seal_endpoint_profile, select_explicit_fallback, select_explicit_route,
        verify_endpoint_profile, verify_route_decision,
    };
    use agentmage_kernel_contracts::{
        CONTRACT_SCHEMA_VERSION, EndpointClass, EndpointProfileId, EndpointProtocol,
        ModelEndpointProfile, ModelProfileId, RouteDecisionId, SessionId,
    };

    fn endpoint(id: &str, class: EndpointClass) -> ModelEndpointProfile {
        let (url, host, credential, region) = match class {
            EndpointClass::StrictLocal => ("http://127.0.0.1:11434", "127.0.0.1", None, "local"),
            EndpointClass::LocalNetworkPrivate => ("https://model.lan", "model.lan", None, "local"),
            EndpointClass::RemotePrivate => (
                "https://private.example.test",
                "private.example.test",
                Some("credential://model/private"),
                "us-central",
            ),
            EndpointClass::RemoteManaged => (
                "https://managed.example.test",
                "managed.example.test",
                Some("credential://model/managed"),
                "us-central",
            ),
        };
        seal_endpoint_profile(ModelEndpointProfile {
            schema_version: CONTRACT_SCHEMA_VERSION,
            endpoint_profile_id: EndpointProfileId::from_raw(id),
            class,
            protocol: EndpointProtocol::OpenAiChat,
            operator: "fixture-operator".to_owned(),
            deployment_owner: "fixture-owner".to_owned(),
            base_url: url.to_owned(),
            allowed_hosts: vec![host.to_owned()],
            redirects_denied: true,
            ambient_proxies_denied: true,
            model_id: "fixture-model@revision".to_owned(),
            credential_reference: credential.map(str::to_owned),
            region: region.to_owned(),
            logging_policy: "no-prompt-logging".to_owned(),
            retention_policy: "zero-retention".to_owned(),
            training_use_policy: "prohibited".to_owned(),
            max_context_tokens: 8192,
            max_output_tokens: 1024,
            qualification_sha256: "a".repeat(64),
            qualified: true,
            profile_sha256: "0".repeat(64),
        })
        .unwrap()
    }

    #[test]
    fn exact_profiles_and_explicit_routes_are_digest_bound() {
        let profile = endpoint("endpoint-local", EndpointClass::StrictLocal);
        verify_endpoint_profile(&profile).unwrap();
        let route = select_explicit_route(
            RouteDecisionId::from_raw("route-local"),
            SessionId::from_raw("session-local"),
            ModelProfileId::from_raw("model-local"),
            &profile,
            EndpointClass::StrictLocal,
            false,
            "b".repeat(64),
        )
        .unwrap();
        verify_route_decision(&route).unwrap();
        assert!(!route.fallback);
    }

    #[test]
    fn strict_local_can_never_fall_back_remotely() {
        let local = endpoint("endpoint-local", EndpointClass::StrictLocal);
        let remote = endpoint("endpoint-remote", EndpointClass::RemotePrivate);
        assert_eq!(
            select_explicit_fallback(
                RouteDecisionId::from_raw("route-fallback"),
                SessionId::from_raw("session-fallback"),
                ModelProfileId::from_raw("model-fallback"),
                &local,
                &remote,
                EndpointClass::RemotePrivate,
                true,
                true,
                "c".repeat(64),
            ),
            Err(ModelGatewayError::FallbackDenied)
        );
    }

    #[test]
    fn every_json_protocol_uses_a_closed_shape_and_extracts_text() {
        let request = CanonicalGatewayRequest {
            model_id: "model@revision".to_owned(),
            context: b"exact context".to_vec(),
            max_output_tokens: 128,
            stream: false,
        };
        let fixtures = [
            (
                EndpointProtocol::Ollama,
                br#"{"response":"ollama"}"#.as_slice(),
                "ollama",
            ),
            (
                EndpointProtocol::OpenAiChat,
                br#"{"choices":[{"message":{"content":"chat"}}]}"#.as_slice(),
                "chat",
            ),
            (
                EndpointProtocol::OpenAiResponses,
                br#"{"output_text":"responses"}"#.as_slice(),
                "responses",
            ),
            (
                EndpointProtocol::AnthropicMessages,
                br#"{"content":[{"text":"anthropic"}]}"#.as_slice(),
                "anthropic",
            ),
        ];
        for (protocol, response, expected) in fixtures {
            let codec = JsonGatewayCodec::new(protocol);
            let encoded = codec.encode(&request).unwrap();
            assert!(!encoded.body.is_empty());
            assert_eq!(codec.decode_text(response).unwrap(), expected);
        }
    }

    #[test]
    fn remote_profile_rejects_http_embedded_credentials_and_redirects() {
        let mut profile = endpoint("endpoint-hostile", EndpointClass::RemoteManaged);
        profile.base_url = "http://secret@managed.example.test".to_owned();
        profile.redirects_denied = false;
        profile.profile_sha256 = "0".repeat(64);
        assert_eq!(
            seal_endpoint_profile(profile),
            Err(ModelGatewayError::InvalidProfile)
        );
    }
}
