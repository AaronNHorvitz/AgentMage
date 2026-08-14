//! Neutral sealed workspace-projection wire contracts.

/// Kind of object included in a sealed workspace projection.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum SnapshotEntryKind {
    /// Exact regular file bytes.
    RegularFile,
    /// Content-free directory identity.
    Directory,
}

/// One exact object projected by a platform worker after path authorization.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SnapshotEntry {
    /// Canonical workspace-relative components.
    pub path: Vec<String>,
    /// Closed supported object kind.
    pub kind: SnapshotEntryKind,
    /// Exact regular-file bytes; directories require an empty vector.
    pub bytes: Vec<u8>,
    /// Stable executable-bit observation from the held object.
    pub executable: bool,
}

/// Bounded sealed projection supplied to pure read-only execution.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkspaceSnapshot {
    /// Exact projected objects. Execution canonicalizes their order.
    pub entries: Vec<SnapshotEntry>,
}
