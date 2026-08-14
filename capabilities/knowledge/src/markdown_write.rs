//! Authority-free, byte-preserving Markdown parsing and edit previews.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

use agentmage_kernel_contracts::WorkspacePath;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{KnowledgeRecord, KnowledgeRecordId, validate_record};

const MAX_MARKDOWN_BYTES: usize = 4 * 1024 * 1024;
const MAX_ELEMENTS: usize = 100_000;
const MAX_REPLACEMENT_BYTES: usize = 1024 * 1024;
const MAX_FRONTMATTER_FIELDS: usize = 128;

/// Closed parser or edit-preview failure without source content.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MarkdownWriteError {
    /// Input or replacement bytes exceeded a fixed bound.
    ResourceLimit,
    /// Input was not supported, deterministic UTF-8 Markdown.
    MalformedMarkdown,
    /// A stable note identity was absent, changed, or malformed.
    InvalidIdentity,
    /// The expected source or target bytes changed.
    StaleSource,
    /// A requested structure was absent or ambiguous.
    AmbiguousTarget,
    /// The request attempted to alter protected source material.
    ProtectedSource,
    /// A frontmatter field is not in the closed writable set.
    FrontmatterDenied,
    /// The replacement did not retain the requested structural kind.
    StructuralDrift,
}

impl MarkdownWriteError {
    /// Returns a stable content-free failure code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::ResourceLimit => "knowledge.markdown.resource_limit",
            Self::MalformedMarkdown => "knowledge.markdown.malformed",
            Self::InvalidIdentity => "knowledge.markdown.invalid_identity",
            Self::StaleSource => "knowledge.markdown.stale_source",
            Self::AmbiguousTarget => "knowledge.markdown.ambiguous_target",
            Self::ProtectedSource => "knowledge.markdown.protected_source",
            Self::FrontmatterDenied => "knowledge.markdown.frontmatter_denied",
            Self::StructuralDrift => "knowledge.markdown.structural_drift",
        }
    }
}

impl std::fmt::Display for MarkdownWriteError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.code())
    }
}

impl std::error::Error for MarkdownWriteError {}

/// Exact line-ending convention observed in one source file.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MarkdownLineEnding {
    /// No line terminator was present.
    None,
    /// Unix line feeds were used consistently.
    Lf,
    /// Carriage-return plus line-feed pairs were used consistently.
    CrLf,
}

impl MarkdownLineEnding {
    pub(crate) fn bytes(self) -> &'static [u8] {
        match self {
            Self::None | Self::Lf => b"\n",
            Self::CrLf => b"\r\n",
        }
    }
}

/// Structural element classes that can be targeted without free-form parsing.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MarkdownElementKind {
    /// YAML-compatible frontmatter delimiter or field.
    Frontmatter,
    /// ATX heading.
    Heading,
    /// Unordered or ordered list item.
    ListItem,
    /// Markdown task-list item.
    Task,
    /// Pipe-delimited table row.
    TableRow,
    /// Opening or closing fenced-code delimiter.
    CodeFence,
    /// Content inside a fenced-code region.
    CodeFenceContent,
    /// Inline Markdown link.
    MarkdownLink,
    /// Obsidian-style wiki link.
    WikiLink,
    /// Ordinary non-empty text block line.
    TextBlock,
    /// Blank source line.
    Blank,
}

/// Exact byte and one-based line range in one immutable source snapshot.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MarkdownSourceRange {
    /// Inclusive source byte offset.
    pub start_byte: usize,
    /// Exclusive source byte offset.
    pub end_byte: usize,
    /// Inclusive one-based start line.
    pub start_line: u32,
    /// Inclusive one-based end line.
    pub end_line: u32,
}

impl MarkdownSourceRange {
    fn intersects(self, other: Self) -> bool {
        self.start_byte < other.end_byte && other.start_byte < self.end_byte
    }
}

/// One parsed structural element with exact source location.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MarkdownElement {
    /// Closed structural class.
    pub kind: MarkdownElementKind,
    /// Exact source range including the line terminator when present.
    pub source_range: MarkdownSourceRange,
    /// Heading level for headings, otherwise none.
    pub heading_level: Option<u8>,
    /// Bounded structural label without raw fenced content.
    pub label: String,
}

/// Visible fidelity condition retained on every preview.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MarkdownFidelityWarning {
    /// A duplicate heading makes a heading-name target ambiguous.
    DuplicateHeading,
    /// Raw HTML was retained byte-for-byte but not semantically interpreted.
    RawHtml,
    /// A reference-style link was retained but not semantically interpreted.
    ReferenceStyleLink,
    /// A setext-like heading was retained but not semantically interpreted.
    SetextHeading,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct FrontmatterField {
    key: String,
    value: String,
    range: MarkdownSourceRange,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct HeadingRegion {
    text: String,
    level: u8,
    heading_range: MarkdownSourceRange,
    body_range: MarkdownSourceRange,
}

/// Immutable, authority-free Markdown parse retaining the exact source bytes.
#[derive(Clone, PartialEq, Eq)]
pub struct MarkdownDocument {
    path: WorkspacePath,
    source_sha256: String,
    stable_id: Option<KnowledgeRecordId>,
    line_ending: MarkdownLineEnding,
    elements: Vec<MarkdownElement>,
    frontmatter: Vec<FrontmatterField>,
    frontmatter_close: Option<MarkdownSourceRange>,
    headings: Vec<HeadingRegion>,
    raw_notes: Vec<MarkdownSourceRange>,
    warnings: Vec<MarkdownFidelityWarning>,
    source: Vec<u8>,
}

impl std::fmt::Debug for MarkdownDocument {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("MarkdownDocument")
            .field("path", &self.path)
            .field("source_sha256", &self.source_sha256)
            .field("stable_id", &self.stable_id)
            .field("line_ending", &self.line_ending)
            .field("elements", &self.elements.len())
            .field("warnings", &self.warnings)
            .finish_non_exhaustive()
    }
}

impl MarkdownDocument {
    /// Parses one exact bounded source snapshot without normalizing any byte.
    pub fn parse(path: WorkspacePath, source: Vec<u8>) -> Result<Self, MarkdownWriteError> {
        parse_document(path, source)
    }

