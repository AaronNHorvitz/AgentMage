//! Authority-free XLSX, CSV, and JSON projection into the shared structured-source contract.

use agentmage_kernel_contracts::{
    CONTRACT_SCHEMA_VERSION, CSV_MEDIA_TYPE, JSON_MEDIA_TYPE, StructuredSourceExtraction,
    StructuredSourceExtractionError, StructuredSourceExtractionRequest, StructuredSourceExtractor,
    StructuredSourceFormat, StructuredSourceProvenance, StructuredSourceSection,
    StructuredSourceSectionKind, StructuredSourceWarning, XLSX_MEDIA_TYPE,
};
use serde_json::Value;

use crate::json_data::{StructuredJsonError, StructuredJsonProfile, parse_structured_json};
use crate::spreadsheet_ooxml::{
    SpreadsheetCell, SpreadsheetError, SpreadsheetFindingKind, SpreadsheetProfile,
    SpreadsheetSheetState, inspect_xlsx,
};
use crate::tabular::{DelimitedDialect, TabularError, TabularProfile, parse_delimited_table};
use crate::word_ooxml::word_sha256;

const EXTRACTOR_ID: &str = "agentmage-spreadsheet-structured-source-v1;xlsx=direct-ooxml-v1;csv=rfc4180-v1;json=strict-v1;formula-evaluation=denied;network=denied;filesystem=denied;execution=denied";
const MAX_SECTIONS: u32 = 5_000_000;
const MAX_OUTPUT_BYTES: u64 = 256 * 1_024 * 1_024;

/// Authority-free implementation over already captured XLSX, CSV, and JSON bytes.
#[derive(Clone, Copy, Debug, Default)]
pub struct SpreadsheetStructuredSourceExtractor;

struct Projection<'a> {
    request: &'a StructuredSourceExtractionRequest,
    format: StructuredSourceFormat,
    sections: Vec<StructuredSourceSection>,
    warnings: Vec<StructuredSourceWarning>,
    output_bytes: u64,
}

impl<'a> Projection<'a> {
    fn new(request: &'a StructuredSourceExtractionRequest, format: StructuredSourceFormat) -> Self {
        Self {
            request,
            format,
            sections: Vec::new(),
            warnings: Vec::new(),
            output_bytes: 0,
        }
    }

    fn push(
        &mut self,
        kind: StructuredSourceSectionKind,
        parent_ordinal: Option<u32>,
        content: String,
        provenance: StructuredSourceProvenance,
    ) -> Result<u32, StructuredSourceExtractionError> {
        let bytes = u64::try_from(content.len())
            .map_err(|_| StructuredSourceExtractionError::ResourceLimit)?;
        let next_output = self
            .output_bytes
            .checked_add(bytes)
            .ok_or(StructuredSourceExtractionError::ResourceLimit)?;
        if self.sections.len() >= self.request.maximum_sections as usize
            || next_output > self.request.maximum_output_bytes
        {
            return Err(StructuredSourceExtractionError::ResourceLimit);
        }
        let ordinal = u32::try_from(self.sections.len())
            .map_err(|_| StructuredSourceExtractionError::ResourceLimit)?;
        self.sections.push(StructuredSourceSection {
            section_id: self.section_id(ordinal),
            parent_section_id: parent_ordinal.map(|parent| self.section_id(parent)),
            ordinal,
            kind,
            content,
            provenance,
        });
        self.output_bytes = next_output;
        Ok(ordinal)
    }

    fn section_id(&self, ordinal: u32) -> String {
        format!("{}-section-{ordinal:08}", self.request.source_id)
    }

    fn warn(&mut self, reason_code: &str, source_part: Option<&str>) {
        self.warnings.push(StructuredSourceWarning {
            warning_id: format!(
                "{}-warning-{:08}",
                self.request.source_id,
                self.warnings.len() + 1
            ),
            reason_code: reason_code.to_owned(),
            source_part: source_part.map(str::to_owned),
            original_remains_authoritative: true,
        });
    }

