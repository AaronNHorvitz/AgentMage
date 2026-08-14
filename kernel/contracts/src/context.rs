//! Bounded context, checked-summary, checkpoint, and resume contracts.

use crate::{
    ActionId, ActionState, ContextPacketId, ContextSummaryId, EvidenceId, GrantId, ModelProfileId,
    PlanId, PlanStepId, PolicyId, ReceiptId, RepositorySnapshotId, SessionCheckpointId, SessionId,
    TaskId, WorkspaceId,
};

/// Closed semantic class used by deterministic context prioritization.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum ContextItemKind {
    /// Effective governing instructions.
    Instruction,
    /// Newest unresolved user request.
    NewestRequest,
    /// Current active objective.
    ActiveObjective,
    /// Current plan step or next safe action.
    PlanStep,
    /// User correction that changes prior understanding.
    Correction,
    /// Explicit user approval or denial.
    Approval,
    /// Current blocker.
    Blocker,
    /// Content-addressed authoritative evidence.
    Evidence,
    /// Continuity memory that is not source evidence.
    Memory,
    /// Required output shape or destination.
    ExpectedOutput,
    /// Lower-value supporting context.
    Supporting,
}

/// Sensitivity label shown in content-free context diagnostics.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum ContextSensitivity {
    /// Public material.
    Public,
    /// Internal operational material.
    Internal,
    /// Private user material.
    Private,
    /// Restricted material requiring a separately approved policy path.
    Restricted,
}

/// Closed pre-composition disposition assigned by deterministic policy.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContextAdmission {
    /// Candidate may enter the bounded context packet.
    Eligible,
    /// Candidate is stale and may not enter until its source is reopened.
    Stale,
    /// Policy denied candidate disclosure to the selected model profile.
    Denied,
}

/// One bounded candidate supplied to deterministic context composition.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContextItemCandidate {
    /// Stable item identity within the session.
    pub item_id: String,
    /// Semantic priority class.
    pub kind: ContextItemKind,
    /// Sensitivity visible in diagnostics.
    pub sensitivity: ContextSensitivity,
    /// Current policy admission state.
    pub admission: ContextAdmission,
    /// Whether this is authoritative source evidence rather than a summary.
    pub authoritative_evidence: bool,
    /// Whether omission must stop composition instead of silently weakening intent.
    pub essential: bool,
    /// Stable source identity, never an ambient path capability.
    pub source_id: String,
    /// Exact source revision.
    pub source_revision: String,
    /// Lowercase SHA-256 digest of the complete source item.
    pub content_sha256: String,
    /// Already minimized bounded excerpt eligible for model disclosure.
    pub bounded_excerpt: String,
    /// Exact token count from the selected profile's pinned counter.
    pub token_count: u32,
}

/// Closed reason a context item was excluded from one packet.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContextOmissionReason {
    /// An identical lower-priority or non-authoritative candidate was removed.
    Duplicate,
    /// The packet byte or token budget had no remaining capacity.
    Budget,
    /// The source identity was stale.
    Stale,
    /// Deterministic disclosure policy denied the item.
    Denied,
}

/// Content-free accounting for one included or excluded candidate.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContextItemAccounting {
    /// Stable candidate identity.
    pub item_id: String,
    /// Semantic class.
    pub kind: ContextItemKind,
    /// Sensitivity label.
    pub sensitivity: ContextSensitivity,
    /// Stable source identity.
    pub source_id: String,
    /// Exact source revision.
    pub source_revision: String,
    /// Source content digest.
    pub content_sha256: String,
    /// Accounted UTF-8 bytes.
    pub byte_count: u64,
    /// Accounted tokens.
    pub token_count: u32,
    /// Whether the item entered the packet.
    pub included: bool,
    /// Exact omission reason when excluded.
    #[serde(deserialize_with = "crate::serialization::deserialize_required_option")]
    pub omission: Option<ContextOmissionReason>,
}

/// Versioned bounded context packet with complete content-free accounting.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ComposedContextPacket {
    /// Contract schema version.
    pub schema_version: u16,
    /// Stable packet identity.
    pub context_packet_id: ContextPacketId,
    /// Inclusive byte ceiling.
    pub max_bytes: u64,
    /// Inclusive token ceiling.
    pub max_tokens: u32,
    /// Exact token counter implementation identity.
    pub token_counter_id: String,
    /// Ordered admitted candidates, including only bounded excerpts.
    pub items: Vec<ContextItemCandidate>,
    /// Complete accounting for every input candidate.
    pub accounting: Vec<ContextItemAccounting>,
    /// Admitted UTF-8 bytes.
    pub used_bytes: u64,
    /// Admitted tokens.
    pub used_tokens: u32,
    /// Digest of the complete canonical packet.
    pub packet_sha256: String,
}

/// Freshness and sufficiency state of one checked summary.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CheckedSummaryState {
    /// Summary checks match current source identities and the requested use.
    Current,
    /// At least one source identity changed.
    Stale,
    /// A user or verifier disputes the summary.
    Disputed,
    /// The summary does not preserve enough detail for the requested use.
    Insufficient,
}