    /// Returns the exact source digest.
    #[must_use]
    pub fn source_sha256(&self) -> &str {
        &self.source_sha256
    }

    /// Returns the exact canonical workspace-relative path.
    #[must_use]
    pub const fn path(&self) -> &WorkspacePath {
        &self.path
    }

    /// Returns the stable note identity when one is declared in frontmatter.
    #[must_use]
    pub const fn stable_id(&self) -> Option<&KnowledgeRecordId> {
        self.stable_id.as_ref()
    }

    /// Returns the exact observed line-ending convention.
    #[must_use]
    pub const fn line_ending(&self) -> MarkdownLineEnding {
        self.line_ending
    }

    /// Returns structural elements in source order.
    #[must_use]
    pub fn elements(&self) -> &[MarkdownElement] {
        &self.elements
    }

    /// Returns visible fidelity warnings in stable order.
    #[must_use]
    pub fn fidelity_warnings(&self) -> &[MarkdownFidelityWarning] {
        &self.warnings
    }

    /// Returns exact source bytes for an approval-bound filesystem proposal.
    #[must_use]
    pub fn source_bytes(&self) -> &[u8] {
        &self.source
    }
}

/// One closed structure-preserving Markdown edit.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MarkdownEdit {
    /// Set or insert one approved scalar frontmatter field.
    SetFrontmatterScalar {
        /// Closed field key.
        key: String,
        /// Expected logical scalar value, or none when inserting.
        expected_value: Option<String>,
        /// Proposed logical scalar value.
        value: String,
    },
    /// Replace the body under one unique ATX heading.
    ReplaceHeadingBody {
        /// Exact heading text.
        heading: String,
        /// Exact heading level.
        level: u8,
        /// Replacement using logical `\n` line separators.
        replacement: String,
    },
    /// Replace one complete structural line while retaining its kind.
    ReplaceStructuralLine {
        /// One-based target line.
        line_number: u32,
        /// Required current structural kind.
        kind: MarkdownElementKind,
        /// Exact current line without its terminator.
        expected_line: String,
        /// Exact replacement line without a terminator.
        replacement_line: String,
    },
    /// Replace one exact UTF-8 text span within an ordinary text block.
    ReplaceBoundedText {
        /// One-based target line.
        line_number: u32,
        /// One-based byte column.
        start_column: u32,
        /// Exclusive one-based byte column.
        end_column: u32,
        /// Exact expected text.
        expected_text: String,
        /// Replacement text without line separators.
        replacement_text: String,
    },
}

/// Exact update request bound to one source digest and stable note identity.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MarkdownUpdateRequest {
    /// Expected exact source digest.
    pub expected_source_sha256: String,
    /// Expected stable note identity.
    pub expected_stable_id: KnowledgeRecordId,
    /// One closed edit.
    pub edit: MarkdownEdit,
}

/// Authority-free exact Markdown update preview.
#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MarkdownUpdatePreview {
    /// Exact canonical path.
    pub path: WorkspacePath,
    /// Stable note identity.
    pub stable_id: KnowledgeRecordId,
    /// Exact current source digest.
    pub expected_source_sha256: String,
    /// Exact proposed source digest.
    pub proposed_source_sha256: String,
    /// Exact changed source range.
    pub changed_range: MarkdownSourceRange,
    /// Hash of the byte-preserved prefix.
    pub preserved_prefix_sha256: String,
    /// Hash of the byte-preserved suffix.
    pub preserved_suffix_sha256: String,
    /// True when unsupported-but-preserved syntax requires visible review.
    pub requires_fidelity_confirmation: bool,
    /// Complete visible fidelity warnings.
    pub fidelity_warnings: Vec<MarkdownFidelityWarning>,
    /// Exact proposed Markdown bytes.
    #[serde(skip_serializing)]
    proposed_markdown: Vec<u8>,
    /// Hash binding every displayed preview field and proposed bytes.
    pub preview_sha256: String,
}

impl std::fmt::Debug for MarkdownUpdatePreview {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("MarkdownUpdatePreview")
            .field("path", &self.path)
            .field("stable_id", &self.stable_id)
            .field("expected_source_sha256", &self.expected_source_sha256)
            .field("proposed_source_sha256", &self.proposed_source_sha256)
            .field("changed_range", &self.changed_range)
            .field("preview_sha256", &self.preview_sha256)
            .finish_non_exhaustive()
    }
}

impl MarkdownUpdatePreview {
    /// Returns the exact proposed bytes for a separately authorized filesystem plan.
    #[must_use]
    pub fn proposed_markdown(&self) -> &[u8] {
        &self.proposed_markdown
    }
}

/// Builds one exact byte-preserving update preview without filesystem authority.
pub fn preview_markdown_update(
    document: &MarkdownDocument,
    request: MarkdownUpdateRequest,
) -> Result<MarkdownUpdatePreview, MarkdownWriteError> {
    if request.expected_source_sha256 != document.source_sha256 {
        return Err(MarkdownWriteError::StaleSource);
    }
    if document.stable_id.as_ref() != Some(&request.expected_stable_id) {
        return Err(MarkdownWriteError::InvalidIdentity);
    }
    let (changed_range, replacement) = edit_replacement(document, &request.edit)?;
    if replacement.len() > MAX_REPLACEMENT_BYTES
        || changed_range.end_byte > document.source.len()
        || changed_range.start_byte > changed_range.end_byte
    {
        return Err(MarkdownWriteError::ResourceLimit);
    }
    if document
        .raw_notes
        .iter()
        .any(|protected| changed_range.intersects(*protected))
    {
        return Err(MarkdownWriteError::ProtectedSource);
    }
    let prefix = &document.source[..changed_range.start_byte];
    let suffix = &document.source[changed_range.end_byte..];
    let mut proposed = Vec::with_capacity(prefix.len() + replacement.len() + suffix.len());
    proposed.extend_from_slice(prefix);
    proposed.extend_from_slice(&replacement);
    proposed.extend_from_slice(suffix);
    if proposed == document.source || proposed.len() > MAX_MARKDOWN_BYTES {
        return Err(MarkdownWriteError::StructuralDrift);
    }
    let reparsed = MarkdownDocument::parse(document.path.clone(), proposed.clone())?;
    if reparsed.stable_id != document.stable_id || reparsed.line_ending != document.line_ending {
        return Err(MarkdownWriteError::StructuralDrift);
    }
    verify_raw_notes_unchanged(document, &reparsed)?;
    verify_requested_structure(&request.edit, &reparsed)?;
    let mut preview = MarkdownUpdatePreview {
        path: document.path.clone(),
        stable_id: request.expected_stable_id,
        expected_source_sha256: document.source_sha256.clone(),
        proposed_source_sha256: sha256(&proposed),
        changed_range,
        preserved_prefix_sha256: sha256(prefix),
        preserved_suffix_sha256: sha256(suffix),
        requires_fidelity_confirmation: !document.warnings.is_empty(),
        fidelity_warnings: document.warnings.clone(),
        proposed_markdown: proposed,
        preview_sha256: String::new(),
    };
    preview.preview_sha256 = preview_digest(&preview)?;
    Ok(preview)
}

