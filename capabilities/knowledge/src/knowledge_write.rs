//! Canonical Markdown creation, namespace validation, and post-write index bindings.

use std::collections::BTreeSet;
use std::fmt::Write as _;

use agentmage_kernel_contracts::WorkspacePath;
use serde::Serialize;
use sha2::{Digest, Sha256};
use unicode_normalization::UnicodeNormalization;

use crate::{
    KnowledgeRecordId, MarkdownDocument, MarkdownFidelityWarning, MarkdownLineEnding,
    MarkdownUpdatePreview, MarkdownWriteError, MemoryItem, MemoryItemStatus, MemoryType,
};

const MAX_NAMESPACE_ITEMS: usize = 100_000;
const MAX_CREATE_BYTES: usize = 4 * 1024 * 1024;
const MAX_PROPERTIES: usize = 64;
const MAX_SECTIONS: usize = 32;
const MAX_TEXT_BYTES: usize = 1024 * 1024;
const MAX_LINKS: usize = 512;

/// Closed canonical note workflow types implemented by the same write boundary.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum KnowledgeWriteWorkflow {
    /// Source-backed task record.
    Task,
    /// Confirmed decision record.
    Decision,
    /// Commitment record.
    Commitment,
    /// Correspondence record.
    Correspondence,
    /// Meeting record with protected raw notes.
    Meeting,
    /// Portable handoff record.
    Handoff,
    /// Deliberately promoted durable memory record.
    Memory,
}

/// One approved scalar property for a new note.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KnowledgeFrontmatterProperty {
    /// Closed writable key.
    pub key: String,
    /// Exact logical scalar value.
    pub value: String,
}

/// One exact second-level section in a new canonical note.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KnowledgeSectionDraft {
    /// Exact section heading from the workflow schema.
    pub heading: String,
    /// Body using logical `\n` separators.
    pub body: String,
}

/// Content-bound proof that one durable memory item already received explicit approval.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct KnowledgeMemoryPromotionApproval {
    memory_id: String,
    candidate_sha256: String,
    decision_sha256: String,
    approved_content_sha256: String,
}

impl KnowledgeMemoryPromotionApproval {
    /// Derives a Markdown-write binding only from one explicitly approved durable item.
    pub fn from_approved_item(item: &MemoryItem) -> Result<Self, MarkdownWriteError> {
        let content = item
            .content
            .as_deref()
            .filter(|content| !content.is_empty())
            .ok_or(MarkdownWriteError::ProtectedSource)?;
        if item.status != MemoryItemStatus::Approved
            || item.memory_type == MemoryType::Working
            || item.evidence.is_empty()
            || item.sensitivity == agentmage_kernel_contracts::DataSensitivity::Restricted
            || item.confidence_bps < 7_000
            || item.decided_at.is_empty()
            || item.last_verified_at.is_empty()
            || item.superseded_by.is_some()
            || content.len() > MAX_TEXT_BYTES
            || invalid_text(content)
            || !valid_sha256(&item.candidate_sha256)
            || !valid_sha256(&item.decision_sha256)
        {
            return Err(MarkdownWriteError::ProtectedSource);
        }
        Ok(Self {
            memory_id: item.memory_id.as_str().to_owned(),
            candidate_sha256: item.candidate_sha256.clone(),
            decision_sha256: item.decision_sha256.clone(),
            approved_content_sha256: sha256(content.as_bytes()),
        })
    }

    /// Returns the exact approved memory identity.
    #[must_use]
    pub fn memory_id(&self) -> &str {
        &self.memory_id
    }

    /// Returns the digest of the explicit user decision evidence.
    #[must_use]
    pub fn decision_sha256(&self) -> &str {
        &self.decision_sha256
    }
}

/// Complete authority-free note-creation request.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KnowledgeNoteCreateRequest {
    /// Exact proposed canonical path.
    pub path: WorkspacePath,
    /// Stable path-independent identity.
    pub stable_id: KnowledgeRecordId,
    /// Closed record workflow.
    pub workflow: KnowledgeWriteWorkflow,
    /// User-visible title.
    pub title: String,
    /// Approved scalar metadata excluding identity and workflow type.
    pub properties: Vec<KnowledgeFrontmatterProperty>,
    /// Exact workflow sections.
    pub sections: Vec<KnowledgeSectionDraft>,
    /// Required explicit promotion proof for memory; prohibited for every other workflow.
    pub memory_promotion: Option<KnowledgeMemoryPromotionApproval>,
    /// Exact output line-ending convention.
    pub line_ending: MarkdownLineEnding,
}

/// Complete already-observed namespace used for collision and link checks.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KnowledgeNamespaceSnapshot {
    /// Every canonical Markdown path in the approved root.
    pub paths: Vec<WorkspacePath>,
    /// Every stable note identity in the approved root.
    pub stable_ids: Vec<KnowledgeRecordId>,
    /// Every exact title, alias, or stable identity accepted as a wiki-link target.
    pub link_targets: Vec<String>,
    /// Exact canonical snapshot digest used by the derived index.
    pub canonical_snapshot_sha256: String,
    /// Current derived-index revision observed with this namespace.
    pub derived_index_revision: u64,
}

