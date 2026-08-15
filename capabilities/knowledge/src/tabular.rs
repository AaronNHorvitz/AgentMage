//! Bounded delimited-table parsing, analysis, comparison, and safe CSV proposals.

use std::collections::{BTreeMap, BTreeSet};

use agentmage_kernel_contracts::{CONTRACT_SCHEMA_VERSION, WorkspacePath};
use serde::{Deserialize, Serialize};
use unicode_normalization::UnicodeNormalization;

use crate::word_ooxml::word_sha256;

const MAX_SOURCE_BYTES: usize = 256 * 1_024 * 1_024;
const MAX_ROWS: usize = 1_000_000;
const MAX_COLUMNS: usize = 16_384;
const MAX_FIELD_BYTES: usize = 16 * 1_024 * 1_024;

/// Closed resource profile for delimited input and derived table operations.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TabularProfile {
    /// Stable profile identity.
    pub profile_id: String,
    /// Maximum source bytes.
    pub maximum_source_bytes: usize,
    /// Maximum logical records including the header.
    pub maximum_rows: usize,
    /// Maximum fields in one record.
    pub maximum_columns: usize,
    /// Maximum UTF-8 bytes in one decoded field.
    pub maximum_field_bytes: usize,
}

impl Default for TabularProfile {
    fn default() -> Self {
        Self {
            profile_id: "tabular-strict-v1".to_owned(),
            maximum_source_bytes: 64 * 1_024 * 1_024,
            maximum_rows: 250_000,
            maximum_columns: 4_096,
            maximum_field_bytes: 4 * 1_024 * 1_024,
        }
    }
}

impl TabularProfile {
    fn valid(&self) -> bool {
        valid_identifier(&self.profile_id)
            && self.maximum_source_bytes > 0
            && self.maximum_source_bytes <= MAX_SOURCE_BYTES
            && self.maximum_rows > 1
            && self.maximum_rows <= MAX_ROWS
            && self.maximum_columns > 0
            && self.maximum_columns <= MAX_COLUMNS
            && self.maximum_field_bytes > 0
            && self.maximum_field_bytes <= MAX_FIELD_BYTES
    }
}

/// Closed supported delimited-text dialect.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DelimitedDialect {
    /// Comma-separated values.
    Comma,
    /// Tab-separated values.
    Tab,
    /// Semicolon-separated values.
    Semicolon,
}

impl DelimitedDialect {
    const fn delimiter(self) -> u8 {
        match self {
            Self::Comma => b',',
            Self::Tab => b'\t',
            Self::Semicolon => b';',
        }
    }
}

/// One bounded read-only table parsed from an exact source.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TabularDocument {
    /// Kernel contract schema version.
    pub schema_version: u16,
    /// Canonical caller-authorized source path.
    pub source_path: WorkspacePath,
    /// Exact source digest.
    pub source_sha256: String,
    /// Exact parser profile.
    pub profile: TabularProfile,
    /// Declared dialect.
    pub dialect: DelimitedDialect,
    /// Original header strings.
    pub headers: Vec<String>,
    /// Stable normalized unique header identities.
    pub normalized_headers: Vec<String>,
    /// Ordered data rows.
    pub rows: Vec<Vec<String>>,
    /// True because the caller-supplied source remains authoritative.
    pub original_preserved: bool,
    /// False because parsing occurs in memory.
    pub filesystem_effect_performed: bool,
    /// False because parsing uses no network.
    pub network_access_performed: bool,
    /// False because cells and formulas remain inert strings.
    pub execution_performed: bool,
}

/// One deterministic table summary.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TabularSummary {
    /// Exact source digest.
    pub source_sha256: String,
    /// Header count.
    pub column_count: usize,
    /// Data-row count.
    pub row_count: usize,
    /// Missing or whitespace-only values by normalized header.
    pub missing_by_column: BTreeMap<String, usize>,
    /// Duplicate row groups by exact canonical row digest.
    pub duplicate_row_groups: BTreeMap<String, Vec<u32>>,
}