#[derive(Clone, Copy)]
struct SourceLine<'source> {
    number: u32,
    start: usize,
    end: usize,
    content: &'source str,
}

fn parse_document(
    path: WorkspacePath,
    source: Vec<u8>,
) -> Result<MarkdownDocument, MarkdownWriteError> {
    if source.is_empty() || source.len() > MAX_MARKDOWN_BYTES || source.contains(&0) {
        return Err(MarkdownWriteError::MalformedMarkdown);
    }
    let line_ending = detect_line_ending(&source)?;
    let text = std::str::from_utf8(&source).map_err(|_| MarkdownWriteError::MalformedMarkdown)?;
    let bom_bytes = usize::from(text.starts_with('\u{feff}')) * '\u{feff}'.len_utf8();
    let lines = source_lines(text, bom_bytes)?;
    let (frontmatter, frontmatter_close, body_start) = parse_frontmatter(&lines)?;
    let stable_id = stable_identity(&frontmatter)?;
    let mut elements = Vec::new();
    let mut heading_points = Vec::new();
    let mut warnings = BTreeSet::new();
    let mut fence: Option<(u8, usize)> = None;
    for line in &lines[body_start..] {
        if let Some((marker, count)) = fence_marker(line.content) {
            match fence {
                Some((active, minimum)) if marker == active && count >= minimum => fence = None,
                None => fence = Some((marker, count)),
                Some(_) => {}
            }
            push_element(
                &mut elements,
                MarkdownElementKind::CodeFence,
                *line,
                None,
                "fence",
            )?;
            continue;
        }
        if fence.is_some() {
            push_element(
                &mut elements,
                MarkdownElementKind::CodeFenceContent,
                *line,
                None,
                "[fenced-content]",
            )?;
            continue;
        }
        if let Some((level, heading)) = atx_heading(line.content) {
            push_element(
                &mut elements,
                MarkdownElementKind::Heading,
                *line,
                Some(level),
                heading,
            )?;
            heading_points.push((heading.to_owned(), level, *line));
        } else if task_line(line.content) {
            push_element(
                &mut elements,
                MarkdownElementKind::Task,
                *line,
                None,
                line.content.trim(),
            )?;
        } else if list_line(line.content) {
            push_element(
                &mut elements,
                MarkdownElementKind::ListItem,
                *line,
                None,
                line.content.trim(),
            )?;
        } else if table_line(line.content) {
            push_element(
                &mut elements,
                MarkdownElementKind::TableRow,
                *line,
                None,
                line.content.trim(),
            )?;
        } else if line.content.trim().is_empty() {
            push_element(&mut elements, MarkdownElementKind::Blank, *line, None, "")?;
        } else {
            push_element(
                &mut elements,
                MarkdownElementKind::TextBlock,
                *line,
                None,
                bounded_label(line.content),
            )?;
        }
        if line.content.contains("[[") {
            if !balanced_markers(line.content, "[[", "]]") {
                return Err(MarkdownWriteError::MalformedMarkdown);
            }
            push_element(
                &mut elements,
                MarkdownElementKind::WikiLink,
                *line,
                None,
                "wiki-link",
            )?;
        }
        if line.content.contains("](") {
            if !balanced_markdown_links(line.content) {
                return Err(MarkdownWriteError::MalformedMarkdown);
            }
            push_element(
                &mut elements,
                MarkdownElementKind::MarkdownLink,
                *line,
                None,
                "markdown-link",
            )?;
        }
        if line.content.trim_start().starts_with('<') {
            warnings.insert(MarkdownFidelityWarning::RawHtml);
        }
        if reference_style_link(line.content) {
            warnings.insert(MarkdownFidelityWarning::ReferenceStyleLink);
        }
    }
    if fence.is_some() {
        return Err(MarkdownWriteError::MalformedMarkdown);
    }
    for pair in lines[body_start..].windows(2) {
        if !pair[0].content.trim().is_empty() && matches!(pair[1].content.trim(), "===" | "---") {
            warnings.insert(MarkdownFidelityWarning::SetextHeading);
        }
    }
    let headings = heading_regions(&heading_points, source.len());
    let mut counts = BTreeMap::new();
    for heading in &headings {
        *counts.entry(heading.text.to_lowercase()).or_insert(0_u32) += 1;
    }
    if counts.values().any(|count| *count > 1) {
        warnings.insert(MarkdownFidelityWarning::DuplicateHeading);
    }
    let raw_notes = headings
        .iter()
        .filter(|heading| heading.text.eq_ignore_ascii_case("raw notes"))
        .map(|heading| MarkdownSourceRange {
            start_byte: heading.heading_range.start_byte,
            end_byte: heading.body_range.end_byte,
            start_line: heading.heading_range.start_line,
            end_line: heading.body_range.end_line,
        })
        .collect::<Vec<_>>();
    Ok(MarkdownDocument {
        path,
        source_sha256: sha256(&source),
        stable_id,
        line_ending,
        elements,
        frontmatter,
        frontmatter_close,
        headings,
        raw_notes,
        warnings: warnings.into_iter().collect(),
        source,
    })
}

