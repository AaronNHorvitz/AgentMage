//! Qualified local-model bridge for Verified Chat.

use agentmage_kernel_contracts::{
    CONTRACT_SCHEMA_VERSION, ContextPacketId, ContractPayload, CorrelationId, EndpointProfileId,
    LocalModelRuntime, ModelContextPacket, ModelFamilyCodec, ModelMessage, ModelMessageId,
    ModelMessageRole, ModelProfileId, ModelProposalKind, ModelRunId, ModelRunRequest,
    ModelRunTerminalState, RouteDecisionId, SchemaId, SchemaReference, TaskId, ToolCatalogId,
    to_canonical_json,
};
use agentmage_kernel_engine::model_runtime::{
    LocalModelController, ModelRuntimeGateError, ModelUsePurpose,
};
use sha2::{Digest, Sha256};

use crate::engineering_runtime::{
    EngineeringModelError, EngineeringModelInput, EngineeringModelPort,
};

/// Qualified bridge construction failure without model or prompt content.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EngineeringModelBridgeError {
    /// The profile was not admitted for product use.
    NotProductQualified,
    /// Exact runtime verification or loading failed.
    LoadFailed(ModelRuntimeGateError),
    /// Endpoint or route identity was empty.
    InvalidRoute,
}

/// Loaded exact-profile model exposed through the narrow Verified Chat port.
pub struct QualifiedLocalEngineeringModel<R: LocalModelRuntime, C: ModelFamilyCodec> {
    controller: LocalModelController<R, C>,
    endpoint_profile_id: EndpointProfileId,
    route_decision_id: RouteDecisionId,
    next_run: u64,
}

impl<R: LocalModelRuntime, C: ModelFamilyCodec> QualifiedLocalEngineeringModel<R, C> {
    /// Verifies and loads one exact profile admitted by the kernel for product use.
    pub fn load(
        mut controller: LocalModelController<R, C>,
        endpoint_profile_id: EndpointProfileId,
        route_decision_id: RouteDecisionId,
    ) -> Result<Self, EngineeringModelBridgeError> {
        if controller.purpose() != ModelUsePurpose::Product {
            return Err(EngineeringModelBridgeError::NotProductQualified);
        }
        if endpoint_profile_id.as_str().is_empty() || route_decision_id.as_str().is_empty() {
            return Err(EngineeringModelBridgeError::InvalidRoute);
        }
        controller
            .load()
            .map_err(EngineeringModelBridgeError::LoadFailed)?;
        controller
            .health()
            .map_err(EngineeringModelBridgeError::LoadFailed)?;
        Ok(Self {
            controller,
            endpoint_profile_id,
            route_decision_id,
            next_run: 0,
        })
    }
}

