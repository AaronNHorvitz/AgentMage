//! Deterministic plain-folder Markdown adapter over authorized snapshots.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write;

use agentmage_kernel_contracts::{WorkspaceId, WorkspacePath};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{
    KnowledgeError, KnowledgeRecord, KnowledgeRecordId, KnowledgeRecordKind,
    KnowledgeRecordSummary, KnowledgeStore, KnowledgeWriteKind, KnowledgeWritePreview,
    knowledge_schemas, validate_record,
};

const MAX_NOTE_BYTES: usize = 4 * 1024 * 1024;
const MAX_NOTES: usize = 100_000;
const MAX_TEMPLATE_COMPONENTS: usize = 16;
const MAX_FRONTMATTER_KEY_BYTES: usize = 64;

/// Closed filename strategy used only when proposing a new canonical path.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KnowledgeFilenameTemplate {
    /// Use the stable identity followed by `.md`.
    RecordId,
    /// Prefix the stable identity with the record kind.
    KindAndRecordId,
}

/// Configurable canonical creation location for one record kind.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KnowledgeKindPathTemplate {
    /// Closed record kind.
    pub kind: KnowledgeRecordKind,
    /// Canonical workspace-relative folder components.
    pub folder_components: Vec<String>,
    /// Filename strategy.
    pub filename: KnowledgeFilenameTemplate,
}

/// Complete plain-folder convention without a host path or filesystem authority.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlainFolderLayout {
    /// Approved workspace identity.
    pub workspace_id: WorkspaceId,
    /// Exact frontmatter key containing the canonical record object.
    pub frontmatter_record_key: String,
    /// Prefix used when rendering searchable tags.
    pub tag_prefix: String,
    /// Opening wiki-link delimiter.
    pub link_open: String,
    /// Closing wiki-link delimiter.
    pub link_close: String,
    /// Stable identifier prefix required by this schema generation.
    pub identifier_prefix: String,
    /// Explicit per-kind path templates.
    pub kind_paths: Vec<KnowledgeKindPathTemplate>,
}

impl PlainFolderLayout {
    /// Returns a storage-neutral default layout with one folder per record kind.
    #[must_use]
    pub fn default_for(workspace_id: WorkspaceId) -> Self {
        Self {
            workspace_id,
            frontmatter_record_key: "agentmage_record".to_owned(),
            tag_prefix: "#".to_owned(),
            link_open: "[[".to_owned(),
            link_close: "]]".to_owned(),
            identifier_prefix: "knowledge-".to_owned(),
            kind_paths: knowledge_schemas()
                .iter()
                .map(|schema| KnowledgeKindPathTemplate {
                    kind: schema.kind,
                    folder_components: vec![kind_wire(schema.kind).to_owned()],
                    filename: KnowledgeFilenameTemplate::RecordId,
                })
                .collect(),
        }
    }

    /// Validates complete kind coverage and all path/rendering templates.
    pub fn validate(&self) -> Result<(), KnowledgeError> {
        if self.frontmatter_record_key.is_empty()
            || self.frontmatter_record_key.len() > MAX_FRONTMATTER_KEY_BYTES
            || !self
                .frontmatter_record_key
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
            || !matches!(self.tag_prefix.as_str(), "#" | "tag:")
            || !matches!(
                (self.link_open.as_str(), self.link_close.as_str()),
                ("[[", "]]") | ("<", ">")
            )
            || self.identifier_prefix != "knowledge-"
            || self.kind_paths.len() != knowledge_schemas().len()
        {
            return Err(KnowledgeError::InvalidTemplate);
        }
        let mut kinds = BTreeSet::new();
        for template in &self.kind_paths {
            if !kinds.insert(template.kind)
                || template.folder_components.is_empty()
                || template.folder_components.len() > MAX_TEMPLATE_COMPONENTS
            {
                return Err(KnowledgeError::InvalidTemplate);
            }
            let mut components = template.folder_components.clone();
            components.push("fixture.md".to_owned());
            WorkspacePath::new(self.workspace_id.clone(), components)
                .map_err(|_| KnowledgeError::InvalidTemplate)?;
        }
        Ok(())
    }

