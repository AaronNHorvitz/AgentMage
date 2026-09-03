//! Bounded direct Open XML spreadsheet inspection without formula evaluation.

use std::collections::{BTreeMap, BTreeSet};
use std::io::{Cursor, Read};
use std::time::{Duration, Instant};

use agentmage_kernel_contracts::{CONTRACT_SCHEMA_VERSION, WorkspacePath};
use quick_xml::escape::unescape;
use quick_xml::events::{BytesRef, BytesStart, Event};
use quick_xml::{Reader, XmlVersion};
use serde::{Deserialize, Serialize};
use zip::{CompressionMethod, ZipArchive};

use crate::word_ooxml::word_sha256;

const MAX_SOURCE_BYTES: u64 = 256 * 1_024 * 1_024;
const MAX_ENTRIES: usize = 65_536;
const MAX_ENTRY_BYTES: u64 = 128 * 1_024 * 1_024;
const MAX_TOTAL_BYTES: u64 = 512 * 1_024 * 1_024;
const MAX_SHEETS: usize = 4_096;
const MAX_CELLS: usize = 5_000_000;
const MAX_SHARED_STRINGS: usize = 5_000_000;
const MAX_ROWS: u32 = 1_048_576;
const MAX_COLUMNS: u32 = 16_384;
const MAX_RELATIONSHIPS: usize = 1_000_000;
const MAX_STRING_BYTES: usize = 16 * 1_024 * 1_024;
const MAX_WORKING_MEMORY_BYTES: u64 = 1_024 * 1_024 * 1_024;
const MAX_ELAPSED_MILLISECONDS: u64 = 3_600_000;

/// Closed resource profile for direct Open XML spreadsheet inspection.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpreadsheetProfile {
    /// Stable profile identity.
    pub profile_id: String,
    /// Maximum compressed package bytes.
    pub maximum_source_bytes: u64,
    /// Maximum ZIP entries.
    pub maximum_entries: usize,
    /// Maximum uncompressed bytes in one part.
    pub maximum_entry_bytes: u64,
    /// Maximum total uncompressed package bytes.
    pub maximum_total_uncompressed_bytes: u64,
    /// Maximum worksheets.
    pub maximum_sheets: usize,
    /// Maximum retained cells across worksheets.
    pub maximum_cells: usize,
    /// Maximum shared strings.
    pub maximum_shared_strings: usize,
    /// Maximum one-based row coordinate.
    pub maximum_rows: u32,
    /// Maximum one-based column coordinate.
    pub maximum_columns: u32,
    /// Maximum relationships retained from any relationship part.
    pub maximum_relationships: usize,
    /// Maximum decoded bytes in any sheet name, target, formula, or cell string.
    pub maximum_string_bytes: usize,
    /// Maximum accounted source plus retained decompressed bytes.
    pub maximum_working_memory_bytes: u64,
    /// Maximum elapsed parser time; checkpoints fail closed when exhausted.
    pub maximum_elapsed_milliseconds: u64,
}

impl Default for SpreadsheetProfile {
    fn default() -> Self {
        Self {
            profile_id: "spreadsheet-ooxml-strict-v1".to_owned(),
            maximum_source_bytes: 64 * 1_024 * 1_024,
            maximum_entries: 8_192,
            maximum_entry_bytes: 32 * 1_024 * 1_024,
            maximum_total_uncompressed_bytes: 128 * 1_024 * 1_024,
            maximum_sheets: 1_024,
            maximum_cells: 1_000_000,
            maximum_shared_strings: 1_000_000,
            maximum_rows: MAX_ROWS,
            maximum_columns: MAX_COLUMNS,
            maximum_relationships: 100_000,
            maximum_string_bytes: 4 * 1_024 * 1_024,
            maximum_working_memory_bytes: 256 * 1_024 * 1_024,
            maximum_elapsed_milliseconds: 60_000,
        }
    }
}

impl SpreadsheetProfile {
    fn valid(&self) -> bool {
        valid_identifier(&self.profile_id)
            && self.maximum_source_bytes > 0
            && self.maximum_source_bytes <= MAX_SOURCE_BYTES
            && self.maximum_entries > 0
            && self.maximum_entries <= MAX_ENTRIES
            && self.maximum_entry_bytes > 0
            && self.maximum_entry_bytes <= MAX_ENTRY_BYTES
            && self.maximum_total_uncompressed_bytes >= self.maximum_entry_bytes
            && self.maximum_total_uncompressed_bytes <= MAX_TOTAL_BYTES
            && self.maximum_sheets > 0
            && self.maximum_sheets <= MAX_SHEETS
            && self.maximum_cells > 0
            && self.maximum_cells <= MAX_CELLS
            && self.maximum_shared_strings > 0
            && self.maximum_shared_strings <= MAX_SHARED_STRINGS
            && self.maximum_rows > 0
            && self.maximum_rows <= MAX_ROWS
            && self.maximum_columns > 0
            && self.maximum_columns <= MAX_COLUMNS
            && self.maximum_relationships > 0
            && self.maximum_relationships <= MAX_RELATIONSHIPS
            && self.maximum_string_bytes > 0
            && self.maximum_string_bytes <= MAX_STRING_BYTES
            && self.maximum_working_memory_bytes >= self.maximum_source_bytes
            && self.maximum_working_memory_bytes <= MAX_WORKING_MEMORY_BYTES
            && self.maximum_elapsed_milliseconds > 0
            && self.maximum_elapsed_milliseconds <= MAX_ELAPSED_MILLISECONDS
    }
}

/// Workbook date serial system.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpreadsheetDateSystem {
    /// Windows 1900 date system, including Excel's serial-60 leap-day convention.
    Excel1900,
    /// Legacy Apple 1904 date system.
    Excel1904,
}

/// Worksheet visibility state.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpreadsheetSheetState {
    /// Visible worksheet.
    Visible,
    /// Hidden worksheet.
    Hidden,
    /// Very-hidden worksheet.
    VeryHidden,
}

/// Closed retained cell value class.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpreadsheetCellKind {
    /// Empty cell.
    Blank,
    /// Shared-string table value.
    SharedString,
    /// Inline string.
    InlineString,
    /// Formula cached string.
    String,
    /// Number retained as exact text.
    Number,
    /// Boolean value.
    Boolean,
    /// Spreadsheet error token.
    Error,
    /// ISO date value.
    Date,
    /// Formula with an inert cached result.
    Formula,
}

/// One inert hyperlink observation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpreadsheetHyperlink {
    /// Cell or range reference.
    pub reference: String,
    /// Inert relationship target or internal location.
    pub target: String,
    /// SHA-256 of the exact target.
    pub target_sha256: String,
    /// True when the relationship declares an external target.
    pub external: bool,
    /// Always false; inspection never follows a link.
    pub followed: bool,
}

/// One exact cell record.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpreadsheetCell {
    /// A1-style address.
    pub address: String,
    /// One-based row.
    pub row: u32,
    /// One-based column.
    pub column: u32,
    /// Closed value class.
    pub kind: SpreadsheetCellKind,
    /// Exact raw cached value when present.
    pub raw_value: Option<String>,
    /// Decoded displayed or cached value when available.
    pub displayed_value: Option<String>,
    /// Inert formula text when present.
    pub formula: Option<String>,
    /// Cell format/style index.
    pub style_index: Option<u32>,
    /// Number format identity resolved from the style when available.
    pub number_format_id: Option<u32>,
    /// Derived ISO date for an admitted date style and valid serial.
    pub date_value: Option<String>,
    /// Inert hyperlink when attached to this exact reference.
    pub hyperlink: Option<SpreadsheetHyperlink>,
}