fn detect_line_ending(source: &[u8]) -> Result<MarkdownLineEnding, MarkdownWriteError> {
    let mut observed = None;
    for (index, byte) in source.iter().enumerate() {
        if *byte == b'\r' && source.get(index + 1) != Some(&b'\n') {
            return Err(MarkdownWriteError::MalformedMarkdown);
        }
        if *byte == b'\n' {
            let current = if index > 0 && source[index - 1] == b'\r' {
                MarkdownLineEnding::CrLf
            } else {
                MarkdownLineEnding::Lf
            };
            if observed.is_some_and(|value| value != current) {
                return Err(MarkdownWriteError::MalformedMarkdown);
            }
            observed = Some(current);
        }
    }
    Ok(observed.unwrap_or(MarkdownLineEnding::None))
}

fn source_lines(text: &str, bom_bytes: usize) -> Result<Vec<SourceLine<'_>>, MarkdownWriteError> {
    let bytes = text.as_bytes();
    let mut lines = Vec::new();
    let mut start = bom_bytes;
    let mut number = 1_u32;
    while start < bytes.len() {
        let newline = bytes[start..]
            .iter()
            .position(|byte| *byte == b'\n')
            .map(|offset| start + offset);
        let end = newline.map_or(bytes.len(), |index| index + 1);
        let content_end = newline.map_or(bytes.len(), |index| {
            if index > start && bytes[index - 1] == b'\r' {
                index - 1
            } else {
                index
            }
        });
        let content = text
            .get(start..content_end)
            .ok_or(MarkdownWriteError::MalformedMarkdown)?;
        lines.push(SourceLine {
            number,
            start,
            end,
            content,
        });
        number = number
            .checked_add(1)
            .ok_or(MarkdownWriteError::ResourceLimit)?;
        start = end;
    }
    if lines.is_empty() {
        return Err(MarkdownWriteError::MalformedMarkdown);
    }
    Ok(lines)
}

fn parse_frontmatter(
    lines: &[SourceLine<'_>],
) -> Result<(Vec<FrontmatterField>, Option<MarkdownSourceRange>, usize), MarkdownWriteError> {
    if lines.first().map(|line| line.content) != Some("---") {
        return Ok((Vec::new(), None, 0));
    }
    let close_index = lines
        .iter()
        .enumerate()
        .skip(1)
        .find_map(|(index, line)| (line.content == "---").then_some(index))
        .ok_or(MarkdownWriteError::MalformedMarkdown)?;
    let mut fields = Vec::new();
    let mut keys = BTreeSet::new();
    let mut index = 1;
    while index < close_index {
        let line = lines[index];
        let trimmed = line.content.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            index += 1;
            continue;
        }
        if line.content.contains('\t') || line.content.starts_with(' ') {
            return Err(MarkdownWriteError::MalformedMarkdown);
        }
        let (key, raw_value) = line
            .content
            .split_once(':')
            .ok_or(MarkdownWriteError::MalformedMarkdown)?;
        if !valid_frontmatter_key(key) || !keys.insert(key.to_ascii_lowercase()) {
            return Err(MarkdownWriteError::MalformedMarkdown);
        }
        let raw_value = raw_value.trim();
        let (value, end_line) = if raw_value.is_empty() {
            let mut values = Vec::new();
            let mut end_line = line;
            index += 1;
            while index < close_index && lines[index].content.starts_with(' ') {
                let sequence_line = lines[index];
                if sequence_line.content.contains('\t') {
                    return Err(MarkdownWriteError::MalformedMarkdown);
                }
                let item = sequence_line
                    .content
                    .trim_start()
                    .strip_prefix("- ")
                    .ok_or(MarkdownWriteError::MalformedMarkdown)?;
                values.push(parse_frontmatter_scalar(item)?);
                end_line = sequence_line;
                index += 1;
            }
            if values.is_empty() || values.len() > MAX_FRONTMATTER_FIELDS {
                return Err(MarkdownWriteError::MalformedMarkdown);
            }
            (
                serde_json::to_string(&values)
                    .map_err(|_| MarkdownWriteError::MalformedMarkdown)?,
                end_line,
            )
        } else {
            index += 1;
            (parse_frontmatter_scalar(raw_value)?, line)
        };
        fields.push(FrontmatterField {
            key: key.to_ascii_lowercase(),
            value,
            range: MarkdownSourceRange {
                start_byte: line.start,
                end_byte: end_line.end,
                start_line: line.number,
                end_line: end_line.number,
            },
        });
        if fields.len() > MAX_FRONTMATTER_FIELDS {
            return Err(MarkdownWriteError::ResourceLimit);
        }
    }
    Ok((
        fields,
        Some(line_range(lines[close_index])),
        close_index + 1,
    ))
}

fn stable_identity(
    frontmatter: &[FrontmatterField],
) -> Result<Option<KnowledgeRecordId>, MarkdownWriteError> {
    for key in ["agentmage_id", "id"] {
        if let Some(field) = frontmatter.iter().find(|field| field.key == key) {
            return KnowledgeRecordId::parse(field.value.clone())
                .map(Some)
                .map_err(|_| MarkdownWriteError::InvalidIdentity);
        }
    }
    if let Some(field) = frontmatter
        .iter()
        .find(|field| field.key == "agentmage_record")
    {
        let record: KnowledgeRecord =
            serde_json::from_str(&field.value).map_err(|_| MarkdownWriteError::InvalidIdentity)?;
        validate_record(&record).map_err(|_| MarkdownWriteError::InvalidIdentity)?;
        return Ok(Some(record.record_id));
    }
    Ok(None)
}

fn parse_frontmatter_scalar(value: &str) -> Result<String, MarkdownWriteError> {
    if value.is_empty() || value.len() > 32 * 1024 || value.chars().any(char::is_control) {
        return Err(MarkdownWriteError::MalformedMarkdown);
    }
    if value.starts_with('"') {
        serde_json::from_str::<String>(value).map_err(|_| MarkdownWriteError::MalformedMarkdown)
    } else if value.starts_with('\'') && value.ends_with('\'') && value.len() >= 2 {
        let inner = &value[1..value.len() - 1];
        if inner.contains('\'') {
            return Err(MarkdownWriteError::MalformedMarkdown);
        }
        Ok(inner.to_owned())
    } else {
        Ok(value.to_owned())
    }
}

