//! Content-addressed backup, restore preview, and layout migration dry runs.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write;

use agentmage_kernel_contracts::WorkspacePath;
use sha2::{Digest, Sha256};

use crate::{
    KNOWLEDGE_SCHEMA_VERSION, KnowledgeError, KnowledgeRecordId, KnowledgeStore,
    PlainFolderEntryKind, PlainFolderKnowledgeStore, PlainFolderLayout, PlainFolderNoteInput,
    render_canonical_markdown,
};

const BACKUP_SCHEMA_VERSION: u16 = 1;
const MAX_BACKUP_FILES: usize = 100_000;
const MAX_BACKUP_BYTES: usize = 256 * 1024 * 1024;

/// One exact canonical note retained in an explicit backup bundle.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KnowledgeBackupEntry {
    /// Canonical workspace-relative source path.
    pub path: WorkspacePath,
    /// Exact SHA-256 of the canonical Markdown bytes.
    pub content_sha256: String,
    /// Exact canonical Markdown bytes.
    pub content: Vec<u8>,
}

/// Versioned explicit plain-folder backup value with no apply authority.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KnowledgeBackup {
    /// Backup schema version.
    pub schema_version: u16,
    /// Knowledge record schema version.
    pub knowledge_schema_version: u16,
    /// Complete entries in canonical path order.
    pub entries: Vec<KnowledgeBackupEntry>,
    /// SHA-256 over the exact backup manifest and bytes.
    pub backup_sha256: String,
}

/// Conflict-preserving restore action kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum KnowledgeRestoreActionKind {
    /// Destination is absent and may later be created after a grant.
    Create,
    /// Destination already contains the exact backed-up bytes.
    Unchanged,
    /// Destination differs and is never overwritten by this preview.
    Conflict,
}

/// One deterministic restore-preview action.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct KnowledgeRestoreAction {
    /// Exact destination path.
    pub path: WorkspacePath,
    /// Restore disposition.
    pub kind: KnowledgeRestoreActionKind,
    /// Backed-up canonical digest.
    pub backup_sha256: String,
    /// Current destination digest, when present.
    pub current_sha256: Option<String>,
}

/// Complete restore preview; no method applies these actions.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KnowledgeRestorePlan {
    /// Exact backup identity.
    pub backup_sha256: String,
    /// Digest of the current validated destination snapshot set.
    pub current_set_sha256: String,
    /// Actions in canonical path order.
    pub actions: Vec<KnowledgeRestoreAction>,
    /// True only when no conflict action exists.
    pub conflict_free: bool,
}

/// One identity-preserving destination note in a migration dry run.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct KnowledgeMigrationEntry {
    /// Stable identity preserved independently of either path.
    pub record_id: KnowledgeRecordId,
    /// Existing canonical path.
    pub source_path: WorkspacePath,
    /// Proposed path under the destination convention.
    pub destination_path: WorkspacePath,
    /// Existing exact Markdown digest.
    pub source_sha256: String,
    /// Proposed exact Markdown digest.
    pub destination_sha256: String,
    /// Whether convention changes alter rendered bytes.
    pub rendering_changes: bool,
}

/// Complete storage-layout migration preview with no move or write authority.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KnowledgeMigrationPlan {
    /// Digest of the exact source snapshot set.
    pub source_set_sha256: String,
    /// Digest of the exact proposed destination set.
    pub destination_set_sha256: String,
    /// Stable-identity migration entries.
    pub entries: Vec<KnowledgeMigrationEntry>,
    /// Explicit no-apply marker.
    pub preview_only: bool,
}