/// Closed filter predicate.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum TabularFilter {
    /// Normalized equality.
    Equals {
        /// Normalized header identity.
        column: String,
        /// Comparison value.
        value: String,
    },
    /// Normalized substring containment.
    Contains {
        /// Normalized header identity.
        column: String,
        /// Required substring.
        value: String,
    },
    /// Missing or whitespace-only value.
    Missing {
        /// Normalized header identity.
        column: String,
    },
}

/// Closed deterministic sort direction.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TabularSortDirection {
    /// Ascending normalized lexical order.
    Ascending,
    /// Descending normalized lexical order.
    Descending,
}

/// One sort key.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TabularSort {
    /// Normalized header identity.
    pub column: String,
    /// Sort direction.
    pub direction: TabularSortDirection,
}

/// Stable source-row projection after filtering and sorting.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TabularRowProjection {
    /// One-based source data-row number, excluding the header.
    pub source_row: u32,
    /// SHA-256 of the exact canonical row values.
    pub row_sha256: String,
}

/// Closed cross-table match or discrepancy reason.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TabularMatchReason {
    /// One unique key and all fields match after declared normalization.
    ExactRow,
    /// One unique key matches but one or more fields differ.
    DifferingFields,
    /// Key exists only in the left source.
    MissingRight,
    /// Key exists only in the right source.
    MissingLeft,
    /// Key is duplicated in the left source.
    DuplicateLeft,
    /// Key is duplicated in the right source.
    DuplicateRight,
}

/// One content-minimized deterministic table comparison entry.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TabularMatch {
    /// SHA-256 of normalized key components.
    pub key_sha256: String,
    /// Stable reason.
    pub reason: TabularMatchReason,
    /// One-based matching left rows.
    pub left_rows: Vec<u32>,
    /// One-based matching right rows.
    pub right_rows: Vec<u32>,
    /// Canonically ordered normalized headers whose values differ.
    pub differing_fields: Vec<String>,
}

/// Exact deterministic comparison between two table sources.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TabularComparison {
    /// Exact left source digest.
    pub left_source_sha256: String,
    /// Exact right source digest.
    pub right_source_sha256: String,
    /// Ordered normalized key headers.
    pub key_columns: Vec<String>,
    /// Canonically ordered match records.
    pub matches: Vec<TabularMatch>,
    /// Count of keys present in both sources.
    pub overlap_key_count: usize,
    /// False because comparison is in memory.
    pub filesystem_effect_performed: bool,
    /// False because comparison uses no network.
    pub network_access_performed: bool,
    /// False because values remain inert data.
    pub execution_performed: bool,
}

/// Safe CSV proposal with exact sanitization accounting.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SafeCsvProposal {
    /// Exact source digest.
    pub source_sha256: String,
    /// Exact output bytes.
    pub csv: Vec<u8>,
    /// Exact output digest.
    pub csv_sha256: String,
    /// Number of cells prefixed to prevent formula interpretation.
    pub formula_sanitized_cell_count: u64,
    /// True only when every output cell passed formula-safety processing.
    pub formula_injection_prevented: bool,
    /// False because the proposal is returned without writing it.
    pub filesystem_effect_performed: bool,
    /// False because output uses no network.
    pub network_access_performed: bool,
    /// False because no cell is evaluated.
    pub execution_performed: bool,
}

/// Stable bounded tabular failure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TabularError {
    /// Profile, header, filter, sort, or key is invalid.
    InvalidInput,
    /// Input is not valid UTF-8.
    InvalidUtf8,
    /// Quoting or record shape is malformed.
    MalformedDelimitedInput,
    /// Source, row, column, field, or output limit was exceeded.
    ResourceLimit,
    /// Two table header sets are incompatible for comparison.
    IncompatibleHeaders,
}

impl TabularError {
    /// Stable content-free error code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidInput => "tabular.input.invalid",
            Self::InvalidUtf8 => "tabular.utf8.invalid",
            Self::MalformedDelimitedInput => "tabular.delimited.malformed",
            Self::ResourceLimit => "tabular.resource.limit",
            Self::IncompatibleHeaders => "tabular.headers.incompatible",
        }
    }
}

impl std::fmt::Display for TabularError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.code())
    }
}

impl std::error::Error for TabularError {}

