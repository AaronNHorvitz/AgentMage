//! Bounded, effect-free parsing for additional local artifact formats.

use std::collections::BTreeSet;
use std::io::Cursor;

use agentmage_kernel_contracts::CONTRACT_SCHEMA_VERSION;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use zip::{CompressionMethod, ZipArchive};

use crate::word_generation::valid_identifier;
use crate::word_ooxml::word_sha256;

const DEFAULT_MAX_SOURCE_BYTES: u64 = 8 * 1_024 * 1_024;
const DEFAULT_MAX_ITEMS: usize = 100_000;
const DEFAULT_MAX_LINE_BYTES: usize = 64 * 1_024;
const DEFAULT_MAX_ARCHIVE_ENTRIES: usize = 8_192;
const DEFAULT_MAX_ARCHIVE_EXPANDED_BYTES: u64 = 128 * 1_024 * 1_024;

/// Closed additional-parser format identity.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AdditionalFormat {
    /// Saved local HTML.
    Html,
    /// Bounded XML without document types or external entities.
    Xml,
    /// Closed YAML configuration subset.
    Yaml,
    /// Jupyter notebook JSON.
    Notebook,
    /// Timestamped text, JSON Lines, stack traces, or event sequences.
    StructuredLog,
    /// ZIP-compatible archive inventory.
    ZipArchive,
}

/// Exact half-open source byte range.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ParserSourceRange {
    /// Inclusive byte offset.
    pub start: u64,
    /// Exclusive byte offset.
    pub end: u64,
}

/// Bounded parser resource policy.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AdditionalParserProfile {
    /// Stable profile identity.
    pub profile_id: String,
    /// Maximum caller-supplied source bytes.
    pub maximum_source_bytes: u64,
    /// Maximum emitted records, cells, lines, or archive entries.
    pub maximum_items: usize,
    /// Maximum bytes in one textual line or cell source.
    pub maximum_line_bytes: usize,
    /// Maximum archive entry count.
    pub maximum_archive_entries: usize,
    /// Maximum declared archive expansion across all entries.
    pub maximum_archive_expanded_bytes: u64,
}

impl Default for AdditionalParserProfile {
    fn default() -> Self {
        Self {
            profile_id: "additional-parser-strict-v1".to_owned(),
            maximum_source_bytes: DEFAULT_MAX_SOURCE_BYTES,
            maximum_items: DEFAULT_MAX_ITEMS,
            maximum_line_bytes: DEFAULT_MAX_LINE_BYTES,
            maximum_archive_entries: DEFAULT_MAX_ARCHIVE_ENTRIES,
            maximum_archive_expanded_bytes: DEFAULT_MAX_ARCHIVE_EXPANDED_BYTES,
        }
    }
}

/// One extracted saved-markup text span.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MarkupTextSpan {
    /// Exact source byte range.
    pub source_range: ParserSourceRange,
    /// Inert decoded UTF-8 text; entities are not expanded.
    pub text: String,
}

/// Saved HTML or XML parse result.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MarkupParseResult {
    /// Closed format identity.
    pub format: AdditionalFormat,
    /// Exact source digest.
    pub source_sha256: String,
    /// Ordered inert text spans.
    pub text_spans: Vec<MarkupTextSpan>,
    /// Canonically ordered unsupported or quarantined feature codes.
    pub unsupported_features: Vec<String>,
    /// True only after the complete bounded input was scanned.
    pub parse_complete: bool,
    /// False because parsing does not execute markup content.
    pub execution_performed: bool,
}

/// Closed notebook cell kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NotebookCellKind {
    /// Markdown source cell.
    Markdown,
    /// Code source cell retained as inert text.
    Code,
    /// Raw source cell.
    Raw,
}

/// One inert notebook output summary.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NotebookOutput {
    /// Zero-based output order within the cell.
    pub output_index: u32,
    /// Declared output type.
    pub output_type: String,
    /// SHA-256 of the canonical complete output value.
    pub content_sha256: String,
    /// Declared MIME keys in canonical order.
    pub mime_types: Vec<String>,
}

/// One inert notebook cell with exact source range and execution metadata.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NotebookCell {
    /// Zero-based cell order.
    pub cell_index: u32,
    /// Cell kind.
    pub kind: NotebookCellKind,
    /// Exact source range of the cell object in canonical notebook bytes when found.
    pub source_range: ParserSourceRange,
    /// Joined inert cell source.
    pub source: String,
    /// Source digest.
    pub source_sha256: String,
    /// Declared execution count, never interpreted as a command.
    pub execution_count: Option<u64>,
    /// Canonical metadata digest.
    pub metadata_sha256: String,
    /// Ordered output summaries.
    pub outputs: Vec<NotebookOutput>,
}