/// Authority-free exact note-creation preview.
#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct KnowledgeNoteCreatePreview {
    /// Exact proposed path.
    pub path: WorkspacePath,
    /// Stable note identity.
    pub stable_id: KnowledgeRecordId,
    /// Closed workflow.
    pub workflow: KnowledgeWriteWorkflow,
    /// Digest of exact proposed Markdown bytes.
    pub proposed_source_sha256: String,
    /// Namespace digest the collision and link checks used.
    pub expected_namespace_sha256: String,
    /// Derived-index revision that must become stale after canonical commit.
    pub expected_index_revision: u64,
    /// Sorted exact wiki-link targets verified against the namespace.
    pub verified_wiki_links: Vec<String>,
    /// Exact fidelity warnings from parsing the generated document.
    pub fidelity_warnings: Vec<MarkdownFidelityWarning>,
    /// Exact explicit promotion proof for memory, otherwise none.
    pub memory_promotion: Option<KnowledgeMemoryPromotionApproval>,
    /// Exact proposed Markdown bytes.
    #[serde(skip_serializing)]
    proposed_markdown: Vec<u8>,
    /// Hash binding every displayed field and proposed bytes.
    pub preview_sha256: String,
}

impl std::fmt::Debug for KnowledgeNoteCreatePreview {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("KnowledgeNoteCreatePreview")
            .field("path", &self.path)
            .field("stable_id", &self.stable_id)
            .field("workflow", &self.workflow)
            .field("proposed_source_sha256", &self.proposed_source_sha256)
            .field("preview_sha256", &self.preview_sha256)
            .finish_non_exhaustive()
    }
}

impl KnowledgeNoteCreatePreview {
    /// Returns the exact proposed bytes for a separately authorized create operation.
    #[must_use]
    pub fn proposed_markdown(&self) -> &[u8] {
        &self.proposed_markdown
    }
}

/// Exact high-risk structural operation requiring additional user confirmation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum KnowledgeStructuralActionKind {
    /// Move one exact note to another folder.
    Move,
    /// Rename one exact note without changing its folder.
    Rename,
    /// Mark one note as superseded without erasing history.
    Supersede,
    /// Move one exact note to an approved trash destination.
    TrashDelete,
}

/// Authority-free high-risk structural action preview.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct KnowledgeStructuralActionPreview {
    /// Closed structural action.
    pub action: KnowledgeStructuralActionKind,
    /// Stable note identity.
    pub stable_id: KnowledgeRecordId,
    /// Exact current source path.
    pub source_path: WorkspacePath,
    /// Exact destination when the action has one.
    pub destination_path: Option<WorkspacePath>,
    /// Exact current content digest.
    pub expected_source_sha256: String,
    /// Always true; ordinary content approval cannot authorize this action.
    pub requires_additional_confirmation: bool,
    /// Always false; bulk structural actions are not representable.
    pub bulk_action: bool,
    /// Hash binding the complete preview.
    pub preview_sha256: String,
}

/// Filesystem result class used only to gate derived-index publication.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CanonicalMarkdownWriteOutcome {
    /// Exact proposed bytes committed and verified.
    Committed,
    /// No canonical byte changed.
    FailedNoChange,
    /// Canonical state requires fresh observation before any projection update.
    Uncertain,
}

/// Exact canonical mutation identity passed to a derived-index coordinator.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CanonicalKnowledgeMutation {
    path: WorkspacePath,
    stable_id: KnowledgeRecordId,
    expected_source_sha256: Option<String>,
    proposed_source_sha256: String,
    preview_sha256: String,
}

impl CanonicalKnowledgeMutation {
    /// Creates a canonical binding from one creation preview.
    #[must_use]
    pub fn from_create(preview: &KnowledgeNoteCreatePreview) -> Self {
        Self {
            path: preview.path.clone(),
            stable_id: preview.stable_id.clone(),
            expected_source_sha256: None,
            proposed_source_sha256: preview.proposed_source_sha256.clone(),
            preview_sha256: preview.preview_sha256.clone(),
        }
    }

    /// Creates a canonical binding from one update preview.
    #[must_use]
    pub fn from_update(preview: &MarkdownUpdatePreview) -> Self {
        Self {
            path: preview.path.clone(),
            stable_id: preview.stable_id.clone(),
            expected_source_sha256: Some(preview.expected_source_sha256.clone()),
            proposed_source_sha256: preview.proposed_source_sha256.clone(),
            preview_sha256: preview.preview_sha256.clone(),
        }
    }

    /// Returns the exact canonical path.
    #[must_use]
    pub const fn path(&self) -> &WorkspacePath {
        &self.path
    }

    /// Returns the stable record identity.
    #[must_use]
    pub const fn stable_id(&self) -> &KnowledgeRecordId {
        &self.stable_id
    }

    /// Returns the expected current digest for updates.
    #[must_use]
    pub fn expected_source_sha256(&self) -> Option<&str> {
        self.expected_source_sha256.as_deref()
    }

    /// Returns the exact proposed digest.
    #[must_use]
    pub fn proposed_source_sha256(&self) -> &str {
        &self.proposed_source_sha256
    }

    /// Returns the exact user-visible preview digest.
    #[must_use]
    pub fn preview_sha256(&self) -> &str {
        &self.preview_sha256
    }
}

/// Derived-index publication decision that never changes canonical Markdown.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KnowledgeIndexPublicationState {
    /// Canonical commit is verified and an exact projection update may run.
    ReadyAfterCommit,
    /// Canonical bytes did not change, so the current projection is retained.
    PreservedAfterFailure,
    /// Canonical state is uncertain and the projection must remain visibly stale.
    RebuildRequired,
}

