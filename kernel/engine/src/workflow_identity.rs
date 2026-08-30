//! Atomic issuance and replay denial for workflow execution identities.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::sync::Mutex;

use agentmage_kernel_contracts::{
    ApprovalId, CONTRACT_SCHEMA_VERSION, CanonicalApprovalRequirement, CanonicalRetryClass,
    CanonicalStepExecutionPolicy, GrantId, ReceiptId, ToolCallId, to_canonical_json,
};
use sha2::{Digest, Sha256};

const MAX_IDENTITIES: usize = 1_000_000;
const ZERO_SHA256: &str = "0000000000000000000000000000000000000000000000000000000000000000";

/// How a proposed attempt relates to its step's prior execution history.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AttemptOpeningKind {
    /// First attempt for a step.
    Initial,
    /// Runtime-requested retry under the exact automatic retry policy.
    AutomaticRetry,
    /// Retry separately authorized by a fresh user approval.
    UserApprovedRetry,
}

/// Current deterministic reconciliation state before opening a successor attempt.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AttemptReconciliationState {
    /// No effect reconciliation is required for this attempt.
    NotRequired,
    /// Current evidence proves that a new attempt cannot duplicate an effect.
    SafeForFreshAttempt,
    /// Whether the prior attempt produced an effect remains unknown.
    Uncertain,
}

/// Proposed identities for a not-yet-executed attempt.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AttemptIdentityCandidate {
    /// Step execution receiving the attempt.
    pub step_execution_id: String,
    /// One-based ordinal within the step execution.
    pub attempt_ordinal: u32,
    /// Exact predecessor identity, absent only for an initial attempt.
    pub predecessor_attempt_id: Option<String>,
    /// Fresh call-envelope identity.
    pub call_id: String,
    /// Fresh existing tool-call identity.
    pub tool_call_id: ToolCallId,
    /// Fresh operation-attempt identity.
    pub attempt_id: String,
    /// Fresh single-use operation-grant identity.
    pub grant_id: GrantId,
    /// Fresh approval identity when the attempt requires approval.
    pub approval_id: Option<ApprovalId>,
    /// Initial, automatic-retry, or separately approved-retry origin.
    pub opening: AttemptOpeningKind,
    /// Current reconciliation result for a successor attempt.
    pub reconciliation: AttemptReconciliationState,
}

/// Opaque, non-cloneable proof that all pre-effect identities were reserved atomically.
///
/// This is an identity proof only. It contains no capability, grant contents, or dispatch method.
#[derive(Debug, PartialEq, Eq)]
pub struct IssuedAttemptIdentities {
    execution_id: String,
    step_execution_id: String,
    attempt_ordinal: u32,
    call_id: String,
    tool_call_id: ToolCallId,
    attempt_id: String,
    grant_id: GrantId,
    approval_id: Option<ApprovalId>,
}

impl IssuedAttemptIdentities {
    /// Returns the workflow execution identity.
    #[must_use]
    pub fn execution_id(&self) -> &str {
        &self.execution_id
    }

    /// Returns the step-execution identity.
    #[must_use]
    pub fn step_execution_id(&self) -> &str {
        &self.step_execution_id
    }

    /// Returns the exact one-based attempt ordinal.
    #[must_use]
    pub const fn attempt_ordinal(&self) -> u32 {
        self.attempt_ordinal
    }

    /// Returns the fresh call identity.
    #[must_use]
    pub fn call_id(&self) -> &str {
        &self.call_id
    }

    /// Returns the fresh tool-call identity.
    #[must_use]
    pub const fn tool_call_id(&self) -> &ToolCallId {
        &self.tool_call_id
    }

    /// Returns the fresh attempt identity.
    #[must_use]
    pub fn attempt_id(&self) -> &str {
        &self.attempt_id
    }

    /// Returns the fresh grant identity.
    #[must_use]
    pub const fn grant_id(&self) -> &GrantId {
        &self.grant_id
    }

    /// Returns the fresh approval identity when policy required one.
    #[must_use]
    pub const fn approval_id(&self) -> Option<&ApprovalId> {
        self.approval_id.as_ref()
    }
}

