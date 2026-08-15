//! Reproduction, hypothesis, alternative, review, and validation-gated change plans.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{
    ChangeImpactState, ChangeImpactSurfaceKind, ChangeIntentRecord, ChangeIntentStatus,
    ChangeRiskDomain, DeepRepositoryIndex, MinimalChangeImpactReport, RepositoryFactKind,
    deep_analysis::verify_deep_index_integrity, verify_change_intent, verify_minimal_change_impact,
};

const CHANGE_PLAN_SCHEMA_VERSION: u16 = 1;
const MAX_ITEMS: usize = 256;
const MAX_TEXT_BYTES: usize = 4_096;

/// Closed kind of proposed work.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChangeWorkKind {
    /// New or changed user-visible behavior.
    Feature,
    /// A defect investigation and fix.
    Defect,
    /// Behavior-preserving structural change.
    Refactor,
    /// Data or compatibility migration.
    Migration,
    /// Dependency change.
    Dependency,
    /// Documentation-only change.
    Documentation,
}

/// Closed reproduction step classes; none grants mutation authority.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReproductionStepKind {
    /// Inspect one already authorized input.
    InspectInput,
    /// Run one separately registered deterministic test.
    RunRegisteredTest,
    /// Observe one bounded result.
    ObserveResult,
    /// Observe one bounded log artifact.
    ObserveLog,
    /// Compare exact result identities.
    CompareResult,
}

/// One ordered reproduction step.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReproductionStep {
    /// One-based stable sequence.
    pub sequence: u16,
    /// Closed step class.
    pub kind: ReproductionStepKind,
    /// Bounded descriptive action.
    pub description: String,
    /// Exact supporting fact identities.
    pub evidence_fact_ids: Vec<String>,
    /// A later execution requires its own separate grant.
    pub separate_grant_required: bool,
    /// A reproduction step cannot inherit write authority.
    pub write_authority: bool,
}

/// Terminal execution observation for one reproduction attempt.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReproductionExecutionStatus {
    /// Every declared step completed.
    Completed,
    /// Policy denied one step.
    Denied,
    /// User or system cancellation stopped the attempt.
    Cancelled,
    /// A fixed timeout stopped the attempt.
    TimedOut,
    /// A dependency failed before an observed result existed.
    DependencyFailed,
}

/// Truthful reproduction result.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReproductionOutcome {
    /// Exact expected failure signature was observed.
    Reproduced,
    /// A completed run did not produce the expected failure signature.
    NotReproduced,
    /// The attempt ended without sufficient evidence.
    Inconclusive,
    /// Reproduction was not attempted because it was unsafe.
    UnsafeToReproduce,
}

/// Unsealed reproduction input.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReproductionInput {
    /// Stable reproduction identity.
    pub reproduction_id: String,
    /// Exact environment identity.
    pub environment_sha256: String,
    /// Exact input-set identity.
    pub inputs_sha256: String,
    /// Stable ordered steps.
    pub steps: Vec<ReproductionStep>,
    /// Exact expected-result identity.
    pub expected_result_sha256: String,
    /// Exact observed-result identity when available.
    pub observed_result_sha256: Option<String>,
    /// Stable exact log artifact identities.
    pub log_sha256s: Vec<String>,
    /// Whether the exact predeclared failure signature was observed.
    pub failure_signature_observed: bool,
    /// Terminal execution state.
    pub execution_status: ReproductionExecutionStatus,
    /// Required bounded rationale when reproduction is unsafe.
    pub unsafe_reason: Option<String>,
}

/// Immutable reproduction record.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReproductionRecord {
    /// Schema version.
    pub schema_version: u16,
    /// Exact deep-index identity.
    pub index_sha256: String,
    /// Exact normalized input.
    pub input: ReproductionInput,
    /// Deterministically derived outcome.
    pub outcome: ReproductionOutcome,
    /// No root cause is established by reproduction alone.
    pub root_cause_proven: bool,
    /// Reproduction records grant no mutation authority.
    pub mutation_authority: bool,
    /// SHA-256 over every preceding field.
    pub reproduction_sha256: String,
}

/// Result of one discriminating hypothesis check.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HypothesisCheckResult {
    /// Exact evidence supports the predicted distinction.
    Passed,
    /// Exact evidence rejects the prediction.
    Failed,
    /// Evidence was absent or ambiguous.
    Inconclusive,
}

/// One discriminating check for one hypothesis.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HypothesisCheck {
    /// Stable check identity.
    pub check_id: String,
    /// Bounded predeclared check.
    pub description: String,
    /// Stable supporting fact identities.
    pub evidence_fact_ids: Vec<String>,
    /// Exact log identities used for correlation.
    pub log_sha256s: Vec<String>,
    /// Observed result.
    pub result: HypothesisCheckResult,
}

/// Explicit hypothesis disposition.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HypothesisStatus {
    /// Proposed but not discriminated.
    Proposed,
    /// Every discriminating check passed against a reproduced defect.
    Supported,
    /// At least one discriminating check failed.
    Rejected,
}

/// One competing explanation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChangeHypothesis {
    /// Stable hypothesis identity.
    pub hypothesis_id: String,
    /// Bounded explanation.
    pub explanation: String,
    /// Stable discriminating checks.
    pub checks: Vec<HypothesisCheck>,
    /// Explicit disposition.
    pub status: HypothesisStatus,
    /// Required rationale for a rejected explanation.
    pub rejection_reason: Option<String>,
}

