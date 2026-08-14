//! Candidate-neutral family-codec boundary for exact context and proposal bytes.

use agentmage_kernel_contracts::{
    ClosedModelProposal, EncodedModelContext, ExactModelProfile, FamilyCodecIdentity,
    ModelContextPacket, ModelFamilyCodec, ModelRunRequest, ModelRuntimeFailure, from_json,
    to_canonical_json,
};
use sha2::{Digest, Sha256};

/// Closed JSON codec parameterized only by an exact family-codec identity.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClosedJsonFamilyCodec {
    identity: FamilyCodecIdentity,
}

impl ClosedJsonFamilyCodec {
    /// Creates a codec for one immutable tokenizer/template/protocol tuple.
    #[must_use]
    pub const fn new(identity: FamilyCodecIdentity) -> Self {
        Self { identity }
    }
}

impl ModelFamilyCodec for ClosedJsonFamilyCodec {
    fn identity(&self) -> &FamilyCodecIdentity {
        &self.identity
    }

    fn encode_context(
        &self,
        profile: &ExactModelProfile,
        packet: &ModelContextPacket,
    ) -> Result<EncodedModelContext, ModelRuntimeFailure> {
        if profile.codec != self.identity
            || packet.profile_id != profile.profile_id
            || packet.manifest_sha256 != profile.manifest_sha256
            || packet.messages.is_empty()
        {
            return Err(failure("model.codec.context-mismatch", None));
        }
        let bytes = to_canonical_json(packet)
            .map_err(|error| failure("model.codec.context-invalid", Some(error)))?;
        Ok(EncodedModelContext {
            schema_version: agentmage_kernel_contracts::CONTRACT_SCHEMA_VERSION,
            codec_id: self.identity.codec_id.clone(),
            profile_id: profile.profile_id.clone(),
            context_packet_id: packet.context_packet_id.clone(),
            sha256: sha256(&bytes),
            bytes,
        })
    }

    fn decode_proposal(
        &self,
        profile: &ExactModelProfile,
        request: &ModelRunRequest,
        response: &[u8],
    ) -> Result<ClosedModelProposal, ModelRuntimeFailure> {
        if profile.codec != self.identity
            || request.profile_id != profile.profile_id
            || request.manifest_sha256 != profile.manifest_sha256
            || request.adapter_id != profile.runtime.adapter_id
        {
            return Err(failure("model.codec.request-mismatch", None));
        }
        let proposal: ClosedModelProposal = from_json(response)
            .map_err(|error| failure("model.codec.proposal-invalid", Some(error)))?;
        if proposal.model_run_id != request.model_run_id
            || proposal.context_packet_id != request.context_packet_id
            || proposal.profile_id != profile.profile_id
            || proposal.codec_id != self.identity.codec_id
            || proposal.correlation_id != request.correlation_id
            || proposal_digest(&proposal)? != proposal.proposal_sha256
        {
            return Err(failure("model.codec.proposal-mismatch", None));
        }
        Ok(proposal)
    }
}

/// Computes the closed proposal digest over a zeroed digest-field preimage.
pub fn proposal_digest(proposal: &ClosedModelProposal) -> Result<String, ModelRuntimeFailure> {
    let mut preimage = proposal.clone();
    preimage.proposal_sha256 = "0".repeat(64);
    let bytes = to_canonical_json(&preimage)
        .map_err(|error| failure("model.codec.proposal-preimage-invalid", Some(error)))?;
    Ok(sha256(&bytes))
}

