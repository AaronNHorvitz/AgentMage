//! Deterministic context admission and model-delivery coverage receipts.

use agentmage_kernel_contracts::{
    ArtifactCaptureResult, ArtifactCoverageState, CONTRACT_SCHEMA_VERSION, ContextArtifactCoverage,
    ContextDeliveryReceipt, ContextPacketId, EndpointProfileId, ModelProfileId, RouteDecisionId,
    RuntimeRunId,
};
use serde::Serialize;
use sha2::{Digest, Sha256};

const ZERO_SHA256: &str = "0000000000000000000000000000000000000000000000000000000000000000";

/// One deterministic context-admission policy for an exact model turn.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContextAdmissionPolicy {
    /// Maximum total exact source bytes placed inline.
    pub max_inline_bytes: u64,
    /// Context token ceiling.
    pub token_limit: u64,
    /// Conservative bytes-per-token estimate, never zero.
    pub estimated_bytes_per_token: u64,
}

/// Stable context-admission refusal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VerifiedContextError {
    /// A profile, artifact, budget, or identity failed closed validation.
    InvalidInput,
    /// Context accounting exceeded a declared numeric bound.
    ResourceExceeded,
    /// Receipt serialization failed.
    Serialization,
}

impl VerifiedContextError {
    /// Returns one stable redacted error code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidInput => "engineering.context.input.invalid",
            Self::ResourceExceeded => "engineering.context.resource.exceeded",
            Self::Serialization => "engineering.context.serialization.failed",
        }
    }
}

/// Builds a deterministic delivery receipt without claiming unperformed retrieval.
#[allow(clippy::too_many_arguments)]
pub fn admit_context(
    context_packet_id: ContextPacketId,
    run_id: RuntimeRunId,
    model_profile_id: ModelProfileId,
    endpoint_profile_id: EndpointProfileId,
    route_decision_id: RouteDecisionId,
    artifacts: &[ArtifactCaptureResult],
    artifact_bytes: &[Vec<u8>],
    policy: &ContextAdmissionPolicy,
) -> Result<ContextDeliveryReceipt, VerifiedContextError> {
    if context_packet_id.as_str().is_empty()
        || run_id.as_str().is_empty()
        || model_profile_id.as_str().is_empty()
        || endpoint_profile_id.as_str().is_empty()
        || route_decision_id.as_str().is_empty()
        || policy.max_inline_bytes == 0
        || policy.token_limit == 0
        || policy.estimated_bytes_per_token == 0
        || artifacts.len() != artifact_bytes.len()
    {
        return Err(VerifiedContextError::InvalidInput);
    }
    let mut inline_bytes = 0_u64;
    let mut coverage = Vec::with_capacity(artifacts.len());
    let mut context_material = Vec::new();
    for (artifact, bytes) in artifacts.iter().zip(artifact_bytes) {
        if artifact.schema_version != CONTRACT_SCHEMA_VERSION
            || artifact.byte_length == 0
            || artifact.source_sha256.len() != 64
            || artifact.byte_length != bytes.len() as u64
            || artifact.source_sha256 != sha256(bytes)
        {
            return Err(VerifiedContextError::InvalidInput);
        }
        let remaining = policy.max_inline_bytes.saturating_sub(inline_bytes);
        let (state, ranges) = if artifact.byte_length <= remaining {
            inline_bytes = inline_bytes
                .checked_add(artifact.byte_length)
                .ok_or(VerifiedContextError::ResourceExceeded)?;
            context_material.extend_from_slice(bytes);
            (
                ArtifactCoverageState::AdmittedInline,
                vec![(0, artifact.byte_length)],
            )
        } else {
            (ArtifactCoverageState::RetrievalAvailable, Vec::new())
        };
        coverage.push(ContextArtifactCoverage {
            artifact_id: artifact.artifact_id.clone(),
            source_sha256: artifact.source_sha256.clone(),
            source_bytes: artifact.byte_length,
            state,
            ranges,
            reason_codes: Vec::new(),
        });
    }
    let estimated_tokens = inline_bytes.saturating_add(policy.estimated_bytes_per_token - 1)
        / policy.estimated_bytes_per_token;
    if estimated_tokens > policy.token_limit {
        return Err(VerifiedContextError::ResourceExceeded);
    }
    let mut receipt = ContextDeliveryReceipt {
        schema_version: CONTRACT_SCHEMA_VERSION,
        context_packet_id,
        run_id,
        model_profile_id,
        endpoint_profile_id,
        route_decision_id,
        artifacts: coverage,
        inline_bytes,
        estimated_tokens,
        token_limit: policy.token_limit,
        context_sha256: sha256(&context_material),
        receipt_sha256: ZERO_SHA256.to_owned(),
    };
    receipt.receipt_sha256 = canonical_sha256(&receipt)?;
    Ok(receipt)
}

