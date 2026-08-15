//! Deterministic reconciliation workbook proposals and pure native-evidence validation.

use std::collections::BTreeMap;

use agentmage_kernel_contracts::{CONTRACT_SCHEMA_VERSION, WorkspacePath};
use serde::{Deserialize, Serialize};

use crate::spreadsheet_ooxml::{
    SpreadsheetCellKind, SpreadsheetInspection, SpreadsheetProfile, inspect_xlsx,
};
use crate::tabular::{TabularComparison, TabularMatchReason};
use crate::word_generation::zip_parts;
use crate::word_ooxml::word_sha256;

const MAX_RECONCILIATION_ROWS: usize = 1_000_000;
const MAX_TITLE_BYTES: usize = 4_096;

type WorkbookParts = BTreeMap<String, Vec<u8>>;
type FormulaSources = Vec<(String, String, usize)>;

/// Closed generation profile for content-minimized reconciliation workbooks.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReconciliationWorkbookProfile {
    /// Stable profile identity.
    pub profile_id: String,
    /// Maximum comparison rows emitted into one workbook.
    pub maximum_rows: usize,
    /// Exact closed formula policy identity.
    pub formula_policy_id: String,
}

impl Default for ReconciliationWorkbookProfile {
    fn default() -> Self {
        Self {
            profile_id: "reconciliation-workbook-v1".to_owned(),
            maximum_rows: 100_000,
            formula_policy_id: "closed-summary-formulas-v1".to_owned(),
        }
    }
}

impl ReconciliationWorkbookProfile {
    fn valid(&self) -> bool {
        valid_identifier(&self.profile_id)
            && self.maximum_rows > 0
            && self.maximum_rows <= MAX_RECONCILIATION_ROWS
            && self.formula_policy_id == "closed-summary-formulas-v1"
    }
}

/// Exact request for a deterministic reconciliation workbook proposal.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReconciliationWorkbookRequest {
    /// Stable workbook identity.
    pub workbook_id: String,
    /// Canonical proposed output path ending in `.xlsx`.
    pub output_path: WorkspacePath,
    /// Human-facing title retained as inert inline text.
    pub title: String,
    /// Stable reconciliation method identity.
    pub method_id: String,
    /// Stable reconciliation method version.
    pub method_version: String,
}

/// One closed generated formula observation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReconciliationFormula {
    /// Formula cell in `Summary`.
    pub cell_reference: String,
    /// Exact formula text digest.
    pub formula_sha256: String,
    /// Exact cached value retained as text.
    pub cached_value: String,
}

/// Deterministic content-minimized XLSX proposal with bounded reopen evidence.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GeneratedReconciliationWorkbook {
    /// Kernel contract schema version.
    pub schema_version: u16,
    /// Stable workbook identity.
    pub workbook_id: String,
    /// Canonical proposed output path.
    pub output_path: WorkspacePath,
    /// Exact left source digest.
    pub left_source_sha256: String,
    /// Exact right source digest.
    pub right_source_sha256: String,
    /// Stable reconciliation method identity.
    pub method_id: String,
    /// Stable reconciliation method version.
    pub method_version: String,
    /// Exact generation profile.
    pub profile: ReconciliationWorkbookProfile,
    /// Exact XLSX bytes.
    pub xlsx: Vec<u8>,
    /// Exact XLSX digest.
    pub xlsx_sha256: String,
    /// Bounded direct reopen inspection.
    pub inspection: SpreadsheetInspection,
    /// Closed formulas and cached values.
    pub formulas: Vec<ReconciliationFormula>,
    /// Formula error cells found during reopen.
    pub formula_error_count: u64,
    /// Generated worksheet count.
    pub worksheet_count: u32,
    /// Generated structured table count.
    pub table_count: u32,
    /// Generated chart count.
    pub chart_count: u32,
    /// Generated data-validation count.
    pub data_validation_count: u32,
    /// True because both input sources remain unchanged.
    pub originals_preserved: bool,
    /// True because this result is a proposal and does not write the path.
    pub proposal_only: bool,
    /// False because generation is in memory.
    pub filesystem_effect_performed: bool,
    /// False because no network is used.
    pub network_access_performed: bool,
    /// False because no formula, macro, link, or embedded object is executed.
    pub execution_performed: bool,
}

/// Platform reported by an external native spreadsheet verifier.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpreadsheetVerificationPlatform {
    /// Fedora Linux.
    Fedora,
    /// Ubuntu Linux.
    Ubuntu,
    /// Windows 11 x64.
    Windows11X64,
    /// Apple Silicon macOS retained after first GA.
    MacosAppleSilicon,
}

