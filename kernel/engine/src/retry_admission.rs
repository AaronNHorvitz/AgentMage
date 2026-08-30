//! Pure fresh-attempt admission after current evidence and authority checks.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::sync::{Arc, Mutex};

use agentmage_kernel_contracts::{
    ApprovalId, ApprovalRequest, CanonicalApprovalRequirement, CanonicalIdempotencyRequirement,
    CanonicalRecoveryAction, CanonicalRecoveryDecision, CanonicalRetryAdmission,
    CanonicalRetryClass, CanonicalStepExecutionPolicy, CapabilityGrant, GrantClass, GrantStatus,
};

use serde::Serialize;
use sha2::{Digest, Sha256};

const ZERO_SHA256: &str = "0000000000000000000000000000000000000000000000000000000000000000";

/// Current deterministic results for every preflight required by one step policy.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CurrentPreflightEvidence {
    step_execution_id: String,
    policy_sha256: String,
    completed_preflight_ids: Vec<String>,
    observed_at_epoch_ms: u64,
    valid_until_epoch_ms: u64,
}

impl CurrentPreflightEvidence {
    /// Constructs bounded, content-free preflight evidence.
    pub fn new(
        step_execution_id: String,
        policy_sha256: String,
        completed_preflight_ids: Vec<String>,
        observed_at_epoch_ms: u64,
        valid_until_epoch_ms: u64,
    ) -> Result<Self, FreshAttemptAdmissionError> {
        if !valid_identifier(&step_execution_id)
            || !valid_sha256(&policy_sha256)
            || completed_preflight_ids.is_empty()
            || completed_preflight_ids.len() > 32
            || completed_preflight_ids
                .iter()
                .any(|identity| !valid_identifier(identity))
            || !all_unique(&completed_preflight_ids)
            || observed_at_epoch_ms >= valid_until_epoch_ms
        {
            return Err(FreshAttemptAdmissionError::InvalidPreflightEvidence);
        }
        Ok(Self {
            step_execution_id,
            policy_sha256,
            completed_preflight_ids,
            observed_at_epoch_ms,
            valid_until_epoch_ms,
        })
    }
}

/// Closed result of reconciling whether another attempt could duplicate a prior effect.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EffectReconciliationDisposition {
    /// Current deterministic evidence proves a fresh attempt cannot duplicate the prior effect.
    SafeForFreshAttempt,
    /// The prior effect remains uncertain.
    EffectUncertain,
    /// The desired effect is already present, so another attempt is unnecessary.
    DesiredStateAlreadyPresent,
}

/// Current deterministic reconciliation evidence for the immediately preceding attempt.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CurrentEffectReconciliation {
    prior_attempt_id: String,
    policy_sha256: String,
    observation_sha256: String,
    disposition: EffectReconciliationDisposition,
    observed_at_epoch_ms: u64,
    valid_until_epoch_ms: u64,
}

impl CurrentEffectReconciliation {
    /// Constructs content-bound reconciliation evidence without granting retry authority.
    pub fn new(
        prior_attempt_id: String,
        policy_sha256: String,
        observation_sha256: String,
        disposition: EffectReconciliationDisposition,
        observed_at_epoch_ms: u64,
        valid_until_epoch_ms: u64,
    ) -> Result<Self, FreshAttemptAdmissionError> {
        if !valid_identifier(&prior_attempt_id)
            || !valid_sha256(&policy_sha256)
            || !valid_sha256(&observation_sha256)
            || observed_at_epoch_ms >= valid_until_epoch_ms
        {
            return Err(FreshAttemptAdmissionError::InvalidReconciliationEvidence);
        }
        Ok(Self {
            prior_attempt_id,
            policy_sha256,
            observation_sha256,
            disposition,
            observed_at_epoch_ms,
            valid_until_epoch_ms,
        })
    }

    /// Returns the digest of the exact current-state observation.
    #[must_use]
    pub fn observation_sha256(&self) -> &str {
        &self.observation_sha256
    }
}

/// Read-only view of one exact issued operation grant used by admission compilation.
#[derive(Clone, Copy, Debug)]
pub struct FreshSingleUseGrant<'a> {
    grant: &'a CapabilityGrant,
}