/// Closed terminal effect state associated with a fresh receipt identity.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AttemptTerminalEffect {
    /// The attempt completed with a known effect state.
    Succeeded,
    /// The attempt failed with a known effect state.
    Failed,
    /// Whether the attempt produced an effect cannot be established.
    Uncertain,
}

/// Opaque, non-cloneable proof that one attempt received one fresh terminal receipt identity.
#[derive(Debug, PartialEq, Eq)]
pub struct IssuedReceiptIdentity {
    execution_id: String,
    step_execution_id: String,
    attempt_id: String,
    receipt_id: ReceiptId,
    effect: AttemptTerminalEffect,
}

impl IssuedReceiptIdentity {
    /// Returns the exact attempt resolved by this receipt identity.
    #[must_use]
    pub fn attempt_id(&self) -> &str {
        &self.attempt_id
    }

    /// Returns the fresh receipt identity.
    #[must_use]
    pub const fn receipt_id(&self) -> &ReceiptId {
        &self.receipt_id
    }

    /// Returns the terminal effect classification without collapsing uncertainty.
    #[must_use]
    pub const fn effect(&self) -> AttemptTerminalEffect {
        self.effect
    }
}

/// Fresh verification identity bound to one exact attempt receipt and verifier policy.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IssuedVerificationIdentity {
    /// Workflow execution identity.
    pub execution_id: String,
    /// Step-execution identity.
    pub step_execution_id: String,
    /// Attempt identity whose receipt is being verified.
    pub attempt_id: String,
    /// Exact receipt identity observed by the verifier.
    pub receipt_id: ReceiptId,
    /// Fresh verification-envelope identity.
    pub verification_id: String,
    /// Exact verifier policy receiving this one identity.
    pub verifier_policy_id: String,
}

/// Stable content-free denial returned by the identity issuer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorkflowIdentityError {
    /// Issuer or candidate identity syntax is malformed or capacity is exhausted.
    InvalidIdentity,
    /// Step policy or attempt binding is malformed or inconsistent.
    PolicyBindingMismatch,
    /// The attempt ordinal or predecessor is not the exact next chain member.
    AttemptChainMismatch,
    /// A prior attempt is still running and therefore cannot have a successor.
    PriorAttemptNotTerminal,
    /// An identity was previously issued in any execution identity role.
    ReplayedIdentity,
    /// Current policy requires a fresh per-attempt approval identity.
    FreshApprovalRequired,
    /// Approval was supplied where current policy does not require one.
    UnexpectedApproval,
    /// The declared effect or retry class cannot be automatically retried.
    AutomaticRetryForbidden,
    /// A successor could duplicate an unreconciled or uncertain prior effect.
    EffectReplayRisk,
    /// The attempt has already received a terminal receipt.
    ReceiptAlreadyIssued,
    /// The receipt does not belong to this issuer or a current issued attempt.
    ReceiptBindingMismatch,
    /// This verifier policy already received an identity for the attempt.
    VerificationAlreadyIssued,
    /// The synchronized ledger is unavailable and fails closed.
    LedgerUnavailable,
}

impl std::fmt::Display for WorkflowIdentityError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}", self.code())
    }
}

impl std::error::Error for WorkflowIdentityError {}

impl WorkflowIdentityError {
    /// Returns a stable content-free denial code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidIdentity => "workflow.identity.invalid",
            Self::PolicyBindingMismatch => "workflow.identity.policy_binding",
            Self::AttemptChainMismatch => "workflow.identity.attempt_chain",
            Self::PriorAttemptNotTerminal => "workflow.identity.prior_not_terminal",
            Self::ReplayedIdentity => "workflow.identity.replayed",
            Self::FreshApprovalRequired => "workflow.identity.approval_required",
            Self::UnexpectedApproval => "workflow.identity.approval_unexpected",
            Self::AutomaticRetryForbidden => "workflow.identity.automatic_retry_forbidden",
            Self::EffectReplayRisk => "workflow.identity.effect_replay_risk",
            Self::ReceiptAlreadyIssued => "workflow.identity.receipt_already_issued",
            Self::ReceiptBindingMismatch => "workflow.identity.receipt_binding",
            Self::VerificationAlreadyIssued => "workflow.identity.verification_already_issued",
            Self::LedgerUnavailable => "workflow.identity.ledger_unavailable",
        }
    }
}