/// Evidence class for a supplied spreadsheet verification observation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpreadsheetVerificationEvidenceKind {
    /// Synthetic fixture used only to test verifier semantics.
    SyntheticFixture,
    /// Observation from an installed native spreadsheet runtime.
    NativeInstalled,
}

/// Exact identity of an externally authorized spreadsheet verifier.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpreadsheetVerificationProfile {
    /// Stable profile identity.
    pub profile_id: String,
    /// Evidence class.
    pub evidence_kind: SpreadsheetVerificationEvidenceKind,
    /// Exact platform.
    pub platform: SpreadsheetVerificationPlatform,
    /// Spreadsheet application name.
    pub application_name: String,
    /// Exact application version.
    pub application_version: String,
    /// Digest of the verifier executable.
    pub executable_sha256: String,
    /// Digest of the installed package manifest.
    pub package_manifest_sha256: String,
    /// Digest of the font manifest used for rendering.
    pub font_manifest_sha256: String,
}

/// One externally observed recalculated formula.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpreadsheetFormulaObservation {
    /// Formula cell reference.
    pub cell_reference: String,
    /// Digest of exact formula text.
    pub formula_sha256: String,
    /// Digest of the recalculated value.
    pub recalculated_value_sha256: String,
    /// True when the native runtime reported a formula error.
    pub formula_error: bool,
}

/// Caller-supplied native reopen, recalculation, visual, and accessibility observation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpreadsheetVerificationObservation {
    /// Exact XLSX digest opened by the external verifier.
    pub source_sha256: String,
    /// Exact profile used by the external verifier.
    pub profile: SpreadsheetVerificationProfile,
    /// Canonically ordered formula observations.
    pub formulas: Vec<SpreadsheetFormulaObservation>,
    /// Number of structural reopen failures.
    pub structural_failure_count: u64,
    /// Number of clipping, overlap, truncation, or chart rendering failures.
    pub visual_failure_count: u64,
    /// Number of reading-order, table-header, contrast, or keyboard failures.
    pub accessibility_failure_count: u64,
    /// True only when the caller reports the complete approved campaign.
    pub observation_complete: bool,
    /// Always false; the core validator did not launch the application.
    pub execution_performed_by_core: bool,
    /// Always false; the core validator did not access the network.
    pub network_access_performed_by_core: bool,
    /// Always false; the core validator did not write a workbook.
    pub filesystem_effect_performed_by_core: bool,
}

/// Pure validation result for an externally supplied spreadsheet observation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpreadsheetVerificationReport {
    /// Kernel contract schema version.
    pub schema_version: u16,
    /// Exact generated workbook digest.
    pub source_sha256: String,
    /// Exact external verifier profile.
    pub profile: SpreadsheetVerificationProfile,
    /// True only when every expected formula is observed without errors.
    pub recalculation_checks_passed: bool,
    /// True only when structural, visual, and accessibility observations all pass.
    pub presentation_checks_passed: bool,
    /// True only for complete native installed evidence with all machine checks passed.
    pub machine_checks_passed: bool,
    /// True for synthetic or failed/incomplete evidence.
    pub human_review_required: bool,
    /// False because validation is pure.
    pub filesystem_effect_performed: bool,
    /// False because validation is pure.
    pub network_access_performed: bool,
    /// False because validation never launches the observed application.
    pub execution_performed: bool,
}

/// Stable reconciliation workbook failure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReconciliationWorkbookError {
    /// Request, comparison, profile, path, or verifier identity is invalid.
    InvalidInput,
    /// Comparison rows or generated package exceed the admitted limits.
    ResourceLimit,
    /// Generated Open XML could not be serialized or reopened safely.
    InvalidGeneratedPackage,
    /// A supplied verification observation is not bound to the exact generated artifact.
    VerificationMismatch,
}

impl ReconciliationWorkbookError {
    /// Stable content-free error code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidInput => "reconciliation.input.invalid",
            Self::ResourceLimit => "reconciliation.resource.limit",
            Self::InvalidGeneratedPackage => "reconciliation.package.invalid",
            Self::VerificationMismatch => "reconciliation.verification.mismatch",
        }
    }
}

impl std::fmt::Display for ReconciliationWorkbookError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.code())
    }
}

impl std::error::Error for ReconciliationWorkbookError {}

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

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}

fn safe_output_path(path: &WorkspacePath) -> bool {
    path.components()
        .last()
        .is_some_and(|item| item.as_str().ends_with(".xlsx") && item.as_str().len() > 5)
}

fn valid_text(value: &str) -> bool {
    !value.trim().is_empty()
        && value.len() <= MAX_TITLE_BYTES
        && value
            .chars()
            .all(|character| matches!(character, '\t' | '\n' | '\r') || !character.is_control())
}