/// One closed worksheet inspection.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpreadsheetWorksheet {
    /// Stable sheet identity from workbook order.
    pub sheet_id: u32,
    /// Visible worksheet name.
    pub name: String,
    /// Visibility state.
    pub state: SpreadsheetSheetState,
    /// Exact package part name.
    pub part_name: String,
    /// Digest of exact worksheet XML bytes.
    pub part_sha256: String,
    /// Declared used-range dimension when present.
    pub dimension: Option<String>,
    /// True when worksheet protection is declared; no password or bypass is attempted.
    pub protected: bool,
    /// True when the declared range is materially larger than retained populated cells.
    pub sparse_dimension: bool,
    /// Canonically ordered retained cells.
    pub cells: Vec<SpreadsheetCell>,
    /// Hidden one-based rows.
    pub hidden_rows: Vec<u32>,
    /// Hidden one-based columns expanded from declared ranges.
    pub hidden_columns: Vec<u32>,
    /// Declared merged-cell ranges.
    pub merged_ranges: Vec<String>,
    /// Inert hyperlink records, including ranges not mapped to one cell.
    pub hyperlinks: Vec<SpreadsheetHyperlink>,
}

/// Closed workbook inspection finding.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpreadsheetFindingKind {
    /// Macro-enabled workbook content type or VBA part.
    MacroContent,
    /// External workbook relationship or formula reference.
    ExternalWorkbookReference,
    /// Dynamic Data Exchange-like formula.
    DdeFormula,
    /// External hyperlink retained inertly.
    ExternalHyperlink,
    /// Unsupported package compression.
    UnsupportedCompression,
    /// Embedded OLE, package, control, or other active object retained but never opened.
    EmbeddedObject,
    /// Worksheet protection preserved without attempting to bypass it.
    ProtectedSheet,
    /// Declared used range is sparse and is not expanded into empty cells.
    SparseRange,
    /// A non-core workbook structure remains authoritative in the original package.
    UnsupportedFeature,
}

/// One content-minimized workbook finding.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpreadsheetFinding {
    /// Stable finding identity.
    pub finding_id: String,
    /// Closed finding class.
    pub kind: SpreadsheetFindingKind,
    /// Package part when available.
    pub part_name: Option<String>,
    /// Sheet name when available.
    pub sheet_name: Option<String>,
    /// Cell reference when available.
    pub cell_reference: Option<String>,
    /// Stable content-free reason code.
    pub reason_code: String,
    /// True when the finding prevents safe analytical reuse.
    pub blocks_safe_analysis: bool,
}

/// Complete direct Open XML workbook inspection.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpreadsheetInspection {
    /// Kernel contract schema version.
    pub schema_version: u16,
    /// Canonical caller-authorized source path.
    pub source_path: WorkspacePath,
    /// Exact source digest.
    pub source_sha256: String,
    /// Exact parser profile.
    pub profile: SpreadsheetProfile,
    /// Date serial system.
    pub date_system: SpreadsheetDateSystem,
    /// Canonically ordered worksheets.
    pub worksheets: Vec<SpreadsheetWorksheet>,
    /// Canonically ordered findings.
    pub findings: Vec<SpreadsheetFinding>,
    /// True only when all admitted structures were inspected within limits.
    pub inspection_complete: bool,
    /// True only when no blocking finding exists.
    pub safe_for_analysis: bool,
    /// True because input bytes remain authoritative.
    pub original_preserved: bool,
    /// False because parsing occurs in memory.
    pub filesystem_effect_performed: bool,
    /// False because links are never resolved.
    pub network_access_performed: bool,
    /// False because formulas, macros, DDE, and embedded content are never executed.
    pub execution_performed: bool,
}

/// Stable direct spreadsheet inspection failure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpreadsheetError {
    /// Profile or source is invalid.
    InvalidInput,
    /// ZIP structure, required part, relationship, cell reference, or XML is malformed.
    MalformedPackage,
    /// Package path is unsafe or duplicated.
    UnsafePackage,
    /// Encrypted ZIP entry is unsupported.
    EncryptedPackage,
    /// A source, entry, sheet, string, cell, or expansion limit was exceeded.
    ResourceLimit,
    /// Caller cancellation was observed at a parser checkpoint.
    Cancelled,
    /// The explicit elapsed-time ceiling was exhausted.
    TimeLimit,
}

impl SpreadsheetError {
    /// Stable content-free error code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidInput => "spreadsheet.input.invalid",
            Self::MalformedPackage => "spreadsheet.package.malformed",
            Self::UnsafePackage => "spreadsheet.package.unsafe",
            Self::EncryptedPackage => "spreadsheet.package.encrypted",
            Self::ResourceLimit => "spreadsheet.resource.limit",
            Self::Cancelled => "spreadsheet.cancelled",
            Self::TimeLimit => "spreadsheet.time.limit",
        }
    }
}

impl std::fmt::Display for SpreadsheetError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.code())
    }
}

impl std::error::Error for SpreadsheetError {}

#[derive(Clone)]
struct Relationship {
    target: String,
    external: bool,
    relation_type: String,
}

#[derive(Clone)]
struct SheetDefinition {
    sheet_id: u32,
    name: String,
    state: SpreadsheetSheetState,
    relationship_id: String,
}

#[derive(Default)]
struct StyleTable {
    cell_number_formats: Vec<u32>,
    custom_formats: BTreeMap<u32, String>,
}

fn valid_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value.is_ascii()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b':' | b'-'))
}

struct InspectionControl<'a> {
    deadline: Instant,
    cancelled: &'a mut dyn FnMut() -> bool,
    accounted_memory_bytes: u64,
    maximum_working_memory_bytes: u64,
}

impl<'a> InspectionControl<'a> {
    fn new(
        profile: &SpreadsheetProfile,
        source_bytes: usize,
        cancelled: &'a mut dyn FnMut() -> bool,
    ) -> Result<Self, SpreadsheetError> {
        let accounted_memory_bytes =
            u64::try_from(source_bytes).map_err(|_| SpreadsheetError::ResourceLimit)?;
        if accounted_memory_bytes > profile.maximum_working_memory_bytes {
            return Err(SpreadsheetError::ResourceLimit);
        }
        Ok(Self {
            deadline: Instant::now() + Duration::from_millis(profile.maximum_elapsed_milliseconds),
            cancelled,
            accounted_memory_bytes,
            maximum_working_memory_bytes: profile.maximum_working_memory_bytes,
        })
    }

    fn checkpoint(&mut self) -> Result<(), SpreadsheetError> {
        if (self.cancelled)() {
            return Err(SpreadsheetError::Cancelled);
        }
        if Instant::now() >= self.deadline {
            return Err(SpreadsheetError::TimeLimit);
        }
        Ok(())
    }

    fn account(&mut self, bytes: u64) -> Result<(), SpreadsheetError> {
        self.accounted_memory_bytes = self
            .accounted_memory_bytes
            .checked_add(bytes)
            .ok_or(SpreadsheetError::ResourceLimit)?;
        if self.accounted_memory_bytes > self.maximum_working_memory_bytes {
            return Err(SpreadsheetError::ResourceLimit);
        }
        Ok(())
    }
}