/// Complete non-executing notebook extraction.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NotebookParseResult {
    /// Exact source digest.
    pub source_sha256: String,
    /// Declared notebook format major version.
    pub nbformat: u64,
    /// Ordered cells.
    pub cells: Vec<NotebookCell>,
    /// Always false; no cell or output is executed or rendered.
    pub execution_performed: bool,
    /// Always false; no external output is resolved.
    pub network_access_performed: bool,
}

/// One YAML scalar with secret-safe value handling.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct YamlScalar {
    /// Exact source range of the complete line.
    pub source_range: ParserSourceRange,
    /// Dot-joined mapping key path.
    pub key_path: String,
    /// Value when non-sensitive; absent for secret fields.
    pub value: Option<String>,
    /// SHA-256 of the exact unredacted scalar.
    pub value_sha256: String,
    /// True when the value was removed from the result.
    pub redacted: bool,
}

/// Closed YAML subset result.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct YamlParseResult {
    /// Exact source digest.
    pub source_sha256: String,
    /// Ordered scalar records.
    pub scalars: Vec<YamlScalar>,
    /// Number of redacted secret values.
    pub redaction_count: u32,
    /// Always false; constructors, tags, aliases, and merge keys are rejected.
    pub constructor_execution_performed: bool,
}

/// Closed structured-log record kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LogRecordKind {
    /// JSON object from one JSON Lines record.
    Json,
    /// Timestamp-prefixed text.
    TimestampedText,
    /// Stack-trace continuation line.
    StackTrace,
    /// Plain bounded text.
    Text,
}

/// One ordered log record.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StructuredLogRecord {
    /// Zero-based record sequence.
    pub sequence: u32,
    /// Exact source line range excluding the newline.
    pub source_range: ParserSourceRange,
    /// Record kind.
    pub kind: LogRecordKind,
    /// Optional leading RFC3339-like timestamp token.
    pub timestamp: Option<String>,
    /// SHA-256 of the complete line.
    pub content_sha256: String,
    /// Canonical JSON value for JSON Lines, otherwise absent.
    pub json: Option<Value>,
}

/// Complete structured-log parse result.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StructuredLogResult {
    /// Exact source digest.
    pub source_sha256: String,
    /// Ordered records.
    pub records: Vec<StructuredLogRecord>,
    /// True only after the complete bounded sequence was parsed.
    pub sequence_complete: bool,
    /// False because log content is never evaluated.
    pub execution_performed: bool,
}

/// One content-minimized archive member.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArchiveEntry {
    /// Zero-based archive order.
    pub entry_index: u32,
    /// Normalized safe relative member name.
    pub name: String,
    /// Compressed byte count.
    pub compressed_size: u64,
    /// Declared uncompressed byte count.
    pub uncompressed_size: u64,
    /// True for a directory entry.
    pub directory: bool,
    /// True when the member name identifies another archive.
    pub nested_archive: bool,
    /// True when policy prevents content extraction.
    pub quarantined: bool,
    /// Stable quarantine reason, if any.
    pub reason_code: Option<String>,
}

/// Metadata-only archive inventory.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArchiveInventory {
    /// Exact source archive digest.
    pub source_sha256: String,
    /// Ordered member inventory.
    pub entries: Vec<ArchiveEntry>,
    /// Declared total expanded bytes.
    pub total_uncompressed_bytes: u64,
    /// Number of quarantined members.
    pub quarantine_count: u32,
    /// Always false: member payloads are not opened by this inventory.
    pub member_content_opened: bool,
    /// Always false: no scratch path is created or written.
    pub filesystem_effect_performed: bool,
    /// Always false: no member is executed.
    pub execution_performed: bool,
}

/// One explicitly disabled deferred format.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeferredFormatDisposition {
    /// Stable format identity.
    pub format_id: String,
    /// Always false until a separate promotion decision and evidence exist.
    pub enabled: bool,
    /// Stable disabled reason.
    pub reason_code: String,
}