fn valid_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value.is_ascii()
        && value
            .bytes()
            .next()
            .is_some_and(|byte| byte.is_ascii_alphanumeric())
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b':' | b'-'))
}

/// Trims and collapses Unicode whitespace without changing case or compatibility form.
#[must_use]
pub fn clean_tabular_text(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Produces a deterministic NFKC, lowercase, whitespace-collapsed comparison value.
#[must_use]
pub fn normalized_tabular_text(value: &str) -> String {
    clean_tabular_text(&value.nfkc().collect::<String>()).to_lowercase()
}

/// Produces an ASCII identifier fragment from normalized text.
#[must_use]
pub fn normalized_tabular_id(value: &str) -> String {
    let normalized = normalized_tabular_text(value);
    let mut result = String::new();
    let mut separator = false;
    for character in normalized.chars() {
        if character.is_ascii_alphanumeric() {
            if separator && !result.is_empty() {
                result.push('-');
            }
            result.push(character);
            separator = false;
        } else {
            separator = true;
        }
    }
    result.trim_matches('-').to_owned()
}

/// Produces a safe deterministic filename component without path separators.
#[must_use]
pub fn normalized_tabular_filename(value: &str) -> String {
    let id = normalized_tabular_id(value);
    if id.is_empty() {
        "untitled".to_owned()
    } else {
        id
    }
}

/// Produces a comparison-only URL form without resolving or fetching it.
#[must_use]
pub fn normalized_tabular_url(value: &str) -> String {
    normalized_tabular_text(value)
        .trim_end_matches('/')
        .to_owned()
}

fn finish_field(
    field: &mut Vec<u8>,
    record: &mut Vec<String>,
    profile: &TabularProfile,
) -> Result<(), TabularError> {
    if field.len() > profile.maximum_field_bytes {
        return Err(TabularError::ResourceLimit);
    }
    record.push(
        std::str::from_utf8(field)
            .map_err(|_| TabularError::InvalidUtf8)?
            .to_owned(),
    );
    field.clear();
    if record.len() > profile.maximum_columns {
        return Err(TabularError::ResourceLimit);
    }
    Ok(())
}

fn finish_record(
    record: &mut Vec<String>,
    records: &mut Vec<Vec<String>>,
    profile: &TabularProfile,
) -> Result<(), TabularError> {
    records.push(std::mem::take(record));
    if records.len() > profile.maximum_rows {
        return Err(TabularError::ResourceLimit);
    }
    Ok(())
}

fn parse_records(
    bytes: &[u8],
    dialect: DelimitedDialect,
    profile: &TabularProfile,
) -> Result<Vec<Vec<String>>, TabularError> {
    let delimiter = dialect.delimiter();
    let (mut records, mut record, mut field) = (Vec::new(), Vec::new(), Vec::new());
    let (mut quoted, mut after_quote, mut index) = (false, false, 0_usize);
    while index < bytes.len() {
        let byte = bytes[index];
        if quoted {
            if byte == b'"' {
                if bytes.get(index + 1) == Some(&b'"') {
                    field.push(b'"');
                    index += 1;
                } else {
                    quoted = false;
                    after_quote = true;
                }
            } else {
                field.push(byte);
            }
        } else if after_quote {
            if byte == delimiter {
                finish_field(&mut field, &mut record, profile)?;
                after_quote = false;
            } else if matches!(byte, b'\r' | b'\n') {
                finish_field(&mut field, &mut record, profile)?;
                finish_record(&mut record, &mut records, profile)?;
                after_quote = false;
                if byte == b'\r' && bytes.get(index + 1) == Some(&b'\n') {
                    index += 1;
                }
            } else {
                return Err(TabularError::MalformedDelimitedInput);
            }
        } else if byte == b'"' {
            if !field.is_empty() {
                return Err(TabularError::MalformedDelimitedInput);
            }
            quoted = true;
        } else if byte == delimiter {
            finish_field(&mut field, &mut record, profile)?;
        } else if matches!(byte, b'\r' | b'\n') {
            finish_field(&mut field, &mut record, profile)?;
            finish_record(&mut record, &mut records, profile)?;
            if byte == b'\r' && bytes.get(index + 1) == Some(&b'\n') {
                index += 1;
            }
        } else {
            field.push(byte);
        }
        if field.len() > profile.maximum_field_bytes {
            return Err(TabularError::ResourceLimit);
        }
        index += 1;
    }
    if quoted {
        return Err(TabularError::MalformedDelimitedInput);
    }
    if after_quote || !field.is_empty() || !record.is_empty() {
        finish_field(&mut field, &mut record, profile)?;
        finish_record(&mut record, &mut records, profile)?;
    }
    Ok(records)
}

/// Parses exact caller-supplied delimited bytes into a closed read-only table.
pub fn parse_delimited_table(
    source_path: WorkspacePath,
    bytes: &[u8],
    dialect: DelimitedDialect,
    profile: &TabularProfile,
) -> Result<TabularDocument, TabularError> {
    if !profile.valid() || bytes.is_empty() {
        return Err(TabularError::InvalidInput);
    }
    if bytes.len() > profile.maximum_source_bytes {
        return Err(TabularError::ResourceLimit);
    }
    std::str::from_utf8(bytes).map_err(|_| TabularError::InvalidUtf8)?;
    let mut records = parse_records(bytes, dialect, profile)?;
    if records.is_empty() {
        return Err(TabularError::InvalidInput);
    }
    let headers = records.remove(0);
    let normalized_headers = headers
        .iter()
        .map(|header| normalized_tabular_id(header))
        .collect::<Vec<_>>();
    if headers.is_empty()
        || normalized_headers.iter().any(String::is_empty)
        || normalized_headers.iter().collect::<BTreeSet<_>>().len() != headers.len()
    {
        return Err(TabularError::InvalidInput);
    }
    if records.iter().any(|row| row.len() != headers.len()) {
        return Err(TabularError::MalformedDelimitedInput);
    }
    Ok(TabularDocument {
        schema_version: CONTRACT_SCHEMA_VERSION,
        source_path,
        source_sha256: word_sha256(bytes),
        profile: profile.clone(),
        dialect,
        headers,
        normalized_headers,
        rows: records,
        original_preserved: true,
        filesystem_effect_performed: false,
        network_access_performed: false,
        execution_performed: false,
    })
}

fn canonical_row(row: &[String]) -> Vec<u8> {
    serde_json::to_vec(row).unwrap_or_default()
}

/// Computes deterministic missing-value and duplicate-row summaries.
#[must_use]
pub fn summarize_tabular(document: &TabularDocument) -> TabularSummary {
    let mut missing_by_column = document
        .normalized_headers
        .iter()
        .map(|header| (header.clone(), 0_usize))
        .collect::<BTreeMap<_, _>>();
    let mut groups: BTreeMap<String, Vec<u32>> = BTreeMap::new();
    for (row_index, row) in document.rows.iter().enumerate() {
        for (column_index, value) in row.iter().enumerate() {
            if clean_tabular_text(value).is_empty() {
                *missing_by_column
                    .get_mut(&document.normalized_headers[column_index])
                    .expect("known header") += 1;
            }
        }
        groups
            .entry(word_sha256(&canonical_row(row)))
            .or_default()
            .push(u32::try_from(row_index + 1).unwrap_or(u32::MAX));
    }
    groups.retain(|_, rows| rows.len() > 1);
    TabularSummary {
        source_sha256: document.source_sha256.clone(),
        column_count: document.headers.len(),
        row_count: document.rows.len(),
        missing_by_column,
        duplicate_row_groups: groups,
    }
}

fn column_index(document: &TabularDocument, column: &str) -> Result<usize, TabularError> {
    document
        .normalized_headers
        .iter()
        .position(|header| header == column)
        .ok_or(TabularError::InvalidInput)
}

/// Applies closed filters and sort keys and returns source-row identities without copying values.
pub fn filter_sort_tabular(
    document: &TabularDocument,
    filters: &[TabularFilter],
    sorts: &[TabularSort],
) -> Result<Vec<TabularRowProjection>, TabularError> {
    let filter_indexes = filters
        .iter()
        .map(|filter| match filter {
            TabularFilter::Equals { column, .. }
            | TabularFilter::Contains { column, .. }
            | TabularFilter::Missing { column } => column_index(document, column),
        })
        .collect::<Result<Vec<_>, _>>()?;
    let sort_indexes = sorts
        .iter()
        .map(|sort| column_index(document, &sort.column))
        .collect::<Result<Vec<_>, _>>()?;
    let mut selected = document
        .rows
        .iter()
        .enumerate()
        .filter(|(_, row)| {
            filters.iter().zip(&filter_indexes).all(|(filter, index)| {
                let actual = normalized_tabular_text(&row[*index]);
                match filter {
                    TabularFilter::Equals { value, .. } => actual == normalized_tabular_text(value),
                    TabularFilter::Contains { value, .. } => {
                        actual.contains(&normalized_tabular_text(value))
                    }
                    TabularFilter::Missing { .. } => actual.is_empty(),
                }
            })
        })
        .collect::<Vec<_>>();
    selected.sort_by(|(left_index, left), (right_index, right)| {
        for (sort, column) in sorts.iter().zip(&sort_indexes) {
            let order = normalized_tabular_text(&left[*column])
                .cmp(&normalized_tabular_text(&right[*column]));
            let order = match sort.direction {
                TabularSortDirection::Ascending => order,
                TabularSortDirection::Descending => order.reverse(),
            };
            if !order.is_eq() {
                return order;
            }
        }
        left_index.cmp(right_index)
    });
    Ok(selected
        .into_iter()
        .map(|(index, row)| TabularRowProjection {
            source_row: u32::try_from(index + 1).unwrap_or(u32::MAX),
            row_sha256: word_sha256(&canonical_row(row)),
        })
        .collect())
}

type KeyedRows<'a> = BTreeMap<Vec<String>, Vec<(u32, &'a Vec<String>)>>;

