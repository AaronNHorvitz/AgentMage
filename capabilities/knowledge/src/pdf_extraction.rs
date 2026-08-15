//! Bounded PDF inspection, page text extraction, and exact-page citation contracts.

use agentmage_kernel_contracts::{CONTRACT_SCHEMA_VERSION, WorkspacePath};
use lopdf::{Document, LoadOptions};
use serde::{Deserialize, Serialize};

use crate::word_ooxml::word_sha256;

const EXTRACTOR_ID: &str = "agentmage-pdf-extractor-v1;lopdf=0.44.0;strict=true;password=denied;ocr=external-admitted-observation;renderer=none;network=denied;filesystem=denied";
const MAX_PROFILE_BYTES: usize = 512 * 1_024 * 1_024;
const MAX_PROFILE_OBJECTS: usize = 1_000_000;
const MAX_PROFILE_PAGES: usize = 100_000;

/// Exact resource profile for one untrusted PDF extraction.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PdfExtractionProfile {
    /// Stable profile identity.
    pub profile_id: String,
    /// Maximum source byte count accepted in memory.
    pub maximum_source_bytes: usize,
    /// Maximum parsed indirect-object count.
    pub maximum_objects: usize,
    /// Maximum logical page count.
    pub maximum_pages: usize,
    /// Maximum bytes produced by any decompressed stream.
    pub maximum_decompressed_stream_bytes: usize,
    /// Maximum extracted UTF-8 bytes retained for one page.
    pub maximum_page_text_bytes: usize,
    /// Maximum extracted UTF-8 bytes retained across the document.
    pub maximum_total_text_bytes: usize,
    /// Minimum non-whitespace character count used to recognize a text layer.
    pub minimum_text_characters: usize,
    /// Maximum image observations retained for one page.
    pub maximum_images_per_page: usize,
}

impl PdfExtractionProfile {
    /// Conservative cross-platform default for ordinary user-owned PDFs.
    #[must_use]
    pub fn strict_default() -> Self {
        Self {
            profile_id: "pdf-extraction-strict-v1".to_owned(),
            maximum_source_bytes: 64 * 1_024 * 1_024,
            maximum_objects: 250_000,
            maximum_pages: 10_000,
            maximum_decompressed_stream_bytes: 32 * 1_024 * 1_024,
            maximum_page_text_bytes: 8 * 1_024 * 1_024,
            maximum_total_text_bytes: 64 * 1_024 * 1_024,
            minimum_text_characters: 1,
            maximum_images_per_page: 4_096,
        }
    }

    fn valid(&self) -> bool {
        valid_identifier(&self.profile_id)
            && self.maximum_source_bytes > 0
            && self.maximum_source_bytes <= MAX_PROFILE_BYTES
            && self.maximum_objects > 0
            && self.maximum_objects <= MAX_PROFILE_OBJECTS
            && self.maximum_pages > 0
            && self.maximum_pages <= MAX_PROFILE_PAGES
            && self.maximum_decompressed_stream_bytes > 0
            && self.maximum_decompressed_stream_bytes <= MAX_PROFILE_BYTES
            && self.maximum_page_text_bytes > 0
            && self.maximum_page_text_bytes <= self.maximum_total_text_bytes
            && self.maximum_total_text_bytes <= MAX_PROFILE_BYTES
            && self.minimum_text_characters > 0
            && self.maximum_images_per_page > 0
            && self.maximum_images_per_page <= 100_000
    }
}

/// Closed extraction method retained with every fragment and citation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PdfExtractionMethod {
    /// Text decoded from the PDF's embedded text layer.
    EmbeddedText,
    /// Text supplied by a separately admitted local OCR observation.
    LocalOcr,
}

/// Closed state for one logical PDF page.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PdfPageState {
    /// A sufficient embedded text layer was extracted.
    Text,
    /// Image content was observed without a sufficient embedded text layer.
    ScannedCandidate,
    /// Neither sufficient text nor page images were observed.
    Empty,
    /// A parser, decoding, or declared resource limit prevented complete extraction.
    ExtractionLimited,
}

