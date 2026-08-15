//! Integrity-protected review packets, finding normalization, and logical commit plans.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::command_runner::{CommandReceipt, PreparedCommand};
use crate::validation_result::{
    ValidationCoverageStatus, ValidationReceipt, ValidationStatus, verify_validation_receipt,
};
use crate::validation_template::{ValidationKind, ValidationTemplate};
use crate::write_approval::{
    ShadowChangeSet, WriteArtifactClass, WriteChangeScope, WriteReviewHook,
};

const REVIEW_PACKET_SCHEMA_VERSION: u16 = 1;
const MAX_CHANGED_FILES: usize = 128;
const MAX_UNRELATED_FILES: usize = 4_096;
const MAX_FINDINGS: usize = 2_048;
const MAX_VALIDATIONS: usize = 256;
const MAX_EVIDENCE_ARTIFACTS: usize = 128;
const MAX_DIFF_BYTES: usize = 8 * 1024 * 1024;
const MIN_VISIBLE_CONFIDENCE_BPS: u16 = 7_000;

/// Stable reason a review packet cannot be constructed or verified.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReviewPacketError {
    /// Syntax, bounds, ordering, identity, or coverage is invalid.
    InvalidInput,
    /// A validation receipt is stale, malformed, or not bound to its exact command.
    ValidationEvidenceInvalid,
    /// Change purposes do not account for the exact shadow change set.
    ChangeSetMismatch,
    /// A commit plan attempts to include unrelated or unapproved work.
    CommitPlanDenied,
    /// Canonical packet construction failed.
    ReceiptFailure,
}

impl ReviewPacketError {
    /// Stable content-free code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidInput => "review-packet.input-invalid",
            Self::ValidationEvidenceInvalid => "review-packet.validation-invalid",
            Self::ChangeSetMismatch => "review-packet.change-set-mismatch",
            Self::CommitPlanDenied => "review-packet.commit-plan-denied",
            Self::ReceiptFailure => "review-packet.receipt-failure",
        }
    }
}

/// Closed purpose used to group one approved changed file.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CommitChangePurpose {
    /// User-visible or system behavior.
    Behavior,
    /// Formatting-only byte changes.
    Formatting,
    /// Test source, fixture, or golden changes.
    Tests,
    /// Documentation or example changes.
    Documentation,
    /// Data, schema, or compatibility migration.
    Migration,
    /// Explicit generated output.
    GeneratedOutput,
}

/// Exact purpose assignment for one shadow operation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChangePurposeAssignment {
    /// Exact operation identity.
    pub operation_id: String,
    /// Review-supported purpose.
    pub purpose: CommitChangePurpose,
}

/// One file derived from an exact shadow change set and retained in a review packet.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReviewChangedFile {
    /// Exact shadow operation identity.
    pub operation_id: String,
    /// Canonical workspace-relative display path.
    pub path: String,
    /// Exact preimage digest.
    pub preimage_sha256: String,
    /// Exact approved postimage digest.
    pub postimage_sha256: String,
    /// Closed source artifact class.
    pub artifact_class: ReviewArtifactClass,
    /// Logical commit purpose.
    pub purpose: CommitChangePurpose,
    /// Whether the repository identified the output as generated.
    pub generated: bool,
    /// Complete escaped exact before/after diff.
    pub complete_diff: String,
    /// Exact shadow operation digest.
    pub operation_sha256: String,
}

/// Serializable mirror of the write artifact classes used by review packets.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviewArtifactClass {
    /// Application or library source.
    Code,
    /// Repository or product configuration.
    Configuration,
    /// Test source, fixture, or golden.
    Test,
    /// Documentation or example prose.
    Documentation,
    /// Data, schema, or compatibility migration.
    Migration,
    /// Explicit generated output.
    GeneratedOutput,
}

/// Exact unrelated user change excluded from every proposed commit group.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UnrelatedChangeEvidence {
    /// Content-minimized path identity.
    pub path_sha256: String,
    /// Exact current byte identity.
    pub content_sha256: String,
    /// Stable reason the work is not part of this change set.
    pub exclusion_code: String,
}

/// Validated change-set projection with no write or Git authority.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ReviewableChangeSet {
    /// Exact shadow change-set identity.
    change_set_id: String,
    /// Exact shadow change-set digest.
    change_set_sha256: String,
    /// Exact change-intent digest.
    intent_sha256: String,
    /// Exact implementation-plan digest.
    plan_sha256: String,
    /// Exact write-approval receipt identity.
    write_approval_receipt_sha256: String,
    /// Exact successful write-application receipt identity.
    write_application_receipt_sha256: String,
    /// Closed reviewed breadth.
    scope: WriteChangeScope,
    /// Required review hooks.
    review_hooks: Vec<WriteReviewHook>,
    /// Exact path-ordered changed files.
    files: Vec<ReviewChangedFile>,
    /// Complete changed-file/diff identity.
    files_sha256: String,
    /// Projection carries no write authority.
    write_authority: bool,
}

impl ReviewableChangeSet {
    /// Returns the exact shadow change-set digest.
    #[must_use]
    pub fn change_set_sha256(&self) -> &str {
        &self.change_set_sha256
    }

    /// Returns the complete path-ordered changed-file records.
    #[must_use]
    pub fn files(&self) -> &[ReviewChangedFile] {
        &self.files
    }
}

