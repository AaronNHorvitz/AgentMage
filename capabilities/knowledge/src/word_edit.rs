//! Exact, proposal-only WordprocessingML replacement, redline, and comment edits.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;

use agentmage_kernel_contracts::{CONTRACT_SCHEMA_VERSION, WorkspacePath};
use serde::{Deserialize, Serialize};

use crate::word_generation::{valid_identifier, xml_escape, zip_parts};
use crate::word_ooxml::{admitted_docx_parts, word_sha256};
use crate::{
    GeneratedWordPart, WordConversionProfile, WordInspectionReport, WordPartSourceRange,
    extract_docx_to_sidecar, inspect_docx,
};

const EDITOR_ID: &str =
    "agentmage-word-edit-v1;exact-fragments=true;comments=true;redlines=true;effects=denied";
const MAX_OPERATIONS: usize = 4_096;
const MAX_TEXT_BYTES: usize = 262_144;

/// Exact source-fragment identity required by every edit operation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WordEditTarget {
    /// Stable extraction fragment identity.
    pub fragment_id: String,
    /// Exact source package-part byte range.
    pub source_range: WordPartSourceRange,
    /// Exact decoded text observed during extraction.
    pub expected_text: String,
}

/// Explicit metadata attached to a tracked replacement.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WordRedlineMetadata {
    /// Caller-selected positive revision identity.
    pub revision_id: u32,
    /// Visible author display value.
    pub author: String,
}

/// Explicit metadata attached to a Word comment.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WordCommentMetadata {
    /// Caller-selected comment identity.
    pub comment_id: u32,
    /// Visible author display value.
    pub author: String,
    /// Visible author initials.
    pub initials: String,
    /// Exact comment text.
    pub comment_text: String,
}

/// One closed exact edit operation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum WordEditOperation {
    /// Replaces only one exact text event.
    ReplaceText {
        /// Stable operation identity.
        operation_id: String,
        /// Exact source target.
        target: WordEditTarget,
        /// Replacement text encoded inertly into OOXML.
        replacement_text: String,
    },
    /// Replaces one simple text run with explicit deletion and insertion elements.
    RedlineReplace {
        /// Stable operation identity.
        operation_id: String,
        /// Exact source target.
        target: WordEditTarget,
        /// Replacement text encoded inertly into OOXML.
        replacement_text: String,
        /// Explicit tracked-change metadata.
        redline: WordRedlineMetadata,
    },
    /// Adds a comment range and body around a simple main-document run.
    AddComment {
        /// Stable operation identity.
        operation_id: String,
        /// Exact source target.
        target: WordEditTarget,
        /// Explicit comment metadata.
        comment: WordCommentMetadata,
    },
}

impl WordEditOperation {
    fn operation_id(&self) -> &str {
        match self {
            Self::ReplaceText { operation_id, .. }
            | Self::RedlineReplace { operation_id, .. }
            | Self::AddComment { operation_id, .. } => operation_id,
        }
    }

    fn target(&self) -> &WordEditTarget {
        match self {
            Self::ReplaceText { target, .. }
            | Self::RedlineReplace { target, .. }
            | Self::AddComment { target, .. } => target,
        }
    }

    fn kind(&self) -> WordEditOperationKind {
        match self {
            Self::ReplaceText { .. } => WordEditOperationKind::ReplaceText,
            Self::RedlineReplace { .. } => WordEditOperationKind::RedlineReplace,
            Self::AddComment { .. } => WordEditOperationKind::AddComment,
        }
    }
}

/// Closed edit-operation class used by receipts.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WordEditOperationKind {
    /// Exact text replacement.
    ReplaceText,
    /// Tracked deletion plus insertion.
    RedlineReplace,
    /// Comment range and comment body addition.
    AddComment,
}

/// Complete exact Word edit request.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WordEditRequest {
    /// Stable edit transaction identity.
    pub edit_id: String,
    /// Exact immutable source path.
    pub source_path: WorkspacePath,
    /// Exact immutable source package digest.
    pub source_sha256: String,
    /// Distinct proposed output path.
    pub output_path: WorkspacePath,
    /// Canonically ordered edit operations.
    pub operations: Vec<WordEditOperation>,
}