/// Content-free publication decision bound to one canonical mutation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KnowledgeIndexPublication {
    /// Exact publication state.
    pub state: KnowledgeIndexPublicationState,
    /// Exact canonical mutation identity.
    pub mutation: CanonicalKnowledgeMutation,
    /// Canonical digest observed after the write attempt when available.
    pub observed_source_sha256: Option<String>,
}

/// Renders and validates one exact workflow note against the complete observed namespace.
pub fn preview_knowledge_note_create(
    request: KnowledgeNoteCreateRequest,
    namespace: &KnowledgeNamespaceSnapshot,
) -> Result<KnowledgeNoteCreatePreview, MarkdownWriteError> {
    validate_namespace(&request, namespace)?;
    validate_create_request(&request)?;
    let proposed_markdown = render_note(&request)?;
    let document = MarkdownDocument::parse(request.path.clone(), proposed_markdown.clone())?;
    if document.stable_id() != Some(&request.stable_id) {
        return Err(MarkdownWriteError::InvalidIdentity);
    }
    let links = wiki_links(&proposed_markdown)?;
    let allowed = namespace
        .link_targets
        .iter()
        .map(String::as_str)
        .chain(namespace.stable_ids.iter().map(KnowledgeRecordId::as_str))
        .map(normalized_name)
        .collect::<BTreeSet<_>>();
    if links
        .iter()
        .any(|target| !allowed.contains(&normalized_name(target)))
    {
        return Err(MarkdownWriteError::AmbiguousTarget);
    }
    let mut preview = KnowledgeNoteCreatePreview {
        path: request.path,
        stable_id: request.stable_id,
        workflow: request.workflow,
        proposed_source_sha256: sha256(&proposed_markdown),
        expected_namespace_sha256: namespace.canonical_snapshot_sha256.clone(),
        expected_index_revision: namespace.derived_index_revision,
        verified_wiki_links: links,
        fidelity_warnings: document.fidelity_warnings().to_vec(),
        memory_promotion: request.memory_promotion,
        proposed_markdown,
        preview_sha256: String::new(),
    };
    preview.preview_sha256 = create_preview_digest(&preview)?;
    Ok(preview)
}

/// Recomputes every creation-preview binding before host composition.
pub fn verify_knowledge_note_create_preview(
    preview: &KnowledgeNoteCreatePreview,
) -> Result<(), MarkdownWriteError> {
    if preview.proposed_source_sha256 != sha256(&preview.proposed_markdown)
        || !valid_sha256(&preview.expected_namespace_sha256)
        || preview.preview_sha256 != create_preview_digest(preview)?
    {
        return Err(MarkdownWriteError::StaleSource);
    }
    let parsed = MarkdownDocument::parse(preview.path.clone(), preview.proposed_markdown.clone())?;
    if parsed.stable_id() != Some(&preview.stable_id)
        || wiki_links(&preview.proposed_markdown)? != preview.verified_wiki_links
        || parsed.fidelity_warnings() != preview.fidelity_warnings
        || !verify_rendered_workflow(preview)?
    {
        return Err(MarkdownWriteError::StructuralDrift);
    }
    Ok(())
}

/// Creates one non-bulk structural-action preview with mandatory extra confirmation.
pub fn preview_knowledge_structural_action(
    document: &MarkdownDocument,
    action: KnowledgeStructuralActionKind,
    destination_path: Option<WorkspacePath>,
) -> Result<KnowledgeStructuralActionPreview, MarkdownWriteError> {
    let stable_id = document
        .stable_id()
        .ok_or(MarkdownWriteError::InvalidIdentity)?
        .clone();
    let destination_required = matches!(
        action,
        KnowledgeStructuralActionKind::Move
            | KnowledgeStructuralActionKind::Rename
            | KnowledgeStructuralActionKind::TrashDelete
    );
    if destination_required != destination_path.is_some()
        || destination_path.as_ref().is_some_and(|path| {
            path.workspace_id() != document.path().workspace_id() || path == document.path()
        })
    {
        return Err(MarkdownWriteError::StructuralDrift);
    }
    let mut preview = KnowledgeStructuralActionPreview {
        action,
        stable_id,
        source_path: document.path().clone(),
        destination_path,
        expected_source_sha256: document.source_sha256().to_owned(),
        requires_additional_confirmation: true,
        bulk_action: false,
        preview_sha256: String::new(),
    };
    preview.preview_sha256 = structural_preview_digest(&preview)?;
    Ok(preview)
}

/// Produces a fail-closed derived-index decision from the exact filesystem outcome.
pub fn decide_index_publication(
    mutation: CanonicalKnowledgeMutation,
    outcome: CanonicalMarkdownWriteOutcome,
    observed_source_sha256: Option<String>,
) -> KnowledgeIndexPublication {
    let state = match outcome {
        CanonicalMarkdownWriteOutcome::Committed
            if observed_source_sha256.as_deref()
                == Some(mutation.proposed_source_sha256.as_str()) =>
        {
            KnowledgeIndexPublicationState::ReadyAfterCommit
        }
        CanonicalMarkdownWriteOutcome::FailedNoChange
            if observed_source_sha256.as_deref() == mutation.expected_source_sha256.as_deref() =>
        {
            KnowledgeIndexPublicationState::PreservedAfterFailure
        }
        _ => KnowledgeIndexPublicationState::RebuildRequired,
    };
    KnowledgeIndexPublication {
        state,
        mutation,
        observed_source_sha256,
    }
}

