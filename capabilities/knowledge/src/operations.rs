//! Deterministic relationship, import, dashboard, and export operations.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write;

use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::{
    KNOWLEDGE_SCHEMA_VERSION, KnowledgeError, KnowledgeLinkKind, KnowledgeRecord,
    KnowledgeRecordId, KnowledgeRecordKind, validate_record,
};

const MAX_IMPORT_RECORDS: usize = 100_000;
const MAX_EXPORT_BYTES: usize = 256 * 1024 * 1024;

/// Deterministic duplicate reason detected before import.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum KnowledgeDuplicateReason {
    /// Two records claim the same stable identity.
    StableIdentity,
    /// Two records of the same kind have an exact normalized title.
    KindAndTitle,
}

/// One ordered duplicate candidate pair.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(deny_unknown_fields)]
pub struct KnowledgeDuplicate {
    /// Lexically first identity.
    pub first_id: KnowledgeRecordId,
    /// Lexically second identity, equal for duplicate stable identities.
    pub second_id: KnowledgeRecordId,
    /// Deterministic duplicate reason.
    pub reason: KnowledgeDuplicateReason,
}

/// One resolved relationship between stable identities.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(deny_unknown_fields)]
pub struct KnowledgeRelationship {
    /// Source record identity.
    pub source_id: KnowledgeRecordId,
    /// Typed relationship.
    pub kind: KnowledgeLinkKind,
    /// Resolved target record identity.
    pub target_id: KnowledgeRecordId,
}

/// Complete pre-import validation result.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct KnowledgeImportReport {
    /// Knowledge schema version.
    pub schema_version: u16,
    /// Number of validated canonical records.
    pub record_count: u64,
    /// Complete stable identities in lexical order.
    pub record_ids: Vec<KnowledgeRecordId>,
    /// Resolved typed relationships in lexical order.
    pub relationships: Vec<KnowledgeRelationship>,
    /// Duplicate candidates; empty for an admitted import.
    pub duplicates: Vec<KnowledgeDuplicate>,
    /// Digest over all canonical record JSON in stable identity order.
    pub import_sha256: String,
}

/// Deterministic user-visible dashboard derived from canonical records.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KnowledgeDashboard {
    /// Number of represented canonical records.
    pub record_count: u64,
    /// Exact derived Markdown bytes.
    pub markdown: Vec<u8>,
    /// SHA-256 of the derived Markdown bytes.
    pub markdown_sha256: String,
    /// Explicit non-authoritative classification.
    pub canonical: bool,
}

/// Explicit JSON Lines export derived from canonical Markdown.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KnowledgeExport {
    /// Number of exported canonical records.
    pub record_count: u64,
    /// Exact versioned JSON Lines bytes.
    pub json_lines: Vec<u8>,
    /// SHA-256 of the exact export bytes.
    pub export_sha256: String,
    /// Explicit non-authoritative classification.
    pub canonical: bool,
}

/// Validates a complete import set before any canonical write can be proposed.
pub fn validate_import(
    records: &[KnowledgeRecord],
) -> Result<KnowledgeImportReport, KnowledgeError> {
    if records.len() > MAX_IMPORT_RECORDS {
        return Err(KnowledgeError::InvalidMetadata);
    }
    for record in records {
        validate_record(record)?;
    }
    let mut sorted: Vec<&KnowledgeRecord> = records.iter().collect();
    sorted.sort_by(|left, right| left.record_id.cmp(&right.record_id));
    let duplicates = detect_duplicates(&sorted);
    if !duplicates.is_empty() {
        return Err(KnowledgeError::DuplicateRecord);
    }
    let identities: BTreeSet<&KnowledgeRecordId> =
        sorted.iter().map(|record| &record.record_id).collect();
    let mut relationships = Vec::new();
    for record in &sorted {
        for link in &record.links {
            if !identities.contains(&link.target_id) {
                return Err(KnowledgeError::UnresolvedLink);
            }
            relationships.push(KnowledgeRelationship {
                source_id: record.record_id.clone(),
                kind: link.kind,
                target_id: link.target_id.clone(),
            });
        }
    }
    relationships.sort();
    let canonical = canonical_records_json(&sorted)?;
    Ok(KnowledgeImportReport {
        schema_version: KNOWLEDGE_SCHEMA_VERSION,
        record_count: sorted.len() as u64,
        record_ids: sorted
            .iter()
            .map(|record| record.record_id.clone())
            .collect(),
        relationships,
        duplicates,
        import_sha256: sha256(&canonical),
    })
}