fn local_name(name: &[u8]) -> &[u8] {
    name.rsplit(|byte| *byte == b':').next().unwrap_or(name)
}

fn attribute(
    event: &BytesStart<'_>,
    expected: &[u8],
    reader: &Reader<&[u8]>,
) -> Result<Option<String>, SpreadsheetError> {
    for item in event.attributes().with_checks(true) {
        let item = item.map_err(|_| SpreadsheetError::MalformedPackage)?;
        if local_name(item.key.as_ref()) == expected {
            return item
                .decoded_and_normalized_value(XmlVersion::Implicit1_0, reader.decoder())
                .map(|value| Some(value.into_owned()))
                .map_err(|_| SpreadsheetError::MalformedPackage);
        }
    }
    Ok(None)
}

fn decoded_text(event: quick_xml::events::BytesText<'_>) -> Result<String, SpreadsheetError> {
    let encoded = event
        .decode()
        .map_err(|_| SpreadsheetError::MalformedPackage)?;
    unescape(&encoded)
        .map(|value| value.into_owned())
        .map_err(|_| SpreadsheetError::MalformedPackage)
}

fn decoded_reference(event: BytesRef<'_>) -> Result<String, SpreadsheetError> {
    let reference = event
        .decode()
        .map_err(|_| SpreadsheetError::MalformedPackage)?;
    let character = match reference.as_ref() {
        "amp" => '&',
        "lt" => '<',
        "gt" => '>',
        "quot" => '"',
        "apos" => '\'',
        value if value.starts_with("#x") => u32::from_str_radix(&value[2..], 16)
            .ok()
            .and_then(char::from_u32)
            .ok_or(SpreadsheetError::MalformedPackage)?,
        value if value.starts_with('#') => value[1..]
            .parse::<u32>()
            .ok()
            .and_then(char::from_u32)
            .ok_or(SpreadsheetError::MalformedPackage)?,
        _ => return Err(SpreadsheetError::MalformedPackage),
    };
    Ok(character.to_string())
}

fn package_parts(
    source: &[u8],
    profile: &SpreadsheetProfile,
    control: &mut InspectionControl<'_>,
) -> Result<BTreeMap<String, Vec<u8>>, SpreadsheetError> {
    if source.is_empty()
        || u64::try_from(source.len()).unwrap_or(u64::MAX) > profile.maximum_source_bytes
    {
        return Err(SpreadsheetError::ResourceLimit);
    }
    let mut archive =
        ZipArchive::new(Cursor::new(source)).map_err(|_| SpreadsheetError::MalformedPackage)?;
    if archive.is_empty() || archive.len() > profile.maximum_entries {
        return Err(SpreadsheetError::ResourceLimit);
    }
    let mut total = 0_u64;
    let mut parts = BTreeMap::new();
    for index in 0..archive.len() {
        control.checkpoint()?;
        let mut file = archive
            .by_index(index)
            .map_err(|_| SpreadsheetError::MalformedPackage)?;
        if file.is_dir() {
            continue;
        }
        if file.encrypted() {
            return Err(SpreadsheetError::EncryptedPackage);
        }
        if !matches!(
            file.compression(),
            CompressionMethod::Stored | CompressionMethod::Deflated
        ) {
            return Err(SpreadsheetError::MalformedPackage);
        }
        let name = file
            .enclosed_name()
            .ok_or(SpreadsheetError::UnsafePackage)?
            .to_string_lossy()
            .replace('\\', "/");
        if name.starts_with('/')
            || name
                .split('/')
                .any(|component| matches!(component, "" | "." | ".."))
        {
            return Err(SpreadsheetError::UnsafePackage);
        }
        if parts.contains_key(&name) {
            return Err(SpreadsheetError::UnsafePackage);
        }
        let expected_size = file.size();
        if expected_size > profile.maximum_entry_bytes {
            return Err(SpreadsheetError::ResourceLimit);
        }
        total = total
            .checked_add(expected_size)
            .ok_or(SpreadsheetError::ResourceLimit)?;
        if total > profile.maximum_total_uncompressed_bytes {
            return Err(SpreadsheetError::ResourceLimit);
        }
        control.account(expected_size)?;
        let mut content = Vec::new();
        (&mut file)
            .take(profile.maximum_entry_bytes + 1)
            .read_to_end(&mut content)
            .map_err(|_| SpreadsheetError::MalformedPackage)?;
        if u64::try_from(content.len()).unwrap_or(u64::MAX) != expected_size {
            return Err(SpreadsheetError::ResourceLimit);
        }
        parts.insert(name, content);
    }
    Ok(parts)
}

fn parse_relationships(
    content: &[u8],
    profile: &SpreadsheetProfile,
    control: &mut InspectionControl<'_>,
) -> Result<BTreeMap<String, Relationship>, SpreadsheetError> {
    let mut reader = Reader::from_reader(content);
    reader.config_mut().trim_text(true);
    let mut relationships = BTreeMap::new();
    loop {
        control.checkpoint()?;
        match reader
            .read_event()
            .map_err(|_| SpreadsheetError::MalformedPackage)?
        {
            Event::Start(event) | Event::Empty(event)
                if local_name(event.name().as_ref()) == b"Relationship" =>
            {
                let id =
                    attribute(&event, b"Id", &reader)?.ok_or(SpreadsheetError::MalformedPackage)?;
                let target = attribute(&event, b"Target", &reader)?
                    .ok_or(SpreadsheetError::MalformedPackage)?;
                let relation_type = attribute(&event, b"Type", &reader)?.unwrap_or_default();
                if id.len() > profile.maximum_string_bytes
                    || target.len() > profile.maximum_string_bytes
                    || relation_type.len() > profile.maximum_string_bytes
                    || relationships.len() >= profile.maximum_relationships
                {
                    return Err(SpreadsheetError::ResourceLimit);
                }
                let external = attribute(&event, b"TargetMode", &reader)?
                    .is_some_and(|value| value.eq_ignore_ascii_case("external"));
                if relationships
                    .insert(
                        id,
                        Relationship {
                            target,
                            external,
                            relation_type,
                        },
                    )
                    .is_some()
                {
                    return Err(SpreadsheetError::MalformedPackage);
                }
            }
            Event::Eof => return Ok(relationships),
            _ => {}
        }
    }
}