/// Stable identity for one logical page in one exact source artifact.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PdfPageIdentity {
    /// Digest of the exact source PDF bytes.
    pub source_sha256: String,
    /// One-based page number presented to the user.
    pub page_number: u32,
    /// PDF indirect-object number for the page dictionary.
    pub object_number: u32,
    /// PDF indirect-object generation for the page dictionary.
    pub object_generation: u16,
    /// Digest binding source, page number, and page object identity.
    pub page_id: String,
}

/// One source-grounded page citation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PdfPageCitation {
    /// Exact page identity.
    pub page: PdfPageIdentity,
    /// Stable fragment identity within the page.
    pub fragment_id: String,
    /// Exact digest of the cited extracted text.
    pub text_sha256: String,
    /// Method used to obtain the cited text.
    pub extraction_method: PdfExtractionMethod,
    /// Confidence in basis points; embedded decoded text is 10,000.
    pub confidence_basis_points: u16,
    /// True because the citation resolves to an exact logical page.
    pub exact_page: bool,
    /// False until an admitted region-aware extractor provides bounded coordinates.
    pub exact_region: bool,
}

/// One explicit extraction or fidelity limitation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PdfExtractionLimitation {
    /// Stable content-free limitation identity.
    pub limitation_id: String,
    /// Optional exact page identity.
    pub page_id: Option<String>,
    /// Stable content-free reason code.
    pub reason_code: String,
    /// Whether the limitation prevents a complete extraction claim.
    pub blocks_complete_extraction: bool,
    /// Whether admitted OCR could address this limitation.
    pub requires_ocr: bool,
    /// Always true; extracted text never replaces the original package.
    pub original_remains_authoritative: bool,
}

/// Extracted text and provenance for one logical PDF page.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PdfPageExtraction {
    /// Exact source and page identity.
    pub identity: PdfPageIdentity,
    /// Closed page extraction state.
    pub state: PdfPageState,
    /// Exact decoded page-content byte count when available within bounds.
    pub decoded_content_bytes: Option<u64>,
    /// Number of image XObjects observed without retaining image payloads.
    pub image_count: u32,
    /// Extracted embedded or admitted-OCR text.
    pub text: String,
    /// Digest of the exact UTF-8 text bytes.
    pub text_sha256: String,
    /// Extraction method when text is present.
    pub extraction_method: Option<PdfExtractionMethod>,
    /// Confidence in basis points when text is present.
    pub confidence_basis_points: Option<u16>,
    /// Exact-page citation when text is present.
    pub citation: Option<PdfPageCitation>,
    /// Stable limitation identifiers associated with this page.
    pub limitation_ids: Vec<String>,
}

/// Complete bounded PDF extraction result.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PdfExtractionResult {
    /// Kernel contract schema version.
    pub schema_version: u16,
    /// Canonical user-authorized source path.
    pub source_path: WorkspacePath,
    /// Digest of the exact source PDF bytes.
    pub source_sha256: String,
    /// Exact extractor identity digest.
    pub extractor_identity_sha256: String,
    /// Resource profile used for extraction.
    pub profile: PdfExtractionProfile,
    /// PDF version string reported by the parser.
    pub pdf_version: String,
    /// Number of parsed indirect objects.
    pub object_count: u64,
    /// Canonical pages ordered by one-based page number.
    pub pages: Vec<PdfPageExtraction>,
    /// Total UTF-8 bytes retained across page text.
    pub total_text_bytes: u64,
    /// Explicit document and page limitations.
    pub limitations: Vec<PdfExtractionLimitation>,
    /// True only when every page was parsed without a blocking limitation.
    pub extraction_complete: bool,
    /// True when one or more pages require separately admitted OCR.
    pub ocr_required: bool,
    /// False because this extractor never invokes OCR.
    pub ocr_performed: bool,
    /// Always true; the input artifact remains authoritative.
    pub original_preserved: bool,
    /// False because the function has no filesystem effect.
    pub filesystem_effect_performed: bool,
    /// False because the function has no network effect.
    pub network_access_performed: bool,
    /// False because the function launches no process.
    pub execution_performed: bool,
}

