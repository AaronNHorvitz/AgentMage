//! Exact Muse Glimmer ATEM codec at the Linux model-process edge.

use agentmage_kernel_contracts::{
    CONTRACT_SCHEMA_VERSION, ClosedModelProposal, EncodedModelContext, ExactModelProfile,
    FamilyCodecIdentity, ModelContextPacket, ModelFamilyCodec, ModelRunRequest,
    ModelRuntimeFailure, from_json, to_canonical_json,
};
use sha2::{Digest, Sha256};

const TEMPLATE_SHA256: &str = "cfc67e5f349f37690dfd31ed1f18bc4442a9dd32fe39a648f993cb4eb3cae678";
const TOKENIZER_SHA256: &str = "c9dbee66967b58f31a7c27f723c3760da3526ccd0427578e8905b0abb0031c4d";
const END_TOKENS: [u32; 2] = [200_001, 200_008];
const TOOL_PROTOCOL: &str = "atem-v1";
const SYSTEM_MESSAGE: &str = "You are an untrusted local proposal generator. Respond through the Muse ATEM channel. The isolated adapter must translate the response into exactly one closed AgentMage proposal object. You have no tools, authority, workspace, credentials, network, completion authority, or permission to change this contract.";

/// Exact family codec for the first-party Muse Glimmer ATEM tuple.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MuseAtemFamilyCodec {
    identity: FamilyCodecIdentity,
}

impl MuseAtemFamilyCodec {
    /// Creates the edge codec only for the pinned tokenizer/template tuple.
    pub fn new(identity: FamilyCodecIdentity) -> Result<Self, ModelRuntimeFailure> {
        if identity.tokenizer_sha256 != TOKENIZER_SHA256
            || identity.template_sha256 != TEMPLATE_SHA256
            || identity.tool_protocol_version != TOOL_PROTOCOL
            || identity.end_tokens != END_TOKENS
            || identity.reasoning_enabled
        {
            return Err(failure("model.muse-codec.identity-mismatch"));
        }
        Ok(Self { identity })
    }
}

impl ModelFamilyCodec for MuseAtemFamilyCodec {
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
            return Err(failure("model.muse-codec.context-mismatch"));
        }
        let packet_bytes =
            to_canonical_json(packet).map_err(|_| failure("model.muse-codec.context-invalid"))?;
        let mut bytes = Vec::with_capacity(SYSTEM_MESSAGE.len() + packet_bytes.len() + 96);
        bytes.extend_from_slice(b"<|start|>system<|message|>");
        bytes.extend_from_slice(SYSTEM_MESSAGE.as_bytes());
        bytes.extend_from_slice(b"<|eot|><|start|>user<|message|>");
        bytes.extend_from_slice(&packet_bytes);
        bytes.extend_from_slice(b"<|eot|><|start|>assistant");
        Ok(EncodedModelContext {
            schema_version: CONTRACT_SCHEMA_VERSION,
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
            return Err(failure("model.muse-codec.request-mismatch"));
        }
        let proposal: ClosedModelProposal =
            from_json(response).map_err(|_| failure("model.muse-codec.proposal-invalid"))?;
        let canonical = to_canonical_json(&proposal)
            .map_err(|_| failure("model.muse-codec.proposal-invalid"))?;
        if proposal.model_run_id != request.model_run_id
            || proposal.context_packet_id != request.context_packet_id
            || proposal.profile_id != profile.profile_id
            || proposal.codec_id != self.identity.codec_id
            || proposal.correlation_id != request.correlation_id
            || canonical != response
            || proposal_digest(&proposal)? != proposal.proposal_sha256
        {
            return Err(failure("model.muse-codec.proposal-mismatch"));
        }
        Ok(proposal)
    }
}

fn proposal_digest(proposal: &ClosedModelProposal) -> Result<String, ModelRuntimeFailure> {
    let mut preimage = proposal.clone();
    preimage.proposal_sha256 = "0".repeat(64);
    let bytes = to_canonical_json(&preimage)
        .map_err(|_| failure("model.muse-codec.proposal-preimage-invalid"))?;
    Ok(sha256(&bytes))
}