/// Builds a stable Markdown dashboard without modifying any canonical record.
pub fn build_dashboard(records: &[KnowledgeRecord]) -> Result<KnowledgeDashboard, KnowledgeError> {
    let report = validate_import(records)?;
    let by_id: BTreeMap<&KnowledgeRecordId, &KnowledgeRecord> = records
        .iter()
        .map(|record| (&record.record_id, record))
        .collect();
    let mut markdown = String::from("# Knowledge Workspace\n\n");
    markdown.push_str("> Derived view. Canonical records remain the source of truth.\n");
    for kind in crate::knowledge_schemas().iter().map(|schema| schema.kind) {
        let selected: Vec<&KnowledgeRecord> = report
            .record_ids
            .iter()
            .filter_map(|identity| by_id.get(identity).copied())
            .filter(|record| record.kind == kind)
            .collect();
        if selected.is_empty() {
            continue;
        }
        write!(&mut markdown, "\n## {}\n", kind_heading(kind))
            .expect("writing to String cannot fail");
        for record in selected {
            writeln!(
                &mut markdown,
                "- [[{}]] - {}",
                record.record_id.as_str(),
                record.title
            )
            .expect("writing to String cannot fail");
        }
    }
    let bytes = markdown.into_bytes();
    Ok(KnowledgeDashboard {
        record_count: report.record_count,
        markdown_sha256: sha256(&bytes),
        markdown: bytes,
        canonical: false,
    })
}

/// Builds a stable versioned JSON Lines export with no import or co-authority behavior.
pub fn build_json_lines_export(
    records: &[KnowledgeRecord],
) -> Result<KnowledgeExport, KnowledgeError> {
    let report = validate_import(records)?;
    let by_id: BTreeMap<&KnowledgeRecordId, &KnowledgeRecord> = records
        .iter()
        .map(|record| (&record.record_id, record))
        .collect();
    let header = ExportHeader {
        schema_version: 1,
        record_type: "agentmage_knowledge_export",
        canonical: false,
        record_count: report.record_count,
        canonical_set_sha256: &report.import_sha256,
    };
    let mut output = serde_json::to_vec(&header).map_err(|_| KnowledgeError::InvalidMetadata)?;
    output.push(b'\n');
    for identity in &report.record_ids {
        let envelope = ExportRecord {
            schema_version: 1,
            record_type: "agentmage_knowledge_record",
            canonical: false,
            record: by_id
                .get(identity)
                .copied()
                .ok_or(KnowledgeError::InvalidIdentity)?,
        };
        let line = serde_json::to_vec(&envelope).map_err(|_| KnowledgeError::InvalidMetadata)?;
        if output.len().saturating_add(line.len()).saturating_add(1) > MAX_EXPORT_BYTES {
            return Err(KnowledgeError::InvalidMetadata);
        }
        output.extend_from_slice(&line);
        output.push(b'\n');
    }
    Ok(KnowledgeExport {
        record_count: report.record_count,
        export_sha256: sha256(&output),
        json_lines: output,
        canonical: false,
    })
}

fn detect_duplicates(records: &[&KnowledgeRecord]) -> Vec<KnowledgeDuplicate> {
    let mut duplicates = Vec::new();
    let mut identities: BTreeMap<&KnowledgeRecordId, &KnowledgeRecordId> = BTreeMap::new();
    let mut titles: BTreeMap<(KnowledgeRecordKind, String), &KnowledgeRecordId> = BTreeMap::new();
    for record in records {
        if let Some(first) = identities.insert(&record.record_id, &record.record_id) {
            duplicates.push(KnowledgeDuplicate {
                first_id: first.clone(),
                second_id: record.record_id.clone(),
                reason: KnowledgeDuplicateReason::StableIdentity,
            });
        }
        let title_key = (record.kind, normalize_title(&record.title));
        if let Some(first) = titles.insert(title_key, &record.record_id) {
            duplicates.push(KnowledgeDuplicate {
                first_id: first.clone(),
                second_id: record.record_id.clone(),
                reason: KnowledgeDuplicateReason::KindAndTitle,
            });
        }
    }
    duplicates.sort();
    duplicates.dedup();
    duplicates
}

fn normalize_title(value: &str) -> String {
    value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

fn canonical_records_json(records: &[&KnowledgeRecord]) -> Result<Vec<u8>, KnowledgeError> {
    serde_json::to_vec(records).map_err(|_| KnowledgeError::InvalidMetadata)
}

fn sha256(value: &[u8]) -> String {
    let mut output = String::with_capacity(64);
    for byte in Sha256::digest(value) {
        write!(&mut output, "{byte:02x}").expect("writing to String cannot fail");
    }
    output
}

fn kind_heading(kind: KnowledgeRecordKind) -> &'static str {
    match kind {
        KnowledgeRecordKind::Person => "People",
        KnowledgeRecordKind::Organization => "Organizations",
        KnowledgeRecordKind::Project => "Projects",
        KnowledgeRecordKind::Meeting => "Meetings",
        KnowledgeRecordKind::Task => "Tasks",
        KnowledgeRecordKind::Decision => "Decisions",
        KnowledgeRecordKind::Commitment => "Commitments",
        KnowledgeRecordKind::Document => "Documents",
        KnowledgeRecordKind::Correspondence => "Correspondence",
        KnowledgeRecordKind::Deadline => "Deadlines",
        KnowledgeRecordKind::Approval => "Approvals",
        KnowledgeRecordKind::Risk => "Risks",
        KnowledgeRecordKind::Question => "Questions",
        KnowledgeRecordKind::Handoff => "Handoffs",
    }
}