/// Common effect-free result envelope for additional local parsers.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AdditionalParserEnvelope<T> {
    /// Kernel contract schema version.
    pub schema_version: u16,
    /// Stable parse operation identity.
    pub operation_id: String,
    /// Closed input format.
    pub format: AdditionalFormat,
    /// Exact source digest.
    pub source_sha256: String,
    /// Format-specific bounded result.
    pub result: T,
    /// False because every parser consumes caller-supplied bytes.
    pub filesystem_effect_performed: bool,
    /// False because no external resources are resolved.
    pub network_access_performed: bool,
    /// False because input content is never executed.
    pub execution_performed: bool,
}

/// Stable additional-parser failure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AdditionalParserError {
    /// An operation identity, profile, or source encoding is invalid.
    InvalidInput,
    /// A byte, item, line, entry, or expansion bound was exceeded.
    ResourceLimit,
    /// A prohibited constructor, entity, path, compression, encryption, or active feature appeared.
    UnsafeContent,
    /// The format is deferred or outside the closed parser set.
    UnsupportedFormat,
}

impl AdditionalParserError {
    /// Stable content-free error code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidInput => "additional_parser.input.invalid",
            Self::ResourceLimit => "additional_parser.resource.limit",
            Self::UnsafeContent => "additional_parser.content.unsafe",
            Self::UnsupportedFormat => "additional_parser.format.unsupported",
        }
    }
}

impl std::fmt::Display for AdditionalParserError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.code())
    }
}

impl std::error::Error for AdditionalParserError {}

fn validate_profile(profile: &AdditionalParserProfile) -> Result<(), AdditionalParserError> {
    if !valid_identifier(&profile.profile_id)
        || profile.maximum_source_bytes == 0
        || profile.maximum_source_bytes > 64 * 1_024 * 1_024
        || profile.maximum_items == 0
        || profile.maximum_items > 1_000_000
        || profile.maximum_line_bytes == 0
        || profile.maximum_line_bytes > 1_024 * 1_024
        || profile.maximum_archive_entries == 0
        || profile.maximum_archive_entries > 100_000
        || profile.maximum_archive_expanded_bytes == 0
        || profile.maximum_archive_expanded_bytes > 1_024 * 1_024 * 1_024
    {
        return Err(AdditionalParserError::InvalidInput);
    }
    Ok(())
}

fn validate_source<'a>(
    source: &'a [u8],
    profile: &AdditionalParserProfile,
) -> Result<&'a str, AdditionalParserError> {
    validate_profile(profile)?;
    if source.is_empty()
        || u64::try_from(source.len()).unwrap_or(u64::MAX) > profile.maximum_source_bytes
    {
        return Err(AdditionalParserError::ResourceLimit);
    }
    std::str::from_utf8(source).map_err(|_| AdditionalParserError::InvalidInput)
}

fn envelope<T>(
    operation_id: &str,
    format: AdditionalFormat,
    source: &[u8],
    result: T,
) -> Result<AdditionalParserEnvelope<T>, AdditionalParserError> {
    if !valid_identifier(operation_id) {
        return Err(AdditionalParserError::InvalidInput);
    }
    Ok(AdditionalParserEnvelope {
        schema_version: CONTRACT_SCHEMA_VERSION,
        operation_id: operation_id.to_owned(),
        format,
        source_sha256: word_sha256(source),
        result,
        filesystem_effect_performed: false,
        network_access_performed: false,
        execution_performed: false,
    })
}

fn parse_markup(
    operation_id: &str,
    source: &[u8],
    profile: &AdditionalParserProfile,
    format: AdditionalFormat,
) -> Result<AdditionalParserEnvelope<MarkupParseResult>, AdditionalParserError> {
    let text = validate_source(source, profile)?;
    let lower = text.to_ascii_lowercase();
    if format == AdditionalFormat::Xml
        && (lower.contains("<!doctype") || lower.contains("<!entity"))
    {
        return Err(AdditionalParserError::UnsafeContent);
    }
    let mut unsupported = BTreeSet::new();
    if format == AdditionalFormat::Html {
        for (needle, code) in [
            ("<script", "html.script.inert"),
            ("<iframe", "html.iframe.inert"),
            ("javascript:", "html.javascript-url.inert"),
            ("http://", "html.external-reference.inert"),
            ("https://", "html.external-reference.inert"),
        ] {
            if lower.contains(needle) {
                unsupported.insert(code.to_owned());
            }
        }
    }
    let mut spans = Vec::new();
    let mut cursor = 0_usize;
    while cursor < text.len() {
        if text.as_bytes()[cursor] == b'<' {
            let relative = text[cursor..]
                .find('>')
                .ok_or(AdditionalParserError::InvalidInput)?;
            cursor = cursor
                .checked_add(relative + 1)
                .ok_or(AdditionalParserError::ResourceLimit)?;
            continue;
        }
        let end = text[cursor..]
            .find('<')
            .map_or(text.len(), |relative| cursor + relative);
        if end > cursor && !text[cursor..end].trim().is_empty() {
            spans.push(MarkupTextSpan {
                source_range: ParserSourceRange {
                    start: u64::try_from(cursor)
                        .map_err(|_| AdditionalParserError::ResourceLimit)?,
                    end: u64::try_from(end).map_err(|_| AdditionalParserError::ResourceLimit)?,
                },
                text: text[cursor..end].to_owned(),
            });
            if spans.len() > profile.maximum_items {
                return Err(AdditionalParserError::ResourceLimit);
            }
        }
        cursor = end;
    }
    let result = MarkupParseResult {
        format,
        source_sha256: word_sha256(source),
        text_spans: spans,
        unsupported_features: unsupported.into_iter().collect(),
        parse_complete: true,
        execution_performed: false,
    };
    envelope(operation_id, format, source, result)
}