/// One exact changed-package-part receipt.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WordEditChange {
    /// Stable operation identity or supporting-part identity.
    pub change_id: String,
    /// Closed edit class, absent only for supporting OOXML parts.
    pub operation_kind: Option<WordEditOperationKind>,
    /// Canonical package part name.
    pub part_name: String,
    /// Exact original content digest, or absent for a newly created part.
    pub before_sha256: Option<String>,
    /// Exact proposed content digest.
    pub after_sha256: String,
    /// Exact original target range, or absent for a supporting-part update.
    pub original_range: Option<WordPartSourceRange>,
}

/// One unchanged package-part receipt.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PreservedWordPart {
    /// Canonical package part name.
    pub part_name: String,
    /// Exact digest shared by source and proposal.
    pub content_sha256: String,
}

/// Closed edit limitation class.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WordPackageEditWarningKind {
    /// ZIP packaging metadata changes while unchanged part content remains exact.
    PackageContainerRebuilt,
    /// A redline operation replaces one complete simple run.
    SimpleRunRedlineOnly,
    /// Comment creation is limited to simple runs in the main document.
    MainDocumentCommentOnly,
    /// Visual fidelity still requires the external renderer workflow.
    VisualVerificationRequired,
}

/// One explicit edit limitation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WordPackageEditWarning {
    /// Closed warning class.
    pub kind: WordPackageEditWarningKind,
    /// Stable content-free reason code.
    pub reason_code: String,
    /// Always true; the immutable original remains authoritative until approval.
    pub original_remains_authoritative: bool,
}

/// Complete exact Word package edit preview.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WordPackageEditPreview {
    /// Contract schema version.
    pub schema_version: u16,
    /// Stable edit transaction identity.
    pub edit_id: String,
    /// Exact source path.
    pub source_path: WorkspacePath,
    /// Exact source package digest.
    pub source_sha256: String,
    /// Distinct proposed output path.
    pub output_path: WorkspacePath,
    /// Exact editor identity digest.
    pub editor_identity_sha256: String,
    /// Canonically ordered changed-part ledger.
    pub changes: Vec<WordEditChange>,
    /// Canonically ordered unchanged-part ledger.
    pub preserved_parts: Vec<PreservedWordPart>,
    /// Exact deterministic proposed package bytes.
    pub package: Vec<u8>,
    /// Exact proposed package digest.
    pub package_sha256: String,
    /// Canonically ordered proposed-part ledger.
    pub parts: Vec<GeneratedWordPart>,
    /// Explicit edit and fidelity warnings.
    pub warnings: Vec<WordPackageEditWarning>,
    /// Reopened bounded structural inspection.
    pub inspection: WordInspectionReport,
    /// Always true; no source bytes are mutated.
    pub original_preserved: bool,
    /// Always true; package bytes are not persisted.
    pub proposal_only: bool,
    /// Always false; the preview has no filesystem authority.
    pub filesystem_effect_performed: bool,
    /// Always false; the preview resolves no external relationship.
    pub network_access_performed: bool,
    /// Always false; the preview executes no document content.
    pub execution_performed: bool,
}

/// Word edit validation or package failure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WordPackageEditorError {
    /// Request fields, target text, or operation metadata are invalid.
    InvalidInput,
    /// The source digest or target fragment is stale.
    StaleSource,
    /// Operations overlap or reuse a comment or revision identity.
    Conflict,
    /// The target is not a supported simple text run.
    UnsupportedTarget,
    /// The source or result package failed bounded structural admission.
    Package,
    /// A supplied preview differs from deterministic recomputation.
    PreviewMismatch,
}

impl fmt::Display for WordPackageEditorError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InvalidInput => "invalid Word edit input",
            Self::StaleSource => "stale Word edit source",
            Self::Conflict => "conflicting Word edit operations",
            Self::UnsupportedTarget => "unsupported Word edit target",
            Self::Package => "Word edit package failure",
            Self::PreviewMismatch => "Word edit preview mismatch",
        })
    }
}

impl Error for WordPackageEditorError {}

fn validate_text(value: &str) -> Result<(), WordPackageEditorError> {
    if value.is_empty()
        || value.len() > MAX_TEXT_BYTES
        || value
            .chars()
            .any(|character| character.is_control() && !matches!(character, '\t' | '\n' | '\r'))
    {
        Err(WordPackageEditorError::InvalidInput)
    } else {
        Ok(())
    }
}