    /// Returns the deterministic proposed path for a new canonical record.
    pub fn proposed_path(&self, record: &KnowledgeRecord) -> Result<WorkspacePath, KnowledgeError> {
        self.validate()?;
        validate_record(record)?;
        let template = self
            .kind_paths
            .iter()
            .find(|template| template.kind == record.kind)
            .ok_or(KnowledgeError::InvalidTemplate)?;
        let filename = match template.filename {
            KnowledgeFilenameTemplate::RecordId => format!("{}.md", record.record_id.as_str()),
            KnowledgeFilenameTemplate::KindAndRecordId => format!(
                "{}-{}.md",
                kind_wire(record.kind),
                record.record_id.as_str()
            ),
        };
        let mut components = template.folder_components.clone();
        components.push(filename);
        WorkspacePath::new(self.workspace_id.clone(), components)
            .map_err(|_| KnowledgeError::InvalidTemplate)
    }
}

/// Observed filesystem kind supplied by a trusted platform adapter.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlainFolderEntryKind {
    /// Ordinary held file.
    RegularFile,
    /// Symbolic link, always rejected.
    SymbolicLink,
    /// Directory where a note file was expected.
    Directory,
    /// Device, socket, pipe, or other unsupported object.
    Special,
}

/// One already-authorized plain-folder note snapshot.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlainFolderNoteInput {
    /// Exact canonical workspace-relative path.
    pub path: WorkspacePath,
    /// Observed entry kind.
    pub entry_kind: PlainFolderEntryKind,
    /// Whether trusted storage classification detected a synchronized root.
    pub cloud_synchronized: bool,
    /// Whether the entry is hidden under the selected platform convention.
    pub hidden: bool,
    /// Exact lowercase SHA-256 supplied with the held snapshot.
    pub content_sha256: String,
    /// Authorized immutable bytes.
    pub content: Vec<u8>,
}

#[derive(Clone)]
struct StoredRecord {
    path: WorkspacePath,
    record: KnowledgeRecord,
    canonical_sha256: String,
}

/// Read-only plain-folder knowledge view with no filesystem handle or mutation method.
pub struct PlainFolderKnowledgeStore {
    layout: PlainFolderLayout,
    records: BTreeMap<KnowledgeRecordId, StoredRecord>,
}

impl PlainFolderKnowledgeStore {
    /// Parses a complete bounded snapshot set and rejects any ambiguous entry.
    pub fn from_snapshots(
        layout: PlainFolderLayout,
        mut inputs: Vec<PlainFolderNoteInput>,
    ) -> Result<Self, KnowledgeError> {
        layout.validate()?;
        if inputs.len() > MAX_NOTES {
            return Err(KnowledgeError::InvalidPath);
        }
        inputs.sort_by(|left, right| left.path.cmp(&right.path));
        let mut paths = BTreeSet::new();
        let mut records = BTreeMap::new();
        for input in inputs {
            if input.path.workspace_id() != &layout.workspace_id
                || input.entry_kind != PlainFolderEntryKind::RegularFile
                || input.cloud_synchronized
                || input.hidden
                || input.content.len() > MAX_NOTE_BYTES
                || input
                    .path
                    .components()
                    .last()
                    .is_none_or(|component| !component.as_str().ends_with(".md"))
                || !paths.insert(input.path.clone())
            {
                return Err(KnowledgeError::InvalidPath);
            }
            let actual_sha256 = sha256(&input.content);
            if input.content_sha256 != actual_sha256 {
                return Err(KnowledgeError::ContentDrift);
            }
            let record = parse_canonical_markdown(&layout, &input.content)?;
            let identity = record.record_id.clone();
            if records
                .insert(
                    identity,
                    StoredRecord {
                        path: input.path,
                        record,
                        canonical_sha256: actual_sha256,
                    },
                )
                .is_some()
            {
                return Err(KnowledgeError::DuplicateRecord);
            }
        }
        Ok(Self { layout, records })
    }

    /// Returns the observed path for one stable identity without deriving identity from it.
    #[must_use]
    pub fn observed_path(&self, record_id: &KnowledgeRecordId) -> Option<&WorkspacePath> {
        self.records.get(record_id).map(|stored| &stored.path)
    }

    /// Returns the selected plain-folder layout.
    #[must_use]
    pub const fn layout(&self) -> &PlainFolderLayout {
        &self.layout
    }
}

impl KnowledgeStore for PlainFolderKnowledgeStore {
    fn summaries(&self) -> Result<Vec<KnowledgeRecordSummary>, KnowledgeError> {
        Ok(self
            .records
            .values()
            .map(|stored| KnowledgeRecordSummary {
                record_id: stored.record.record_id.clone(),
                kind: stored.record.kind,
                title: stored.record.title.clone(),
                canonical_sha256: stored.canonical_sha256.clone(),
            })
            .collect())
    }