fn parse_workbook(
    content: &[u8],
    profile: &SpreadsheetProfile,
    control: &mut InspectionControl<'_>,
) -> Result<(SpreadsheetDateSystem, Vec<SheetDefinition>), SpreadsheetError> {
    let mut reader = Reader::from_reader(content);
    reader.config_mut().trim_text(true);
    let mut date_system = SpreadsheetDateSystem::Excel1900;
    let mut sheets = Vec::new();
    loop {
        control.checkpoint()?;
        match reader
            .read_event()
            .map_err(|_| SpreadsheetError::MalformedPackage)?
        {
            Event::Start(event) | Event::Empty(event)
                if local_name(event.name().as_ref()) == b"workbookPr"
                    && attribute(&event, b"date1904", &reader)?
                        .is_some_and(|value| matches!(value.as_str(), "1" | "true")) =>
            {
                date_system = SpreadsheetDateSystem::Excel1904;
            }
            Event::Start(event) | Event::Empty(event)
                if local_name(event.name().as_ref()) == b"sheet" =>
            {
                let name = attribute(&event, b"name", &reader)?
                    .ok_or(SpreadsheetError::MalformedPackage)?;
                let sheet_id = attribute(&event, b"sheetId", &reader)?
                    .ok_or(SpreadsheetError::MalformedPackage)?
                    .parse()
                    .map_err(|_| SpreadsheetError::MalformedPackage)?;
                let relationship_id =
                    attribute(&event, b"id", &reader)?.ok_or(SpreadsheetError::MalformedPackage)?;
                if name.len() > profile.maximum_string_bytes
                    || relationship_id.len() > profile.maximum_string_bytes
                    || sheets.len() >= profile.maximum_sheets
                {
                    return Err(SpreadsheetError::ResourceLimit);
                }
                let state = match attribute(&event, b"state", &reader)?.as_deref() {
                    None | Some("visible") => SpreadsheetSheetState::Visible,
                    Some("hidden") => SpreadsheetSheetState::Hidden,
                    Some("veryHidden") => SpreadsheetSheetState::VeryHidden,
                    _ => return Err(SpreadsheetError::MalformedPackage),
                };
                sheets.push(SheetDefinition {
                    sheet_id,
                    name,
                    state,
                    relationship_id,
                });
            }
            Event::Eof => return Ok((date_system, sheets)),
            _ => {}
        }
    }
}

fn parse_shared_strings(
    content: Option<&Vec<u8>>,
    profile: &SpreadsheetProfile,
    control: &mut InspectionControl<'_>,
) -> Result<Vec<String>, SpreadsheetError> {
    let Some(content) = content else {
        return Ok(Vec::new());
    };
    let mut reader = Reader::from_reader(content.as_slice());
    reader.config_mut().trim_text(false);
    let mut values = Vec::new();
    let mut in_item = false;
    let mut in_text = false;
    let mut value = String::new();
    loop {
        control.checkpoint()?;
        match reader
            .read_event()
            .map_err(|_| SpreadsheetError::MalformedPackage)?
        {
            Event::Start(event) if local_name(event.name().as_ref()) == b"si" => {
                in_item = true;
                value.clear();
            }
            Event::Start(event) if in_item && local_name(event.name().as_ref()) == b"t" => {
                in_text = true
            }
            Event::Text(event) if in_item && in_text => value.push_str(&decoded_text(event)?),
            Event::GeneralRef(event) if in_item && in_text => {
                value.push_str(&decoded_reference(event)?)
            }
            Event::End(event) if local_name(event.name().as_ref()) == b"t" => in_text = false,
            Event::End(event) if local_name(event.name().as_ref()) == b"si" => {
                if value.len() > profile.maximum_string_bytes {
                    return Err(SpreadsheetError::ResourceLimit);
                }
                values.push(value.clone());
                if values.len() > profile.maximum_shared_strings {
                    return Err(SpreadsheetError::ResourceLimit);
                }
                in_item = false;
            }
            Event::Eof => return Ok(values),
            _ => {}
        }
    }
}

fn parse_styles(
    content: Option<&Vec<u8>>,
    profile: &SpreadsheetProfile,
    control: &mut InspectionControl<'_>,
) -> Result<StyleTable, SpreadsheetError> {
    let Some(content) = content else {
        return Ok(StyleTable::default());
    };
    let mut reader = Reader::from_reader(content.as_slice());
    reader.config_mut().trim_text(true);
    let mut table = StyleTable::default();
    let mut in_cell_xfs = false;
    loop {
        control.checkpoint()?;
        match reader
            .read_event()
            .map_err(|_| SpreadsheetError::MalformedPackage)?
        {
            Event::Start(event) if local_name(event.name().as_ref()) == b"cellXfs" => {
                in_cell_xfs = true
            }
            Event::End(event) if local_name(event.name().as_ref()) == b"cellXfs" => {
                in_cell_xfs = false
            }
            Event::Start(event) | Event::Empty(event)
                if local_name(event.name().as_ref()) == b"numFmt" =>
            {
                let id = attribute(&event, b"numFmtId", &reader)?
                    .ok_or(SpreadsheetError::MalformedPackage)?
                    .parse()
                    .map_err(|_| SpreadsheetError::MalformedPackage)?;
                let code = attribute(&event, b"formatCode", &reader)?
                    .ok_or(SpreadsheetError::MalformedPackage)?;
                if code.len() > profile.maximum_string_bytes {
                    return Err(SpreadsheetError::ResourceLimit);
                }
                table.custom_formats.insert(id, code);
            }
            Event::Start(event) | Event::Empty(event)
                if in_cell_xfs && local_name(event.name().as_ref()) == b"xf" =>
            {
                let id = attribute(&event, b"numFmtId", &reader)?
                    .unwrap_or_else(|| "0".to_owned())
                    .parse()
                    .map_err(|_| SpreadsheetError::MalformedPackage)?;
                table.cell_number_formats.push(id);
            }
            Event::Eof => return Ok(table),
            _ => {}
        }
    }
}

fn date_format(id: u32, styles: &StyleTable) -> bool {
    if matches!(id, 14..=22 | 27..=36 | 45..=47 | 50..=58) {
        return true;
    }
    styles.custom_formats.get(&id).is_some_and(|format| {
        let mut cleaned = String::new();
        let mut quoted = false;
        for character in format.chars() {
            if character == '"' {
                quoted = !quoted;
            } else if !quoted {
                cleaned.push(character.to_ascii_lowercase());
            }
        }
        cleaned.contains('y') && (cleaned.contains('m') || cleaned.contains('d'))
    })
}

fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let day_of_era = z - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let mut year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_prime = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_prime + 2) / 5 + 1;
    let month = month_prime + if month_prime < 10 { 3 } else { -9 };
    year += i64::from(month <= 2);
    (year, month, day)
}

/// Converts an exact integral Excel date serial to an ISO date without evaluating formulas.
#[must_use]
pub fn excel_serial_date(value: &str, system: SpreadsheetDateSystem) -> Option<String> {
    let serial: i64 = value.parse().ok()?;
    if serial < 0 {
        return None;
    }
    if system == SpreadsheetDateSystem::Excel1900 && serial == 60 {
        return Some("1900-02-29".to_owned());
    }
    let unix_days = match system {
        SpreadsheetDateSystem::Excel1900 => -25_568 + serial - i64::from(serial > 60),
        SpreadsheetDateSystem::Excel1904 => -24_107 + serial,
    };
    let (year, month, day) = civil_from_days(unix_days);
    Some(format!("{year:04}-{month:02}-{day:02}"))
}

fn cell_coordinates(address: &str) -> Option<(u32, u32)> {
    let bytes = address.as_bytes();
    let split = bytes.iter().position(u8::is_ascii_digit)?;
    if split == 0 || split == bytes.len() || !bytes[..split].iter().all(u8::is_ascii_alphabetic) {
        return None;
    }
    let mut column = 0_u32;
    for byte in &bytes[..split] {
        column = column
            .checked_mul(26)?
            .checked_add(u32::from(byte.to_ascii_uppercase() - b'A' + 1))?;
    }
    let row = address[split..].parse().ok()?;
    (row > 0 && column > 0).then_some((row, column))
}

