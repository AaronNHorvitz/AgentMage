//! Closed, truthful completion receipts for Word artifact workflows.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;

use agentmage_kernel_contracts::{CONTRACT_SCHEMA_VERSION, WorkspacePath};
use serde::{Deserialize, Serialize};

use crate::word_generation::valid_identifier;
use crate::word_ooxml::word_sha256;
use crate::{
    WordEditOperationKind, WordRenderEvidenceKind, WordRenderPlatform, WordVisualComparisonReport,
};

const MAX_RECEIPT_ITEMS: usize = 4_096;

/// One exact immutable artifact input.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WordArtifactInput {
    /// Stable input role.
    pub role: String,
    /// Exact workspace-relative path.
    pub path: WorkspacePath,
    /// Exact input SHA-256.
    pub sha256: String,
}

/// One exact change included in an artifact proposal.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WordArtifactChangeReference {
    /// Stable change identity.
    pub change_id: String,
    /// Closed Word edit operation class, absent for generation-only changes.
    pub operation_kind: Option<WordEditOperationKind>,
    /// Canonical package part name.
    pub part_name: String,
    /// Original part SHA-256, absent for a newly generated part.
    pub before_sha256: Option<String>,
    /// Proposed part SHA-256.
    pub after_sha256: String,
}

/// Closed artifact verification class.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WordArtifactCheckKind {
    /// OOXML package structure and relationships.
    Structural,
    /// Intended text, style, and edit semantics.
    Semantic,
    /// Pinned page-image comparison.
    Visual,
    /// Declared document accessibility checks.
    Accessibility,
    /// Malware and active-content policy.
    MalwarePolicy,
    /// Parser and renderer canary corpus.
    Canary,
}

/// Closed verification result state.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WordArtifactCheckStatus {
    /// The exact check passed with retained evidence.
    Passed,
    /// The exact check ran and failed.
    Failed,
    /// Required infrastructure or independent evidence is unavailable.
    Unavailable,
}

/// One exact artifact verification result.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WordArtifactCheck {
    /// Stable check identity.
    pub check_id: String,
    /// Closed verification class.
    pub kind: WordArtifactCheckKind,
    /// Exact result state.
    pub status: WordArtifactCheckStatus,
    /// SHA-256 of retained evidence, absent only when unavailable.
    pub evidence_sha256: Option<String>,
    /// Stable content-free outcome or blocker code.
    pub reason_code: String,
}

/// One explicit fidelity limitation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WordFidelityLimit {
    /// Stable limitation code.
    pub limit_code: String,
    /// True when the limitation blocks artifact completion.
    pub blocks_completion: bool,
}

/// Compact exact reference to one visual-comparison report.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WordRenderEvidenceReference {
    /// Exact render platform.
    pub platform: WordRenderPlatform,
    /// Exact visual-comparison report SHA-256.
    pub report_sha256: String,
    /// True only for pinned native-renderer evidence.
    pub native_renderer_evidence: bool,
    /// Exact machine-threshold outcome.
    pub machine_checks_passed: bool,
    /// Exact human-review requirement from the report.
    pub human_review_required: bool,
}

/// Truthful receipt completion state.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WordArtifactCompletionState {
    /// Every required local check and native platform report passed.
    LocallyVerified,
    /// Required evidence is absent, synthetic, or awaiting review.
    Blocked,
    /// At least one required check or render failed.
    Failed,
}

/// Immutable request used to build a deterministic receipt.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WordArtifactReceiptRequest {
    /// Stable artifact identity.
    pub artifact_id: String,
    /// Exact proposed artifact SHA-256.
    pub artifact_sha256: String,
    /// Canonically ordered immutable inputs.
    pub inputs: Vec<WordArtifactInput>,
    /// Generator, converter, and editor identities by stable role.
    pub implementation_identities: BTreeMap<String, String>,
    /// Canonically ordered exact changes.
    pub changes: Vec<WordArtifactChangeReference>,
    /// Canonically ordered verification checks.
    pub checks: Vec<WordArtifactCheck>,
    /// Visual comparison reports supplied by platform adapters or fixtures.
    pub visual_reports: Vec<WordVisualComparisonReport>,
    /// Required release platforms for this receipt.
    pub required_platforms: Vec<WordRenderPlatform>,
    /// Canonically ordered known fidelity limitations.
    pub known_fidelity_limits: Vec<WordFidelityLimit>,
}

