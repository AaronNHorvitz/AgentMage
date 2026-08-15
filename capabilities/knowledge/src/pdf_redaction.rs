//! Regeneration-based redaction for AgentMage-generated PDF reports.

use std::collections::BTreeSet;

use agentmage_kernel_contracts::{CONTRACT_SCHEMA_VERSION, WorkspacePath};
use lopdf::{Document, Object};
use serde::{Deserialize, Serialize};

use crate::pdf_extraction::PdfExtractionProfile;
use crate::pdf_generation::{
    GeneratedPdfReport, PdfGenerationError, PdfReportBlock, PdfReportRequest, generate_pdf_report,
};
use crate::word_ooxml::word_sha256;

const REDACTOR_ID: &str = "agentmage-pdf-regeneration-redactor-v1;arbitrary-pdf=false;incremental-update=false;network=denied;filesystem=denied";
const MAX_TARGETS: usize = 256;
const MAX_TARGET_BYTES: usize = 4_096;

/// One exact, caller-authorized redaction target.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PdfRedactionTarget {
    /// Stable non-sensitive target identity.
    pub target_id: String,
    /// Exact case-sensitive text to remove from every supported report layer.
    pub exact_text: String,
}

/// Exact regeneration-based redaction request.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PdfRedactionRequest {
    /// Stable redaction operation identity.
    pub redaction_id: String,
    /// Exact source PDF digest.
    pub source_pdf_sha256: String,
    /// Distinct proposed output path.
    pub output_path: WorkspacePath,
    /// Ordered exact targets.
    pub targets: Vec<PdfRedactionTarget>,
}

/// One residue layer checked after regeneration.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PdfRedactionLayer {
    /// Escaped static HTML proposal bytes.
    HtmlBytes,
    /// Serialized PDF bytes.
    PdfBytes,
    /// Parsed PDF names, strings, dictionaries, arrays, and decoded streams.
    PdfObjects,
    /// Reopened document metadata.
    Metadata,
    /// Reopened form field names and value-presence state.
    Forms,
    /// Reopened extracted page text.
    ExtractedText,
    /// Cross-reference and end-of-file structure.
    IncrementalUpdates,
}

/// Content-free result for one checked residue layer.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PdfRedactionLayerCheck {
    /// Checked layer.
    pub layer: PdfRedactionLayer,
    /// Number of target occurrences found.
    pub residue_count: u32,
    /// True only when no target remains.
    pub passed: bool,
}

/// Verifiable receipt for one regeneration-based redaction.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PdfRedactionReceipt {
    /// Kernel contract schema version.
    pub schema_version: u16,
    /// Stable redaction operation identity.
    pub redaction_id: String,
    /// Exact source PDF digest.
    pub source_pdf_sha256: String,
    /// Exact output PDF digest.
    pub output_pdf_sha256: String,
    /// Exact redactor implementation identity.
    pub redactor_identity_sha256: String,
    /// Hashes of exact targets; raw targets are never retained in the receipt.
    pub target_sha256: Vec<String>,
    /// Complete canonical layer checks.
    pub layer_checks: Vec<PdfRedactionLayerCheck>,
    /// True only for exact regeneration from an AgentMage report specification.
    pub source_specification_reproduced: bool,
    /// True because no incremental update is appended.
    pub full_regeneration_performed: bool,
    /// True only when every supported residue layer passes.
    pub residue_scan_passed: bool,
    /// Always true because native visual review remains a separate required observation.
    pub human_visual_review_required: bool,
    /// False because the function returns proposal bytes without writing them.
    pub filesystem_effect_performed: bool,
    /// False because no remote resources are used.
    pub network_access_performed: bool,
    /// False because no renderer or document action is executed.
    pub execution_performed: bool,
}

/// Regenerated redacted report and its residue receipt.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RedactedPdfReport {
    /// Sanitized generated report proposal.
    pub report: GeneratedPdfReport,
    /// Content-free redaction receipt.
    pub receipt: PdfRedactionReceipt,
}