    fn record(
        &self,
        record_id: &KnowledgeRecordId,
    ) -> Result<Option<KnowledgeRecord>, KnowledgeError> {
        Ok(self
            .records
            .get(record_id)
            .map(|stored| stored.record.clone()))
    }

    fn preview_create(
        &self,
        record: &KnowledgeRecord,
    ) -> Result<KnowledgeWritePreview, KnowledgeError> {
        if self.records.contains_key(&record.record_id) {
            return Err(KnowledgeError::DuplicateRecord);
        }
        preview(&self.layout, KnowledgeWriteKind::Create, None, record)
    }

    fn preview_update(
        &self,
        expected_sha256: &str,
        record: &KnowledgeRecord,
    ) -> Result<KnowledgeWritePreview, KnowledgeError> {
        let current = self
            .records
            .get(&record.record_id)
            .ok_or(KnowledgeError::InvalidIdentity)?;
        if current.canonical_sha256 != expected_sha256 {
            return Err(KnowledgeError::ContentDrift);
        }
        preview(
            &self.layout,
            KnowledgeWriteKind::Update,
            Some(expected_sha256.to_owned()),
            record,
        )
    }
}

fn preview(
    layout: &PlainFolderLayout,
    kind: KnowledgeWriteKind,
    expected_sha256: Option<String>,
    record: &KnowledgeRecord,
) -> Result<KnowledgeWritePreview, KnowledgeError> {
    let proposed_markdown = render_canonical_markdown(layout, record)?;
    Ok(KnowledgeWritePreview {
        kind,
        record_id: record.record_id.clone(),
        expected_sha256,
        proposed_sha256: sha256(&proposed_markdown),
        proposed_markdown,
    })
}

/// Renders one exact canonical Markdown record under the selected convention.
pub fn render_canonical_markdown(
    layout: &PlainFolderLayout,
    record: &KnowledgeRecord,
) -> Result<Vec<u8>, KnowledgeError> {
    layout.validate()?;
    validate_record(record)?;
    let record_json = serde_json::to_string(record).map_err(|_| KnowledgeError::InvalidMarkdown)?;
    let mut markdown = format!(
        "---\n{}: {}\n---\n\n# {}\n",
        layout.frontmatter_record_key, record_json, record.title
    );
    if !record.fields.is_empty() {
        markdown.push_str("\n## Fields\n");
        for field in &record.fields {
            markdown.push_str(&format!("- `{}`: {}\n", field.name, field.value));
        }
    }
    if !record.links.is_empty() {
        markdown.push_str("\n## Links\n");
        for link in &record.links {
            markdown.push_str(&format!(
                "- {}{}{} (`{}`)\n",
                layout.link_open,
                link.target_id.as_str(),
                layout.link_close,
                link_kind_wire(link.kind)
            ));
        }
    }
    if !record.tags.is_empty() {
        markdown.push_str("\n## Tags\n");
        for tag in &record.tags {
            markdown.push_str(&format!("{}{}\n", layout.tag_prefix, tag));
        }
    }
    Ok(markdown.into_bytes())
}