fn validate_request(
    request: &WordEditRequest,
    source: &[u8],
) -> Result<(), WordPackageEditorError> {
    if request.source_sha256 != word_sha256(source) {
        return Err(WordPackageEditorError::StaleSource);
    }
    if !valid_identifier(&request.edit_id)
        || request.source_path == request.output_path
        || request.operations.is_empty()
        || request.operations.len() > MAX_OPERATIONS
    {
        return Err(WordPackageEditorError::InvalidInput);
    }
    let mut operation_ids = BTreeSet::new();
    let mut comment_ids = BTreeSet::new();
    let mut revision_ids = BTreeSet::new();
    let mut previous = "";
    for operation in &request.operations {
        let operation_id = operation.operation_id();
        if !valid_identifier(operation_id)
            || !operation_ids.insert(operation_id)
            || (!previous.is_empty() && operation_id <= previous)
            || !valid_identifier(&operation.target().fragment_id)
            || operation.target().source_range.end_byte
                <= operation.target().source_range.start_byte
        {
            return Err(WordPackageEditorError::Conflict);
        }
        previous = operation_id;
        validate_text(&operation.target().expected_text)?;
        match operation {
            WordEditOperation::ReplaceText {
                replacement_text, ..
            } => validate_text(replacement_text)?,
            WordEditOperation::RedlineReplace {
                replacement_text,
                redline,
                ..
            } => {
                validate_text(replacement_text)?;
                validate_text(&redline.author)?;
                if redline.revision_id == 0 || !revision_ids.insert(redline.revision_id) {
                    return Err(WordPackageEditorError::Conflict);
                }
            }
            WordEditOperation::AddComment { comment, .. } => {
                validate_text(&comment.author)?;
                validate_text(&comment.initials)?;
                validate_text(&comment.comment_text)?;
                if comment.initials.len() > 16 || !comment_ids.insert(comment.comment_id) {
                    return Err(WordPackageEditorError::Conflict);
                }
            }
        }
    }
    Ok(())
}

fn find_bytes(haystack: &[u8], needle: &[u8], start: usize) -> Option<usize> {
    haystack
        .get(start..)?
        .windows(needle.len())
        .position(|window| window == needle)
        .map(|offset| start + offset)
}

fn rfind_bytes(haystack: &[u8], needle: &[u8], end: usize) -> Option<usize> {
    haystack
        .get(..end)?
        .windows(needle.len())
        .rposition(|window| window == needle)
}

fn enclosing_simple_run(
    part: &[u8],
    range: &WordPartSourceRange,
) -> Result<(usize, usize, String), WordPackageEditorError> {
    let start =
        usize::try_from(range.start_byte).map_err(|_| WordPackageEditorError::InvalidInput)?;
    let end = usize::try_from(range.end_byte).map_err(|_| WordPackageEditorError::InvalidInput)?;
    let run_start = match (
        rfind_bytes(part, b"<w:r>", start),
        rfind_bytes(part, b"<w:r ", start),
    ) {
        (Some(left), Some(right)) => left.max(right),
        (Some(value), None) | (None, Some(value)) => value,
        (None, None) => return Err(WordPackageEditorError::UnsupportedTarget),
    };
    let run_end = find_bytes(part, b"</w:r>", end)
        .and_then(|value| value.checked_add(b"</w:r>".len()))
        .ok_or(WordPackageEditorError::UnsupportedTarget)?;
    let run = part
        .get(run_start..run_end)
        .ok_or(WordPackageEditorError::UnsupportedTarget)?;
    let run_text =
        std::str::from_utf8(run).map_err(|_| WordPackageEditorError::UnsupportedTarget)?;
    for unsupported in ["<w:drawing", "<w:object", "<w:fld", "<w:instrText", "<w:br"] {
        if run_text.contains(unsupported) {
            return Err(WordPackageEditorError::UnsupportedTarget);
        }
    }
    if run_text.matches("<w:t").count() + run_text.matches("<w:delText").count() != 1 {
        return Err(WordPackageEditorError::UnsupportedTarget);
    }
    let properties = if let Some(start) = run_text.find("<w:rPr") {
        let end = run_text[start..]
            .find("</w:rPr>")
            .map(|offset| start + offset + "</w:rPr>".len())
            .ok_or(WordPackageEditorError::UnsupportedTarget)?;
        run_text[start..end].to_owned()
    } else {
        String::new()
    };
    Ok((run_start, run_end, properties))
}