impl<'a> FreshSingleUseGrant<'a> {
    /// Wraps an existing canonical grant; all currentness checks occur during compilation.
    #[must_use]
    pub const fn new(grant: &'a CapabilityGrant) -> Self {
        Self { grant }
    }
}

/// Read-only view of one exact approval request used by admission compilation.
#[derive(Clone, Copy, Debug)]
pub struct CurrentAttemptApproval<'a> {
    approval: &'a ApprovalRequest,
}

impl<'a> CurrentAttemptApproval<'a> {
    /// Wraps an existing canonical approval; all freshness checks occur during compilation.
    #[must_use]
    pub const fn new(approval: &'a ApprovalRequest) -> Self {
        Self { approval }
    }
}

/// Complete prior-use ledger consulted before issuing any successor identity.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PriorExecutionIdentityLedger {
    attempt_ids: BTreeSet<String>,
    call_ids: BTreeSet<String>,
    tool_call_ids: BTreeSet<String>,
    grant_ids: BTreeSet<String>,
    approval_ids: BTreeSet<String>,
    receipt_ids: BTreeSet<String>,
    idempotency_key_sha256s: BTreeSet<String>,
}

impl PriorExecutionIdentityLedger {
    /// Constructs a closed identity ledger and rejects malformed or repeated entries.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        attempt_ids: Vec<String>,
        call_ids: Vec<String>,
        tool_call_ids: Vec<String>,
        grant_ids: Vec<String>,
        approval_ids: Vec<String>,
        receipt_ids: Vec<String>,
        idempotency_key_sha256s: Vec<String>,
    ) -> Result<Self, FreshAttemptAdmissionError> {
        if !valid_identity_list(&attempt_ids)
            || !valid_identity_list(&call_ids)
            || !valid_identity_list(&tool_call_ids)
            || !valid_identity_list(&grant_ids)
            || !valid_identity_list(&approval_ids)
            || !valid_identity_list(&receipt_ids)
            || idempotency_key_sha256s.len() > 1_024
            || idempotency_key_sha256s
                .iter()
                .any(|digest| !valid_sha256(digest))
            || !all_unique(&idempotency_key_sha256s)
        {
            return Err(FreshAttemptAdmissionError::InvalidIdentityLedger);
        }
        Ok(Self {
            attempt_ids: attempt_ids.into_iter().collect(),
            call_ids: call_ids.into_iter().collect(),
            tool_call_ids: tool_call_ids.into_iter().collect(),
            grant_ids: grant_ids.into_iter().collect(),
            approval_ids: approval_ids.into_iter().collect(),
            receipt_ids: receipt_ids.into_iter().collect(),
            idempotency_key_sha256s: idempotency_key_sha256s.into_iter().collect(),
        })
    }
}

/// Complete non-executing input to one fresh-attempt admission decision.
#[derive(Debug)]
pub struct FreshAttemptAdmissionInput<'a> {
    /// Candidate canonical admission to return only if every prerequisite passes.
    pub candidate: CanonicalRetryAdmission,
    /// Exact immutable step policy in force.
    pub policy: &'a CanonicalStepExecutionPolicy,
    /// Exact recovery decision that selected a new attempt.
    pub decision: &'a CanonicalRecoveryDecision,
    /// Current deterministic preflight evidence.
    pub preflight: &'a CurrentPreflightEvidence,
    /// Current effect reconciliation when the retry class requires it.
    pub reconciliation: Option<&'a CurrentEffectReconciliation>,
    /// Exact fresh single-use grant candidate.
    pub successor_grant: FreshSingleUseGrant<'a>,
    /// Current approval when the policy requires a fresh approval per attempt.
    pub successor_approval: Option<CurrentAttemptApproval<'a>>,
    /// Approval used by the prior attempt, when one existed.
    pub prior_approval_id: Option<&'a ApprovalId>,
    /// Complete prior-use identity ledger for this step execution.
    pub prior_identities: &'a PriorExecutionIdentityLedger,
    /// Fresh idempotency-key digest when the step policy requires one.
    pub successor_idempotency_key_sha256: Option<&'a str>,
    /// Any receipt presented for the not-yet-executed successor; this must remain absent.
    pub presented_successor_receipt_id: Option<&'a str>,
    /// Trusted current time used for every freshness decision.
    pub now_epoch_ms: u64,
}

