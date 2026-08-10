//! Evidence-reference and operation-receipt contracts.

use crate::{
    ActionId, ContractError, CorrelationId, EvidenceId, OperationOutcome, ReceiptId, SessionId,
    TaskId, ToolCallId,
};

/// Class of evidence referenced by a contract record.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceKind {
    /// Direct deterministic observation.
    Observation,
    /// Versioned document or structured record.
    Document,
    /// Prior operation receipt.
    Receipt,
    /// Bounded output from a tool attempt.
    ToolOutput,
    /// Validation result.
    Validation,
    /// Explicit user, policy, or review decision.
    Decision,
}

/// Versioned content-addressed reference to bounded evidence.
///
/// The reference carries identity and freshness material, not ambient filesystem access.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvidenceReference {
    /// Contract schema version.
    pub schema_version: u16,
    /// Stable evidence identity.
    pub evidence_id: EvidenceId,
    /// Evidence class.
    pub kind: EvidenceKind,
    /// Stable source identity resolved by the owning adapter.
    pub source_id: String,
    /// Stable object identity within the source.
    pub object_id: String,
    /// Optional structured fragment or bounded range identity.
    pub fragment: Option<String>,
    /// Lowercase SHA-256 digest of the observed source content.
    pub content_sha256: String,
    /// Source revision observed when the reference was created.
    pub observed_revision: Option<String>,
}

/// Versioned terminal record for one operation attempt.
///
/// This base contract records identities, outcome, evidence, and chain fields. Later
/// stories add durable keyed integrity, retention, and complete product event coverage.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Receipt {
    /// Contract schema version.
    pub schema_version: u16,
    /// Stable receipt identity.
    pub receipt_id: ReceiptId,
    /// Monotonic sequence within the owning session.
    pub sequence: u64,
    /// Correlation identity shared by related records.
    pub correlation_id: CorrelationId,
    /// Owning local session.
    pub session_id: SessionId,
    /// Owning task.
    pub task_id: TaskId,
    /// Attempted action.
    pub action_id: ActionId,
    /// Tool-call identity when the action attempted a tool.
    pub tool_call_id: Option<ToolCallId>,
    /// Exact terminal outcome.
    pub outcome: OperationOutcome,
    /// Lowercase SHA-256 digest of the canonical operation description.
    pub operation_sha256: String,
    /// Evidence references retained for the attempt.
    pub evidence: Vec<EvidenceReference>,
    /// Optional typed terminal error.
    pub error: Option<ContractError>,
    /// Previous receipt digest or the all-zero genesis digest.
    pub previous_receipt_sha256: String,
    /// Digest of this canonical receipt record.
    pub receipt_sha256: String,
    /// UTC wall-clock timestamp retained as a wire value.
    pub occurred_at: String,
}

#[cfg(test)]
mod tests {
    use super::{EvidenceKind, EvidenceReference};
    use crate::EvidenceId;

    #[test]
    fn evidence_reference_is_content_addressed_not_path_authority() {
        let evidence = EvidenceReference {
            schema_version: 1,
            evidence_id: EvidenceId::from_raw("evidence-0001"),
            kind: EvidenceKind::Observation,
            source_id: "synthetic-corpus-v1".to_owned(),
            object_id: "case-0001".to_owned(),
            fragment: Some("record:1".to_owned()),
            content_sha256: "1".repeat(64),
            observed_revision: Some("fixture-v1".to_owned()),
        };
        assert_eq!(evidence.content_sha256.len(), 64);
        assert_eq!(evidence.source_id, "synthetic-corpus-v1");
    }
}
