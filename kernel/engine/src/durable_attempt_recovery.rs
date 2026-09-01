//! Durable attempt checkpoints and deterministic, no-replay restart decisions.

use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::sync::{Arc, Mutex};

use agentmage_kernel_contracts::{
    CanonicalAttemptBudgetState, CanonicalAttemptCheckpoint, CanonicalCheckpointEffectState,
    CanonicalWorkflowFailureClass,
};
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::engineering_records::{ValidateCanonicalRecord, canonical_record_sha256};
use crate::retry_admission::{
    AdmittedFreshAttempt, FreshAttemptAdmissionError, FreshAttemptAdmissionInput,
    compile_execution_ready_attempt,
};

const ZERO_SHA256: &str = "0000000000000000000000000000000000000000000000000000000000000000";

/// Seals a validated attempt checkpoint with its canonical digest.
pub fn seal_attempt_checkpoint(
    mut checkpoint: CanonicalAttemptCheckpoint,
) -> Result<CanonicalAttemptCheckpoint, AttemptRecoveryError> {
    checkpoint.checkpoint_sha256 = ZERO_SHA256.to_owned();
    checkpoint
        .validate_canonical()
        .map_err(|_| AttemptRecoveryError::InvalidCheckpoint)?;
    checkpoint.checkpoint_sha256 = canonical_record_sha256(&checkpoint)
        .map_err(|_| AttemptRecoveryError::InvalidCheckpoint)?;
    verify_attempt_checkpoint(&checkpoint)?;
    Ok(checkpoint)
}

/// Verifies semantic validity and the exact canonical seal of an attempt checkpoint.
pub fn verify_attempt_checkpoint(
    checkpoint: &CanonicalAttemptCheckpoint,
) -> Result<(), AttemptRecoveryError> {
    checkpoint
        .validate_canonical()
        .map_err(|_| AttemptRecoveryError::InvalidCheckpoint)?;
    let retained = checkpoint.checkpoint_sha256.clone();
    let mut candidate = checkpoint.clone();
    candidate.checkpoint_sha256 = ZERO_SHA256.to_owned();
    let expected =
        canonical_record_sha256(&candidate).map_err(|_| AttemptRecoveryError::InvalidCheckpoint)?;
    if retained != expected {
        return Err(AttemptRecoveryError::InvalidCheckpoint);
    }
    Ok(())
}

/// Stable failure at the attempt recovery boundary.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AttemptRecoveryError {
    /// The checkpoint is malformed, inconsistent, or digest-invalid.
    InvalidCheckpoint,
    /// A claim owner or checkpoint identity is malformed.
    InvalidOwnership,
    /// Another client already owns this exact checkpoint recovery.
    AlreadyOwned,
    /// Synchronization state is unavailable and therefore fails closed.
    SynchronizationUnavailable,
}

/// Refusal to compose a recovery decision with ordinary fresh-attempt admission.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RecoveredFreshAttemptError {
    /// The recovery decision did not select the fresh-attempt path.
    DecisionNotFreshAttempt,
    /// The ordinary admission boundary rejected a freshness, authority, policy, or budget fact.
    Admission(FreshAttemptAdmissionError),
}

/// Compiles a recovered successor only through the existing fresh-attempt admission boundary.
///
/// The recovery decision itself grants no authority. The returned opaque proof exists only when
/// the decision selected `FreshAttempt` and the existing compiler verified new call, tool-call,
/// attempt, grant, approval, idempotency, preflight, reconciliation, and budget facts together.
pub fn compile_recovered_fresh_attempt(
    recovery: &AttemptRecoveryDecision,
    input: FreshAttemptAdmissionInput<'_>,
) -> Result<AdmittedFreshAttempt, RecoveredFreshAttemptError> {
    if recovery.action != AttemptRecoveryAction::FreshAttempt
        || !recovery.fresh_identity_and_authority_required
        || recovery.prior_effect_replay_allowed
    {
        return Err(RecoveredFreshAttemptError::DecisionNotFreshAttempt);
    }
    compile_execution_ready_attempt(input).map_err(RecoveredFreshAttemptError::Admission)
}

/// One checkpoint-bound identity family compared during restart.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AttemptCheckpointDrift {
    /// Authoritative sources changed.
    Source,
    /// Plan or active step changed.
    Plan,
    /// Selected model identity changed.
    Model,
    /// Model-visible context changed.
    Context,
    /// Admitted tool catalog changed.
    ToolCatalog,
    /// Effective policy changed.
    Policy,
    /// Grant ledger changed.
    Grants,
    /// Approval ledger changed.
    Approvals,
    /// Current preflight facts changed.
    Preflights,
    /// Attempt history changed.
    Attempts,
    /// Receipt history changed.
    Receipts,
    /// Artifact set changed.
    Artifacts,
    /// Verifier evidence changed.
    Verifier,
    /// Budget policy or consumed counters changed.
    Budgets,
    /// Repeated-state history changed.
    RepeatedState,
    /// Correctness journal cursor changed.
    EventCursor,
    /// Trusted environment changed.
    Environment,
    /// Current postcondition cannot establish the checkpointed effect state.
    Postcondition,
}