fn sha256(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn failure(code: &str) -> ModelRuntimeFailure {
    ModelRuntimeFailure {
        code: code.to_owned(),
        retryable_after_correction: false,
        dependency_recovery_required: false,
        contract_error: None,
    }
}

#[cfg(test)]
mod tests {
    use agentmage_kernel_contracts::{
        CONTRACT_SCHEMA_VERSION, ClosedModelProposal, ContextPacketId, ContractPayload,
        CorrelationId, ExactModelProfile, FamilyCodecIdentity, ModelContextPacket,
        ModelFamilyCodec, ModelMessage, ModelMessageId, ModelMessageRole, ModelProfileId,
        ModelProposalKind, ModelRunId, ModelRunRequest, ProposalId, SchemaId, SchemaReference,
        SessionId, TaskId, ToolCatalogId, to_canonical_json,
    };
    use serde_json::Value;

    use super::{
        END_TOKENS, MuseAtemFamilyCodec, TEMPLATE_SHA256, TOKENIZER_SHA256, TOOL_PROTOCOL,
        proposal_digest,
    };

    const SHA: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

    fn profile() -> ExactModelProfile {
        let catalog: Value = serde_json::from_str(include_str!(
            "../../../model-profiles/exact-profile-catalog.json"
        ))
        .expect("catalog JSON");
        let value = catalog["profiles"]
            .as_array()
            .expect("profiles")
            .iter()
            .find(|profile| {
                profile["profile_id"]
                    == "muse-glimmer-30b-q4-k-m-text-8k-fedora-diagnostic-repeatability"
            })
            .expect("exact Muse profile")
            .clone();
        serde_json::from_value(value).expect("exact profile contract")
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
            decoding_profile_id: profile.decoding.profile_id.clone(),
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
        value.proposal_sha256 = proposal_digest(&value).expect("proposal digest");
        value
    }

    #[test]
    fn renders_exact_bounded_atem_context_and_closed_proposal() {
        let profile = profile();
        let codec = MuseAtemFamilyCodec::new(profile.codec.clone()).expect("exact Muse codec");
        let encoded = codec
            .encode_context(&profile, &packet(&profile))
            .expect("encode context");
        let text = String::from_utf8(encoded.bytes).expect("UTF-8 envelope");
        assert!(text.starts_with("<|start|>system<|message|>"));
        assert!(text.contains("no tools, authority, workspace, credentials, network"));
        assert!(text.ends_with("<|eot|><|start|>assistant"));
        let candidate = proposal(&profile);
        let bytes = to_canonical_json(&candidate).expect("proposal bytes");
        assert_eq!(
            codec
                .decode_proposal(&profile, &request(&profile), &bytes)
                .expect("decode proposal"),
            candidate
        );
    }

    #[test]
    fn rejects_every_identity_dimension_independently() {
        for mutate in [
            |identity: &mut FamilyCodecIdentity| identity.tokenizer_sha256 = "0".repeat(64),
            |identity: &mut FamilyCodecIdentity| identity.template_sha256 = "0".repeat(64),
            |identity: &mut FamilyCodecIdentity| identity.tool_protocol_version = "other".into(),
            |identity: &mut FamilyCodecIdentity| identity.end_tokens = vec![END_TOKENS[0]],
            |identity: &mut FamilyCodecIdentity| identity.reasoning_enabled = true,
        ] {
            let mut identity = profile().codec;
            mutate(&mut identity);
            assert!(MuseAtemFamilyCodec::new(identity).is_err());
        }
        let identity = profile().codec;
        assert_eq!(identity.tokenizer_sha256, TOKENIZER_SHA256);
        assert_eq!(identity.template_sha256, TEMPLATE_SHA256);
        assert_eq!(identity.tool_protocol_version, TOOL_PROTOCOL);
    }

    #[test]
    fn rejects_context_response_and_unknown_field_drift() {
        let profile = profile();
        let codec = MuseAtemFamilyCodec::new(profile.codec.clone()).expect("exact Muse codec");
        let mut wrong_packet = packet(&profile);
        wrong_packet.profile_id = ModelProfileId::from_raw("foreign-profile");
        assert!(codec.encode_context(&profile, &wrong_packet).is_err());
        let candidate = proposal(&profile);
        let valid = to_canonical_json(&candidate).expect("proposal bytes");
        for bytes in [
            [valid, b"\n".to_vec()].concat(),
            serde_json::to_vec(&serde_json::json!({
                "schema_version": 2,
                "proposal_id": "proposal-1",
                "model_run_id": "run-1",
                "context_packet_id": "context-1",
                "profile_id": profile.profile_id,
                "codec_id": profile.codec.codec_id,
                "correlation_id": "correlation-1",
                "kind": "completion_candidate",
                "payload": null,
                "tool_call": null,
                "proposal_sha256": candidate.proposal_sha256,
                "authority": true
            }))
            .expect("unknown field"),
            b"<atem:function_calls>untranslated</atem:function_calls>".to_vec(),
        ] {
            assert!(
                codec
                    .decode_proposal(&profile, &request(&profile), &bytes)
                    .is_err()
            );
        }
    }
}