/// Exact identity of an approved local OCR package and model.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PdfOcrAdmission {
    /// Stable package name.
    pub package_name: String,
    /// Exact package version.
    pub package_version: String,
    /// Digest of the exact local executable or library artifact.
    pub package_sha256: String,
    /// Digest of the exact local OCR model artifact.
    pub model_sha256: String,
    /// SPDX license expression approved by policy.
    pub license_expression: String,
    /// Digest of the independently verified admission receipt.
    pub admission_receipt_sha256: String,
    /// True only when the trusted caller verified the admission receipt.
    pub admission_verified_by_caller: bool,
}

/// Untrusted output returned by one separately executed local OCR operation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PdfOcrObservation {
    /// Exact source digest supplied to OCR.
    pub source_sha256: String,
    /// Exact page identity supplied to OCR.
    pub page_id: String,
    /// One-based page number supplied to OCR.
    pub page_number: u32,
    /// OCR-produced text.
    pub text: String,
    /// OCR engine confidence in basis points.
    pub confidence_basis_points: u16,
    /// Digest of the raw OCR result envelope retained by the caller.
    pub observation_sha256: String,
}

/// Validated OCR projection that can replace one scan-candidate page in a proposal.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PdfOcrProjection {
    /// Exact admitted package identity.
    pub admission: PdfOcrAdmission,
    /// Exact OCR observation identity.
    pub observation_sha256: String,
    /// Replacement page retaining explicit OCR provenance and uncertainty.
    pub page: PdfPageExtraction,
    /// Stable warning that OCR text is probabilistic.
    pub uncertainty_reason_code: String,
    /// False because validation does not launch the OCR package.
    pub execution_performed: bool,
}

/// Stable fail-closed PDF extraction error.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PdfExtractionError {
    /// Identity, profile, source, or observation is invalid.
    InvalidInput,
    /// PDF syntax or required structure is malformed or truncated.
    MalformedDocument,
    /// The document is encrypted and password handling is outside this boundary.
    EncryptedDocument,
    /// A declared source, object, page, stream, or text ceiling was exceeded.
    ResourceLimit,
    /// OCR package admission, source binding, or page binding did not verify.
    OcrNotAdmitted,
}

impl PdfExtractionError {
    /// Stable content-free error code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidInput => "pdf_extraction.input.invalid",
            Self::MalformedDocument => "pdf_extraction.document.malformed",
            Self::EncryptedDocument => "pdf_extraction.document.encrypted",
            Self::ResourceLimit => "pdf_extraction.resource.limit",
            Self::OcrNotAdmitted => "pdf_extraction.ocr.not_admitted",
        }
    }
}

impl std::fmt::Display for PdfExtractionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.code())
    }
}

impl std::error::Error for PdfExtractionError {}

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
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn page_identity(source_sha256: &str, page_number: u32, object: (u32, u16)) -> PdfPageIdentity {
    let page_id = word_sha256(
        format!(
            "pdf-page-v1\n{source_sha256}\n{page_number}\n{}\n{}",
            object.0, object.1
        )
        .as_bytes(),
    );
    PdfPageIdentity {
        source_sha256: source_sha256.to_owned(),
        page_number,
        object_number: object.0,
        object_generation: object.1,
        page_id,
    }
}

fn limitation(
    page_id: Option<&str>,
    reason_code: &str,
    blocks_complete_extraction: bool,
    requires_ocr: bool,
) -> PdfExtractionLimitation {
    let identity = page_id.unwrap_or("document");
    PdfExtractionLimitation {
        limitation_id: format!(
            "pdf-limit:{}",
            word_sha256(format!("{identity}\n{reason_code}").as_bytes())
        ),
        page_id: page_id.map(str::to_owned),
        reason_code: reason_code.to_owned(),
        blocks_complete_extraction,
        requires_ocr,
        original_remains_authoritative: true,
    }
}