/// Closed denials returned before a new attempt is admitted.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FreshAttemptAdmissionError {
    /// Supplied preflight evidence is malformed.
    InvalidPreflightEvidence,
    /// Supplied reconciliation evidence is malformed.
    InvalidReconciliationEvidence,
    /// Prior-use identity evidence is malformed, duplicated, or oversized.
    InvalidIdentityLedger,
    /// A canonical policy, decision, admission, or approval digest does not match its exact bytes.
    IntegrityMismatch,
    /// Candidate, decision, and policy identities do not describe the same step.
    BindingMismatch,
    /// The recovery decision does not permit a new attempt.
    DecisionNotRetry,
    /// The effect or retry class forbids automatic admission.
    AutomaticRetryForbidden,
    /// A prior call, tool call, grant, or attempt identity would be replayed.
    ReplayedIdentity,
    /// Current preflight evidence is missing, stale, or incomplete.
    PreflightNotCurrent,
    /// Required effect reconciliation is missing, stale, or unsafe.
    ReconciliationNotCurrent,
    /// The step has no remaining attempt budget.
    AttemptBudgetExhausted,
    /// The successor grant is not exact, fresh, issued, and single use.
    GrantNotFreshSingleUse,
    /// The approval requirement differs from the governing policy.
    ApprovalRequirementMismatch,
    /// A required per-attempt approval is absent, stale, mismatched, or reused.
    FreshApprovalRequired,
    /// Required idempotency evidence is missing, malformed, or already used.
    FreshIdempotencyEvidenceRequired,
    /// A receipt was presented before the successor attempt executed.
    PrematureReceipt,
}