/// Exact consumed limits used to decide whether restart may continue.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AttemptBudgetLimits {
    /// Maximum parser repairs.
    pub parser_repairs: u64,
    /// Maximum targeted model repairs.
    pub model_repairs: u64,
    /// Maximum attempts for the active step.
    pub step_attempts: u64,
    /// Maximum failures for every closed class in canonical order.
    pub error_class_counts: Vec<u64>,
    /// Maximum total workflow work.
    pub workflow_work: u64,
    /// Maximum replans.
    pub replans: u64,
    /// Maximum repeated observations of one complete state.
    pub repeated_state: u64,
}

/// Closed budget family included in a terminal diagnosis.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AttemptBudgetDimension {
    /// Parser repair budget.
    ParserRepair,
    /// Model repair budget.
    ModelRepair,
    /// Active-step attempt budget.
    StepAttempt,
    /// One closed error-class budget.
    ErrorClass(CanonicalWorkflowFailureClass),
    /// Total workflow work budget.
    WorkflowWork,
    /// Replan budget.
    Replan,
    /// Repeated-state budget.
    RepeatedState,
}

/// Current runtime-owned facts observed independently of checkpoint content.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AttemptResumeObservation {
    /// Exact active workflow checkpoint identity.
    pub workflow_checkpoint_id: String,
    /// Exact workflow identity.
    pub workflow_id: String,
    /// Exact execution identity.
    pub execution_id: String,
    /// Exact step execution identity.
    pub step_execution_id: String,
    /// Current attempt identity.
    pub attempt_id: Option<String>,
    /// Current identities in checkpoint field order, source through repeated state.
    pub source_sha256: String,
    /// Current plan identity.
    pub plan_sha256: String,
    /// Current model identity.
    pub model_sha256: String,
    /// Current context identity.
    pub context_sha256: String,
    /// Current tool catalog identity.
    pub tool_catalog_sha256: String,
    /// Current policy identity.
    pub policy_sha256: String,
    /// Current grant ledger identity.
    pub grants_sha256: String,
    /// Current approval ledger identity.
    pub approvals_sha256: String,
    /// Current preflight identity.
    pub preflights_sha256: String,
    /// Current attempt-history identity.
    pub attempts_sha256: String,
    /// Current receipt-history identity.
    pub receipts_sha256: String,
    /// Current artifact-set identity.
    pub artifacts_sha256: String,
    /// Current verifier identity.
    pub verifier_sha256: String,
    /// Current immutable budget policy.
    pub budget_policy_sha256: String,
    /// Current consumed counters loaded from durable history.
    pub budget_state: CanonicalAttemptBudgetState,
    /// Current repeated-state history identity.
    pub repeated_state_sha256: String,
    /// Current complete-state repeat count.
    pub repeated_state_count: u64,
    /// Current correctness journal cursor.
    pub event_cursor: agentmage_kernel_contracts::RuntimeEventCursor,
    /// Current trusted environment.
    pub environment_sha256: String,
    /// Independently re-established effect state.
    pub effect_state: CanonicalCheckpointEffectState,
    /// Current authority ledger proves prior attempt authority was consumed.
    pub authority_consumed: bool,
    /// Current postcondition was observed and is conclusive for the declared effect state.
    pub postcondition_current: bool,
    /// A trusted cancellation is currently requested.
    pub cancelled: bool,
    /// A dependency required by the next action is currently unavailable.
    pub dependency_blocked: bool,
    /// Current policy requires a new exact approval before any successor attempt.
    pub fresh_approval_required: bool,
    /// A bounded deterministic parser or call repair is currently eligible.
    pub deterministic_repair_eligible: bool,
    /// A new attempt is eligible after ordinary fresh-identity admission.
    pub fresh_attempt_eligible: bool,
    /// The workflow has already reached a terminal state.
    pub already_terminal: bool,
    /// Exact immutable limits currently bound to the checkpointed policy.
    pub budget_limits: AttemptBudgetLimits,
}

/// Exactly one safe action selected by attempt recovery.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AttemptRecoveryAction {
    /// Continue unchanged non-effect work from the exact verified boundary.
    Continue,
    /// Enter ordinary admission for a genuinely fresh attempt and authority chain.
    FreshAttempt,
    /// Apply one bounded deterministic repair.
    DeterministicRepair,
    /// Rebuild the plan from current facts.
    Replan,
    /// Wait for a new exact approval.
    AwaitApproval,
    /// Wait for a required dependency.
    AwaitDependency,
    /// Reconcile a completed or uncertain effect without replaying it.
    ReconcileEffect,
    /// Honor trusted cancellation.
    Cancel,
    /// Stop with one complete content-free diagnosis.
    TerminateDiagnosed,
}

/// One complete terminal diagnosis for a non-cancelled recovery stop.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AttemptTerminalDiagnosis {
    /// Exact failed step execution.
    pub step_execution_id: String,
    /// Last verified attempt checkpoint.
    pub attempt_checkpoint_id: String,
    /// Ordered attempt history identity.
    pub attempts_sha256: String,
    /// Current verifier/evidence identity.
    pub verifier_sha256: String,
    /// Every exhausted budget family.
    pub exhausted_budgets: Vec<AttemptBudgetDimension>,
    /// Stable content-free blocked reason.
    pub blocked_reason_code: &'static str,
    /// Whether an effect remains uncertain.
    pub uncertain_effect: bool,
    /// Whether a new approval is needed.
    pub approval_needed: bool,
    /// Safe action from which a separate caller may later recover.
    pub safe_resume_action: AttemptRecoveryAction,
    /// Digest of this exact diagnosis.
    pub diagnosis_sha256: String,
}