fn validate_namespace(
    request: &KnowledgeNoteCreateRequest,
    namespace: &KnowledgeNamespaceSnapshot,
) -> Result<(), MarkdownWriteError> {
    if namespace.paths.len() > MAX_NAMESPACE_ITEMS
        || namespace.stable_ids.len() > MAX_NAMESPACE_ITEMS
        || namespace.link_targets.len() > MAX_NAMESPACE_ITEMS
        || !valid_sha256(&namespace.canonical_snapshot_sha256)
        || request
            .path
            .components()
            .last()
            .is_none_or(|name| !name.as_str().to_ascii_lowercase().ends_with(".md"))
    {
        return Err(MarkdownWriteError::ResourceLimit);
    }
    let candidate = normalized_path(&request.path);
    let mut paths = BTreeSet::new();
    for path in &namespace.paths {
        if path.workspace_id() != request.path.workspace_id()
            || !paths.insert(normalized_path(path))
            || normalized_path(path) == candidate
        {
            return Err(MarkdownWriteError::AmbiguousTarget);
        }
    }
    let mut identities = BTreeSet::new();
    for identity in &namespace.stable_ids {
        if !identities.insert(identity.as_str()) || identity == &request.stable_id {
            return Err(MarkdownWriteError::InvalidIdentity);
        }
    }
    let mut targets = BTreeSet::new();
    if namespace.link_targets.iter().any(|target| {
        target.is_empty() || target.len() > 4096 || !targets.insert(normalized_name(target))
    }) {
        return Err(MarkdownWriteError::AmbiguousTarget);
    }
    Ok(())
}

fn validate_create_request(request: &KnowledgeNoteCreateRequest) -> Result<(), MarkdownWriteError> {
    if request.line_ending == MarkdownLineEnding::None
        || request.title.trim().is_empty()
        || request.title.len() > 512
        || invalid_text(&request.title)
        || request.properties.len() > MAX_PROPERTIES
        || request.sections.len() > MAX_SECTIONS
    {
        return Err(MarkdownWriteError::ResourceLimit);
    }
    let mut property_keys = BTreeSet::new();
    for property in &request.properties {
        if !writable_property_key(&property.key)
            || !property_keys.insert(property.key.as_str())
            || property.value.is_empty()
            || property.value.len() > 32 * 1024
            || invalid_text(&property.value)
        {
            return Err(MarkdownWriteError::FrontmatterDenied);
        }
    }
    let required = required_sections(request.workflow);
    if request.sections.len() != required.len() {
        return Err(MarkdownWriteError::StructuralDrift);
    }
    let mut headings = BTreeSet::new();
    for section in &request.sections {
        if !required.contains(&section.heading.as_str())
            || !headings.insert(section.heading.as_str())
            || section.body.len() > MAX_TEXT_BYTES
            || section.body.contains('\r')
            || invalid_text(&section.body)
        {
            return Err(MarkdownWriteError::StructuralDrift);
        }
    }
    if headings != required {
        return Err(MarkdownWriteError::StructuralDrift);
    }
    match (request.workflow, request.memory_promotion.as_ref()) {
        (KnowledgeWriteWorkflow::Memory, Some(approval)) => {
            let memory = request
                .sections
                .iter()
                .find(|section| section.heading == "Memory")
                .ok_or(MarkdownWriteError::ProtectedSource)?;
            if !valid_memory_promotion(approval)
                || sha256(memory.body.as_bytes()) != approval.approved_content_sha256
            {
                return Err(MarkdownWriteError::ProtectedSource);
            }
        }
        (KnowledgeWriteWorkflow::Memory, None) | (_, Some(_)) => {
            return Err(MarkdownWriteError::ProtectedSource);
        }
        (_, None) => {}
    }
    Ok(())
}

fn render_note(request: &KnowledgeNoteCreateRequest) -> Result<Vec<u8>, MarkdownWriteError> {
    let ending = request.line_ending.bytes();
    let mut output = Vec::new();
    append_line(&mut output, b"---", ending);
    append_line(
        &mut output,
        format!("agentmage_id: {}", yaml_scalar(request.stable_id.as_str())?).as_bytes(),
        ending,
    );
    append_line(
        &mut output,
        format!("type: {}", workflow_wire(request.workflow)).as_bytes(),
        ending,
    );
    if let Some(approval) = &request.memory_promotion {
        for (key, value) in [
            ("memory_id", approval.memory_id.as_str()),
            (
                "memory_candidate_sha256",
                approval.candidate_sha256.as_str(),
            ),
            ("memory_decision_sha256", approval.decision_sha256.as_str()),
            (
                "memory_content_sha256",
                approval.approved_content_sha256.as_str(),
            ),
        ] {
            append_line(
                &mut output,
                format!("{key}: {}", yaml_scalar(value)?).as_bytes(),
                ending,
            );
        }
    }
    let mut properties = request.properties.clone();
    properties.sort_by(|left, right| left.key.cmp(&right.key));
    for property in properties {
        append_line(
            &mut output,
            format!("{}: {}", property.key, yaml_scalar(&property.value)?).as_bytes(),
            ending,
        );
    }
    append_line(&mut output, b"---", ending);
    append_line(&mut output, b"", ending);
    append_line(
        &mut output,
        format!("# {}", request.title.trim()).as_bytes(),
        ending,
    );
    let sections = request
        .sections
        .iter()
        .map(|section| (section.heading.as_str(), section.body.as_str()))
        .collect::<std::collections::BTreeMap<_, _>>();
    for heading in required_section_order(request.workflow) {
        append_line(&mut output, b"", ending);
        append_line(&mut output, format!("## {heading}").as_bytes(), ending);
        let body = sections
            .get(heading)
            .ok_or(MarkdownWriteError::StructuralDrift)?;
        append_logical_text(&mut output, body, ending);
    }
    if output.len() > MAX_CREATE_BYTES {
        return Err(MarkdownWriteError::ResourceLimit);
    }
    Ok(output)
}

