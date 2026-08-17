//! Path-free runtime artifact and resumable-checkpoint contracts.

use crate::{
    AgentStateKind, AgentStateTransition, ContextSensitivity, EvidenceReference, PolicyId,
    ReceiptId, RuntimeArtifactId, RuntimeEventCursor, RuntimeEventRetention, RuntimeOperationId,
    RuntimeRunId, RuntimeTurnId, SessionCheckpointId, SessionId, TaskId, ToolCallId, ToolResult,
};

/// Closed semantic family for one runtime-generated payload.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeArtifactKind {
    /// A proposed or applied source-code patch.
    Patch,
    /// Standard output captured from a bounded command.
    StandardOutput,
    /// Standard error captured from a bounded command.
    StandardError,
    /// Output produced by a trusted validation or test run.
    TestLog,
    /// A generated file whose bytes remain outside the event envelope.
    GeneratedFile,
    /// A generated analysis or verification report.
    Report,
    /// Bounded model output too large for the event or transcript projection.
    ModelOutput,
}

/// Integrity state established for immutable artifact bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeArtifactIntegrityState {
    /// The payload exists and matches its exact digest and byte size.
    Verified,
    /// The payload was isolated after a failed publication or verification.
    Quarantined,
    /// The referenced payload is absent.
    Missing,
    /// The retained payload no longer matches its immutable identity.
    Corrupt,
    /// The payload was intentionally removed under its retention policy.
    Deleted,
}

/// Current SQLite-authoritative lifecycle state for one artifact reference.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeArtifactLifecycleState {
    /// The reference is current and its verified payload may be opened under policy.
    Active,
    /// The reference is retained for inspection but cannot return payload bytes.
    Quarantined,
    /// The owning session or explicit user operation released the reference.
    Released,
    /// Metadata records an already completed payload deletion.
    Deleted,
}

/// Bounded user-visible text derived from the beginning of an artifact.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeArtifactPreview {
    /// UTF-8 preview text retained in encrypted metadata.
    pub text: String,
    /// Exact UTF-8 byte count of `text`.
    pub byte_size: u32,
    /// Whether the payload contains bytes beyond this preview.
    pub truncated: bool,
    /// Lowercase SHA-256 digest of the exact preview bytes.
    pub sha256: String,
}

/// Path-free verified reference carried by events, checkpoints, and clients.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeArtifactRef {
    /// Contract schema version.
    pub schema_version: u16,
    /// Stable logical artifact identity; this is not path authority.
    pub artifact_id: RuntimeArtifactId,
    /// Digest of the immutable manifest that grants meaning to this reference.
    pub manifest_sha256: String,
    /// Lowercase SHA-256 digest used as the private payload content address.
    pub payload_sha256: String,
    /// Exact immutable payload size.
    pub byte_size: u64,
    /// Closed or policy-approved payload media type.
    pub media_type: String,
}

/// Immutable metadata created when a runtime payload is admitted.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeArtifactManifest {
    /// Contract schema version.
    pub schema_version: u16,
    /// Stable logical artifact identity.
    pub artifact_id: RuntimeArtifactId,
    /// Semantic payload family.
    pub kind: RuntimeArtifactKind,
    /// Lowercase SHA-256 digest used as the private payload content address.
    pub payload_sha256: String,
    /// Exact immutable payload size.
    pub byte_size: u64,
    /// Closed or policy-approved payload media type.
    pub media_type: String,
    /// Sensitivity assigned before publication.
    pub sensitivity: ContextSensitivity,
    /// Exact retention assignment owned by canonical metadata.
    pub retention: RuntimeEventRetention,
    /// Owning local session; knowledge of this identity grants no access by itself.
    pub session_id: SessionId,
    /// Owning task.
    pub task_id: TaskId,
    /// Runtime run that produced the payload.
    pub producer_run_id: RuntimeRunId,
    /// Producing turn when the payload came from one turn.
    #[serde(deserialize_with = "crate::serialization::deserialize_required_option")]
    pub producer_turn_id: Option<RuntimeTurnId>,
    /// Producing operation when the payload came from one authorized operation.
    #[serde(deserialize_with = "crate::serialization::deserialize_required_option")]
    pub producer_operation_id: Option<RuntimeOperationId>,
    /// Terminal receipt for an effect-produced payload, when applicable.
    #[serde(deserialize_with = "crate::serialization::deserialize_required_option")]
    pub receipt_id: Option<ReceiptId>,
    /// Governing deterministic policy identity.
    pub policy_id: PolicyId,
    /// Digest of the exact governing policy revision.
    pub policy_sha256: String,
    /// Trusted creation time in Unix epoch milliseconds.
    pub created_at_epoch_ms: u64,
    /// Integrity state established at immutable publication.
    pub integrity: RuntimeArtifactIntegrityState,
    /// Optional bounded text preview retained separately from event payloads.
    #[serde(deserialize_with = "crate::serialization::deserialize_required_option")]
    pub preview: Option<RuntimeArtifactPreview>,
    /// Digest of this canonical manifest with this field set to all zeroes.
    pub manifest_sha256: String,
}