/// Parses saved HTML without resolving links, loading frames, or executing script.
pub fn parse_saved_html(
    operation_id: &str,
    source: &[u8],
    profile: &AdditionalParserProfile,
) -> Result<AdditionalParserEnvelope<MarkupParseResult>, AdditionalParserError> {
    parse_markup(operation_id, source, profile, AdditionalFormat::Html)
}

/// Parses bounded XML while rejecting document types and entity declarations.
pub fn parse_bounded_xml(
    operation_id: &str,
    source: &[u8],
    profile: &AdditionalParserProfile,
) -> Result<AdditionalParserEnvelope<MarkupParseResult>, AdditionalParserError> {
    parse_markup(operation_id, source, profile, AdditionalFormat::Xml)
}

fn canonical_json(value: &Value) -> Result<Vec<u8>, AdditionalParserError> {
    serde_json::to_vec(value).map_err(|_| AdditionalParserError::InvalidInput)
}

fn joined_source(value: &Value) -> Result<String, AdditionalParserError> {
    match value {
        Value::String(text) => Ok(text.clone()),
        Value::Array(parts) if parts.iter().all(Value::is_string) => {
            Ok(parts.iter().filter_map(Value::as_str).collect::<String>())
        }
        _ => Err(AdditionalParserError::InvalidInput),
    }
}