fn replace_range(part: &mut Vec<u8>, start: usize, end: usize, replacement: &[u8]) {
    part.splice(start..end, replacement.iter().copied());
}

fn insert_before_closing(
    part: &mut Vec<u8>,
    closing: &[u8],
    insertion: &[u8],
) -> Result<(), WordPackageEditorError> {
    let index = rfind_bytes(part, closing, part.len()).ok_or(WordPackageEditorError::Package)?;
    part.splice(index..index, insertion.iter().copied());
    Ok(())
}

fn support_comments_part(
    parts: &mut BTreeMap<String, Vec<u8>>,
    comments: &[(u32, String, String, String)],
) -> Result<(), WordPackageEditorError> {
    if comments.is_empty() {
        return Ok(());
    }
    let content_types = parts
        .get_mut("[Content_Types].xml")
        .ok_or(WordPackageEditorError::Package)?;
    if !content_types
        .windows(b"/word/comments.xml".len())
        .any(|window| window == b"/word/comments.xml")
    {
        insert_before_closing(
            content_types,
            b"</Types>",
            b"<Override PartName=\"/word/comments.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.wordprocessingml.comments+xml\"/>",
        )?;
    }
    let relationships = parts
        .get_mut("word/_rels/document.xml.rels")
        .ok_or(WordPackageEditorError::Package)?;
    if !relationships
        .windows(b"relationships/comments".len())
        .any(|window| window == b"relationships/comments")
    {
        if relationships
            .windows(b"rIdAgentMageComments".len())
            .any(|window| window == b"rIdAgentMageComments")
        {
            return Err(WordPackageEditorError::Conflict);
        }
        insert_before_closing(
            relationships,
            b"</Relationships>",
            b"<Relationship Id=\"rIdAgentMageComments\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/comments\" Target=\"comments.xml\"/>",
        )?;
    }
    let comments_part = parts
        .entry("word/comments.xml".to_owned())
        .or_insert_with(|| {
            b"<?xml version=\"1.0\" encoding=\"UTF-8\"?><w:comments xmlns:w=\"http://schemas.openxmlformats.org/wordprocessingml/2006/main\"></w:comments>".to_vec()
        });
    for (comment_id, author, initials, text) in comments {
        if String::from_utf8_lossy(comments_part).contains(&format!("w:id=\"{comment_id}\"")) {
            return Err(WordPackageEditorError::Conflict);
        }
        let entry = format!(
            "<w:comment w:id=\"{comment_id}\" w:author=\"{}\" w:initials=\"{}\"><w:p><w:r><w:t xml:space=\"preserve\">{}</w:t></w:r></w:p></w:comment>",
            xml_escape(author),
            xml_escape(initials),
            xml_escape(text)
        );
        insert_before_closing(comments_part, b"</w:comments>", entry.as_bytes())?;
    }
    Ok(())
}

fn warning(kind: WordPackageEditWarningKind, reason: &str) -> WordPackageEditWarning {
    WordPackageEditWarning {
        kind,
        reason_code: reason.to_owned(),
        original_remains_authoritative: true,
    }
}

