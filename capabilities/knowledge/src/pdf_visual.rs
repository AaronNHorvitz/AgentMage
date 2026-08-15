//! Effect-free comparison of externally rendered PDF page images and observations.

use agentmage_kernel_contracts::CONTRACT_SCHEMA_VERSION;
use serde::{Deserialize, Serialize};

use crate::WordRenderProfile;
use crate::word_ooxml::word_sha256;
use crate::word_visual::{
    WordPageImage, WordRenderEvidenceKind, WordRenderOutput, WordRenderPlatform,
    WordVisualComparisonError, WordVisualComparisonReport, compare_word_page_images,
    word_render_profile_sha256,
};

/// PDF page image representation shared with the pure visual comparator.
pub type PdfPageImage = WordPageImage;
/// Closed native platform identity shared with document rendering evidence.
pub type PdfRenderPlatform = WordRenderPlatform;
/// Native-versus-synthetic evidence class.
pub type PdfRenderEvidenceKind = WordRenderEvidenceKind;

/// Exact PDF render identity and accepted semantic-observation thresholds.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PdfRenderProfile {
    /// Reused deterministic pixel-comparison and renderer profile.
    pub visual_profile: WordRenderProfile,
    /// Maximum observed redaction-mask failures.
    pub max_redaction_failure_count: u32,
    /// Maximum observed reading-order failures.
    pub max_reading_order_failure_count: u32,
    /// Maximum observed missing alternative-text records.
    pub max_missing_alt_text_count: u32,
    /// Maximum observed inaccessible form fields.
    pub max_inaccessible_form_field_count: u32,
}

/// Exact externally produced PDF render evidence consumed by the pure comparator.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PdfRenderOutput {
    /// Stable output identity.
    pub render_id: String,
    /// SHA-256 of the exact PDF package rendered.
    pub pdf_sha256: String,
    /// Platform on which rendering occurred.
    pub platform: PdfRenderPlatform,
    /// SHA-256 of the complete PDF render profile.
    pub profile_sha256: String,
    /// Native or synthetic provenance.
    pub evidence_kind: PdfRenderEvidenceKind,
    /// Canonically ordered exact decoded RGBA pages.
    pub pages: Vec<PdfPageImage>,
    /// Renderer-reported clipping observations.
    pub clipping_count: u32,
    /// Renderer-reported overlapping-element observations.
    pub overlap_count: u32,
    /// Renderer-reported font substitutions.
    pub font_fallback_count: u32,
    /// Renderer-reported table layout regressions.
    pub table_regression_count: u32,
    /// Renderer-reported image regressions.
    pub image_regression_count: u32,
    /// Observer-reported visible redaction-mask failures.
    pub redaction_failure_count: u32,
    /// Observer-reported reading-order failures.
    pub reading_order_failure_count: u32,
    /// Observer-reported missing alternative-text records.
    pub missing_alt_text_count: u32,
    /// Observer-reported inaccessible form fields.
    pub inaccessible_form_field_count: u32,
    /// True only when the observation adapter completed all declared checks.
    pub observation_complete: bool,
}

/// Complete pure PDF visual and accessibility comparison report.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PdfVisualComparisonReport {
    /// Kernel contract schema version.
    pub schema_version: u16,
    /// Stable comparison identity.
    pub comparison_id: String,
    /// SHA-256 of the complete PDF render profile.
    pub profile_sha256: String,
    /// Exact pixel-level comparison.
    pub visual: WordVisualComparisonReport,
    /// After-render visible redaction failures.
    pub redaction_failure_count: u32,
    /// After-render reading-order failures.
    pub reading_order_failure_count: u32,
    /// After-render missing alternative-text records.
    pub missing_alt_text_count: u32,
    /// After-render inaccessible form fields.
    pub inaccessible_form_field_count: u32,
    /// True only when the external observation adapter completed its checks.
    pub observation_complete: bool,
    /// True only when pixel and declared semantic thresholds pass.
    pub machine_checks_passed: bool,
    /// True for synthetic evidence, incomplete observations, or a failed threshold.
    pub human_review_required: bool,
    /// False because this comparator receives decoded pixels and observations.
    pub filesystem_effect_performed: bool,
    /// False because this comparator uses no network.
    pub network_access_performed: bool,
    /// False because this comparator launches no renderer or document content.
    pub execution_performed: bool,
}

/// PDF visual input or comparison failure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PdfVisualComparisonError {
    /// An identity, digest, profile, page, or observation is invalid.
    InvalidInput,
    /// The supplied renders have incompatible platform, profile, or evidence provenance.
    IncompatibleEvidence,
    /// A page or decoded pixel resource ceiling was exceeded.
    ResourceLimit,
}

