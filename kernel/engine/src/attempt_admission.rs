//! Deterministic, non-authoritative admission of one new operation attempt.
//!
//! A new attempt is admitted only after all six declared preconditions hold over
//! current facts: deterministic preflight, prior-effect reconciliation, remaining
//! declared budget, fresh call identity, a fresh single-use grant, and a fresh user
//! decision when the operation requires one. The preconditions are evaluated in that
//! exact order and the first failure is returned, so one denied attempt reports one
//! stable reason.
//!
//! Nothing here issues, widens, or revives authority. Admission states only that a new
//! attempt may be prepared from objects that were already verified fresh; it never
//! permits reusing a prior call, grant, approval, or effect, and an unestablished fact
//! is always a denial rather than an admission.

use agentmage_kernel_contracts::{
    AttemptApprovalState, AttemptBudgetState, AttemptCallIdentityState, AttemptDenial,
    AttemptGrantState, AttemptIdempotencyIdentity, AttemptPreflightState,
    AttemptReconciliationState, CapabilityGrant, EffectRepetition, GrantClass, GrantStatus,
    NewAttemptDecision, NewAttemptFacts, OperationBinding,
};

use crate::runtime_recovery::RuntimeRecoveryEffectState;

/// Decides whether exactly one new attempt may be prepared from current facts.
#[must_use]
pub const fn admit_new_attempt(facts: &NewAttemptFacts) -> NewAttemptDecision {
    let approval_required = facts.requires_fresh_approval();

    if let Some(denial) = malformed_request(facts) {
        return NewAttemptDecision::denied(denial, approval_required);
    }
    if let Some(denial) = preflight_denial(facts.preflight) {
        return NewAttemptDecision::denied(denial, approval_required);
    }
    if let Some(denial) = reconciliation_denial(facts) {
        return NewAttemptDecision::denied(denial, approval_required);
    }
    if let Some(denial) = budget_denial(facts.budget) {
        return NewAttemptDecision::denied(denial, approval_required);
    }
    if let Some(denial) = call_identity_denial(facts.call_identity) {
        return NewAttemptDecision::denied(denial, approval_required);
    }
    if let Some(denial) = grant_denial(facts.grant) {
        return NewAttemptDecision::denied(denial, approval_required);
    }
    if let Some(denial) = approval_denial(facts.approval, approval_required) {
        return NewAttemptDecision::denied(denial, approval_required);
    }
    NewAttemptDecision::admitted(approval_required)
}

/// Reduces one current grant record to its closed attempt-admission state.
///
/// Freshness is never inferred from the record alone. `bound_to_earlier_attempt` comes
/// from the caller's attempt ledger, and a grant that already served an earlier attempt
/// is reported as reused even when its own counters still look current.
#[must_use]
pub fn observe_attempt_grant(
    grant: &CapabilityGrant,
    operation: OperationBinding,
    bound_to_earlier_attempt: bool,
) -> AttemptGrantState {
    if grant.grant_class != GrantClass::Operation || grant.operation != operation {
        return AttemptGrantState::Absent;
    }
    if bound_to_earlier_attempt {
        return AttemptGrantState::Reused;
    }
    if grant.status != GrantStatus::Issued {
        return AttemptGrantState::NotCurrent;
    }
    if grant.use_limit != 1 {
        return AttemptGrantState::MultiUse;
    }
    if grant.use_count != 0 {
        return AttemptGrantState::Reused;
    }
    AttemptGrantState::FreshSingleUse
}

/// Reduces one freshly observed runtime effect state to its closed reconciliation state.
///
/// An effect whose completion cannot be established is never treated as a failure that
/// may be attempted again; it stays unreconciled.
#[must_use]
pub const fn observe_attempt_reconciliation(
    effect: RuntimeRecoveryEffectState,
) -> AttemptReconciliationState {
    match effect {
        RuntimeRecoveryEffectState::NotStarted => AttemptReconciliationState::NoPriorEffect,
        RuntimeRecoveryEffectState::FailedNoChangeVerified => {
            AttemptReconciliationState::FailedNoChangeVerified
        }
        RuntimeRecoveryEffectState::CompletedVerified => {
            AttemptReconciliationState::CompletedVerified
        }
        RuntimeRecoveryEffectState::Uncertain => AttemptReconciliationState::Unreconciled,
    }
}