    fn finish(self, extractor_sha256: String) -> StructuredSourceExtraction {
        StructuredSourceExtraction {
            schema_version: CONTRACT_SCHEMA_VERSION,
            source_id: self.request.source_id.clone(),
            format: self.format,
            source_sha256: self.request.source_sha256.clone(),
            extractor_sha256,
            sections: self.sections,
            warnings: self.warnings,
            output_bytes: self.output_bytes,
            extraction_complete: true,
            original_preserved: true,
            filesystem_effect_performed: false,
            network_access_performed: false,
            execution_performed: false,
        }
    }
}

fn valid_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 255
        && value.is_ascii()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b':' | b'-'))
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn provenance(
    source_part: &str,
    structural_path: String,
    table: Option<u32>,
    row: Option<u32>,
    cell: Option<u32>,
    relationship_id: Option<String>,
) -> StructuredSourceProvenance {
    StructuredSourceProvenance {
        source_part: source_part.to_owned(),
        structural_path,
        start_byte: None,
        end_byte_exclusive: None,
        paragraph: None,
        run: None,
        table,
        row,
        cell,
        relationship_id,
        rendered_page: None,
    }
}

fn cell_content(cell: &SpreadsheetCell) -> Result<String, StructuredSourceExtractionError> {
    serde_json::to_string(&serde_json::json!({
        "address": cell.address,
        "cached_result": cell.raw_value,
        "cell_type": cell.kind,
        "date_value": cell.date_value,
        "displayed_value": cell.displayed_value,
        "formula": cell.formula,
        "number_format_id": cell.number_format_id,
        "style_index": cell.style_index,
    }))
    .map_err(|_| StructuredSourceExtractionError::Malformed)
}

fn map_spreadsheet_error(error: SpreadsheetError) -> StructuredSourceExtractionError {
    match error {
        SpreadsheetError::InvalidInput => StructuredSourceExtractionError::InvalidInput,
        SpreadsheetError::ResourceLimit => StructuredSourceExtractionError::ResourceLimit,
        SpreadsheetError::EncryptedPackage | SpreadsheetError::UnsafePackage => {
            StructuredSourceExtractionError::Quarantined
        }
        SpreadsheetError::MalformedPackage => StructuredSourceExtractionError::Malformed,
    }
}

fn map_tabular_error(error: TabularError) -> StructuredSourceExtractionError {
    match error {
        TabularError::ResourceLimit => StructuredSourceExtractionError::ResourceLimit,
        TabularError::MalformedDelimitedInput | TabularError::InvalidUtf8 => {
            StructuredSourceExtractionError::Malformed
        }
        TabularError::InvalidInput | TabularError::IncompatibleHeaders => {
            StructuredSourceExtractionError::InvalidInput
        }
    }
}

fn map_json_error(error: StructuredJsonError) -> StructuredSourceExtractionError {
    match error {
        StructuredJsonError::ResourceLimit => StructuredSourceExtractionError::ResourceLimit,
        StructuredJsonError::ProhibitedKey => StructuredSourceExtractionError::Quarantined,
        StructuredJsonError::MalformedJson => StructuredSourceExtractionError::Malformed,
        StructuredJsonError::InvalidInput | StructuredJsonError::MissingRedactionTarget => {
            StructuredSourceExtractionError::InvalidInput
        }
    }
}