/// Parses only exact canonical Markdown and rejects stale or independently edited projections.
pub fn parse_canonical_markdown(
    layout: &PlainFolderLayout,
    content: &[u8],
) -> Result<KnowledgeRecord, KnowledgeError> {
    layout.validate()?;
    if content.is_empty() || content.len() > MAX_NOTE_BYTES {
        return Err(KnowledgeError::InvalidMarkdown);
    }
    let text = std::str::from_utf8(content).map_err(|_| KnowledgeError::InvalidMarkdown)?;
    let mut lines = text.lines();
    if lines.next() != Some("---") {
        return Err(KnowledgeError::InvalidMarkdown);
    }
    let record_line = lines.next().ok_or(KnowledgeError::InvalidMarkdown)?;
    let prefix = format!("{}: ", layout.frontmatter_record_key);
    let json = record_line
        .strip_prefix(&prefix)
        .ok_or(KnowledgeError::InvalidMarkdown)?;
    if lines.next() != Some("---") {
        return Err(KnowledgeError::InvalidMarkdown);
    }
    let record: KnowledgeRecord =
        serde_json::from_str(json).map_err(|_| KnowledgeError::InvalidMarkdown)?;
    validate_record(&record)?;
    if render_canonical_markdown(layout, &record)? != content {
        return Err(KnowledgeError::InvalidMarkdown);
    }
    Ok(record)
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

fn link_kind_wire(kind: crate::KnowledgeLinkKind) -> &'static str {
    match kind {
        crate::KnowledgeLinkKind::Related => "related",
        crate::KnowledgeLinkKind::OwnedBy => "owned_by",
        crate::KnowledgeLinkKind::About => "about",
        crate::KnowledgeLinkKind::Supports => "supports",
        crate::KnowledgeLinkKind::DependsOn => "depends_on",
        crate::KnowledgeLinkKind::Supersedes => "supersedes",
        crate::KnowledgeLinkKind::FollowUpTo => "follow_up_to",
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
    use agentmage_kernel_contracts::{
        DataSensitivity, EvidenceId, EvidenceKind, EvidenceReference,
    };

    use super::*;
    use crate::{
        KNOWLEDGE_SCHEMA_VERSION, KnowledgeField, KnowledgeLink, KnowledgeLinkKind,
        KnowledgePrivacy, KnowledgeRetention, KnowledgeRetentionKind, knowledge_schema,
    };

    type InputMutation = Box<dyn Fn(&mut PlainFolderNoteInput)>;

    fn record(identity: &str, kind: KnowledgeRecordKind) -> KnowledgeRecord {
        KnowledgeRecord {
            schema_version: KNOWLEDGE_SCHEMA_VERSION,
            record_id: KnowledgeRecordId::parse(identity).expect("identity"),
            kind,
            title: "Synthetic record".to_owned(),
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
            evidence: vec![EvidenceReference {
                schema_version: agentmage_kernel_contracts::CONTRACT_SCHEMA_VERSION,
                evidence_id: EvidenceId::from_raw(format!("evidence-{identity}")),
                kind: EvidenceKind::Document,
                source_id: "fixture-source".to_owned(),
                object_id: identity.to_owned(),
                fragment: None,
                content_sha256: "a".repeat(64),
                observed_revision: Some("fixture-v1".to_owned()),
            }],
        }
    }

    fn input(
        layout: &PlainFolderLayout,
        path: WorkspacePath,
        record: &KnowledgeRecord,
    ) -> PlainFolderNoteInput {
        let content = render_canonical_markdown(layout, record).expect("markdown");
        PlainFolderNoteInput {
            path,
            entry_kind: PlainFolderEntryKind::RegularFile,
            cloud_synchronized: false,
            hidden: false,
            content_sha256: sha256(&content),
            content,
        }
    }

    #[test]
    fn exact_record_round_trips_and_path_does_not_define_identity() {
        let workspace = WorkspaceId::from_raw("workspace-knowledge");
        let layout = PlainFolderLayout::default_for(workspace.clone());
        let value = record("knowledge-person-001", KnowledgeRecordKind::Person);
        let first = WorkspacePath::new(workspace.clone(), ["people", "first.md"]).expect("path");
        let moved = WorkspacePath::new(workspace, ["archive", "renamed.md"]).expect("path");
        let first_store = PlainFolderKnowledgeStore::from_snapshots(
            layout.clone(),
            vec![input(&layout, first, &value)],
        )
        .expect("store");
        let moved_store = PlainFolderKnowledgeStore::from_snapshots(
            layout.clone(),
            vec![input(&layout, moved, &value)],
        )
        .expect("store");
        assert_eq!(
            first_store.summaries().expect("summaries"),
            moved_store.summaries().expect("summaries")
        );
        assert_eq!(
            parse_canonical_markdown(
                &layout,
                &render_canonical_markdown(&layout, &value).expect("render")
            ),
            Ok(value)
        );
    }

    #[test]
    fn layout_is_complete_configurable_and_generates_safe_paths() {
        let workspace = WorkspaceId::from_raw("workspace-knowledge");
        let mut layout = PlainFolderLayout::default_for(workspace);
        layout.tag_prefix = "tag:".to_owned();
        layout.link_open = "<".to_owned();
        layout.link_close = ">".to_owned();
        layout.kind_paths[0].folder_components = vec!["custom people".to_owned()];
        layout.kind_paths[0].filename = KnowledgeFilenameTemplate::KindAndRecordId;
        layout.validate().expect("layout");
        let path = layout
            .proposed_path(&record("knowledge-person-001", KnowledgeRecordKind::Person))
            .expect("path");
        assert_eq!(path.components()[0].as_str(), "custom people");
        assert_eq!(
            path.components()[1].as_str(),
            "person-knowledge-person-001.md"
        );
    }

    #[test]
    fn symlink_cloud_hidden_foreign_hash_and_non_markdown_inputs_fail_closed() {
        let workspace = WorkspaceId::from_raw("workspace-knowledge");
        let layout = PlainFolderLayout::default_for(workspace.clone());
        let value = record("knowledge-person-001", KnowledgeRecordKind::Person);
        let path = layout.proposed_path(&value).expect("path");
        let base = input(&layout, path, &value);
        let mutations: Vec<InputMutation> = vec![
            Box::new(|item| item.entry_kind = PlainFolderEntryKind::SymbolicLink),
            Box::new(|item| item.cloud_synchronized = true),
            Box::new(|item| item.hidden = true),
            Box::new(|item| item.content_sha256 = "b".repeat(64)),
            Box::new(|item| {
                item.path = WorkspacePath::new(
                    WorkspaceId::from_raw("foreign-workspace"),
                    ["person", "record.md"],
                )
                .expect("path")
            }),
        ];
        for mutate in mutations {
            let mut changed = base.clone();
            mutate(&mut changed);
            assert!(
                PlainFolderKnowledgeStore::from_snapshots(layout.clone(), vec![changed]).is_err()
            );
        }
    }

    #[test]
    fn duplicate_identity_path_and_markdown_mutation_fail_closed() {
        let workspace = WorkspaceId::from_raw("workspace-knowledge");
        let layout = PlainFolderLayout::default_for(workspace.clone());
        let first = record("knowledge-person-001", KnowledgeRecordKind::Person);
        let path_a = WorkspacePath::new(workspace.clone(), ["person", "a.md"]).expect("path");
        let path_b = WorkspacePath::new(workspace, ["person", "b.md"]).expect("path");
        assert_eq!(
            PlainFolderKnowledgeStore::from_snapshots(
                layout.clone(),
                vec![
                    input(&layout, path_a.clone(), &first),
                    input(&layout, path_b, &first)
                ],
            )
            .err(),
            Some(KnowledgeError::DuplicateRecord)
        );
        assert!(
            PlainFolderKnowledgeStore::from_snapshots(
                layout.clone(),
                vec![
                    input(&layout, path_a.clone(), &first),
                    input(&layout, path_a, &first)
                ],
            )
            .is_err()
        );
        let mut changed = render_canonical_markdown(&layout, &first).expect("render");
        changed.extend_from_slice(b"unreviewed");
        assert_eq!(
            parse_canonical_markdown(&layout, &changed),
            Err(KnowledgeError::InvalidMarkdown)
        );
    }

    #[test]
    fn previews_are_deterministic_compare_and_swap_values_without_apply() {
        let workspace = WorkspaceId::from_raw("workspace-knowledge");
        let layout = PlainFolderLayout::default_for(workspace);
        let current = record("knowledge-person-001", KnowledgeRecordKind::Person);
        let path = layout.proposed_path(&current).expect("path");
        let store = PlainFolderKnowledgeStore::from_snapshots(
            layout.clone(),
            vec![input(&layout, path, &current)],
        )
        .expect("store");
        let current_hash = store.summaries().expect("summary")[0]
            .canonical_sha256
            .clone();
        let mut changed = current.clone();
        changed.updated_at = "2026-08-14T01:00:00Z".to_owned();
        changed.fields.push(KnowledgeField {
            name: "notes".to_owned(),
            value: "reviewed change".to_owned(),
        });
        let preview = store
            .preview_update(&current_hash, &changed)
            .expect("preview");
        assert_eq!(preview.kind, KnowledgeWriteKind::Update);
        assert_eq!(preview.expected_sha256, Some(current_hash));
        assert_eq!(preview.proposed_sha256, sha256(&preview.proposed_markdown));
        assert_eq!(store.record(&current.record_id), Ok(Some(current)));
        assert_eq!(
            store.preview_update(&"0".repeat(64), &changed),
            Err(KnowledgeError::ContentDrift)
        );

        let mut related = record("knowledge-project-002", KnowledgeRecordKind::Project);
        related.links.push(KnowledgeLink {
            kind: KnowledgeLinkKind::Related,
            target_id: changed.record_id,
        });
        assert_eq!(
            store.preview_create(&related).expect("create").kind,
            KnowledgeWriteKind::Create
        );
    }
}