/// Captures one complete exact change-set projection for later review.
pub fn capture_reviewable_change_set(
    change_set: &ShadowChangeSet,
    mut purposes: Vec<ChangePurposeAssignment>,
    write_approval_receipt_sha256: impl Into<String>,
    write_application_receipt_sha256: impl Into<String>,
) -> Result<ReviewableChangeSet, ReviewPacketError> {
    let write_approval_receipt_sha256 = write_approval_receipt_sha256.into();
    let write_application_receipt_sha256 = write_application_receipt_sha256.into();
    if !is_sha256(&write_approval_receipt_sha256)
        || !is_sha256(&write_application_receipt_sha256)
        || change_set.operations().is_empty()
        || change_set.operations().len() > MAX_CHANGED_FILES
    {
        return Err(ReviewPacketError::InvalidInput);
    }
    purposes.sort_by(|left, right| left.operation_id.cmp(&right.operation_id));
    if purposes
        .windows(2)
        .any(|pair| pair[0].operation_id >= pair[1].operation_id)
    {
        return Err(ReviewPacketError::ChangeSetMismatch);
    }
    let purpose_by_operation = purposes
        .into_iter()
        .map(|assignment| (assignment.operation_id, assignment.purpose))
        .collect::<BTreeMap<_, _>>();
    if purpose_by_operation.len() != change_set.operations().len() {
        return Err(ReviewPacketError::ChangeSetMismatch);
    }
    let mut total_diff_bytes = 0_usize;
    let mut files = Vec::with_capacity(change_set.operations().len());
    for operation in change_set.operations() {
        let Some(purpose) = purpose_by_operation.get(operation.operation_id()).copied() else {
            return Err(ReviewPacketError::ChangeSetMismatch);
        };
        let artifact_class = map_artifact_class(operation.artifact_class());
        if !purpose_matches_artifact(purpose, artifact_class, operation.generated_file()) {
            return Err(ReviewPacketError::ChangeSetMismatch);
        }
        total_diff_bytes = total_diff_bytes
            .checked_add(operation.complete_diff().len())
            .ok_or(ReviewPacketError::InvalidInput)?;
        if total_diff_bytes > MAX_DIFF_BYTES {
            return Err(ReviewPacketError::InvalidInput);
        }
        files.push(ReviewChangedFile {
            operation_id: operation.operation_id().to_owned(),
            path: operation.path().to_owned(),
            preimage_sha256: operation.preimage_sha256().to_owned(),
            postimage_sha256: operation.expected_postimage_sha256().to_owned(),
            artifact_class,
            purpose,
            generated: operation.generated_file(),
            complete_diff: operation.complete_diff().to_owned(),
            operation_sha256: operation.operation_sha256().to_owned(),
        });
    }
    files.sort_by(|left, right| left.path.cmp(&right.path));
    if files.windows(2).any(|pair| pair[0].path >= pair[1].path)
        || purpose_by_operation
            .keys()
            .any(|operation_id| !files.iter().any(|file| &file.operation_id == operation_id))
    {
        return Err(ReviewPacketError::ChangeSetMismatch);
    }
    Ok(ReviewableChangeSet {
        change_set_id: change_set.change_set_id().to_owned(),
        change_set_sha256: change_set.change_set_sha256().to_owned(),
        intent_sha256: change_set.intent_sha256().to_owned(),
        plan_sha256: change_set.plan_sha256().to_owned(),
        write_approval_receipt_sha256,
        write_application_receipt_sha256,
        scope: change_set.scope(),
        review_hooks: change_set.review_hooks().to_vec(),
        files_sha256: sha256_json(&files)?,
        files,
        write_authority: false,
    })
}

/// Verified projection of one exact Sprint 46 validation receipt.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct VerifiedValidationEvidence {
    /// Exact validation identity.
    validation_id: String,
    /// Exact validation kind.
    kind: ValidationKind,
    /// Exact normalized status.
    status: ValidationStatus,
    /// Exact requested-coverage status.
    coverage: ValidationCoverageStatus,
    /// Exact validation receipt digest.
    receipt_sha256: String,
    /// Whether this one result is a complete pass.
    full_pass: bool,
}

/// Verifies and projects one exact validation receipt for a review packet.
pub fn bind_validation_evidence(
    template: &ValidationTemplate,
    prepared: &PreparedCommand,
    command_receipt: &CommandReceipt,
    validation_receipt: &ValidationReceipt,
) -> Result<VerifiedValidationEvidence, ReviewPacketError> {
    if !verify_validation_receipt(template, prepared, command_receipt, validation_receipt) {
        return Err(ReviewPacketError::ValidationEvidenceInvalid);
    }
    Ok(VerifiedValidationEvidence {
        validation_id: validation_receipt.validation_id.clone(),
        kind: validation_receipt.kind,
        status: validation_receipt.status,
        coverage: validation_receipt.coverage,
        receipt_sha256: validation_receipt.receipt_sha256.clone(),
        full_pass: validation_receipt.is_full_pass(),
    })
}

/// Closed code-review mode set.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviewMode {
    /// Behavioral correctness.
    Correctness,
    /// Unnecessary complexity.
    Simplicity,
    /// Long-term maintainability.
    Maintainability,
    /// Security boundaries and attack surface.
    Security,
    /// Data integrity and migration safety.
    DataIntegrity,
    /// Accessibility behavior.
    Accessibility,
    /// Measured performance behavior.
    Performance,
    /// Test adequacy and fidelity.
    Tests,
    /// Documentation accuracy.
    Documentation,
}

/// Closed finding severity.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FindingSeverity {
    /// Informational observation.
    Info,
    /// Low-impact concern.
    Low,
    /// Material concern.
    Medium,
    /// High-impact concern.
    High,
    /// Release-blocking critical concern.
    Critical,
}

/// One evidence-backed candidate finding before normalization.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReviewFindingInput {
    /// Stable finding identity.
    pub finding_id: String,
    /// Exact review mode.
    pub mode: ReviewMode,
    /// Stable defect family.
    pub code: String,
    /// Exact source path anchor.
    pub path: String,
    /// One-based source line.
    pub line: u32,
    /// Severity with evidence-backed rationale.
    pub severity: FindingSeverity,
    /// Calibrated confidence in basis points.
    pub confidence_bps: u16,
    /// Exact immutable supporting evidence.
    pub evidence_sha256: String,
    /// Stable impact rationale code.
    pub rationale_code: String,
}