fn keyed_rows<'a>(document: &'a TabularDocument, indexes: &[usize]) -> KeyedRows<'a> {
    let mut values: KeyedRows<'a> = BTreeMap::new();
    for (index, row) in document.rows.iter().enumerate() {
        let key = indexes
            .iter()
            .map(|column| normalized_tabular_text(&row[*column]))
            .collect::<Vec<_>>();
        values
            .entry(key)
            .or_default()
            .push((u32::try_from(index + 1).unwrap_or(u32::MAX), row));
    }
    values
}

/// Compares two compatible tables by exact normalized key columns and deterministic reason codes.
pub fn compare_tabular(
    left: &TabularDocument,
    right: &TabularDocument,
    key_columns: Vec<String>,
) -> Result<TabularComparison, TabularError> {
    if left.normalized_headers != right.normalized_headers || key_columns.is_empty() {
        return Err(TabularError::IncompatibleHeaders);
    }
    let mut unique = BTreeSet::new();
    let indexes = key_columns
        .iter()
        .map(|column| {
            if !unique.insert(column.as_str()) {
                return Err(TabularError::InvalidInput);
            }
            column_index(left, column)
        })
        .collect::<Result<Vec<_>, _>>()?;
    let left_rows = keyed_rows(left, &indexes);
    let right_rows = keyed_rows(right, &indexes);
    let keys = left_rows
        .keys()
        .chain(right_rows.keys())
        .cloned()
        .collect::<BTreeSet<_>>();
    let mut overlap_key_count = 0_usize;
    let mut matches = Vec::new();
    for key in keys {
        let left_group = left_rows.get(&key).map(Vec::as_slice).unwrap_or(&[]);
        let right_group = right_rows.get(&key).map(Vec::as_slice).unwrap_or(&[]);
        overlap_key_count += usize::from(!left_group.is_empty() && !right_group.is_empty());
        let (reason, differing_fields) = if left_group.len() > 1 {
            (TabularMatchReason::DuplicateLeft, Vec::new())
        } else if right_group.len() > 1 {
            (TabularMatchReason::DuplicateRight, Vec::new())
        } else if right_group.is_empty() {
            (TabularMatchReason::MissingRight, Vec::new())
        } else if left_group.is_empty() {
            (TabularMatchReason::MissingLeft, Vec::new())
        } else {
            let differing = left_group[0]
                .1
                .iter()
                .zip(right_group[0].1)
                .enumerate()
                .filter(|(_, (left_value, right_value))| {
                    normalized_tabular_text(left_value) != normalized_tabular_text(right_value)
                })
                .map(|(index, _)| left.normalized_headers[index].clone())
                .collect::<Vec<_>>();
            if differing.is_empty() {
                (TabularMatchReason::ExactRow, differing)
            } else {
                (TabularMatchReason::DifferingFields, differing)
            }
        };
        matches.push(TabularMatch {
            key_sha256: word_sha256(&serde_json::to_vec(&key).unwrap_or_default()),
            reason,
            left_rows: left_group.iter().map(|item| item.0).collect(),
            right_rows: right_group.iter().map(|item| item.0).collect(),
            differing_fields,
        });
    }
    Ok(TabularComparison {
        left_source_sha256: left.source_sha256.clone(),
        right_source_sha256: right.source_sha256.clone(),
        key_columns,
        matches,
        overlap_key_count,
        filesystem_effect_performed: false,
        network_access_performed: false,
        execution_performed: false,
    })
}