/// Complete competing-hypothesis and state-trace record.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HypothesisRecord {
    /// Stable ordered competing hypotheses.
    pub hypotheses: Vec<ChangeHypothesis>,
    /// Exact selected supported hypothesis, if one exists.
    pub selected_hypothesis_id: Option<String>,
    /// Exact facts used for state tracing.
    pub state_trace_fact_ids: Vec<String>,
    /// Exact correlated log identities.
    pub correlated_log_sha256s: Vec<String>,
    /// SHA-256 over every preceding field.
    pub hypothesis_sha256: String,
}

/// Regression-test disposition before a defect fix.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RegressionTestDisposition {
    /// A current cited failing test exists.
    FailingTestRetained,
    /// A failing test must be created and observed before implementation.
    RequiredBeforeFix,
    /// A failing test is unsafe, with explicit rationale.
    UnsafeWithRationale,
    /// A failing test is infeasible, with explicit rationale.
    InfeasibleWithRationale,
    /// Work is not a defect fix.
    NotDefect,
}

/// Exact regression-test planning record.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RegressionTestPlan {
    /// Explicit disposition.
    pub disposition: RegressionTestDisposition,
    /// Exact existing test facts where applicable.
    pub test_fact_ids: Vec<String>,
    /// Required rationale for unsafe, infeasible, or not-defect states.
    pub rationale: Option<String>,
    /// SHA-256 over every preceding field.
    pub regression_sha256: String,
}

/// Closed decision dimension requiring alternatives.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChangeAlternativeDimension {
    /// Architecture choice.
    Architecture,
    /// Dependency choice.
    Dependency,
    /// Access or authority choice.
    Access,
    /// Resource or operating-cost choice.
    Cost,
    /// Irreversibility choice.
    Irreversibility,
}

impl ChangeAlternativeDimension {
    /// Complete stable alternative inventory.
    pub const ALL: [Self; 5] = [
        Self::Architecture,
        Self::Dependency,
        Self::Access,
        Self::Cost,
        Self::Irreversibility,
    ];
}

/// One bounded alternative option.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChangeAlternativeOption {
    /// Stable option identity.
    pub option_id: String,
    /// Bounded option description.
    pub description: String,
    /// Bounded stable tradeoffs.
    pub tradeoffs: Vec<String>,
    /// Exact supporting fact identities.
    pub evidence_fact_ids: Vec<String>,
    /// Whether selecting this option creates an irreversible effect.
    pub irreversible: bool,
}

/// Competing alternatives and one explicit selection.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChangeAlternativeSet {
    /// Closed decision dimension.
    pub dimension: ChangeAlternativeDimension,
    /// At least two stable options.
    pub options: Vec<ChangeAlternativeOption>,
    /// Exact selected option identity.
    pub selected_option_id: String,
    /// Stable decision-record identity.
    pub decision_id: String,
    /// Bounded decision rationale.
    pub rationale: String,
    /// SHA-256 over every preceding field.
    pub alternative_sha256: String,
}

/// Reviews selected deterministically from risks and affected surfaces.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChangeReviewKind {
    /// Security review.
    Security,
    /// Privacy review.
    Privacy,
    /// Data review.
    Data,
    /// Accessibility review.
    Accessibility,
    /// Performance review.
    Performance,
    /// Migration review.
    Migration,
    /// Rollback and recovery review.
    Rollback,
}

/// Closed validation classes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlannedValidationKind {
    /// Unit tests.
    Unit,
    /// Integration tests.
    Integration,
    /// Security checks.
    Security,
    /// Privacy checks.
    Privacy,
    /// Accessibility checks.
    Accessibility,
    /// Performance checks.
    Performance,
    /// Migration checks.
    Migration,
    /// Rollback checks.
    Rollback,
}

/// One descriptive validation step requiring separate later authority.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlannedValidation {
    /// Stable validation identity.
    pub validation_id: String,
    /// Closed validation class.
    pub kind: PlannedValidationKind,
    /// Bounded expected check.
    pub description: String,
    /// Exact acceptance checks this validation covers.
    pub acceptance_checks: Vec<String>,
    /// Later execution requires a separate exact grant.
    pub separate_grant_required: bool,
    /// Validation planning cannot inherit write authority.
    pub write_authority: bool,
}

/// Input for one complete descriptive change plan.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChangePlanInput {
    /// Stable plan identity.
    pub plan_id: String,
    /// Closed work class.
    pub work_kind: ChangeWorkKind,
    /// Optional exact reproduction record.
    pub reproduction: Option<ReproductionRecord>,
    /// Regression-test gate.
    pub regression_test: RegressionTestPlan,
    /// Competing hypotheses and selected explanation.
    pub hypotheses: HypothesisRecord,
    /// Exactly one alternative set per closed decision dimension.
    pub alternatives: Vec<ChangeAlternativeSet>,
    /// Stable separately grantable validation steps.
    pub validations: Vec<PlannedValidation>,
}

/// Explicit change-plan readiness.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChangePlanStatus {
    /// Material intent ambiguity remains.
    ClarificationRequired,
    /// Impact includes an unassessed surface.
    ImpactReviewRequired,
    /// A defect has not been reproduced safely and exactly.
    ReproductionRequired,
    /// A safe feasible failing regression test is still required.
    RegressionTestRequired,
    /// A reproduced defect lacks one supported selected hypothesis.
    HypothesisRequired,
    /// The smallest current plan is internally reviewable, not authorized.
    ReadyForImplementationReview,
}