/// Complete deterministic Word artifact receipt.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WordArtifactReceipt {
    /// Contract schema version.
    pub schema_version: u16,
    /// Stable artifact identity.
    pub artifact_id: String,
    /// Exact proposed artifact SHA-256.
    pub artifact_sha256: String,
    /// Canonically ordered immutable inputs.
    pub inputs: Vec<WordArtifactInput>,
    /// Generator, converter, and editor identities by stable role.
    pub implementation_identities: BTreeMap<String, String>,
    /// Canonically ordered exact changes.
    pub changes: Vec<WordArtifactChangeReference>,
    /// Canonically ordered verification checks.
    pub checks: Vec<WordArtifactCheck>,
    /// Canonically ordered render evidence.
    pub render_outputs: Vec<WordRenderEvidenceReference>,
    /// Canonically ordered known fidelity limitations.
    pub known_fidelity_limits: Vec<WordFidelityLimit>,
    /// Exact truthful completion state.
    pub completion_state: WordArtifactCompletionState,
    /// Canonically ordered content-free blocker or failure codes.
    pub disposition_codes: Vec<String>,
    /// Always true; the original remains authoritative.
    pub original_preserved: bool,
    /// Always true; the receipt describes an unpersisted proposal.
    pub proposal_only: bool,
    /// Always false; receipt creation writes no artifact.
    pub filesystem_effect_performed: bool,
    /// Always false; receipt creation uses no network.
    pub network_access_performed: bool,
    /// Always false; receipt creation launches no renderer or document content.
    pub execution_performed: bool,
}

/// Receipt validation or verification failure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WordArtifactReceiptError {
    /// An identity, hash, collection, or ordering rule is invalid.
    InvalidInput,
    /// A supplied receipt differs from deterministic recomputation.
    ReceiptMismatch,
}

impl fmt::Display for WordArtifactReceiptError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InvalidInput => "invalid Word artifact receipt input",
            Self::ReceiptMismatch => "Word artifact receipt mismatch",
        })
    }
}

impl Error for WordArtifactReceiptError {}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn validate_identifier_sequence<'a>(
    values: impl IntoIterator<Item = &'a str>,
) -> Result<(), WordArtifactReceiptError> {
    let mut previous = "";
    let mut observed = BTreeSet::new();
    for value in values {
        if !valid_identifier(value)
            || (!previous.is_empty() && value <= previous)
            || !observed.insert(value)
        {
            return Err(WordArtifactReceiptError::InvalidInput);
        }
        previous = value;
    }
    Ok(())
}