fn parser_error(error: &lopdf::Error) -> PdfExtractionError {
    match error {
        lopdf::Error::InvalidPassword
        | lopdf::Error::UnsupportedSecurityHandler(_)
        | lopdf::Error::Decryption(_) => PdfExtractionError::EncryptedDocument,
        lopdf::Error::Decompress(lopdf::DecompressError::MemoryLimitExceeded { .. }) => {
            PdfExtractionError::ResourceLimit
        }
        _ => PdfExtractionError::MalformedDocument,
    }
}

/// Returns the digest of the exact parser, version, and effect boundary.
#[must_use]
pub fn pdf_extractor_identity_sha256() -> String {
    word_sha256(EXTRACTOR_ID.as_bytes())
}

/// Extracts bounded page text and exact-page citations from in-memory PDF bytes.
pub fn extract_pdf_to_pages(
    source_path: &WorkspacePath,
    source: &[u8],
    profile: &PdfExtractionProfile,
) -> Result<PdfExtractionResult, PdfExtractionError> {
    if !profile.valid()
        || source.is_empty()
        || source.len() > profile.maximum_source_bytes
        || !source.starts_with(b"%PDF-")
    {
        return Err(PdfExtractionError::InvalidInput);
    }

    let document = Document::load_mem_with_options(
        source,
        LoadOptions {
            password: None,
            filter: None,
            strict: true,
            max_decompressed_size: Some(profile.maximum_decompressed_stream_bytes),
        },
    )
    .map_err(|error| parser_error(&error))?;

    if document.is_encrypted() || document.was_encrypted() {
        return Err(PdfExtractionError::EncryptedDocument);
    }
    if document.objects.len() > profile.maximum_objects {
        return Err(PdfExtractionError::ResourceLimit);
    }

    let page_objects = document.get_pages();
    if page_objects.is_empty() {
        return Err(PdfExtractionError::MalformedDocument);
    }
    if page_objects.len() > profile.maximum_pages {
        return Err(PdfExtractionError::ResourceLimit);
    }

    let source_sha256 = word_sha256(source);
    let mut pages = Vec::with_capacity(page_objects.len());
    let mut limitations = Vec::new();
    let mut total_text_bytes = 0_usize;

    for (expected_index, (page_number, object_id)) in page_objects.iter().enumerate() {
        if usize::try_from(*page_number).ok() != Some(expected_index + 1) {
            return Err(PdfExtractionError::MalformedDocument);
        }
        let identity = page_identity(&source_sha256, *page_number, *object_id);
        let mut page_limitations = Vec::new();

        let decoded_content = document
            .get_page_content_with_limit(*object_id, profile.maximum_decompressed_stream_bytes);
        let decoded_content_bytes = match decoded_content {
            Ok(ref content) => {
                Some(u64::try_from(content.len()).map_err(|_| PdfExtractionError::ResourceLimit)?)
            }
            Err(error) => {
                let reason = match parser_error(&error) {
                    PdfExtractionError::ResourceLimit => "pdf.page.content-resource-limit",
                    _ => "pdf.page.content-malformed",
                };
                let item = limitation(Some(&identity.page_id), reason, true, false);
                page_limitations.push(item.limitation_id.clone());
                limitations.push(item);
                None
            }
        };

        let image_count = match document.get_page_images(*object_id) {
            Ok(images) if images.len() <= profile.maximum_images_per_page => {
                u32::try_from(images.len()).map_err(|_| PdfExtractionError::ResourceLimit)?
            }
            Ok(_) => {
                let item = limitation(
                    Some(&identity.page_id),
                    "pdf.page.image-count-resource-limit",
                    true,
                    false,
                );
                page_limitations.push(item.limitation_id.clone());
                limitations.push(item);
                0
            }
            Err(_) => {
                let item = limitation(
                    Some(&identity.page_id),
                    "pdf.page.image-inventory-unavailable",
                    true,
                    false,
                );
                page_limitations.push(item.limitation_id.clone());
                limitations.push(item);
                0
            }
        };

        let extracted = document
            .extract_text_with_limit(&[*page_number], profile.maximum_decompressed_stream_bytes);
        let mut text = String::new();
        let mut state = PdfPageState::ExtractionLimited;
        let mut extraction_method = None;
        let mut confidence_basis_points = None;
        let mut citation = None;

        if decoded_content_bytes.is_some() {
            match extracted {
                Ok(candidate)
                    if candidate.len() <= profile.maximum_page_text_bytes
                        && total_text_bytes
                            .checked_add(candidate.len())
                            .is_some_and(|value| value <= profile.maximum_total_text_bytes) =>
                {
                    let visible_characters = candidate
                        .chars()
                        .filter(|item| !item.is_whitespace())
                        .count();
                    if visible_characters >= profile.minimum_text_characters {
                        total_text_bytes += candidate.len();
                        text = candidate;
                        state = PdfPageState::Text;
                        extraction_method = Some(PdfExtractionMethod::EmbeddedText);
                        confidence_basis_points = Some(10_000);
                        let fragment_id = format!("{}:fragment:1", identity.page_id);
                        citation = Some(PdfPageCitation {
                            page: identity.clone(),
                            fragment_id,
                            text_sha256: word_sha256(text.as_bytes()),
                            extraction_method: PdfExtractionMethod::EmbeddedText,
                            confidence_basis_points: 10_000,
                            exact_page: true,
                            exact_region: false,
                        });
                        let item = limitation(
                            Some(&identity.page_id),
                            "pdf.citation.region-unavailable",
                            false,
                            false,
                        );
                        page_limitations.push(item.limitation_id.clone());
                        limitations.push(item);
                    } else if image_count > 0 {
                        state = PdfPageState::ScannedCandidate;
                        let item = limitation(
                            Some(&identity.page_id),
                            "pdf.page.scanned-candidate-ocr-required",
                            true,
                            true,
                        );
                        page_limitations.push(item.limitation_id.clone());
                        limitations.push(item);
                    } else {
                        state = PdfPageState::Empty;
                    }
                }
                Ok(_) => {
                    let item = limitation(
                        Some(&identity.page_id),
                        "pdf.page.text-resource-limit",
                        true,
                        false,
                    );
                    page_limitations.push(item.limitation_id.clone());
                    limitations.push(item);
                }
                Err(error) => {
                    let reason = match parser_error(&error) {
                        PdfExtractionError::ResourceLimit => "pdf.page.text-resource-limit",
                        _ => "pdf.page.text-decoding-limited",
                    };
                    let item = limitation(Some(&identity.page_id), reason, true, false);
                    page_limitations.push(item.limitation_id.clone());
                    limitations.push(item);
                }
            }
        }

        pages.push(PdfPageExtraction {
            identity,
            state,
            decoded_content_bytes,
            image_count,
            text_sha256: word_sha256(text.as_bytes()),
            text,
            extraction_method,
            confidence_basis_points,
            citation,
            limitation_ids: page_limitations,
        });
    }

    let extraction_complete = limitations
        .iter()
        .all(|item| !item.blocks_complete_extraction);
    let ocr_required = limitations.iter().any(|item| item.requires_ocr);
    Ok(PdfExtractionResult {
        schema_version: CONTRACT_SCHEMA_VERSION,
        source_path: source_path.clone(),
        source_sha256,
        extractor_identity_sha256: pdf_extractor_identity_sha256(),
        profile: profile.clone(),
        pdf_version: document.version,
        object_count: u64::try_from(document.objects.len())
            .map_err(|_| PdfExtractionError::ResourceLimit)?,
        pages,
        total_text_bytes: u64::try_from(total_text_bytes)
            .map_err(|_| PdfExtractionError::ResourceLimit)?,
        limitations,
        extraction_complete,
        ocr_required,
        ocr_performed: false,
        original_preserved: true,
        filesystem_effect_performed: false,
        network_access_performed: false,
        execution_performed: false,
    })
}