/// Stable fail-closed redaction error.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PdfRedactionError {
    /// The request identity, path, or target set is invalid.
    InvalidInput,
    /// Source bytes do not exactly reproduce from the supplied source specification.
    SourceBindingMismatch,
    /// The target occurs in an unsupported identity or output-path field.
    UnsupportedIdentityTarget,
    /// Regeneration failed.
    GenerationFailed,
    /// One or more target residues remain.
    ResidueDetected,
    /// Reopened object scanning failed.
    InspectionFailed,
}

impl PdfRedactionError {
    /// Stable content-free error code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidInput => "pdf_redaction.input.invalid",
            Self::SourceBindingMismatch => "pdf_redaction.source.binding_mismatch",
            Self::UnsupportedIdentityTarget => "pdf_redaction.target.unsupported_identity",
            Self::GenerationFailed => "pdf_redaction.generation.failed",
            Self::ResidueDetected => "pdf_redaction.residue.detected",
            Self::InspectionFailed => "pdf_redaction.inspection.failed",
        }
    }
}

impl std::fmt::Display for PdfRedactionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.code())
    }
}

impl std::error::Error for PdfRedactionError {}

fn valid_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value.is_ascii()
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

fn validate_request(request: &PdfRedactionRequest) -> Result<(), PdfRedactionError> {
    if !valid_identifier(&request.redaction_id)
        || !valid_sha256(&request.source_pdf_sha256)
        || request.targets.is_empty()
        || request.targets.len() > MAX_TARGETS
        || !request
            .output_path
            .components()
            .last()
            .is_some_and(|name| name.as_str().ends_with(".pdf") && name.as_str().len() > 4)
    {
        return Err(PdfRedactionError::InvalidInput);
    }
    let mut ids = BTreeSet::new();
    let mut texts = BTreeSet::new();
    for target in &request.targets {
        if !valid_identifier(&target.target_id)
            || !ids.insert(target.target_id.as_str())
            || target.exact_text.trim().is_empty()
            || target.exact_text.len() > MAX_TARGET_BYTES
            || !target.exact_text.is_ascii()
            || !texts.insert(target.exact_text.as_str())
        {
            return Err(PdfRedactionError::InvalidInput);
        }
    }
    Ok(())
}

fn replace_targets(value: &str, targets: &[PdfRedactionTarget]) -> String {
    targets.iter().fold(value.to_owned(), |current, target| {
        current.replace(&target.exact_text, "[REDACTED]")
    })
}

fn sanitized_field_name(value: &str, targets: &[PdfRedactionTarget]) -> String {
    if targets
        .iter()
        .any(|target| value.contains(&target.exact_text))
    {
        format!("redacted_{}", &word_sha256(value.as_bytes())[..16])
    } else {
        value.to_owned()
    }
}

fn sanitize_request(
    source: &PdfReportRequest,
    request: &PdfRedactionRequest,
) -> Result<PdfReportRequest, PdfRedactionError> {
    let output_path = request
        .output_path
        .components()
        .iter()
        .map(|component| component.as_str())
        .collect::<Vec<_>>()
        .join("/");
    let identity_values = [source.report_id.as_str(), output_path.as_str()];
    if request.targets.iter().any(|target| {
        identity_values
            .iter()
            .any(|value| value.contains(&target.exact_text))
    }) {
        return Err(PdfRedactionError::UnsupportedIdentityTarget);
    }
    let mut sanitized = source.clone();
    sanitized.output_path = request.output_path.clone();
    sanitized.metadata.title = replace_targets(&sanitized.metadata.title, &request.targets);
    sanitized.metadata.author = replace_targets(&sanitized.metadata.author, &request.targets);
    sanitized.metadata.subject = sanitized
        .metadata
        .subject
        .as_deref()
        .map(|value| replace_targets(value, &request.targets));
    sanitized.metadata.creation_date =
        replace_targets(&sanitized.metadata.creation_date, &request.targets);
    for block in &mut sanitized.blocks {
        match block {
            PdfReportBlock::Heading { text, .. }
            | PdfReportBlock::Paragraph { text }
            | PdfReportBlock::ListItem { text } => {
                *text = replace_targets(text, &request.targets);
            }
            PdfReportBlock::TableRow { cells, .. } => {
                for cell in cells {
                    *cell = replace_targets(cell, &request.targets);
                }
            }
            PdfReportBlock::InternalPageLink { label, .. } => {
                *label = replace_targets(label, &request.targets);
            }
            PdfReportBlock::TextFormField { field } => {
                field.name = sanitized_field_name(&field.name, &request.targets);
                field.label = replace_targets(&field.label, &request.targets);
                field.initial_value = field
                    .initial_value
                    .as_deref()
                    .map(|value| replace_targets(value, &request.targets));
            }
            PdfReportBlock::Spacer => {}
        }
    }
    Ok(sanitized)
}