fn validate_request(request: &WordArtifactReceiptRequest) -> Result<(), WordArtifactReceiptError> {
    if !valid_identifier(&request.artifact_id)
        || !valid_sha256(&request.artifact_sha256)
        || request.inputs.is_empty()
        || request.inputs.len() > MAX_RECEIPT_ITEMS
        || request.changes.len() > MAX_RECEIPT_ITEMS
        || request.checks.len() > MAX_RECEIPT_ITEMS
        || request.visual_reports.len() > MAX_RECEIPT_ITEMS
        || request.required_platforms.is_empty()
        || request.required_platforms.len() > 4
        || request.known_fidelity_limits.len() > MAX_RECEIPT_ITEMS
    {
        return Err(WordArtifactReceiptError::InvalidInput);
    }
    validate_identifier_sequence(request.inputs.iter().map(|item| item.role.as_str()))?;
    validate_identifier_sequence(request.changes.iter().map(|item| item.change_id.as_str()))?;
    validate_identifier_sequence(request.checks.iter().map(|item| item.check_id.as_str()))?;
    validate_identifier_sequence(
        request
            .known_fidelity_limits
            .iter()
            .map(|item| item.limit_code.as_str()),
    )?;
    if request
        .inputs
        .iter()
        .any(|item| !valid_sha256(&item.sha256))
        || request.implementation_identities.is_empty()
        || request
            .implementation_identities
            .iter()
            .any(|(role, digest)| !valid_identifier(role) || !valid_sha256(digest))
        || request.changes.iter().any(|item| {
            !valid_sha256(&item.after_sha256)
                || item
                    .before_sha256
                    .as_deref()
                    .is_some_and(|digest| !valid_sha256(digest))
                || item.part_name.is_empty()
                || item.part_name.len() > 1_024
        })
        || request.checks.iter().any(|item| {
            !valid_identifier(&item.reason_code)
                || match item.status {
                    WordArtifactCheckStatus::Passed | WordArtifactCheckStatus::Failed => item
                        .evidence_sha256
                        .as_deref()
                        .is_none_or(|digest| !valid_sha256(digest)),
                    WordArtifactCheckStatus::Unavailable => item.evidence_sha256.is_some(),
                }
        })
        || request
            .known_fidelity_limits
            .iter()
            .any(|item| !valid_identifier(&item.limit_code))
    {
        return Err(WordArtifactReceiptError::InvalidInput);
    }
    if request
        .required_platforms
        .windows(2)
        .any(|items| items[0] >= items[1])
        || request
            .visual_reports
            .windows(2)
            .any(|items| items[0].platform >= items[1].platform)
    {
        return Err(WordArtifactReceiptError::InvalidInput);
    }
    let kinds = request
        .checks
        .iter()
        .map(|item| item.kind)
        .collect::<BTreeSet<_>>();
    if kinds.len() != request.checks.len() {
        return Err(WordArtifactReceiptError::InvalidInput);
    }
    Ok(())
}

fn report_reference(
    report: &WordVisualComparisonReport,
) -> Result<WordRenderEvidenceReference, WordArtifactReceiptError> {
    let encoded = serde_json::to_vec(report).map_err(|_| WordArtifactReceiptError::InvalidInput)?;
    Ok(WordRenderEvidenceReference {
        platform: report.platform,
        report_sha256: word_sha256(&encoded),
        native_renderer_evidence: report.evidence_kind == WordRenderEvidenceKind::NativeRenderer,
        machine_checks_passed: report.machine_checks_passed,
        human_review_required: report.human_review_required,
    })
}

/// Builds a truthful deterministic Word artifact receipt.
pub fn build_word_artifact_receipt(
    request: &WordArtifactReceiptRequest,
) -> Result<WordArtifactReceipt, WordArtifactReceiptError> {
    validate_request(request)?;
    let render_outputs = request
        .visual_reports
        .iter()
        .map(report_reference)
        .collect::<Result<Vec<_>, _>>()?;
    let required_checks = [
        WordArtifactCheckKind::Structural,
        WordArtifactCheckKind::Semantic,
        WordArtifactCheckKind::Visual,
        WordArtifactCheckKind::Accessibility,
        WordArtifactCheckKind::MalwarePolicy,
        WordArtifactCheckKind::Canary,
    ];
    let mut disposition_codes = BTreeSet::new();
    let failed_check = request
        .checks
        .iter()
        .any(|item| item.status == WordArtifactCheckStatus::Failed);
    for kind in required_checks {
        match request.checks.iter().find(|item| item.kind == kind) {
            None => {
                disposition_codes
                    .insert(format!("word.receipt.check.{kind:?}.missing").to_lowercase());
            }
            Some(item) if item.status != WordArtifactCheckStatus::Passed => {
                disposition_codes.insert(item.reason_code.clone());
            }
            Some(_) => {}
        }
    }
    let mut failed_render = false;
    for platform in &request.required_platforms {
        match render_outputs
            .iter()
            .find(|item| &item.platform == platform)
        {
            None => {
                disposition_codes
                    .insert(format!("word.receipt.render.{platform:?}.missing").to_lowercase());
            }
            Some(item) if !item.machine_checks_passed => {
                failed_render = true;
                disposition_codes
                    .insert(format!("word.receipt.render.{platform:?}.failed").to_lowercase());
            }
            Some(item) if !item.native_renderer_evidence => {
                disposition_codes
                    .insert(format!("word.receipt.render.{platform:?}.synthetic").to_lowercase());
            }
            Some(item) if item.human_review_required => {
                disposition_codes.insert(
                    format!("word.receipt.render.{platform:?}.review-required").to_lowercase(),
                );
            }
            Some(_) => {}
        }
    }
    for limit in request
        .known_fidelity_limits
        .iter()
        .filter(|item| item.blocks_completion)
    {
        disposition_codes.insert(limit.limit_code.clone());
    }
    let completion_state = if failed_check || failed_render {
        WordArtifactCompletionState::Failed
    } else if disposition_codes.is_empty() {
        WordArtifactCompletionState::LocallyVerified
    } else {
        WordArtifactCompletionState::Blocked
    };
    Ok(WordArtifactReceipt {
        schema_version: CONTRACT_SCHEMA_VERSION,
        artifact_id: request.artifact_id.clone(),
        artifact_sha256: request.artifact_sha256.clone(),
        inputs: request.inputs.clone(),
        implementation_identities: request.implementation_identities.clone(),
        changes: request.changes.clone(),
        checks: request.checks.clone(),
        render_outputs,
        known_fidelity_limits: request.known_fidelity_limits.clone(),
        completion_state,
        disposition_codes: disposition_codes.into_iter().collect(),
        original_preserved: true,
        proposal_only: true,
        filesystem_effect_performed: false,
        network_access_performed: false,
        execution_performed: false,
    })
}