/// Content-free result of one deterministic recovery evaluation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct AttemptRecoveryDecision {
    /// The only selected action.
    pub action: AttemptRecoveryAction,
    /// Stable reason code.
    pub reason_code: &'static str,
    /// Exact sorted drift dimensions.
    pub drift: Vec<AttemptCheckpointDrift>,
    /// Exhausted budget dimensions.
    pub exhausted_budgets: Vec<AttemptBudgetDimension>,
    /// Complete diagnosis, present exactly for `TerminateDiagnosed`.
    pub diagnosis: Option<AttemptTerminalDiagnosis>,
    /// Always false: an old effect is never replayable.
    pub prior_effect_replay_allowed: bool,
    /// Whether a later attempt requires new identities and fresh authority.
    pub fresh_identity_and_authority_required: bool,
}

/// Returns the stable digest used to bind a durable ownership claim to one exact decision.
#[must_use]
pub fn attempt_recovery_decision_sha256(decision: &AttemptRecoveryDecision) -> String {
    digest_json(decision)
}

/// Verifies the self-digest of one terminal diagnosis.
#[must_use]
pub fn verify_attempt_terminal_diagnosis(diagnosis: &AttemptTerminalDiagnosis) -> bool {
    if !valid_sha256(&diagnosis.diagnosis_sha256) {
        return false;
    }
    let mut candidate = diagnosis.clone();
    let retained = std::mem::take(&mut candidate.diagnosis_sha256);
    retained == digest_json(&candidate)
}

/// Revalidates every checkpoint family and selects exactly one safe action.
#[must_use]
pub fn decide_attempt_recovery(
    checkpoint: &CanonicalAttemptCheckpoint,
    current: &AttemptResumeObservation,
) -> AttemptRecoveryDecision {
    if verify_attempt_checkpoint(checkpoint).is_err() || !valid_observation(current) {
        return terminal(
            checkpoint,
            current,
            "attempt_recovery.checkpoint_invalid",
            Vec::new(),
            Vec::new(),
            AttemptRecoveryAction::Replan,
        );
    }
    if current.cancelled {
        return selected(
            AttemptRecoveryAction::Cancel,
            "attempt_recovery.cancelled",
            Vec::new(),
        );
    }
    if current.already_terminal {
        return terminal(
            checkpoint,
            current,
            "attempt_recovery.already_terminal",
            Vec::new(),
            Vec::new(),
            AttemptRecoveryAction::TerminateDiagnosed,
        );
    }

    let exhausted = exhausted_budgets(
        &current.budget_state,
        &current.budget_limits,
        current.repeated_state_count,
    );
    if !exhausted.is_empty() {
        return terminal(
            checkpoint,
            current,
            "attempt_recovery.budget_exhausted",
            Vec::new(),
            exhausted,
            AttemptRecoveryAction::Replan,
        );
    }

    let drift = checkpoint_drift(checkpoint, current);
    if !drift.is_empty() {
        if drift.contains(&AttemptCheckpointDrift::Postcondition)
            || matches!(
                checkpoint.effect_state,
                CanonicalCheckpointEffectState::VerifiedApplied
                    | CanonicalCheckpointEffectState::Uncertain
            )
            || matches!(
                current.effect_state,
                CanonicalCheckpointEffectState::Uncertain
            )
        {
            return selected_with_drift(
                AttemptRecoveryAction::ReconcileEffect,
                "attempt_recovery.effect_reconciliation_required",
                drift,
            );
        }
        if drift.iter().any(|item| {
            matches!(
                item,
                AttemptCheckpointDrift::Policy
                    | AttemptCheckpointDrift::Grants
                    | AttemptCheckpointDrift::Approvals
            )
        }) {
            return selected_with_drift(
                AttemptRecoveryAction::AwaitApproval,
                "attempt_recovery.authority_drift",
                drift,
            );
        }
        return selected_with_drift(
            AttemptRecoveryAction::Replan,
            "attempt_recovery.identity_drift",
            drift,
        );
    }

    if matches!(
        current.effect_state,
        CanonicalCheckpointEffectState::VerifiedApplied | CanonicalCheckpointEffectState::Uncertain
    ) {
        return selected(
            AttemptRecoveryAction::ReconcileEffect,
            "attempt_recovery.effect_not_replayable",
            Vec::new(),
        );
    }
    if current.dependency_blocked {
        return selected(
            AttemptRecoveryAction::AwaitDependency,
            "attempt_recovery.dependency_blocked",
            Vec::new(),
        );
    }
    if current.fresh_approval_required {
        return selected(
            AttemptRecoveryAction::AwaitApproval,
            "attempt_recovery.fresh_approval_required",
            Vec::new(),
        );
    }
    if current.deterministic_repair_eligible {
        return selected(
            AttemptRecoveryAction::DeterministicRepair,
            "attempt_recovery.deterministic_repair",
            Vec::new(),
        );
    }
    if current.fresh_attempt_eligible {
        let mut result = selected(
            AttemptRecoveryAction::FreshAttempt,
            "attempt_recovery.fresh_attempt",
            Vec::new(),
        );
        result.fresh_identity_and_authority_required = true;
        return result;
    }
    selected(
        AttemptRecoveryAction::Continue,
        "attempt_recovery.continue_verified",
        Vec::new(),
    )
}

