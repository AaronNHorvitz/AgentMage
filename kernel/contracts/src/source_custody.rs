//! Logical source-artifact ownership and retention over the existing artifact store.
//!
//! These contracts add no physical store. Every retained source payload is one object in
//! the existing encrypted content-addressed runtime artifact store, published under one
//! already declared [`RuntimeArtifactKind`] member and one already declared
//! [`RuntimeEventRetention`] assignment.

use crate::{
    RuntimeArtifactId, RuntimeArtifactKind, RuntimeEventRetention, RuntimeRunId, SessionId, TaskId,
};

/// Contract schema version for the source-artifact custody family.
pub const SOURCE_ARTIFACT_CUSTODY_SCHEMA_VERSION: u16 = 1;

/// Closed artifact family reused for every retained source payload.
///
/// Source custody reuses one published `RuntimeArtifactKind` member rather than widening
/// that closed family, so a retained source payload is indistinguishable from any other
/// object in the same store.
pub const SOURCE_CUSTODY_ARTIFACT_KIND: RuntimeArtifactKind = RuntimeArtifactKind::GeneratedFile;

/// Physical backend permitted to retain source-artifact payload bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceCustodyBackend {
    /// The one existing encrypted content-addressed runtime artifact store.
    RuntimeArtifactStore,
}

/// Current logical custody state of one source artifact.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceCustodyState {
    /// No durable payload exists; no bytes were admitted to the store.
    NotRetained,
    /// The logical reference is current and its payload may be opened under policy.
    Active,
    /// The reference is retained for inspection but returns no payload bytes.
    Quarantined,
    /// The owner released the logical reference.
    Released,
    /// Canonical metadata records a completed payload deletion.
    Deleted,
}

/// Exact logical binding between one source artifact and its retained payload.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceCustodyBinding {
    /// Logical artifact identity in the existing store; this is not path authority.
    pub runtime_artifact_id: RuntimeArtifactId,
    /// Lowercase SHA-256 payload content address held by that store.
    pub payload_sha256: String,
    /// Exact retained payload size.
    pub byte_size: u64,
}

/// Logical ownership and retention assignment for one source artifact.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceArtifactCustody {
    /// Contract schema version.
    pub schema_version: u16,
    /// Owning source-artifact identity.
    pub source_artifact_id: String,
    /// Backend that retains payload bytes; exactly one store is representable.
    pub backend: SourceCustodyBackend,
    /// Reused closed artifact family.
    pub artifact_kind: RuntimeArtifactKind,
    /// Exact payload binding; absent while nothing durable is retained.
    #[serde(deserialize_with = "crate::serialization::deserialize_required_option")]
    pub binding: Option<SourceCustodyBinding>,
    /// Owning local session; knowledge of this identity grants no access by itself.
    pub owner_session_id: SessionId,
    /// Owning task.
    pub owner_task_id: TaskId,
    /// Owning runtime run.
    pub owner_run_id: RuntimeRunId,
    /// Exact retention assignment reusing the canonical runtime retention family.
    pub retention: RuntimeEventRetention,
    /// Current logical custody state.
    pub state: SourceCustodyState,
    /// Whether a current durable checkpoint roots this reference against release.
    pub checkpoint_rooted: bool,
    /// Stable content-free reason for a quarantined, released, or deleted reference.
    #[serde(deserialize_with = "crate::serialization::deserialize_required_option")]
    pub release_reason_code: Option<String>,
}