fn dimension_area(dimension: &str) -> Option<u64> {
    let (start, end) = dimension
        .split_once(':')
        .map_or((dimension, dimension), |(start, end)| (start, end));
    let (start_row, start_column) = cell_coordinates(start)?;
    let (end_row, end_column) = cell_coordinates(end)?;
    if end_row < start_row || end_column < start_column {
        return None;
    }
    u64::from(end_row - start_row + 1).checked_mul(u64::from(end_column - start_column + 1))
}

fn worksheet_relationship_path(part_name: &str) -> Option<String> {
    let (parent, file) = part_name.rsplit_once('/')?;
    Some(format!("{parent}/_rels/{file}.rels"))
}

fn workbook_target(target: &str) -> Result<String, SpreadsheetError> {
    let normalized = if let Some(value) = target.strip_prefix('/') {
        value.to_owned()
    } else {
        format!("xl/{target}")
    };
    if normalized
        .split('/')
        .any(|item| matches!(item, "" | "." | ".."))
    {
        return Err(SpreadsheetError::UnsafePackage);
    }
    Ok(normalized)
}

fn hyperlink_records(
    worksheet: &[u8],
    relationships: &BTreeMap<String, Relationship>,
    profile: &SpreadsheetProfile,
    control: &mut InspectionControl<'_>,
) -> Result<Vec<SpreadsheetHyperlink>, SpreadsheetError> {
    let mut reader = Reader::from_reader(worksheet);
    reader.config_mut().trim_text(true);
    let mut links = Vec::new();
    loop {
        control.checkpoint()?;
        match reader
            .read_event()
            .map_err(|_| SpreadsheetError::MalformedPackage)?
        {
            Event::Start(event) | Event::Empty(event)
                if local_name(event.name().as_ref()) == b"hyperlink" =>
            {
                let reference = attribute(&event, b"ref", &reader)?
                    .ok_or(SpreadsheetError::MalformedPackage)?;
                let relation = attribute(&event, b"id", &reader)?;
                let location = attribute(&event, b"location", &reader)?;
                let (target, external) = if let Some(id) = relation {
                    let item = relationships
                        .get(&id)
                        .ok_or(SpreadsheetError::MalformedPackage)?;
                    (item.target.clone(), item.external)
                } else {
                    (location.ok_or(SpreadsheetError::MalformedPackage)?, false)
                };
                if reference.len() > profile.maximum_string_bytes
                    || target.len() > profile.maximum_string_bytes
                    || links.len() >= profile.maximum_relationships
                {
                    return Err(SpreadsheetError::ResourceLimit);
                }
                links.push(SpreadsheetHyperlink {
                    reference,
                    target_sha256: word_sha256(target.as_bytes()),
                    target,
                    external,
                    followed: false,
                });
            }
            Event::Eof => {
                links.sort_by(|left, right| left.reference.cmp(&right.reference));
                return Ok(links);
            }
            _ => {}
        }
    }
}

struct WorksheetContext<'a> {
    relationships: &'a BTreeMap<String, Relationship>,
    shared: &'a [String],
    styles: &'a StyleTable,
    date_system: SpreadsheetDateSystem,
}