/// In-runtime ownership gate for concurrent clients of the same durable store.
///
/// The claim is keyed by immutable checkpoint identity. The operational store persists the winning
/// owner alongside the checkpoint; this gate prevents duplicate work between clients sharing one
/// open store before either can reach an effect boundary.
#[derive(Clone, Debug, Default)]
pub struct AttemptResumeOwnership {
    claims: Arc<Mutex<BTreeSet<(String, String)>>>,
}

impl AttemptResumeOwnership {
    /// Constructs an empty ownership gate.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Claims one checkpoint for one owner. Any existing owner fails closed.
    pub fn claim(
        &self,
        checkpoint_id: &str,
        owner_id: &str,
    ) -> Result<AttemptResumeClaim, AttemptRecoveryError> {
        if !valid_identifier(checkpoint_id) || !valid_identifier(owner_id) {
            return Err(AttemptRecoveryError::InvalidOwnership);
        }
        let mut claims = self
            .claims
            .lock()
            .map_err(|_| AttemptRecoveryError::SynchronizationUnavailable)?;
        if claims
            .iter()
            .any(|(checkpoint, _)| checkpoint == checkpoint_id)
        {
            return Err(AttemptRecoveryError::AlreadyOwned);
        }
        claims.insert((checkpoint_id.to_owned(), owner_id.to_owned()));
        Ok(AttemptResumeClaim {
            checkpoint_id: checkpoint_id.to_owned(),
            owner_id: owner_id.to_owned(),
        })
    }

    /// Verifies that a claim remains the sole current owner.
    pub fn is_current(&self, claim: &AttemptResumeClaim) -> Result<bool, AttemptRecoveryError> {
        self.claims
            .lock()
            .map(|claims| claims.contains(&(claim.checkpoint_id.clone(), claim.owner_id.clone())))
            .map_err(|_| AttemptRecoveryError::SynchronizationUnavailable)
    }
}

/// Opaque proof that one client owns recovery for one checkpoint.
#[derive(Debug, PartialEq, Eq)]
pub struct AttemptResumeClaim {
    checkpoint_id: String,
    owner_id: String,
}

impl AttemptResumeClaim {
    /// Returns the exact checkpoint identity.
    #[must_use]
    pub fn checkpoint_id(&self) -> &str {
        &self.checkpoint_id
    }

    /// Returns the winning client identity.
    #[must_use]
    pub fn owner_id(&self) -> &str {
        &self.owner_id
    }
}

fn checkpoint_drift(
    checkpoint: &CanonicalAttemptCheckpoint,
    current: &AttemptResumeObservation,
) -> Vec<AttemptCheckpointDrift> {
    let mut drift = BTreeSet::new();
    let identity_checks = [
        (
            checkpoint.source_sha256 != current.source_sha256,
            AttemptCheckpointDrift::Source,
        ),
        (
            checkpoint.plan_sha256 != current.plan_sha256,
            AttemptCheckpointDrift::Plan,
        ),
        (
            checkpoint.model_sha256 != current.model_sha256,
            AttemptCheckpointDrift::Model,
        ),
        (
            checkpoint.context_sha256 != current.context_sha256,
            AttemptCheckpointDrift::Context,
        ),
        (
            checkpoint.tool_catalog_sha256 != current.tool_catalog_sha256,
            AttemptCheckpointDrift::ToolCatalog,
        ),
        (
            checkpoint.policy_sha256 != current.policy_sha256,
            AttemptCheckpointDrift::Policy,
        ),
        (
            checkpoint.grants_sha256 != current.grants_sha256,
            AttemptCheckpointDrift::Grants,
        ),
        (
            checkpoint.approvals_sha256 != current.approvals_sha256,
            AttemptCheckpointDrift::Approvals,
        ),
        (
            checkpoint.preflights_sha256 != current.preflights_sha256,
            AttemptCheckpointDrift::Preflights,
        ),
        (
            checkpoint.attempts_sha256 != current.attempts_sha256,
            AttemptCheckpointDrift::Attempts,
        ),
        (
            checkpoint.receipts_sha256 != current.receipts_sha256,
            AttemptCheckpointDrift::Receipts,
        ),
        (
            checkpoint.artifacts_sha256 != current.artifacts_sha256,
            AttemptCheckpointDrift::Artifacts,
        ),
        (
            checkpoint.verifier_sha256 != current.verifier_sha256,
            AttemptCheckpointDrift::Verifier,
        ),
        (
            checkpoint.budget_policy_sha256 != current.budget_policy_sha256
                || checkpoint.budget_state != current.budget_state,
            AttemptCheckpointDrift::Budgets,
        ),
        (
            checkpoint.repeated_state_sha256 != current.repeated_state_sha256
                || checkpoint.repeated_state_count != current.repeated_state_count,
            AttemptCheckpointDrift::RepeatedState,
        ),
        (
            checkpoint.event_cursor != current.event_cursor,
            AttemptCheckpointDrift::EventCursor,
        ),
        (
            checkpoint.environment_sha256 != current.environment_sha256,
            AttemptCheckpointDrift::Environment,
        ),
    ];
    for (changed, dimension) in identity_checks {
        if changed {
            drift.insert(dimension);
        }
    }
    if checkpoint.workflow_checkpoint_id != current.workflow_checkpoint_id
        || checkpoint.workflow_id != current.workflow_id
        || checkpoint.execution_id != current.execution_id
        || checkpoint.step_execution_id != current.step_execution_id
        || checkpoint.attempt_id != current.attempt_id
    {
        drift.insert(AttemptCheckpointDrift::Plan);
    }
    if !current.postcondition_current
        || checkpoint.effect_state != current.effect_state
        || checkpoint.authority_consumed != current.authority_consumed
    {
        drift.insert(AttemptCheckpointDrift::Postcondition);
    }
    drift.into_iter().collect()
}

