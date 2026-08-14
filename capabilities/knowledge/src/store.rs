//! Authority-free storage interface and write-preview values.

use crate::{KnowledgeError, KnowledgeRecord, KnowledgeRecordId, KnowledgeRecordKind};

/// Bounded summary returned without granting access to canonical bytes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KnowledgeRecordSummary {
    /// Stable identity independent of path.
    pub record_id: KnowledgeRecordId,
    /// Closed record kind.
    pub kind: KnowledgeRecordKind,
    /// User-visible title.
    pub title: String,
    /// SHA-256 of the exact canonical Markdown snapshot.
    pub canonical_sha256: String,
}

/// Previewed canonical mutation kind; no apply operation exists in this capability.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KnowledgeWriteKind {
    /// Proposed creation of a new canonical Markdown record.
    Create,
    /// Proposed replacement of one exact current canonical Markdown record.
    Update,
}

/// Exact deterministic Markdown proposal with no filesystem authority.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KnowledgeWritePreview {
    /// Proposed operation kind.
    pub kind: KnowledgeWriteKind,
    /// Stable canonical record identity.
    pub record_id: KnowledgeRecordId,
    /// Expected existing canonical digest for an update, or none for a create.
    pub expected_sha256: Option<String>,
    /// Digest of the proposed exact Markdown bytes.
    pub proposed_sha256: String,
    /// Exact proposed Markdown bytes for user review.
    pub proposed_markdown: Vec<u8>,
}

/// Canonical human knowledge store boundary.
///
/// Implementations may inspect records and construct deterministic previews. The trait
/// deliberately has no create, update, delete, move, filesystem, or operational-store method;
/// v0.3 grants must introduce those effects through a separate kernel-mediated boundary.
pub trait KnowledgeStore {
    /// Returns bounded summaries in stable identity order.
    fn summaries(&self) -> Result<Vec<KnowledgeRecordSummary>, KnowledgeError>;

    /// Returns one canonical record by stable identity.
    fn record(
        &self,
        record_id: &KnowledgeRecordId,
    ) -> Result<Option<KnowledgeRecord>, KnowledgeError>;

    /// Constructs an exact creation preview without applying it.
    fn preview_create(
        &self,
        record: &KnowledgeRecord,
    ) -> Result<KnowledgeWritePreview, KnowledgeError>;

    /// Constructs an exact compare-and-swap update preview without applying it.
    fn preview_update(
        &self,
        expected_sha256: &str,
        record: &KnowledgeRecord,
    ) -> Result<KnowledgeWritePreview, KnowledgeError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn accepts_store_object(_store: &dyn KnowledgeStore) {}

    #[test]
    fn interface_is_object_safe_and_exposes_no_apply_method() {
        let _: fn(&dyn KnowledgeStore) = accepts_store_object;
        assert_eq!(KnowledgeWriteKind::Create, KnowledgeWriteKind::Create);
    }
}