/// Rebuilds a receipt from exact inputs and requires byte-meaning equality.
pub fn verify_word_artifact_receipt(
    request: &WordArtifactReceiptRequest,
    receipt: &WordArtifactReceipt,
) -> Result<(), WordArtifactReceiptError> {
    let expected = build_word_artifact_receipt(request)?;
    if &expected == receipt {
        Ok(())
    } else {
        Err(WordArtifactReceiptError::ReceiptMismatch)
    }
}

#[cfg(test)]
mod tests {
    use agentmage_kernel_contracts::{WorkspaceId, WorkspacePath};

    use super::*;
    use crate::{
        WordPageImage, WordRenderOutput, WordRenderProfile, compare_word_page_images,
        word_render_profile_sha256,
    };

    fn path() -> WorkspacePath {
        WorkspacePath::new(
            WorkspaceId::from_raw("workspace-receipt"),
            ["in", "source.docx"],
        )
        .expect("path")
    }

    fn check(
        id: &str,
        kind: WordArtifactCheckKind,
        status: WordArtifactCheckStatus,
    ) -> WordArtifactCheck {
        WordArtifactCheck {
            check_id: id.to_owned(),
            kind,
            status,
            evidence_sha256: (status != WordArtifactCheckStatus::Unavailable)
                .then(|| "e".repeat(64)),
            reason_code: format!("word.check.{id}"),
        }
    }

    fn visual_report() -> WordVisualComparisonReport {
        let profile = WordRenderProfile {
            profile_id: "word-render-v1".to_owned(),
            renderer_id: "pinned-renderer".to_owned(),
            renderer_version: "1.0.0".to_owned(),
            renderer_binary_sha256: "a".repeat(64),
            font_manifest_sha256: "b".repeat(64),
            dots_per_inch: 144,
            max_changed_pixel_ratio_ppm: 0,
            max_channel_delta: 0,
            max_pagination_delta: 0,
            max_clipping_count: 0,
            max_overlap_count: 0,
            max_font_fallback_count: 0,
            max_table_regression_count: 0,
            max_image_regression_count: 0,
        };
        let profile_sha256 = word_render_profile_sha256(&profile).expect("profile");
        let page = WordPageImage {
            page_number: 1,
            width: 1,
            height: 1,
            rgba: vec![0, 0, 0, 255],
            rgba_sha256: word_sha256(&[0, 0, 0, 255]),
        };
        let output = |id: &str| WordRenderOutput {
            render_id: id.to_owned(),
            package_sha256: "c".repeat(64),
            platform: WordRenderPlatform::Fedora,
            profile_sha256: profile_sha256.clone(),
            evidence_kind: WordRenderEvidenceKind::SyntheticFixture,
            pages: vec![page.clone()],
            clipping_count: 0,
            overlap_count: 0,
            font_fallback_count: 0,
            table_regression_count: 0,
            image_regression_count: 0,
        };
        compare_word_page_images(
            "receipt-visual",
            &profile,
            &output("before"),
            &output("after"),
        )
        .expect("compare")
    }