#[derive(Clone, Debug)]
struct AttemptState {
    ordinal: u32,
    attempt_id: String,
    terminal_effect: Option<AttemptTerminalEffect>,
    receipt_id: Option<String>,
    verifier_policy_ids: BTreeSet<String>,
}

#[derive(Debug, Default)]
struct LedgerState {
    all_identities: BTreeSet<String>,
    attempts_by_step: BTreeMap<String, AttemptState>,
}

/// Workflow-scoped synchronized issuer for all effect-bearing execution identities.
#[derive(Debug)]
pub struct WorkflowIdentityIssuer {
    execution_id: String,
    state: Mutex<LedgerState>,
}

impl WorkflowIdentityIssuer {
    /// Creates an empty issuer for one exact workflow execution.
    pub fn new(execution_id: String) -> Result<Self, WorkflowIdentityError> {
        if !valid_identifier(&execution_id) {
            return Err(WorkflowIdentityError::InvalidIdentity);
        }
        Ok(Self {
            execution_id,
            state: Mutex::new(LedgerState::default()),
        })
    }

    /// Atomically reserves every pre-effect identity for one exact attempt.
    ///
    /// The returned value does not contain grant authority and cannot execute an effect.
    pub fn issue_attempt(
        &self,
        policy: &CanonicalStepExecutionPolicy,
        candidate: AttemptIdentityCandidate,
    ) -> Result<IssuedAttemptIdentities, WorkflowIdentityError> {
        validate_attempt_candidate(policy, &candidate)?;
        let mut state = self
            .state
            .lock()
            .map_err(|_| WorkflowIdentityError::LedgerUnavailable)?;
        if state.all_identities.len() >= MAX_IDENTITIES {
            return Err(WorkflowIdentityError::InvalidIdentity);
        }

        let previous = state.attempts_by_step.get(&candidate.step_execution_id);
        match (candidate.opening, previous) {
            (AttemptOpeningKind::Initial, None) => {
                if candidate.attempt_ordinal != 1 || candidate.predecessor_attempt_id.is_some() {
                    return Err(WorkflowIdentityError::AttemptChainMismatch);
                }
            }
            (AttemptOpeningKind::Initial, Some(_)) => {
                return Err(WorkflowIdentityError::AttemptChainMismatch);
            }
            (AttemptOpeningKind::AutomaticRetry | AttemptOpeningKind::UserApprovedRetry, None) => {
                return Err(WorkflowIdentityError::AttemptChainMismatch);
            }
            (
                AttemptOpeningKind::AutomaticRetry | AttemptOpeningKind::UserApprovedRetry,
                Some(prior),
            ) => {
                if prior.terminal_effect.is_none() {
                    return Err(WorkflowIdentityError::PriorAttemptNotTerminal);
                }
                if prior.ordinal.checked_add(1) != Some(candidate.attempt_ordinal)
                    || candidate.predecessor_attempt_id.as_deref()
                        != Some(prior.attempt_id.as_str())
                {
                    return Err(WorkflowIdentityError::AttemptChainMismatch);
                }
                if prior.terminal_effect == Some(AttemptTerminalEffect::Uncertain) {
                    if candidate.opening == AttemptOpeningKind::AutomaticRetry {
                        return Err(WorkflowIdentityError::AutomaticRetryForbidden);
                    }
                    if candidate.reconciliation != AttemptReconciliationState::SafeForFreshAttempt {
                        return Err(WorkflowIdentityError::EffectReplayRisk);
                    }
                }
            }
        }

        if candidate.opening == AttemptOpeningKind::AutomaticRetry {
            if !policy.effect_class.permits_automatic_retry()
                || !policy.retry_class.is_automatic()
                || !policy
                    .effect_class
                    .permitted_retry_classes()
                    .contains(&policy.retry_class)
            {
                return Err(WorkflowIdentityError::AutomaticRetryForbidden);
            }
            if policy.retry_class == CanonicalRetryClass::ConditionalAfterReconciliation
                && candidate.reconciliation != AttemptReconciliationState::SafeForFreshAttempt
            {
                return Err(WorkflowIdentityError::EffectReplayRisk);
            }
        }
        if candidate.opening == AttemptOpeningKind::UserApprovedRetry
            && candidate.reconciliation != AttemptReconciliationState::SafeForFreshAttempt
        {
            return Err(WorkflowIdentityError::EffectReplayRisk);
        }

        let raw_identities = candidate_raw_identities(&candidate);
        if raw_identities.iter().collect::<BTreeSet<_>>().len() != raw_identities.len()
            || raw_identities
                .iter()
                .any(|identity| state.all_identities.contains(*identity))
        {
            return Err(WorkflowIdentityError::ReplayedIdentity);
        }
        for identity in &raw_identities {
            state.all_identities.insert((*identity).to_owned());
        }
        state.attempts_by_step.insert(
            candidate.step_execution_id.clone(),
            AttemptState {
                ordinal: candidate.attempt_ordinal,
                attempt_id: candidate.attempt_id.clone(),
                terminal_effect: None,
                receipt_id: None,
                verifier_policy_ids: BTreeSet::new(),
            },
        );
        Ok(IssuedAttemptIdentities {
            execution_id: self.execution_id.clone(),
            step_execution_id: candidate.step_execution_id,
            attempt_ordinal: candidate.attempt_ordinal,
            call_id: candidate.call_id,
            tool_call_id: candidate.tool_call_id,
            attempt_id: candidate.attempt_id,
            grant_id: candidate.grant_id,
            approval_id: candidate.approval_id,
        })
    }

