//! Kernel-owned authority-transaction records.

use crate::{
    ActionId, ApprovalId, AuthorityTransactionId, CorrelationId, GrantId, OperationAttemptId,
    OperationBinding, OperationOutcome, ReceiptId, SessionId, TaskId, ToolCallId,
};

/// Durable ordering state for one kernel-owned authority transaction.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum AuthorityTransactionState {
    /// Exact identities and current descriptive inputs were recorded before consumption.
    Prepared,
    /// Current policy admitted and atomically consumed the exact grant.
    GrantConsumed,
    /// A non-replayable attempt record was created before worker launch.
    AttemptRecorded,
    /// The kernel durably committed to crossing the worker launch boundary.
    LaunchCommitted,
    /// A terminal worker result is being reconciled against expected effects.
    Reconciling,
    /// Outcome and receipt were retained together; no later transition is legal.
    Terminal,
}

/// Versioned snapshot of one authority transaction revision.
///
/// This is a storage contract, not authority. A record cannot be converted into a
/// grant, launch token, held object, or worker capability.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuthorityTransactionRecord {
    /// Contract schema version.
    pub schema_version: u16,
    /// Stable transaction identity.
    pub authority_transaction_id: AuthorityTransactionId,
    /// Monotonically increasing transaction revision.
    pub revision: u32,
    /// Exact non-replayable attempt identity.
    pub operation_attempt_id: OperationAttemptId,
    /// Exact explicit approval decision.
    pub approval_id: ApprovalId,
    /// Exact grant considered or consumed by this transaction.
    pub grant_id: GrantId,
    /// Versioned canonical operation and independent authority class.
    pub operation: OperationBinding,
    /// Correlation identity shared with call, result, and receipt.
    pub correlation_id: CorrelationId,
    /// Owning session.
    pub session_id: SessionId,
    /// Owning task.
    pub task_id: TaskId,
    /// Exact attempted action.
    pub action_id: ActionId,
    /// Exact tool-call attempt proposed to the transaction.
    pub tool_call_id: ToolCallId,
    /// Current ordered transaction state.
    pub state: AuthorityTransactionState,
    /// Canonical digest of the consumed grant revision, absent before consumption.
    #[serde(deserialize_with = "crate::serialization::deserialize_required_option")]
    pub consumed_grant_sha256: Option<String>,
    /// Canonical digest of the reconciled result, absent before reconciliation.
    #[serde(deserialize_with = "crate::serialization::deserialize_required_option")]
    pub result_sha256: Option<String>,
    /// Reconciled outcome, absent until a result is durably available.
    #[serde(deserialize_with = "crate::serialization::deserialize_required_option")]
    pub outcome: Option<OperationOutcome>,
    /// Whether a possible effect could not be established safely.
    pub uncertain_effect: bool,
    /// Receipt identity retained atomically with terminal state.
    #[serde(deserialize_with = "crate::serialization::deserialize_required_option")]
    pub receipt_id: Option<ReceiptId>,
    /// Canonical digest of the retained receipt.
    #[serde(deserialize_with = "crate::serialization::deserialize_required_option")]
    pub receipt_sha256: Option<String>,
    /// UTC wall-clock timestamp associated with this transaction.
    pub occurred_at: String,
}

#[cfg(test)]
mod tests {
    use super::{AuthorityTransactionRecord, AuthorityTransactionState};
    use crate::{
        ActionId, ApprovalId, AuthorityTransactionId, CorrelationId, GrantId, GrantOperation,
        OperationAttemptId, OperationBinding, SessionId, TaskId, ToolCallId,
    };

    #[test]
    fn transaction_record_is_descriptive_and_binds_every_authority_identity() {
        let record = AuthorityTransactionRecord {
            schema_version: crate::CONTRACT_SCHEMA_VERSION,
            authority_transaction_id: AuthorityTransactionId::from_raw("transaction-0001"),
            revision: 1,
            operation_attempt_id: OperationAttemptId::from_raw("attempt-0001"),
            approval_id: ApprovalId::from_raw("approval-0001"),
            grant_id: GrantId::from_raw("grant-0001"),
            operation: OperationBinding::new(GrantOperation::WorkspaceRead),
            correlation_id: CorrelationId::from_raw("correlation-0001"),
            session_id: SessionId::from_raw("session-0001"),
            task_id: TaskId::from_raw("task-0001"),
            action_id: ActionId::from_raw("action-0001"),
            tool_call_id: ToolCallId::from_raw("call-0001"),
            state: AuthorityTransactionState::Prepared,
            consumed_grant_sha256: None,
            result_sha256: None,
            outcome: None,
            uncertain_effect: false,
            receipt_id: None,
            receipt_sha256: None,
            occurred_at: "1970-01-01T00:00:01Z".to_owned(),
        };
        let bytes = crate::to_canonical_json(&record).expect("record must encode");
        let decoded =
            crate::from_json::<AuthorityTransactionRecord>(&bytes).expect("record must decode");
        assert_eq!(decoded, record);
    }
}