/// Compiles one canonical successor admission only after every current prerequisite passes.
///
/// This function does not consume the grant or dispatch the successor. Atomic grant consumption
/// remains the authority transaction's responsibility immediately before effect execution.
pub fn compile_fresh_attempt_admission(
    input: FreshAttemptAdmissionInput<'_>,
) -> Result<CanonicalRetryAdmission, FreshAttemptAdmissionError> {
    let FreshAttemptAdmissionInput {
        candidate,
        policy,
        decision,
        preflight,
        reconciliation,
        successor_grant,
        successor_approval,
        prior_approval_id,
        prior_identities,
        successor_idempotency_key_sha256,
        presented_successor_receipt_id,
        now_epoch_ms,
    } = input;

    if !policy_integrity(policy)
        || !decision_integrity(decision)
        || !admission_integrity(&candidate)
    {
        return Err(FreshAttemptAdmissionError::IntegrityMismatch);
    }

    if candidate.policy_id != policy.policy_id
        || candidate.step_execution_id != preflight.step_execution_id
        || candidate.decision_id != decision.decision_id
        || candidate.step_execution_id != decision.step_execution_id
        || candidate.policy_id != decision.policy_id
        || candidate.prior_attempt_id != decision.attempt_id
    {
        return Err(FreshAttemptAdmissionError::BindingMismatch);
    }
    if decision.decision != CanonicalRecoveryAction::RetryNewAttempt
        || decision.uncertain_outcome
        || !decision.failure_class.is_transient()
    {
        return Err(FreshAttemptAdmissionError::DecisionNotRetry);
    }
    if !policy.effect_class.permits_automatic_retry()
        || !policy.retry_class.is_automatic()
        || !policy
            .effect_class
            .permitted_retry_classes()
            .contains(&policy.retry_class)
    {
        return Err(FreshAttemptAdmissionError::AutomaticRetryForbidden);
    }
    if !candidate.opens_new_attempt()
        || candidate
            .prior_attempt_ordinal
            .checked_add(1)
            .is_none_or(|ordinal| ordinal != candidate.successor_attempt_ordinal)
    {
        return Err(FreshAttemptAdmissionError::ReplayedIdentity);
    }
    if prior_identities
        .attempt_ids
        .contains(&candidate.successor_attempt_id)
        || prior_identities
            .call_ids
            .contains(&candidate.successor_call_id)
        || prior_identities
            .tool_call_ids
            .contains(candidate.successor_tool_call_id.as_str())
        || prior_identities
            .grant_ids
            .contains(candidate.successor_grant_id.as_str())
        || candidate
            .successor_approval_id
            .as_ref()
            .is_some_and(|id| prior_identities.approval_ids.contains(id))
    {
        return Err(FreshAttemptAdmissionError::ReplayedIdentity);
    }
    if let Some(receipt_id) = presented_successor_receipt_id {
        if !valid_identifier(receipt_id) || prior_identities.receipt_ids.contains(receipt_id) {
            return Err(FreshAttemptAdmissionError::ReplayedIdentity);
        }
        return Err(FreshAttemptAdmissionError::PrematureReceipt);
    }
    if preflight.policy_sha256 != policy.policy_sha256
        || preflight.completed_preflight_ids != policy.required_preflight_ids
        || preflight.observed_at_epoch_ms > now_epoch_ms
        || now_epoch_ms >= preflight.valid_until_epoch_ms
    {
        return Err(FreshAttemptAdmissionError::PreflightNotCurrent);
    }

    let reconciliation_required =
        policy.retry_class == CanonicalRetryClass::ConditionalAfterReconciliation;
    if candidate.reconciliation_required != reconciliation_required
        || candidate.reconciled != reconciliation_required
    {
        return Err(FreshAttemptAdmissionError::ReconciliationNotCurrent);
    }
    match (reconciliation_required, reconciliation) {
        (true, Some(evidence))
            if evidence.prior_attempt_id == candidate.prior_attempt_id
                && evidence.policy_sha256 == policy.policy_sha256
                && evidence.disposition == EffectReconciliationDisposition::SafeForFreshAttempt
                && evidence.observed_at_epoch_ms <= now_epoch_ms
                && now_epoch_ms < evidence.valid_until_epoch_ms => {}
        (false, None) => {}
        _ => return Err(FreshAttemptAdmissionError::ReconciliationNotCurrent),
    }

    if u64::from(candidate.prior_attempt_ordinal) >= policy.budgets.attempts
        || u64::from(candidate.successor_attempt_ordinal) > policy.budgets.attempts
    {
        return Err(FreshAttemptAdmissionError::AttemptBudgetExhausted);
    }

    let grant = successor_grant.grant;
    if grant.grant_id.as_str() != candidate.successor_grant_id.as_str()
        || grant.grant_id.as_str() == candidate.prior_grant_id.as_str()
        || grant.grant_class != GrantClass::Operation
        || grant.revision != 1
        || grant.status != GrantStatus::Issued
        || grant.use_limit != 1
        || grant.use_count != 0
        || grant.policy_sha256 != policy.policy_sha256
        || grant.issued_at_epoch_ms > now_epoch_ms
        || now_epoch_ms >= grant.expires_at_epoch_ms
    {
        return Err(FreshAttemptAdmissionError::GrantNotFreshSingleUse);
    }

    if candidate.approval_requirement != policy.approval_requirement {
        return Err(FreshAttemptAdmissionError::ApprovalRequirementMismatch);
    }
    if policy.approval_requirement == CanonicalApprovalRequirement::RequiredPerAttempt {
        let Some(successor_approval_id) = candidate.successor_approval_id.as_deref() else {
            return Err(FreshAttemptAdmissionError::FreshApprovalRequired);
        };
        let Some(approval) = successor_approval.map(|value| value.approval) else {
            return Err(FreshAttemptAdmissionError::FreshApprovalRequired);
        };
        if !approval_integrity(approval)
            || approval.approval_id.as_str() != successor_approval_id
            || prior_approval_id.is_some_and(|prior| prior == &approval.approval_id)
            || approval.proposed_grant_id != grant.grant_id
            || grant.approval_id.as_ref() != Some(&approval.approval_id)
            || approval.parent_grant_id.as_str()
                != grant
                    .parent_grant_id
                    .as_ref()
                    .map_or("", agentmage_kernel_contracts::GrantId::as_str)
            || approval.parent_grant_sha256.as_str()
                != grant.parent_grant_sha256.as_deref().unwrap_or_default()
            || approval.actor_id != grant.actor_id
            || approval.session_id != grant.session_id
            || approval.task_id != grant.task_id
            || grant.action_kind != Some(approval.action_kind)
            || grant.action_id.as_ref() != Some(&approval.tool_call.action_id)
            || approval.operation != grant.operation
            || grant.tool_id.as_ref() != Some(&approval.tool_call.tool_id)
            || grant.tool_version.as_deref() != Some(approval.tool_call.tool_version.as_str())
            || approval.tool_call.arguments.sha256 != grant.argument_sha256
            || approval.targets != grant.targets
            || approval.excluded_targets != grant.excluded_targets
            || approval.sensitivity != grant.sensitivity
            || approval.preimages != grant.preimages
            || approval.expected_side_effects != grant.expected_side_effects
            || approval.rollback_description != grant.rollback_description
            || approval.policy_sha256 != policy.policy_sha256
            || approval.tool_call.tool_call_id != candidate.successor_tool_call_id
            || approval.issued_at_epoch_ms > now_epoch_ms
            || now_epoch_ms >= approval.expires_at_epoch_ms
            || approval.issued_at_epoch_ms != grant.issued_at_epoch_ms
            || approval.expires_at_epoch_ms != grant.expires_at_epoch_ms
        {
            return Err(FreshAttemptAdmissionError::FreshApprovalRequired);
        }
    } else if successor_approval.is_some() {
        return Err(FreshAttemptAdmissionError::FreshApprovalRequired);
    }

    match policy.idempotency_key_requirement {
        CanonicalIdempotencyRequirement::Required => {
            let Some(digest) = successor_idempotency_key_sha256 else {
                return Err(FreshAttemptAdmissionError::FreshIdempotencyEvidenceRequired);
            };
            if !valid_sha256(digest) || prior_identities.idempotency_key_sha256s.contains(digest) {
                return Err(FreshAttemptAdmissionError::FreshIdempotencyEvidenceRequired);
            }
        }
        CanonicalIdempotencyRequirement::NotApplicable
        | CanonicalIdempotencyRequirement::VerifiedDesiredState => {
            if successor_idempotency_key_sha256.is_some() {
                return Err(FreshAttemptAdmissionError::FreshIdempotencyEvidenceRequired);
            }
        }
    }

    Ok(candidate)
}