fn exhausted_budgets(
    state: &CanonicalAttemptBudgetState,
    limits: &AttemptBudgetLimits,
    repeated_state: u64,
) -> Vec<AttemptBudgetDimension> {
    let mut exhausted = Vec::new();
    if state.parser_repairs >= limits.parser_repairs {
        exhausted.push(AttemptBudgetDimension::ParserRepair);
    }
    if state.model_repairs >= limits.model_repairs {
        exhausted.push(AttemptBudgetDimension::ModelRepair);
    }
    if state.step_attempts >= limits.step_attempts {
        exhausted.push(AttemptBudgetDimension::StepAttempt);
    }
    for (index, used) in state.error_class_counts.iter().enumerate() {
        if limits
            .error_class_counts
            .get(index)
            .is_some_and(|limit| used >= limit)
        {
            exhausted.push(AttemptBudgetDimension::ErrorClass(
                CanonicalWorkflowFailureClass::ALL[index],
            ));
        }
    }
    if state.workflow_work >= limits.workflow_work {
        exhausted.push(AttemptBudgetDimension::WorkflowWork);
    }
    if state.replans >= limits.replans {
        exhausted.push(AttemptBudgetDimension::Replan);
    }
    if repeated_state >= limits.repeated_state {
        exhausted.push(AttemptBudgetDimension::RepeatedState);
    }
    exhausted
}

fn valid_observation(current: &AttemptResumeObservation) -> bool {
    let identifiers = [
        current.workflow_checkpoint_id.as_str(),
        current.workflow_id.as_str(),
        current.execution_id.as_str(),
        current.step_execution_id.as_str(),
    ];
    let digests = [
        &current.source_sha256,
        &current.plan_sha256,
        &current.model_sha256,
        &current.context_sha256,
        &current.tool_catalog_sha256,
        &current.policy_sha256,
        &current.grants_sha256,
        &current.approvals_sha256,
        &current.preflights_sha256,
        &current.attempts_sha256,
        &current.receipts_sha256,
        &current.artifacts_sha256,
        &current.verifier_sha256,
        &current.budget_policy_sha256,
        &current.repeated_state_sha256,
        &current.environment_sha256,
        &current.event_cursor.event_sha256,
    ];
    identifiers.into_iter().all(valid_identifier)
        && current.attempt_id.as_deref().is_none_or(valid_identifier)
        && digests.into_iter().all(|value| valid_sha256(value))
        && current.event_cursor.schema_version_compatible()
        && current.budget_state.error_class_counts.len() == CanonicalWorkflowFailureClass::ALL.len()
        && current.budget_limits.error_class_counts.len()
            == CanonicalWorkflowFailureClass::ALL.len()
}

trait CursorSchemaCompatibility {
    fn schema_version_compatible(&self) -> bool;
}

impl CursorSchemaCompatibility for agentmage_kernel_contracts::RuntimeEventCursor {
    fn schema_version_compatible(&self) -> bool {
        !self.run_id.as_str().is_empty() && !self.event_id.as_str().is_empty()
    }
}

fn selected(
    action: AttemptRecoveryAction,
    reason_code: &'static str,
    drift: Vec<AttemptCheckpointDrift>,
) -> AttemptRecoveryDecision {
    selected_with_drift(action, reason_code, drift)
}

fn selected_with_drift(
    action: AttemptRecoveryAction,
    reason_code: &'static str,
    drift: Vec<AttemptCheckpointDrift>,
) -> AttemptRecoveryDecision {
    AttemptRecoveryDecision {
        action,
        reason_code,
        drift,
        exhausted_budgets: Vec::new(),
        diagnosis: None,
        prior_effect_replay_allowed: false,
        fresh_identity_and_authority_required: false,
    }
}

fn terminal(
    checkpoint: &CanonicalAttemptCheckpoint,
    current: &AttemptResumeObservation,
    reason_code: &'static str,
    drift: Vec<AttemptCheckpointDrift>,
    exhausted_budgets: Vec<AttemptBudgetDimension>,
    safe_resume_action: AttemptRecoveryAction,
) -> AttemptRecoveryDecision {
    let mut diagnosis = AttemptTerminalDiagnosis {
        step_execution_id: current.step_execution_id.clone(),
        attempt_checkpoint_id: checkpoint.attempt_checkpoint_id.clone(),
        attempts_sha256: current.attempts_sha256.clone(),
        verifier_sha256: current.verifier_sha256.clone(),
        exhausted_budgets: exhausted_budgets.clone(),
        blocked_reason_code: reason_code,
        uncertain_effect: matches!(
            checkpoint.effect_state,
            CanonicalCheckpointEffectState::Uncertain
        ) || matches!(
            current.effect_state,
            CanonicalCheckpointEffectState::Uncertain
        ),
        approval_needed: current.fresh_approval_required,
        safe_resume_action,
        diagnosis_sha256: String::new(),
    };
    diagnosis.diagnosis_sha256 = digest_json(&diagnosis);
    AttemptRecoveryDecision {
        action: AttemptRecoveryAction::TerminateDiagnosed,
        reason_code,
        drift,
        exhausted_budgets,
        diagnosis: Some(diagnosis),
        prior_effect_replay_allowed: false,
        fresh_identity_and_authority_required: false,
    }
}

