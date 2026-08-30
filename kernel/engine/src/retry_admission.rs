//! Pure fresh-attempt admission after current evidence and authority checks.

use std::collections::BTreeSet;

use agentmage_kernel_contracts::{
    ApprovalId, ApprovalRequest, CanonicalApprovalRequirement, CanonicalRecoveryAction,
    CanonicalRecoveryDecision, CanonicalRetryAdmission, CanonicalRetryClass,
    CanonicalStepExecutionPolicy, CapabilityGrant, GrantClass, GrantStatus,
};

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
    /// Candidate, decision, and policy identities do not describe the same step.
    BindingMismatch,
    /// The recovery decision does not permit a new attempt.
    DecisionNotRetry,
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
        now_epoch_ms,
    } = input;

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
    if !candidate.opens_new_attempt()
        || candidate
            .prior_attempt_ordinal
            .checked_add(1)
            .is_none_or(|ordinal| ordinal != candidate.successor_attempt_ordinal)
    {
        return Err(FreshAttemptAdmissionError::ReplayedIdentity);
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
        if approval.approval_id.as_str() != successor_approval_id
            || prior_approval_id.is_some_and(|prior| prior == &approval.approval_id)
            || approval.proposed_grant_id != grant.grant_id
            || grant.approval_id.as_ref() != Some(&approval.approval_id)
            || approval.policy_sha256 != policy.policy_sha256
            || approval.tool_call.tool_call_id != candidate.successor_tool_call_id
            || approval.issued_at_epoch_ms > now_epoch_ms
            || now_epoch_ms >= approval.expires_at_epoch_ms
            || approval.expires_at_epoch_ms != grant.expires_at_epoch_ms
        {
            return Err(FreshAttemptAdmissionError::FreshApprovalRequired);
        }
    } else if successor_approval.is_some() {
        return Err(FreshAttemptAdmissionError::FreshApprovalRequired);
    }

    Ok(candidate)
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