fn parse_worksheet(
    definition: &SheetDefinition,
    part_name: &str,
    content: &[u8],
    context: &WorksheetContext<'_>,
    remaining_cells: &mut usize,
    profile: &SpreadsheetProfile,
    control: &mut InspectionControl<'_>,
) -> Result<SpreadsheetWorksheet, SpreadsheetError> {
    let hyperlinks = hyperlink_records(content, context.relationships, profile, control)?;
    let hyperlink_map = hyperlinks
        .iter()
        .map(|item| (item.reference.as_str(), item.clone()))
        .collect::<BTreeMap<_, _>>();
    let mut reader = Reader::from_reader(content);
    reader.config_mut().trim_text(false);
    let mut dimension = None;
    let mut protected = false;
    let mut hidden_rows = BTreeSet::new();
    let mut hidden_columns = BTreeSet::new();
    let mut merged_ranges = Vec::new();
    let mut cells = Vec::new();
    let mut current_cell: Option<(String, Option<String>, Option<u32>)> = None;
    let mut formula = None;
    let mut value = None;
    let mut inline = String::new();
    let mut capture_formula = false;
    let mut capture_value = false;
    let mut capture_inline = false;
    loop {
        control.checkpoint()?;
        match reader
            .read_event()
            .map_err(|_| SpreadsheetError::MalformedPackage)?
        {
            Event::Start(event) | Event::Empty(event)
                if local_name(event.name().as_ref()) == b"dimension" =>
            {
                dimension = attribute(&event, b"ref", &reader)?;
            }
            Event::Start(event) | Event::Empty(event)
                if local_name(event.name().as_ref()) == b"sheetProtection" =>
            {
                protected = true;
            }
            Event::Start(event)
                if local_name(event.name().as_ref()) == b"row"
                    && attribute(&event, b"hidden", &reader)?
                        .is_some_and(|item| matches!(item.as_str(), "1" | "true")) =>
            {
                let row = attribute(&event, b"r", &reader)?
                    .ok_or(SpreadsheetError::MalformedPackage)?
                    .parse()
                    .map_err(|_| SpreadsheetError::MalformedPackage)?;
                hidden_rows.insert(row);
            }
            Event::Start(event) | Event::Empty(event)
                if local_name(event.name().as_ref()) == b"col"
                    && attribute(&event, b"hidden", &reader)?
                        .is_some_and(|item| matches!(item.as_str(), "1" | "true")) =>
            {
                let min: u32 = attribute(&event, b"min", &reader)?
                    .ok_or(SpreadsheetError::MalformedPackage)?
                    .parse()
                    .map_err(|_| SpreadsheetError::MalformedPackage)?;
                let max: u32 = attribute(&event, b"max", &reader)?
                    .ok_or(SpreadsheetError::MalformedPackage)?
                    .parse()
                    .map_err(|_| SpreadsheetError::MalformedPackage)?;
                if min == 0 || max < min || max - min > 16_384 {
                    return Err(SpreadsheetError::ResourceLimit);
                }
                hidden_columns.extend(min..=max);
            }
            Event::Start(event) if local_name(event.name().as_ref()) == b"c" => {
                if current_cell.is_some() {
                    return Err(SpreadsheetError::MalformedPackage);
                }
                let address =
                    attribute(&event, b"r", &reader)?.ok_or(SpreadsheetError::MalformedPackage)?;
                let data_type = attribute(&event, b"t", &reader)?;
                let style = attribute(&event, b"s", &reader)?
                    .map(|item| item.parse().map_err(|_| SpreadsheetError::MalformedPackage))
                    .transpose()?;
                current_cell = Some((address, data_type, style));
                formula = None;
                value = None;
                inline.clear();
            }
            Event::Start(event)
                if current_cell.is_some() && local_name(event.name().as_ref()) == b"f" =>
            {
                capture_formula = true
            }
            Event::Start(event)
                if current_cell.is_some() && local_name(event.name().as_ref()) == b"v" =>
            {
                capture_value = true
            }
            Event::Start(event)
                if current_cell.is_some() && local_name(event.name().as_ref()) == b"t" =>
            {
                capture_inline = true
            }
            Event::Text(event) if capture_formula => formula
                .get_or_insert_with(String::new)
                .push_str(&decoded_text(event)?),
            Event::Text(event) if capture_value => value
                .get_or_insert_with(String::new)
                .push_str(&decoded_text(event)?),
            Event::Text(event) if capture_inline => inline.push_str(&decoded_text(event)?),
            Event::GeneralRef(event) if capture_formula => formula
                .get_or_insert_with(String::new)
                .push_str(&decoded_reference(event)?),
            Event::GeneralRef(event) if capture_value => value
                .get_or_insert_with(String::new)
                .push_str(&decoded_reference(event)?),
            Event::GeneralRef(event) if capture_inline => {
                inline.push_str(&decoded_reference(event)?)
            }
            Event::End(event) if local_name(event.name().as_ref()) == b"f" => {
                capture_formula = false
            }
            Event::End(event) if local_name(event.name().as_ref()) == b"v" => capture_value = false,
            Event::End(event) if local_name(event.name().as_ref()) == b"t" => {
                capture_inline = false
            }
            Event::End(event) if local_name(event.name().as_ref()) == b"c" => {
                let (address, data_type, style_index) = current_cell
                    .take()
                    .ok_or(SpreadsheetError::MalformedPackage)?;
                let (row, column) =
                    cell_coordinates(&address).ok_or(SpreadsheetError::MalformedPackage)?;
                if row > profile.maximum_rows
                    || column > profile.maximum_columns
                    || address.len() > profile.maximum_string_bytes
                    || formula
                        .as_ref()
                        .is_some_and(|item| item.len() > profile.maximum_string_bytes)
                    || value
                        .as_ref()
                        .is_some_and(|item| item.len() > profile.maximum_string_bytes)
                    || inline.len() > profile.maximum_string_bytes
                {
                    return Err(SpreadsheetError::ResourceLimit);
                }
                if *remaining_cells == 0 {
                    return Err(SpreadsheetError::ResourceLimit);
                }
                *remaining_cells -= 1;
                let number_format_id = style_index.and_then(|index| {
                    context
                        .styles
                        .cell_number_formats
                        .get(index as usize)
                        .copied()
                });
                let date_value = number_format_id
                    .filter(|id| date_format(*id, context.styles))
                    .and(value.as_deref())
                    .and_then(|item| excel_serial_date(item, context.date_system));
                let displayed_value = match data_type.as_deref() {
                    Some("s") => value
                        .as_deref()
                        .and_then(|item| item.parse::<usize>().ok())
                        .and_then(|index| context.shared.get(index))
                        .cloned(),
                    Some("inlineStr") => (!inline.is_empty()).then(|| inline.clone()),
                    _ => date_value.clone().or_else(|| value.clone()),
                };
                let kind = if formula.is_some() {
                    SpreadsheetCellKind::Formula
                } else if date_value.is_some() {
                    SpreadsheetCellKind::Date
                } else {
                    match data_type.as_deref() {
                        Some("s") => SpreadsheetCellKind::SharedString,
                        Some("inlineStr") => SpreadsheetCellKind::InlineString,
                        Some("str") => SpreadsheetCellKind::String,
                        Some("b") => SpreadsheetCellKind::Boolean,
                        Some("e") => SpreadsheetCellKind::Error,
                        _ if value.is_some() => SpreadsheetCellKind::Number,
                        _ => SpreadsheetCellKind::Blank,
                    }
                };
                cells.push(SpreadsheetCell {
                    hyperlink: hyperlink_map.get(address.as_str()).cloned(),
                    address,
                    row,
                    column,
                    kind,
                    raw_value: value.clone(),
                    displayed_value,
                    formula: formula.clone(),
                    style_index,
                    number_format_id,
                    date_value,
                });
            }
            Event::Start(event) | Event::Empty(event)
                if local_name(event.name().as_ref()) == b"mergeCell" =>
            {
                let range = attribute(&event, b"ref", &reader)?
                    .ok_or(SpreadsheetError::MalformedPackage)?;
                if range.len() > profile.maximum_string_bytes {
                    return Err(SpreadsheetError::ResourceLimit);
                }
                merged_ranges.push(range);
            }
            Event::Eof => break,
            _ => {}
        }
    }
    if current_cell.is_some() || capture_formula || capture_value || capture_inline {
        return Err(SpreadsheetError::MalformedPackage);
    }
    cells.sort_by_key(|cell| (cell.row, cell.column));
    merged_ranges.sort();
    merged_ranges.dedup();
    let sparse_dimension = dimension
        .as_deref()
        .and_then(dimension_area)
        .is_some_and(|area| {
            area > u64::try_from(cells.len())
                .unwrap_or(u64::MAX)
                .saturating_mul(1_024)
        });
    Ok(SpreadsheetWorksheet {
        sheet_id: definition.sheet_id,
        name: definition.name.clone(),
        state: definition.state,
        part_name: part_name.to_owned(),
        part_sha256: word_sha256(content),
        dimension,
        protected,
        sparse_dimension,
        cells,
        hidden_rows: hidden_rows.into_iter().collect(),
        hidden_columns: hidden_columns.into_iter().collect(),
        merged_ranges,
        hyperlinks,
    })
}

fn finding(
    findings: &mut Vec<SpreadsheetFinding>,
    kind: SpreadsheetFindingKind,
    part_name: Option<&str>,
    sheet_name: Option<&str>,
    cell_reference: Option<&str>,
    reason_code: &str,
    blocks: bool,
) {
    let identity = format!(
        "{}:{}:{}",
        part_name.unwrap_or("package"),
        sheet_name.unwrap_or("workbook"),
        cell_reference.unwrap_or("none")
    );
    findings.push(SpreadsheetFinding {
        finding_id: format!(
            "spreadsheet-finding:{}",
            &word_sha256(format!("{reason_code}\n{identity}").as_bytes())[..24]
        ),
        kind,
        part_name: part_name.map(str::to_owned),
        sheet_name: sheet_name.map(str::to_owned),
        cell_reference: cell_reference.map(str::to_owned),
        reason_code: reason_code.to_owned(),
        blocks_safe_analysis: blocks,
    });
}

/// Inspects caller-supplied `.xlsx` bytes through a bounded direct Open XML path.
pub fn inspect_xlsx(
    source_path: WorkspacePath,
    source: &[u8],
    profile: &SpreadsheetProfile,
) -> Result<SpreadsheetInspection, SpreadsheetError> {
    inspect_xlsx_with_control(source_path, source, profile, &mut || false)
}