fn count_bytes(haystack: &[u8], needle: &[u8]) -> u32 {
    if needle.is_empty() || needle.len() > haystack.len() {
        return 0;
    }
    let count = haystack
        .windows(needle.len())
        .filter(|window| *window == needle)
        .count();
    u32::try_from(count).unwrap_or(u32::MAX)
}

fn count_targets_bytes(bytes: &[u8], targets: &[PdfRedactionTarget]) -> u32 {
    targets.iter().fold(0_u32, |total, target| {
        total.saturating_add(count_bytes(bytes, target.exact_text.as_bytes()))
    })
}

fn object_residue(object: &Object, targets: &[PdfRedactionTarget]) -> u32 {
    match object {
        Object::String(bytes, _) | Object::Name(bytes) => count_targets_bytes(bytes, targets),
        Object::Array(items) => items.iter().fold(0_u32, |total, item| {
            total.saturating_add(object_residue(item, targets))
        }),
        Object::Dictionary(dictionary) => dictionary.iter().fold(0_u32, |total, (key, value)| {
            total
                .saturating_add(count_targets_bytes(key, targets))
                .saturating_add(object_residue(value, targets))
        }),
        Object::Stream(stream) => {
            let dictionary_count = stream.dict.iter().fold(0_u32, |total, (key, value)| {
                total
                    .saturating_add(count_targets_bytes(key, targets))
                    .saturating_add(object_residue(value, targets))
            });
            let content = stream
                .decompressed_content()
                .unwrap_or_else(|_| stream.content.clone());
            dictionary_count.saturating_add(count_targets_bytes(&content, targets))
        }
        Object::Null
        | Object::Boolean(_)
        | Object::Integer(_)
        | Object::Real(_)
        | Object::Reference(_) => 0,
    }
}

fn parsed_object_residue(
    bytes: &[u8],
    targets: &[PdfRedactionTarget],
) -> Result<u32, PdfRedactionError> {
    let document = Document::load_mem(bytes).map_err(|_| PdfRedactionError::InspectionFailed)?;
    Ok(document.objects.values().fold(0_u32, |total, object| {
        total.saturating_add(object_residue(object, targets))
    }))
}

fn text_residue(values: impl Iterator<Item = String>, targets: &[PdfRedactionTarget]) -> u32 {
    values.fold(0_u32, |total, value| {
        total.saturating_add(count_targets_bytes(value.as_bytes(), targets))
    })
}

fn layer_check(layer: PdfRedactionLayer, residue_count: u32) -> PdfRedactionLayerCheck {
    PdfRedactionLayerCheck {
        layer,
        residue_count,
        passed: residue_count == 0,
    }
}

/// Returns the exact regeneration redactor identity digest.
#[must_use]
pub fn pdf_redactor_identity_sha256() -> String {
    word_sha256(REDACTOR_ID.as_bytes())
}