fn valid_frontmatter_key(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'_' | b'-')
        })
}

fn writable_frontmatter_key(value: &str) -> bool {
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
            | "type"
            | "updated_at"
    )
}

fn push_element(
    elements: &mut Vec<MarkdownElement>,
    kind: MarkdownElementKind,
    line: SourceLine<'_>,
    heading_level: Option<u8>,
    label: &str,
) -> Result<(), MarkdownWriteError> {
    elements.push(MarkdownElement {
        kind,
        source_range: line_range(line),
        heading_level,
        label: bounded_label(label).to_owned(),
    });
    if elements.len() > MAX_ELEMENTS {
        return Err(MarkdownWriteError::ResourceLimit);
    }
    Ok(())
}

fn line_range(line: SourceLine<'_>) -> MarkdownSourceRange {
    MarkdownSourceRange {
        start_byte: line.start,
        end_byte: line.end,
        start_line: line.number,
        end_line: line.number,
    }
}

fn bounded_label(value: &str) -> &str {
    let mut end = value.len().min(512);
    while !value.is_char_boundary(end) {
        end -= 1;
    }
    &value[..end]
}

fn fence_marker(line: &str) -> Option<(u8, usize)> {
    let trimmed = line.trim_start();
    let marker = *trimmed.as_bytes().first()?;
    if !matches!(marker, b'`' | b'~') {
        return None;
    }
    let count = trimmed
        .as_bytes()
        .iter()
        .take_while(|byte| **byte == marker)
        .count();
    (count >= 3).then_some((marker, count))
}

fn atx_heading(line: &str) -> Option<(u8, &str)> {
    let trimmed = line.trim_start();
    let count = trimmed.bytes().take_while(|byte| *byte == b'#').count();
    if !(1..=6).contains(&count) || trimmed.as_bytes().get(count) != Some(&b' ') {
        return None;
    }
    let text = trimmed[count + 1..].trim_end_matches('#').trim();
    (!text.is_empty()).then_some((u8::try_from(count).ok()?, text))
}

fn task_line(line: &str) -> bool {
    let trimmed = line.trim_start();
    ["- [ ] ", "- [x] ", "- [X] ", "* [ ] ", "+ [ ] "]
        .iter()
        .any(|prefix| trimmed.starts_with(prefix))
}

fn list_line(line: &str) -> bool {
    let trimmed = line.trim_start();
    if ["- ", "* ", "+ "]
        .iter()
        .any(|prefix| trimmed.starts_with(prefix))
    {
        return true;
    }
    let digits = trimmed.bytes().take_while(u8::is_ascii_digit).count();
    digits > 0 && trimmed.as_bytes().get(digits..digits + 2) == Some(b". ")
}

fn table_line(line: &str) -> bool {
    let trimmed = line.trim();
    trimmed.starts_with('|') && trimmed.ends_with('|') && trimmed.matches('|').count() >= 3
}

fn balanced_markers(line: &str, open: &str, close: &str) -> bool {
    let mut offset = 0;
    while let Some(start) = line[offset..].find(open) {
        let content = offset + start + open.len();
        let Some(end) = line[content..].find(close) else {
            return false;
        };
        if line[content..content + end].trim().is_empty() {
            return false;
        }
        offset = content + end + close.len();
    }
    true
}

fn balanced_markdown_links(line: &str) -> bool {
    let mut offset = 0;
    while let Some(start) = line[offset..].find("](") {
        let content = offset + start + 2;
        let Some(end) = line[content..].find(')') else {
            return false;
        };
        if line[content..content + end].trim().is_empty() {
            return false;
        }
        offset = content + end + 1;
    }
    true
}

fn reference_style_link(line: &str) -> bool {
    line.contains("][") || (line.trim_start().starts_with('[') && line.contains("]: "))
}

