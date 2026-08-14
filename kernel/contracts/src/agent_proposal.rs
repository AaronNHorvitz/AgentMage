//! Exact non-authoritative identity contract for one model proposal.

use crate::{
    ContextPacketId, CorrelationId, ModelRunId, PolicyId, ProposalId, RepositorySnapshotId,
    SessionId, TaskId, ToolCatalogId,
};

/// Versioned identity envelope for one inert model proposal candidate.
///
/// The proposal carries no operation, grant, destination, executable payload, or completion
/// claim. Its digest identifies separately validated closed proposal bytes at the model edge.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentProposal {
    /// Contract schema version.
    pub schema_version: u16,
    /// Unique proposal candidate identity.
    pub proposal_id: ProposalId,
    /// Owning local session.
    pub session_id: SessionId,
    /// Owning user-directed task.
    pub task_id: TaskId,
    /// Positive expected turn number.
    pub turn: u64,
    /// Exact bounded model run that produced the candidate.
    pub model_run_id: ModelRunId,
    /// Exact context packet supplied to that run.
    pub context_packet_id: ContextPacketId,
    /// Exact repository snapshot observed by the context builder.
    pub repository_snapshot_id: RepositorySnapshotId,
    /// Exact frozen tool catalog visible to the model edge.
    pub tool_catalog_id: ToolCatalogId,
    /// Exact deterministic policy revision governing admission.
    pub policy_id: PolicyId,
    /// Correlation identity shared with derived validation records.
    pub correlation_id: CorrelationId,
    /// Lowercase SHA-256 digest of the separately validated closed proposal bytes.
    pub proposal_sha256: String,
}