/// Marks exact model-invoked retrieval without broadening the admitted source set.
pub fn record_retrieval(
    receipt: &mut ContextDeliveryReceipt,
    artifact_id: &agentmage_kernel_contracts::RuntimeArtifactId,
    start: u64,
    end: u64,
) -> Result<(), VerifiedContextError> {
    if start >= end {
        return Err(VerifiedContextError::InvalidInput);
    }
    let artifact = receipt
        .artifacts
        .iter_mut()
        .find(|candidate| &candidate.artifact_id == artifact_id)
        .ok_or(VerifiedContextError::InvalidInput)?;
    if end > artifact.source_bytes || artifact.state == ArtifactCoverageState::Blocked {
        return Err(VerifiedContextError::InvalidInput);
    }
    artifact.ranges.push((start, end));
    artifact.ranges.sort_unstable();
    artifact.state = if artifact.ranges == vec![(0, artifact.source_bytes)] {
        ArtifactCoverageState::Retrieved
    } else {
        ArtifactCoverageState::PartiallyExamined
    };
    receipt.receipt_sha256 = ZERO_SHA256.to_owned();
    receipt.receipt_sha256 = canonical_sha256(receipt)?;
    Ok(())
}

fn canonical_sha256<T: Serialize>(value: &T) -> Result<String, VerifiedContextError> {
    serde_json::to_vec(value)
        .map(|bytes| sha256(&bytes))
        .map_err(|_| VerifiedContextError::Serialization)
}

fn sha256(bytes: &[u8]) -> String {
    let mut digest = Sha256::new();
    digest.update(bytes);
    hex_digest(digest.finalize().as_slice())
}

fn hex_digest(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(char::from(HEX[usize::from(byte >> 4)]));
        output.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    output
}

#[cfg(test)]
mod tests {
    use super::{ContextAdmissionPolicy, admit_context, record_retrieval};
    use agentmage_kernel_contracts::{
        ArtifactCaptureDisposition, ArtifactCaptureResult, ArtifactCoverageState,
        ArtifactSourceKind, ArtifactUploadId, CONTRACT_SCHEMA_VERSION, ContextPacketId,
        EndpointProfileId, ModelProfileId, RouteDecisionId, RuntimeArtifactId, RuntimeRunId,
        SessionId,
    };

    fn capture(id: &str, bytes: u64) -> ArtifactCaptureResult {
        ArtifactCaptureResult {
            schema_version: CONTRACT_SCHEMA_VERSION,
            upload_id: ArtifactUploadId::from_raw(format!("upload-{id}")),
            artifact_id: RuntimeArtifactId::from_raw(id),
            session_id: SessionId::from_raw("session-context"),
            source_kind: ArtifactSourceKind::Paste,
            display_name: id.to_owned(),
            media_type: "text/plain".to_owned(),
            byte_length: bytes,
            line_count: Some(1),
            source_sha256: super::sha256(&vec![b'a'; bytes as usize]),
            disposition: ArtifactCaptureDisposition::CapturedExactly,
            payload_deduplicated: false,
            completed_at_epoch_ms: 1,
            warning_codes: Vec::new(),
            receipt_sha256: "b".repeat(64),
        }
    }

    #[test]
    fn small_sources_inline_and_large_sources_remain_exactly_retrievable() {
        let mut receipt = admit_context(
            ContextPacketId::from_raw("context-1"),
            RuntimeRunId::from_raw("run-1"),
            ModelProfileId::from_raw("model-1"),
            EndpointProfileId::from_raw("endpoint-1"),
            RouteDecisionId::from_raw("route-1"),
            &[capture("small", 128), capture("large", 50_000)],
            &[vec![b'a'; 128], vec![b'a'; 50_000]],
            &ContextAdmissionPolicy {
                max_inline_bytes: 1024,
                token_limit: 4096,
                estimated_bytes_per_token: 4,
            },
        )
        .unwrap();
        assert_eq!(
            receipt.artifacts[0].state,
            ArtifactCoverageState::AdmittedInline
        );
        assert_eq!(
            receipt.artifacts[1].state,
            ArtifactCoverageState::RetrievalAvailable
        );
        record_retrieval(
            &mut receipt,
            &RuntimeArtifactId::from_raw("large"),
            0,
            50_000,
        )
        .unwrap();
        assert_eq!(receipt.artifacts[1].state, ArtifactCoverageState::Retrieved);
    }
}