/// Opaque non-cloneable proof that every fresh-attempt prerequisite passed together.
#[derive(Debug)]
pub struct AdmittedFreshAttempt {
    admission: CanonicalRetryAdmission,
}

impl AdmittedFreshAttempt {
    /// Returns the admitted canonical record without exposing an execution constructor.
    #[must_use]
    pub const fn admission(&self) -> &CanonicalRetryAdmission {
        &self.admission
    }
}

/// Compiles an opaque execution-ready proof through the same pure admission boundary.
pub fn compile_execution_ready_attempt(
    input: FreshAttemptAdmissionInput<'_>,
) -> Result<AdmittedFreshAttempt, FreshAttemptAdmissionError> {
    compile_fresh_attempt_admission(input).map(|admission| AdmittedFreshAttempt { admission })
}

/// Closed outcome reported by the exact admitted effect boundary.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AttemptEffectOutcome {
    /// The exact successor effect completed successfully.
    Succeeded,
    /// The exact successor effect failed with a known non-success outcome.
    Failed,
    /// Whether the exact successor effect occurred cannot be established.
    Uncertain,
}

/// Content-free receipt proving which exact attempt entered the guarded effect boundary.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AttemptExecutionReceipt {
    /// Step execution identity.
    pub step_execution_id: String,
    /// Predecessor attempt consumed by this successor decision.
    pub prior_attempt_id: String,
    /// Exact successor attempt that entered execution.
    pub successor_attempt_id: String,
    /// Exact terminal effect outcome, including uncertainty without coercion.
    pub outcome: AttemptEffectOutcome,
}

/// Atomic execution-gate refusal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AttemptExecutionGateError {
    /// This predecessor step/attempt already admitted one exact successor to execution.
    PriorAttemptAlreadyEntered,
    /// Kernel synchronization state is unavailable and therefore fails closed.
    GateUnavailable,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AttemptExecutionState {
    Running,
    Terminal(AttemptEffectOutcome),
}

/// Process-safe-in-one-runtime gate admitting at most one successor for a predecessor attempt.
#[derive(Clone, Debug, Default)]
pub struct SynchronizedAttemptExecutionGate {
    states: Arc<Mutex<BTreeMap<(String, String), AttemptExecutionState>>>,
}

