//! Persisted agent restart-reconciliation input contract.

use crate::{
    AgentStateKind, AuthorityTransactionRecord, CapabilityGrant, PolicyId, Receipt,
    RepositorySnapshotId, TaskId,
};

/// Complete persisted facts required before an interrupted agent may transition again.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentRestartSnapshot {
    /// Contract schema version.
    pub schema_version: u16,
    /// Exact current task identity.
    pub task_id: TaskId,
    /// Exact repository snapshot identity.
    pub repository_snapshot_id: RepositorySnapshotId,
    /// Exact deterministic policy identity.
    pub policy_id: PolicyId,
    /// Lowercase SHA-256 digest of the current policy revision.
    pub policy_sha256: String,
    /// Exact selected configuration profile identity.
    pub selected_profile_id: String,
    /// Lowercase SHA-256 digest of the selected profile revision.
    pub selected_profile_sha256: String,
    /// Persisted agent state at interruption.
    pub agent_state: AgentStateKind,
    /// Persisted monotonic agent-state revision.
    pub agent_state_revision: u64,
    /// Current revision of every authority transaction owned by the task.
    pub authority_transactions: Vec<AuthorityTransactionRecord>,
    /// Current revision of every capability grant owned by the task.
    pub grants: Vec<CapabilityGrant>,
    /// Complete retained receipt chain visible to the task recovery boundary.
    pub receipts: Vec<Receipt>,
}