/// One normalized evidence-backed review finding.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReviewFinding {
    /// Stable finding identity.
    pub finding_id: String,
    /// Exact review mode.
    pub mode: ReviewMode,
    /// Stable defect family.
    pub code: String,
    /// Exact source path anchor.
    pub path: String,
    /// One-based source line.
    pub line: u32,
    /// Evidence-backed severity.
    pub severity: FindingSeverity,
    /// Calibrated confidence in basis points.
    pub confidence_bps: u16,
    /// Exact immutable supporting evidence.
    pub evidence_sha256: String,
    /// Stable impact rationale code.
    pub rationale_code: String,
    /// Digest over all finding fields.
    pub finding_sha256: String,
}

/// Stable reason a finding is retained but hidden from the primary result.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FindingSuppressionReason {
    /// Confidence is below the visible threshold.
    LowConfidence,
    /// Stronger evidence already represents the same anchored defect.
    Duplicate,
}

/// Preserved evidence for one suppressed finding.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SuppressedReviewFinding {
    /// Complete original normalized finding.
    pub finding: ReviewFinding,
    /// Deterministic suppression reason.
    pub reason: FindingSuppressionReason,
    /// Winning finding identity for duplicate suppression.
    pub superseded_by: Option<String>,
}

/// Complete deterministic review-mode output.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReviewModeResults {
    /// Exact modes requested and completed.
    pub modes: Vec<ReviewMode>,
    /// Visible source-ordered findings.
    pub findings: Vec<ReviewFinding>,
    /// Preserved duplicate and low-confidence evidence.
    pub suppressed: Vec<SuppressedReviewFinding>,
    /// Digest of the complete result.
    pub results_sha256: String,
    /// Review output has no mutation authority.
    pub mutation_authority: bool,
}

/// Normalizes evidence-backed findings and preserves every suppression decision.
pub fn normalize_review_findings(
    mut modes: Vec<ReviewMode>,
    inputs: Vec<ReviewFindingInput>,
) -> Result<ReviewModeResults, ReviewPacketError> {
    modes.sort();
    modes.dedup();
    if modes.is_empty() || inputs.len() > MAX_FINDINGS {
        return Err(ReviewPacketError::InvalidInput);
    }
    let allowed_modes = modes.iter().copied().collect::<BTreeSet<_>>();
    let mut identities = BTreeSet::new();
    let mut normalized = Vec::with_capacity(inputs.len());
    for input in inputs {
        if !valid_identifier(&input.finding_id)
            || !identities.insert(input.finding_id.clone())
            || !allowed_modes.contains(&input.mode)
            || !valid_identifier(&input.code)
            || !valid_path(&input.path)
            || input.line == 0
            || input.confidence_bps > 10_000
            || !is_sha256(&input.evidence_sha256)
            || !valid_identifier(&input.rationale_code)
        {
            return Err(ReviewPacketError::InvalidInput);
        }
        let mut finding = ReviewFinding {
            finding_id: input.finding_id,
            mode: input.mode,
            code: input.code,
            path: input.path,
            line: input.line,
            severity: input.severity,
            confidence_bps: input.confidence_bps,
            evidence_sha256: input.evidence_sha256,
            rationale_code: input.rationale_code,
            finding_sha256: String::new(),
        };
        finding.finding_sha256 = finding_digest(&finding)?;
        normalized.push(finding);
    }
    normalized.sort_by(finding_order);
    let mut winner_by_key = BTreeMap::<FindingKey, usize>::new();
    for (index, finding) in normalized.iter().enumerate() {
        let key = FindingKey::from(finding);
        winner_by_key
            .entry(key)
            .and_modify(|winner| {
                if finding_rank(finding) > finding_rank(&normalized[*winner]) {
                    *winner = index;
                }
            })
            .or_insert(index);
    }
    let winners = winner_by_key.values().copied().collect::<BTreeSet<_>>();
    let winner_id_by_key = winner_by_key
        .iter()
        .map(|(key, index)| (key.clone(), normalized[*index].finding_id.clone()))
        .collect::<BTreeMap<_, _>>();
    let mut findings = Vec::new();
    let mut suppressed = Vec::new();
    for (index, finding) in normalized.into_iter().enumerate() {
        if !winners.contains(&index) {
            let superseded_by = winner_id_by_key.get(&FindingKey::from(&finding)).cloned();
            suppressed.push(SuppressedReviewFinding {
                finding,
                reason: FindingSuppressionReason::Duplicate,
                superseded_by,
            });
        } else if finding.confidence_bps < MIN_VISIBLE_CONFIDENCE_BPS {
            suppressed.push(SuppressedReviewFinding {
                finding,
                reason: FindingSuppressionReason::LowConfidence,
                superseded_by: None,
            });
        } else {
            findings.push(finding);
        }
    }
    findings.sort_by(finding_order);
    suppressed.sort_by(|left, right| finding_order(&left.finding, &right.finding));
    let mut results = ReviewModeResults {
        modes,
        findings,
        suppressed,
        results_sha256: String::new(),
        mutation_authority: false,
    };
    results.results_sha256 = review_results_digest(&results)?;
    Ok(results)
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct FindingKey {
    mode: ReviewMode,
    code: String,
    path: String,
    line: u32,
}

impl From<&ReviewFinding> for FindingKey {
    fn from(value: &ReviewFinding) -> Self {
        Self {
            mode: value.mode,
            code: value.code.clone(),
            path: value.path.clone(),
            line: value.line,
        }
    }
}

/// Closed evidence-artifact family included by digest only.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviewEvidenceArtifactKind {
    /// Screenshot evidence.
    Screenshot,
    /// Bounded command or product output.
    Output,
    /// Migration or data comparison.
    DataComparison,
}