fn heading_regions(
    headings: &[(String, u8, SourceLine<'_>)],
    source_len: usize,
) -> Vec<HeadingRegion> {
    headings
        .iter()
        .enumerate()
        .map(|(index, (text, level, line))| {
            let next = headings[index + 1..]
                .iter()
                .find(|(_, candidate_level, _)| candidate_level <= level)
                .map(|(_, _, candidate)| candidate.start)
                .unwrap_or(source_len);
            HeadingRegion {
                text: text.clone(),
                level: *level,
                heading_range: line_range(*line),
                body_range: MarkdownSourceRange {
                    start_byte: line.end,
                    end_byte: next,
                    start_line: line.number.saturating_add(1),
                    end_line: headings[index + 1..]
                        .iter()
                        .find(|(_, candidate_level, _)| candidate_level <= level)
                        .map_or_else(
                            || line.number.saturating_add(1),
                            |(_, _, candidate)| candidate.number.saturating_sub(1),
                        ),
                },
            }
        })
        .collect()
}

fn edit_replacement(
    document: &MarkdownDocument,
    edit: &MarkdownEdit,
) -> Result<(MarkdownSourceRange, Vec<u8>), MarkdownWriteError> {
    match edit {
        MarkdownEdit::SetFrontmatterScalar {
            key,
            expected_value,
            value,
        } => frontmatter_replacement(document, key, expected_value.as_deref(), value),
        MarkdownEdit::ReplaceHeadingBody {
            heading,
            level,
            replacement,
        } => heading_replacement(document, heading, *level, replacement),
        MarkdownEdit::ReplaceStructuralLine {
            line_number,
            kind,
            expected_line,
            replacement_line,
        } => line_replacement(
            document,
            *line_number,
            *kind,
            expected_line,
            replacement_line,
        ),
        MarkdownEdit::ReplaceBoundedText {
            line_number,
            start_column,
            end_column,
            expected_text,
            replacement_text,
        } => bounded_text_replacement(
            document,
            *line_number,
            *start_column,
            *end_column,
            expected_text,
            replacement_text,
        ),
    }
}

fn frontmatter_replacement(
    document: &MarkdownDocument,
    key: &str,
    expected: Option<&str>,
    value: &str,
) -> Result<(MarkdownSourceRange, Vec<u8>), MarkdownWriteError> {
    if !valid_frontmatter_key(key)
        || !writable_frontmatter_key(key)
        || value.is_empty()
        || value.len() > 32 * 1024
        || value.chars().any(char::is_control)
    {
        return Err(MarkdownWriteError::FrontmatterDenied);
    }
    let encoded =
        serde_json::to_string(value).map_err(|_| MarkdownWriteError::FrontmatterDenied)?;
    let line = format!("{key}: {encoded}");
    if let Some(field) = document.frontmatter.iter().find(|field| field.key == key) {
        if expected != Some(field.value.as_str()) {
            return Err(MarkdownWriteError::StaleSource);
        }
        let mut replacement = line.into_bytes();
        replacement.extend_from_slice(terminator_for_range(document, field.range));
        return Ok((field.range, replacement));
    }
    if expected.is_some() {
        return Err(MarkdownWriteError::StaleSource);
    }
    let close = document
        .frontmatter_close
        .ok_or(MarkdownWriteError::FrontmatterDenied)?;
    let insertion = MarkdownSourceRange {
        start_byte: close.start_byte,
        end_byte: close.start_byte,
        start_line: close.start_line,
        end_line: close.start_line,
    };
    let mut replacement = line.into_bytes();
    replacement.extend_from_slice(document.line_ending.bytes());
    Ok((insertion, replacement))
}

fn heading_replacement(
    document: &MarkdownDocument,
    heading: &str,
    level: u8,
    replacement: &str,
) -> Result<(MarkdownSourceRange, Vec<u8>), MarkdownWriteError> {
    if heading.is_empty()
        || !(1..=6).contains(&level)
        || replacement.len() > MAX_REPLACEMENT_BYTES
        || replacement.contains('\r')
        || replacement.contains('\0')
        || heading.eq_ignore_ascii_case("raw notes")
    {
        return Err(MarkdownWriteError::ProtectedSource);
    }
    let matches = document
        .headings
        .iter()
        .filter(|candidate| candidate.text == heading && candidate.level == level)
        .collect::<Vec<_>>();
    if matches.len() != 1 {
        return Err(MarkdownWriteError::AmbiguousTarget);
    }
    let range = matches[0].body_range;
    let normalized = normalize_replacement(
        replacement,
        document.line_ending,
        range.end_byte < document.source.len(),
    );
    Ok((range, normalized))
}

fn line_replacement(
    document: &MarkdownDocument,
    line_number: u32,
    kind: MarkdownElementKind,
    expected: &str,
    replacement: &str,
) -> Result<(MarkdownSourceRange, Vec<u8>), MarkdownWriteError> {
    if matches!(
        kind,
        MarkdownElementKind::Frontmatter
            | MarkdownElementKind::Heading
            | MarkdownElementKind::CodeFence
            | MarkdownElementKind::CodeFenceContent
            | MarkdownElementKind::Blank
    ) || replacement.contains(['\r', '\n', '\0'])
        || replacement.len() > MAX_REPLACEMENT_BYTES
    {
        return Err(MarkdownWriteError::StructuralDrift);
    }
    let element = document
        .elements
        .iter()
        .find(|element| element.kind == kind && element.source_range.start_line == line_number)
        .ok_or(MarkdownWriteError::AmbiguousTarget)?;
    let range = element.source_range;
    let current = content_without_terminator(&document.source[range.start_byte..range.end_byte]);
    if current != expected.as_bytes() {
        return Err(MarkdownWriteError::StaleSource);
    }
    let mut bytes = replacement.as_bytes().to_vec();
    bytes.extend_from_slice(terminator_for_range(document, range));
    Ok((range, bytes))
}

fn bounded_text_replacement(
    document: &MarkdownDocument,
    line_number: u32,
    start_column: u32,
    end_column: u32,
    expected: &str,
    replacement: &str,
) -> Result<(MarkdownSourceRange, Vec<u8>), MarkdownWriteError> {
    if start_column == 0
        || end_column <= start_column
        || replacement.contains(['\r', '\n', '\0'])
        || replacement.len() > MAX_REPLACEMENT_BYTES
    {
        return Err(MarkdownWriteError::StructuralDrift);
    }
    let element = document
        .elements
        .iter()
        .find(|element| {
            element.kind == MarkdownElementKind::TextBlock
                && element.source_range.start_line == line_number
        })
        .ok_or(MarkdownWriteError::AmbiguousTarget)?;
    let line = content_without_terminator(
        &document.source[element.source_range.start_byte..element.source_range.end_byte],
    );
    let line_text = std::str::from_utf8(line).map_err(|_| MarkdownWriteError::MalformedMarkdown)?;
    let start = usize::try_from(start_column - 1).map_err(|_| MarkdownWriteError::ResourceLimit)?;
    let end = usize::try_from(end_column - 1).map_err(|_| MarkdownWriteError::ResourceLimit)?;
    if end > line.len()
        || !line_text.is_char_boundary(start)
        || !line_text.is_char_boundary(end)
        || line.get(start..end) != Some(expected.as_bytes())
    {
        return Err(MarkdownWriteError::StaleSource);
    }
    Ok((
        MarkdownSourceRange {
            start_byte: element.source_range.start_byte + start,
            end_byte: element.source_range.start_byte + end,
            start_line: line_number,
            end_line: line_number,
        },
        replacement.as_bytes().to_vec(),
    ))
}

fn normalize_replacement(value: &str, ending: MarkdownLineEnding, require_final: bool) -> Vec<u8> {
    let separator = ending.bytes();
    let mut output = Vec::new();
    for (index, part) in value.split('\n').enumerate() {
        if index > 0 {
            output.extend_from_slice(separator);
        }
        output.extend_from_slice(part.as_bytes());
    }
    if require_final && !output.ends_with(separator) {
        output.extend_from_slice(separator);
    }
    output
}

fn content_without_terminator(value: &[u8]) -> &[u8] {
    value
        .strip_suffix(b"\r\n")
        .or_else(|| value.strip_suffix(b"\n"))
        .unwrap_or(value)
}

fn terminator_for_range(document: &MarkdownDocument, range: MarkdownSourceRange) -> &'static [u8] {
    let value = &document.source[range.start_byte..range.end_byte];
    if value.ends_with(b"\r\n") {
        b"\r\n"
    } else if value.ends_with(b"\n") {
        b"\n"
    } else {
        b""
    }
}

fn verify_raw_notes_unchanged(
    before: &MarkdownDocument,
    after: &MarkdownDocument,
) -> Result<(), MarkdownWriteError> {
    let snapshots = |document: &MarkdownDocument| {
        document
            .raw_notes
            .iter()
            .map(|range| sha256(&document.source[range.start_byte..range.end_byte]))
            .collect::<Vec<_>>()
    };
    if snapshots(before) != snapshots(after) {
        return Err(MarkdownWriteError::ProtectedSource);
    }
    Ok(())
}

fn verify_requested_structure(
    edit: &MarkdownEdit,
    after: &MarkdownDocument,
) -> Result<(), MarkdownWriteError> {
    if let MarkdownEdit::ReplaceStructuralLine {
        line_number,
        kind,
        replacement_line,
        ..
    } = edit
        && !after.elements.iter().any(|element| {
            element.kind == *kind
                && element.source_range.start_line == *line_number
                && content_without_terminator(
                    &after.source[element.source_range.start_byte..element.source_range.end_byte],
                ) == replacement_line.as_bytes()
        })
    {
        return Err(MarkdownWriteError::StructuralDrift);
    }
    Ok(())
}

fn preview_digest(preview: &MarkdownUpdatePreview) -> Result<String, MarkdownWriteError> {
    let json = serde_json::to_vec(&(
        &preview.path,
        &preview.stable_id,
        &preview.expected_source_sha256,
        &preview.proposed_source_sha256,
        preview.changed_range,
        &preview.preserved_prefix_sha256,
        &preview.preserved_suffix_sha256,
        preview.requires_fidelity_confirmation,
        &preview.fidelity_warnings,
        sha256(&preview.proposed_markdown),
    ))
    .map_err(|_| MarkdownWriteError::StructuralDrift)?;
    Ok(sha256(&json))
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
    use agentmage_kernel_contracts::{WorkspaceId, WorkspacePath};

    use super::*;

    fn path() -> WorkspacePath {
        WorkspacePath::new(
            WorkspaceId::from_raw("workspace-markdown"),
            ["notes", "note.md"],
        )
        .expect("path")
    }

    fn source(ending: &str) -> Vec<u8> {
        [
            "---",
            "agentmage_id: knowledge-meeting-001",
            "status: current",
            "---",
            "# Meeting",
            "",
            "Intro with [evidence](local.md) and [[Project One|project]].",
            "",
            "## Summary",
            "Original summary.",
            "",
            "## Actions",
            "- ordinary item",
            "- [ ] owned task",
            "",
            "| Owner | State |",
            "| --- | --- |",
            "| Aaron | Open |",
            "",
            "```text",
            "- [ ] not a task [[not-a-link]]",
            "```",
            "",
            "## Raw Notes",
            "Verbatim material.",
            "",
        ]
        .join(ending)
        .into_bytes()
    }

    fn request(document: &MarkdownDocument, edit: MarkdownEdit) -> MarkdownUpdateRequest {
        MarkdownUpdateRequest {
            expected_source_sha256: document.source_sha256().to_owned(),
            expected_stable_id: document.stable_id().expect("stable identity").clone(),
            edit,
        }
    }

    #[test]
    fn parser_classifies_structures_and_excludes_fenced_content() {
        let document = MarkdownDocument::parse(path(), source("\n")).expect("document");
        assert_eq!(document.line_ending(), MarkdownLineEnding::Lf);
        assert_eq!(
            document.stable_id().map(KnowledgeRecordId::as_str),
            Some("knowledge-meeting-001")
        );
        for kind in [
            MarkdownElementKind::Heading,
            MarkdownElementKind::ListItem,
            MarkdownElementKind::Task,
            MarkdownElementKind::TableRow,
            MarkdownElementKind::CodeFence,
            MarkdownElementKind::CodeFenceContent,
            MarkdownElementKind::MarkdownLink,
            MarkdownElementKind::WikiLink,
            MarkdownElementKind::TextBlock,
        ] {
            assert!(document.elements().iter().any(|item| item.kind == kind));
        }
        assert_eq!(
            document
                .elements()
                .iter()
                .filter(|item| item.kind == MarkdownElementKind::Task)
                .count(),
            1
        );
    }

    #[test]
    fn heading_frontmatter_and_bounded_text_edits_preserve_unrelated_bytes() {
        let document = MarkdownDocument::parse(path(), source("\n")).expect("document");
        let frontmatter = preview_markdown_update(
            &document,
            request(
                &document,
                MarkdownEdit::SetFrontmatterScalar {
                    key: "status".to_owned(),
                    expected_value: Some("current".to_owned()),
                    value: "reviewed".to_owned(),
                },
            ),
        )
        .expect("frontmatter preview");
        assert!(
            std::str::from_utf8(frontmatter.proposed_markdown())
                .expect("utf8")
                .contains("status: \"reviewed\"")
        );

        let heading = preview_markdown_update(
            &document,
            request(
                &document,
                MarkdownEdit::ReplaceHeadingBody {
                    heading: "Summary".to_owned(),
                    level: 2,
                    replacement: "Revised summary.".to_owned(),
                },
            ),
        )
        .expect("heading preview");
        let text = std::str::from_utf8(heading.proposed_markdown()).expect("utf8");
        assert!(text.contains("## Summary\nRevised summary.\n"));
        assert!(text.contains("## Raw Notes\nVerbatim material."));

        let bounded = preview_markdown_update(
            &document,
            request(
                &document,
                MarkdownEdit::ReplaceBoundedText {
                    line_number: 10,
                    start_column: 1,
                    end_column: 9,
                    expected_text: "Original".to_owned(),
                    replacement_text: "Updated".to_owned(),
                },
            ),
        )
        .expect("bounded preview");
        assert!(
            std::str::from_utf8(bounded.proposed_markdown())
                .expect("utf8")
                .contains("Updated summary.")
        );
    }

    #[test]
    fn list_task_table_and_link_lines_retain_kind_and_line_endings() {
        let document = MarkdownDocument::parse(path(), source("\r\n")).expect("document");
        assert_eq!(document.line_ending(), MarkdownLineEnding::CrLf);
        let cases = [
            (
                13,
                MarkdownElementKind::ListItem,
                "- ordinary item",
                "- revised item",
            ),
            (
                14,
                MarkdownElementKind::Task,
                "- [ ] owned task",
                "- [x] owned task",
            ),
            (
                18,
                MarkdownElementKind::TableRow,
                "| Aaron | Open |",
                "| Aaron | Done |",
            ),
            (
                7,
                MarkdownElementKind::MarkdownLink,
                "Intro with [evidence](local.md) and [[Project One|project]].",
                "Intro with [evidence](current.md) and [[Project One|project]].",
            ),
            (
                7,
                MarkdownElementKind::WikiLink,
                "Intro with [evidence](local.md) and [[Project One|project]].",
                "Intro with [evidence](local.md) and [[Project Two|project]].",
            ),
        ];
        for (line_number, kind, expected, replacement) in cases {
            let preview = preview_markdown_update(
                &document,
                request(
                    &document,
                    MarkdownEdit::ReplaceStructuralLine {
                        line_number,
                        kind,
                        expected_line: expected.to_owned(),
                        replacement_line: replacement.to_owned(),
                    },
                ),
            )
            .expect("structural preview");
            assert!(
                preview
                    .proposed_markdown()
                    .windows(2)
                    .any(|value| value == b"\r\n")
            );
            assert!(
                !preview
                    .proposed_markdown()
                    .windows(2)
                    .any(|value| value == b"\n\n")
            );
        }
    }

    #[test]
    fn raw_notes_fences_identity_and_hidden_frontmatter_fail_closed() {
        let document = MarkdownDocument::parse(path(), source("\n")).expect("document");
        let raw = preview_markdown_update(
            &document,
            request(
                &document,
                MarkdownEdit::ReplaceHeadingBody {
                    heading: "Raw Notes".to_owned(),
                    level: 2,
                    replacement: "erased".to_owned(),
                },
            ),
        );
        assert_eq!(raw, Err(MarkdownWriteError::ProtectedSource));
        let fence = preview_markdown_update(
            &document,
            request(
                &document,
                MarkdownEdit::ReplaceStructuralLine {
                    line_number: 21,
                    kind: MarkdownElementKind::Task,
                    expected_line: "- [ ] not a task [[not-a-link]]".to_owned(),
                    replacement_line: "- [x] not a task [[not-a-link]]".to_owned(),
                },
            ),
        );
        assert_eq!(fence, Err(MarkdownWriteError::AmbiguousTarget));
        let hidden = preview_markdown_update(
            &document,
            request(
                &document,
                MarkdownEdit::SetFrontmatterScalar {
                    key: "hidden_prompt".to_owned(),
                    expected_value: None,
                    value: "inject".to_owned(),
                },
            ),
        );
        assert_eq!(hidden, Err(MarkdownWriteError::FrontmatterDenied));

        let mut changed_identity = source("\n");
        let offset = changed_identity
            .windows(b"knowledge-meeting-001".len())
            .position(|value| value == b"knowledge-meeting-001")
            .expect("identity offset");
        changed_identity[offset + b"knowledge-meeting-001".len() - 1] = b'2';
        let changed = MarkdownDocument::parse(path(), changed_identity).expect("changed document");
        assert_eq!(
            preview_markdown_update(
                &changed,
                MarkdownUpdateRequest {
                    expected_source_sha256: changed.source_sha256().to_owned(),
                    expected_stable_id: KnowledgeRecordId::parse("knowledge-meeting-001")
                        .expect("identity"),
                    edit: MarkdownEdit::ReplaceHeadingBody {
                        heading: "Summary".to_owned(),
                        level: 2,
                        replacement: "revised".to_owned(),
                    },
                },
            ),
            Err(MarkdownWriteError::InvalidIdentity)
        );
    }

    #[test]
    fn malformed_duplicate_and_unsupported_syntax_is_refused_or_visible() {
        for malformed in [
            b"---\nid: knowledge-note-001\nid: knowledge-note-002\n---\n# Note\n".to_vec(),
            b"---\nid: knowledge-note-001\n# Note\n".to_vec(),
            b"---\nid: knowledge-note-001\n---\n```\nunterminated\n".to_vec(),
            b"---\nid: knowledge-note-001\n---\nline [[broken\n".to_vec(),
            b"---\r\nid: knowledge-note-001\n---\r\n# Note\r\n".to_vec(),
            vec![0xff, 0xfe],
        ] {
            assert!(MarkdownDocument::parse(path(), malformed).is_err());
        }

        let visible = b"---\nid: knowledge-note-001\n---\n# Same\ntext\n# Same\n<html>\n[ref][id]\nTitle\n===\n".to_vec();
        let document = MarkdownDocument::parse(path(), visible).expect("visible warnings");
        assert_eq!(
            document.fidelity_warnings(),
            &[
                MarkdownFidelityWarning::DuplicateHeading,
                MarkdownFidelityWarning::RawHtml,
                MarkdownFidelityWarning::ReferenceStyleLink,
                MarkdownFidelityWarning::SetextHeading,
            ]
        );
        assert_eq!(document.source_bytes(), b"---\nid: knowledge-note-001\n---\n# Same\ntext\n# Same\n<html>\n[ref][id]\nTitle\n===\n");
    }

    #[test]
    fn frontmatter_sequences_are_retained_and_replaced_as_one_exact_field() {
        let bytes = b"---\nagentmage_id: knowledge-note-001\naliases:\n  - First Name\n  - Second Name\n---\n# Note\nBody.\n".to_vec();
        let document = MarkdownDocument::parse(path(), bytes).expect("sequence document");
        let preview = preview_markdown_update(
            &document,
            request(
                &document,
                MarkdownEdit::SetFrontmatterScalar {
                    key: "aliases".to_owned(),
                    expected_value: Some("[\"First Name\",\"Second Name\"]".to_owned()),
                    value: "Current Name".to_owned(),
                },
            ),
        )
        .expect("sequence replacement");
        assert_eq!(
            preview.proposed_markdown(),
            b"---\nagentmage_id: knowledge-note-001\naliases: \"Current Name\"\n---\n# Note\nBody.\n"
        );
    }
}