    fn request() -> WordArtifactReceiptRequest {
        WordArtifactReceiptRequest {
            artifact_id: "artifact-1".to_owned(),
            artifact_sha256: "d".repeat(64),
            inputs: vec![WordArtifactInput {
                role: "source".to_owned(),
                path: path(),
                sha256: "a".repeat(64),
            }],
            implementation_identities: BTreeMap::from([("generator".to_owned(), "b".repeat(64))]),
            changes: vec![WordArtifactChangeReference {
                change_id: "change-1".to_owned(),
                operation_kind: Some(WordEditOperationKind::ReplaceText),
                part_name: "word/document.xml".to_owned(),
                before_sha256: Some("c".repeat(64)),
                after_sha256: "d".repeat(64),
            }],
            checks: vec![
                check(
                    "accessibility",
                    WordArtifactCheckKind::Accessibility,
                    WordArtifactCheckStatus::Passed,
                ),
                check(
                    "canary",
                    WordArtifactCheckKind::Canary,
                    WordArtifactCheckStatus::Passed,
                ),
                check(
                    "malware",
                    WordArtifactCheckKind::MalwarePolicy,
                    WordArtifactCheckStatus::Passed,
                ),
                check(
                    "semantic",
                    WordArtifactCheckKind::Semantic,
                    WordArtifactCheckStatus::Passed,
                ),
                check(
                    "structural",
                    WordArtifactCheckKind::Structural,
                    WordArtifactCheckStatus::Passed,
                ),
                check(
                    "visual",
                    WordArtifactCheckKind::Visual,
                    WordArtifactCheckStatus::Passed,
                ),
            ],
            visual_reports: vec![visual_report()],
            required_platforms: vec![WordRenderPlatform::Fedora],
            known_fidelity_limits: vec![],
        }
    }

    #[test]
    fn synthetic_or_missing_platform_evidence_blocks_completion_truthfully() {
        let request = request();
        let receipt = build_word_artifact_receipt(&request).expect("receipt");
        assert_eq!(
            receipt.completion_state,
            WordArtifactCompletionState::Blocked
        );
        assert!(
            receipt
                .disposition_codes
                .iter()
                .any(|code| code.contains("synthetic"))
        );
        assert!(receipt.original_preserved && receipt.proposal_only);
        assert!(!receipt.filesystem_effect_performed);
        assert!(!receipt.network_access_performed);
        assert!(!receipt.execution_performed);
        verify_word_artifact_receipt(&request, &receipt).expect("verify");
    }

    #[test]
    fn failed_checks_win_and_tampering_is_detected() {
        let mut request = request();
        request.checks[0] = check(
            "accessibility",
            WordArtifactCheckKind::Accessibility,
            WordArtifactCheckStatus::Failed,
        );
        let receipt = build_word_artifact_receipt(&request).expect("receipt");
        assert_eq!(
            receipt.completion_state,
            WordArtifactCompletionState::Failed
        );
        let mut tampered = receipt.clone();
        tampered.completion_state = WordArtifactCompletionState::LocallyVerified;
        assert_eq!(
            verify_word_artifact_receipt(&request, &tampered),
            Err(WordArtifactReceiptError::ReceiptMismatch)
        );

        let mut unordered = request;
        unordered.checks.swap(0, 1);
        assert_eq!(
            build_word_artifact_receipt(&unordered),
            Err(WordArtifactReceiptError::InvalidInput)
        );
    }
}
