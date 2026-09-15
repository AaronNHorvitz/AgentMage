//! Logical source-artifact ownership and retention over the existing payload store.
//!
//! These contracts add ownership, retention, and cleanup state for an ingested
//! source artifact. They name retained bytes only through an existing
//! [`RuntimeArtifactRef`], so no second physical store exists and no additional
//! [`RuntimeArtifactKind`] is introduced.

use crate::{
    ContextSensitivity, PolicyId, RuntimeArtifactCleanupState, RuntimeArtifactKind,
    RuntimeArtifactLifecycleState, RuntimeArtifactRef, RuntimeEventRetention, RuntimeRunId,
    SessionId, SourceArtifactId, SourceRetentionId, TaskId,
};

/// Identity of the one private encrypted content-addressed payload store.
pub const SOURCE_ARTIFACT_PAYLOAD_STORE: &str = "runtime-artifact-payload-store-v1";

/// Exact binding between one retained source artifact and its published payload.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceArtifactPayloadBinding {
    /// Path-free reference already published through the existing artifact store.
    pub reference: RuntimeArtifactRef,
    /// Existing closed semantic family assigned by that publication.
    pub kind: RuntimeArtifactKind,
}

/// Canonical logical ownership and retention state for one ingested source artifact.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceArtifactRetention {
    /// Contract schema version.
    pub schema_version: u16,
    /// Stable identity of this ownership and retention record.
    pub retention_id: SourceRetentionId,
    /// Ingested source artifact governed by this record.
    pub source_artifact_id: SourceArtifactId,
    /// Digest of the sealed source-artifact record this state governs.
    pub source_artifact_sha256: String,
    /// Identity of the single physical payload store; a second store is never admitted.
    pub payload_store: String,
    /// Owning local session; knowledge of this identity grants no access by itself.
    pub session_id: SessionId,
    /// Owning task.
    pub task_id: TaskId,
    /// Runtime run that admitted the source.
    pub owner_run_id: RuntimeRunId,
    /// Governing deterministic policy identity.
    pub policy_id: PolicyId,
    /// Digest of the exact governing policy revision.
    pub policy_sha256: String,
    /// Sensitivity assigned before any durable retention.
    pub sensitivity: ContextSensitivity,
    /// Exact retention assignment owned by canonical metadata.
    pub retention: RuntimeEventRetention,
    /// Durable payload binding, or `None` while the source remains memory-only.
    #[serde(deserialize_with = "crate::serialization::deserialize_required_option")]
    pub payload: Option<SourceArtifactPayloadBinding>,
    /// Current metadata lifecycle state.
    pub lifecycle: RuntimeArtifactLifecycleState,
    /// Content-free cleanup disposition derived from lifecycle and ownership.
    pub cleanup: RuntimeArtifactCleanupState,
    /// Number of logical owners that currently hold this source artifact.
    pub logical_reference_count: u32,
    /// Number of current checkpoints that name this source artifact.
    pub checkpoint_reference_count: u32,
    /// Stable content-free reason for the current state.
    pub reason_code: String,
    /// Trusted creation time in Unix epoch milliseconds.
    pub created_at_epoch_ms: u64,
    /// Last trusted lifecycle-transition time.
    pub updated_at_epoch_ms: u64,
    /// Digest of this canonical record with this field set to all zeroes.
    pub retention_sha256: String,
}