fn sha256(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn failure(
    code: &str,
    contract_error: Option<Box<agentmage_kernel_contracts::ContractError>>,
) -> ModelRuntimeFailure {
    ModelRuntimeFailure {
        code: code.to_owned(),
        retryable_after_correction: true,
        dependency_recovery_required: false,
        contract_error,
    }
}

#[cfg(test)]
mod tests {
    use agentmage_kernel_contracts::{
        CONTRACT_SCHEMA_VERSION, ClosedModelProposal, ContextBudget, ContextPacketId,
        ContractPayload, CorrelationId, DecodingProfile, ExactModelProfile, FamilyCodecIdentity,
        HardwareEnvelope, ModelAdapterId, ModelArtifact, ModelCapability, ModelCapabilityState,
        ModelCodecId, ModelContextPacket, ModelFamilyCodec, ModelLifecycleState, ModelManifestId,
        ModelMessage, ModelMessageId, ModelMessageRole, ModelModality, ModelProfileId,
        ModelProposalKind, ModelRole, ModelRunId, ModelRunRequest, ModelRuntimeIdentity,
        ModelRuntimeKind, PlatformArchitecture, PlatformFamily, ProposalId, SchemaId,
        SchemaReference, SessionId, TaskId, ToolCatalogId, to_canonical_json,
    };

    use super::{ClosedJsonFamilyCodec, proposal_digest};

    const SHA: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

    fn profile(family: &str) -> ExactModelProfile {
        let codec = FamilyCodecIdentity {
            codec_id: ModelCodecId::from_raw(format!("fixture-{family}-codec")),
            codec_version: "1".to_owned(),
            codec_sha256: SHA.to_owned(),
            tokenizer: format!("fixture-{family}-tokenizer"),
            tokenizer_sha256: SHA.to_owned(),
            template: format!("fixture-{family}-template"),
            template_sha256: SHA.to_owned(),
            tool_protocol_version: "closed-v1".to_owned(),
            end_tokens: vec![1],
            reasoning_enabled: false,
        };
        ExactModelProfile {
            schema_version: CONTRACT_SCHEMA_VERSION,
            profile_id: ModelProfileId::from_raw(format!("fixture-{family}")),
            manifest_id: ModelManifestId::from_raw(format!("fixture-{family}-manifest")),
            manifest_sha256: SHA.to_owned(),
            display_name: format!("Fixture {family}"),
            family: format!("deterministic_fake_{family}"),
            publisher_control: "fixture".to_owned(),
            lineage: vec!["fixture".to_owned()],
            license_spdx: "Apache-2.0".to_owned(),
            license_terms_sha256: SHA.to_owned(),
            artifact: ModelArtifact {
                artifact_id: "fixture".to_owned(),
                publisher: "fixture".to_owned(),
                source_revision: "fixture".to_owned(),
                format: "fixture".to_owned(),
                bytes: 1,
                sha256: SHA.to_owned(),
            },
            transformations: vec![],
            codec,
            runtime: ModelRuntimeIdentity {
                adapter_id: ModelAdapterId::from_raw(format!("fixture-{family}-adapter")),
                kind: ModelRuntimeKind::DeterministicFake,
                contract_version: 1,
                runtime_build: "fixture".to_owned(),
                runtime_sha256: SHA.to_owned(),
                platform: PlatformFamily::DeterministicFake,
                architecture: PlatformArchitecture::X86_64,
            },
            quantization: "none".to_owned(),
            modalities: vec![ModelModality::Text],
            context: ContextBudget {
                max_context_tokens: 64,
                max_input_bytes: 1_024,
                max_messages: 4,
                token_counter: "fixture".to_owned(),
                token_counter_sha256: SHA.to_owned(),
            },
            decoding: DecodingProfile {
                profile_id: "fixture".to_owned(),
                sampler_order: vec!["exact".to_owned()],
                temperature: 0.0,
                top_p: 1.0,
                top_k: 1,
                repeat_penalty: 1.0,
                seed: 1,
                max_output_tokens: 16,
            },
            hardware: vec![HardwareEnvelope {
                platform: PlatformFamily::DeterministicFake,
                architecture: PlatformArchitecture::X86_64,
                minimum_system_memory_bytes: 1,
                minimum_accelerator_memory_bytes: 0,
                accelerator: "none".to_owned(),
                driver_constraint: "none".to_owned(),
            }],
            capabilities: vec![ModelCapability {
                role: ModelRole::Dialogue,
                state: ModelCapabilityState::NotEvaluated,
                evaluation_profile: None,
                result_sha256: None,
                limitations: vec!["fixture".to_owned()],
            }],
            policy_sha256: SHA.to_owned(),
            lifecycle: ModelLifecycleState::Candidate,
            enabled: false,
            automatic_fallback: false,
        }
    }

    fn packet(profile: &ExactModelProfile) -> ModelContextPacket {
        ModelContextPacket {
            schema_version: CONTRACT_SCHEMA_VERSION,
            context_packet_id: ContextPacketId::from_raw("context-1"),
            session_id: SessionId::from_raw("session-1"),
            task_id: TaskId::from_raw("task-1"),
            profile_id: profile.profile_id.clone(),
            manifest_sha256: profile.manifest_sha256.clone(),
            tool_catalog_id: ToolCatalogId::from_raw("tools-1"),
            messages: vec![ModelMessage {
                message_id: ModelMessageId::from_raw("message-1"),
                role: ModelMessageRole::User,
                content: ContractPayload {
                    schema: SchemaReference {
                        schema_id: SchemaId::from_raw("fixture"),
                        schema_version: 1,
                        schema_sha256: SHA.to_owned(),
                    },
                    media_type: "text/plain".to_owned(),
                    bytes: b"fixture".to_vec(),
                    sha256: SHA.to_owned(),
                },
            }],
            input_bytes: 7,
            input_tokens: 1,
            packet_sha256: SHA.to_owned(),
        }
    }

    fn request(profile: &ExactModelProfile) -> ModelRunRequest {
        ModelRunRequest {
            schema_version: CONTRACT_SCHEMA_VERSION,
            model_run_id: ModelRunId::from_raw("run-1"),
            correlation_id: CorrelationId::from_raw("correlation-1"),
            context_packet_id: ContextPacketId::from_raw("context-1"),
            profile_id: profile.profile_id.clone(),
            manifest_sha256: profile.manifest_sha256.clone(),
            adapter_id: profile.runtime.adapter_id.clone(),
            decoding_profile_id: "fixture".to_owned(),
            max_output_tokens: 16,
            timeout_ms: 100,
        }
    }

    fn proposal(profile: &ExactModelProfile) -> ClosedModelProposal {
        let mut value = ClosedModelProposal {
            schema_version: CONTRACT_SCHEMA_VERSION,
            proposal_id: ProposalId::from_raw("proposal-1"),
            model_run_id: ModelRunId::from_raw("run-1"),
            context_packet_id: ContextPacketId::from_raw("context-1"),
            profile_id: profile.profile_id.clone(),
            codec_id: profile.codec.codec_id.clone(),
            correlation_id: CorrelationId::from_raw("correlation-1"),
            kind: ModelProposalKind::CompletionCandidate,
            payload: None,
            tool_call: None,
            proposal_sha256: "0".repeat(64),
        };
        value.proposal_sha256 = proposal_digest(&value).expect("digest");
        value
    }

    #[test]
    fn fake_muse_and_gemma_use_the_same_closed_codec_contract() {
        for family in ["muse", "gemma"] {
            let profile = profile(family);
            let codec = ClosedJsonFamilyCodec::new(profile.codec.clone());
            let encoded = codec
                .encode_context(&profile, &packet(&profile))
                .expect("encode");
            assert_eq!(encoded.codec_id, profile.codec.codec_id);
            let candidate = proposal(&profile);
            let bytes = to_canonical_json(&candidate).expect("proposal bytes");
            assert_eq!(
                codec
                    .decode_proposal(&profile, &request(&profile), &bytes)
                    .expect("decode"),
                candidate
            );
        }
    }

    #[test]
    fn malformed_stale_replayed_and_authority_seeking_envelopes_remain_inert() {
        let profile = profile("muse");
        let codec = ClosedJsonFamilyCodec::new(profile.codec.clone());
        let request = request(&profile);
        let valid = proposal(&profile);
        let mut stale = valid.clone();
        stale.model_run_id = ModelRunId::from_raw("stale-run");
        stale.proposal_sha256 = proposal_digest(&stale).expect("digest");
        let mut changed_hash = valid.clone();
        changed_hash.proposal_sha256 = SHA.to_owned();
        for bytes in [
            b"{".to_vec(),
            [
                to_canonical_json(&valid).expect("valid"),
                b" trailing".to_vec(),
            ]
            .concat(),
            serde_json::to_vec(&serde_json::json!({"schema_version": 2, "grant": true}))
                .expect("authority bytes"),
            to_canonical_json(&stale).expect("stale"),
            to_canonical_json(&changed_hash).expect("changed hash"),
        ] {
            assert!(codec.decode_proposal(&profile, &request, &bytes).is_err());
        }
    }

    #[test]
    fn codec_identity_or_context_substitution_fails_before_encoding_or_decoding() {
        let selected = profile("muse");
        let foreign = profile("gemma");
        let codec = ClosedJsonFamilyCodec::new(selected.codec.clone());
        assert!(codec.encode_context(&foreign, &packet(&foreign)).is_err());
        let bytes = to_canonical_json(&proposal(&foreign)).expect("foreign");
        assert!(
            codec
                .decode_proposal(&foreign, &request(&foreign), &bytes)
                .is_err()
        );
    }
}