/// Validates an externally executed local OCR observation against an admitted identity.
pub fn validate_pdf_ocr_observation(
    extraction: &PdfExtractionResult,
    admission: &PdfOcrAdmission,
    observation: &PdfOcrObservation,
) -> Result<PdfOcrProjection, PdfExtractionError> {
    if !admission.admission_verified_by_caller
        || !valid_identifier(&admission.package_name)
        || admission.package_version.is_empty()
        || admission.package_version.len() > 128
        || !admission.package_version.is_ascii()
        || admission.license_expression.is_empty()
        || admission.license_expression.len() > 256
        || !admission.license_expression.is_ascii()
        || !valid_sha256(&admission.package_sha256)
        || !valid_sha256(&admission.model_sha256)
        || !valid_sha256(&admission.admission_receipt_sha256)
        || !valid_sha256(&observation.source_sha256)
        || !valid_sha256(&observation.page_id)
        || !valid_sha256(&observation.observation_sha256)
        || observation.source_sha256 != extraction.source_sha256
        || observation.confidence_basis_points > 10_000
        || observation.text.is_empty()
        || observation.text.len() > extraction.profile.maximum_page_text_bytes
    {
        return Err(PdfExtractionError::OcrNotAdmitted);
    }
    let source_page = extraction
        .pages
        .iter()
        .find(|page| {
            page.identity.page_id == observation.page_id
                && page.identity.page_number == observation.page_number
        })
        .ok_or(PdfExtractionError::OcrNotAdmitted)?;
    if source_page.state != PdfPageState::ScannedCandidate {
        return Err(PdfExtractionError::OcrNotAdmitted);
    }

    let text_sha256 = word_sha256(observation.text.as_bytes());
    let fragment_id = format!("{}:ocr-fragment:1", source_page.identity.page_id);
    let uncertainty_id = format!(
        "pdf-limit:{}",
        word_sha256(format!("{}\npdf.ocr.probabilistic", source_page.identity.page_id).as_bytes())
    );
    let page = PdfPageExtraction {
        identity: source_page.identity.clone(),
        state: PdfPageState::Text,
        decoded_content_bytes: source_page.decoded_content_bytes,
        image_count: source_page.image_count,
        text: observation.text.clone(),
        text_sha256: text_sha256.clone(),
        extraction_method: Some(PdfExtractionMethod::LocalOcr),
        confidence_basis_points: Some(observation.confidence_basis_points),
        citation: Some(PdfPageCitation {
            page: source_page.identity.clone(),
            fragment_id,
            text_sha256,
            extraction_method: PdfExtractionMethod::LocalOcr,
            confidence_basis_points: observation.confidence_basis_points,
            exact_page: true,
            exact_region: false,
        }),
        limitation_ids: vec![uncertainty_id],
    };
    Ok(PdfOcrProjection {
        admission: admission.clone(),
        observation_sha256: observation.observation_sha256.clone(),
        page,
        uncertainty_reason_code: "pdf.ocr.probabilistic".to_owned(),
        execution_performed: false,
    })
}

