//! Interface-independent reusable-runtime request and outcome contracts.

use crate::{
    AgentStateKind, ApprovalId, ContextBudget, ContractPayload, EvidenceReference,
    ExactModelProfile, GrantId, GrantOperation, PolicyId, ReceiptId, RepositorySnapshotId,
    RuntimeEventId, RuntimeOperationId, RuntimePayloadReference, RuntimeRunId, RuntimeTurnId,
    SessionId, Task, TaskId, ToolCallId, ToolCatalogId, ToolId, WorkPacket, WorkspaceId,
};

/// Closed persistence and authority mode selected for one runtime run.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeSessionMode {
    /// Read-only execution whose state and output remain in process memory.
    EphemeralReadOnly,
    /// Read-only execution with explicitly enabled journal, checkpoint, and artifact ports.
    DurableReadOnly,
    /// Controlled state-changing execution through separately authorized effect boundaries.
    ControlledWrite,
}

/// Exact visible tool identity frozen into one runtime request.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeToolReference {
    /// Exact tool identity.
    pub tool_id: ToolId,
    /// Exact immutable tool-contract version.
    pub tool_version: String,
    /// Lowercase SHA-256 digest of the canonical registered definition.
    pub definition_sha256: String,
}

/// Inclusive ceilings for one reusable-runtime run.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeRunLimits {
    /// Maximum ordered turns.
    pub max_turns: u32,
    /// Maximum admitted model calls.
    pub max_model_calls: u32,
    /// Maximum admitted tool calls.
    pub max_tool_calls: u32,
    /// Maximum occurrences of one semantically identical tool call.
    pub max_repeated_tool_calls: u8,
    /// Maximum nested tool-call depth.
    pub max_tool_call_depth: u8,
    /// Maximum consecutive turns without new evidence or a terminal decision.
    pub max_no_progress_turns: u32,
    /// Maximum context recompositions.
    pub max_context_refreshes: u32,
    /// Maximum emitted runtime events.
    pub max_events: u32,
    /// Logical elapsed-time ceiling in milliseconds.
    pub max_elapsed_ms: u64,
    /// Maximum total user-visible output bytes.
    pub max_output_bytes: u64,
}

/// Exact continuation point in an already verified runtime-event chain.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeEventCursor {
    /// Runtime run whose event chain is being continued.
    pub run_id: RuntimeRunId,
    /// Last verified event identity.
    pub event_id: RuntimeEventId,
    /// Last verified zero-based event sequence.
    pub sequence: u64,
    /// Lowercase SHA-256 digest of the last verified event.
    pub event_sha256: String,
}

/// User decision accepted by a protected runtime approval boundary.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeApprovalDisposition {
    /// Permit only the exact operation after separate grant validation and consumption.
    Allow,
    /// Decline the exact operation without effect.
    Deny,
}

/// One immutable protected approval challenge returned by the coordinator.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeApprovalChallenge {
    /// Contract schema version.
    pub schema_version: u16,
    /// Owning runtime run.
    pub run_id: RuntimeRunId,
    /// Owning task.
    pub task_id: TaskId,
    /// Exact turn awaiting the decision.
    pub turn_id: RuntimeTurnId,
    /// Exact proposed operation.
    pub operation_id: RuntimeOperationId,
    /// Exact proposed tool call.
    pub tool_call_id: ToolCallId,
    /// Stable protected approval identity.
    pub approval_id: ApprovalId,
    /// Closed canonical operation awaiting approval.
    pub operation: GrantOperation,
    /// Digest of the complete user-visible preview.
    pub preview_sha256: String,
    /// Exclusive approval expiration in Unix epoch milliseconds.
    pub expires_at_epoch_ms: u64,
    /// Digest of this challenge with this field set to all zeroes.
    pub challenge_sha256: String,
}

/// One exact client response to a protected runtime approval challenge.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeApprovalResponse {
    /// Contract schema version.
    pub schema_version: u16,
    /// Owning runtime run.
    pub run_id: RuntimeRunId,
    /// Exact protected approval identity.
    pub approval_id: ApprovalId,
    /// User-selected disposition.
    pub disposition: RuntimeApprovalDisposition,
    /// Exact challenge digest displayed to the user.
    pub challenge_sha256: String,
    /// Exact separately issued grant identity only for an allowed response.
    #[serde(deserialize_with = "crate::serialization::deserialize_required_option")]
    pub grant_id: Option<GrantId>,
}