/// Builds a bounded backup only after every source note validates.
pub fn build_backup(
    layout: &PlainFolderLayout,
    mut inputs: Vec<PlainFolderNoteInput>,
) -> Result<KnowledgeBackup, KnowledgeError> {
    PlainFolderKnowledgeStore::from_snapshots(layout.clone(), inputs.clone())?;
    if inputs.len() > MAX_BACKUP_FILES
        || inputs
            .iter()
            .try_fold(0usize, |total, input| {
                total.checked_add(input.content.len())
            })
            .is_none_or(|total| total > MAX_BACKUP_BYTES)
    {
        return Err(KnowledgeError::InvalidMetadata);
    }
    inputs.sort_by(|left, right| left.path.cmp(&right.path));
    let entries: Vec<KnowledgeBackupEntry> = inputs
        .into_iter()
        .map(|input| KnowledgeBackupEntry {
            path: input.path,
            content_sha256: input.content_sha256,
            content: input.content,
        })
        .collect();
    Ok(KnowledgeBackup {
        schema_version: BACKUP_SCHEMA_VERSION,
        knowledge_schema_version: KNOWLEDGE_SCHEMA_VERSION,
        backup_sha256: backup_sha256(&entries),
        entries,
    })
}

/// Verifies every backup binding and reconstructs validated snapshots.
pub fn verify_backup(
    layout: &PlainFolderLayout,
    backup: &KnowledgeBackup,
) -> Result<Vec<PlainFolderNoteInput>, KnowledgeError> {
    if backup.schema_version != BACKUP_SCHEMA_VERSION
        || backup.knowledge_schema_version != KNOWLEDGE_SCHEMA_VERSION
        || backup.entries.len() > MAX_BACKUP_FILES
        || backup.backup_sha256 != backup_sha256(&backup.entries)
    {
        return Err(KnowledgeError::ContentDrift);
    }
    let inputs: Vec<PlainFolderNoteInput> = backup
        .entries
        .iter()
        .map(|entry| PlainFolderNoteInput {
            path: entry.path.clone(),
            entry_kind: PlainFolderEntryKind::RegularFile,
            cloud_synchronized: false,
            hidden: false,
            content_sha256: entry.content_sha256.clone(),
            content: entry.content.clone(),
        })
        .collect();
    PlainFolderKnowledgeStore::from_snapshots(layout.clone(), inputs.clone())?;
    Ok(inputs)
}

/// Compares one verified backup to a validated current destination snapshot.
pub fn preview_restore(
    layout: &PlainFolderLayout,
    backup: &KnowledgeBackup,
    current: Vec<PlainFolderNoteInput>,
) -> Result<KnowledgeRestorePlan, KnowledgeError> {
    let backup_inputs = verify_backup(layout, backup)?;
    PlainFolderKnowledgeStore::from_snapshots(layout.clone(), current.clone())?;
    let current_by_path: BTreeMap<WorkspacePath, String> = current
        .iter()
        .map(|input| (input.path.clone(), input.content_sha256.clone()))
        .collect();
    let actions: Vec<KnowledgeRestoreAction> = backup_inputs
        .iter()
        .map(|entry| {
            let current_sha256 = current_by_path.get(&entry.path).cloned();
            let kind = match current_sha256.as_deref() {
                None => KnowledgeRestoreActionKind::Create,
                Some(current) if current == entry.content_sha256 => {
                    KnowledgeRestoreActionKind::Unchanged
                }
                Some(_) => KnowledgeRestoreActionKind::Conflict,
            };
            KnowledgeRestoreAction {
                path: entry.path.clone(),
                kind,
                backup_sha256: entry.content_sha256.clone(),
                current_sha256,
            }
        })
        .collect();
    Ok(KnowledgeRestorePlan {
        backup_sha256: backup.backup_sha256.clone(),
        current_set_sha256: snapshot_set_sha256(&current),
        conflict_free: actions
            .iter()
            .all(|action| action.kind != KnowledgeRestoreActionKind::Conflict),
        actions,
    })
}