/// Complete immutable change plan.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChangePlanRecord {
    /// Schema version.
    pub schema_version: u16,
    /// Exact deep-index identity.
    pub index_sha256: String,
    /// Exact intent identity.
    pub intent_sha256: String,
    /// Exact minimal-impact identity.
    pub impact_sha256: String,
    /// Exact normalized planning input.
    pub input: ChangePlanInput,
    /// Deterministically required reviews.
    pub required_reviews: Vec<ChangeReviewKind>,
    /// Explicit readiness.
    pub status: ChangePlanStatus,
    /// Planning never grants mutation authority.
    pub mutation_authority: bool,
    /// SHA-256 over every preceding field.
    pub plan_sha256: String,
}

/// Content-free change-plan failure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChangePlanError {
    /// Index, intent, or impact is invalid.
    InvalidSource,
    /// Reproduction input or record is invalid.
    ReproductionInvalid,
    /// Hypothesis/check state is invalid.
    HypothesisInvalid,
    /// Regression-test state is invalid.
    RegressionInvalid,
    /// Alternative or decision state is invalid.
    AlternativeInvalid,
    /// Validation step is invalid.
    ValidationInvalid,
}

impl ChangePlanError {
    /// Stable content-free error code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidSource => "change.plan.source_invalid",
            Self::ReproductionInvalid => "change.plan.reproduction_invalid",
            Self::HypothesisInvalid => "change.plan.hypothesis_invalid",
            Self::RegressionInvalid => "change.plan.regression_invalid",
            Self::AlternativeInvalid => "change.plan.alternative_invalid",
            Self::ValidationInvalid => "change.plan.validation_invalid",
        }
    }
}

/// Seals one truthful reproduction record.
pub fn seal_reproduction(
    index: &DeepRepositoryIndex,
    input: ReproductionInput,
) -> Result<ReproductionRecord, ChangePlanError> {
    if !verify_deep_index_integrity(index) || !valid_reproduction_input(index, &input) {
        return Err(ChangePlanError::ReproductionInvalid);
    }
    let outcome = reproduction_outcome(&input)?;
    let mut record = ReproductionRecord {
        schema_version: CHANGE_PLAN_SCHEMA_VERSION,
        index_sha256: index.index_sha256.clone(),
        input,
        outcome,
        root_cause_proven: false,
        mutation_authority: false,
        reproduction_sha256: String::new(),
    };
    record.reproduction_sha256 = reproduction_digest(&record);
    Ok(record)
}

/// Verifies one reproduction record by exact recomputation.
#[must_use]
pub fn verify_reproduction(index: &DeepRepositoryIndex, record: &ReproductionRecord) -> bool {
    !record.root_cause_proven
        && !record.mutation_authority
        && record.reproduction_sha256 == reproduction_digest(record)
        && seal_reproduction(index, record.input.clone()).is_ok_and(|expected| expected == *record)
}

/// Seals hypotheses, state tracing, and rejected explanations.
pub fn seal_hypotheses(
    index: &DeepRepositoryIndex,
    reproduction: Option<&ReproductionRecord>,
    mut record: HypothesisRecord,
) -> Result<HypothesisRecord, ChangePlanError> {
    if !verify_deep_index_integrity(index)
        || reproduction.is_some_and(|item| !verify_reproduction(index, item))
    {
        return Err(ChangePlanError::HypothesisInvalid);
    }
    record.hypothesis_sha256.clear();
    validate_hypotheses(index, reproduction, &record)?;
    record.hypothesis_sha256 = hypothesis_digest(&record);
    Ok(record)
}

/// Seals one regression-test gate.
pub fn seal_regression_test_plan(
    index: &DeepRepositoryIndex,
    mut record: RegressionTestPlan,
) -> Result<RegressionTestPlan, ChangePlanError> {
    record.regression_sha256.clear();
    let facts = index
        .facts
        .iter()
        .map(|fact| (fact.fact_id.as_str(), fact.kind))
        .collect::<BTreeMap<_, _>>();
    if !record
        .test_fact_ids
        .windows(2)
        .all(|pair| pair[0] < pair[1])
        || record
            .test_fact_ids
            .iter()
            .any(|identity| facts.get(identity.as_str()) != Some(&RepositoryFactKind::Test))
        || match record.disposition {
            RegressionTestDisposition::FailingTestRetained => {
                record.test_fact_ids.is_empty() || record.rationale.is_some()
            }
            RegressionTestDisposition::RequiredBeforeFix => {
                !record.test_fact_ids.is_empty() || record.rationale.is_some()
            }
            RegressionTestDisposition::UnsafeWithRationale
            | RegressionTestDisposition::InfeasibleWithRationale
            | RegressionTestDisposition::NotDefect => {
                !record.test_fact_ids.is_empty()
                    || record
                        .rationale
                        .as_deref()
                        .is_none_or(|value| !valid_text(value))
            }
        }
    {
        return Err(ChangePlanError::RegressionInvalid);
    }
    record.regression_sha256 = regression_digest(&record);
    Ok(record)
}

/// Builds one complete descriptive plan and derives its truthful readiness.
pub fn build_change_plan(
    index: &DeepRepositoryIndex,
    intent: &ChangeIntentRecord,
    impact: &MinimalChangeImpactReport,
    mut input: ChangePlanInput,
) -> Result<ChangePlanRecord, ChangePlanError> {
    if !verify_deep_index_integrity(index)
        || !verify_change_intent(index, intent)
        || !verify_minimal_change_impact(index, intent, impact)
        || !valid_identifier(&input.plan_id)
    {
        return Err(ChangePlanError::InvalidSource);
    }
    if input
        .reproduction
        .as_ref()
        .is_some_and(|record| !verify_reproduction(index, record))
    {
        return Err(ChangePlanError::ReproductionInvalid);
    }
    input.regression_test = seal_regression_test_plan(index, input.regression_test)?;
    input.hypotheses = seal_hypotheses(index, input.reproduction.as_ref(), input.hypotheses)?;
    validate_alternatives(index, &mut input.alternatives)?;
    validate_validations(intent, &input.validations)?;
    validate_work_kind(input.work_kind, &input)?;
    let required_reviews = required_reviews(intent, impact);
    let status = derive_plan_status(intent, impact, &input);
    let mut record = ChangePlanRecord {
        schema_version: CHANGE_PLAN_SCHEMA_VERSION,
        index_sha256: index.index_sha256.clone(),
        intent_sha256: intent.intent_sha256.clone(),
        impact_sha256: impact.impact_sha256.clone(),
        input,
        required_reviews,
        status,
        mutation_authority: false,
        plan_sha256: String::new(),
    };
    record.plan_sha256 = plan_digest(&record);
    Ok(record)
}