/// One versioned request admitted by the reusable runtime coordinator.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeRunRequest {
    /// Contract schema version.
    pub schema_version: u16,
    /// Stable reusable-runtime run identity.
    pub run_id: RuntimeRunId,
    /// Owning local session.
    pub session_id: SessionId,
    /// Exact selected persistence and authority mode.
    pub mode: RuntimeSessionMode,
    /// Complete user-directed task contract.
    pub task: Task,
    /// Exact revision-controlled work packet.
    pub work_packet: WorkPacket,
    /// Authorized workspace identity.
    pub workspace_id: WorkspaceId,
    /// Digest of the exact sealed workspace projection supplied to tools.
    pub workspace_snapshot_sha256: String,
    /// Exact repository observation identity.
    pub repository_snapshot_id: RepositorySnapshotId,
    /// Digest of the repository observation bound to this run.
    pub repository_snapshot_sha256: String,
    /// Complete exact admitted model tuple selected without fallback.
    pub model_profile: ExactModelProfile,
    /// Run-specific context ceiling, no broader than the selected profile.
    pub context_budget: ContextBudget,
    /// Exact frozen visible tool-catalog identity.
    pub tool_catalog_id: ToolCatalogId,
    /// Digest of the canonical visible tool catalog.
    pub tool_catalog_sha256: String,
    /// Stable ordered references to every visible tool definition.
    pub visible_tools: Vec<RuntimeToolReference>,
    /// Exact deterministic policy identity.
    pub policy_id: PolicyId,
    /// Digest of the complete policy revision.
    pub policy_sha256: String,
    /// Inclusive run ceilings.
    pub limits: RuntimeRunLimits,
    /// Optional verified prior event only when durable resume is enabled.
    #[serde(deserialize_with = "crate::serialization::deserialize_required_option")]
    pub event_cursor: Option<RuntimeEventCursor>,
    /// Lowercase SHA-256 digest of this request with this field set to all zeroes.
    pub request_sha256: String,
}

/// Bounded output carried directly or by content-addressed artifact reference.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "storage", rename_all = "snake_case", deny_unknown_fields)]
pub enum RuntimeOutput {
    /// Inline bounded output, permitted only when the selected mode allows it.
    Inline {
        /// Schema-bound output bytes.
        payload: ContractPayload,
    },
    /// Verified content-addressed output retained by the artifact port.
    Artifact {
        /// Exact immutable artifact reference.
        reference: RuntimePayloadReference,
    },
}

/// One canonical terminal result returned to every runtime client.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeOutcome {
    /// Contract schema version.
    pub schema_version: u16,
    /// Exact owning runtime run.
    pub run_id: RuntimeRunId,
    /// Exact owning session.
    pub session_id: SessionId,
    /// Exact owning task.
    pub task_id: TaskId,
    /// Digest of the admitted runtime request.
    pub request_sha256: String,
    /// One terminal agent state, with success admitted only by a verifier.
    pub state: AgentStateKind,
    /// Number of turns that began.
    pub turn_count: u32,
    /// Number of model calls admitted.
    pub model_call_count: u32,
    /// Number of tool calls admitted.
    pub tool_call_count: u32,
    /// Verified event immediately preceding the terminal event.
    pub prior_event_id: RuntimeEventId,
    /// Digest of the verified event immediately preceding the terminal event.
    pub prior_event_sha256: String,
    /// Current grounded evidence retained in stable order.
    pub evidence: Vec<EvidenceReference>,
    /// Canonical operation receipts retained in stable order.
    pub receipt_ids: Vec<ReceiptId>,
    /// Stable content-free blockers, failures, and unresolved-item codes.
    pub unresolved_codes: Vec<String>,
    /// Optional bounded inline output or verified artifact reference.
    #[serde(deserialize_with = "crate::serialization::deserialize_required_option")]
    pub output: Option<RuntimeOutput>,
    /// Digest of this outcome with this field set to all zeroes.
    pub outcome_sha256: String,
}