#[cfg(test)]
mod tests {
    use agentmage_kernel_contracts::{WorkspaceId, WorkspacePath};
    use lopdf::content::{Content, Operation};
    use lopdf::encryption::{EncryptionState, EncryptionVersion, Permissions};
    use lopdf::{Document, Object, Stream, dictionary};

    use super::*;

    fn path() -> WorkspacePath {
        WorkspacePath::new(
            WorkspaceId::from_raw("workspace-pdf"),
            ["docs", "report.pdf"],
        )
        .expect("path")
    }

    fn pdf(texts: &[Option<&str>], image_pages: &[usize]) -> Vec<u8> {
        let mut document = Document::with_version("1.7");
        let pages_id = document.new_object_id();
        let font_id = document.add_object(dictionary! {
            "Type" => "Font",
            "Subtype" => "Type1",
            "BaseFont" => "Helvetica",
        });
        let mut page_ids = Vec::new();
        for (index, text) in texts.iter().enumerate() {
            let mut resources = dictionary! {
                "Font" => dictionary! { "F1" => font_id },
            };
            if image_pages.contains(&index) {
                let image_id = document.add_object(Stream::new(
                    dictionary! {
                        "Type" => "XObject",
                        "Subtype" => "Image",
                        "Width" => 1,
                        "Height" => 1,
                        "ColorSpace" => "DeviceGray",
                        "BitsPerComponent" => 8,
                    },
                    vec![0],
                ));
                resources.set("XObject", dictionary! { "Im1" => image_id });
            }
            let operations = text.map_or_else(Vec::new, |value| {
                vec![
                    Operation::new("BT", vec![]),
                    Operation::new("Tf", vec!["F1".into(), 12.into()]),
                    Operation::new("Td", vec![50.into(), 700.into()]),
                    Operation::new("Tj", vec![Object::string_literal(value)]),
                    Operation::new("ET", vec![]),
                ]
            });
            let content_id = document.add_object(Stream::new(
                dictionary! {},
                Content { operations }.encode().expect("content"),
            ));
            let page_id = document.add_object(dictionary! {
                "Type" => "Page",
                "Parent" => pages_id,
                "Contents" => content_id,
                "Resources" => resources,
                "MediaBox" => vec![0.into(), 0.into(), 612.into(), 792.into()],
            });
            page_ids.push(page_id);
        }
        document.objects.insert(
            pages_id,
            Object::Dictionary(dictionary! {
                "Type" => "Pages",
                "Kids" => page_ids.iter().copied().map(Object::Reference).collect::<Vec<_>>(),
                "Count" => i64::try_from(page_ids.len()).expect("page count"),
            }),
        );
        let catalog_id = document.add_object(dictionary! {
            "Type" => "Catalog",
            "Pages" => pages_id,
        });
        document.trailer.set("Root", catalog_id);
        let mut output = Vec::new();
        document.save_to(&mut output).expect("save");
        output
    }