const fn malformed_request(facts: &NewAttemptFacts) -> Option<AttemptDenial> {
    if facts.attempt_ordinal == 0 {
        return Some(AttemptDenial::OrdinalInvalid);
    }
    if facts.attempt_ordinal == 1
        && !matches!(facts.reconciliation, AttemptReconciliationState::NoPriorEffect)
    {
        return Some(AttemptDenial::FirstAttemptWithPriorEffect);
    }
    if !matches!(facts.effect.repetition(), EffectRepetition::IdentityBound)
        && !matches!(facts.idempotency_identity, AttemptIdempotencyIdentity::Absent)
    {
        return Some(AttemptDenial::IdempotencyIdentityNotApplicable);
    }
    None
}

const fn preflight_denial(preflight: AttemptPreflightState) -> Option<AttemptDenial> {
    match preflight {
        AttemptPreflightState::CurrentAdmitted => None,
        AttemptPreflightState::CurrentDenied => Some(AttemptDenial::PreflightDenied),
        AttemptPreflightState::Stale => Some(AttemptDenial::PreflightStale),
        AttemptPreflightState::Absent => Some(AttemptDenial::PreflightAbsent),
    }
}

const fn reconciliation_denial(facts: &NewAttemptFacts) -> Option<AttemptDenial> {
    if facts.effect.changes_state()
        && matches!(facts.reconciliation, AttemptReconciliationState::CompletedVerified)
    {
        return Some(AttemptDenial::EffectAlreadyCompleted);
    }
    if facts.effect.requires_reconciliation()
        && matches!(facts.reconciliation, AttemptReconciliationState::Unreconciled)
    {
        return Some(AttemptDenial::EffectUnreconciled);
    }
    match facts.effect.repetition() {
        EffectRepetition::IdentityBound => match facts.idempotency_identity {
            AttemptIdempotencyIdentity::VerifiedStable => None,
            AttemptIdempotencyIdentity::Changed => Some(AttemptDenial::IdempotencyIdentityChanged),
            AttemptIdempotencyIdentity::Absent => Some(AttemptDenial::IdempotencyIdentityAbsent),
        },
        EffectRepetition::SafeToRepeat
        | EffectRepetition::ReconciliationRequired
        | EffectRepetition::NeverAutomatic => None,
    }
}

const fn budget_denial(budget: AttemptBudgetState) -> Option<AttemptDenial> {
    match budget {
        AttemptBudgetState::Remaining => None,
        AttemptBudgetState::Exhausted => Some(AttemptDenial::BudgetExhausted),
        AttemptBudgetState::Undeclared => Some(AttemptDenial::BudgetUndeclared),
    }
}

const fn call_identity_denial(identity: AttemptCallIdentityState) -> Option<AttemptDenial> {
    match identity {
        AttemptCallIdentityState::Fresh => None,
        AttemptCallIdentityState::Reused => Some(AttemptDenial::CallIdentityReused),
        AttemptCallIdentityState::Absent => Some(AttemptDenial::CallIdentityAbsent),
        AttemptCallIdentityState::Unverified => Some(AttemptDenial::CallIdentityUnverified),
    }
}

const fn grant_denial(grant: AttemptGrantState) -> Option<AttemptDenial> {
    match grant {
        AttemptGrantState::FreshSingleUse => None,
        AttemptGrantState::MultiUse => Some(AttemptDenial::GrantMultiUse),
        AttemptGrantState::Reused => Some(AttemptDenial::GrantReused),
        AttemptGrantState::NotCurrent => Some(AttemptDenial::GrantNotCurrent),
        AttemptGrantState::Absent => Some(AttemptDenial::GrantAbsent),
    }
}