impl<R: LocalModelRuntime, C: ModelFamilyCodec> EngineeringModelPort
    for QualifiedLocalEngineeringModel<R, C>
{
    fn model_profile_id(&self) -> ModelProfileId {
        self.controller.exact_profile().profile_id.clone()
    }

    fn endpoint_profile_id(&self) -> EndpointProfileId {
        self.endpoint_profile_id.clone()
    }

    fn route_decision_id(&self) -> RouteDecisionId {
        self.route_decision_id.clone()
    }

    fn execute(&mut self, input: &EngineeringModelInput) -> Result<String, EngineeringModelError> {
        self.next_run = self
            .next_run
            .checked_add(1)
            .ok_or(EngineeringModelError::Failed)?;
        let profile = self.controller.exact_profile().clone();
        let suffix = &sha256(
            format!(
                "{}:{}:{}",
                input.session_id.as_str(),
                input.context.context_packet_id.as_str(),
                self.next_run
            )
            .as_bytes(),
        )[..24];
        let payload = ContractPayload {
            schema: SchemaReference {
                schema_id: SchemaId::from_raw("engineering.verified-chat.user-text.v1"),
                schema_version: 1,
                schema_sha256: sha256(b"engineering.verified-chat.user-text.v1"),
            },
            media_type: "text/plain".to_owned(),
            sha256: sha256(&input.context_bytes),
            bytes: input.context_bytes.clone(),
        };
        let mut packet = ModelContextPacket {
            schema_version: CONTRACT_SCHEMA_VERSION,
            context_packet_id: ContextPacketId::from_raw(
                input.context.context_packet_id.as_str().to_owned(),
            ),
            session_id: input.session_id.clone(),
            task_id: TaskId::from_raw(format!("engineering-task-{suffix}")),
            profile_id: profile.profile_id.clone(),
            manifest_sha256: profile.manifest_sha256.clone(),
            tool_catalog_id: ToolCatalogId::from_raw("engineering-verified-chat-no-tools-v1"),
            messages: vec![ModelMessage {
                message_id: ModelMessageId::from_raw(format!("engineering-message-{suffix}")),
                role: ModelMessageRole::User,
                content: payload,
            }],
            input_bytes: input.context_bytes.len() as u64,
            input_tokens: 1,
            packet_sha256: "0".repeat(64),
        };
        packet.packet_sha256 = packet_digest(&packet)?;
        self.controller
            .bind_token_count(&mut packet)
            .map_err(|_| EngineeringModelError::Failed)?;
        let request = ModelRunRequest {
            schema_version: CONTRACT_SCHEMA_VERSION,
            model_run_id: ModelRunId::from_raw(format!("engineering-model-run-{suffix}")),
            correlation_id: CorrelationId::from_raw(format!("engineering-model-{suffix}")),
            context_packet_id: packet.context_packet_id.clone(),
            profile_id: profile.profile_id,
            manifest_sha256: profile.manifest_sha256,
            adapter_id: profile.runtime.adapter_id,
            decoding_profile_id: profile.decoding.profile_id,
            max_output_tokens: profile.decoding.max_output_tokens,
            timeout_ms: 120_000,
        };
        let output = self
            .controller
            .stream_with_output(&request, &packet, None)
            .map_err(|_| EngineeringModelError::Failed)?;
        match output.result.terminal_state {
            ModelRunTerminalState::AdvisoryText => {
                String::from_utf8(output.response_bytes).map_err(|_| EngineeringModelError::Failed)
            }
            ModelRunTerminalState::Proposed => {
                let proposal = output
                    .result
                    .proposal
                    .ok_or(EngineeringModelError::Failed)?;
                if proposal.kind != ModelProposalKind::Text || proposal.tool_call.is_some() {
                    return Err(EngineeringModelError::Failed);
                }
                let payload = proposal.payload.ok_or(EngineeringModelError::Failed)?;
                if payload.media_type != "text/plain" || payload.sha256 != sha256(&payload.bytes) {
                    return Err(EngineeringModelError::Failed);
                }
                String::from_utf8(payload.bytes).map_err(|_| EngineeringModelError::Failed)
            }
            _ => Err(EngineeringModelError::Failed),
        }
    }
}

fn packet_digest(packet: &ModelContextPacket) -> Result<String, EngineeringModelError> {
    let mut candidate = packet.clone();
    candidate.packet_sha256 = "0".repeat(64);
    let bytes = to_canonical_json(&candidate).map_err(|_| EngineeringModelError::Failed)?;
    Ok(sha256(&bytes))
}

fn sha256(bytes: &[u8]) -> String {
    let mut encoded = String::with_capacity(64);
    for byte in Sha256::digest(bytes) {
        use std::fmt::Write as _;
        let _ = write!(encoded, "{byte:02x}");
    }
    encoded
}

#[cfg(test)]
mod tests {
    use agentmage_kernel_contracts::ExactModelProfile;
    use agentmage_kernel_engine::model_runtime::{ModelAdmissionCatalog, ModelUsePurpose};

    #[test]
    fn current_local_profiles_cannot_cross_the_product_bridge() {
        let catalog: serde_json::Value = serde_json::from_str(include_str!(
            "../../../model-profiles/exact-profile-catalog.json"
        ))
        .expect("catalog JSON");
        let profiles: Vec<ExactModelProfile> = catalog["profiles"]
            .as_array()
            .expect("profiles")
            .iter()
            .cloned()
            .map(|value| serde_json::from_value(value).expect("exact profile"))
            .collect();
        let admission = ModelAdmissionCatalog::new(profiles.clone()).expect("valid catalog");
        assert!(
            profiles
                .iter()
                .all(|profile| { admission.admit(profile, ModelUsePurpose::Product).is_err() })
        );
    }
}