    #[test]
    fn extracts_canonical_pages_and_exact_page_citations() {
        let source = pdf(&[Some("Page one"), Some("Page two")], &[]);
        let first = extract_pdf_to_pages(&path(), &source, &PdfExtractionProfile::strict_default())
            .expect("extract");
        let second =
            extract_pdf_to_pages(&path(), &source, &PdfExtractionProfile::strict_default())
                .expect("extract");
        assert_eq!(first, second);
        assert!(first.extraction_complete);
        assert_eq!(first.pages.len(), 2);
        assert_eq!(first.pages[0].identity.page_number, 1);
        assert!(first.pages[0].text.contains("Page one"));
        let citation = first.pages[1].citation.as_ref().expect("citation");
        assert!(citation.exact_page);
        assert!(!citation.exact_region);
        assert_eq!(citation.page.page_number, 2);
        assert_eq!(citation.text_sha256, first.pages[1].text_sha256);
        assert!(!first.filesystem_effect_performed);
        assert!(!first.network_access_performed);
        assert!(!first.execution_performed);
    }

    #[test]
    fn detects_scan_candidate_without_running_ocr() {
        let source = pdf(&[None], &[0]);
        let result =
            extract_pdf_to_pages(&path(), &source, &PdfExtractionProfile::strict_default())
                .expect("inspect");
        assert_eq!(result.pages[0].state, PdfPageState::ScannedCandidate);
        assert_eq!(result.pages[0].image_count, 1);
        assert!(result.ocr_required);
        assert!(!result.ocr_performed);
        assert!(!result.extraction_complete);
    }