/// Content-minimized review artifact reference.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReviewEvidenceArtifact {
    /// Stable artifact identity.
    pub artifact_id: String,
    /// Closed artifact family.
    pub kind: ReviewEvidenceArtifactKind,
    /// Exact artifact digest.
    pub sha256: String,
    /// Exact artifact byte count.
    pub byte_length: u64,
    /// Whether a trusted scanner found sensitive material.
    pub sensitive_content_detected: bool,
}

/// One deterministic logical commit group.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LogicalCommitGroup {
    /// Stable group identity.
    pub group_id: String,
    /// Closed group purpose.
    pub purpose: CommitChangePurpose,
    /// Exact included operation identities.
    pub operation_ids: Vec<String>,
    /// Exact included changed paths.
    pub paths: Vec<String>,
    /// Deterministically drafted message from the approved group.
    pub proposed_message: String,
    /// Exact group digest.
    pub group_sha256: String,
}

/// Complete authority-free logical commit plan.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LogicalCommitPlan {
    /// Exact reviewable change-set digest.
    pub change_set_sha256: String,
    /// Stable purpose-ordered groups.
    pub groups: Vec<LogicalCommitGroup>,
    /// Exact unrelated user changes excluded from all groups.
    pub excluded_unrelated: Vec<UnrelatedChangeEvidence>,
    /// Automatic commit remains prohibited.
    pub automatic_commit: bool,
    /// Push remains prohibited.
    pub push_authority: bool,
    /// Merge remains prohibited.
    pub merge_authority: bool,
    /// Release remains prohibited.
    pub release_authority: bool,
    /// Exact plan digest.
    pub plan_sha256: String,
}

/// Builds stable logical commit groups only from the exact reviewable change set.
pub fn build_logical_commit_plan(
    change_set: &ReviewableChangeSet,
    mut unrelated: Vec<UnrelatedChangeEvidence>,
) -> Result<LogicalCommitPlan, ReviewPacketError> {
    if !verify_reviewable_change_set(change_set)
        || unrelated.len() > MAX_UNRELATED_FILES
        || unrelated.iter().any(|item| {
            !is_sha256(&item.path_sha256)
                || !is_sha256(&item.content_sha256)
                || !valid_identifier(&item.exclusion_code)
        })
    {
        return Err(ReviewPacketError::CommitPlanDenied);
    }
    unrelated.sort();
    if unrelated.windows(2).any(|pair| pair[0] >= pair[1]) {
        return Err(ReviewPacketError::CommitPlanDenied);
    }
    let mut by_purpose = BTreeMap::<CommitChangePurpose, Vec<&ReviewChangedFile>>::new();
    for file in &change_set.files {
        by_purpose.entry(file.purpose).or_default().push(file);
    }
    let mut groups = Vec::new();
    for (sequence, (purpose, files)) in by_purpose.into_iter().enumerate() {
        let operation_ids = files
            .iter()
            .map(|file| file.operation_id.clone())
            .collect::<Vec<_>>();
        let paths = files
            .iter()
            .map(|file| file.path.clone())
            .collect::<Vec<_>>();
        let mut operation_ids = operation_ids;
        let mut paths = paths;
        operation_ids.sort();
        paths.sort();
        let mut group = LogicalCommitGroup {
            group_id: format!("commit-group-{:02}", sequence + 1),
            purpose,
            proposed_message: commit_message(purpose, paths.len()),
            operation_ids,
            paths,
            group_sha256: String::new(),
        };
        group.group_sha256 = commit_group_digest(&group)?;
        groups.push(group);
    }
    if groups.is_empty() {
        return Err(ReviewPacketError::CommitPlanDenied);
    }
    let mut plan = LogicalCommitPlan {
        change_set_sha256: change_set.change_set_sha256.clone(),
        groups,
        excluded_unrelated: unrelated,
        automatic_commit: false,
        push_authority: false,
        merge_authority: false,
        release_authority: false,
        plan_sha256: String::new(),
    };
    plan.plan_sha256 = logical_commit_plan_digest(&plan)?;
    Ok(plan)
}

/// Complete review packet input.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReviewPacketInput {
    /// Stable packet identity.
    pub packet_id: String,
    /// Exact canonical repository identity.
    pub repository_sha256: String,
    /// Exact Git base object.
    pub base_object: String,
    /// Exact candidate worktree snapshot digest.
    pub candidate_snapshot_sha256: String,
    /// Exact current repository preservation manifest.
    pub preservation_manifest_sha256: String,
    /// User-facing objective.
    pub objective: String,
    /// Exact behavior delta.
    pub behavior_delta: String,
    /// Validated change set.
    pub change_set: ReviewableChangeSet,
    /// Verified validation evidence.
    pub validations: Vec<VerifiedValidationEvidence>,
    /// Requested checks not run.
    pub checks_not_run: Vec<ValidationKind>,
    /// Normalized review results.
    pub review_results: ReviewModeResults,
    /// Content-minimized screenshots and outputs.
    pub evidence_artifacts: Vec<ReviewEvidenceArtifact>,
    /// Authority-free logical commit plan.
    pub commit_plan: LogicalCommitPlan,
    /// Exact rollback description.
    pub rollback: String,
}