fn formula_risky(value: &str) -> bool {
    let trimmed = value
        .trim_start_matches(|character: char| character.is_whitespace() || character == '\u{feff}');
    value.starts_with(['\t', '\r', '\n'])
        || trimmed.starts_with(['=', '+', '-', '@'])
        || trimmed.to_ascii_lowercase().starts_with("dde")
}

fn safe_cell(value: &str) -> (String, bool) {
    if formula_risky(value) {
        (format!("'{value}"), true)
    } else {
        (value.to_owned(), false)
    }
}

fn write_csv_field(output: &mut Vec<u8>, value: &str) {
    let quote = value.contains([',', '"', '\r', '\n']);
    if quote {
        output.push(b'"');
    }
    for byte in value.bytes() {
        if byte == b'"' {
            output.extend_from_slice(b"\"\"");
        } else {
            output.push(byte);
        }
    }
    if quote {
        output.push(b'"');
    }
}

/// Builds comma-separated output while prefixing every formula-like header and cell.
pub fn build_safe_csv(document: &TabularDocument) -> Result<SafeCsvProposal, TabularError> {
    let mut output = Vec::new();
    let mut sanitized = 0_u64;
    for (row_index, row) in std::iter::once(&document.headers)
        .chain(document.rows.iter())
        .enumerate()
    {
        for (column_index, value) in row.iter().enumerate() {
            if column_index > 0 {
                output.push(b',');
            }
            let (safe, changed) = safe_cell(value);
            sanitized = sanitized.saturating_add(u64::from(changed));
            write_csv_field(&mut output, &safe);
        }
        if row_index < document.rows.len() {
            output.extend_from_slice(b"\r\n");
        }
        if output.len() > MAX_SOURCE_BYTES {
            return Err(TabularError::ResourceLimit);
        }
    }
    Ok(SafeCsvProposal {
        source_sha256: document.source_sha256.clone(),
        csv_sha256: word_sha256(&output),
        csv: output,
        formula_sanitized_cell_count: sanitized,
        formula_injection_prevented: true,
        filesystem_effect_performed: false,
        network_access_performed: false,
        execution_performed: false,
    })
}