/// Verifies one plan by exact deterministic recomputation.
#[must_use]
pub fn verify_change_plan(
    index: &DeepRepositoryIndex,
    intent: &ChangeIntentRecord,
    impact: &MinimalChangeImpactReport,
    record: &ChangePlanRecord,
) -> bool {
    !record.mutation_authority
        && record.plan_sha256 == plan_digest(record)
        && build_change_plan(index, intent, impact, record.input.clone())
            .is_ok_and(|expected| expected == *record)
}

fn valid_reproduction_input(index: &DeepRepositoryIndex, input: &ReproductionInput) -> bool {
    let fact_ids = index
        .facts
        .iter()
        .map(|fact| fact.fact_id.as_str())
        .collect::<BTreeSet<_>>();
    valid_identifier(&input.reproduction_id)
        && is_sha256(&input.environment_sha256)
        && is_sha256(&input.inputs_sha256)
        && !input.steps.is_empty()
        && input.steps.len() <= MAX_ITEMS
        && input.steps.iter().enumerate().all(|(index, step)| {
            step.sequence as usize == index + 1
                && valid_text(&step.description)
                && valid_sorted_hashes(&step.evidence_fact_ids, true)
                && step
                    .evidence_fact_ids
                    .iter()
                    .all(|identity| fact_ids.contains(identity.as_str()))
                && step.separate_grant_required
                && !step.write_authority
        })
        && is_sha256(&input.expected_result_sha256)
        && input
            .observed_result_sha256
            .as_deref()
            .is_none_or(is_sha256)
        && valid_sorted_hashes(&input.log_sha256s, true)
        && input.unsafe_reason.as_deref().is_none_or(valid_text)
}

fn reproduction_outcome(input: &ReproductionInput) -> Result<ReproductionOutcome, ChangePlanError> {
    if input.unsafe_reason.is_some() {
        if input.execution_status != ReproductionExecutionStatus::Denied
            || input.observed_result_sha256.is_some()
            || input.failure_signature_observed
        {
            return Err(ChangePlanError::ReproductionInvalid);
        }
        return Ok(ReproductionOutcome::UnsafeToReproduce);
    }
    if input.execution_status != ReproductionExecutionStatus::Completed {
        if input.failure_signature_observed {
            return Err(ChangePlanError::ReproductionInvalid);
        }
        return Ok(ReproductionOutcome::Inconclusive);
    }
    let Some(observed) = input.observed_result_sha256.as_deref() else {
        return Err(ChangePlanError::ReproductionInvalid);
    };
    if input.failure_signature_observed {
        if observed == input.expected_result_sha256 {
            return Err(ChangePlanError::ReproductionInvalid);
        }
        Ok(ReproductionOutcome::Reproduced)
    } else {
        Ok(ReproductionOutcome::NotReproduced)
    }
}

fn validate_hypotheses(
    index: &DeepRepositoryIndex,
    reproduction: Option<&ReproductionRecord>,
    record: &HypothesisRecord,
) -> Result<(), ChangePlanError> {
    let fact_ids = index
        .facts
        .iter()
        .map(|fact| fact.fact_id.as_str())
        .collect::<BTreeSet<_>>();
    if record.hypotheses.len() > MAX_ITEMS
        || record
            .hypotheses
            .windows(2)
            .any(|pair| pair[0].hypothesis_id >= pair[1].hypothesis_id)
        || !valid_sorted_hashes(&record.state_trace_fact_ids, true)
        || record
            .state_trace_fact_ids
            .iter()
            .any(|identity| !fact_ids.contains(identity.as_str()))
        || !valid_sorted_hashes(&record.correlated_log_sha256s, true)
    {
        return Err(ChangePlanError::HypothesisInvalid);
    }
    let mut supported = Vec::new();
    for hypothesis in &record.hypotheses {
        if !valid_identifier(&hypothesis.hypothesis_id)
            || !valid_text(&hypothesis.explanation)
            || hypothesis.checks.is_empty()
            || hypothesis.checks.len() > MAX_ITEMS
            || hypothesis
                .checks
                .windows(2)
                .any(|pair| pair[0].check_id >= pair[1].check_id)
        {
            return Err(ChangePlanError::HypothesisInvalid);
        }
        for check in &hypothesis.checks {
            if !valid_identifier(&check.check_id)
                || !valid_text(&check.description)
                || !valid_sorted_hashes(&check.evidence_fact_ids, true)
                || check
                    .evidence_fact_ids
                    .iter()
                    .any(|identity| !fact_ids.contains(identity.as_str()))
                || !valid_sorted_hashes(&check.log_sha256s, true)
            {
                return Err(ChangePlanError::HypothesisInvalid);
            }
        }
        match hypothesis.status {
            HypothesisStatus::Supported => {
                if reproduction.map(|item| item.outcome) != Some(ReproductionOutcome::Reproduced)
                    || hypothesis
                        .checks
                        .iter()
                        .any(|check| check.result != HypothesisCheckResult::Passed)
                    || hypothesis.checks.iter().all(|check| {
                        check.evidence_fact_ids.is_empty() && check.log_sha256s.is_empty()
                    })
                    || hypothesis.rejection_reason.is_some()
                {
                    return Err(ChangePlanError::HypothesisInvalid);
                }
                supported.push(hypothesis.hypothesis_id.as_str());
            }
            HypothesisStatus::Rejected => {
                if !hypothesis
                    .checks
                    .iter()
                    .any(|check| check.result == HypothesisCheckResult::Failed)
                    || hypothesis
                        .rejection_reason
                        .as_deref()
                        .is_none_or(|reason| !valid_text(reason))
                {
                    return Err(ChangePlanError::HypothesisInvalid);
                }
            }
            HypothesisStatus::Proposed => {
                if hypothesis.rejection_reason.is_some()
                    || hypothesis
                        .checks
                        .iter()
                        .all(|check| check.result != HypothesisCheckResult::Inconclusive)
                {
                    return Err(ChangePlanError::HypothesisInvalid);
                }
            }
        }
    }
    if record
        .selected_hypothesis_id
        .as_deref()
        .is_some_and(|selected| {
            supported.len() != 1 || supported.first().copied() != Some(selected)
        })
        || (record.selected_hypothesis_id.is_none() && !supported.is_empty())
    {
        return Err(ChangePlanError::HypothesisInvalid);
    }
    Ok(())
}