/// Integrity-protected complete local review packet.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LocalReviewPacket {
    /// Closed schema version.
    pub schema_version: u16,
    /// Stable packet identity.
    pub packet_id: String,
    /// Exact canonical repository identity.
    pub repository_sha256: String,
    /// Exact Git base object.
    pub base_object: String,
    /// Exact candidate worktree snapshot digest.
    pub candidate_snapshot_sha256: String,
    /// Exact current repository preservation manifest.
    pub preservation_manifest_sha256: String,
    /// User-facing objective.
    pub objective: String,
    /// Exact behavior delta.
    pub behavior_delta: String,
    /// Complete changed-file and diff packet.
    pub change_set: ReviewableChangeSet,
    /// Verified validation evidence.
    pub validations: Vec<VerifiedValidationEvidence>,
    /// Every requested check not run.
    pub checks_not_run: Vec<ValidationKind>,
    /// Complete review-mode findings and suppression evidence.
    pub review_results: ReviewModeResults,
    /// Screenshots and outputs included by digest only.
    pub evidence_artifacts: Vec<ReviewEvidenceArtifact>,
    /// Logical commit proposal excluding unrelated work.
    pub commit_plan: LogicalCommitPlan,
    /// Exact rollback description.
    pub rollback: String,
    /// Number of high or critical visible findings.
    pub blocking_finding_count: u32,
    /// Packet carries no commit authority.
    pub commit_authority: bool,
    /// Packet carries no publication authority.
    pub publication_authority: bool,
    /// Canonical packet digest.
    pub packet_sha256: String,
}

/// Seals one complete review packet only when all exact evidence is current and accounted.
pub fn build_review_packet(
    mut input: ReviewPacketInput,
) -> Result<LocalReviewPacket, ReviewPacketError> {
    input.validations.sort_by(|left, right| {
        (left.kind, left.validation_id.as_str()).cmp(&(right.kind, right.validation_id.as_str()))
    });
    input.checks_not_run.sort();
    input.evidence_artifacts.sort();
    if !valid_identifier(&input.packet_id)
        || !is_sha256(&input.repository_sha256)
        || !valid_object_id(&input.base_object)
        || !is_sha256(&input.candidate_snapshot_sha256)
        || !is_sha256(&input.preservation_manifest_sha256)
        || !valid_text(&input.objective)
        || !valid_text(&input.behavior_delta)
        || !valid_text(&input.rollback)
        || !verify_reviewable_change_set(&input.change_set)
        || input.validations.len() > MAX_VALIDATIONS
        || input.validations.windows(2).any(|pair| {
            (pair[0].kind, pair[0].validation_id.as_str())
                >= (pair[1].kind, pair[1].validation_id.as_str())
        })
        || input
            .validations
            .iter()
            .any(|validation| !is_sha256(&validation.receipt_sha256))
        || input
            .checks_not_run
            .windows(2)
            .any(|pair| pair[0] >= pair[1])
        || input.validations.iter().any(|validation| {
            input.checks_not_run.contains(&validation.kind)
                || validation.full_pass
                    != (validation.status == ValidationStatus::Passed
                        && validation.coverage == ValidationCoverageStatus::Complete)
        })
        || input.evidence_artifacts.len() > MAX_EVIDENCE_ARTIFACTS
        || input
            .evidence_artifacts
            .windows(2)
            .any(|pair| pair[0] >= pair[1])
        || input.evidence_artifacts.iter().any(|artifact| {
            !valid_identifier(&artifact.artifact_id)
                || !is_sha256(&artifact.sha256)
                || artifact.byte_length == 0
                || artifact.sensitive_content_detected
        })
        || !verify_review_results(&input.review_results)
        || !verify_logical_commit_plan(&input.change_set, &input.commit_plan)
    {
        return Err(ReviewPacketError::InvalidInput);
    }
    let blocking_finding_count = input
        .review_results
        .findings
        .iter()
        .filter(|finding| {
            matches!(
                finding.severity,
                FindingSeverity::High | FindingSeverity::Critical
            )
        })
        .count()
        .try_into()
        .map_err(|_| ReviewPacketError::InvalidInput)?;
    let mut packet = LocalReviewPacket {
        schema_version: REVIEW_PACKET_SCHEMA_VERSION,
        packet_id: input.packet_id,
        repository_sha256: input.repository_sha256,
        base_object: input.base_object,
        candidate_snapshot_sha256: input.candidate_snapshot_sha256,
        preservation_manifest_sha256: input.preservation_manifest_sha256,
        objective: input.objective,
        behavior_delta: input.behavior_delta,
        change_set: input.change_set,
        validations: input.validations,
        checks_not_run: input.checks_not_run,
        review_results: input.review_results,
        evidence_artifacts: input.evidence_artifacts,
        commit_plan: input.commit_plan,
        rollback: input.rollback,
        blocking_finding_count,
        commit_authority: false,
        publication_authority: false,
        packet_sha256: String::new(),
    };
    packet.packet_sha256 = review_packet_digest(&packet)?;
    Ok(packet)
}

/// Verifies a complete review packet and its cross-field accounting.
#[must_use]
pub fn verify_review_packet(packet: &LocalReviewPacket) -> bool {
    packet.schema_version == REVIEW_PACKET_SCHEMA_VERSION
        && !packet.commit_authority
        && !packet.publication_authority
        && build_review_packet(ReviewPacketInput {
            packet_id: packet.packet_id.clone(),
            repository_sha256: packet.repository_sha256.clone(),
            base_object: packet.base_object.clone(),
            candidate_snapshot_sha256: packet.candidate_snapshot_sha256.clone(),
            preservation_manifest_sha256: packet.preservation_manifest_sha256.clone(),
            objective: packet.objective.clone(),
            behavior_delta: packet.behavior_delta.clone(),
            change_set: packet.change_set.clone(),
            validations: packet.validations.clone(),
            checks_not_run: packet.checks_not_run.clone(),
            review_results: packet.review_results.clone(),
            evidence_artifacts: packet.evidence_artifacts.clone(),
            commit_plan: packet.commit_plan.clone(),
            rollback: packet.rollback.clone(),
        })
        .is_ok_and(|expected| expected == *packet)
        && review_packet_digest(packet).as_ref() == Ok(&packet.packet_sha256)
}