impl PdfVisualComparisonError {
    /// Stable content-free error code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidInput => "pdf_visual.input.invalid",
            Self::IncompatibleEvidence => "pdf_visual.evidence.incompatible",
            Self::ResourceLimit => "pdf_visual.resource.limit",
        }
    }
}

impl std::fmt::Display for PdfVisualComparisonError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.code())
    }
}

impl std::error::Error for PdfVisualComparisonError {}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}

/// Returns the exact digest of a validated PDF render profile.
pub fn pdf_render_profile_sha256(
    profile: &PdfRenderProfile,
) -> Result<String, PdfVisualComparisonError> {
    word_render_profile_sha256(&profile.visual_profile).map_err(PdfVisualComparisonError::from)?;
    if profile.max_redaction_failure_count != 0
        || profile.max_reading_order_failure_count != 0
        || profile.max_missing_alt_text_count != 0
        || profile.max_inaccessible_form_field_count != 0
    {
        return Err(PdfVisualComparisonError::InvalidInput);
    }
    let bytes = serde_json::to_vec(profile).map_err(|_| PdfVisualComparisonError::InvalidInput)?;
    Ok(word_sha256(&bytes))
}

fn word_output(
    output: &PdfRenderOutput,
    word_profile_sha256: &str,
) -> Result<WordRenderOutput, PdfVisualComparisonError> {
    if !valid_sha256(&output.pdf_sha256) {
        return Err(PdfVisualComparisonError::InvalidInput);
    }
    Ok(WordRenderOutput {
        render_id: output.render_id.clone(),
        package_sha256: output.pdf_sha256.clone(),
        platform: output.platform,
        profile_sha256: word_profile_sha256.to_owned(),
        evidence_kind: output.evidence_kind,
        pages: output.pages.clone(),
        clipping_count: output.clipping_count,
        overlap_count: output.overlap_count,
        font_fallback_count: output.font_fallback_count,
        table_regression_count: output.table_regression_count,
        image_regression_count: output.image_regression_count,
    })
}

/// Compares caller-supplied PDF renders without launching or trusting a renderer implicitly.
pub fn compare_pdf_page_images(
    comparison_id: &str,
    profile: &PdfRenderProfile,
    before: &PdfRenderOutput,
    after: &PdfRenderOutput,
) -> Result<PdfVisualComparisonReport, PdfVisualComparisonError> {
    let profile_sha256 = pdf_render_profile_sha256(profile)?;
    if before.profile_sha256 != profile_sha256
        || after.profile_sha256 != profile_sha256
        || before.platform != after.platform
        || before.evidence_kind != after.evidence_kind
    {
        return Err(PdfVisualComparisonError::IncompatibleEvidence);
    }
    let word_profile_sha256 = word_render_profile_sha256(&profile.visual_profile)
        .map_err(PdfVisualComparisonError::from)?;
    let visual = compare_word_page_images(
        comparison_id,
        &profile.visual_profile,
        &word_output(before, &word_profile_sha256)?,
        &word_output(after, &word_profile_sha256)?,
    )
    .map_err(PdfVisualComparisonError::from)?;
    let semantic_checks_passed = after.observation_complete
        && after.redaction_failure_count <= profile.max_redaction_failure_count
        && after.reading_order_failure_count <= profile.max_reading_order_failure_count
        && after.missing_alt_text_count <= profile.max_missing_alt_text_count
        && after.inaccessible_form_field_count <= profile.max_inaccessible_form_field_count;
    let machine_checks_passed = visual.machine_checks_passed && semantic_checks_passed;
    Ok(PdfVisualComparisonReport {
        schema_version: CONTRACT_SCHEMA_VERSION,
        comparison_id: comparison_id.to_owned(),
        profile_sha256,
        visual,
        redaction_failure_count: after.redaction_failure_count,
        reading_order_failure_count: after.reading_order_failure_count,
        missing_alt_text_count: after.missing_alt_text_count,
        inaccessible_form_field_count: after.inaccessible_form_field_count,
        observation_complete: after.observation_complete,
        machine_checks_passed,
        human_review_required: after.evidence_kind == PdfRenderEvidenceKind::SyntheticFixture
            || !machine_checks_passed,
        filesystem_effect_performed: false,
        network_access_performed: false,
        execution_performed: false,
    })
}