fn validate_alternatives(
    index: &DeepRepositoryIndex,
    alternatives: &mut [ChangeAlternativeSet],
) -> Result<(), ChangePlanError> {
    alternatives.sort_by_key(|alternative| alternative.dimension);
    if alternatives.len() != ChangeAlternativeDimension::ALL.len()
        || alternatives
            .iter()
            .map(|alternative| alternative.dimension)
            .ne(ChangeAlternativeDimension::ALL)
    {
        return Err(ChangePlanError::AlternativeInvalid);
    }
    let fact_ids = index
        .facts
        .iter()
        .map(|fact| fact.fact_id.as_str())
        .collect::<BTreeSet<_>>();
    for alternative in alternatives {
        alternative.alternative_sha256.clear();
        if alternative.options.len() < 2
            || alternative.options.len() > MAX_ITEMS
            || alternative
                .options
                .windows(2)
                .any(|pair| pair[0].option_id >= pair[1].option_id)
            || !valid_identifier(&alternative.selected_option_id)
            || !valid_identifier(&alternative.decision_id)
            || !valid_text(&alternative.rationale)
            || alternative
                .options
                .iter()
                .filter(|option| option.option_id == alternative.selected_option_id)
                .count()
                != 1
        {
            return Err(ChangePlanError::AlternativeInvalid);
        }
        for option in &alternative.options {
            if !valid_identifier(&option.option_id)
                || !valid_text(&option.description)
                || !valid_sorted_text(&option.tradeoffs, false)
                || !valid_sorted_hashes(&option.evidence_fact_ids, true)
                || option
                    .evidence_fact_ids
                    .iter()
                    .any(|identity| !fact_ids.contains(identity.as_str()))
            {
                return Err(ChangePlanError::AlternativeInvalid);
            }
        }
        alternative.alternative_sha256 = alternative_digest(alternative);
    }
    Ok(())
}

fn validate_validations(
    intent: &ChangeIntentRecord,
    validations: &[PlannedValidation],
) -> Result<(), ChangePlanError> {
    if validations.is_empty()
        || validations.len() > MAX_ITEMS
        || validations
            .windows(2)
            .any(|pair| pair[0].validation_id >= pair[1].validation_id)
        || validations.iter().any(|validation| {
            !valid_identifier(&validation.validation_id)
                || !valid_text(&validation.description)
                || !valid_sorted_text(&validation.acceptance_checks, false)
                || validation
                    .acceptance_checks
                    .iter()
                    .any(|check| intent.input.acceptance_checks.binary_search(check).is_err())
                || !validation.separate_grant_required
                || validation.write_authority
        })
    {
        return Err(ChangePlanError::ValidationInvalid);
    }
    let covered = validations
        .iter()
        .flat_map(|validation| validation.acceptance_checks.iter())
        .collect::<BTreeSet<_>>();
    if intent
        .input
        .acceptance_checks
        .iter()
        .any(|check| !covered.contains(check))
    {
        return Err(ChangePlanError::ValidationInvalid);
    }
    Ok(())
}

fn validate_work_kind(
    work_kind: ChangeWorkKind,
    input: &ChangePlanInput,
) -> Result<(), ChangePlanError> {
    if work_kind == ChangeWorkKind::Defect {
        if input.regression_test.disposition == RegressionTestDisposition::NotDefect {
            return Err(ChangePlanError::RegressionInvalid);
        }
    } else if input.regression_test.disposition != RegressionTestDisposition::NotDefect {
        return Err(ChangePlanError::RegressionInvalid);
    }
    Ok(())
}