/// Exact durable linkage between one safe checkpoint and runtime persistence.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeResumeBinding {
    /// Contract schema version.
    pub schema_version: u16,
    /// Existing metadata-only safe-boundary checkpoint.
    pub checkpoint_id: SessionCheckpointId,
    /// Digest of the exact `SessionCheckpoint` record.
    pub checkpoint_sha256: String,
    /// Owning local session.
    pub session_id: SessionId,
    /// Exact active task.
    pub task_id: TaskId,
    /// Runtime run being resumed.
    pub run_id: RuntimeRunId,
    /// Last event known committed before this checkpoint became current.
    pub event_cursor: RuntimeEventCursor,
    /// Sorted exact artifact references required to reconstruct this checkpoint.
    pub artifacts: Vec<RuntimeArtifactRef>,
    /// Digest of this binding with this field set to all zeroes.
    pub binding_sha256: String,
}

/// Content-free repeated-call guard state retained across one durable restart.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeToolAttemptState {
    /// Contract schema version.
    pub schema_version: u16,
    /// Monotonic one-based attempt sequence.
    pub sequence: u64,
    /// Exact admitted tool-call identity.
    pub tool_call_id: ToolCallId,
    /// Digest of tool, version, action, schema, and exact argument identity.
    pub semantic_sha256: String,
    /// One-based occurrence of this semantic call.
    pub occurrence: u8,
    /// Explicit nested call depth.
    pub call_depth: u8,
}

/// Content-free resource usage retained across one durable restart.
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeResourceUsage {
    /// Admitted plan-step or turn attempts.
    pub plan_steps: u64,
    /// Admitted model calls.
    pub model_calls: u64,
    /// Admitted tool-call attempts.
    pub tool_calls: u64,
    /// Aggregate context and tool-argument bytes admitted as input.
    pub input_bytes: u64,
    /// Aggregate user-visible payload bytes admitted as output.
    pub output_bytes: u64,
    /// Aggregate model and trusted-worker elapsed milliseconds.
    pub elapsed_ms: u64,
    /// Peak trusted memory observation in bytes.
    pub peak_memory_bytes: u64,
    /// Aggregate retained artifact and scratch bytes accounted to the run.
    pub disk_bytes: u64,
    /// Conservative process attempts admitted before launch.
    pub process_count: u64,
    /// Canonical event envelopes admitted by the coordinator.
    pub event_count: u32,
    /// Aggregate canonical event-envelope bytes admitted by the coordinator.
    pub event_bytes: u64,
    /// Immutable artifacts admitted by the coordinator.
    pub artifact_count: u32,
    /// Aggregate immutable artifact payload bytes admitted by the coordinator.
    pub artifact_bytes: u64,
    /// Policy or user denials observed by the coordinator.
    pub denial_count: u32,
    /// Malformed model proposals observed by the coordinator.
    pub parser_failure_count: u32,
    /// Runtime-internal retries attempted by the coordinator; currently always zero.
    pub retry_count: u32,
}

/// Canonical interface-neutral coordinator state retained only at a safe continuation boundary.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeContinuationState {
    /// Contract schema version.
    pub schema_version: u16,
    /// Exact admitted runtime request digest.
    pub request_sha256: String,
    /// Runtime run being continued.
    pub run_id: RuntimeRunId,
    /// Owning local session.
    pub session_id: SessionId,
    /// Exact active task.
    pub task_id: TaskId,
    /// Last event represented by the continuation payload before its own publication event.
    pub event_cursor: RuntimeEventCursor,
    /// Exact nonterminal agent state at the safe boundary.
    pub agent_state: AgentStateKind,
    /// Current monotonic agent-state revision.
    pub agent_state_revision: u64,
    /// Complete ordered state-transition history used for deterministic reconstruction.
    pub state_transitions: Vec<AgentStateTransition>,
    /// Number of turns that have begun.
    pub turn_count: u32,
    /// Number of admitted model calls.
    pub model_call_count: u32,
    /// Number of admitted tool calls.
    pub tool_call_count: u32,
    /// Number of context recompositions.
    pub context_refresh_count: u32,
    /// Consecutive safe-boundary turns that produced no new evidence.
    pub no_progress_turns: u32,
    /// Complete content-free resource accounting at this safe boundary.
    pub resources: RuntimeResourceUsage,
    /// Ordered content-free repeated-call guard state.
    pub tool_attempts: Vec<RuntimeToolAttemptState>,
    /// Ordered tool results required to reconstruct the next bounded context.
    pub tool_results: Vec<ToolResult>,
    /// Current grounded evidence in stable identity order.
    pub evidence: Vec<EvidenceReference>,
    /// Canonical effect receipt identities in stable order.
    pub receipt_ids: Vec<ReceiptId>,
    /// Exact prior runtime artifacts in stable artifact-identity order.
    pub artifacts: Vec<RuntimeArtifactRef>,
    /// Digest of this canonical record with this field set to all zeroes.
    pub continuation_sha256: String,
}