fn project_xlsx<'a>(
    request: &'a StructuredSourceExtractionRequest,
    source: &[u8],
    cancelled: &mut dyn FnMut() -> bool,
) -> Result<Projection<'a>, StructuredSourceExtractionError> {
    let inspection = inspect_xlsx(
        request.source_path.clone(),
        source,
        &SpreadsheetProfile::default(),
    )
    .map_err(map_spreadsheet_error)?;
    let mut output = Projection::new(request, StructuredSourceFormat::Xlsx);
    let root = output.push(
        StructuredSourceSectionKind::Document,
        None,
        serde_json::to_string(&serde_json::json!({
            "date_system": inspection.date_system,
            "sheet_count": inspection.worksheets.len(),
        }))
        .map_err(|_| StructuredSourceExtractionError::Malformed)?,
        provenance(
            "xl/workbook.xml",
            "/workbook".to_owned(),
            None,
            None,
            None,
            None,
        ),
    )?;
    for (sheet_index, sheet) in inspection.worksheets.iter().enumerate() {
        if cancelled() {
            return Err(StructuredSourceExtractionError::Cancelled);
        }
        let sheet_number = u32::try_from(sheet_index + 1)
            .map_err(|_| StructuredSourceExtractionError::ResourceLimit)?;
        let sheet_root = output.push(
            StructuredSourceSectionKind::Table,
            Some(root),
            serde_json::to_string(&serde_json::json!({
                "dimension": sheet.dimension,
                "hidden_columns": sheet.hidden_columns,
                "hidden_rows": sheet.hidden_rows,
                "merged_ranges": sheet.merged_ranges,
                "name": sheet.name,
                "protected": sheet.protected,
                "sheet_id": sheet.sheet_id,
                "sparse_dimension": sheet.sparse_dimension,
                "state": sheet.state,
            }))
            .map_err(|_| StructuredSourceExtractionError::Malformed)?,
            provenance(
                &sheet.part_name,
                format!("/workbook/sheets/sheet[{sheet_number}]"),
                Some(sheet_number),
                None,
                None,
                None,
            ),
        )?;
        if sheet.state != SpreadsheetSheetState::Visible {
            let code = match sheet.state {
                SpreadsheetSheetState::Hidden => "spreadsheet.sheet.hidden-preserved",
                SpreadsheetSheetState::VeryHidden => "spreadsheet.sheet.very-hidden-preserved",
                SpreadsheetSheetState::Visible => unreachable!(),
            };
            output.warn(code, Some(&sheet.part_name));
        }
        let mut active_row = 0;
        let mut row_root = None;
        for cell in &sheet.cells {
            if cancelled() {
                return Err(StructuredSourceExtractionError::Cancelled);
            }
            if cell.row != active_row {
                active_row = cell.row;
                row_root = Some(output.push(
                    StructuredSourceSectionKind::TableRow,
                    Some(sheet_root),
                    String::new(),
                    provenance(
                        &sheet.part_name,
                        format!("/workbook/sheets/sheet[{sheet_number}]/row[{}]", cell.row),
                        Some(sheet_number),
                        Some(cell.row),
                        None,
                        None,
                    ),
                )?);
            }
            output.push(
                StructuredSourceSectionKind::TableCell,
                row_root,
                cell_content(cell)?,
                provenance(
                    &sheet.part_name,
                    format!(
                        "/workbook/sheets/sheet[{sheet_number}]/row[{}]/cell[{}]",
                        cell.row, cell.address
                    ),
                    Some(sheet_number),
                    Some(cell.row),
                    Some(cell.column),
                    cell.hyperlink
                        .as_ref()
                        .map(|link| link.target_sha256.clone()),
                ),
            )?;
            if cell.formula.is_some() {
                output.warn(
                    "spreadsheet.formula-preserved-not-calculated",
                    Some(&sheet.part_name),
                );
            }
        }
    }
    for finding in &inspection.findings {
        let code = match finding.kind {
            SpreadsheetFindingKind::MacroContent => "spreadsheet.macro-content-not-executed",
            SpreadsheetFindingKind::ExternalWorkbookReference => {
                "spreadsheet.external-workbook-reference-not-resolved"
            }
            SpreadsheetFindingKind::DdeFormula => "spreadsheet.dde-formula-not-executed",
            SpreadsheetFindingKind::ExternalHyperlink => {
                "spreadsheet.external-hyperlink-not-followed"
            }
            SpreadsheetFindingKind::UnsupportedCompression => {
                "spreadsheet.unsupported-compression-preserved"
            }
            SpreadsheetFindingKind::EmbeddedObject => "spreadsheet.embedded-object-not-opened",
            SpreadsheetFindingKind::ProtectedSheet => "spreadsheet.sheet-protection-preserved",
            SpreadsheetFindingKind::SparseRange => "spreadsheet.sparse-range-not-expanded",
            SpreadsheetFindingKind::UnsupportedFeature => {
                "spreadsheet.feature-preserved-not-interpreted"
            }
        };
        output.warn(code, finding.part_name.as_deref());
    }
    Ok(output)
}