fn verify_reviewable_change_set(change_set: &ReviewableChangeSet) -> bool {
    let operation_count = change_set
        .files
        .iter()
        .map(|file| file.operation_id.as_str())
        .collect::<BTreeSet<_>>()
        .len();
    let total_diff_bytes = change_set.files.iter().try_fold(0_usize, |total, file| {
        total.checked_add(file.complete_diff.len())
    });
    !change_set.write_authority
        && valid_identifier(&change_set.change_set_id)
        && is_sha256(&change_set.change_set_sha256)
        && is_sha256(&change_set.intent_sha256)
        && is_sha256(&change_set.plan_sha256)
        && is_sha256(&change_set.write_approval_receipt_sha256)
        && is_sha256(&change_set.write_application_receipt_sha256)
        && change_set
            .review_hooks
            .windows(2)
            .all(|pair| pair[0] < pair[1])
        && !change_set.files.is_empty()
        && change_set.files.len() <= MAX_CHANGED_FILES
        && operation_count == change_set.files.len()
        && total_diff_bytes.is_some_and(|total| total <= MAX_DIFF_BYTES)
        && change_set
            .files
            .windows(2)
            .all(|pair| pair[0].path < pair[1].path)
        && change_set.files.iter().all(|file| {
            valid_identifier(&file.operation_id)
                && valid_path(&file.path)
                && is_sha256(&file.preimage_sha256)
                && is_sha256(&file.postimage_sha256)
                && is_sha256(&file.operation_sha256)
                && file.preimage_sha256 != file.postimage_sha256
                && !file.complete_diff.is_empty()
                && file.complete_diff.len() <= MAX_DIFF_BYTES
                && !file.complete_diff.contains('\0')
                && purpose_matches_artifact(file.purpose, file.artifact_class, file.generated)
        })
        && is_sha256(&change_set.files_sha256)
        && sha256_json(&change_set.files).as_ref() == Ok(&change_set.files_sha256)
}

fn verify_review_results(results: &ReviewModeResults) -> bool {
    let inputs = results
        .findings
        .iter()
        .chain(results.suppressed.iter().map(|entry| &entry.finding))
        .map(|finding| ReviewFindingInput {
            finding_id: finding.finding_id.clone(),
            mode: finding.mode,
            code: finding.code.clone(),
            path: finding.path.clone(),
            line: finding.line,
            severity: finding.severity,
            confidence_bps: finding.confidence_bps,
            evidence_sha256: finding.evidence_sha256.clone(),
            rationale_code: finding.rationale_code.clone(),
        })
        .collect::<Vec<_>>();
    !results.mutation_authority
        && !results.modes.is_empty()
        && results.modes.windows(2).all(|pair| pair[0] < pair[1])
        && inputs.len() <= MAX_FINDINGS
        && results
            .findings
            .windows(2)
            .all(|pair| finding_order(&pair[0], &pair[1]).is_lt())
        && results
            .suppressed
            .windows(2)
            .all(|pair| finding_order(&pair[0].finding, &pair[1].finding).is_lt())
        && results.findings.iter().all(|finding| {
            finding.confidence_bps >= MIN_VISIBLE_CONFIDENCE_BPS
                && finding.finding_sha256 == finding_digest(finding).unwrap_or_default()
        })
        && results.suppressed.iter().all(|suppressed| {
            suppressed.finding.finding_sha256
                == finding_digest(&suppressed.finding).unwrap_or_default()
        })
        && normalize_review_findings(results.modes.clone(), inputs)
            .is_ok_and(|expected| expected == *results)
        && review_results_digest(results).as_ref() == Ok(&results.results_sha256)
}

fn verify_logical_commit_plan(change_set: &ReviewableChangeSet, plan: &LogicalCommitPlan) -> bool {
    plan.change_set_sha256 == change_set.change_set_sha256
        && !plan.automatic_commit
        && !plan.push_authority
        && !plan.merge_authority
        && !plan.release_authority
        && build_logical_commit_plan(change_set, plan.excluded_unrelated.clone())
            .is_ok_and(|expected| expected == *plan)
        && logical_commit_plan_digest(plan).as_ref() == Ok(&plan.plan_sha256)
}

fn map_artifact_class(value: WriteArtifactClass) -> ReviewArtifactClass {
    match value {
        WriteArtifactClass::Code => ReviewArtifactClass::Code,
        WriteArtifactClass::Configuration => ReviewArtifactClass::Configuration,
        WriteArtifactClass::Test => ReviewArtifactClass::Test,
        WriteArtifactClass::Documentation => ReviewArtifactClass::Documentation,
        WriteArtifactClass::Migration => ReviewArtifactClass::Migration,
        WriteArtifactClass::GeneratedOutput => ReviewArtifactClass::GeneratedOutput,
    }
}

fn purpose_matches_artifact(
    purpose: CommitChangePurpose,
    artifact: ReviewArtifactClass,
    generated: bool,
) -> bool {
    match purpose {
        CommitChangePurpose::Behavior | CommitChangePurpose::Formatting => {
            matches!(
                artifact,
                ReviewArtifactClass::Code | ReviewArtifactClass::Configuration
            ) && !generated
        }
        CommitChangePurpose::Tests => artifact == ReviewArtifactClass::Test && !generated,
        CommitChangePurpose::Documentation => {
            artifact == ReviewArtifactClass::Documentation && !generated
        }
        CommitChangePurpose::Migration => artifact == ReviewArtifactClass::Migration && !generated,
        CommitChangePurpose::GeneratedOutput => {
            artifact == ReviewArtifactClass::GeneratedOutput && generated
        }
    }
}

fn finding_rank(finding: &ReviewFinding) -> (FindingSeverity, u16, &str) {
    (
        finding.severity,
        finding.confidence_bps,
        finding.finding_id.as_str(),
    )
}

fn finding_order(left: &ReviewFinding, right: &ReviewFinding) -> std::cmp::Ordering {
    (
        left.path.as_str(),
        left.line,
        left.mode,
        left.code.as_str(),
        left.finding_id.as_str(),
    )
        .cmp(&(
            right.path.as_str(),
            right.line,
            right.mode,
            right.code.as_str(),
            right.finding_id.as_str(),
        ))
}