#[cfg(test)]
mod tests {
    use agentmage_kernel_contracts::{WorkspaceId, WorkspacePath};

    use super::*;

    fn path(name: &str) -> WorkspacePath {
        WorkspacePath::new(WorkspaceId::from_raw("workspace-tabular"), ["data", name])
            .expect("path")
    }

    #[test]
    fn parses_rfc4180_shapes_and_computes_bounded_summary() {
        let bytes = b"ID,Name,Note\r\n1,Alice,\"line one\nline two\"\r\n2,Bob,\"said \"\"hello\"\"\"\r\n2,Bob,\"said \"\"hello\"\"\"\r\n";
        let document = parse_delimited_table(
            path("input.csv"),
            bytes,
            DelimitedDialect::Comma,
            &TabularProfile::default(),
        )
        .expect("parse");
        assert_eq!(document.rows.len(), 3);
        assert_eq!(document.rows[0][2], "line one\nline two");
        assert_eq!(document.rows[1][2], "said \"hello\"");
        assert_eq!(summarize_tabular(&document).duplicate_row_groups.len(), 1);
        assert!(document.original_preserved);
        assert!(!document.execution_performed);
    }

    #[test]
    fn rejects_malformed_duplicate_headers_shape_utf8_and_limits() {
        let profile = TabularProfile::default();
        for bytes in [
            b"A,A\n1,2\n".as_slice(),
            b"A,B\n1\n".as_slice(),
            b"A,B\n\"unterminated,2".as_slice(),
            b"A,B\n\"closed\"x,2\n".as_slice(),
            &[0xff, 0xfe],
        ] {
            assert!(
                parse_delimited_table(path("bad.csv"), bytes, DelimitedDialect::Comma, &profile)
                    .is_err()
            );
        }
        let mut tiny = profile;
        tiny.maximum_rows = 2;
        assert_eq!(
            parse_delimited_table(
                path("large.csv"),
                b"A\n1\n2\n",
                DelimitedDialect::Comma,
                &tiny
            ),
            Err(TabularError::ResourceLimit)
        );
    }