fn append_line(output: &mut Vec<u8>, line: &[u8], ending: &[u8]) {
    output.extend_from_slice(line);
    output.extend_from_slice(ending);
}

fn append_logical_text(output: &mut Vec<u8>, value: &str, ending: &[u8]) {
    for line in value.split('\n') {
        append_line(output, line.as_bytes(), ending);
    }
}

fn yaml_scalar(value: &str) -> Result<String, MarkdownWriteError> {
    if value.is_empty() || invalid_text(value) {
        return Err(MarkdownWriteError::MalformedMarkdown);
    }
    serde_json::to_string(value).map_err(|_| MarkdownWriteError::MalformedMarkdown)
}

fn wiki_links(bytes: &[u8]) -> Result<Vec<String>, MarkdownWriteError> {
    let text = std::str::from_utf8(bytes).map_err(|_| MarkdownWriteError::MalformedMarkdown)?;
    let mut links = BTreeSet::new();
    let mut offset = 0;
    while let Some(start) = text[offset..].find("[[") {
        let content_start = offset + start + 2;
        let end = text[content_start..]
            .find("]]")
            .ok_or(MarkdownWriteError::MalformedMarkdown)?;
        let raw = &text[content_start..content_start + end];
        let target_without_alias = raw.split_once('|').map_or(raw, |(target, _)| target);
        let target = target_without_alias
            .split_once('#')
            .map_or(target_without_alias, |(target, _)| target)
            .trim();
        if prohibited_link(target) {
            return Err(MarkdownWriteError::AmbiguousTarget);
        }
        links.insert(target.to_owned());
        if links.len() > MAX_LINKS {
            return Err(MarkdownWriteError::ResourceLimit);
        }
        offset = content_start + end + 2;
    }
    Ok(links.into_iter().collect())
}

fn prohibited_link(value: &str) -> bool {
    value.is_empty()
        || value.len() > 4096
        || value.contains("..")
        || value.contains('/')
        || value.contains('\\')
        || value.contains(':')
        || value.chars().any(char::is_control)
}

fn required_sections(workflow: KnowledgeWriteWorkflow) -> BTreeSet<&'static str> {
    required_section_order(workflow).iter().copied().collect()
}

fn required_section_order(workflow: KnowledgeWriteWorkflow) -> &'static [&'static str] {
    match workflow {
        KnowledgeWriteWorkflow::Task => &["Task", "Evidence"],
        KnowledgeWriteWorkflow::Decision => &["Decision", "Evidence"],
        KnowledgeWriteWorkflow::Commitment => &["Commitment", "Evidence"],
        KnowledgeWriteWorkflow::Correspondence => &["Correspondence", "Evidence"],
        KnowledgeWriteWorkflow::Meeting => &["Summary", "Decisions", "Actions", "Raw Notes"],
        KnowledgeWriteWorkflow::Handoff => &["Current State", "Next Actions", "Evidence"],
        KnowledgeWriteWorkflow::Memory => &["Memory", "Sources"],
    }
}

const fn workflow_wire(workflow: KnowledgeWriteWorkflow) -> &'static str {
    match workflow {
        KnowledgeWriteWorkflow::Task => "task",
        KnowledgeWriteWorkflow::Decision => "decision",
        KnowledgeWriteWorkflow::Commitment => "commitment",
        KnowledgeWriteWorkflow::Correspondence => "correspondence",
        KnowledgeWriteWorkflow::Meeting => "meeting",
        KnowledgeWriteWorkflow::Handoff => "handoff",
        KnowledgeWriteWorkflow::Memory => "memory",
    }
}

fn valid_memory_promotion(approval: &KnowledgeMemoryPromotionApproval) -> bool {
    !approval.memory_id.is_empty()
        && approval.memory_id.len() <= 128
        && approval.memory_id.starts_with("memory-")
        && approval
            .memory_id
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
        && valid_sha256(&approval.candidate_sha256)
        && valid_sha256(&approval.decision_sha256)
        && valid_sha256(&approval.approved_content_sha256)
}