/// Extracts notebook cells and output identities without executing cells or rendering outputs.
pub fn parse_notebook(
    operation_id: &str,
    source: &[u8],
    profile: &AdditionalParserProfile,
) -> Result<AdditionalParserEnvelope<NotebookParseResult>, AdditionalParserError> {
    let text = validate_source(source, profile)?;
    let value: Value =
        serde_json::from_str(text).map_err(|_| AdditionalParserError::InvalidInput)?;
    let object = value
        .as_object()
        .ok_or(AdditionalParserError::InvalidInput)?;
    let nbformat = object
        .get("nbformat")
        .and_then(Value::as_u64)
        .ok_or(AdditionalParserError::InvalidInput)?;
    if nbformat != 4 {
        return Err(AdditionalParserError::UnsupportedFormat);
    }
    let values = object
        .get("cells")
        .and_then(Value::as_array)
        .ok_or(AdditionalParserError::InvalidInput)?;
    if values.len() > profile.maximum_items {
        return Err(AdditionalParserError::ResourceLimit);
    }
    let mut search_from = 0_usize;
    let mut cells = Vec::with_capacity(values.len());
    for (index, value) in values.iter().enumerate() {
        let cell = value
            .as_object()
            .ok_or(AdditionalParserError::InvalidInput)?;
        let kind = match cell.get("cell_type").and_then(Value::as_str) {
            Some("markdown") => NotebookCellKind::Markdown,
            Some("code") => NotebookCellKind::Code,
            Some("raw") => NotebookCellKind::Raw,
            _ => return Err(AdditionalParserError::UnsupportedFormat),
        };
        let source_text = joined_source(
            cell.get("source")
                .ok_or(AdditionalParserError::InvalidInput)?,
        )?;
        if source_text.len() > profile.maximum_line_bytes {
            return Err(AdditionalParserError::ResourceLimit);
        }
        let start = text[search_from..]
            .find("\"cell_type\"")
            .map(|relative| search_from + relative)
            .ok_or(AdditionalParserError::InvalidInput)?;
        let next_search = start.saturating_add("\"cell_type\"".len()).min(text.len());
        let end = text[next_search..]
            .find("\"cell_type\"")
            .map_or(text.len(), |relative| next_search + relative);
        search_from = end;
        let metadata = cell
            .get("metadata")
            .cloned()
            .unwrap_or_else(|| Value::Object(Default::default()));
        let execution_count = match cell.get("execution_count") {
            None | Some(Value::Null) => None,
            Some(value) => Some(value.as_u64().ok_or(AdditionalParserError::InvalidInput)?),
        };
        let output_values = cell
            .get("outputs")
            .and_then(Value::as_array)
            .map_or(&[][..], Vec::as_slice);
        if output_values.len() > profile.maximum_items {
            return Err(AdditionalParserError::ResourceLimit);
        }
        let mut outputs = Vec::with_capacity(output_values.len());
        for (output_index, output) in output_values.iter().enumerate() {
            let output_object = output
                .as_object()
                .ok_or(AdditionalParserError::InvalidInput)?;
            let output_type = output_object
                .get("output_type")
                .and_then(Value::as_str)
                .ok_or(AdditionalParserError::InvalidInput)?;
            let mut mime_types = output_object
                .get("data")
                .and_then(Value::as_object)
                .map(|data| data.keys().cloned().collect::<Vec<_>>())
                .unwrap_or_default();
            mime_types.sort();
            outputs.push(NotebookOutput {
                output_index: u32::try_from(output_index)
                    .map_err(|_| AdditionalParserError::ResourceLimit)?,
                output_type: output_type.to_owned(),
                content_sha256: word_sha256(&canonical_json(output)?),
                mime_types,
            });
        }
        cells.push(NotebookCell {
            cell_index: u32::try_from(index).map_err(|_| AdditionalParserError::ResourceLimit)?,
            kind,
            source_range: ParserSourceRange {
                start: u64::try_from(start).unwrap_or(u64::MAX),
                end: u64::try_from(end).unwrap_or(u64::MAX),
            },
            source_sha256: word_sha256(source_text.as_bytes()),
            source: source_text,
            execution_count,
            metadata_sha256: word_sha256(&canonical_json(&metadata)?),
            outputs,
        });
    }
    let result = NotebookParseResult {
        source_sha256: word_sha256(source),
        nbformat,
        cells,
        execution_performed: false,
        network_access_performed: false,
    };
    envelope(operation_id, AdditionalFormat::Notebook, source, result)
}

fn secret_key(value: &str) -> bool {
    let normalized = value.to_ascii_lowercase().replace('-', "_");
    [
        "password",
        "passwd",
        "secret",
        "token",
        "api_key",
        "private_key",
        "credential",
    ]
    .iter()
    .any(|needle| normalized.contains(needle))
}

/// Parses a closed YAML mapping subset and removes values for secret-bearing keys.
pub fn parse_yaml_configuration(
    operation_id: &str,
    source: &[u8],
    profile: &AdditionalParserProfile,
) -> Result<AdditionalParserEnvelope<YamlParseResult>, AdditionalParserError> {
    let text = validate_source(source, profile)?;
    if text.contains("!!")
        || text.contains("!<")
        || text.contains("<<:")
        || text.contains("&")
        || text.contains("*")
    {
        return Err(AdditionalParserError::UnsafeContent);
    }
    let mut scalars = Vec::new();
    let mut stack: Vec<(usize, String)> = Vec::new();
    let mut offset = 0_usize;
    for line in text.split_inclusive('\n') {
        let content = line
            .strip_suffix('\n')
            .unwrap_or(line)
            .strip_suffix('\r')
            .unwrap_or(line.strip_suffix('\n').unwrap_or(line));
        if content.len() > profile.maximum_line_bytes {
            return Err(AdditionalParserError::ResourceLimit);
        }
        let trimmed = content.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            offset += line.len();
            continue;
        }
        let indent = content.len() - content.trim_start_matches(' ').len();
        if indent % 2 != 0 || content[..indent].contains('\t') {
            return Err(AdditionalParserError::InvalidInput);
        }
        let (key, raw) = trimmed
            .split_once(':')
            .ok_or(AdditionalParserError::InvalidInput)?;
        if key.is_empty()
            || !key
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
        {
            return Err(AdditionalParserError::InvalidInput);
        }
        while stack.last().is_some_and(|(level, _)| *level >= indent) {
            stack.pop();
        }
        if raw.trim().is_empty() {
            stack.push((indent, key.to_owned()));
            offset += line.len();
            continue;
        }
        let mut path = stack
            .iter()
            .map(|(_, part)| part.as_str())
            .collect::<Vec<_>>();
        path.push(key);
        let value = raw.trim().trim_matches('"').trim_matches('\'').to_owned();
        let redacted = path.iter().any(|part| secret_key(part));
        scalars.push(YamlScalar {
            source_range: ParserSourceRange {
                start: u64::try_from(offset).unwrap_or(u64::MAX),
                end: u64::try_from(offset + content.len()).unwrap_or(u64::MAX),
            },
            key_path: path.join("."),
            value_sha256: word_sha256(value.as_bytes()),
            value: (!redacted).then_some(value),
            redacted,
        });
        if scalars.len() > profile.maximum_items {
            return Err(AdditionalParserError::ResourceLimit);
        }
        offset += line.len();
    }
    let redaction_count =
        u32::try_from(scalars.iter().filter(|item| item.redacted).count()).unwrap_or(u32::MAX);
    let result = YamlParseResult {
        source_sha256: word_sha256(source),
        scalars,
        redaction_count,
        constructor_execution_performed: false,
    };
    envelope(operation_id, AdditionalFormat::Yaml, source, result)
}