/// Checked continuity summary kept structurally separate from source evidence.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CheckedContextSummary {
    /// Contract schema version.
    pub schema_version: u16,
    /// Stable summary identity.
    pub summary_id: ContextSummaryId,
    /// Current summary state.
    pub state: CheckedSummaryState,
    /// Bounded summary prose; never authoritative source evidence.
    pub summary: String,
    /// Exact paths that must survive condensation.
    pub paths: Vec<String>,
    /// Stable error codes or exact bounded error text.
    pub errors: Vec<String>,
    /// Stable identifiers.
    pub identifiers: Vec<String>,
    /// Exact commands relevant to safe continuation.
    pub commands: Vec<String>,
    /// Explicit decisions and approvals.
    pub decisions: Vec<String>,
    /// Unresolved questions.
    pub unresolved_questions: Vec<String>,
    /// Source evidence identities referenced, never embedded, by the summary.
    pub evidence_ids: Vec<EvidenceId>,
    /// Citation identities referenced by the summary.
    pub citation_ids: Vec<String>,
    /// Receipt identities referenced by the summary.
    pub receipt_ids: Vec<ReceiptId>,
    /// Digest of the source-identity set used to check the summary.
    pub source_set_sha256: String,
}

/// One content-addressed file identity retained for resume drift checks.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CheckpointFileIdentity {
    /// Stable workspace-relative object identity.
    pub object_id: String,
    /// Exact content digest at checkpoint time.
    pub content_sha256: String,
    /// Exact observed revision.
    pub observed_revision: String,
}

/// Metadata-only safe-boundary checkpoint; it carries no authority and no source content.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SessionCheckpoint {
    /// Contract schema version.
    pub schema_version: u16,
    /// Stable checkpoint identity.
    pub checkpoint_id: SessionCheckpointId,
    /// Owning session.
    pub session_id: SessionId,
    /// Exact active task.
    pub task_id: TaskId,
    /// Digest of the active objective.
    pub objective_sha256: String,
    /// Exact plan identity.
    pub plan_id: PlanId,
    /// Exact plan revision.
    pub plan_revision: u32,
    /// Current plan step.
    pub plan_step_id: PlanStepId,
    /// Digest of the next safe action description.
    pub next_action_sha256: String,
    /// Active workspace.
    pub workspace_id: WorkspaceId,
    /// Digest of the complete session-environment capture.
    pub workspace_state_sha256: String,
    /// Exact repository snapshot.
    pub repository_snapshot_id: RepositorySnapshotId,
    /// Exact repository branch or detached-head marker.
    pub repository_branch: String,
    /// Digest of the pinned structural repository map.
    pub repository_map_sha256: String,
    /// File identities required by the next safe action.
    pub files: Vec<CheckpointFileIdentity>,
    /// Digest of effective instructions.
    pub instruction_sha256: String,
    /// Permission-profile identity.
    pub permission_profile_id: String,
    /// Permission-profile revision digest.
    pub permission_profile_sha256: String,
    /// Governing deterministic policy identity.
    pub policy_id: PolicyId,
    /// Governing policy revision digest.
    pub policy_sha256: String,
    /// Exact selected model profile.
    pub model_profile_id: ModelProfileId,
    /// Exact model manifest digest.
    pub model_manifest_sha256: String,
    /// Exact model runtime digest.
    pub model_runtime_sha256: String,
    /// Evidence identities required for continuation.
    pub evidence_ids: Vec<EvidenceId>,
    /// Digest of the citation identity set.
    pub citation_set_sha256: String,
    /// Stable blocker codes.
    pub blockers: Vec<String>,
    /// Digest of the bounded context manifest.
    pub context_packet_sha256: String,
    /// Last action included in the atomic publication, when any.
    #[serde(deserialize_with = "crate::serialization::deserialize_required_option")]
    pub action_id: Option<ActionId>,
    /// State of the last action, when any.
    #[serde(deserialize_with = "crate::serialization::deserialize_required_option")]
    pub action_state: Option<ActionState>,
    /// Consumed grant included in the atomic publication, when any.
    #[serde(deserialize_with = "crate::serialization::deserialize_required_option")]
    pub consumed_grant_id: Option<GrantId>,
    /// Terminal receipt included in the atomic publication, when any.
    #[serde(deserialize_with = "crate::serialization::deserialize_required_option")]
    pub receipt_id: Option<ReceiptId>,
    /// Digest of the terminal receipt, when any.
    #[serde(deserialize_with = "crate::serialization::deserialize_required_option")]
    pub receipt_sha256: Option<String>,
    /// Whether the session is explicitly memory-only and must not be persisted.
    pub ephemeral: bool,
    /// Digest of this checkpoint with this field set to the all-zero digest.
    pub checkpoint_sha256: String,
}

/// Closed resume drift dimensions that must be checked before another action.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum ResumeDriftDimension {
    /// Active workspace identity or state changed.
    Workspace,
    /// A required file identity changed.
    File,
    /// Effective instructions changed.
    Instruction,
    /// Branch or detached-head state changed.
    Branch,
    /// Repository structural map changed.
    RepositoryMap,
    /// Citation identity set changed.
    Citation,
    /// Model artifact, profile, or runtime changed.
    Model,
    /// Effective permission profile changed.
    Permission,
    /// Deterministic policy changed.
    Policy,
}

/// Explicit user choice required after material drift.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResumeDriftDecision {
    /// Continue only after the changed state is explicitly accepted and re-checkpointed.
    Continue,
    /// Restart planning from current observations.
    Restart,
    /// Cancel the interrupted work.
    Cancel,
}