fn required_reviews(
    intent: &ChangeIntentRecord,
    impact: &MinimalChangeImpactReport,
) -> Vec<ChangeReviewKind> {
    let mut reviews = BTreeSet::from([ChangeReviewKind::Rollback]);
    for risk in &intent.input.risks {
        reviews.insert(match risk {
            ChangeRiskDomain::Security | ChangeRiskDomain::Dependency => ChangeReviewKind::Security,
            ChangeRiskDomain::Privacy => ChangeReviewKind::Privacy,
            ChangeRiskDomain::Data => ChangeReviewKind::Data,
            ChangeRiskDomain::Accessibility => ChangeReviewKind::Accessibility,
            ChangeRiskDomain::Performance => ChangeReviewKind::Performance,
            ChangeRiskDomain::Migration => ChangeReviewKind::Migration,
            ChangeRiskDomain::Reliability => ChangeReviewKind::Rollback,
        });
    }
    for surface in &impact.surfaces {
        if surface.state != ChangeImpactState::EvidenceBacked {
            continue;
        }
        match surface.kind {
            ChangeImpactSurfaceKind::Permissions | ChangeImpactSurfaceKind::Dependencies => {
                reviews.insert(ChangeReviewKind::Security);
            }
            ChangeImpactSurfaceKind::Data => {
                reviews.insert(ChangeReviewKind::Data);
            }
            ChangeImpactSurfaceKind::Migrations => {
                reviews.insert(ChangeReviewKind::Migration);
            }
            _ => {}
        }
    }
    reviews.into_iter().collect()
}

fn derive_plan_status(
    intent: &ChangeIntentRecord,
    impact: &MinimalChangeImpactReport,
    input: &ChangePlanInput,
) -> ChangePlanStatus {
    if intent.status == ChangeIntentStatus::ClarificationRequired {
        return ChangePlanStatus::ClarificationRequired;
    }
    if !impact.complete {
        return ChangePlanStatus::ImpactReviewRequired;
    }
    if input.work_kind == ChangeWorkKind::Defect {
        let Some(reproduction) = &input.reproduction else {
            return ChangePlanStatus::ReproductionRequired;
        };
        if !matches!(
            reproduction.outcome,
            ReproductionOutcome::Reproduced | ReproductionOutcome::UnsafeToReproduce
        ) {
            return ChangePlanStatus::ReproductionRequired;
        }
        if input.regression_test.disposition == RegressionTestDisposition::RequiredBeforeFix {
            return ChangePlanStatus::RegressionTestRequired;
        }
        if reproduction.outcome == ReproductionOutcome::Reproduced
            && input.hypotheses.selected_hypothesis_id.is_none()
        {
            return ChangePlanStatus::HypothesisRequired;
        }
    }
    ChangePlanStatus::ReadyForImplementationReview
}

fn valid_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || "-._:".contains(character))
}

fn valid_text(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_TEXT_BYTES
        && !value.chars().any(|character| character.is_control())
}

fn valid_sorted_text(values: &[String], allow_empty: bool) -> bool {
    (allow_empty || !values.is_empty())
        && values.len() <= MAX_ITEMS
        && values.iter().all(|value| valid_text(value))
        && values.windows(2).all(|pair| pair[0] < pair[1])
}