fn timestamp_token(line: &str) -> Option<String> {
    let token = line.split_ascii_whitespace().next()?;
    (token.len() >= 20
        && token.as_bytes().get(4) == Some(&b'-')
        && token.contains('T')
        && (token.ends_with('Z') || token.contains('+')))
    .then(|| token.to_owned())
}

/// Parses timestamped text, JSON Lines, stack traces, and bounded ordered event sequences.
pub fn parse_structured_log(
    operation_id: &str,
    source: &[u8],
    profile: &AdditionalParserProfile,
) -> Result<AdditionalParserEnvelope<StructuredLogResult>, AdditionalParserError> {
    let text = validate_source(source, profile)?;
    let mut records = Vec::new();
    let mut offset = 0_usize;
    for line in text.split_inclusive('\n') {
        let content = line
            .strip_suffix('\n')
            .unwrap_or(line)
            .strip_suffix('\r')
            .unwrap_or(line.strip_suffix('\n').unwrap_or(line));
        if content.len() > profile.maximum_line_bytes {
            return Err(AdditionalParserError::ResourceLimit);
        }
        if content.is_empty() {
            offset += line.len();
            continue;
        }
        let parsed = serde_json::from_str::<Value>(content).ok();
        if parsed.as_ref().is_some_and(|value| !value.is_object()) {
            return Err(AdditionalParserError::InvalidInput);
        }
        let timestamp = parsed
            .as_ref()
            .and_then(|value| value.get("timestamp"))
            .and_then(Value::as_str)
            .map(str::to_owned)
            .or_else(|| timestamp_token(content));
        let kind = if parsed.is_some() {
            LogRecordKind::Json
        } else if timestamp.is_some() {
            LogRecordKind::TimestampedText
        } else if content.starts_with(char::is_whitespace)
            || content.starts_with("at ")
            || content.starts_with("Caused by:")
        {
            LogRecordKind::StackTrace
        } else {
            LogRecordKind::Text
        };
        records.push(StructuredLogRecord {
            sequence: u32::try_from(records.len())
                .map_err(|_| AdditionalParserError::ResourceLimit)?,
            source_range: ParserSourceRange {
                start: u64::try_from(offset).unwrap_or(u64::MAX),
                end: u64::try_from(offset + content.len()).unwrap_or(u64::MAX),
            },
            kind,
            timestamp,
            content_sha256: word_sha256(content.as_bytes()),
            json: parsed,
        });
        if records.len() > profile.maximum_items {
            return Err(AdditionalParserError::ResourceLimit);
        }
        offset += line.len();
    }
    let result = StructuredLogResult {
        source_sha256: word_sha256(source),
        records,
        sequence_complete: true,
        execution_performed: false,
    };
    envelope(
        operation_id,
        AdditionalFormat::StructuredLog,
        source,
        result,
    )
}

fn nested_archive(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    [".zip", ".7z", ".rar", ".tar", ".tgz", ".gz", ".bz2", ".xz"]
        .iter()
        .any(|suffix| lower.ends_with(suffix))
}