fn xml_escape(value: &str) -> String {
    let mut output = String::with_capacity(value.len());
    for character in value.chars() {
        match character {
            '&' => output.push_str("&amp;"),
            '<' => output.push_str("&lt;"),
            '>' => output.push_str("&gt;"),
            '"' => output.push_str("&quot;"),
            '\'' => output.push_str("&apos;"),
            _ => output.push(character),
        }
    }
    output
}

fn inline_cell(reference: &str, value: &str, style: u32) -> String {
    format!(
        "<c r=\"{reference}\" t=\"inlineStr\" s=\"{style}\"><is><t xml:space=\"preserve\">{}</t></is></c>",
        xml_escape(value)
    )
}

fn number_cell(reference: &str, value: usize, style: u32) -> String {
    format!("<c r=\"{reference}\" s=\"{style}\"><v>{value}</v></c>")
}

fn formula_cell(reference: &str, formula: &str, cached: usize) -> String {
    format!(
        "<c r=\"{reference}\"><f>{}</f><v>{cached}</v></c>",
        xml_escape(formula)
    )
}

fn reason_text(reason: TabularMatchReason) -> &'static str {
    match reason {
        TabularMatchReason::ExactRow => "exact_row",
        TabularMatchReason::DifferingFields => "differing_fields",
        TabularMatchReason::MissingRight => "missing_right",
        TabularMatchReason::MissingLeft => "missing_left",
        TabularMatchReason::DuplicateLeft => "duplicate_left",
        TabularMatchReason::DuplicateRight => "duplicate_right",
    }
}

fn joined_rows(values: &[u32]) -> String {
    values
        .iter()
        .map(u32::to_string)
        .collect::<Vec<_>>()
        .join(";")
}

fn summary_sheet(
    request: &ReconciliationWorkbookRequest,
    comparison: &TabularComparison,
) -> (String, Vec<(String, String, usize)>) {
    let last_row = comparison.matches.len().saturating_add(1).max(2);
    let total_formula = format!("COUNTA(Reconciliation!A2:A{last_row})");
    let exact_formula = format!("COUNTIF(Reconciliation!B2:B{last_row},\"exact_row\")");
    let exact_count = comparison
        .matches
        .iter()
        .filter(|item| item.reason == TabularMatchReason::ExactRow)
        .count();
    let counts = [
        TabularMatchReason::ExactRow,
        TabularMatchReason::DifferingFields,
        TabularMatchReason::MissingRight,
        TabularMatchReason::MissingLeft,
        TabularMatchReason::DuplicateLeft,
        TabularMatchReason::DuplicateRight,
    ]
    .map(|reason| {
        (
            reason_text(reason),
            comparison
                .matches
                .iter()
                .filter(|item| item.reason == reason)
                .count(),
        )
    });
    let mut rows = vec![
        format!(
            "<row r=\"1\">{}</row>",
            inline_cell("A1", &request.title, 1)
        ),
        format!(
            "<row r=\"3\">{}{}</row>",
            inline_cell("A3", "Left source SHA-256", 1),
            inline_cell("B3", &comparison.left_source_sha256, 2)
        ),
        format!(
            "<row r=\"4\">{}{}</row>",
            inline_cell("A4", "Right source SHA-256", 1),
            inline_cell("B4", &comparison.right_source_sha256, 2)
        ),
        format!(
            "<row r=\"5\">{}{}</row>",
            inline_cell("A5", "Method", 1),
            inline_cell(
                "B5",
                &format!("{}@{}", request.method_id, request.method_version),
                0
            )
        ),
        format!(
            "<row r=\"7\">{}{}</row>",
            inline_cell("A7", "Total reconciliation rows", 1),
            formula_cell("B7", &total_formula, comparison.matches.len())
        ),
        format!(
            "<row r=\"8\">{}{}</row>",
            inline_cell("A8", "Exact rows", 1),
            formula_cell("B8", &exact_formula, exact_count)
        ),
        format!(
            "<row r=\"10\">{}{}</row>",
            inline_cell("A10", "Reason", 1),
            inline_cell("B10", "Count", 1)
        ),
    ];
    for (offset, (reason, count)) in counts.into_iter().enumerate() {
        let row = 11 + offset;
        rows.push(format!(
            "<row r=\"{row}\">{}{}</row>",
            inline_cell(&format!("A{row}"), reason, 0),
            number_cell(&format!("B{row}"), count, 0)
        ));
    }
    (
        format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?><worksheet xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\" xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\"><dimension ref=\"A1:B16\"/><cols><col min=\"1\" max=\"1\" width=\"30\" customWidth=\"1\"/><col min=\"2\" max=\"2\" width=\"68\" customWidth=\"1\"/></cols><sheetData>{}</sheetData><drawing r:id=\"rId1\"/></worksheet>",
            rows.join("")
        ),
        vec![
            ("B7".to_owned(), total_formula, comparison.matches.len()),
            ("B8".to_owned(), exact_formula, exact_count),
        ],
    )
}