/// Regenerates an AgentMage-authored report without exact targets and scans all supported layers.
pub fn redact_generated_pdf_report(
    source_report: &GeneratedPdfReport,
    source_request: &PdfReportRequest,
    request: &PdfRedactionRequest,
    inspection_profile: &PdfExtractionProfile,
) -> Result<RedactedPdfReport, PdfRedactionError> {
    validate_request(request)?;
    if request.source_pdf_sha256 != source_report.pdf_sha256
        || source_report.report_id != source_request.report_id
    {
        return Err(PdfRedactionError::SourceBindingMismatch);
    }
    let reproduced = generate_pdf_report(source_request, inspection_profile)
        .map_err(|_| PdfRedactionError::GenerationFailed)?;
    if reproduced.pdf_sha256 != source_report.pdf_sha256
        || reproduced.html_sha256 != source_report.html_sha256
    {
        return Err(PdfRedactionError::SourceBindingMismatch);
    }
    let sanitized = sanitize_request(source_request, request)?;
    let report = generate_pdf_report(&sanitized, inspection_profile)
        .map_err(|_| PdfRedactionError::GenerationFailed)?;

    let metadata_values = [
        report.inspection.metadata.title.clone(),
        report.inspection.metadata.author.clone(),
        report.inspection.metadata.subject.clone(),
        report.inspection.metadata.keywords.clone(),
        report.inspection.metadata.creator.clone(),
        report.inspection.metadata.producer.clone(),
        report.inspection.metadata.creation_date.clone(),
        report.inspection.metadata.modification_date.clone(),
    ];
    let metadata_residue = text_residue(metadata_values.into_iter().flatten(), &request.targets);
    let forms_residue = text_residue(
        report.inspection.forms.iter().flat_map(|form| {
            [form.field_name.clone(), Some(form.field_type.clone())]
                .into_iter()
                .flatten()
        }),
        &request.targets,
    );
    let extracted_residue = report
        .inspection
        .extraction
        .as_ref()
        .map_or(0, |extraction| {
            text_residue(
                extraction.pages.iter().map(|page| page.text.clone()),
                &request.targets,
            )
        });
    let start_xref_count = count_bytes(&report.pdf, b"startxref");
    let eof_count = count_bytes(&report.pdf, b"%%EOF");
    let incremental_residue = u32::from(start_xref_count != 1 || eof_count != 1);
    let layer_checks = vec![
        layer_check(
            PdfRedactionLayer::HtmlBytes,
            count_targets_bytes(&report.html, &request.targets),
        ),
        layer_check(
            PdfRedactionLayer::PdfBytes,
            count_targets_bytes(&report.pdf, &request.targets),
        ),
        layer_check(
            PdfRedactionLayer::PdfObjects,
            parsed_object_residue(&report.pdf, &request.targets)?,
        ),
        layer_check(PdfRedactionLayer::Metadata, metadata_residue),
        layer_check(PdfRedactionLayer::Forms, forms_residue),
        layer_check(PdfRedactionLayer::ExtractedText, extracted_residue),
        layer_check(PdfRedactionLayer::IncrementalUpdates, incremental_residue),
    ];
    if layer_checks.iter().any(|check| !check.passed) {
        return Err(PdfRedactionError::ResidueDetected);
    }
    let mut target_sha256 = request
        .targets
        .iter()
        .map(|target| word_sha256(target.exact_text.as_bytes()))
        .collect::<Vec<_>>();
    target_sha256.sort();
    let receipt = PdfRedactionReceipt {
        schema_version: CONTRACT_SCHEMA_VERSION,
        redaction_id: request.redaction_id.clone(),
        source_pdf_sha256: source_report.pdf_sha256.clone(),
        output_pdf_sha256: report.pdf_sha256.clone(),
        redactor_identity_sha256: pdf_redactor_identity_sha256(),
        target_sha256,
        layer_checks,
        source_specification_reproduced: true,
        full_regeneration_performed: true,
        residue_scan_passed: true,
        human_visual_review_required: true,
        filesystem_effect_performed: false,
        network_access_performed: false,
        execution_performed: false,
    };
    Ok(RedactedPdfReport { report, receipt })
}

impl From<PdfGenerationError> for PdfRedactionError {
    fn from(_error: PdfGenerationError) -> Self {
        Self::GenerationFailed
    }
}

#[cfg(test)]
mod tests {
    use agentmage_kernel_contracts::{WorkspaceId, WorkspacePath};

    use super::*;
    use crate::{PdfPageSettings, PdfReportFormField, PdfReportMetadata};

    fn path(name: &str) -> WorkspacePath {
        WorkspacePath::new(
            WorkspaceId::from_raw("workspace-pdf-redaction"),
            ["reports", name],
        )
        .expect("path")
    }