impl SynchronizedAttemptExecutionGate {
    /// Creates an empty gate.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Atomically claims the predecessor and runs one exact effect callback at most once.
    pub fn execute<F>(
        &self,
        admitted: AdmittedFreshAttempt,
        effect: F,
    ) -> Result<AttemptExecutionReceipt, AttemptExecutionGateError>
    where
        F: FnOnce() -> AttemptEffectOutcome,
    {
        let key = (
            admitted.admission.step_execution_id.clone(),
            admitted.admission.prior_attempt_id.clone(),
        );
        {
            let mut states = self
                .states
                .lock()
                .map_err(|_| AttemptExecutionGateError::GateUnavailable)?;
            if states.contains_key(&key) {
                return Err(AttemptExecutionGateError::PriorAttemptAlreadyEntered);
            }
            states.insert(key.clone(), AttemptExecutionState::Running);
        }

        let outcome = effect();
        let mut states = self
            .states
            .lock()
            .map_err(|_| AttemptExecutionGateError::GateUnavailable)?;
        states.insert(key, AttemptExecutionState::Terminal(outcome));
        Ok(AttemptExecutionReceipt {
            step_execution_id: admitted.admission.step_execution_id,
            prior_attempt_id: admitted.admission.prior_attempt_id,
            successor_attempt_id: admitted.admission.successor_attempt_id,
            outcome,
        })
    }

    /// Returns the terminal result retained for an exact predecessor, if established.
    pub fn outcome(
        &self,
        step_execution_id: &str,
        prior_attempt_id: &str,
    ) -> Result<Option<AttemptEffectOutcome>, AttemptExecutionGateError> {
        let states = self
            .states
            .lock()
            .map_err(|_| AttemptExecutionGateError::GateUnavailable)?;
        Ok(
            match states.get(&(step_execution_id.to_owned(), prior_attempt_id.to_owned())) {
                Some(AttemptExecutionState::Terminal(outcome)) => Some(*outcome),
                Some(AttemptExecutionState::Running) | None => None,
            },
        )
    }
}

fn policy_integrity(policy: &CanonicalStepExecutionPolicy) -> bool {
    if !valid_sha256(&policy.policy_sha256) {
        return false;
    }
    let mut candidate = policy.clone();
    candidate.policy_sha256 = ZERO_SHA256.to_owned();
    record_sha256(&candidate).is_some_and(|digest| digest == policy.policy_sha256)
}

fn decision_integrity(decision: &CanonicalRecoveryDecision) -> bool {
    if !valid_sha256(&decision.decision_sha256) {
        return false;
    }
    let mut candidate = decision.clone();
    candidate.decision_sha256 = ZERO_SHA256.to_owned();
    record_sha256(&candidate).is_some_and(|digest| digest == decision.decision_sha256)
}

fn admission_integrity(admission: &CanonicalRetryAdmission) -> bool {
    if !valid_sha256(&admission.admission_sha256) {
        return false;
    }
    let mut candidate = admission.clone();
    candidate.admission_sha256 = ZERO_SHA256.to_owned();
    record_sha256(&candidate).is_some_and(|digest| digest == admission.admission_sha256)
}

fn approval_integrity(approval: &ApprovalRequest) -> bool {
    if !valid_sha256(&approval.confirmation_sha256) {
        return false;
    }
    let mut candidate = approval.clone();
    candidate.confirmation_sha256 = ZERO_SHA256.to_owned();
    record_sha256(&candidate).is_some_and(|digest| digest == approval.confirmation_sha256)
}

fn record_sha256(value: &impl Serialize) -> Option<String> {
    let bytes = serde_json::to_vec(value).ok()?;
    let mut output = String::with_capacity(64);
    for byte in Sha256::digest(bytes) {
        let _ = write!(output, "{byte:02x}");
    }
    Some(output)
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

fn all_unique(values: &[String]) -> bool {
    values.iter().collect::<BTreeSet<_>>().len() == values.len()
}

fn valid_identity_list(values: &[String]) -> bool {
    values.len() <= 1_024
        && values.iter().all(|value| valid_identifier(value))
        && all_unique(values)
}