fn reconciliation_sheet(comparison: &TabularComparison) -> String {
    let mut rows = vec![format!(
        "<row r=\"1\">{}{}{}{}{}{}</row>",
        inline_cell("A1", "Key SHA-256", 1),
        inline_cell("B1", "Reason", 1),
        inline_cell("C1", "Left rows", 1),
        inline_cell("D1", "Right rows", 1),
        inline_cell("E1", "Differing fields", 1),
        inline_cell("F1", "Evidence state", 1),
    )];
    for (index, item) in comparison.matches.iter().enumerate() {
        let row = index + 2;
        rows.push(format!(
            "<row r=\"{row}\">{}{}{}{}{}{}</row>",
            inline_cell(&format!("A{row}"), &item.key_sha256, 2),
            inline_cell(&format!("B{row}"), reason_text(item.reason), 0),
            inline_cell(&format!("C{row}"), &joined_rows(&item.left_rows), 0),
            inline_cell(&format!("D{row}"), &joined_rows(&item.right_rows), 0),
            inline_cell(&format!("E{row}"), &item.differing_fields.join(";"), 0),
            inline_cell(&format!("F{row}"), "observed", 0),
        ));
    }
    let last_row = comparison.matches.len().saturating_add(1).max(1);
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?><worksheet xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\" xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\"><dimension ref=\"A1:F{last_row}\"/><cols><col min=\"1\" max=\"1\" width=\"68\" customWidth=\"1\"/><col min=\"2\" max=\"6\" width=\"24\" customWidth=\"1\"/></cols><sheetData>{}</sheetData><dataValidations count=\"1\"><dataValidation type=\"list\" allowBlank=\"0\" sqref=\"B2:B1048576\"><formula1>&quot;exact_row,differing_fields,missing_right,missing_left,duplicate_left,duplicate_right&quot;</formula1></dataValidation></dataValidations><tableParts count=\"1\"><tablePart r:id=\"rId1\"/></tableParts></worksheet>",
        rows.join("")
    )
}

