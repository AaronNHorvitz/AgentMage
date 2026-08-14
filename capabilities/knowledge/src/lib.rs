#![deny(missing_docs)]
#![forbid(unsafe_code)]
//! Canonical user-owned Markdown knowledge-domain contracts without ambient authority.

mod domain;
mod index;
mod lifecycle;
mod operations;
mod plain_folder;
mod schema;
mod store;

pub use domain::{
    KnowledgeError, KnowledgeField, KnowledgeLink, KnowledgeLinkKind, KnowledgePrivacy,
    KnowledgeRecord, KnowledgeRecordId, KnowledgeRecordKind, KnowledgeRetention,
    KnowledgeRetentionKind, validate_record,
};
pub use index::{KnowledgeIndex, KnowledgeIndexError, KnowledgeIndexHit, KnowledgeIndexReport};
pub use lifecycle::{
    KnowledgeBackup, KnowledgeBackupEntry, KnowledgeMigrationEntry, KnowledgeMigrationPlan,
    KnowledgeRestoreAction, KnowledgeRestoreActionKind, KnowledgeRestorePlan, build_backup,
    preview_migration, preview_restore, verify_backup,
};
pub use operations::{
    KnowledgeDashboard, KnowledgeDuplicate, KnowledgeDuplicateReason, KnowledgeExport,
    KnowledgeImportReport, KnowledgeRelationship, build_dashboard, build_json_lines_export,
    validate_import,
};
pub use plain_folder::{
    KnowledgeFilenameTemplate, KnowledgeKindPathTemplate, PlainFolderEntryKind,
    PlainFolderKnowledgeStore, PlainFolderLayout, PlainFolderNoteInput, parse_canonical_markdown,
    render_canonical_markdown,
};
pub use schema::{
    KnowledgeRecordSchema, knowledge_schema, knowledge_schemas, verify_schema_registry,
};
pub use store::{
    KnowledgeRecordSummary, KnowledgeStore, KnowledgeWriteKind, KnowledgeWritePreview,
};

/// Stable component identity used by diagnostics and build verification.
pub const COMPONENT_ID: &str = "capability-knowledge";

/// Immutable schema version for the first canonical knowledge-domain family.
pub const KNOWLEDGE_SCHEMA_VERSION: u16 = 1;

/// Returns the identity of the contracts consumed by this capability.
#[must_use]
pub const fn contract_component_id() -> &'static str {
    agentmage_kernel_contracts::COMPONENT_ID
}

#[cfg(test)]
mod tests {
    use super::{COMPONENT_ID, KNOWLEDGE_SCHEMA_VERSION, contract_component_id};

    #[test]
    fn capability_depends_only_on_contracts() {
        assert_eq!(COMPONENT_ID, "capability-knowledge");
        assert_eq!(KNOWLEDGE_SCHEMA_VERSION, 1);
        assert_eq!(contract_component_id(), "kernel-contracts");
    }
}
