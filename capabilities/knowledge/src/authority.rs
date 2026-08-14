//! Field-level ownership and rebuild policy for the knowledge domain.

use std::collections::BTreeSet;

use agentmage_kernel_contracts::DataSensitivity;

use crate::{KnowledgeRecordKind, knowledge_schemas};

/// Sole owner of one stored or derived knowledge field.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum KnowledgeDataOwner {
    /// User-owned canonical Markdown.
    CanonicalMarkdown,
    /// Disposable SQLite knowledge index.
    DerivedIndex,
    /// Explicit JSON Lines or dashboard export.
    DerivedExport,
    /// In-process parsing or preview state.
    Temporary,
}

/// Closed storage rule for one knowledge field.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KnowledgeStorageRule {
    /// Ordinary user-owned Markdown under user filesystem controls.
    UserOwnedMarkdown,
    /// Disposable local SQLite projection with no independent authority.
    DisposableSqlite,
    /// User-directed derived artifact that cannot be imported as authority.
    ExplicitExport,
    /// Process memory only.
    ProcessMemory,
}

/// Complete policy for one canonical or derived knowledge field.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KnowledgeFieldPolicy {
    /// Stable policy identity.
    pub field_id: String,
    /// Sole authority owner.
    pub owner: KnowledgeDataOwner,
    /// Purpose for retaining or deriving the field.
    pub purpose: &'static str,
    /// Shared data-sensitivity class.
    pub sensitivity: DataSensitivity,
    /// Exact storage rule.
    pub storage: KnowledgeStorageRule,
    /// Whether AgentMage encrypts this representation.
    pub product_encrypted: bool,
    /// Visible retention rule.
    pub retention: &'static str,
    /// Visible correction behavior.
    pub correction: &'static str,
    /// Visible export behavior.
    pub export: &'static str,
    /// Visible deletion behavior.
    pub deletion: &'static str,
    /// Exact recovery or rebuild source.
    pub recovery_source: &'static str,
}

const COMMON_FIELDS: &[&str] = &[
    "schema_version",
    "record_id",
    "kind",
    "title",
    "privacy",
    "sensitivity",
    "retention",
    "created_at",
    "updated_at",
    "last_verified_at",
    "links",
    "tags",
    "evidence",
];

/// Builds the complete field-level policy registry in stable identity order.
#[must_use]
pub fn knowledge_data_dictionary() -> Vec<KnowledgeFieldPolicy> {
    let mut policies = Vec::new();
    for field in COMMON_FIELDS {
        policies.push(canonical_policy(format!("record.common.{field}")));
    }
    for schema in knowledge_schemas() {
        for field in schema.required_fields.iter().chain(schema.optional_fields) {
            policies.push(canonical_policy(format!(
                "record.{}.{}",
                kind_wire(schema.kind),
                field
            )));
        }
    }
    for field in [
        "canonical_record_sha256",
        "kind",
        "record_id",
        "record_json",
        "relationship",
        "title",
    ] {
        policies.push(KnowledgeFieldPolicy {
            field_id: format!("index.{field}"),
            owner: KnowledgeDataOwner::DerivedIndex,
            purpose: "Bounded local search and relationship projection",
            sensitivity: DataSensitivity::Durable,
            storage: KnowledgeStorageRule::DisposableSqlite,
            product_encrypted: false,
            retention: "Until index deletion or complete rebuild",
            correction: "Never edited; rebuild from canonical Markdown",
            export: "Excluded unless generated through the explicit export operation",
            deletion: "Delete the complete disposable index",
            recovery_source: "Validated canonical Markdown records",
        });
    }
    for field in ["dashboard", "json_lines"] {
        policies.push(KnowledgeFieldPolicy {
            field_id: format!("export.{field}"),
            owner: KnowledgeDataOwner::DerivedExport,
            purpose: "Explicit user-directed portable or summary view",
            sensitivity: DataSensitivity::Durable,
            storage: KnowledgeStorageRule::ExplicitExport,
            product_encrypted: false,
            retention: "User-selected destination policy",
            correction: "Regenerate from corrected canonical Markdown",
            export: "This field is itself an explicit non-authoritative export",
            deletion: "Delete the artifact without changing canonical records",
            recovery_source: "Validated canonical Markdown records",
        });
    }
    policies.push(KnowledgeFieldPolicy {
        field_id: "temporary.preview_bytes".to_owned(),
        owner: KnowledgeDataOwner::Temporary,
        purpose: "Exact local user review before a later grant",
        sensitivity: DataSensitivity::Ephemeral,
        storage: KnowledgeStorageRule::ProcessMemory,
        product_encrypted: false,
        retention: "End of preview or process",
        correction: "Regenerate from the current canonical record proposal",
        export: "Never exported implicitly",
        deletion: "Discard from process memory",
        recovery_source: "No recovery; regenerate from current inputs",
    });
    policies.sort_by(|left, right| left.field_id.cmp(&right.field_id));
    policies
}