fn workbook_parts(
    request: &ReconciliationWorkbookRequest,
    comparison: &TabularComparison,
) -> (WorkbookParts, FormulaSources) {
    let (summary, formulas) = summary_sheet(request, comparison);
    let reconciliation = reconciliation_sheet(comparison);
    let last_row = comparison.matches.len().saturating_add(1).max(1);
    let parts = BTreeMap::from([
        ("[Content_Types].xml".to_owned(), b"<?xml version=\"1.0\" encoding=\"UTF-8\"?><Types xmlns=\"http://schemas.openxmlformats.org/package/2006/content-types\"><Default Extension=\"rels\" ContentType=\"application/vnd.openxmlformats-package.relationships+xml\"/><Default Extension=\"xml\" ContentType=\"application/xml\"/><Override PartName=\"/xl/workbook.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml\"/><Override PartName=\"/xl/worksheets/sheet1.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml\"/><Override PartName=\"/xl/worksheets/sheet2.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml\"/><Override PartName=\"/xl/styles.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.spreadsheetml.styles+xml\"/><Override PartName=\"/xl/tables/table1.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.spreadsheetml.table+xml\"/><Override PartName=\"/xl/drawings/drawing1.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.drawing+xml\"/><Override PartName=\"/xl/charts/chart1.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.drawingml.chart+xml\"/></Types>".to_vec()),
        ("_rels/.rels".to_owned(), b"<?xml version=\"1.0\" encoding=\"UTF-8\"?><Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\"><Relationship Id=\"rId1\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument\" Target=\"xl/workbook.xml\"/></Relationships>".to_vec()),
        ("xl/workbook.xml".to_owned(), b"<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?><workbook xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\" xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\"><workbookPr date1904=\"0\"/><sheets><sheet name=\"Summary\" sheetId=\"1\" r:id=\"rId1\"/><sheet name=\"Reconciliation\" sheetId=\"2\" r:id=\"rId2\"/></sheets><calcPr calcId=\"0\" calcMode=\"manual\" fullCalcOnLoad=\"0\" forceFullCalc=\"0\"/></workbook>".to_vec()),
        ("xl/_rels/workbook.xml.rels".to_owned(), b"<?xml version=\"1.0\" encoding=\"UTF-8\"?><Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\"><Relationship Id=\"rId1\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet\" Target=\"worksheets/sheet1.xml\"/><Relationship Id=\"rId2\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet\" Target=\"worksheets/sheet2.xml\"/><Relationship Id=\"rId3\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles\" Target=\"styles.xml\"/></Relationships>".to_vec()),
        ("xl/styles.xml".to_owned(), b"<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?><styleSheet xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\"><fonts count=\"2\"><font><sz val=\"11\"/><name val=\"Arial\"/></font><font><b/><sz val=\"11\"/><color rgb=\"FFFFFFFF\"/><name val=\"Arial\"/></font></fonts><fills count=\"3\"><fill><patternFill patternType=\"none\"/></fill><fill><patternFill patternType=\"gray125\"/></fill><fill><patternFill patternType=\"solid\"><fgColor rgb=\"FF1F4E78\"/><bgColor indexed=\"64\"/></patternFill></fill></fills><borders count=\"1\"><border><left/><right/><top/><bottom/><diagonal/></border></borders><cellStyleXfs count=\"1\"><xf numFmtId=\"0\" fontId=\"0\" fillId=\"0\" borderId=\"0\"/></cellStyleXfs><cellXfs count=\"3\"><xf numFmtId=\"0\" fontId=\"0\" fillId=\"0\" borderId=\"0\" xfId=\"0\"/><xf numFmtId=\"0\" fontId=\"1\" fillId=\"2\" borderId=\"0\" xfId=\"0\" applyFont=\"1\" applyFill=\"1\"/><xf numFmtId=\"49\" fontId=\"0\" fillId=\"0\" borderId=\"0\" xfId=\"0\" applyNumberFormat=\"1\"/></cellXfs></styleSheet>".to_vec()),
        ("xl/worksheets/sheet1.xml".to_owned(), summary.into_bytes()),
        ("xl/worksheets/_rels/sheet1.xml.rels".to_owned(), b"<?xml version=\"1.0\" encoding=\"UTF-8\"?><Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\"><Relationship Id=\"rId1\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/drawing\" Target=\"../drawings/drawing1.xml\"/></Relationships>".to_vec()),
        ("xl/worksheets/sheet2.xml".to_owned(), reconciliation.into_bytes()),
        ("xl/worksheets/_rels/sheet2.xml.rels".to_owned(), b"<?xml version=\"1.0\" encoding=\"UTF-8\"?><Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\"><Relationship Id=\"rId1\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/table\" Target=\"../tables/table1.xml\"/></Relationships>".to_vec()),
        ("xl/tables/table1.xml".to_owned(), format!("<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?><table xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\" id=\"1\" name=\"ReconciliationTable\" displayName=\"ReconciliationTable\" ref=\"A1:F{last_row}\" totalsRowShown=\"0\"><autoFilter ref=\"A1:F{last_row}\"/><tableColumns count=\"6\"><tableColumn id=\"1\" name=\"Key SHA-256\"/><tableColumn id=\"2\" name=\"Reason\"/><tableColumn id=\"3\" name=\"Left rows\"/><tableColumn id=\"4\" name=\"Right rows\"/><tableColumn id=\"5\" name=\"Differing fields\"/><tableColumn id=\"6\" name=\"Evidence state\"/></tableColumns><tableStyleInfo name=\"TableStyleMedium2\" showFirstColumn=\"0\" showLastColumn=\"0\" showRowStripes=\"1\" showColumnStripes=\"0\"/></table>").into_bytes()),
        ("xl/drawings/drawing1.xml".to_owned(), b"<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?><xdr:wsDr xmlns:xdr=\"http://schemas.openxmlformats.org/drawingml/2006/spreadsheetDrawing\" xmlns:a=\"http://schemas.openxmlformats.org/drawingml/2006/main\" xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\"><xdr:twoCellAnchor><xdr:from><xdr:col>3</xdr:col><xdr:colOff>0</xdr:colOff><xdr:row>1</xdr:row><xdr:rowOff>0</xdr:rowOff></xdr:from><xdr:to><xdr:col>10</xdr:col><xdr:colOff>0</xdr:colOff><xdr:row>18</xdr:row><xdr:rowOff>0</xdr:rowOff></xdr:to><xdr:graphicFrame macro=\"\"><xdr:nvGraphicFramePr><xdr:cNvPr id=\"2\" name=\"Reconciliation reason counts\"/><xdr:cNvGraphicFramePr/></xdr:nvGraphicFramePr><xdr:xfrm/><a:graphic><a:graphicData uri=\"http://schemas.openxmlformats.org/drawingml/2006/chart\"><c:chart xmlns:c=\"http://schemas.openxmlformats.org/drawingml/2006/chart\" r:id=\"rId1\"/></a:graphicData></a:graphic></xdr:graphicFrame><xdr:clientData/></xdr:twoCellAnchor></xdr:wsDr>".to_vec()),
        ("xl/drawings/_rels/drawing1.xml.rels".to_owned(), b"<?xml version=\"1.0\" encoding=\"UTF-8\"?><Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\"><Relationship Id=\"rId1\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/chart\" Target=\"../charts/chart1.xml\"/></Relationships>".to_vec()),
        ("xl/charts/chart1.xml".to_owned(), b"<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?><c:chartSpace xmlns:c=\"http://schemas.openxmlformats.org/drawingml/2006/chart\" xmlns:a=\"http://schemas.openxmlformats.org/drawingml/2006/main\"><c:chart><c:title><c:tx><c:rich><a:bodyPr/><a:lstStyle/><a:p><a:r><a:t>Reconciliation reason counts</a:t></a:r></a:p></c:rich></c:tx></c:title><c:plotArea><c:layout/><c:barChart><c:barDir val=\"col\"/><c:grouping val=\"clustered\"/><c:ser><c:idx val=\"0\"/><c:order val=\"0\"/><c:cat><c:strRef><c:f>Summary!$A$11:$A$16</c:f></c:strRef></c:cat><c:val><c:numRef><c:f>Summary!$B$11:$B$16</c:f></c:numRef></c:val></c:ser><c:axId val=\"1\"/><c:axId val=\"2\"/></c:barChart><c:catAx><c:axId val=\"1\"/><c:scaling><c:orientation val=\"minMax\"/></c:scaling><c:axPos val=\"b\"/><c:crossAx val=\"2\"/></c:catAx><c:valAx><c:axId val=\"2\"/><c:scaling><c:orientation val=\"minMax\"/></c:scaling><c:axPos val=\"l\"/><c:crossAx val=\"1\"/></c:valAx></c:plotArea><c:plotVisOnly val=\"1\"/></c:chart></c:chartSpace>".to_vec()),
    ]);
    (parts, formulas)
}