/// Inspects `.xlsx` bytes with cooperative cancellation and the profile's hard elapsed ceiling.
pub fn inspect_xlsx_with_control(
    source_path: WorkspacePath,
    source: &[u8],
    profile: &SpreadsheetProfile,
    cancelled: &mut dyn FnMut() -> bool,
) -> Result<SpreadsheetInspection, SpreadsheetError> {
    if !profile.valid() {
        return Err(SpreadsheetError::InvalidInput);
    }
    let mut control = InspectionControl::new(profile, source.len(), cancelled)?;
    control.checkpoint()?;
    let parts = package_parts(source, profile, &mut control)?;
    let workbook = parts
        .get("xl/workbook.xml")
        .ok_or(SpreadsheetError::MalformedPackage)?;
    let workbook_relationships = parse_relationships(
        parts
            .get("xl/_rels/workbook.xml.rels")
            .ok_or(SpreadsheetError::MalformedPackage)?,
        profile,
        &mut control,
    )?;
    let (date_system, definitions) = parse_workbook(workbook, profile, &mut control)?;
    if definitions.is_empty() || definitions.len() > profile.maximum_sheets {
        return Err(SpreadsheetError::ResourceLimit);
    }
    let shared = parse_shared_strings(parts.get("xl/sharedStrings.xml"), profile, &mut control)?;
    let styles = parse_styles(parts.get("xl/styles.xml"), profile, &mut control)?;
    let mut remaining_cells = profile.maximum_cells;
    let mut worksheets = Vec::new();
    let mut findings = Vec::new();
    if parts.keys().any(|name| name.ends_with("vbaProject.bin"))
        || parts.iter().any(|(name, content)| {
            (name.ends_with(".xml") || name.ends_with(".rels"))
                && String::from_utf8_lossy(content)
                    .to_ascii_lowercase()
                    .contains("macroenabled")
        })
    {
        finding(
            &mut findings,
            SpreadsheetFindingKind::MacroContent,
            None,
            None,
            None,
            "spreadsheet.macro-content",
            true,
        );
    }
    for part_name in parts.keys().filter(|name| {
        name.starts_with("xl/embeddings/")
            || name.starts_with("xl/activeX/")
            || name.starts_with("xl/ctrlProps/")
    }) {
        control.checkpoint()?;
        finding(
            &mut findings,
            SpreadsheetFindingKind::EmbeddedObject,
            Some(part_name),
            None,
            None,
            "spreadsheet.embedded-object-not-opened",
            true,
        );
    }
    for part_name in parts.keys().filter(|name| {
        name.starts_with("xl/pivotCache/")
            || name.starts_with("xl/pivotTables/")
            || name.starts_with("xl/slicers/")
            || name.starts_with("xl/queryTables/")
            || name.as_str() == "xl/connections.xml"
    }) {
        control.checkpoint()?;
        finding(
            &mut findings,
            SpreadsheetFindingKind::UnsupportedFeature,
            Some(part_name),
            None,
            None,
            "spreadsheet.feature-preserved-not-interpreted",
            false,
        );
    }
    for definition in &definitions {
        control.checkpoint()?;
        let relationship = workbook_relationships
            .get(&definition.relationship_id)
            .ok_or(SpreadsheetError::MalformedPackage)?;
        if relationship.external {
            return Err(SpreadsheetError::UnsafePackage);
        }
        if !relationship.relation_type.ends_with("/worksheet") {
            return Err(SpreadsheetError::MalformedPackage);
        }
        let part_name = workbook_target(&relationship.target)?;
        let content = parts
            .get(&part_name)
            .ok_or(SpreadsheetError::MalformedPackage)?;
        let rels = worksheet_relationship_path(&part_name)
            .and_then(|name| parts.get(&name))
            .map(|content| parse_relationships(content, profile, &mut control))
            .transpose()?
            .unwrap_or_default();
        let context = WorksheetContext {
            relationships: &rels,
            shared: &shared,
            styles: &styles,
            date_system,
        };
        let sheet = parse_worksheet(
            definition,
            &part_name,
            content,
            &context,
            &mut remaining_cells,
            profile,
            &mut control,
        )?;
        if sheet.protected {
            finding(
                &mut findings,
                SpreadsheetFindingKind::ProtectedSheet,
                Some(&part_name),
                Some(&sheet.name),
                None,
                "spreadsheet.sheet-protection-preserved",
                false,
            );
        }
        if sheet.sparse_dimension {
            finding(
                &mut findings,
                SpreadsheetFindingKind::SparseRange,
                Some(&part_name),
                Some(&sheet.name),
                sheet.dimension.as_deref(),
                "spreadsheet.sparse-range-not-expanded",
                false,
            );
        }
        for link in &sheet.hyperlinks {
            control.checkpoint()?;
            if link.external {
                finding(
                    &mut findings,
                    SpreadsheetFindingKind::ExternalHyperlink,
                    Some(&part_name),
                    Some(&sheet.name),
                    Some(&link.reference),
                    "spreadsheet.external-hyperlink-inert",
                    false,
                );
            }
        }
        for cell in &sheet.cells {
            control.checkpoint()?;
            if let Some(formula) = cell.formula.as_deref() {
                let lower = formula.to_ascii_lowercase();
                if lower.contains("dde") {
                    finding(
                        &mut findings,
                        SpreadsheetFindingKind::DdeFormula,
                        Some(&part_name),
                        Some(&sheet.name),
                        Some(&cell.address),
                        "spreadsheet.dde-formula-inert",
                        true,
                    );
                }
                if formula.contains('[') && formula.contains(']') {
                    finding(
                        &mut findings,
                        SpreadsheetFindingKind::ExternalWorkbookReference,
                        Some(&part_name),
                        Some(&sheet.name),
                        Some(&cell.address),
                        "spreadsheet.external-workbook-formula-inert",
                        true,
                    );
                }
            }
        }
        worksheets.push(sheet);
    }
    for relationship in workbook_relationships.values() {
        control.checkpoint()?;
        if relationship.external || relationship.relation_type.contains("externalLink") {
            finding(
                &mut findings,
                SpreadsheetFindingKind::ExternalWorkbookReference,
                Some("xl/_rels/workbook.xml.rels"),
                None,
                None,
                "spreadsheet.external-workbook-relationship-inert",
                true,
            );
        }
    }
    findings.sort_by(|left, right| left.finding_id.cmp(&right.finding_id));
    let safe_for_analysis = !findings.iter().any(|item| item.blocks_safe_analysis);
    Ok(SpreadsheetInspection {
        schema_version: CONTRACT_SCHEMA_VERSION,
        source_path,
        source_sha256: word_sha256(source),
        profile: profile.clone(),
        date_system,
        worksheets,
        findings,
        inspection_complete: true,
        safe_for_analysis,
        original_preserved: true,
        filesystem_effect_performed: false,
        network_access_performed: false,
        execution_performed: false,
    })
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::thread;

    use agentmage_kernel_contracts::{WorkspaceId, WorkspacePath};

    use super::*;
    use crate::word_generation::zip_parts;

    fn path() -> WorkspacePath {
        WorkspacePath::new(
            WorkspaceId::from_raw("workspace-sheet"),
            ["data", "book.xlsx"],
        )
        .expect("path")
    }

    fn fixture_parts(formula: &str) -> BTreeMap<String, Vec<u8>> {
        let mut parts = BTreeMap::new();
        parts.insert("[Content_Types].xml".to_owned(), b"<?xml version=\"1.0\"?><Types xmlns=\"http://schemas.openxmlformats.org/package/2006/content-types\"><Override PartName=\"/xl/workbook.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml\"/></Types>".to_vec());
        parts.insert("xl/workbook.xml".to_owned(), b"<?xml version=\"1.0\"?><workbook xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\" xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\"><workbookPr date1904=\"0\"/><sheets><sheet name=\"Data\" sheetId=\"1\" r:id=\"rId1\"/></sheets></workbook>".to_vec());
        parts.insert("xl/_rels/workbook.xml.rels".to_owned(), b"<?xml version=\"1.0\"?><Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\"><Relationship Id=\"rId1\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet\" Target=\"worksheets/sheet1.xml\"/></Relationships>".to_vec());
        parts.insert("xl/sharedStrings.xml".to_owned(), b"<?xml version=\"1.0\"?><sst xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\"><si><t>Alice</t></si></sst>".to_vec());
        parts.insert("xl/styles.xml".to_owned(), b"<?xml version=\"1.0\"?><styleSheet xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\"><cellXfs count=\"2\"><xf numFmtId=\"0\"/><xf numFmtId=\"14\"/></cellXfs></styleSheet>".to_vec());
        parts.insert("xl/worksheets/_rels/sheet1.xml.rels".to_owned(), b"<?xml version=\"1.0\"?><Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\"><Relationship Id=\"rIdH\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/hyperlink\" Target=\"https://example.invalid\" TargetMode=\"External\"/></Relationships>".to_vec());
        parts.insert("xl/worksheets/sheet1.xml".to_owned(), format!("<?xml version=\"1.0\"?><worksheet xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\" xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\"><dimension ref=\"A1:D3\"/><cols><col min=\"2\" max=\"2\" hidden=\"1\"/></cols><sheetData><row r=\"1\"><c r=\"A1\" t=\"s\"><v>0</v></c><c r=\"B1\" t=\"inlineStr\"><is><t>Link</t></is></c></row><row r=\"2\" hidden=\"1\"><c r=\"A2\"><v>2</v></c><c r=\"B2\" s=\"1\"><v>46000</v></c><c r=\"C2\"><f>{formula}</f><v>3</v></c><c r=\"D2\" t=\"b\"><v>1</v></c></row></sheetData><mergeCells count=\"1\"><mergeCell ref=\"A3:B3\"/></mergeCells><hyperlinks><hyperlink ref=\"B1\" r:id=\"rIdH\"/></hyperlinks></worksheet>").into_bytes());
        parts
    }

    fn fixture(formula: &str) -> Vec<u8> {
        zip_parts(&fixture_parts(formula)).expect("zip")
    }

    #[test]
    fn inspects_cells_styles_dates_links_hidden_data_and_merges_without_effects() {
        let report = inspect_xlsx(
            path(),
            &fixture("SUM(A2,1)"),
            &SpreadsheetProfile::default(),
        )
        .expect("inspect");
        assert!(report.safe_for_analysis);
        assert_eq!(report.worksheets.len(), 1);
        let sheet = &report.worksheets[0];
        assert_eq!(sheet.hidden_rows, [2]);
        assert_eq!(sheet.hidden_columns, [2]);
        assert_eq!(sheet.merged_ranges, ["A3:B3"]);
        assert_eq!(sheet.cells[0].displayed_value.as_deref(), Some("Alice"));
        assert!(
            sheet
                .cells
                .iter()
                .find(|cell| cell.address == "B2")
                .expect("date")
                .date_value
                .is_some()
        );
        assert_eq!(
            sheet
                .cells
                .iter()
                .find(|cell| cell.address == "C2")
                .expect("formula")
                .formula
                .as_deref(),
            Some("SUM(A2,1)")
        );
        assert!(
            sheet
                .cells
                .iter()
                .find(|cell| cell.address == "B1")
                .expect("link")
                .hyperlink
                .as_ref()
                .is_some_and(|link| link.external && !link.followed)
        );
        assert!(
            report
                .findings
                .iter()
                .all(|item| !item.blocks_safe_analysis)
        );
        assert!(report.original_preserved);
        assert!(!report.filesystem_effect_performed);
        assert!(!report.network_access_performed);
        assert!(!report.execution_performed);
    }

    #[test]
    fn external_formula_and_dde_are_inert_and_block_safe_analysis() {
        for formula in ["[outside.xlsx]Sheet1!A1", "DDE(server,topic)"] {
            let report = inspect_xlsx(path(), &fixture(formula), &SpreadsheetProfile::default())
                .expect("inspect");
            assert!(!report.safe_for_analysis);
            assert!(report.findings.iter().any(|item| item.blocks_safe_analysis));
            assert!(!report.execution_performed);
        }
    }

    #[test]
    fn malformed_missing_oversized_and_macro_packages_fail_or_quarantine() {
        assert!(inspect_xlsx(path(), b"not a zip", &SpreadsheetProfile::default()).is_err());
        let tiny = SpreadsheetProfile {
            maximum_source_bytes: 8,
            ..SpreadsheetProfile::default()
        };
        assert_eq!(
            inspect_xlsx(path(), &fixture("1+1"), &tiny),
            Err(SpreadsheetError::ResourceLimit)
        );
        let mut parts = BTreeMap::new();
        parts.insert("xl/workbook.xml".to_owned(), b"<workbook/>".to_vec());
        assert!(
            inspect_xlsx(
                path(),
                &zip_parts(&parts).expect("zip"),
                &SpreadsheetProfile::default()
            )
            .is_err()
        );

        let mut macro_parts = fixture_parts("1+1");
        macro_parts.insert("xl/vbaProject.bin".to_owned(), b"inert fixture".to_vec());
        let macro_report = inspect_xlsx(
            path(),
            &zip_parts(&macro_parts).expect("zip"),
            &SpreadsheetProfile::default(),
        )
        .expect("quarantine macro");
        assert!(!macro_report.safe_for_analysis);
        assert!(macro_report.findings.iter().any(|item| {
            item.kind == SpreadsheetFindingKind::MacroContent && item.blocks_safe_analysis
        }));
        assert!(!macro_report.execution_performed);
    }

    #[test]
    fn excel_serial_conversion_preserves_both_date_systems_and_leap_bug() {
        assert_eq!(
            excel_serial_date("1", SpreadsheetDateSystem::Excel1900).as_deref(),
            Some("1900-01-01")
        );
        assert_eq!(
            excel_serial_date("60", SpreadsheetDateSystem::Excel1900).as_deref(),
            Some("1900-02-29")
        );
        assert_eq!(
            excel_serial_date("0", SpreadsheetDateSystem::Excel1904).as_deref(),
            Some("1904-01-01")
        );
        assert_eq!(
            excel_serial_date("-1", SpreadsheetDateSystem::Excel1900),
            None
        );
        assert_eq!(
            excel_serial_date("1.5", SpreadsheetDateSystem::Excel1900),
            None
        );
    }

    #[test]
    fn explicit_coordinate_memory_time_and_cancellation_ceilings_fail_closed() {
        let source = fixture("SUM(A2,1)");
        for profile in [
            SpreadsheetProfile {
                maximum_rows: 1,
                ..SpreadsheetProfile::default()
            },
            SpreadsheetProfile {
                maximum_columns: 2,
                ..SpreadsheetProfile::default()
            },
            SpreadsheetProfile {
                maximum_string_bytes: 4,
                ..SpreadsheetProfile::default()
            },
            SpreadsheetProfile {
                maximum_source_bytes: source.len() as u64,
                maximum_working_memory_bytes: source.len() as u64,
                ..SpreadsheetProfile::default()
            },
        ] {
            assert_eq!(
                inspect_xlsx(path(), &source, &profile),
                Err(SpreadsheetError::ResourceLimit)
            );
        }

        assert_eq!(
            inspect_xlsx_with_control(path(), &source, &SpreadsheetProfile::default(), &mut || {
                true
            },),
            Err(SpreadsheetError::Cancelled)
        );
        let timed = SpreadsheetProfile {
            maximum_elapsed_milliseconds: 1,
            ..SpreadsheetProfile::default()
        };
        let mut first = true;
        assert_eq!(
            inspect_xlsx_with_control(path(), &source, &timed, &mut || {
                if first {
                    first = false;
                    thread::sleep(Duration::from_millis(2));
                }
                false
            }),
            Err(SpreadsheetError::TimeLimit)
        );
    }
}