fn commit_message(purpose: CommitChangePurpose, file_count: usize) -> String {
    let subject = match purpose {
        CommitChangePurpose::Behavior => "feat: apply approved behavior change",
        CommitChangePurpose::Formatting => "style: apply approved formatting",
        CommitChangePurpose::Tests => "test: update approved tests",
        CommitChangePurpose::Documentation => "docs: update approved documentation",
        CommitChangePurpose::Migration => "chore(migration): apply approved migration",
        CommitChangePurpose::GeneratedOutput => "chore(generate): refresh approved output",
    };
    format!("{subject}\n\nFiles: {file_count}")
}

fn finding_digest(finding: &ReviewFinding) -> Result<String, ReviewPacketError> {
    let mut canonical = finding.clone();
    canonical.finding_sha256.clear();
    sha256_json(&canonical)
}

fn review_results_digest(results: &ReviewModeResults) -> Result<String, ReviewPacketError> {
    let mut canonical = results.clone();
    canonical.results_sha256.clear();
    sha256_json(&canonical)
}

fn commit_group_digest(group: &LogicalCommitGroup) -> Result<String, ReviewPacketError> {
    let mut canonical = group.clone();
    canonical.group_sha256.clear();
    sha256_json(&canonical)
}

fn logical_commit_plan_digest(plan: &LogicalCommitPlan) -> Result<String, ReviewPacketError> {
    let mut canonical = plan.clone();
    canonical.plan_sha256.clear();
    sha256_json(&canonical)
}

fn review_packet_digest(packet: &LocalReviewPacket) -> Result<String, ReviewPacketError> {
    let mut canonical = packet.clone();
    canonical.packet_sha256.clear();
    sha256_json(&canonical)
}

fn sha256_json(value: &impl Serialize) -> Result<String, ReviewPacketError> {
    serde_json::to_vec(value)
        .map(|bytes| sha256_hex(&bytes))
        .map_err(|_| ReviewPacketError::ReceiptFailure)
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(64);
    for byte in Sha256::digest(bytes) {
        write!(&mut output, "{byte:02x}").expect("writing to String cannot fail");
    }
    output
}

fn valid_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b':' | b'-'))
}

fn valid_path(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 4_096
        && !value.starts_with('/')
        && !value.contains('\\')
        && !value.chars().any(char::is_control)
        && value
            .split('/')
            .all(|component| !component.is_empty() && !matches!(component, "." | ".."))
}

fn valid_text(value: &str) -> bool {
    !value.trim().is_empty()
        && value.len() <= 16 * 1024
        && !value
            .chars()
            .any(|character| character.is_control() && !matches!(character, '\n' | '\r' | '\t'))
}