/// Generates a deterministic XLSX proposal, then reopens it through the bounded direct inspector.
pub fn generate_reconciliation_workbook(
    request: &ReconciliationWorkbookRequest,
    comparison: &TabularComparison,
    profile: &ReconciliationWorkbookProfile,
) -> Result<GeneratedReconciliationWorkbook, ReconciliationWorkbookError> {
    if !profile.valid()
        || !valid_identifier(&request.workbook_id)
        || !valid_identifier(&request.method_id)
        || !valid_identifier(&request.method_version)
        || !valid_text(&request.title)
        || !safe_output_path(&request.output_path)
        || !valid_sha256(&comparison.left_source_sha256)
        || !valid_sha256(&comparison.right_source_sha256)
    {
        return Err(ReconciliationWorkbookError::InvalidInput);
    }
    if comparison.matches.len() > profile.maximum_rows {
        return Err(ReconciliationWorkbookError::ResourceLimit);
    }
    let (parts, formula_source) = workbook_parts(request, comparison);
    let xlsx =
        zip_parts(&parts).map_err(|_| ReconciliationWorkbookError::InvalidGeneratedPackage)?;
    let inspection = inspect_xlsx(
        request.output_path.clone(),
        &xlsx,
        &SpreadsheetProfile::default(),
    )
    .map_err(|_| ReconciliationWorkbookError::InvalidGeneratedPackage)?;
    if !inspection.safe_for_analysis || !inspection.inspection_complete {
        return Err(ReconciliationWorkbookError::InvalidGeneratedPackage);
    }
    let formulas = formula_source
        .into_iter()
        .map(|(cell_reference, formula, cached)| ReconciliationFormula {
            cell_reference,
            formula_sha256: word_sha256(formula.as_bytes()),
            cached_value: cached.to_string(),
        })
        .collect::<Vec<_>>();
    let formula_error_count = inspection
        .worksheets
        .iter()
        .flat_map(|sheet| &sheet.cells)
        .filter(|cell| {
            cell.kind == SpreadsheetCellKind::Error
                || cell
                    .raw_value
                    .as_deref()
                    .is_some_and(|value| value.starts_with('#'))
        })
        .count() as u64;
    Ok(GeneratedReconciliationWorkbook {
        schema_version: CONTRACT_SCHEMA_VERSION,
        workbook_id: request.workbook_id.clone(),
        output_path: request.output_path.clone(),
        left_source_sha256: comparison.left_source_sha256.clone(),
        right_source_sha256: comparison.right_source_sha256.clone(),
        method_id: request.method_id.clone(),
        method_version: request.method_version.clone(),
        profile: profile.clone(),
        xlsx_sha256: word_sha256(&xlsx),
        xlsx,
        inspection,
        formulas,
        formula_error_count,
        worksheet_count: 2,
        table_count: 1,
        chart_count: 1,
        data_validation_count: 1,
        originals_preserved: true,
        proposal_only: true,
        filesystem_effect_performed: false,
        network_access_performed: false,
        execution_performed: false,
    })
}