/// Verifies unique ownership and exact canonical or derived rebuild rules.
#[must_use]
pub fn verify_data_dictionary() -> bool {
    let policies = knowledge_data_dictionary();
    let identities: BTreeSet<&str> = policies
        .iter()
        .map(|policy| policy.field_id.as_str())
        .collect();
    identities.len() == policies.len()
        && policies.iter().all(|policy| {
            !policy.field_id.is_empty()
                && !policy.purpose.is_empty()
                && !policy.retention.is_empty()
                && !policy.correction.is_empty()
                && !policy.export.is_empty()
                && !policy.deletion.is_empty()
                && !policy.recovery_source.is_empty()
                && match policy.owner {
                    KnowledgeDataOwner::CanonicalMarkdown => {
                        policy.storage == KnowledgeStorageRule::UserOwnedMarkdown
                            && policy.sensitivity == DataSensitivity::Durable
                            && policy.recovery_source == "Verified plain-folder backup"
                    }
                    KnowledgeDataOwner::DerivedIndex => {
                        policy.storage == KnowledgeStorageRule::DisposableSqlite
                            && policy.recovery_source == "Validated canonical Markdown records"
                    }
                    KnowledgeDataOwner::DerivedExport => {
                        policy.storage == KnowledgeStorageRule::ExplicitExport
                            && policy.recovery_source == "Validated canonical Markdown records"
                    }
                    KnowledgeDataOwner::Temporary => {
                        policy.storage == KnowledgeStorageRule::ProcessMemory
                            && policy.sensitivity == DataSensitivity::Ephemeral
                    }
                }
        })
}

fn canonical_policy(field_id: String) -> KnowledgeFieldPolicy {
    KnowledgeFieldPolicy {
        field_id,
        owner: KnowledgeDataOwner::CanonicalMarkdown,
        purpose: "Inspectable user-owned human knowledge",
        sensitivity: DataSensitivity::Durable,
        storage: KnowledgeStorageRule::UserOwnedMarkdown,
        product_encrypted: false,
        retention: "Record retention template, user supersession, or explicit deletion",
        correction: "Exact compare-and-swap Markdown update preview",
        export: "Included only in explicit dashboard, JSON Lines, backup, or migration preview",
        deletion: "Later grant must preview and approve canonical Markdown deletion",
        recovery_source: "Verified plain-folder backup",
    }
}

fn kind_wire(kind: KnowledgeRecordKind) -> &'static str {
    match kind {
        KnowledgeRecordKind::Person => "person",
        KnowledgeRecordKind::Organization => "organization",
        KnowledgeRecordKind::Project => "project",
        KnowledgeRecordKind::Meeting => "meeting",
        KnowledgeRecordKind::Task => "task",
        KnowledgeRecordKind::Decision => "decision",
        KnowledgeRecordKind::Commitment => "commitment",
        KnowledgeRecordKind::Document => "document",
        KnowledgeRecordKind::Correspondence => "correspondence",
        KnowledgeRecordKind::Deadline => "deadline",
        KnowledgeRecordKind::Approval => "approval",
        KnowledgeRecordKind::Risk => "risk",
        KnowledgeRecordKind::Question => "question",
        KnowledgeRecordKind::Handoff => "handoff",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_field_has_one_complete_owner_storage_retention_and_rebuild_policy() {
        assert!(verify_data_dictionary());
        let policies = knowledge_data_dictionary();
        assert!(policies.iter().any(|policy| {
            policy.field_id == "record.person.name"
                && policy.owner == KnowledgeDataOwner::CanonicalMarkdown
        }));
        assert!(policies.iter().any(|policy| {
            policy.field_id == "index.record_json"
                && policy.owner == KnowledgeDataOwner::DerivedIndex
        }));
    }

    #[test]
    fn no_operational_owner_or_operational_recovery_source_exists() {
        for policy in knowledge_data_dictionary() {
            assert!(!policy.field_id.starts_with("operational."));
            assert!(!policy.recovery_source.contains("operational"));
        }
    }
}