    #[test]
    fn filters_sorts_and_compares_with_stable_reason_codes() {
        let left = parse_delimited_table(
            path("left.csv"),
            b"ID,Name,State\n2,Bob,Open\n1,Alice,Closed\n3,,Open\n",
            DelimitedDialect::Comma,
            &TabularProfile::default(),
        )
        .expect("left");
        let selected = filter_sort_tabular(
            &left,
            &[TabularFilter::Equals {
                column: "state".to_owned(),
                value: "OPEN".to_owned(),
            }],
            &[TabularSort {
                column: "id".to_owned(),
                direction: TabularSortDirection::Ascending,
            }],
        )
        .expect("projection");
        assert_eq!(
            selected
                .iter()
                .map(|item| item.source_row)
                .collect::<Vec<_>>(),
            [1, 3]
        );
        let right = parse_delimited_table(
            path("right.csv"),
            b"ID,Name,State\n1,Alice,Closed\n2,Robert,Open\n4,Dana,Open\n",
            DelimitedDialect::Comma,
            &TabularProfile::default(),
        )
        .expect("right");
        let report = compare_tabular(&left, &right, vec!["id".to_owned()]).expect("compare");
        assert_eq!(report.overlap_key_count, 2);
        for reason in [
            TabularMatchReason::ExactRow,
            TabularMatchReason::DifferingFields,
            TabularMatchReason::MissingRight,
            TabularMatchReason::MissingLeft,
        ] {
            assert!(report.matches.iter().any(|item| item.reason == reason));
        }
    }

    #[test]
    fn safe_csv_prefixes_formula_dde_and_round_trips_as_text() {
        let document = parse_delimited_table(
            path("unsafe.csv"),
            b"ID,Value\n1,=2+3\n2, +SUM(A1:A2)\n3,@cmd\n4,DDE payload\n5,safe\n",
            DelimitedDialect::Comma,
            &TabularProfile::default(),
        )
        .expect("parse");
        let proposal = build_safe_csv(&document).expect("proposal");
        let output = String::from_utf8(proposal.csv.clone()).expect("utf8");
        for expected in ["'=2+3", "' +SUM", "'@cmd", "'DDE payload"] {
            assert!(output.contains(expected));
        }
        assert_eq!(proposal.formula_sanitized_cell_count, 4);
        let reopened = parse_delimited_table(
            path("safe.csv"),
            &proposal.csv,
            DelimitedDialect::Comma,
            &TabularProfile::default(),
        )
        .expect("reopen");
        assert!(reopened.rows.iter().all(|row| !formula_risky(&row[1])));
    }

    #[test]
    fn normalization_helpers_are_stable_and_path_free() {
        assert_eq!(normalized_tabular_text("  A\u{212b}  B "), "aå b");
        assert_eq!(normalized_tabular_id(" Report / 2026 "), "report-2026");
        assert_eq!(normalized_tabular_filename("../"), "untitled");
        assert_eq!(
            normalized_tabular_url(" HTTPS://EXAMPLE.COM/ "),
            "https://example.com"
        );
    }
}