/// Validates caller-supplied native spreadsheet evidence without launching or trusting the observed application implicitly.
pub fn verify_reconciliation_workbook(
    generated: &GeneratedReconciliationWorkbook,
    observation: &SpreadsheetVerificationObservation,
) -> Result<SpreadsheetVerificationReport, ReconciliationWorkbookError> {
    let profile = &observation.profile;
    if observation.source_sha256 != generated.xlsx_sha256
        || !valid_identifier(&profile.profile_id)
        || !valid_text(&profile.application_name)
        || !valid_text(&profile.application_version)
        || !valid_sha256(&profile.executable_sha256)
        || !valid_sha256(&profile.package_manifest_sha256)
        || !valid_sha256(&profile.font_manifest_sha256)
        || observation.execution_performed_by_core
        || observation.network_access_performed_by_core
        || observation.filesystem_effect_performed_by_core
    {
        return Err(ReconciliationWorkbookError::VerificationMismatch);
    }
    if !observation
        .formulas
        .windows(2)
        .all(|items| items[0].cell_reference < items[1].cell_reference)
        || observation.formulas.iter().any(|item| {
            !valid_sha256(&item.formula_sha256) || !valid_sha256(&item.recalculated_value_sha256)
        })
    {
        return Err(ReconciliationWorkbookError::VerificationMismatch);
    }
    let recalculation_checks_passed = generated.formulas.len() == observation.formulas.len()
        && generated
            .formulas
            .iter()
            .zip(&observation.formulas)
            .all(|(expected, actual)| {
                expected.cell_reference == actual.cell_reference
                    && expected.formula_sha256 == actual.formula_sha256
                    && !actual.formula_error
            });
    let presentation_checks_passed = observation.observation_complete
        && observation.structural_failure_count == 0
        && observation.visual_failure_count == 0
        && observation.accessibility_failure_count == 0;
    let machine_checks_passed = profile.evidence_kind
        == SpreadsheetVerificationEvidenceKind::NativeInstalled
        && recalculation_checks_passed
        && presentation_checks_passed;
    Ok(SpreadsheetVerificationReport {
        schema_version: CONTRACT_SCHEMA_VERSION,
        source_sha256: generated.xlsx_sha256.clone(),
        profile: profile.clone(),
        recalculation_checks_passed,
        presentation_checks_passed,
        machine_checks_passed,
        human_review_required: profile.evidence_kind
            == SpreadsheetVerificationEvidenceKind::SyntheticFixture
            || !machine_checks_passed,
        filesystem_effect_performed: false,
        network_access_performed: false,
        execution_performed: false,
    })
}

#[cfg(test)]
mod tests {
    use agentmage_kernel_contracts::{WorkspaceId, WorkspacePath};

    use super::*;
    use crate::tabular::{TabularMatch, TabularMatchReason};

    fn path() -> WorkspacePath {
        WorkspacePath::new(
            WorkspaceId::from_raw("workspace-reconcile"),
            ["reports", "reconciliation.xlsx"],
        )
        .expect("path")
    }

    fn comparison() -> TabularComparison {
        TabularComparison {
            schema_version: CONTRACT_SCHEMA_VERSION,
            left_source_sha256: "a".repeat(64),
            right_source_sha256: "b".repeat(64),
            key_columns: vec!["id".to_owned()],
            matches: vec![
                TabularMatch {
                    key_sha256: "c".repeat(64),
                    reason: TabularMatchReason::ExactRow,
                    left_rows: vec![1],
                    right_rows: vec![1],
                    differing_fields: Vec::new(),
                },
                TabularMatch {
                    key_sha256: "d".repeat(64),
                    reason: TabularMatchReason::DifferingFields,
                    left_rows: vec![2],
                    right_rows: vec![2],
                    differing_fields: vec!["amount".to_owned()],
                },
            ],
            overlap_key_count: 2,
            filesystem_effect_performed: false,
            network_access_performed: false,
            execution_performed: false,
        }
    }

    fn request(title: &str) -> ReconciliationWorkbookRequest {
        ReconciliationWorkbookRequest {
            workbook_id: "reconcile-1".to_owned(),
            output_path: path(),
            title: title.to_owned(),
            method_id: "exact-key-v1".to_owned(),
            method_version: "1.0.0".to_owned(),
        }
    }