fn verify_rendered_workflow(
    preview: &KnowledgeNoteCreatePreview,
) -> Result<bool, MarkdownWriteError> {
    let text = std::str::from_utf8(&preview.proposed_markdown)
        .map_err(|_| MarkdownWriteError::MalformedMarkdown)?;
    let normalized = text.replace("\r\n", "\n");
    let expected_type = format!("type: {}", workflow_wire(preview.workflow));
    if normalized
        .lines()
        .filter(|line| *line == expected_type)
        .count()
        != 1
    {
        return Ok(false);
    }
    let headings = normalized
        .lines()
        .filter_map(|line| line.strip_prefix("## "))
        .collect::<Vec<_>>();
    if headings != required_section_order(preview.workflow) {
        return Ok(false);
    }
    match (preview.workflow, preview.memory_promotion.as_ref()) {
        (KnowledgeWriteWorkflow::Memory, Some(approval)) if valid_memory_promotion(approval) => {
            for (key, value) in [
                ("memory_id", approval.memory_id.as_str()),
                (
                    "memory_candidate_sha256",
                    approval.candidate_sha256.as_str(),
                ),
                ("memory_decision_sha256", approval.decision_sha256.as_str()),
                (
                    "memory_content_sha256",
                    approval.approved_content_sha256.as_str(),
                ),
            ] {
                let expected = format!("{key}: {}", yaml_scalar(value)?);
                if normalized.lines().filter(|line| *line == expected).count() != 1 {
                    return Ok(false);
                }
            }
            let marker = "## Memory\n";
            let next = "\n## Sources\n";
            if normalized.matches(marker).count() != 1 || normalized.matches(next).count() != 1 {
                return Ok(false);
            }
            let Some(after_heading) = normalized.split_once(marker).map(|(_, body)| body) else {
                return Ok(false);
            };
            let Some(body) = after_heading
                .split_once(next)
                .map(|(body, _)| body.strip_suffix('\n').unwrap_or(body))
            else {
                return Ok(false);
            };
            Ok(sha256(body.as_bytes()) == approval.approved_content_sha256)
        }
        (KnowledgeWriteWorkflow::Memory, _) | (_, Some(_)) => Ok(false),
        (_, None) => Ok(!normalized.lines().any(|line| line.starts_with("memory_"))),
    }
}

fn writable_property_key(value: &str) -> bool {
    matches!(
        value,
        "aliases"
            | "due"
            | "evidence"
            | "last_verified_at"
            | "owner"
            | "privacy"
            | "project"
            | "retention"
            | "source"
            | "status"
            | "tags"
            | "title"
            | "updated_at"
    )
}

fn invalid_text(value: &str) -> bool {
    value.contains('\0')
        || value
            .chars()
            .any(|character| character.is_control() && character != '\n')
}

fn normalized_path(path: &WorkspacePath) -> String {
    path.components()
        .iter()
        .map(|component| normalized_name(component.as_str()))
        .collect::<Vec<_>>()
        .join("/")
}

fn normalized_name(value: &str) -> String {
    value.nfkc().flat_map(char::to_lowercase).collect()
}

fn create_preview_digest(
    preview: &KnowledgeNoteCreatePreview,
) -> Result<String, MarkdownWriteError> {
    let bytes = serde_json::to_vec(&(
        &preview.path,
        &preview.stable_id,
        preview.workflow,
        &preview.proposed_source_sha256,
        &preview.expected_namespace_sha256,
        preview.expected_index_revision,
        &preview.verified_wiki_links,
        &preview.fidelity_warnings,
        &preview.memory_promotion,
        sha256(&preview.proposed_markdown),
    ))
    .map_err(|_| MarkdownWriteError::StructuralDrift)?;
    Ok(sha256(&bytes))
}