const fn approval_denial(
    approval: AttemptApprovalState,
    approval_required: bool,
) -> Option<AttemptDenial> {
    match approval {
        AttemptApprovalState::Fresh => None,
        AttemptApprovalState::Reused => Some(AttemptDenial::ApprovalReused),
        AttemptApprovalState::Absent => {
            if approval_required {
                Some(AttemptDenial::ApprovalAbsent)
            } else {
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{admit_new_attempt, observe_attempt_grant, observe_attempt_reconciliation};
    use crate::runtime_recovery::RuntimeRecoveryEffectState;
    use agentmage_kernel_contracts::{
        ActionId, ActionKind, ActorId, ApprovalId, AttemptApprovalState, AttemptBudgetState,
        AttemptCallIdentityState, AttemptDenial, AttemptGrantState, AttemptIdempotencyIdentity,
        AttemptPrecondition, AttemptPreflightState, AttemptReconciliationState, CapabilityGrant,
        DataSensitivity, EffectClass, EffectDeclaration, EffectRepetition, GrantClass, GrantId,
        GrantNonce, GrantOperation, GrantStatus, NewAttemptFacts, OperationBinding, SessionId,
        TaskId, ToolId,
    };

    const DIGEST: &str = "1111111111111111111111111111111111111111111111111111111111111111";

    fn eligible(class: EffectClass) -> NewAttemptFacts {
        NewAttemptFacts {
            operation: OperationBinding::new(GrantOperation::WorkspaceWrite),
            effect: EffectDeclaration::new(class),
            attempt_ordinal: 2,
            registration_requires_approval: false,
            preflight: AttemptPreflightState::CurrentAdmitted,
            reconciliation: AttemptReconciliationState::FailedNoChangeVerified,
            budget: AttemptBudgetState::Remaining,
            call_identity: AttemptCallIdentityState::Fresh,
            grant: AttemptGrantState::FreshSingleUse,
            approval: AttemptApprovalState::Fresh,
            idempotency_identity: match class.repetition() {
                EffectRepetition::IdentityBound => AttemptIdempotencyIdentity::VerifiedStable,
                _ => AttemptIdempotencyIdentity::Absent,
            },
        }
    }

    fn grant(status: GrantStatus, use_limit: u32, use_count: u32) -> CapabilityGrant {
        CapabilityGrant {
            schema_version: agentmage_kernel_contracts::CONTRACT_SCHEMA_VERSION,
            grant_id: GrantId::from_raw("grant-0001"),
            revision: 1,
            grant_class: GrantClass::Operation,
            actor_id: ActorId::from_raw("actor-0001"),
            approval_id: Some(ApprovalId::from_raw("approval-0001")),
            session_id: SessionId::from_raw("session-0001"),
            task_id: TaskId::from_raw("task-0001"),
            action_id: Some(ActionId::from_raw("action-0001")),
            action_kind: Some(ActionKind::DeterministicTool),
            operation: OperationBinding::new(GrantOperation::WorkspaceWrite),
            tool_id: Some(ToolId::from_raw("tool-0001")),
            tool_version: Some("1.0.0".to_owned()),
            targets: Vec::new(),
            excluded_targets: Vec::new(),
            sensitivity: DataSensitivity::Operational,
            argument_sha256: DIGEST.to_owned(),
            preimages: Vec::new(),
            expected_side_effects: Vec::new(),
            rollback_description: "Restore the exact preimage".to_owned(),
            issued_at_epoch_ms: 1,
            expires_at_epoch_ms: 10,
            nonce: GrantNonce::from_raw("nonce-0001"),
            use_limit,
            use_count,
            parent_grant_id: None,
            parent_grant_sha256: None,
            preview_sha256: DIGEST.to_owned(),
            policy_sha256: DIGEST.to_owned(),
            status,
        }
    }

    #[test]
    fn story_5_2_preconditions_are_evaluated_in_one_exact_declared_order() {
        let mut facts = eligible(EffectClass::Conditional);
        facts.preflight = AttemptPreflightState::Absent;
        facts.reconciliation = AttemptReconciliationState::Unreconciled;
        facts.budget = AttemptBudgetState::Exhausted;
        facts.call_identity = AttemptCallIdentityState::Reused;
        facts.grant = AttemptGrantState::Reused;
        facts.approval = AttemptApprovalState::Reused;

        let expected = [
            AttemptDenial::PreflightAbsent,
            AttemptDenial::EffectUnreconciled,
            AttemptDenial::BudgetExhausted,
            AttemptDenial::CallIdentityReused,
            AttemptDenial::GrantReused,
            AttemptDenial::ApprovalReused,
        ];
        for (precondition, denial) in AttemptPrecondition::ALL.into_iter().zip(expected) {
            let decision = admit_new_attempt(&facts);
            assert!(!decision.is_admitted(), "precondition {precondition:?}");
            assert_eq!(decision.denial(), Some(denial));
            assert_eq!(decision.failed_precondition(), Some(precondition));
            assert_eq!(decision.code(), denial.code());
            assert!(!decision.replay_permitted());

            match precondition {
                AttemptPrecondition::Preflight => {
                    facts.preflight = AttemptPreflightState::CurrentAdmitted;
                }
                AttemptPrecondition::EffectReconciliation => {
                    facts.reconciliation = AttemptReconciliationState::FailedNoChangeVerified;
                }
                AttemptPrecondition::Budget => facts.budget = AttemptBudgetState::Remaining,
                AttemptPrecondition::CallIdentity => {
                    facts.call_identity = AttemptCallIdentityState::Fresh;
                }
                AttemptPrecondition::Grant => facts.grant = AttemptGrantState::FreshSingleUse,
                AttemptPrecondition::Approval => facts.approval = AttemptApprovalState::Absent,
            }
        }

        let decision = admit_new_attempt(&facts);
        assert!(decision.is_admitted());
        assert_eq!(decision.code(), "attempt.admit.eligible");
        assert!(!decision.replay_permitted());
    }

    #[test]
    fn story_5_2_every_unestablished_or_stale_precondition_fails_closed() {
        let preflight = [
            (AttemptPreflightState::CurrentDenied, AttemptDenial::PreflightDenied),
            (AttemptPreflightState::Stale, AttemptDenial::PreflightStale),
            (AttemptPreflightState::Absent, AttemptDenial::PreflightAbsent),
        ];
        for (state, denial) in preflight {
            let mut facts = eligible(EffectClass::Conditional);
            facts.preflight = state;
            assert_eq!(admit_new_attempt(&facts).denial(), Some(denial));
        }

        let budgets = [
            (AttemptBudgetState::Exhausted, AttemptDenial::BudgetExhausted),
            (AttemptBudgetState::Undeclared, AttemptDenial::BudgetUndeclared),
        ];
        for (state, denial) in budgets {
            let mut facts = eligible(EffectClass::Conditional);
            facts.budget = state;
            assert_eq!(admit_new_attempt(&facts).denial(), Some(denial));
        }

        let identities = [
            (AttemptCallIdentityState::Reused, AttemptDenial::CallIdentityReused),
            (AttemptCallIdentityState::Absent, AttemptDenial::CallIdentityAbsent),
            (AttemptCallIdentityState::Unverified, AttemptDenial::CallIdentityUnverified),
        ];
        for (state, denial) in identities {
            let mut facts = eligible(EffectClass::Conditional);
            facts.call_identity = state;
            assert_eq!(admit_new_attempt(&facts).denial(), Some(denial));
        }

        let grants = [
            (AttemptGrantState::MultiUse, AttemptDenial::GrantMultiUse),
            (AttemptGrantState::Reused, AttemptDenial::GrantReused),
            (AttemptGrantState::NotCurrent, AttemptDenial::GrantNotCurrent),
            (AttemptGrantState::Absent, AttemptDenial::GrantAbsent),
        ];
        for (state, denial) in grants {
            let mut facts = eligible(EffectClass::Conditional);
            facts.grant = state;
            assert_eq!(admit_new_attempt(&facts).denial(), Some(denial));
        }
    }

    #[test]
    fn story_5_2_a_completed_effect_is_never_attempted_again() {
        for class in EffectClass::ALL {
            let mut facts = eligible(class);
            facts.reconciliation = AttemptReconciliationState::CompletedVerified;
            facts.approval = AttemptApprovalState::Fresh;
            let decision = admit_new_attempt(&facts);
            assert!(!decision.replay_permitted());

            if class.changes_state() {
                assert_eq!(
                    decision.denial(),
                    Some(AttemptDenial::EffectAlreadyCompleted),
                    "class {class:?}"
                );
            } else {
                assert!(decision.is_admitted(), "class {class:?}");
            }
        }
    }

    #[test]
    fn story_5_2_unreconciled_effects_admit_only_verified_identity_bound_convergence() {
        for class in EffectClass::ALL {
            let mut facts = eligible(class);
            facts.reconciliation = AttemptReconciliationState::Unreconciled;
            let decision = admit_new_attempt(&facts);
            assert!(!decision.replay_permitted());

            match class.repetition() {
                EffectRepetition::SafeToRepeat | EffectRepetition::IdentityBound => {
                    assert!(decision.is_admitted(), "class {class:?}");
                }
                EffectRepetition::ReconciliationRequired | EffectRepetition::NeverAutomatic => {
                    assert_eq!(
                        decision.denial(),
                        Some(AttemptDenial::EffectUnreconciled),
                        "class {class:?}"
                    );
                }
            }
        }

        let mut absent = eligible(EffectClass::IdempotentWrite);
        absent.reconciliation = AttemptReconciliationState::Unreconciled;
        absent.idempotency_identity = AttemptIdempotencyIdentity::Absent;
        assert_eq!(
            admit_new_attempt(&absent).denial(),
            Some(AttemptDenial::IdempotencyIdentityAbsent)
        );

        let mut changed = eligible(EffectClass::IdempotentWrite);
        changed.reconciliation = AttemptReconciliationState::Unreconciled;
        changed.idempotency_identity = AttemptIdempotencyIdentity::Changed;
        assert_eq!(
            admit_new_attempt(&changed).denial(),
            Some(AttemptDenial::IdempotencyIdentityChanged)
        );
    }

    #[test]
    fn story_5_2_a_replayed_decision_is_denied_and_a_required_one_is_never_waived() {
        for class in EffectClass::ALL {
            let required = class.requires_fresh_approval();

            let mut reused = eligible(class);
            reused.approval = AttemptApprovalState::Reused;
            let decision = admit_new_attempt(&reused);
            assert_eq!(decision.denial(), Some(AttemptDenial::ApprovalReused));
            assert_eq!(decision.approval_required(), required);

            let mut absent = eligible(class);
            absent.approval = AttemptApprovalState::Absent;
            let decision = admit_new_attempt(&absent);
            assert_eq!(decision.approval_required(), required);
            if required {
                assert_eq!(decision.denial(), Some(AttemptDenial::ApprovalAbsent));
            } else {
                assert!(decision.is_admitted(), "class {class:?}");
            }

            let mut registered = eligible(class);
            registered.registration_requires_approval = true;
            registered.approval = AttemptApprovalState::Absent;
            let decision = admit_new_attempt(&registered);
            assert!(decision.approval_required());
            assert_eq!(decision.denial(), Some(AttemptDenial::ApprovalAbsent));
        }
    }

    #[test]
    fn story_5_2_incoherent_requests_fail_closed_before_any_precondition() {
        let mut zero = eligible(EffectClass::Conditional);
        zero.attempt_ordinal = 0;
        zero.preflight = AttemptPreflightState::Absent;
        let decision = admit_new_attempt(&zero);
        assert_eq!(decision.denial(), Some(AttemptDenial::OrdinalInvalid));
        assert_eq!(decision.failed_precondition(), None);

        for reconciliation in [
            AttemptReconciliationState::FailedNoChangeVerified,
            AttemptReconciliationState::CompletedVerified,
            AttemptReconciliationState::Unreconciled,
        ] {
            let mut first = eligible(EffectClass::Conditional);
            first.attempt_ordinal = 1;
            first.reconciliation = reconciliation;
            assert_eq!(
                admit_new_attempt(&first).denial(),
                Some(AttemptDenial::FirstAttemptWithPriorEffect)
            );
        }

        let mut first = eligible(EffectClass::Conditional);
        first.attempt_ordinal = 1;
        first.reconciliation = AttemptReconciliationState::NoPriorEffect;
        assert!(admit_new_attempt(&first).is_admitted());

        for class in EffectClass::ALL {
            if class.repetition() == EffectRepetition::IdentityBound {
                continue;
            }
            for identity in [
                AttemptIdempotencyIdentity::VerifiedStable,
                AttemptIdempotencyIdentity::Changed,
            ] {
                let mut facts = eligible(class);
                facts.idempotency_identity = identity;
                assert_eq!(
                    admit_new_attempt(&facts).denial(),
                    Some(AttemptDenial::IdempotencyIdentityNotApplicable),
                    "class {class:?}"
                );
            }
        }
    }

    #[test]
    fn story_5_2_only_a_current_unused_single_use_grant_is_fresh() {
        let operation = OperationBinding::new(GrantOperation::WorkspaceWrite);

        assert_eq!(
            observe_attempt_grant(&grant(GrantStatus::Issued, 1, 0), operation, false),
            AttemptGrantState::FreshSingleUse
        );
        assert_eq!(
            observe_attempt_grant(&grant(GrantStatus::Issued, 1, 0), operation, true),
            AttemptGrantState::Reused
        );
        assert_eq!(
            observe_attempt_grant(&grant(GrantStatus::Issued, 2, 0), operation, false),
            AttemptGrantState::MultiUse
        );
        assert_eq!(
            observe_attempt_grant(&grant(GrantStatus::Issued, 1, 1), operation, false),
            AttemptGrantState::Reused
        );

        for status in [
            GrantStatus::Consumed,
            GrantStatus::Revoked,
            GrantStatus::Expired,
            GrantStatus::Invalidated,
            GrantStatus::Uncertain,
        ] {
            assert_eq!(
                observe_attempt_grant(&grant(status, 1, 0), operation, false),
                AttemptGrantState::NotCurrent,
                "status {status:?}"
            );
        }

        let other = OperationBinding::new(GrantOperation::WorkspaceDelete);
        assert_eq!(
            observe_attempt_grant(&grant(GrantStatus::Issued, 1, 0), other, false),
            AttemptGrantState::Absent
        );

        let mut parent = grant(GrantStatus::Issued, 1, 0);
        parent.grant_class = GrantClass::SessionRead;
        assert_eq!(observe_attempt_grant(&parent, operation, false), AttemptGrantState::Absent);
    }

    #[test]
    fn story_5_2_uncertain_runtime_effects_never_reduce_to_a_retryable_failure() {
        assert_eq!(
            observe_attempt_reconciliation(RuntimeRecoveryEffectState::NotStarted),
            AttemptReconciliationState::NoPriorEffect
        );
        assert_eq!(
            observe_attempt_reconciliation(RuntimeRecoveryEffectState::FailedNoChangeVerified),
            AttemptReconciliationState::FailedNoChangeVerified
        );
        assert_eq!(
            observe_attempt_reconciliation(RuntimeRecoveryEffectState::CompletedVerified),
            AttemptReconciliationState::CompletedVerified
        );
        assert_eq!(
            observe_attempt_reconciliation(RuntimeRecoveryEffectState::Uncertain),
            AttemptReconciliationState::Unreconciled
        );

        let uncertain = observe_attempt_reconciliation(RuntimeRecoveryEffectState::Uncertain);
        let mut facts = eligible(EffectClass::NonIdempotent);
        facts.reconciliation = uncertain;
        let decision = admit_new_attempt(&facts);
        assert_eq!(decision.denial(), Some(AttemptDenial::EffectUnreconciled));
        assert!(!decision.replay_permitted());
    }
}