/// Creates a deterministic exact edit preview without mutating source or destination.
pub fn preview_word_package_edit(
    request: &WordEditRequest,
    source: &[u8],
    profile: &WordConversionProfile,
) -> Result<WordPackageEditPreview, WordPackageEditorError> {
    validate_request(request, source)?;
    let (_, original_parts) = admitted_docx_parts(&request.source_path, source, profile)
        .map_err(|_| WordPackageEditorError::Package)?;
    let extraction = extract_docx_to_sidecar(&request.source_path, source, profile)
        .map_err(|_| WordPackageEditorError::Package)?;
    let fragments = extraction
        .fragments
        .iter()
        .map(|fragment| (fragment.fragment_id.as_str(), fragment))
        .collect::<BTreeMap<_, _>>();
    let mut parts = original_parts.clone();
    let mut operations_by_part: BTreeMap<String, Vec<&WordEditOperation>> = BTreeMap::new();
    for operation in &request.operations {
        let target = operation.target();
        let fragment = fragments
            .get(target.fragment_id.as_str())
            .ok_or(WordPackageEditorError::StaleSource)?;
        if fragment.source_range != target.source_range || fragment.text != target.expected_text {
            return Err(WordPackageEditorError::StaleSource);
        }
        if matches!(operation, WordEditOperation::AddComment { .. })
            && target.source_range.part_name != "word/document.xml"
        {
            return Err(WordPackageEditorError::UnsupportedTarget);
        }
        operations_by_part
            .entry(target.source_range.part_name.clone())
            .or_default()
            .push(operation);
    }
    let mut changes = Vec::new();
    let mut comments = Vec::new();
    for (part_name, operations) in &mut operations_by_part {
        let part = parts
            .get_mut(part_name)
            .ok_or(WordPackageEditorError::StaleSource)?;
        let before_sha256 = word_sha256(part);
        let mut expanded = Vec::new();
        for operation in operations.iter() {
            let target = operation.target();
            let start = usize::try_from(target.source_range.start_byte)
                .map_err(|_| WordPackageEditorError::InvalidInput)?;
            let end = usize::try_from(target.source_range.end_byte)
                .map_err(|_| WordPackageEditorError::InvalidInput)?;
            let (edit_start, edit_end) = match operation {
                WordEditOperation::ReplaceText { .. } => (start, end),
                WordEditOperation::RedlineReplace { .. } | WordEditOperation::AddComment { .. } => {
                    let (run_start, run_end, _) = enclosing_simple_run(part, &target.source_range)?;
                    (run_start, run_end)
                }
            };
            expanded.push((edit_start, edit_end, *operation));
        }
        expanded.sort_by_key(|(start, _, _)| *start);
        if expanded.windows(2).any(|items| items[0].1 > items[1].0) {
            return Err(WordPackageEditorError::Conflict);
        }
        for (start, end, operation) in expanded.into_iter().rev() {
            let target = operation.target();
            match operation {
                WordEditOperation::ReplaceText {
                    replacement_text, ..
                } => replace_range(part, start, end, xml_escape(replacement_text).as_bytes()),
                WordEditOperation::RedlineReplace {
                    replacement_text,
                    redline,
                    ..
                } => {
                    let (_, _, properties) = enclosing_simple_run(part, &target.source_range)?;
                    let replacement = format!(
                        "<w:del w:id=\"{}\" w:author=\"{}\"><w:r>{properties}<w:delText xml:space=\"preserve\">{}</w:delText></w:r></w:del><w:ins w:id=\"{}\" w:author=\"{}\"><w:r>{properties}<w:t xml:space=\"preserve\">{}</w:t></w:r></w:ins>",
                        redline.revision_id,
                        xml_escape(&redline.author),
                        xml_escape(&target.expected_text),
                        redline.revision_id,
                        xml_escape(&redline.author),
                        xml_escape(replacement_text)
                    );
                    replace_range(part, start, end, replacement.as_bytes());
                }
                WordEditOperation::AddComment { comment, .. } => {
                    let original_run = part[start..end].to_vec();
                    let mut replacement =
                        format!("<w:commentRangeStart w:id=\"{}\"/>", comment.comment_id)
                            .into_bytes();
                    replacement.extend_from_slice(&original_run);
                    replacement.extend_from_slice(
                        format!(
                            "<w:commentRangeEnd w:id=\"{}\"/><w:r><w:commentReference w:id=\"{}\"/></w:r>",
                            comment.comment_id, comment.comment_id
                        )
                        .as_bytes(),
                    );
                    replace_range(part, start, end, &replacement);
                    comments.push((
                        comment.comment_id,
                        comment.author.clone(),
                        comment.initials.clone(),
                        comment.comment_text.clone(),
                    ));
                }
            }
            changes.push(WordEditChange {
                change_id: operation.operation_id().to_owned(),
                operation_kind: Some(operation.kind()),
                part_name: part_name.clone(),
                before_sha256: Some(before_sha256.clone()),
                after_sha256: String::new(),
                original_range: Some(target.source_range.clone()),
            });
        }
        let after_sha256 = word_sha256(part);
        for change in changes
            .iter_mut()
            .filter(|item| item.part_name == *part_name)
        {
            change.after_sha256.clone_from(&after_sha256);
        }
    }
    comments.sort_by_key(|item| item.0);
    support_comments_part(&mut parts, &comments)?;
    let touched = changes
        .iter()
        .map(|item| item.part_name.clone())
        .collect::<BTreeSet<_>>();
    for (part_name, content) in &parts {
        if original_parts.get(part_name) != Some(content) && !touched.contains(part_name) {
            changes.push(WordEditChange {
                change_id: format!("support:{}", part_name.replace(['/', '.'], ":")),
                operation_kind: None,
                part_name: part_name.clone(),
                before_sha256: original_parts
                    .get(part_name)
                    .map(|value| word_sha256(value)),
                after_sha256: word_sha256(content),
                original_range: None,
            });
        }
    }
    changes.sort_by(|left, right| left.change_id.cmp(&right.change_id));
    let preserved_parts = original_parts
        .iter()
        .filter(|(name, content)| parts.get(*name) == Some(*content))
        .map(|(part_name, content)| PreservedWordPart {
            part_name: part_name.clone(),
            content_sha256: word_sha256(content),
        })
        .collect::<Vec<_>>();
    let part_ledger = parts
        .iter()
        .map(|(part_name, content)| GeneratedWordPart {
            part_name: part_name.clone(),
            content_sha256: word_sha256(content),
            bytes: content.len() as u64,
        })
        .collect::<Vec<_>>();
    let package = zip_parts(&parts).map_err(|_| WordPackageEditorError::Package)?;
    let inspection = inspect_docx(&request.output_path, &package, profile)
        .map_err(|_| WordPackageEditorError::Package)?;
    if inspection.quarantined || !inspection.inspection_complete {
        return Err(WordPackageEditorError::Package);
    }
    let mut warning_kinds = BTreeMap::from([
        (
            WordPackageEditWarningKind::PackageContainerRebuilt,
            "word.edit.package-container-rebuilt",
        ),
        (
            WordPackageEditWarningKind::VisualVerificationRequired,
            "word.edit.visual-verification-required",
        ),
    ]);
    if request
        .operations
        .iter()
        .any(|item| matches!(item, WordEditOperation::RedlineReplace { .. }))
    {
        warning_kinds.insert(
            WordPackageEditWarningKind::SimpleRunRedlineOnly,
            "word.edit.simple-run-redline-only",
        );
    }
    if !comments.is_empty() {
        warning_kinds.insert(
            WordPackageEditWarningKind::MainDocumentCommentOnly,
            "word.edit.main-document-comment-only",
        );
    }
    Ok(WordPackageEditPreview {
        schema_version: CONTRACT_SCHEMA_VERSION,
        edit_id: request.edit_id.clone(),
        source_path: request.source_path.clone(),
        source_sha256: request.source_sha256.clone(),
        output_path: request.output_path.clone(),
        editor_identity_sha256: word_sha256(EDITOR_ID.as_bytes()),
        changes,
        preserved_parts,
        package_sha256: word_sha256(&package),
        package,
        parts: part_ledger,
        warnings: warning_kinds
            .into_iter()
            .map(|(kind, reason)| warning(kind, reason))
            .collect(),
        inspection,
        original_preserved: true,
        proposal_only: true,
        filesystem_effect_performed: false,
        network_access_performed: false,
        execution_performed: false,
    })
}