#[derive(Serialize)]
struct ExportHeader<'a> {
    schema_version: u16,
    record_type: &'static str,
    canonical: bool,
    record_count: u64,
    canonical_set_sha256: &'a str,
}

#[derive(Serialize)]
struct ExportRecord<'a> {
    schema_version: u16,
    record_type: &'static str,
    canonical: bool,
    record: &'a KnowledgeRecord,
}

#[cfg(test)]
mod tests {
    use agentmage_kernel_contracts::DataSensitivity;

    use super::*;
    use crate::{
        KnowledgeField, KnowledgeLink, KnowledgePrivacy, KnowledgeRetention,
        KnowledgeRetentionKind, knowledge_schema,
    };

    fn record(identity: &str, kind: KnowledgeRecordKind, title: &str) -> KnowledgeRecord {
        KnowledgeRecord {
            schema_version: KNOWLEDGE_SCHEMA_VERSION,
            record_id: KnowledgeRecordId::parse(identity).expect("identity"),
            kind,
            title: title.to_owned(),
            privacy: KnowledgePrivacy::Private,
            sensitivity: DataSensitivity::Durable,
            retention: KnowledgeRetention {
                kind: KnowledgeRetentionKind::UntilSupersededOrDeleted,
                expires_at: None,
            },
            created_at: "2026-08-14T00:00:00Z".to_owned(),
            updated_at: "2026-08-14T00:00:00Z".to_owned(),
            last_verified_at: None,
            fields: knowledge_schema(kind)
                .required_fields
                .iter()
                .map(|name| KnowledgeField {
                    name: (*name).to_owned(),
                    value: "fixture".to_owned(),
                })
                .collect(),
            links: Vec::new(),
            tags: vec!["fixture".to_owned()],
            evidence: Vec::new(),
        }
    }

    #[test]
    fn import_resolves_relationships_and_is_input_order_invariant() {
        let person = record(
            "knowledge-person-001",
            KnowledgeRecordKind::Person,
            "Person",
        );
        let mut project = record(
            "knowledge-project-001",
            KnowledgeRecordKind::Project,
            "Project",
        );
        project.links.push(KnowledgeLink {
            kind: KnowledgeLinkKind::OwnedBy,
            target_id: person.record_id.clone(),
        });
        let first = validate_import(&[person.clone(), project.clone()]).expect("import");
        let second = validate_import(&[project, person]).expect("import");
        assert_eq!(first, second);
        assert_eq!(first.relationships.len(), 1);
        assert!(first.duplicates.is_empty());
    }

    #[test]
    fn duplicate_identity_title_and_unresolved_link_block_import() {
        let first = record(
            "knowledge-person-001",
            KnowledgeRecordKind::Person,
            "Same Name",
        );
        let same_id = first.clone();
        assert_eq!(
            validate_import(&[first.clone(), same_id]),
            Err(KnowledgeError::DuplicateRecord)
        );
        let second = record(
            "knowledge-person-002",
            KnowledgeRecordKind::Person,
            " same   name ",
        );
        assert_eq!(
            validate_import(&[first.clone(), second]),
            Err(KnowledgeError::DuplicateRecord)
        );
        let mut unresolved = first;
        unresolved.links.push(KnowledgeLink {
            kind: KnowledgeLinkKind::Related,
            target_id: KnowledgeRecordId::parse("knowledge-absent").expect("identity"),
        });
        assert_eq!(
            validate_import(&[unresolved]),
            Err(KnowledgeError::UnresolvedLink)
        );
    }

    #[test]
    fn dashboard_and_json_lines_are_stable_derived_views() {
        let person = record(
            "knowledge-person-001",
            KnowledgeRecordKind::Person,
            "A Person",
        );
        let project = record(
            "knowledge-project-001",
            KnowledgeRecordKind::Project,
            "A Project",
        );
        let first = vec![person.clone(), project.clone()];
        let second = vec![project, person];
        let dashboard = build_dashboard(&first).expect("dashboard");
        assert_eq!(dashboard, build_dashboard(&second).expect("dashboard"));
        assert!(!dashboard.canonical);
        assert!(String::from_utf8_lossy(&dashboard.markdown).contains("Derived view"));
        let export = build_json_lines_export(&first).expect("export");
        assert_eq!(export, build_json_lines_export(&second).expect("export"));
        assert!(!export.canonical);
        let lines: Vec<&[u8]> = export
            .json_lines
            .split(|byte| *byte == b'\n')
            .filter(|line| !line.is_empty())
            .collect();
        assert_eq!(lines.len(), 3);
        for line in lines {
            let value: serde_json::Value = serde_json::from_slice(line).expect("json line");
            assert_eq!(value["canonical"], false);
        }
    }
}