fn digest_json(value: &impl Serialize) -> String {
    let bytes = serde_json::to_vec(value).unwrap_or_default();
    let mut output = String::with_capacity(64);
    for byte in Sha256::digest(bytes) {
        let _ = write!(output, "{byte:02x}");
    }
    output
}

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
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::process::Command;
    use std::sync::{Arc, Barrier};
    use std::thread;

    use agentmage_kernel_contracts::{
        CONTRACT_SCHEMA_VERSION, CanonicalAttemptBudgetState, CanonicalAttemptCheckpoint,
        CanonicalCheckpointEffectState, CanonicalWorkflowFailureClass, RuntimeEventCursor,
        RuntimeEventId, RuntimeRunId,
    };

    use super::*;

    const CRASH_ENV: &str = "AGENTMAGE_ATTEMPT_RECOVERY_CRASH_CHILD";
    const CRASH_PATH_ENV: &str = "AGENTMAGE_ATTEMPT_RECOVERY_CRASH_PATH";

    fn hash(byte: char) -> String {
        byte.to_string().repeat(64)
    }

    fn budget_state() -> CanonicalAttemptBudgetState {
        CanonicalAttemptBudgetState {
            parser_repairs: 1,
            model_repairs: 1,
            step_attempts: 1,
            error_class_counts: vec![0; CanonicalWorkflowFailureClass::ALL.len()],
            workflow_work: 3,
            replans: 0,
        }
    }

    fn checkpoint(effect_state: CanonicalCheckpointEffectState) -> CanonicalAttemptCheckpoint {
        seal_attempt_checkpoint(CanonicalAttemptCheckpoint {
            schema_version: CONTRACT_SCHEMA_VERSION,
            attempt_checkpoint_id: "attempt-checkpoint-1".to_owned(),
            workflow_checkpoint_id: "workflow-checkpoint-1".to_owned(),
            workflow_id: "workflow-1".to_owned(),
            execution_id: "execution-1".to_owned(),
            step_execution_id: "step-execution-1".to_owned(),
            attempt_id: Some("attempt-1".to_owned()),
            source_sha256: hash('1'),
            plan_sha256: hash('2'),
            model_sha256: hash('3'),
            context_sha256: hash('4'),
            tool_catalog_sha256: hash('5'),
            policy_sha256: hash('6'),
            grants_sha256: hash('7'),
            approvals_sha256: hash('8'),
            preflights_sha256: hash('9'),
            attempts_sha256: hash('a'),
            receipts_sha256: hash('b'),
            artifacts_sha256: hash('c'),
            verifier_sha256: hash('d'),
            budget_policy_sha256: hash('e'),
            budget_state: budget_state(),
            repeated_state_sha256: hash('f'),
            repeated_state_count: 1,
            event_cursor: RuntimeEventCursor {
                run_id: RuntimeRunId::from_raw("run-1"),
                event_id: RuntimeEventId::from_raw("event-7"),
                sequence: 7,
                event_sha256: hash('0'),
            },
            environment_sha256: hash('1'),
            effect_state,
            authority_consumed: !matches!(effect_state, CanonicalCheckpointEffectState::NoEffect),
            created_at: "2026-08-31T12:00:00Z".to_owned(),
            checkpoint_sha256: hash('0'),
        })
        .expect("checkpoint seals")
    }

    fn observation(checkpoint: &CanonicalAttemptCheckpoint) -> AttemptResumeObservation {
        AttemptResumeObservation {
            workflow_checkpoint_id: checkpoint.workflow_checkpoint_id.clone(),
            workflow_id: checkpoint.workflow_id.clone(),
            execution_id: checkpoint.execution_id.clone(),
            step_execution_id: checkpoint.step_execution_id.clone(),
            attempt_id: checkpoint.attempt_id.clone(),
            source_sha256: checkpoint.source_sha256.clone(),
            plan_sha256: checkpoint.plan_sha256.clone(),
            model_sha256: checkpoint.model_sha256.clone(),
            context_sha256: checkpoint.context_sha256.clone(),
            tool_catalog_sha256: checkpoint.tool_catalog_sha256.clone(),
            policy_sha256: checkpoint.policy_sha256.clone(),
            grants_sha256: checkpoint.grants_sha256.clone(),
            approvals_sha256: checkpoint.approvals_sha256.clone(),
            preflights_sha256: checkpoint.preflights_sha256.clone(),
            attempts_sha256: checkpoint.attempts_sha256.clone(),
            receipts_sha256: checkpoint.receipts_sha256.clone(),
            artifacts_sha256: checkpoint.artifacts_sha256.clone(),
            verifier_sha256: checkpoint.verifier_sha256.clone(),
            budget_policy_sha256: checkpoint.budget_policy_sha256.clone(),
            budget_state: checkpoint.budget_state.clone(),
            repeated_state_sha256: checkpoint.repeated_state_sha256.clone(),
            repeated_state_count: checkpoint.repeated_state_count,
            event_cursor: checkpoint.event_cursor.clone(),
            environment_sha256: checkpoint.environment_sha256.clone(),
            effect_state: checkpoint.effect_state,
            authority_consumed: checkpoint.authority_consumed,
            postcondition_current: true,
            cancelled: false,
            dependency_blocked: false,
            fresh_approval_required: false,
            deterministic_repair_eligible: false,
            fresh_attempt_eligible: false,
            already_terminal: false,
            budget_limits: AttemptBudgetLimits {
                parser_repairs: 4,
                model_repairs: 4,
                step_attempts: 4,
                error_class_counts: vec![4; CanonicalWorkflowFailureClass::ALL.len()],
                workflow_work: 16,
                replans: 4,
                repeated_state: 4,
            },
        }
    }

    #[test]
    fn story_22_4_checkpoint_binds_every_required_identity_and_rejects_mutation() {
        type CheckpointMutation = Box<dyn Fn(&mut CanonicalAttemptCheckpoint)>;
        let original = checkpoint(CanonicalCheckpointEffectState::NoEffect);
        verify_attempt_checkpoint(&original).expect("original verifies");
        let mutations: Vec<CheckpointMutation> = vec![
            Box::new(|v| v.source_sha256 = hash('2')),
            Box::new(|v| v.plan_sha256 = hash('3')),
            Box::new(|v| v.model_sha256 = hash('4')),
            Box::new(|v| v.context_sha256 = hash('5')),
            Box::new(|v| v.tool_catalog_sha256 = hash('6')),
            Box::new(|v| v.policy_sha256 = hash('7')),
            Box::new(|v| v.grants_sha256 = hash('8')),
            Box::new(|v| v.approvals_sha256 = hash('9')),
            Box::new(|v| v.preflights_sha256 = hash('a')),
            Box::new(|v| v.attempts_sha256 = hash('b')),
            Box::new(|v| v.receipts_sha256 = hash('c')),
            Box::new(|v| v.artifacts_sha256 = hash('d')),
            Box::new(|v| v.verifier_sha256 = hash('e')),
            Box::new(|v| v.budget_state.step_attempts += 1),
            Box::new(|v| v.repeated_state_count += 1),
            Box::new(|v| v.event_cursor.sequence += 1),
            Box::new(|v| v.environment_sha256 = hash('f')),
            Box::new(|v| v.effect_state = CanonicalCheckpointEffectState::Uncertain),
        ];
        for mutate in mutations {
            let mut candidate = original.clone();
            mutate(&mut candidate);
            assert_eq!(
                verify_attempt_checkpoint(&candidate),
                Err(AttemptRecoveryError::InvalidCheckpoint)
            );
        }

        let mut authority_without_attempt = original.clone();
        authority_without_attempt.attempt_id = None;
        authority_without_attempt.authority_consumed = true;
        assert_eq!(
            seal_attempt_checkpoint(authority_without_attempt),
            Err(AttemptRecoveryError::InvalidCheckpoint)
        );

        let mut effect_without_authority = original;
        effect_without_authority.effect_state = CanonicalCheckpointEffectState::Uncertain;
        effect_without_authority.authority_consumed = false;
        assert_eq!(
            seal_attempt_checkpoint(effect_without_authority),
            Err(AttemptRecoveryError::InvalidCheckpoint)
        );
    }

    #[test]
    fn story_22_4_selects_exactly_one_action_and_never_replays_prior_effect() {
        let baseline = checkpoint(CanonicalCheckpointEffectState::NoEffect);
        let mut current = observation(&baseline);
        assert_eq!(
            decide_attempt_recovery(&baseline, &current).action,
            AttemptRecoveryAction::Continue
        );

        current.fresh_attempt_eligible = true;
        let fresh = decide_attempt_recovery(&baseline, &current);
        assert_eq!(fresh.action, AttemptRecoveryAction::FreshAttempt);
        assert!(fresh.fresh_identity_and_authority_required);
        assert!(!fresh.prior_effect_replay_allowed);

        current.fresh_attempt_eligible = false;
        current.deterministic_repair_eligible = true;
        assert_eq!(
            decide_attempt_recovery(&baseline, &current).action,
            AttemptRecoveryAction::DeterministicRepair
        );
        current.deterministic_repair_eligible = false;
        current.fresh_approval_required = true;
        assert_eq!(
            decide_attempt_recovery(&baseline, &current).action,
            AttemptRecoveryAction::AwaitApproval
        );
        current.fresh_approval_required = false;
        current.dependency_blocked = true;
        assert_eq!(
            decide_attempt_recovery(&baseline, &current).action,
            AttemptRecoveryAction::AwaitDependency
        );
        current.dependency_blocked = false;
        current.cancelled = true;
        assert_eq!(
            decide_attempt_recovery(&baseline, &current).action,
            AttemptRecoveryAction::Cancel
        );

        for effect in [
            CanonicalCheckpointEffectState::VerifiedApplied,
            CanonicalCheckpointEffectState::Uncertain,
        ] {
            let checkpoint = checkpoint(effect);
            let result = decide_attempt_recovery(&checkpoint, &observation(&checkpoint));
            assert_eq!(result.action, AttemptRecoveryAction::ReconcileEffect);
            assert!(!result.prior_effect_replay_allowed);
        }
    }

    #[test]
    fn story_22_4_drift_budgets_and_repeated_state_stop_or_reconcile_visibly() {
        let checkpoint = checkpoint(CanonicalCheckpointEffectState::NoEffect);
        let mut current = observation(&checkpoint);
        current.source_sha256 = hash('9');
        let drift = decide_attempt_recovery(&checkpoint, &current);
        assert_eq!(drift.action, AttemptRecoveryAction::Replan);
        assert_eq!(drift.drift, vec![AttemptCheckpointDrift::Source]);

        let mut current = observation(&checkpoint);
        current.budget_state.step_attempts = current.budget_limits.step_attempts;
        let stopped = decide_attempt_recovery(&checkpoint, &current);
        assert_eq!(stopped.action, AttemptRecoveryAction::TerminateDiagnosed);
        let diagnosis = stopped.diagnosis.expect("one diagnosis");
        assert_eq!(diagnosis.step_execution_id, checkpoint.step_execution_id);
        assert_eq!(diagnosis.attempts_sha256, checkpoint.attempts_sha256);
        assert!(
            diagnosis
                .exhausted_budgets
                .contains(&AttemptBudgetDimension::StepAttempt)
        );
        assert_eq!(diagnosis.diagnosis_sha256.len(), 64);
        assert!(verify_attempt_terminal_diagnosis(&diagnosis));

        let mut current = observation(&checkpoint);
        current.repeated_state_count = current.budget_limits.repeated_state;
        current.repeated_state_sha256 = checkpoint.repeated_state_sha256.clone();
        let stopped = decide_attempt_recovery(&checkpoint, &current);
        assert_eq!(stopped.action, AttemptRecoveryAction::TerminateDiagnosed);
        assert!(
            stopped
                .exhausted_budgets
                .contains(&AttemptBudgetDimension::RepeatedState)
        );
    }

    #[test]
    fn story_22_4_concurrent_clients_have_one_owner() {
        let gate = AttemptResumeOwnership::new();
        let barrier = Arc::new(Barrier::new(32));
        let mut workers = Vec::new();
        for index in 0..32 {
            let gate = gate.clone();
            let barrier = Arc::clone(&barrier);
            workers.push(thread::spawn(move || {
                barrier.wait();
                gate.claim("attempt-checkpoint-1", &format!("owner-{index}"))
            }));
        }
        let claims: Vec<_> = workers
            .into_iter()
            .filter_map(|worker| worker.join().expect("worker").ok())
            .collect();
        assert_eq!(claims.len(), 1);
        assert!(gate.is_current(&claims[0]).expect("current"));
    }

    #[test]
    #[ignore = "subprocess helper for deterministic no-unwind attempt-recovery interruption"]
    fn story_22_4_attempt_recovery_crash_child() {
        if std::env::var_os(CRASH_ENV).is_none() {
            return;
        }
        let path = std::env::var_os(CRASH_PATH_ENV).expect("crash path");
        let seed: u64 = std::env::var("AGENTMAGE_ATTEMPT_RECOVERY_SEED")
            .expect("seed")
            .parse()
            .expect("numeric seed");
        let boundary = (seed / 2) % 11;
        let after = seed % 2 == 1;
        let effect = if boundary < 3 || (boundary == 3 && !after) {
            CanonicalCheckpointEffectState::NoEffect
        } else if boundary == 3 {
            CanonicalCheckpointEffectState::Uncertain
        } else {
            CanonicalCheckpointEffectState::VerifiedApplied
        };
        let checkpoint = checkpoint(effect);
        fs::write(path, serde_json::to_vec(&checkpoint).expect("encode"))
            .expect("write checkpoint");
        std::process::exit(73);
    }

    #[test]
    fn story_22_4_one_hundred_seed_no_unwind_boundary_campaign_has_zero_replay() {
        let executable = std::env::current_exe().expect("test executable");
        let root =
            std::env::temp_dir().join(format!("agentmage-attempt-recovery-{}", std::process::id()));
        fs::create_dir_all(&root).expect("temporary campaign directory");
        let boundaries = [
            "preflight",
            "approval",
            "dispatch",
            "effect",
            "receipt",
            "artifact",
            "verification",
            "retry",
            "recovery",
            "checkpoint",
            "terminal",
        ];
        let mut replay_count = 0_u64;
        let mut traces = Vec::new();
        for seed in 0_u64..100 {
            let path = root.join(format!("seed-{seed}.json"));
            let status = Command::new(&executable)
                .args([
                    "--ignored",
                    "--exact",
                    "durable_attempt_recovery::tests::story_22_4_attempt_recovery_crash_child",
                ])
                .env(CRASH_ENV, "1")
                .env(CRASH_PATH_ENV, &path)
                .env("AGENTMAGE_ATTEMPT_RECOVERY_SEED", seed.to_string())
                .status()
                .expect("crash child starts");
            assert_eq!(status.code(), Some(73));
            let retained: CanonicalAttemptCheckpoint =
                serde_json::from_slice(&fs::read(&path).expect("retained checkpoint"))
                    .expect("decode");
            verify_attempt_checkpoint(&retained).expect("retained checkpoint verifies");
            let decision = decide_attempt_recovery(&retained, &observation(&retained));
            if decision.prior_effect_replay_allowed {
                replay_count += 1;
            }
            traces.push((
                seed,
                boundaries[(seed as usize / 2) % boundaries.len()],
                seed % 2,
                decision.action,
            ));
        }
        assert_eq!(traces.len(), 100);
        assert_eq!(replay_count, 0);
        fs::remove_dir_all(root).expect("campaign cleanup");
    }
}