fn project_csv<'a>(
    request: &'a StructuredSourceExtractionRequest,
    source: &[u8],
    cancelled: &mut dyn FnMut() -> bool,
) -> Result<Projection<'a>, StructuredSourceExtractionError> {
    let document = parse_delimited_table(
        request.source_path.clone(),
        source,
        DelimitedDialect::Comma,
        &TabularProfile::default(),
    )
    .map_err(map_tabular_error)?;
    let mut output = Projection::new(request, StructuredSourceFormat::Csv);
    let root = output.push(
        StructuredSourceSectionKind::Document,
        None,
        String::new(),
        provenance("csv", "/csv".to_owned(), None, None, None, None),
    )?;
    let table = output.push(
        StructuredSourceSectionKind::Table,
        Some(root),
        serde_json::to_string(&document.headers)
            .map_err(|_| StructuredSourceExtractionError::Malformed)?,
        provenance("csv", "/csv/table[1]".to_owned(), Some(1), None, None, None),
    )?;
    for (row_index, row) in document.rows.iter().enumerate() {
        if cancelled() {
            return Err(StructuredSourceExtractionError::Cancelled);
        }
        let row_number = u32::try_from(row_index + 2)
            .map_err(|_| StructuredSourceExtractionError::ResourceLimit)?;
        let row_root = output.push(
            StructuredSourceSectionKind::TableRow,
            Some(table),
            String::new(),
            provenance(
                "csv",
                format!("/csv/table[1]/row[{row_number}]"),
                Some(1),
                Some(row_number),
                None,
                None,
            ),
        )?;
        for (column_index, value) in row.iter().enumerate() {
            let column = u32::try_from(column_index + 1)
                .map_err(|_| StructuredSourceExtractionError::ResourceLimit)?;
            output.push(
                StructuredSourceSectionKind::TableCell,
                Some(row_root),
                value.clone(),
                provenance(
                    "csv",
                    format!("/csv/table[1]/row[{row_number}]/cell[{column}]"),
                    Some(1),
                    Some(row_number),
                    Some(column),
                    None,
                ),
            )?;
        }
    }
    Ok(output)
}

fn project_json_value(
    output: &mut Projection<'_>,
    value: &Value,
    pointer: &str,
    parent: u32,
    cancelled: &mut dyn FnMut() -> bool,
) -> Result<(), StructuredSourceExtractionError> {
    if cancelled() {
        return Err(StructuredSourceExtractionError::Cancelled);
    }
    match value {
        Value::Object(values) => {
            for (key, value) in values {
                let escaped = key.replace('~', "~0").replace('/', "~1");
                let child = format!("{pointer}/{escaped}");
                let ordinal = output.push(
                    StructuredSourceSectionKind::TableRow,
                    Some(parent),
                    key.clone(),
                    provenance("json", child.clone(), None, None, None, None),
                )?;
                project_json_value(output, value, &child, ordinal, cancelled)?;
            }
        }
        Value::Array(values) => {
            for (index, value) in values.iter().enumerate() {
                let child = format!("{pointer}/{index}");
                let ordinal = output.push(
                    StructuredSourceSectionKind::TableRow,
                    Some(parent),
                    index.to_string(),
                    provenance("json", child.clone(), None, None, None, None),
                )?;
                project_json_value(output, value, &child, ordinal, cancelled)?;
            }
        }
        _ => {
            output.push(
                StructuredSourceSectionKind::TableCell,
                Some(parent),
                serde_json::to_string(value)
                    .map_err(|_| StructuredSourceExtractionError::Malformed)?,
                provenance("json", pointer.to_owned(), None, None, None, None),
            )?;
        }
    }
    Ok(())
}

fn project_json<'a>(
    request: &'a StructuredSourceExtractionRequest,
    source: &[u8],
    cancelled: &mut dyn FnMut() -> bool,
) -> Result<Projection<'a>, StructuredSourceExtractionError> {
    let document = parse_structured_json(
        request.source_path.clone(),
        source,
        &StructuredJsonProfile::default(),
        None,
    )
    .map_err(map_json_error)?;
    let mut output = Projection::new(request, StructuredSourceFormat::Json);
    let root = output.push(
        StructuredSourceSectionKind::Document,
        None,
        String::new(),
        provenance("json", String::new(), None, None, None, None),
    )?;
    project_json_value(&mut output, &document.value, "", root, cancelled)?;
    Ok(output)
}