fn valid_object_id(value: &str) -> bool {
    matches!(value.len(), 40 | 64)
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hash(character: char) -> String {
        character.to_string().repeat(64)
    }

    fn file(
        sequence: usize,
        path: &str,
        artifact_class: ReviewArtifactClass,
        purpose: CommitChangePurpose,
        generated: bool,
    ) -> ReviewChangedFile {
        ReviewChangedFile {
            operation_id: format!("operation-{sequence:02}"),
            path: path.to_owned(),
            preimage_sha256: hash('1'),
            postimage_sha256: hash(char::from_digit((sequence % 8 + 2) as u32, 10).unwrap()),
            artifact_class,
            purpose,
            generated,
            complete_diff: format!("-old-{sequence}\n+new-{sequence}"),
            operation_sha256: hash('a'),
        }
    }

    fn change_set() -> ReviewableChangeSet {
        let files = vec![
            file(
                1,
                "docs/guide.md",
                ReviewArtifactClass::Documentation,
                CommitChangePurpose::Documentation,
                false,
            ),
            file(
                2,
                "generated/api.json",
                ReviewArtifactClass::GeneratedOutput,
                CommitChangePurpose::GeneratedOutput,
                true,
            ),
            file(
                3,
                "migrations/001.sql",
                ReviewArtifactClass::Migration,
                CommitChangePurpose::Migration,
                false,
            ),
            file(
                4,
                "src/format.rs",
                ReviewArtifactClass::Code,
                CommitChangePurpose::Formatting,
                false,
            ),
            file(
                5,
                "src/lib.rs",
                ReviewArtifactClass::Code,
                CommitChangePurpose::Behavior,
                false,
            ),
            file(
                6,
                "tests/lib.rs",
                ReviewArtifactClass::Test,
                CommitChangePurpose::Tests,
                false,
            ),
        ];
        ReviewableChangeSet {
            change_set_id: "change-set-review-0001".to_owned(),
            change_set_sha256: hash('b'),
            intent_sha256: hash('c'),
            plan_sha256: hash('d'),
            write_approval_receipt_sha256: hash('e'),
            write_application_receipt_sha256: hash('f'),
            scope: WriteChangeScope::Minimal,
            review_hooks: vec![WriteReviewHook::Security],
            files_sha256: sha256_json(&files).expect("files digest"),
            files,
            write_authority: false,
        }
    }

    fn finding(
        identity: &str,
        confidence_bps: u16,
        severity: FindingSeverity,
    ) -> ReviewFindingInput {
        ReviewFindingInput {
            finding_id: identity.to_owned(),
            mode: ReviewMode::Correctness,
            code: "review.correctness.boundary".to_owned(),
            path: "src/lib.rs".to_owned(),
            line: 7,
            severity,
            confidence_bps,
            evidence_sha256: hash('1'),
            rationale_code: "review.impact.behavior".to_owned(),
        }
    }

    fn all_modes() -> Vec<ReviewMode> {
        vec![
            ReviewMode::Correctness,
            ReviewMode::Simplicity,
            ReviewMode::Maintainability,
            ReviewMode::Security,
            ReviewMode::DataIntegrity,
            ReviewMode::Accessibility,
            ReviewMode::Performance,
            ReviewMode::Tests,
            ReviewMode::Documentation,
        ]
    }

    #[test]
    fn all_review_modes_preserve_duplicate_and_low_confidence_evidence() {
        let mut low = finding("finding-low", 4_000, FindingSeverity::Medium);
        low.code = "review.correctness.low-confidence".to_owned();
        low.line = 8;
        let results = normalize_review_findings(
            all_modes(),
            vec![
                finding("finding-weaker", 8_000, FindingSeverity::Medium),
                finding("finding-winner", 9_000, FindingSeverity::High),
                low,
            ],
        )
        .expect("results");
        assert_eq!(results.modes.len(), 9);
        assert_eq!(results.findings.len(), 1);
        assert_eq!(results.findings[0].finding_id, "finding-winner");
        assert_eq!(results.suppressed.len(), 2);
        assert!(results.suppressed.iter().any(|entry| {
            entry.reason == FindingSuppressionReason::Duplicate
                && entry.superseded_by.as_deref() == Some("finding-winner")
        }));
        assert!(results.suppressed.iter().any(|entry| {
            entry.reason == FindingSuppressionReason::LowConfidence && entry.superseded_by.is_none()
        }));
        assert!(verify_review_results(&results));

        let mut forged = results;
        forged.suppressed[0].superseded_by = Some("missing-finding".to_owned());
        forged.results_sha256 = review_results_digest(&forged).expect("forged digest");
        assert!(!verify_review_results(&forged));
    }

    #[test]
    fn logical_commit_plan_accounts_for_every_approved_file_and_excludes_unrelated_work() {
        let change_set = change_set();
        assert!(verify_reviewable_change_set(&change_set));
        let unrelated = UnrelatedChangeEvidence {
            path_sha256: hash('2'),
            content_sha256: hash('3'),
            exclusion_code: "review.unrelated.user-work".to_owned(),
        };
        let plan =
            build_logical_commit_plan(&change_set, vec![unrelated.clone()]).expect("commit plan");
        assert_eq!(plan.groups.len(), 6);
        assert_eq!(plan.excluded_unrelated, [unrelated]);
        assert!(!plan.automatic_commit);
        assert!(!plan.push_authority);
        assert!(!plan.merge_authority);
        assert!(!plan.release_authority);
        assert!(verify_logical_commit_plan(&change_set, &plan));
        assert!(plan.groups.iter().all(|group| {
            group.proposed_message.contains("approved")
                && !group.proposed_message.contains("unrelated")
        }));

        let mut forged = plan;
        forged.groups[0]
            .operation_ids
            .push("operation-unrelated".to_owned());
        forged.groups[0].operation_ids.sort();
        forged.groups[0].paths.push("user/notes.md".to_owned());
        forged.groups[0].paths.sort();
        forged.groups[0].group_sha256 =
            commit_group_digest(&forged.groups[0]).expect("group digest");
        forged.plan_sha256 = logical_commit_plan_digest(&forged).expect("plan digest");
        assert!(!verify_logical_commit_plan(&change_set, &forged));
    }

    #[test]
    fn review_packet_is_stable_complete_and_has_no_commit_or_publication_authority() {
        let change_set = change_set();
        let commit_plan = build_logical_commit_plan(&change_set, Vec::new()).expect("plan");
        let review_results = normalize_review_findings(
            all_modes(),
            vec![finding("finding-high", 9_000, FindingSeverity::High)],
        )
        .expect("reviews");
        let packet = build_review_packet(ReviewPacketInput {
            packet_id: "review-packet-0001".to_owned(),
            repository_sha256: hash('4'),
            base_object: "5".repeat(40),
            candidate_snapshot_sha256: hash('6'),
            preservation_manifest_sha256: hash('7'),
            objective: "Apply the exact approved fixture change.".to_owned(),
            behavior_delta: "The fixture returns the new exact value.".to_owned(),
            change_set,
            validations: Vec::new(),
            checks_not_run: vec![ValidationKind::EndToEnd, ValidationKind::Security],
            review_results,
            evidence_artifacts: vec![ReviewEvidenceArtifact {
                artifact_id: "screenshot-0001".to_owned(),
                kind: ReviewEvidenceArtifactKind::Screenshot,
                sha256: hash('8'),
                byte_length: 100,
                sensitive_content_detected: false,
            }],
            commit_plan,
            rollback: "Restore every exact preimage.".to_owned(),
        })
        .expect("packet");
        assert!(verify_review_packet(&packet));
        assert_eq!(packet.blocking_finding_count, 1);
        assert!(!packet.commit_authority);
        assert!(!packet.publication_authority);

        let mut changed = packet.clone();
        changed.change_set.files[0]
            .complete_diff
            .push_str("\n+hidden");
        changed.packet_sha256 = review_packet_digest(&changed).expect("packet digest");
        assert!(!verify_review_packet(&changed));

        let mut sensitive = packet;
        sensitive.evidence_artifacts[0].sensitive_content_detected = true;
        sensitive.packet_sha256 = review_packet_digest(&sensitive).expect("packet digest");
        assert!(!verify_review_packet(&sensitive));
    }

    #[test]
    fn purpose_and_artifact_matrix_rejects_mislabeled_generated_and_migration_work() {
        assert!(purpose_matches_artifact(
            CommitChangePurpose::Behavior,
            ReviewArtifactClass::Code,
            false
        ));
        assert!(purpose_matches_artifact(
            CommitChangePurpose::GeneratedOutput,
            ReviewArtifactClass::GeneratedOutput,
            true
        ));
        assert!(!purpose_matches_artifact(
            CommitChangePurpose::Behavior,
            ReviewArtifactClass::GeneratedOutput,
            true
        ));
        assert!(!purpose_matches_artifact(
            CommitChangePurpose::Migration,
            ReviewArtifactClass::Code,
            false
        ));
    }
}