    fn source_request() -> PdfReportRequest {
        PdfReportRequest {
            report_id: "redaction-source".to_owned(),
            output_path: path("source.pdf"),
            metadata: PdfReportMetadata {
                title: "Secret Delta Review".to_owned(),
                author: "Secret Delta Owner".to_owned(),
                subject: Some("Secret Delta subject".to_owned()),
                creation_date: "D:20260815000000Z".to_owned(),
            },
            page_settings: PdfPageSettings::letter_default(),
            blocks: vec![
                PdfReportBlock::Paragraph {
                    text: "Remove Secret Delta everywhere.".to_owned(),
                },
                PdfReportBlock::TableRow {
                    cells: vec!["Key".to_owned(), "Secret Delta".to_owned()],
                    header: true,
                },
                PdfReportBlock::TextFormField {
                    field: PdfReportFormField {
                        name: "SecretDeltaOwner".to_owned(),
                        label: "Secret Delta owner".to_owned(),
                        initial_value: Some("Secret Delta".to_owned()),
                    },
                },
            ],
            source_markdown_sha256: None,
        }
    }

    #[test]
    fn regenerates_and_proves_exact_target_absence_across_supported_layers() {
        let profile = PdfExtractionProfile::strict_default();
        let source_request = source_request();
        let source = generate_pdf_report(&source_request, &profile).expect("source");
        let request = PdfRedactionRequest {
            redaction_id: "redaction-001".to_owned(),
            source_pdf_sha256: source.pdf_sha256.clone(),
            output_path: path("redacted.pdf"),
            targets: vec![PdfRedactionTarget {
                target_id: "secret-delta".to_owned(),
                exact_text: "Secret".to_owned(),
            }],
        };
        let redacted = redact_generated_pdf_report(&source, &source_request, &request, &profile)
            .expect("redact");
        assert!(redacted.receipt.residue_scan_passed);
        assert!(
            redacted
                .receipt
                .layer_checks
                .iter()
                .all(|check| check.passed)
        );
        assert!(
            !redacted
                .report
                .html
                .windows(6)
                .any(|window| window == b"Secret")
        );
        assert!(
            !redacted
                .report
                .pdf
                .windows(6)
                .any(|window| window == b"Secret")
        );
        assert_eq!(
            redacted.report.inspection.metadata.title.as_deref(),
            Some("[REDACTED] Delta Review")
        );
        assert!(
            redacted.report.inspection.forms[0]
                .field_name
                .as_deref()
                .expect("field name")
                .starts_with("redacted_")
        );
    }

    #[test]
    fn refuses_mismatched_source_bytes_and_identity_targets() {
        let profile = PdfExtractionProfile::strict_default();
        let source_request = source_request();
        let source = generate_pdf_report(&source_request, &profile).expect("source");
        let mut request = PdfRedactionRequest {
            redaction_id: "redaction-002".to_owned(),
            source_pdf_sha256: "a".repeat(64),
            output_path: path("redacted.pdf"),
            targets: vec![PdfRedactionTarget {
                target_id: "secret".to_owned(),
                exact_text: "Secret Delta".to_owned(),
            }],
        };
        assert_eq!(
            redact_generated_pdf_report(&source, &source_request, &request, &profile)
                .expect_err("binding"),
            PdfRedactionError::SourceBindingMismatch
        );
        request.source_pdf_sha256 = source.pdf_sha256.clone();
        request.targets[0].exact_text = "redaction-source".to_owned();
        assert_eq!(
            redact_generated_pdf_report(&source, &source_request, &request, &profile)
                .expect_err("identity"),
            PdfRedactionError::UnsupportedIdentityTarget
        );
    }

    #[test]
    fn rejects_duplicate_empty_and_non_ascii_targets() {
        let request = PdfRedactionRequest {
            redaction_id: "redaction-003".to_owned(),
            source_pdf_sha256: "b".repeat(64),
            output_path: path("redacted.pdf"),
            targets: vec![
                PdfRedactionTarget {
                    target_id: "one".to_owned(),
                    exact_text: "".to_owned(),
                },
                PdfRedactionTarget {
                    target_id: "one".to_owned(),
                    exact_text: "snowman \u{2603}".to_owned(),
                },
            ],
        };
        assert_eq!(
            validate_request(&request),
            Err(PdfRedactionError::InvalidInput)
        );
    }
}