impl StructuredSourceExtractor for SpreadsheetStructuredSourceExtractor {
    fn extractor_sha256(&self) -> String {
        word_sha256(EXTRACTOR_ID.as_bytes())
    }

    fn extract(
        &self,
        request: &StructuredSourceExtractionRequest,
        source: &[u8],
        cancelled: &mut dyn FnMut() -> bool,
    ) -> Result<StructuredSourceExtraction, StructuredSourceExtractionError> {
        if !request.supported_version()
            || !valid_id(&request.source_id)
            || !valid_sha256(&request.source_sha256)
            || request.source_sha256 != word_sha256(source)
            || source.is_empty()
            || request.maximum_sections == 0
            || request.maximum_sections > MAX_SECTIONS
            || request.maximum_output_bytes == 0
            || request.maximum_output_bytes > MAX_OUTPUT_BYTES
        {
            return Err(StructuredSourceExtractionError::InvalidInput);
        }
        if cancelled() {
            return Err(StructuredSourceExtractionError::Cancelled);
        }
        let projection = match request.media_type.as_str() {
            XLSX_MEDIA_TYPE => project_xlsx(request, source, cancelled)?,
            CSV_MEDIA_TYPE => project_csv(request, source, cancelled)?,
            JSON_MEDIA_TYPE => project_json(request, source, cancelled)?,
            _ => return Err(StructuredSourceExtractionError::Unsupported),
        };
        Ok(projection.finish(self.extractor_sha256()))
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use agentmage_kernel_contracts::{WorkspaceId, WorkspacePath};

    use super::*;
    use crate::word_generation::zip_parts;

    fn request(
        media_type: &str,
        source: &[u8],
        maximum_sections: u32,
    ) -> StructuredSourceExtractionRequest {
        StructuredSourceExtractionRequest {
            schema_version: CONTRACT_SCHEMA_VERSION,
            source_id: "structured-data-fixture".to_owned(),
            media_type: media_type.to_owned(),
            source_sha256: word_sha256(source),
            source_path: WorkspacePath::new(
                WorkspaceId::from_raw("workspace-data"),
                ["sources", "fixture"],
            )
            .expect("path"),
            maximum_sections,
            maximum_output_bytes: 1_024 * 1_024,
        }
    }

    fn xlsx_fixture() -> Vec<u8> {
        let mut parts = BTreeMap::new();
        parts.insert("[Content_Types].xml".to_owned(), b"<?xml version=\"1.0\"?><Types xmlns=\"http://schemas.openxmlformats.org/package/2006/content-types\"><Override PartName=\"/xl/workbook.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml\"/></Types>".to_vec());
        parts.insert("xl/workbook.xml".to_owned(), b"<?xml version=\"1.0\"?><workbook xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\" xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\"><workbookPr date1904=\"0\"/><sheets><sheet name=\"Hidden Data\" sheetId=\"1\" state=\"veryHidden\" r:id=\"rId1\"/></sheets></workbook>".to_vec());
        parts.insert("xl/_rels/workbook.xml.rels".to_owned(), b"<?xml version=\"1.0\"?><Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\"><Relationship Id=\"rId1\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet\" Target=\"worksheets/sheet1.xml\"/></Relationships>".to_vec());
        parts.insert("xl/styles.xml".to_owned(), b"<?xml version=\"1.0\"?><styleSheet xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\"><cellXfs count=\"2\"><xf numFmtId=\"0\"/><xf numFmtId=\"14\"/></cellXfs></styleSheet>".to_vec());
        parts.insert("xl/worksheets/sheet1.xml".to_owned(), b"<?xml version=\"1.0\"?><worksheet xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\"><dimension ref=\"A1:XFD1048576\"/><sheetProtection sheet=\"1\"/><sheetData><row r=\"1\"><c r=\"A1\" s=\"1\"><v>46000</v></c><c r=\"B1\"><f>SUM(A1,1)</f><v>46001</v></c></row></sheetData></worksheet>".to_vec());
        parts.insert(
            "xl/embeddings/oleObject1.bin".to_owned(),
            b"inert embedded object".to_vec(),
        );
        zip_parts(&parts).expect("zip")
    }

    #[test]
    fn xlsx_projection_preserves_hidden_formula_cache_date_and_cell_provenance() {
        let source = xlsx_fixture();
        let result = SpreadsheetStructuredSourceExtractor
            .extract(
                &request(XLSX_MEDIA_TYPE, &source, 100),
                &source,
                &mut || false,
            )
            .expect("extract");
        assert_eq!(result.format, StructuredSourceFormat::Xlsx);
        let formula = result
            .sections
            .iter()
            .find(|section| section.provenance.structural_path.ends_with("/cell[B1]"))
            .expect("formula cell");
        assert!(formula.content.contains("SUM(A1,1)"));
        assert!(formula.content.contains("46001"));
        let date = result
            .sections
            .iter()
            .find(|section| section.provenance.structural_path.ends_with("/cell[A1]"))
            .expect("date cell");
        assert!(date.content.contains("date_value"));
        assert!(
            result.warnings.iter().any(|warning| {
                warning.reason_code == "spreadsheet.sheet.very-hidden-preserved"
            })
        );
        assert!(result.warnings.iter().any(|warning| {
            warning.reason_code == "spreadsheet.formula-preserved-not-calculated"
        }));
        for reason in [
            "spreadsheet.embedded-object-not-opened",
            "spreadsheet.sheet-protection-preserved",
            "spreadsheet.sparse-range-not-expanded",
        ] {
            assert!(
                result
                    .warnings
                    .iter()
                    .any(|warning| warning.reason_code == reason)
            );
        }
        assert!(!result.execution_performed);
    }

    #[test]
    fn csv_projection_preserves_exact_rows_cells_and_inert_formulas() {
        let source = b"name,amount\nAlice,7\nBob,=1+1\n";
        let result = SpreadsheetStructuredSourceExtractor
            .extract(&request(CSV_MEDIA_TYPE, source, 100), source, &mut || false)
            .expect("extract");
        assert_eq!(result.format, StructuredSourceFormat::Csv);
        assert!(result.sections.iter().any(|section| {
            section.content == "=1+1"
                && section.provenance.structural_path == "/csv/table[1]/row[3]/cell[2]"
        }));
        assert!(!result.execution_performed);
        assert!(!result.network_access_performed);
        assert!(!result.filesystem_effect_performed);
    }

    #[test]
    fn json_projection_preserves_pointer_provenance_and_scalar_types() {
        let source = br#"{"rows":[{"active":true,"amount":7}],"empty":null}"#;
        let result = SpreadsheetStructuredSourceExtractor
            .extract(&request(JSON_MEDIA_TYPE, source, 100), source, &mut || {
                false
            })
            .expect("extract");
        assert_eq!(result.format, StructuredSourceFormat::Json);
        assert!(result.sections.iter().any(|section| {
            section.content == "true" && section.provenance.structural_path == "/rows/0/active"
        }));
        assert!(result.sections.iter().any(|section| {
            section.content == "null" && section.provenance.structural_path == "/empty"
        }));
    }

    #[test]
    fn cancellation_and_output_bounds_fail_closed() {
        let source = b"name,amount\nAlice,7\n";
        assert_eq!(
            SpreadsheetStructuredSourceExtractor.extract(
                &request(CSV_MEDIA_TYPE, source, 100),
                source,
                &mut || true,
            ),
            Err(StructuredSourceExtractionError::Cancelled)
        );
        assert_eq!(
            SpreadsheetStructuredSourceExtractor.extract(
                &request(CSV_MEDIA_TYPE, source, 2),
                source,
                &mut || false,
            ),
            Err(StructuredSourceExtractionError::ResourceLimit)
        );
    }

    #[test]
    fn rejects_digest_mismatch_and_unknown_media_type() {
        let source = b"name\nAlice\n";
        let mut invalid = request(CSV_MEDIA_TYPE, source, 100);
        invalid.source_sha256 = "0".repeat(64);
        assert_eq!(
            SpreadsheetStructuredSourceExtractor.extract(&invalid, source, &mut || false),
            Err(StructuredSourceExtractionError::InvalidInput)
        );
        assert_eq!(
            SpreadsheetStructuredSourceExtractor.extract(
                &request("application/octet-stream", source, 100),
                source,
                &mut || false,
            ),
            Err(StructuredSourceExtractionError::Unsupported)
        );
    }
}