/// Recomputes an edit preview from immutable inputs and requires exact equality.
pub fn verify_word_package_edit_preview(
    request: &WordEditRequest,
    source: &[u8],
    profile: &WordConversionProfile,
    preview: &WordPackageEditPreview,
) -> Result<(), WordPackageEditorError> {
    let expected = preview_word_package_edit(request, source, profile)?;
    if &expected == preview {
        Ok(())
    } else {
        Err(WordPackageEditorError::PreviewMismatch)
    }
}

#[cfg(test)]
mod tests {
    use agentmage_kernel_contracts::{WorkspaceId, WorkspacePath};

    use super::*;
    use crate::{MarkdownDocument, WordFeatureKind, generate_docx_from_markdown};

    fn path(name: &str) -> WorkspacePath {
        WorkspacePath::new(WorkspaceId::from_raw("workspace-word-edit"), ["docs", name])
            .expect("path")
    }

    fn source() -> (Vec<u8>, crate::WordExtractionResult) {
        let markdown = MarkdownDocument::parse(
            path("source.md"),
            b"# Report\n\nAlpha\n\nBeta\n\nGamma\n".to_vec(),
        )
        .expect("Markdown");
        let package = generate_docx_from_markdown(
            "edit-source",
            path("source.docx"),
            &markdown,
            &WordConversionProfile::strict_default(),
        )
        .expect("generate")
        .package;
        let extraction = extract_docx_to_sidecar(
            &path("source.docx"),
            &package,
            &WordConversionProfile::strict_default(),
        )
        .expect("extract");
        (package, extraction)
    }