    /// Issues exactly one fresh receipt identity after an issued attempt reaches a terminal state.
    pub fn issue_receipt(
        &self,
        issued: &IssuedAttemptIdentities,
        receipt_id: ReceiptId,
        effect: AttemptTerminalEffect,
    ) -> Result<IssuedReceiptIdentity, WorkflowIdentityError> {
        if issued.execution_id != self.execution_id || !valid_identifier(receipt_id.as_str()) {
            return Err(WorkflowIdentityError::ReceiptBindingMismatch);
        }
        let mut state = self
            .state
            .lock()
            .map_err(|_| WorkflowIdentityError::LedgerUnavailable)?;
        if state.all_identities.len() >= MAX_IDENTITIES {
            return Err(WorkflowIdentityError::InvalidIdentity);
        }
        if state.all_identities.contains(receipt_id.as_str()) {
            return Err(WorkflowIdentityError::ReplayedIdentity);
        }
        let attempt = state
            .attempts_by_step
            .get_mut(&issued.step_execution_id)
            .filter(|attempt| {
                attempt.attempt_id == issued.attempt_id && attempt.ordinal == issued.attempt_ordinal
            })
            .ok_or(WorkflowIdentityError::ReceiptBindingMismatch)?;
        if attempt.receipt_id.is_some() {
            return Err(WorkflowIdentityError::ReceiptAlreadyIssued);
        }
        attempt.terminal_effect = Some(effect);
        attempt.receipt_id = Some(receipt_id.as_str().to_owned());
        state.all_identities.insert(receipt_id.as_str().to_owned());
        Ok(IssuedReceiptIdentity {
            execution_id: self.execution_id.clone(),
            step_execution_id: issued.step_execution_id.clone(),
            attempt_id: issued.attempt_id.clone(),
            receipt_id,
            effect,
        })
    }