/// Previews a layout migration while preserving canonical values by identity.
pub fn preview_migration(
    source_layout: &PlainFolderLayout,
    destination_layout: &PlainFolderLayout,
    source: Vec<PlainFolderNoteInput>,
) -> Result<KnowledgeMigrationPlan, KnowledgeError> {
    destination_layout.validate()?;
    let source_set_sha256 = snapshot_set_sha256(&source);
    let source_store = PlainFolderKnowledgeStore::from_snapshots(source_layout.clone(), source)?;
    let mut destination_paths = BTreeSet::new();
    let mut destination_material = Vec::new();
    let mut entries = Vec::new();
    for summary in source_store.summaries()? {
        let record = source_store
            .record(&summary.record_id)?
            .ok_or(KnowledgeError::InvalidIdentity)?;
        let source_path = source_store
            .observed_path(&summary.record_id)
            .ok_or(KnowledgeError::InvalidPath)?
            .clone();
        let destination_path = destination_layout.proposed_path(&record)?;
        if !destination_paths.insert(destination_path.clone()) {
            return Err(KnowledgeError::DuplicateRecord);
        }
        let destination_markdown = render_canonical_markdown(destination_layout, &record)?;
        let destination_sha256 = sha256(&destination_markdown);
        destination_material.extend_from_slice(record.record_id.as_str().as_bytes());
        destination_material.push(0);
        destination_material.extend_from_slice(destination_sha256.as_bytes());
        destination_material.push(b'\n');
        entries.push(KnowledgeMigrationEntry {
            record_id: record.record_id,
            source_path,
            destination_path,
            source_sha256: summary.canonical_sha256.clone(),
            rendering_changes: summary.canonical_sha256 != destination_sha256,
            destination_sha256,
        });
    }
    entries.sort();
    Ok(KnowledgeMigrationPlan {
        source_set_sha256,
        destination_set_sha256: sha256(&destination_material),
        entries,
        preview_only: true,
    })
}

fn backup_sha256(entries: &[KnowledgeBackupEntry]) -> String {
    let mut material = Vec::new();
    material.extend_from_slice(b"agentmage-knowledge-backup-v1\0");
    for entry in entries {
        append_path(&mut material, &entry.path);
        material.extend_from_slice(entry.content_sha256.as_bytes());
        material.push(0);
        material.extend_from_slice(&(entry.content.len() as u64).to_be_bytes());
        material.extend_from_slice(&entry.content);
    }
    sha256(&material)
}

fn snapshot_set_sha256(inputs: &[PlainFolderNoteInput]) -> String {
    let mut ordered: Vec<&PlainFolderNoteInput> = inputs.iter().collect();
    ordered.sort_by(|left, right| left.path.cmp(&right.path));
    let mut material = Vec::new();
    for input in ordered {
        append_path(&mut material, &input.path);
        material.extend_from_slice(input.content_sha256.as_bytes());
        material.push(b'\n');
    }
    sha256(&material)
}

fn append_path(output: &mut Vec<u8>, path: &WorkspacePath) {
    output.extend_from_slice(path.workspace_id().as_str().as_bytes());
    output.push(0);
    for component in path.components() {
        output.extend_from_slice(component.as_str().as_bytes());
        output.push(0);
    }
}

fn sha256(value: &[u8]) -> String {
    let mut output = String::with_capacity(64);
    for byte in Sha256::digest(value) {
        write!(&mut output, "{byte:02x}").expect("writing to String cannot fail");
    }
    output
}

#[cfg(test)]
mod tests {
    use agentmage_kernel_contracts::{DataSensitivity, WorkspaceId};

    use super::*;
    use crate::{
        KnowledgeField, KnowledgeFilenameTemplate, KnowledgePrivacy, KnowledgeRecord,
        KnowledgeRecordKind, KnowledgeRetention, KnowledgeRetentionKind, knowledge_schema,
    };