/// Inventories an archive without opening member payloads or creating ambient scratch state.
pub fn inventory_zip_archive(
    operation_id: &str,
    source: &[u8],
    profile: &AdditionalParserProfile,
) -> Result<AdditionalParserEnvelope<ArchiveInventory>, AdditionalParserError> {
    validate_profile(profile)?;
    if source.is_empty()
        || u64::try_from(source.len()).unwrap_or(u64::MAX) > profile.maximum_source_bytes
    {
        return Err(AdditionalParserError::ResourceLimit);
    }
    let mut archive =
        ZipArchive::new(Cursor::new(source)).map_err(|_| AdditionalParserError::InvalidInput)?;
    if archive.len() > profile.maximum_archive_entries || archive.len() > profile.maximum_items {
        return Err(AdditionalParserError::ResourceLimit);
    }
    let mut entries = Vec::with_capacity(archive.len());
    let mut total = 0_u64;
    let mut names = BTreeSet::new();
    for index in 0..archive.len() {
        let file = archive
            .by_index(index)
            .map_err(|_| AdditionalParserError::InvalidInput)?;
        if file.encrypted() {
            return Err(AdditionalParserError::UnsafeContent);
        }
        if !matches!(
            file.compression(),
            CompressionMethod::Stored | CompressionMethod::Deflated
        ) {
            return Err(AdditionalParserError::UnsafeContent);
        }
        let name = file
            .enclosed_name()
            .ok_or(AdditionalParserError::UnsafeContent)?
            .to_string_lossy()
            .replace('\\', "/");
        if name.starts_with('/')
            || name.split('/').any(|part| matches!(part, "" | "." | ".."))
            || !names.insert(name.clone())
        {
            return Err(AdditionalParserError::UnsafeContent);
        }
        total = total
            .checked_add(file.size())
            .ok_or(AdditionalParserError::ResourceLimit)?;
        if total > profile.maximum_archive_expanded_bytes {
            return Err(AdditionalParserError::ResourceLimit);
        }
        if file.compressed_size() > 0 && file.size() / file.compressed_size().max(1) > 1_000 {
            return Err(AdditionalParserError::ResourceLimit);
        }
        let nested = nested_archive(&name);
        entries.push(ArchiveEntry {
            entry_index: u32::try_from(index).map_err(|_| AdditionalParserError::ResourceLimit)?,
            name,
            compressed_size: file.compressed_size(),
            uncompressed_size: file.size(),
            directory: file.is_dir(),
            nested_archive: nested,
            quarantined: nested,
            reason_code: nested.then(|| "archive.nested.quarantined".to_owned()),
        });
    }
    let quarantine_count =
        u32::try_from(entries.iter().filter(|item| item.quarantined).count()).unwrap_or(u32::MAX);
    let result = ArchiveInventory {
        source_sha256: word_sha256(source),
        entries,
        total_uncompressed_bytes: total,
        quarantine_count,
        member_content_opened: false,
        filesystem_effect_performed: false,
        execution_performed: false,
    };
    envelope(operation_id, AdditionalFormat::ZipArchive, source, result)
}