impl From<WordVisualComparisonError> for PdfVisualComparisonError {
    fn from(error: WordVisualComparisonError) -> Self {
        match error {
            WordVisualComparisonError::InvalidInput => Self::InvalidInput,
            WordVisualComparisonError::ResourceLimit => Self::ResourceLimit,
            WordVisualComparisonError::IncompatibleEvidence => Self::IncompatibleEvidence,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profile() -> PdfRenderProfile {
        PdfRenderProfile {
            visual_profile: WordRenderProfile {
                profile_id: "pdf-render-v1".to_owned(),
                renderer_id: "pinned-pdf-renderer".to_owned(),
                renderer_version: "1.0.0".to_owned(),
                renderer_binary_sha256: "a".repeat(64),
                font_manifest_sha256: "b".repeat(64),
                dots_per_inch: 144,
                max_changed_pixel_ratio_ppm: 250_000,
                max_channel_delta: 16,
                max_pagination_delta: 0,
                max_clipping_count: 0,
                max_overlap_count: 0,
                max_font_fallback_count: 0,
                max_table_regression_count: 0,
                max_image_regression_count: 0,
            },
            max_redaction_failure_count: 0,
            max_reading_order_failure_count: 0,
            max_missing_alt_text_count: 0,
            max_inaccessible_form_field_count: 0,
        }
    }

    fn output(render_id: &str, profile_sha256: &str, pixels: Vec<u8>) -> PdfRenderOutput {
        PdfRenderOutput {
            render_id: render_id.to_owned(),
            pdf_sha256: "c".repeat(64),
            platform: PdfRenderPlatform::Fedora,
            profile_sha256: profile_sha256.to_owned(),
            evidence_kind: PdfRenderEvidenceKind::SyntheticFixture,
            pages: vec![PdfPageImage {
                page_number: 1,
                width: 2,
                height: 2,
                rgba_sha256: word_sha256(&pixels),
                rgba: pixels,
            }],
            clipping_count: 0,
            overlap_count: 0,
            font_fallback_count: 0,
            table_regression_count: 0,
            image_regression_count: 0,
            redaction_failure_count: 0,
            reading_order_failure_count: 0,
            missing_alt_text_count: 0,
            inaccessible_form_field_count: 0,
            observation_complete: true,
        }
    }

    #[test]
    fn compares_synthetic_pages_without_claiming_release_evidence() {
        let profile = profile();
        let hash = pdf_render_profile_sha256(&profile).expect("profile");
        let before = output("before", &hash, vec![0; 16]);
        let after = output("after", &hash, vec![0; 16]);
        let report =
            compare_pdf_page_images("pdf-comparison", &profile, &before, &after).expect("compare");
        assert!(report.machine_checks_passed);
        assert!(report.human_review_required);
        assert!(report.visual.pages[0].passed);
        assert!(!report.filesystem_effect_performed);
        assert!(!report.network_access_performed);
        assert!(!report.execution_performed);
    }

    #[test]
    fn fails_machine_checks_for_layout_redaction_and_accessibility_observations() {
        let profile = profile();
        let hash = pdf_render_profile_sha256(&profile).expect("profile");
        let before = output("before", &hash, vec![0; 16]);
        let mut after = output("after", &hash, vec![255; 16]);
        after.clipping_count = 1;
        after.redaction_failure_count = 1;
        after.reading_order_failure_count = 1;
        let report =
            compare_pdf_page_images("pdf-comparison", &profile, &before, &after).expect("report");
        assert!(!report.machine_checks_passed);
        assert!(report.human_review_required);
    }

    #[test]
    fn rejects_profile_mismatch_tampering_and_incomplete_observations() {
        let profile = profile();
        let hash = pdf_render_profile_sha256(&profile).expect("profile");
        let before = output("before", &hash, vec![0; 16]);
        let mut mismatched = output("after", &"d".repeat(64), vec![0; 16]);
        assert_eq!(
            compare_pdf_page_images("pdf-comparison", &profile, &before, &mismatched),
            Err(PdfVisualComparisonError::IncompatibleEvidence)
        );
        mismatched.profile_sha256 = hash.clone();
        mismatched.pages[0].rgba[0] = 1;
        assert_eq!(
            compare_pdf_page_images("pdf-comparison", &profile, &before, &mismatched),
            Err(PdfVisualComparisonError::InvalidInput)
        );
        let mut incomplete = output("after", &hash, vec![0; 16]);
        incomplete.observation_complete = false;
        let report = compare_pdf_page_images("pdf-comparison", &profile, &before, &incomplete)
            .expect("report");
        assert!(!report.machine_checks_passed);
        assert!(report.human_review_required);
    }
}