    fn record(identity: &str, kind: KnowledgeRecordKind) -> KnowledgeRecord {
        KnowledgeRecord {
            schema_version: KNOWLEDGE_SCHEMA_VERSION,
            record_id: KnowledgeRecordId::parse(identity).expect("identity"),
            kind,
            title: format!("Synthetic {identity}"),
            privacy: KnowledgePrivacy::Private,
            sensitivity: DataSensitivity::Durable,
            retention: KnowledgeRetention {
                kind: KnowledgeRetentionKind::UntilSupersededOrDeleted,
                expires_at: None,
            },
            created_at: "2026-08-14T00:00:00Z".to_owned(),
            updated_at: "2026-08-14T00:00:00Z".to_owned(),
            last_verified_at: Some("2026-08-14T01:00:00Z".to_owned()),
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

    fn input(layout: &PlainFolderLayout, record: &KnowledgeRecord) -> PlainFolderNoteInput {
        let content = render_canonical_markdown(layout, record).expect("markdown");
        PlainFolderNoteInput {
            path: layout.proposed_path(record).expect("path"),
            entry_kind: PlainFolderEntryKind::RegularFile,
            cloud_synchronized: false,
            hidden: false,
            content_sha256: sha256(&content),
            content,
        }
    }

    #[test]
    fn backup_is_order_invariant_verified_and_content_addressed() {
        let layout = PlainFolderLayout::default_for(WorkspaceId::from_raw("workspace-knowledge"));
        let first = input(
            &layout,
            &record("knowledge-person-001", KnowledgeRecordKind::Person),
        );
        let second = input(
            &layout,
            &record("knowledge-project-001", KnowledgeRecordKind::Project),
        );
        let backup = build_backup(&layout, vec![first.clone(), second.clone()]).expect("backup");
        assert_eq!(
            backup,
            build_backup(&layout, vec![second, first]).expect("backup")
        );
        assert_eq!(verify_backup(&layout, &backup).expect("verify").len(), 2);
    }

    #[test]
    fn backup_version_manifest_and_content_mutation_fail_closed() {
        let layout = PlainFolderLayout::default_for(WorkspaceId::from_raw("workspace-knowledge"));
        let source = input(
            &layout,
            &record("knowledge-person-001", KnowledgeRecordKind::Person),
        );
        let base = build_backup(&layout, vec![source]).expect("backup");
        let mut changed = base.clone();
        changed.schema_version += 1;
        assert_eq!(
            verify_backup(&layout, &changed),
            Err(KnowledgeError::ContentDrift)
        );
        changed = base.clone();
        changed.entries[0].content.push(b'x');
        assert_eq!(
            verify_backup(&layout, &changed),
            Err(KnowledgeError::ContentDrift)
        );
    }

    #[test]
    fn restore_preview_never_overwrites_conflicting_content() {
        let layout = PlainFolderLayout::default_for(WorkspaceId::from_raw("workspace-knowledge"));
        let source_record = record("knowledge-person-001", KnowledgeRecordKind::Person);
        let source = input(&layout, &source_record);
        let backup = build_backup(&layout, vec![source.clone()]).expect("backup");
        assert_eq!(
            preview_restore(&layout, &backup, Vec::new())
                .expect("preview")
                .actions[0]
                .kind,
            KnowledgeRestoreActionKind::Create
        );
        assert_eq!(
            preview_restore(&layout, &backup, vec![source])
                .expect("preview")
                .actions[0]
                .kind,
            KnowledgeRestoreActionKind::Unchanged
        );
        let mut changed = source_record;
        changed.updated_at = "2026-08-14T02:00:00Z".to_owned();
        let plan = preview_restore(&layout, &backup, vec![input(&layout, &changed)]).expect("plan");
        assert!(!plan.conflict_free);
        assert_eq!(plan.actions[0].kind, KnowledgeRestoreActionKind::Conflict);
    }

    #[test]
    fn migration_preserves_identity_across_layout_conventions() {
        let workspace = WorkspaceId::from_raw("workspace-knowledge");
        let source_layout = PlainFolderLayout::default_for(workspace.clone());
        let mut destination_layout = PlainFolderLayout::default_for(workspace);
        destination_layout.tag_prefix = "tag:".to_owned();
        destination_layout.link_open = "<".to_owned();
        destination_layout.link_close = ">".to_owned();
        for template in &mut destination_layout.kind_paths {
            template.folder_components = vec!["migrated".to_owned()];
            template.filename = KnowledgeFilenameTemplate::KindAndRecordId;
        }
        let source_record = record("knowledge-person-001", KnowledgeRecordKind::Person);
        let plan = preview_migration(
            &source_layout,
            &destination_layout,
            vec![input(&source_layout, &source_record)],
        )
        .expect("migration");
        assert!(plan.preview_only);
        assert_eq!(plan.entries[0].record_id, source_record.record_id);
        assert_ne!(
            plan.entries[0].source_path,
            plan.entries[0].destination_path
        );
        assert!(plan.entries[0].rendering_changes);
    }
}