/// Returns the closed disabled inventory for proprietary and embedded-execution formats.
#[must_use]
pub fn deferred_format_dispositions() -> Vec<DeferredFormatDisposition> {
    [
        "apple-pages",
        "binary-office",
        "embedded-executable",
        "encrypted-archive",
        "proprietary-notebook",
        "rich-media-project",
    ]
    .into_iter()
    .map(|format_id| DeferredFormatDisposition {
        format_id: format_id.to_owned(),
        enabled: false,
        reason_code: "format.separate-promotion-required".to_owned(),
    })
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use zip::write::SimpleFileOptions;

    #[test]
    fn markup_ranges_are_exact_and_active_features_remain_inert() {
        let profile = AdditionalParserProfile::default();
        let html =
            b"<h1>Hello</h1><script>never()</script><a href='https://example.invalid'>Link</a>";
        let result = parse_saved_html("html-1", html, &profile).expect("html");
        assert_eq!(
            result.result.text_spans[0].source_range,
            ParserSourceRange { start: 4, end: 9 }
        );
        assert!(
            result
                .result
                .unsupported_features
                .contains(&"html.script.inert".to_owned())
        );
        assert!(!result.execution_performed && !result.network_access_performed);
        assert_eq!(
            parse_bounded_xml(
                "xml-1",
                b"<!DOCTYPE x [<!ENTITY y SYSTEM 'file:///x'>]><x>&y;</x>",
                &profile
            ),
            Err(AdditionalParserError::UnsafeContent)
        );
    }

    #[test]
    fn notebook_extracts_markdown_code_outputs_and_metadata_without_execution() {
        let source = br##"{"nbformat":4,"cells":[{"cell_type":"markdown","metadata":{"tag":"a"},"source":["# Title"]},{"cell_type":"code","metadata":{},"source":"print(1)","execution_count":7,"outputs":[{"output_type":"display_data","data":{"text/plain":"1","image/png":"AA=="}}]}]}"##;
        let result = parse_notebook("notebook-1", source, &AdditionalParserProfile::default())
            .expect("notebook");
        assert_eq!(result.result.cells.len(), 2);
        assert!(
            result.result.cells[0].source_range.end <= result.result.cells[1].source_range.start
        );
        assert_eq!(result.result.cells[1].execution_count, Some(7));
        assert_eq!(
            result.result.cells[1].outputs[0].mime_types,
            ["image/png", "text/plain"]
        );
        assert!(!result.result.execution_performed && !result.result.network_access_performed);
    }

    #[test]
    fn yaml_redacts_secrets_and_rejects_executable_constructors() {
        let source = b"service:\n  endpoint: local\n  api_token: sensitive-value\n";
        let result =
            parse_yaml_configuration("yaml-1", source, &AdditionalParserProfile::default())
                .expect("yaml");
        assert_eq!(result.result.redaction_count, 1);
        assert_eq!(result.result.scalars[1].value, None);
        assert_ne!(
            result.result.scalars[1].value_sha256,
            word_sha256(b"[REDACTED]")
        );
        assert_eq!(
            parse_yaml_configuration(
                "yaml-2",
                b"value: !!python/object:x {}",
                &AdditionalParserProfile::default()
            ),
            Err(AdditionalParserError::UnsafeContent)
        );
    }

    #[test]
    fn logs_preserve_sequence_ranges_json_timestamps_and_stack_lines() {
        let source = b"{\"timestamp\":\"2026-09-03T00:00:00Z\",\"event\":\"start\"}\n2026-09-03T00:00:01Z done\n  at module:1\n";
        let result = parse_structured_log("log-1", source, &AdditionalParserProfile::default())
            .expect("log");
        assert_eq!(result.result.records.len(), 3);
        assert_eq!(result.result.records[0].kind, LogRecordKind::Json);
        assert_eq!(result.result.records[2].kind, LogRecordKind::StackTrace);
        assert_eq!(result.result.records[2].sequence, 2);
        assert!(result.result.sequence_complete && !result.result.execution_performed);
    }

    #[test]
    fn archive_inventory_never_opens_members_and_quarantines_nested_content() {
        let mut bytes = Vec::new();
        {
            let mut writer = zip::ZipWriter::new(Cursor::new(&mut bytes));
            writer
                .start_file("notes/readme.txt", SimpleFileOptions::default())
                .expect("file");
            writer.write_all(b"hello").expect("write");
            writer
                .start_file("nested.zip", SimpleFileOptions::default())
                .expect("nested");
            writer.write_all(b"PK").expect("write");
            writer.finish().expect("finish");
        }
        let result =
            inventory_zip_archive("archive-1", &bytes, &AdditionalParserProfile::default())
                .expect("inventory");
        assert_eq!(result.result.entries.len(), 2);
        assert!(result.result.entries[1].quarantined);
        assert!(
            !result.result.member_content_opened
                && !result.result.filesystem_effect_performed
                && !result.result.execution_performed
        );
    }

    #[test]
    fn bounds_paths_and_deferred_formats_fail_closed() {
        let profile = AdditionalParserProfile {
            maximum_source_bytes: 4,
            ..AdditionalParserProfile::default()
        };
        assert_eq!(
            parse_saved_html("html-1", b"12345", &profile),
            Err(AdditionalParserError::ResourceLimit)
        );
        let mut hostile = Vec::new();
        {
            let mut writer = zip::ZipWriter::new(Cursor::new(&mut hostile));
            writer
                .start_file("../escape", SimpleFileOptions::default())
                .expect("hostile name");
            writer.write_all(b"content").expect("write");
            writer.finish().expect("finish");
        }
        assert_eq!(
            inventory_zip_archive("archive-2", &hostile, &AdditionalParserProfile::default()),
            Err(AdditionalParserError::UnsafeContent)
        );
        assert!(
            deferred_format_dispositions()
                .iter()
                .all(|item| !item.enabled)
        );
        assert_eq!(deferred_format_dispositions().len(), 6);
    }
}