    fn target(fragment: &crate::WordTextFragment) -> WordEditTarget {
        WordEditTarget {
            fragment_id: fragment.fragment_id.clone(),
            source_range: fragment.source_range.clone(),
            expected_text: fragment.text.clone(),
        }
    }

    #[test]
    fn replacement_redline_and_comment_are_exact_reopenable_and_effect_free() {
        let (source, extraction) = source();
        let request = WordEditRequest {
            edit_id: "edit-1".to_owned(),
            source_path: path("source.docx"),
            source_sha256: word_sha256(&source),
            output_path: path("edited.docx"),
            operations: vec![
                WordEditOperation::ReplaceText {
                    operation_id: "change-1".to_owned(),
                    target: target(&extraction.fragments[1]),
                    replacement_text: "Alpha <script>& revised".to_owned(),
                },
                WordEditOperation::RedlineReplace {
                    operation_id: "change-2".to_owned(),
                    target: target(&extraction.fragments[2]),
                    replacement_text: "Beta revised".to_owned(),
                    redline: WordRedlineMetadata {
                        revision_id: 7,
                        author: "Reviewer".to_owned(),
                    },
                },
                WordEditOperation::AddComment {
                    operation_id: "change-3".to_owned(),
                    target: target(&extraction.fragments[3]),
                    comment: WordCommentMetadata {
                        comment_id: 9,
                        author: "Reviewer".to_owned(),
                        initials: "RV".to_owned(),
                        comment_text: "Verify this statement.".to_owned(),
                    },
                },
            ],
        };
        let profile = WordConversionProfile::strict_default();
        let first = preview_word_package_edit(&request, &source, &profile).expect("preview");
        let second = preview_word_package_edit(&request, &source, &profile).expect("preview");
        assert_eq!(first, second);
        verify_word_package_edit_preview(&request, &source, &profile, &first).expect("verify");
        assert_eq!(
            first
                .changes
                .iter()
                .filter(|item| item.operation_kind.is_some())
                .count(),
            3
        );
        assert!(first.original_preserved && first.proposal_only);
        assert!(!first.filesystem_effect_performed);
        assert!(!first.network_access_performed);
        assert!(!first.execution_performed);
        let (_, source_parts) =
            admitted_docx_parts(&request.source_path, &source, &profile).expect("source parts");
        let (_, edited_parts) = admitted_docx_parts(&request.output_path, &first.package, &profile)
            .expect("edited parts");
        for preserved in &first.preserved_parts {
            assert_eq!(
                source_parts.get(&preserved.part_name),
                edited_parts.get(&preserved.part_name)
            );
            assert_eq!(
                source_parts
                    .get(&preserved.part_name)
                    .map(|content| word_sha256(content)),
                Some(preserved.content_sha256.clone())
            );
        }
        for feature in [
            WordFeatureKind::Comment,
            WordFeatureKind::TrackedInsertion,
            WordFeatureKind::TrackedDeletion,
        ] {
            assert!(
                first
                    .inspection
                    .features
                    .iter()
                    .any(|item| item.kind == feature)
            );
        }
        let reopened = extract_docx_to_sidecar(&path("edited.docx"), &first.package, &profile)
            .expect("extract edited");
        let text = String::from_utf8(reopened.sidecar).expect("UTF-8");
        for expected in [
            "Alpha <script>& revised",
            "Beta",
            "Beta revised",
            "Verify this statement.",
        ] {
            assert!(text.contains(expected), "missing {expected}: {text}");
        }
    }