    /// Issues one fresh verification identity for one exact terminal receipt and verifier policy.
    pub fn issue_verification(
        &self,
        receipt: &IssuedReceiptIdentity,
        verification_id: String,
        verifier_policy_id: String,
    ) -> Result<IssuedVerificationIdentity, WorkflowIdentityError> {
        if receipt.execution_id != self.execution_id
            || !valid_identifier(&verification_id)
            || !valid_identifier(&verifier_policy_id)
        {
            return Err(WorkflowIdentityError::ReceiptBindingMismatch);
        }
        let mut state = self
            .state
            .lock()
            .map_err(|_| WorkflowIdentityError::LedgerUnavailable)?;
        if state.all_identities.len() >= MAX_IDENTITIES {
            return Err(WorkflowIdentityError::InvalidIdentity);
        }
        if state.all_identities.contains(&verification_id) {
            return Err(WorkflowIdentityError::ReplayedIdentity);
        }
        let attempt = state
            .attempts_by_step
            .get_mut(&receipt.step_execution_id)
            .filter(|attempt| {
                attempt.attempt_id == receipt.attempt_id
                    && attempt.receipt_id.as_deref() == Some(receipt.receipt_id.as_str())
                    && attempt.terminal_effect == Some(receipt.effect)
            })
            .ok_or(WorkflowIdentityError::ReceiptBindingMismatch)?;
        if !attempt
            .verifier_policy_ids
            .insert(verifier_policy_id.clone())
        {
            return Err(WorkflowIdentityError::VerificationAlreadyIssued);
        }
        state.all_identities.insert(verification_id.clone());
        Ok(IssuedVerificationIdentity {
            execution_id: self.execution_id.clone(),
            step_execution_id: receipt.step_execution_id.clone(),
            attempt_id: receipt.attempt_id.clone(),
            receipt_id: receipt.receipt_id.clone(),
            verification_id,
            verifier_policy_id,
        })
    }
}

fn validate_attempt_candidate(
    policy: &CanonicalStepExecutionPolicy,
    candidate: &AttemptIdentityCandidate,
) -> Result<(), WorkflowIdentityError> {
    if policy.schema_version != CONTRACT_SCHEMA_VERSION
        || !valid_identifier(&policy.policy_id)
        || policy.plan_step_id.as_str().is_empty()
        || !policy_integrity(policy)
        || !valid_identifier(&candidate.step_execution_id)
        || candidate.attempt_ordinal == 0
        || candidate
            .predecessor_attempt_id
            .as_deref()
            .is_some_and(|identity| !valid_identifier(identity))
        || candidate_raw_identities(candidate)
            .iter()
            .any(|identity| !valid_identifier(identity))
    {
        return Err(WorkflowIdentityError::PolicyBindingMismatch);
    }
    if !policy
        .effect_class
        .permitted_retry_classes()
        .contains(&policy.retry_class)
    {
        return Err(WorkflowIdentityError::PolicyBindingMismatch);
    }
    let approval_required = policy.effect_class.requires_approval()
        || policy.approval_requirement == CanonicalApprovalRequirement::RequiredPerAttempt
        || (candidate.opening == AttemptOpeningKind::Initial
            && policy.approval_requirement == CanonicalApprovalRequirement::RequiredOnce)
        || candidate.opening == AttemptOpeningKind::UserApprovedRetry;
    match (approval_required, candidate.approval_id.is_some()) {
        (true, false) => return Err(WorkflowIdentityError::FreshApprovalRequired),
        (false, true) => return Err(WorkflowIdentityError::UnexpectedApproval),
        _ => {}
    }
    if policy.effect_class.requires_approval()
        && policy.approval_requirement != CanonicalApprovalRequirement::RequiredPerAttempt
    {
        return Err(WorkflowIdentityError::PolicyBindingMismatch);
    }
    Ok(())
}

fn candidate_raw_identities(candidate: &AttemptIdentityCandidate) -> Vec<&str> {
    let mut values = vec![
        candidate.call_id.as_str(),
        candidate.tool_call_id.as_str(),
        candidate.attempt_id.as_str(),
        candidate.grant_id.as_str(),
    ];
    if let Some(approval_id) = &candidate.approval_id {
        values.push(approval_id.as_str());
    }
    values
}

fn valid_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value.is_ascii()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b':' | b'-'))
}

fn policy_integrity(policy: &CanonicalStepExecutionPolicy) -> bool {
    if policy.policy_sha256.len() != 64
        || !policy
            .policy_sha256
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return false;
    }
    let mut candidate = policy.clone();
    candidate.policy_sha256 = ZERO_SHA256.to_owned();
    let Ok(bytes) = to_canonical_json(&candidate) else {
        return false;
    };
    let mut actual = String::with_capacity(64);
    for byte in Sha256::digest(bytes) {
        let _ = write!(actual, "{byte:02x}");
    }
    actual == policy.policy_sha256
}