    #[test]
    fn fails_closed_for_malformed_truncated_and_oversized_sources() {
        assert_eq!(
            extract_pdf_to_pages(
                &path(),
                b"%PDF-1.7\ntruncated",
                &PdfExtractionProfile::strict_default()
            )
            .expect_err("truncated"),
            PdfExtractionError::MalformedDocument
        );
        let source = pdf(&[Some("bounded")], &[]);
        let mut profile = PdfExtractionProfile::strict_default();
        profile.maximum_source_bytes = source.len() - 1;
        assert_eq!(
            extract_pdf_to_pages(&path(), &source, &profile).expect_err("source limit"),
            PdfExtractionError::InvalidInput
        );
        let mut profile = PdfExtractionProfile::strict_default();
        profile.maximum_objects = 1;
        assert_eq!(
            extract_pdf_to_pages(&path(), &source, &profile).expect_err("object limit"),
            PdfExtractionError::ResourceLimit
        );
    }

    #[test]
    fn marks_text_limit_without_partial_best_effort_text() {
        let source = pdf(&[Some("text larger than limit")], &[]);
        let mut profile = PdfExtractionProfile::strict_default();
        profile.maximum_page_text_bytes = 4;
        let result = extract_pdf_to_pages(&path(), &source, &profile).expect("bounded report");
        assert_eq!(result.pages[0].state, PdfPageState::ExtractionLimited);
        assert!(result.pages[0].text.is_empty());
        assert!(!result.extraction_complete);
        assert!(
            result
                .limitations
                .iter()
                .any(|item| item.reason_code == "pdf.page.text-resource-limit")
        );
    }

    #[test]
    fn refuses_encrypted_documents_even_with_an_empty_user_password() {
        let source = pdf(&[Some("private")], &[]);
        let mut document = Document::load_mem(&source).expect("load fixture");
        document.trailer.set(
            "ID",
            Object::Array(vec![
                Object::string_literal(vec![1_u8; 16]),
                Object::string_literal(vec![2_u8; 16]),
            ]),
        );
        let state = EncryptionState::try_from(EncryptionVersion::V2 {
            document: &document,
            owner_password: "owner",
            user_password: "",
            key_length: 128,
            permissions: Permissions::all(),
        })
        .expect("state");
        document.encrypt(&state).expect("encrypt");
        let mut encrypted = Vec::new();
        document.save_to(&mut encrypted).expect("save encrypted");
        assert_eq!(
            extract_pdf_to_pages(&path(), &encrypted, &PdfExtractionProfile::strict_default())
                .expect_err("encrypted"),
            PdfExtractionError::EncryptedDocument
        );
    }

    #[test]
    fn accepts_only_exact_admitted_ocr_observations_for_scan_pages() {
        let source = pdf(&[None], &[0]);
        let result =
            extract_pdf_to_pages(&path(), &source, &PdfExtractionProfile::strict_default())
                .expect("inspect");
        let admission = PdfOcrAdmission {
            package_name: "approved-ocr".to_owned(),
            package_version: "1.2.3".to_owned(),
            package_sha256: "a".repeat(64),
            model_sha256: "b".repeat(64),
            license_expression: "Apache-2.0".to_owned(),
            admission_receipt_sha256: "c".repeat(64),
            admission_verified_by_caller: true,
        };
        let observation = PdfOcrObservation {
            source_sha256: result.source_sha256.clone(),
            page_id: result.pages[0].identity.page_id.clone(),
            page_number: 1,
            text: "uncertain OCR text".to_owned(),
            confidence_basis_points: 8_250,
            observation_sha256: "d".repeat(64),
        };
        let projection = validate_pdf_ocr_observation(&result, &admission, &observation)
            .expect("admitted observation");
        assert_eq!(
            projection.page.extraction_method,
            Some(PdfExtractionMethod::LocalOcr)
        );
        assert_eq!(projection.page.confidence_basis_points, Some(8_250));
        assert_eq!(projection.uncertainty_reason_code, "pdf.ocr.probabilistic");
        assert!(!projection.execution_performed);

        let mut wrong = observation;
        wrong.page_number = 2;
        assert_eq!(
            validate_pdf_ocr_observation(&result, &admission, &wrong).expect_err("page binding"),
            PdfExtractionError::OcrNotAdmitted
        );
    }
}