fn structural_preview_digest(
    preview: &KnowledgeStructuralActionPreview,
) -> Result<String, MarkdownWriteError> {
    let bytes = serde_json::to_vec(&(
        preview.action,
        &preview.stable_id,
        &preview.source_path,
        &preview.destination_path,
        &preview.expected_source_sha256,
        preview.requires_additional_confirmation,
        preview.bulk_action,
    ))
    .map_err(|_| MarkdownWriteError::StructuralDrift)?;
    Ok(sha256(&bytes))
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
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
        DataSensitivity, EvidenceId, EvidenceKind, EvidenceReference, WorkspaceId, WorkspacePath,
    };

    use crate::{
        MemoryCandidate, MemoryScope, UserMemoryDecision, evaluate_memory_candidate,
        resolve_memory_candidate,
    };

    use super::*;

    fn workspace() -> WorkspaceId {
        WorkspaceId::from_raw("workspace-knowledge-write")
    }

    fn path(name: &str) -> WorkspacePath {
        WorkspacePath::new(workspace(), ["notes", name]).expect("path")
    }

    fn namespace() -> KnowledgeNamespaceSnapshot {
        KnowledgeNamespaceSnapshot {
            paths: vec![path("existing.md")],
            stable_ids: vec![KnowledgeRecordId::parse("knowledge-project-001").expect("identity")],
            link_targets: vec!["Project One".to_owned()],
            canonical_snapshot_sha256: "a".repeat(64),
            derived_index_revision: 7,
        }
    }

    fn approved_memory_item() -> MemoryItem {
        let candidate = MemoryCandidate {
            memory_id: crate::MemoryId::parse("memory-approved-001").expect("memory identity"),
            memory_type: MemoryType::Semantic,
            scope: MemoryScope {
                workspace_id: workspace(),
                project_id: Some("project-one".to_owned()),
                conversation_id: None,
            },
            content: "Approved durable memory.".to_owned(),
            fact_key: Some("project.reviewed-fact".to_owned()),
            tags: vec!["reviewed".to_owned()],
            links: Vec::new(),
            evidence: vec![EvidenceReference {
                schema_version: 1,
                evidence_id: EvidenceId::from_raw("evidence-memory-approval"),
                kind: EvidenceKind::Document,
                source_id: "fixture-source".to_owned(),
                object_id: "fixture-object".to_owned(),
                fragment: Some("section-memory".to_owned()),
                content_sha256: "a".repeat(64),
                observed_revision: Some("revision-1".to_owned()),
            }],
            sensitivity: DataSensitivity::Durable,
            confidence_bps: 9_000,
            created_at: "2026-08-14T12:00:00Z".to_owned(),
            expires_at: None,
            inferred_sensitive: false,
            model_requested_promotion: true,
        };
        let policy = evaluate_memory_candidate(&candidate, &[]).expect("memory policy");
        resolve_memory_candidate(
            &candidate,
            &policy,
            UserMemoryDecision::Approve,
            "c".repeat(64),
            "2026-08-14T12:01:00Z".to_owned(),
        )
        .expect("explicit memory approval")
    }

    fn memory_approval() -> KnowledgeMemoryPromotionApproval {
        KnowledgeMemoryPromotionApproval::from_approved_item(&approved_memory_item())
            .expect("promotion binding")
    }

    fn sections(workflow: KnowledgeWriteWorkflow) -> Vec<KnowledgeSectionDraft> {
        required_section_order(workflow)
            .iter()
            .map(|heading| KnowledgeSectionDraft {
                heading: (*heading).to_owned(),
                body: if *heading == "Raw Notes" {
                    "Verbatim source material.".to_owned()
                } else if workflow == KnowledgeWriteWorkflow::Memory && *heading == "Memory" {
                    "Approved durable memory.".to_owned()
                } else {
                    "Reviewed content linked to [[Project One]].".to_owned()
                },
            })
            .collect()
    }

    fn request(workflow: KnowledgeWriteWorkflow) -> KnowledgeNoteCreateRequest {
        KnowledgeNoteCreateRequest {
            path: path(&format!("{}.md", workflow_wire(workflow))),
            stable_id: KnowledgeRecordId::parse(format!(
                "knowledge-{}-001",
                workflow_wire(workflow)
            ))
            .expect("identity"),
            workflow,
            title: "Synthetic note".to_owned(),
            properties: vec![KnowledgeFrontmatterProperty {
                key: "status".to_owned(),
                value: "current".to_owned(),
            }],
            sections: sections(workflow),
            memory_promotion: (workflow == KnowledgeWriteWorkflow::Memory).then(memory_approval),
            line_ending: MarkdownLineEnding::Lf,
        }
    }

    #[test]
    fn every_workflow_renders_one_parseable_link_checked_note() {
        for workflow in [
            KnowledgeWriteWorkflow::Task,
            KnowledgeWriteWorkflow::Decision,
            KnowledgeWriteWorkflow::Commitment,
            KnowledgeWriteWorkflow::Correspondence,
            KnowledgeWriteWorkflow::Meeting,
            KnowledgeWriteWorkflow::Handoff,
            KnowledgeWriteWorkflow::Memory,
        ] {
            let preview = preview_knowledge_note_create(request(workflow), &namespace())
                .expect("create preview");
            assert_eq!(preview.workflow, workflow);
            assert_eq!(preview.verified_wiki_links, ["Project One"]);
            assert!(preview.fidelity_warnings.is_empty());
            let parsed =
                MarkdownDocument::parse(preview.path.clone(), preview.proposed_markdown().to_vec())
                    .expect("parse rendered note");
            assert_eq!(parsed.stable_id(), Some(&preview.stable_id));
            if workflow == KnowledgeWriteWorkflow::Meeting {
                assert!(
                    std::str::from_utf8(preview.proposed_markdown())
                        .expect("utf8")
                        .contains("## Raw Notes\nVerbatim source material.")
                );
            }
        }
    }

    #[test]
    fn memory_workflow_requires_explicit_content_bound_promotion() {
        let mut rejected = approved_memory_item();
        rejected.status = MemoryItemStatus::Rejected;
        assert_eq!(
            KnowledgeMemoryPromotionApproval::from_approved_item(&rejected),
            Err(MarkdownWriteError::ProtectedSource)
        );

        let mut missing = request(KnowledgeWriteWorkflow::Memory);
        missing.memory_promotion = None;
        assert_eq!(
            preview_knowledge_note_create(missing, &namespace()),
            Err(MarkdownWriteError::ProtectedSource)
        );

        let preview =
            preview_knowledge_note_create(request(KnowledgeWriteWorkflow::Memory), &namespace())
                .expect("approved memory preview");
        assert!(verify_knowledge_note_create_preview(&preview).is_ok());
        let rendered = std::str::from_utf8(preview.proposed_markdown()).expect("utf8");
        assert!(rendered.contains("memory_decision_sha256: \""));

        let mut drifted = preview.clone();
        drifted
            .memory_promotion
            .as_mut()
            .expect("promotion")
            .approved_content_sha256 = "d".repeat(64);
        drifted.preview_sha256 = create_preview_digest(&drifted).expect("digest");
        assert_eq!(
            verify_knowledge_note_create_preview(&drifted),
            Err(MarkdownWriteError::StructuralDrift)
        );

        let mut unrelated = request(KnowledgeWriteWorkflow::Decision);
        unrelated.memory_promotion = Some(memory_approval());
        assert_eq!(
            preview_knowledge_note_create(unrelated, &namespace()),
            Err(MarkdownWriteError::ProtectedSource)
        );
    }

    #[test]
    fn create_verification_recomputes_declared_links_from_proposed_bytes() {
        let mut preview =
            preview_knowledge_note_create(request(KnowledgeWriteWorkflow::Decision), &namespace())
                .expect("create preview");
        preview.verified_wiki_links = vec!["Project Two".to_owned()];
        preview.preview_sha256 = create_preview_digest(&preview).expect("digest");
        assert_eq!(
            verify_knowledge_note_create_preview(&preview),
            Err(MarkdownWriteError::StructuralDrift)
        );
    }

    #[test]
    fn path_identity_link_and_schema_collisions_fail_closed() {
        let mut exact = request(KnowledgeWriteWorkflow::Task);
        exact.path = path("existing.md");
        assert_eq!(
            preview_knowledge_note_create(exact, &namespace()),
            Err(MarkdownWriteError::AmbiguousTarget)
        );

        let mut case_namespace = namespace();
        case_namespace.paths = vec![path("TASK.MD")];
        assert_eq!(
            preview_knowledge_note_create(request(KnowledgeWriteWorkflow::Task), &case_namespace),
            Err(MarkdownWriteError::AmbiguousTarget)
        );

        let mut unicode = request(KnowledgeWriteWorkflow::Task);
        unicode.path = path("R\u{e9}sum\u{e9}.md");
        let mut unicode_namespace = namespace();
        unicode_namespace.paths = vec![path("R\u{c9}SUM\u{c9}.MD")];
        assert_eq!(
            preview_knowledge_note_create(unicode, &unicode_namespace),
            Err(MarkdownWriteError::AmbiguousTarget)
        );

        let mut duplicate_id = request(KnowledgeWriteWorkflow::Task);
        duplicate_id.stable_id =
            KnowledgeRecordId::parse("knowledge-project-001").expect("identity");
        assert_eq!(
            preview_knowledge_note_create(duplicate_id, &namespace()),
            Err(MarkdownWriteError::InvalidIdentity)
        );

        let mut broken = request(KnowledgeWriteWorkflow::Task);
        broken.sections[0].body = "Unknown [[Missing Note]].".to_owned();
        assert_eq!(
            preview_knowledge_note_create(broken, &namespace()),
            Err(MarkdownWriteError::AmbiguousTarget)
        );

        let mut hidden = request(KnowledgeWriteWorkflow::Task);
        hidden.properties[0].key = "hidden_prompt".to_owned();
        assert_eq!(
            preview_knowledge_note_create(hidden, &namespace()),
            Err(MarkdownWriteError::FrontmatterDenied)
        );
    }

    #[test]
    fn structural_actions_are_single_file_and_always_high_risk() {
        let create =
            preview_knowledge_note_create(request(KnowledgeWriteWorkflow::Meeting), &namespace())
                .expect("create");
        let document =
            MarkdownDocument::parse(create.path.clone(), create.proposed_markdown().to_vec())
                .expect("document");
        for action in [
            KnowledgeStructuralActionKind::Move,
            KnowledgeStructuralActionKind::Rename,
            KnowledgeStructuralActionKind::TrashDelete,
        ] {
            let preview = preview_knowledge_structural_action(
                &document,
                action,
                Some(path(&format!("destination-{}.md", action_wire(action)))),
            )
            .expect("structural preview");
            assert!(preview.requires_additional_confirmation);
            assert!(!preview.bulk_action);
        }
        let supersede = preview_knowledge_structural_action(
            &document,
            KnowledgeStructuralActionKind::Supersede,
            None,
        )
        .expect("supersede");
        assert!(supersede.requires_additional_confirmation);
        assert_eq!(
            preview_knowledge_structural_action(
                &document,
                KnowledgeStructuralActionKind::Move,
                None,
            ),
            Err(MarkdownWriteError::StructuralDrift)
        );
    }

    #[test]
    fn index_publication_is_ready_only_for_exact_verified_commit() {
        let preview =
            preview_knowledge_note_create(request(KnowledgeWriteWorkflow::Decision), &namespace())
                .expect("create");
        let mutation = CanonicalKnowledgeMutation::from_create(&preview);
        let committed = decide_index_publication(
            mutation.clone(),
            CanonicalMarkdownWriteOutcome::Committed,
            Some(preview.proposed_source_sha256.clone()),
        );
        assert_eq!(
            committed.state,
            KnowledgeIndexPublicationState::ReadyAfterCommit
        );
        for (outcome, observed) in [
            (
                CanonicalMarkdownWriteOutcome::Committed,
                Some("b".repeat(64)),
            ),
            (CanonicalMarkdownWriteOutcome::FailedNoChange, None),
            (
                CanonicalMarkdownWriteOutcome::Uncertain,
                Some(preview.proposed_source_sha256.clone()),
            ),
        ] {
            assert_ne!(
                decide_index_publication(mutation.clone(), outcome, observed).state,
                KnowledgeIndexPublicationState::ReadyAfterCommit
            );
        }
    }

    const fn action_wire(action: KnowledgeStructuralActionKind) -> &'static str {
        match action {
            KnowledgeStructuralActionKind::Move => "move",
            KnowledgeStructuralActionKind::Rename => "rename",
            KnowledgeStructuralActionKind::Supersede => "supersede",
            KnowledgeStructuralActionKind::TrashDelete => "trash",
        }
    }
}