    #[test]
    fn generates_deterministic_styled_table_chart_validation_and_closed_formulas() {
        let first = generate_reconciliation_workbook(
            &request("Reconciliation report"),
            &comparison(),
            &ReconciliationWorkbookProfile::default(),
        )
        .expect("generate");
        let second = generate_reconciliation_workbook(
            &request("Reconciliation report"),
            &comparison(),
            &ReconciliationWorkbookProfile::default(),
        )
        .expect("generate");
        assert_eq!(first.xlsx, second.xlsx);
        assert_eq!(first.inspection.worksheets.len(), 2);
        assert_eq!(first.formulas.len(), 2);
        assert_eq!(first.formula_error_count, 0);
        assert_eq!(
            (
                first.table_count,
                first.chart_count,
                first.data_validation_count
            ),
            (1, 1, 1)
        );
        assert!(first.inspection.safe_for_analysis);
        assert!(first.originals_preserved && first.proposal_only);
        assert!(!first.filesystem_effect_performed);
        assert!(!first.network_access_performed);
        assert!(!first.execution_performed);
    }

    #[test]
    fn caller_text_remains_inline_and_cannot_become_a_formula() {
        let generated = generate_reconciliation_workbook(
            &request("=HYPERLINK(\"https://example.invalid\") <report>"),
            &comparison(),
            &ReconciliationWorkbookProfile::default(),
        )
        .expect("generate");
        let summary = &generated.inspection.worksheets[0];
        let title = summary
            .cells
            .iter()
            .find(|cell| cell.address == "A1")
            .expect("title");
        assert_eq!(title.kind, SpreadsheetCellKind::InlineString);
        assert!(title.formula.is_none());
        assert_eq!(
            title.displayed_value.as_deref(),
            Some("=HYPERLINK(\"https://example.invalid\") <report>")
        );
    }

    #[test]
    fn invalid_paths_profiles_and_row_limits_fail_closed() {
        let mut invalid = request("Report");
        invalid.output_path = WorkspacePath::new(
            WorkspaceId::from_raw("workspace-reconcile"),
            ["reports", "report.xls"],
        )
        .expect("path");
        assert!(
            generate_reconciliation_workbook(
                &invalid,
                &comparison(),
                &ReconciliationWorkbookProfile::default()
            )
            .is_err()
        );
        let tiny = ReconciliationWorkbookProfile {
            maximum_rows: 1,
            ..ReconciliationWorkbookProfile::default()
        };
        assert_eq!(
            generate_reconciliation_workbook(&request("Report"), &comparison(), &tiny),
            Err(ReconciliationWorkbookError::ResourceLimit)
        );
    }

    #[test]
    fn synthetic_verification_can_test_semantics_but_never_close_native_evidence() {
        let generated = generate_reconciliation_workbook(
            &request("Report"),
            &comparison(),
            &ReconciliationWorkbookProfile::default(),
        )
        .expect("generate");
        let formulas = generated
            .formulas
            .iter()
            .map(|item| SpreadsheetFormulaObservation {
                cell_reference: item.cell_reference.clone(),
                formula_sha256: item.formula_sha256.clone(),
                recalculated_value_sha256: word_sha256(item.cached_value.as_bytes()),
                formula_error: false,
            })
            .collect();
        let report = verify_reconciliation_workbook(
            &generated,
            &SpreadsheetVerificationObservation {
                source_sha256: generated.xlsx_sha256.clone(),
                profile: SpreadsheetVerificationProfile {
                    profile_id: "synthetic-office-v1".to_owned(),
                    evidence_kind: SpreadsheetVerificationEvidenceKind::SyntheticFixture,
                    platform: SpreadsheetVerificationPlatform::Fedora,
                    application_name: "Synthetic fixture".to_owned(),
                    application_version: "1".to_owned(),
                    executable_sha256: "1".repeat(64),
                    package_manifest_sha256: "2".repeat(64),
                    font_manifest_sha256: "3".repeat(64),
                },
                formulas,
                structural_failure_count: 0,
                visual_failure_count: 0,
                accessibility_failure_count: 0,
                observation_complete: true,
                execution_performed_by_core: false,
                network_access_performed_by_core: false,
                filesystem_effect_performed_by_core: false,
            },
        )
        .expect("verify");
        assert!(report.recalculation_checks_passed);
        assert!(report.presentation_checks_passed);
        assert!(!report.machine_checks_passed);
        assert!(report.human_review_required);
        assert!(!report.execution_performed);
    }
}
