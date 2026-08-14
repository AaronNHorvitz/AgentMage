//! Candidate-neutral family-codec boundary for exact context and proposal bytes.

use agentmage_kernel_contracts::{
    ClosedModelProposal, ContractPayload, EncodedModelContext, ExactModelProfile,
    FamilyCodecIdentity, ModelContextPacket, ModelFamilyCodec, ModelProposalKind,
    ModelProposalWireCandidate, ModelRunRequest, ModelRuntimeFailure, ModelToolCallCandidate,
    ProposalId, ToolCallId, from_json, to_canonical_json,
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
        let candidate: ModelProposalWireCandidate = from_json(response)
            .map_err(|error| failure("model.codec.proposal-invalid", Some(error)))?;
        let canonical = to_canonical_json(&candidate)
            .map_err(|error| failure("model.codec.proposal-invalid", Some(error)))?;
        if canonical != response || !valid_wire_candidate(&candidate) {
            return Err(failure("model.codec.proposal-mismatch", None));
        }
        let response_sha256 = sha256(response);
        let tool_call = candidate.tool_call.map(|tool_call| ModelToolCallCandidate {
            tool_call_id: ToolCallId::from_raw(format!("model-tool-call:{response_sha256}")),
            tool_id: tool_call.tool_id,
            tool_version: tool_call.tool_version,
            arguments: tool_call.arguments,
        });
        let mut proposal = ClosedModelProposal {
            schema_version: agentmage_kernel_contracts::CONTRACT_SCHEMA_VERSION,
            proposal_id: ProposalId::from_raw(format!("model-proposal:{response_sha256}")),
            model_run_id: request.model_run_id.clone(),
            context_packet_id: request.context_packet_id.clone(),
            profile_id: profile.profile_id.clone(),
            codec_id: self.identity.codec_id.clone(),
            correlation_id: request.correlation_id.clone(),
            kind: candidate.kind,
            payload: candidate.payload,
            tool_call,
            proposal_sha256: "0".repeat(64),
        };
        proposal.proposal_sha256 = proposal_digest(&proposal)?;
        Ok(proposal)
    }
}

fn valid_wire_candidate(candidate: &ModelProposalWireCandidate) -> bool {
    candidate.schema_version == agentmage_kernel_contracts::CONTRACT_SCHEMA_VERSION
        && matches!(candidate.kind, ModelProposalKind::ToolCall) == candidate.tool_call.is_some()
        && candidate.payload.as_ref().is_none_or(valid_payload)
        && candidate.tool_call.as_ref().is_none_or(|tool_call| {
            valid_identifier(tool_call.tool_id.as_str())
                && valid_identifier(&tool_call.tool_version)
                && valid_payload(&tool_call.arguments)
        })
}

fn valid_payload(payload: &ContractPayload) -> bool {
    valid_identifier(payload.schema.schema_id.as_str())
        && payload.schema.schema_version > 0
        && valid_sha256(&payload.schema.schema_sha256)
        && !payload.media_type.is_empty()
        && payload.media_type.len() <= 128
        && payload.media_type.is_ascii()
        && payload.bytes.len() <= 1_048_576
        && payload.sha256 == sha256(&payload.bytes)
}

fn valid_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 256
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b':' | b'-'))
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
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
pub(crate) mod tests_support {
    use agentmage_kernel_contracts::{
        CONTRACT_SCHEMA_VERSION, ClosedModelProposal, ContextBudget, ContextPacketId,
        ContractPayload, CorrelationId, DecodingProfile, ExactModelProfile, FamilyCodecIdentity,
        HardwareEnvelope, ModelAdapterId, ModelArtifact, ModelCapability, ModelCapabilityState,
        ModelCodecId, ModelContextPacket, ModelFamilyCodec, ModelLifecycleState, ModelManifestId,
        ModelMessage, ModelMessageId, ModelMessageRole, ModelModality, ModelProfileId,
        ModelProposalKind, ModelProposalWireCandidate, ModelRole, ModelRunId, ModelRunRequest,
        ModelRuntimeIdentity, ModelRuntimeKind, PlatformArchitecture, PlatformFamily, ProposalId,
        SchemaId, SchemaReference, SessionId, TaskId, ToolCatalogId, to_canonical_json,
    };

    use super::{ClosedJsonFamilyCodec, proposal_digest};

    const SHA: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

    pub(crate) fn profile(family: &str) -> ExactModelProfile {
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

    pub(crate) fn request(profile: &ExactModelProfile) -> ModelRunRequest {
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

    pub(crate) fn proposal(profile: &ExactModelProfile) -> ClosedModelProposal {
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

    pub(crate) fn wire_candidate() -> ModelProposalWireCandidate {
        ModelProposalWireCandidate {
            schema_version: CONTRACT_SCHEMA_VERSION,
            kind: ModelProposalKind::CompletionCandidate,
            payload: None,
            tool_call: None,
        }
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
            let bytes = to_canonical_json(&wire_candidate()).expect("wire candidate bytes");
            let proposal = codec
                .decode_proposal(&profile, &request(&profile), &bytes)
                .expect("decode");
            assert_eq!(proposal.model_run_id.as_str(), "run-1");
            assert_eq!(proposal.context_packet_id.as_str(), "context-1");
            assert_eq!(proposal.profile_id, profile.profile_id);
            assert_eq!(proposal.codec_id, profile.codec.codec_id);
            assert_eq!(proposal.correlation_id.as_str(), "correlation-1");
            assert_eq!(proposal.kind, ModelProposalKind::CompletionCandidate);
            assert_eq!(
                proposal.proposal_sha256,
                proposal_digest(&proposal).expect("digest")
            );
        }
    }

    #[test]
    fn malformed_stale_replayed_and_authority_seeking_envelopes_remain_inert() {
        let profile = profile("muse");
        let codec = ClosedJsonFamilyCodec::new(profile.codec.clone());
        let request = request(&profile);
        let valid = wire_candidate();
        for bytes in [
            b"{".to_vec(),
            [
                to_canonical_json(&valid).expect("valid"),
                b" trailing".to_vec(),
            ]
            .concat(),
            serde_json::to_vec(&serde_json::json!({"schema_version": 2, "grant": true}))
                .expect("authority bytes"),
            serde_json::to_vec(&serde_json::json!({
                "schema_version": 2,
                "kind": "tool_call",
                "payload": null,
                "tool_call": null
            }))
            .expect("missing tool call"),
            to_canonical_json(&proposal(&profile)).expect("forged trusted fields"),
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
        let bytes = to_canonical_json(&wire_candidate()).expect("foreign candidate");
        assert!(
            codec
                .decode_proposal(&foreign, &request(&foreign), &bytes)
                .is_err()
        );
    }
}