    #[test]
    fn stale_overlapping_unresolved_and_source_overwrite_edits_fail_closed() {
        let (source, extraction) = source();
        let profile = WordConversionProfile::strict_default();
        let base = WordEditRequest {
            edit_id: "edit-1".to_owned(),
            source_path: path("source.docx"),
            source_sha256: word_sha256(&source),
            output_path: path("edited.docx"),
            operations: vec![WordEditOperation::ReplaceText {
                operation_id: "change-1".to_owned(),
                target: target(&extraction.fragments[1]),
                replacement_text: "Changed".to_owned(),
            }],
        };
        let mut stale = base.clone();
        stale.source_sha256 = "0".repeat(64);
        assert_eq!(
            preview_word_package_edit(&stale, &source, &profile),
            Err(WordPackageEditorError::StaleSource)
        );
        let mut overwrite = base.clone();
        overwrite.output_path = overwrite.source_path.clone();
        assert_eq!(
            preview_word_package_edit(&overwrite, &source, &profile),
            Err(WordPackageEditorError::InvalidInput)
        );
        let mut unresolved = base.clone();
        unresolved.operations[0] = WordEditOperation::ReplaceText {
            operation_id: "change-1".to_owned(),
            target: WordEditTarget {
                expected_text: "stale".to_owned(),
                ..target(&extraction.fragments[1])
            },
            replacement_text: "Changed".to_owned(),
        };
        assert_eq!(
            preview_word_package_edit(&unresolved, &source, &profile),
            Err(WordPackageEditorError::StaleSource)
        );
        let mut overlapping = base.clone();
        overlapping.operations.push(WordEditOperation::AddComment {
            operation_id: "change-2".to_owned(),
            target: target(&extraction.fragments[1]),
            comment: WordCommentMetadata {
                comment_id: 1,
                author: "A".to_owned(),
                initials: "A".to_owned(),
                comment_text: "Comment".to_owned(),
            },
        });
        assert_eq!(
            preview_word_package_edit(&overlapping, &source, &profile),
            Err(WordPackageEditorError::Conflict)
        );
        let preview = preview_word_package_edit(&base, &source, &profile).expect("preview");
        let mut changed = preview.clone();
        changed.package_sha256 = "f".repeat(64);
        assert_eq!(
            verify_word_package_edit_preview(&base, &source, &profile, &changed),
            Err(WordPackageEditorError::PreviewMismatch)
        );
    }

    #[test]
    fn comment_relationship_identity_collision_fails_closed() {
        let (source, _) = source();
        let profile = WordConversionProfile::strict_default();
        let (_, mut parts) =
            admitted_docx_parts(&path("source.docx"), &source, &profile).expect("source parts");
        let relationships = parts
            .get_mut("word/_rels/document.xml.rels")
            .expect("relationships");
        insert_before_closing(
            relationships,
            b"</Relationships>",
            b"<Relationship Id=\"rIdAgentMageComments\" Type=\"urn:unrelated\" Target=\"unrelated.xml\"/>",
        )
        .expect("insert collision");
        let collided = zip_parts(&parts).expect("package");
        let extraction =
            extract_docx_to_sidecar(&path("source.docx"), &collided, &profile).expect("extract");
        let request = WordEditRequest {
            edit_id: "edit-comment-collision".to_owned(),
            source_path: path("source.docx"),
            source_sha256: word_sha256(&collided),
            output_path: path("edited.docx"),
            operations: vec![WordEditOperation::AddComment {
                operation_id: "change-1".to_owned(),
                target: target(&extraction.fragments[1]),
                comment: WordCommentMetadata {
                    comment_id: 1,
                    author: "Reviewer".to_owned(),
                    initials: "RV".to_owned(),
                    comment_text: "Review".to_owned(),
                },
            }],
        };
        assert_eq!(
            preview_word_package_edit(&request, &collided, &profile),
            Err(WordPackageEditorError::Conflict)
        );
    }
}