fn valid_sorted_hashes(values: &[String], allow_empty: bool) -> bool {
    (allow_empty || !values.is_empty())
        && values.len() <= MAX_ITEMS
        && values.iter().all(|value| is_sha256(value))
        && values.windows(2).all(|pair| pair[0] < pair[1])
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn reproduction_digest(record: &ReproductionRecord) -> String {
    sha256_json(&(
        record.schema_version,
        &record.index_sha256,
        &record.input,
        record.outcome,
        record.root_cause_proven,
        record.mutation_authority,
    ))
}

fn hypothesis_digest(record: &HypothesisRecord) -> String {
    sha256_json(&(
        &record.hypotheses,
        &record.selected_hypothesis_id,
        &record.state_trace_fact_ids,
        &record.correlated_log_sha256s,
    ))
}

fn regression_digest(record: &RegressionTestPlan) -> String {
    sha256_json(&(record.disposition, &record.test_fact_ids, &record.rationale))
}

fn alternative_digest(record: &ChangeAlternativeSet) -> String {
    sha256_json(&(
        record.dimension,
        &record.options,
        &record.selected_option_id,
        &record.decision_id,
        &record.rationale,
    ))
}

fn plan_digest(record: &ChangePlanRecord) -> String {
    sha256_json(&(
        record.schema_version,
        &record.index_sha256,
        &record.intent_sha256,
        &record.impact_sha256,
        &record.input,
        &record.required_reviews,
        record.status,
        record.mutation_authority,
    ))
}

fn sha256_json<T: Serialize>(value: &T) -> String {
    serde_json::to_vec(value)
        .map(|bytes| sha256_hex(&bytes))
        .unwrap_or_else(|_| sha256_hex(b"change-plan-serialization-failed"))
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(64);
    for byte in Sha256::digest(bytes) {
        write!(&mut output, "{byte:02x}").expect("writing to String cannot fail");
    }
    output
}

#[cfg(test)]
mod tests {
    use agentmage_kernel_contracts::WorkspaceId;

    use super::*;
    use crate::{
        ChangeImpactSurfaceKind, ChangeIntentInput, ChangeSurfaceExclusion, GitTrackedState,
        RepositoryFileInput, RepositoryMapInput, build_deep_repository_index,
        build_minimal_change_impact, build_repository_map, normalize_change_intent,
    };

    fn source(path: &[&str], content: &str) -> RepositoryFileInput {
        RepositoryFileInput {
            path: path.iter().map(|value| (*value).to_owned()).collect(),
            size_bytes: content.len() as u64,
            content_sha256: sha256_hex(content.as_bytes()),
            content: Some(content.as_bytes().to_vec()),
            git_state: GitTrackedState::TrackedClean,
            policy_excluded: false,
            generated: false,
            vendored: false,
        }
    }

    fn fixture() -> (
        DeepRepositoryIndex,
        ChangeIntentRecord,
        MinimalChangeImpactReport,
        String,
    ) {
        let map = build_repository_map(RepositoryMapInput {
            workspace_id: WorkspaceId::from_raw("workspace-change-plan"),
            repository_sha256: "a".repeat(64),
            worktree_sha256: "b".repeat(64),
            branch: Some("agentmage/tasks/change-plan".to_owned()),
            commit_id: "c".repeat(40),
            policy_sha256: "d".repeat(64),
            freshness_sha256: "e".repeat(64),
            files: vec![
                source(&["src", "lib.rs"], "pub fn calculate() -> u64 { 1 }\n"),
                source(
                    &["tests", "calculate_test.rs"],
                    "fn calculation_fails() {}\n",
                ),
            ],
        })
        .expect("map");
        let index = build_deep_repository_index(&map, Vec::new(), Vec::new()).expect("index");
        let target = index
            .facts
            .iter()
            .find(|fact| fact.label == "calculate")
            .expect("target")
            .fact_id
            .clone();
        let test = index
            .facts
            .iter()
            .find(|fact| fact.kind == RepositoryFactKind::Test)
            .expect("test")
            .fact_id
            .clone();
        let mut targets = vec![target];
        targets.push(test.clone());
        targets.sort();
        let observed_surfaces = [
            ChangeImpactSurfaceKind::Files,
            ChangeImpactSurfaceKind::Tests,
        ];
        let mut exclusions = ChangeImpactSurfaceKind::ALL
            .into_iter()
            .filter(|surface| !observed_surfaces.contains(surface))
            .map(|surface| ChangeSurfaceExclusion {
                surface,
                rationale: "Fixture establishes no change in this surface".to_owned(),
            })
            .collect::<Vec<_>>();
        exclusions.sort_by_key(|item| item.surface);
        let intent = normalize_change_intent(
            &index,
            ChangeIntentInput {
                intent_id: "intent-defect".to_owned(),
                requested_behavior: "Return the expected fixture value".to_owned(),
                current_behavior: "The cited function returns the failing fixture value".to_owned(),
                current_behavior_fact_ids: targets.clone(),
                target_fact_ids: targets,
                users: vec!["library callers".to_owned()],
                acceptance_checks: vec!["The exact regression fixture passes".to_owned()],
                exclusions: vec!["No interface change".to_owned()],
                risks: vec![ChangeRiskDomain::Reliability],
                rollback: "Restore the exact preimage".to_owned(),
                rollback_reversible: true,
                surface_exclusions: exclusions,
                clarifications: Vec::new(),
            },
        )
        .expect("intent");
        let impact = build_minimal_change_impact(&index, &intent).expect("impact");
        (index, intent, impact, test)
    }

    fn reproduction(index: &DeepRepositoryIndex) -> ReproductionRecord {
        seal_reproduction(
            index,
            ReproductionInput {
                reproduction_id: "reproduction-defect".to_owned(),
                environment_sha256: "1".repeat(64),
                inputs_sha256: "2".repeat(64),
                steps: vec![ReproductionStep {
                    sequence: 1,
                    kind: ReproductionStepKind::RunRegisteredTest,
                    description: "Run the exact registered regression fixture".to_owned(),
                    evidence_fact_ids: Vec::new(),
                    separate_grant_required: true,
                    write_authority: false,
                }],
                expected_result_sha256: "3".repeat(64),
                observed_result_sha256: Some("4".repeat(64)),
                log_sha256s: vec!["5".repeat(64)],
                failure_signature_observed: true,
                execution_status: ReproductionExecutionStatus::Completed,
                unsafe_reason: None,
            },
        )
        .expect("reproduction")
    }

    fn hypotheses(
        index: &DeepRepositoryIndex,
        reproduction: &ReproductionRecord,
    ) -> HypothesisRecord {
        seal_hypotheses(
            index,
            Some(reproduction),
            HypothesisRecord {
                hypotheses: vec![
                    ChangeHypothesis {
                        hypothesis_id: "hypothesis-a".to_owned(),
                        explanation: "The cited calculation selects the old value".to_owned(),
                        checks: vec![HypothesisCheck {
                            check_id: "check-a".to_owned(),
                            description: "Compare the cited result to the failure signature"
                                .to_owned(),
                            evidence_fact_ids: Vec::new(),
                            log_sha256s: vec!["5".repeat(64)],
                            result: HypothesisCheckResult::Passed,
                        }],
                        status: HypothesisStatus::Supported,
                        rejection_reason: None,
                    },
                    ChangeHypothesis {
                        hypothesis_id: "hypothesis-b".to_owned(),
                        explanation: "The environment changes the result".to_owned(),
                        checks: vec![HypothesisCheck {
                            check_id: "check-b".to_owned(),
                            description: "Compare the pinned environment identity".to_owned(),
                            evidence_fact_ids: Vec::new(),
                            log_sha256s: Vec::new(),
                            result: HypothesisCheckResult::Failed,
                        }],
                        status: HypothesisStatus::Rejected,
                        rejection_reason: Some("The same pinned environment reproduces".to_owned()),
                    },
                ],
                selected_hypothesis_id: Some("hypothesis-a".to_owned()),
                state_trace_fact_ids: Vec::new(),
                correlated_log_sha256s: vec!["5".repeat(64)],
                hypothesis_sha256: String::new(),
            },
        )
        .expect("hypotheses")
    }

    fn alternatives() -> Vec<ChangeAlternativeSet> {
        ChangeAlternativeDimension::ALL
            .into_iter()
            .map(|dimension| ChangeAlternativeSet {
                dimension,
                options: vec![
                    ChangeAlternativeOption {
                        option_id: "option-a".to_owned(),
                        description: "Use the smallest cited local change".to_owned(),
                        tradeoffs: vec!["Limits the current blast radius".to_owned()],
                        evidence_fact_ids: Vec::new(),
                        irreversible: false,
                    },
                    ChangeAlternativeOption {
                        option_id: "option-b".to_owned(),
                        description: "Use a broader structural change".to_owned(),
                        tradeoffs: vec!["Touches more unproven scope".to_owned()],
                        evidence_fact_ids: Vec::new(),
                        irreversible: dimension == ChangeAlternativeDimension::Irreversibility,
                    },
                ],
                selected_option_id: "option-a".to_owned(),
                decision_id: format!("decision-{dimension:?}").to_ascii_lowercase(),
                rationale: "The selected option has the smallest cited scope".to_owned(),
                alternative_sha256: String::new(),
            })
            .collect()
    }

    fn validation(intent: &ChangeIntentRecord) -> Vec<PlannedValidation> {
        vec![PlannedValidation {
            validation_id: "validation-regression".to_owned(),
            kind: PlannedValidationKind::Unit,
            description: "Run the exact regression fixture".to_owned(),
            acceptance_checks: intent.input.acceptance_checks.clone(),
            separate_grant_required: true,
            write_authority: false,
        }]
    }

    #[test]
    fn reproduced_defect_with_regression_hypothesis_and_alternatives_is_review_ready() {
        let (index, intent, impact, test) = fixture();
        let reproduction = reproduction(&index);
        let hypotheses = hypotheses(&index, &reproduction);
        let plan = build_change_plan(
            &index,
            &intent,
            &impact,
            ChangePlanInput {
                plan_id: "plan-defect".to_owned(),
                work_kind: ChangeWorkKind::Defect,
                reproduction: Some(reproduction),
                regression_test: RegressionTestPlan {
                    disposition: RegressionTestDisposition::FailingTestRetained,
                    test_fact_ids: vec![test],
                    rationale: None,
                    regression_sha256: String::new(),
                },
                hypotheses,
                alternatives: alternatives(),
                validations: validation(&intent),
            },
        )
        .expect("plan");
        assert!(verify_change_plan(&index, &intent, &impact, &plan));
        assert_eq!(plan.status, ChangePlanStatus::ReadyForImplementationReview);
        assert_eq!(plan.required_reviews, vec![ChangeReviewKind::Rollback]);
        assert!(!plan.mutation_authority);
        assert!(
            plan.input
                .validations
                .iter()
                .all(|item| { item.separate_grant_required && !item.write_authority })
        );
    }

    #[test]
    fn nonreproduction_and_missing_failing_test_never_become_proven_fix_plans() {
        let (index, intent, impact, _) = fixture();
        let mut reproduction = reproduction(&index);
        reproduction.input.failure_signature_observed = false;
        reproduction.input.observed_result_sha256 = Some("3".repeat(64));
        reproduction = seal_reproduction(&index, reproduction.input).expect("not reproduced");
        let plan = build_change_plan(
            &index,
            &intent,
            &impact,
            ChangePlanInput {
                plan_id: "plan-not-reproduced".to_owned(),
                work_kind: ChangeWorkKind::Defect,
                reproduction: Some(reproduction),
                regression_test: RegressionTestPlan {
                    disposition: RegressionTestDisposition::RequiredBeforeFix,
                    test_fact_ids: Vec::new(),
                    rationale: None,
                    regression_sha256: String::new(),
                },
                hypotheses: HypothesisRecord {
                    hypotheses: Vec::new(),
                    selected_hypothesis_id: None,
                    state_trace_fact_ids: Vec::new(),
                    correlated_log_sha256s: Vec::new(),
                    hypothesis_sha256: String::new(),
                },
                alternatives: alternatives(),
                validations: validation(&intent),
            },
        )
        .expect("blocked plan");
        assert_eq!(plan.status, ChangePlanStatus::ReproductionRequired);
        assert!(plan.input.reproduction.is_some_and(|item| {
            item.outcome == ReproductionOutcome::NotReproduced && !item.root_cause_proven
        }));
    }

    #[test]
    fn validation_authority_alternative_and_hypothesis_forgeries_fail_closed() {
        let (index, intent, impact, test) = fixture();
        let reproduction = reproduction(&index);
        let hypotheses = hypotheses(&index, &reproduction);
        let mut validations = validation(&intent);
        validations[0].write_authority = true;
        assert_eq!(
            build_change_plan(
                &index,
                &intent,
                &impact,
                ChangePlanInput {
                    plan_id: "plan-forged".to_owned(),
                    work_kind: ChangeWorkKind::Defect,
                    reproduction: Some(reproduction.clone()),
                    regression_test: RegressionTestPlan {
                        disposition: RegressionTestDisposition::FailingTestRetained,
                        test_fact_ids: vec![test.clone()],
                        rationale: None,
                        regression_sha256: String::new(),
                    },
                    hypotheses: hypotheses.clone(),
                    alternatives: alternatives(),
                    validations,
                }
            ),
            Err(ChangePlanError::ValidationInvalid)
        );

        let mut forged_alternatives = alternatives();
        forged_alternatives.pop();
        assert_eq!(
            build_change_plan(
                &index,
                &intent,
                &impact,
                ChangePlanInput {
                    plan_id: "plan-missing-alternative".to_owned(),
                    work_kind: ChangeWorkKind::Defect,
                    reproduction: Some(reproduction),
                    regression_test: RegressionTestPlan {
                        disposition: RegressionTestDisposition::FailingTestRetained,
                        test_fact_ids: vec![test],
                        rationale: None,
                        regression_sha256: String::new(),
                    },
                    hypotheses,
                    alternatives: forged_alternatives,
                    validations: validation(&intent),
                }
            ),
            Err(ChangePlanError::AlternativeInvalid)
        );
    }
}
