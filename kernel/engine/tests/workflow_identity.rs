use std::fmt::Write as _;
use std::sync::{Arc, Barrier};

use agentmage_kernel_contracts::{
    ApprovalId, CONTRACT_SCHEMA_VERSION, CanonicalApprovalRequirement,
    CanonicalDiagnosticDisclosure, CanonicalEffectClass, CanonicalExecutionBudgets,
    CanonicalIdempotencyRequirement, CanonicalRetryClass, CanonicalStepExecutionPolicy,
    CanonicalVerificationRequirement, GrantId, PlanId, PlanStepId, ReceiptId, ToolCallId,
    to_canonical_json,
};
use agentmage_kernel_engine::workflow_identity::{
    AttemptIdentityCandidate, AttemptOpeningKind, AttemptReconciliationState,
    AttemptTerminalEffect, WorkflowIdentityError, WorkflowIdentityIssuer,
};
use sha2::{Digest, Sha256};

const ZERO: &str = "0000000000000000000000000000000000000000000000000000000000000000";

fn sha256(bytes: &[u8]) -> String {
    let mut value = String::with_capacity(64);
    for byte in Sha256::digest(bytes) {
        let _ = write!(value, "{byte:02x}");
    }
    value
}

fn policy(
    effect_class: CanonicalEffectClass,
    retry_class: CanonicalRetryClass,
) -> CanonicalStepExecutionPolicy {
    let (approval_requirement, idempotency_key_requirement) = match effect_class {
        CanonicalEffectClass::ReadOnly => (
            CanonicalApprovalRequirement::NotRequired,
            CanonicalIdempotencyRequirement::NotApplicable,
        ),
        CanonicalEffectClass::IdempotentWrite | CanonicalEffectClass::Conditional => (
            CanonicalApprovalRequirement::NotRequired,
            CanonicalIdempotencyRequirement::Required,
        ),
        CanonicalEffectClass::NonIdempotent
        | CanonicalEffectClass::Destructive
        | CanonicalEffectClass::External
        | CanonicalEffectClass::Unknown => (
            CanonicalApprovalRequirement::RequiredPerAttempt,
            CanonicalIdempotencyRequirement::VerifiedDesiredState,
        ),
    };
    let mut policy = CanonicalStepExecutionPolicy {
        schema_version: CONTRACT_SCHEMA_VERSION,
        policy_id: "policy:identity".to_owned(),
        plan_id: PlanId::from_raw("plan:identity"),
        plan_step_id: PlanStepId::from_raw("plan-step:identity"),
        plan_revision: 1,
        preflight_policy_id: "preflight-policy:identity".to_owned(),
        required_preflight_ids: vec!["preflight:identity".to_owned()],
        side_effect_policy_id: "effect-policy:identity".to_owned(),
        effect_class,
        approval_policy_id: "approval-policy:identity".to_owned(),
        approval_requirement,
        idempotency_policy_id: "idempotency-policy:identity".to_owned(),
        idempotency_key_requirement,
        verifier_policy_id: "verifier-policy:identity".to_owned(),
        verification_requirement: CanonicalVerificationRequirement::VerifierEvidenceRequired,
        required_verifier_ids: vec!["verifier:identity".to_owned()],
        deferral_reason_code: None,
        retry_policy_id: "retry-policy:identity".to_owned(),
        retry_class,
        budget_policy_id: "budget-policy:identity".to_owned(),
        budgets: CanonicalExecutionBudgets {
            turns: 1,
            tokens: 1_024,
            duration_ms: 10_000,
            tool_calls: 1,
            attempts: 3,
            no_progress_events: 1,
            output_bytes: 4_096,
            memory_bytes: 65_536,
            cost_minor_units: 0,
        },
        diagnostic_policy_id: "diagnostic-policy:identity".to_owned(),
        diagnostic_disclosure: CanonicalDiagnosticDisclosure::ContentFreeCodes,
        recorded_at: "2026-08-30T12:00:00Z".to_owned(),
        policy_sha256: ZERO.to_owned(),
    };
    policy.policy_sha256 = sha256(&to_canonical_json(&policy).unwrap());
    policy
}

fn candidate(
    suffix: &str,
    ordinal: u32,
    predecessor: Option<&str>,
    opening: AttemptOpeningKind,
    reconciliation: AttemptReconciliationState,
    approval: bool,
) -> AttemptIdentityCandidate {
    AttemptIdentityCandidate {
        step_execution_id: "step-execution:1".to_owned(),
        attempt_ordinal: ordinal,
        predecessor_attempt_id: predecessor.map(str::to_owned),
        call_id: format!("call:{suffix}"),
        tool_call_id: ToolCallId::from_raw(format!("tool-call:{suffix}")),
        attempt_id: format!("attempt:{suffix}"),
        grant_id: GrantId::from_raw(format!("grant:{suffix}")),
        approval_id: approval.then(|| ApprovalId::from_raw(format!("approval:{suffix}"))),
        opening,
        reconciliation,
    }
}

#[test]
fn issues_the_complete_identity_chain_without_execution_authority() {
    let issuer = WorkflowIdentityIssuer::new("execution:1".to_owned()).unwrap();
    let attempt = issuer
        .issue_attempt(
            &policy(
                CanonicalEffectClass::ReadOnly,
                CanonicalRetryClass::RecoverableRead,
            ),
            candidate(
                "1",
                1,
                None,
                AttemptOpeningKind::Initial,
                AttemptReconciliationState::NotRequired,
                false,
            ),
        )
        .unwrap();
    assert_eq!(attempt.execution_id(), "execution:1");
    assert_eq!(attempt.step_execution_id(), "step-execution:1");
    assert_eq!(attempt.attempt_ordinal(), 1);
    assert_eq!(attempt.call_id(), "call:1");
    assert_eq!(attempt.tool_call_id().as_str(), "tool-call:1");
    assert_eq!(attempt.attempt_id(), "attempt:1");
    assert_eq!(attempt.grant_id().as_str(), "grant:1");
    assert_eq!(attempt.approval_id(), None);

    let receipt = issuer
        .issue_receipt(
            &attempt,
            ReceiptId::from_raw("receipt:1"),
            AttemptTerminalEffect::Succeeded,
        )
        .unwrap();
    assert_eq!(receipt.attempt_id(), "attempt:1");
    assert_eq!(receipt.receipt_id().as_str(), "receipt:1");
    let verification = issuer
        .issue_verification(
            &receipt,
            "verification:1".to_owned(),
            "verifier:identity".to_owned(),
        )
        .unwrap();
    assert_eq!(verification.receipt_id.as_str(), "receipt:1");
    assert_eq!(verification.verification_id, "verification:1");
}

#[test]
fn every_pre_effect_identity_is_globally_fresh_and_failed_issue_is_atomic() {
    let issuer = WorkflowIdentityIssuer::new("execution:1".to_owned()).unwrap();
    let policy = policy(
        CanonicalEffectClass::ReadOnly,
        CanonicalRetryClass::RecoverableRead,
    );
    let first = issuer
        .issue_attempt(
            &policy,
            candidate(
                "1",
                1,
                None,
                AttemptOpeningKind::Initial,
                AttemptReconciliationState::NotRequired,
                false,
            ),
        )
        .unwrap();
    issuer
        .issue_receipt(
            &first,
            ReceiptId::from_raw("receipt:1"),
            AttemptTerminalEffect::Failed,
        )
        .unwrap();

    for mutation in 0..4 {
        let mut next = candidate(
            &format!("replay-{mutation}"),
            2,
            Some("attempt:1"),
            AttemptOpeningKind::AutomaticRetry,
            AttemptReconciliationState::NotRequired,
            false,
        );
        match mutation {
            0 => next.call_id = "call:1".to_owned(),
            1 => next.tool_call_id = ToolCallId::from_raw("tool-call:1"),
            2 => next.attempt_id = "attempt:1".to_owned(),
            3 => next.grant_id = GrantId::from_raw("grant:1"),
            _ => unreachable!(),
        }
        assert_eq!(
            issuer.issue_attempt(&policy, next),
            Err(WorkflowIdentityError::ReplayedIdentity),
            "identity mutation {mutation}"
        );
    }
    let fresh = issuer.issue_attempt(
        &policy,
        candidate(
            "2",
            2,
            Some("attempt:1"),
            AttemptOpeningKind::AutomaticRetry,
            AttemptReconciliationState::NotRequired,
            false,
        ),
    );
    assert!(
        fresh.is_ok(),
        "denied candidates must not partially mutate the ledger"
    );
}

#[test]
fn ordinals_predecessors_and_terminal_order_are_exact() {
    let issuer = WorkflowIdentityIssuer::new("execution:1".to_owned()).unwrap();
    let policy = policy(
        CanonicalEffectClass::ReadOnly,
        CanonicalRetryClass::RecoverableRead,
    );
    let first = issuer
        .issue_attempt(
            &policy,
            candidate(
                "1",
                1,
                None,
                AttemptOpeningKind::Initial,
                AttemptReconciliationState::NotRequired,
                false,
            ),
        )
        .unwrap();
    assert_eq!(
        issuer.issue_attempt(
            &policy,
            candidate(
                "2",
                2,
                Some("attempt:1"),
                AttemptOpeningKind::AutomaticRetry,
                AttemptReconciliationState::NotRequired,
                false,
            )
        ),
        Err(WorkflowIdentityError::PriorAttemptNotTerminal)
    );
    issuer
        .issue_receipt(
            &first,
            ReceiptId::from_raw("receipt:1"),
            AttemptTerminalEffect::Failed,
        )
        .unwrap();
    for (ordinal, predecessor) in [(1, Some("attempt:1")), (3, Some("attempt:1")), (2, None)] {
        assert_eq!(
            issuer.issue_attempt(
                &policy,
                candidate(
                    &format!("bad-{ordinal}-{}", predecessor.is_some()),
                    ordinal,
                    predecessor,
                    AttemptOpeningKind::AutomaticRetry,
                    AttemptReconciliationState::NotRequired,
                    false,
                )
            ),
            Err(WorkflowIdentityError::AttemptChainMismatch)
        );
    }
}

#[test]
fn automatic_retry_is_forbidden_for_every_high_or_unknown_effect() {
    for effect in [
        CanonicalEffectClass::NonIdempotent,
        CanonicalEffectClass::Destructive,
        CanonicalEffectClass::External,
        CanonicalEffectClass::Unknown,
    ] {
        let issuer = WorkflowIdentityIssuer::new(format!("execution:{effect:?}")).unwrap();
        let policy = policy(effect, CanonicalRetryClass::UserDecisionRequired);
        let first = issuer
            .issue_attempt(
                &policy,
                candidate(
                    "1",
                    1,
                    None,
                    AttemptOpeningKind::Initial,
                    AttemptReconciliationState::NotRequired,
                    true,
                ),
            )
            .unwrap();
        issuer
            .issue_receipt(
                &first,
                ReceiptId::from_raw("receipt:1"),
                AttemptTerminalEffect::Failed,
            )
            .unwrap();
        assert_eq!(
            issuer.issue_attempt(
                &policy,
                candidate(
                    "2",
                    2,
                    Some("attempt:1"),
                    AttemptOpeningKind::AutomaticRetry,
                    AttemptReconciliationState::SafeForFreshAttempt,
                    true,
                )
            ),
            Err(WorkflowIdentityError::AutomaticRetryForbidden),
            "effect {effect:?}"
        );
    }
}

#[test]
fn uncertain_effects_cannot_open_a_successor_until_safely_reconciled() {
    let issuer = WorkflowIdentityIssuer::new("execution:1".to_owned()).unwrap();
    let policy = policy(
        CanonicalEffectClass::ReadOnly,
        CanonicalRetryClass::RecoverableRead,
    );
    let first = issuer
        .issue_attempt(
            &policy,
            candidate(
                "1",
                1,
                None,
                AttemptOpeningKind::Initial,
                AttemptReconciliationState::NotRequired,
                false,
            ),
        )
        .unwrap();
    issuer
        .issue_receipt(
            &first,
            ReceiptId::from_raw("receipt:1"),
            AttemptTerminalEffect::Uncertain,
        )
        .unwrap();
    assert_eq!(
        issuer.issue_attempt(
            &policy,
            candidate(
                "2",
                2,
                Some("attempt:1"),
                AttemptOpeningKind::AutomaticRetry,
                AttemptReconciliationState::Uncertain,
                false,
            )
        ),
        Err(WorkflowIdentityError::AutomaticRetryForbidden)
    );
    assert_eq!(
        issuer.issue_attempt(
            &policy,
            candidate(
                "2-safe-auto",
                2,
                Some("attempt:1"),
                AttemptOpeningKind::AutomaticRetry,
                AttemptReconciliationState::SafeForFreshAttempt,
                false,
            )
        ),
        Err(WorkflowIdentityError::AutomaticRetryForbidden)
    );
    assert!(
        issuer
            .issue_attempt(
                &policy,
                candidate(
                    "2-safe-approved",
                    2,
                    Some("attempt:1"),
                    AttemptOpeningKind::UserApprovedRetry,
                    AttemptReconciliationState::SafeForFreshAttempt,
                    true,
                )
            )
            .is_ok()
    );
}

#[test]
fn approvals_receipts_and_verifications_are_fresh_single_issue_identities() {
    let issuer = WorkflowIdentityIssuer::new("execution:1".to_owned()).unwrap();
    let policy = policy(
        CanonicalEffectClass::External,
        CanonicalRetryClass::UserDecisionRequired,
    );
    let mut missing = candidate(
        "1",
        1,
        None,
        AttemptOpeningKind::Initial,
        AttemptReconciliationState::NotRequired,
        false,
    );
    assert_eq!(
        issuer.issue_attempt(&policy, missing.clone()),
        Err(WorkflowIdentityError::FreshApprovalRequired)
    );
    missing.approval_id = Some(ApprovalId::from_raw("approval:1"));
    let attempt = issuer.issue_attempt(&policy, missing).unwrap();
    let receipt = issuer
        .issue_receipt(
            &attempt,
            ReceiptId::from_raw("receipt:1"),
            AttemptTerminalEffect::Succeeded,
        )
        .unwrap();
    assert_eq!(
        issuer.issue_receipt(
            &attempt,
            ReceiptId::from_raw("receipt:2"),
            AttemptTerminalEffect::Succeeded
        ),
        Err(WorkflowIdentityError::ReceiptAlreadyIssued)
    );
    issuer
        .issue_verification(
            &receipt,
            "verification:1".to_owned(),
            "verifier:identity".to_owned(),
        )
        .unwrap();
    assert_eq!(
        issuer.issue_verification(
            &receipt,
            "verification:2".to_owned(),
            "verifier:identity".to_owned()
        ),
        Err(WorkflowIdentityError::VerificationAlreadyIssued)
    );
}

#[test]
fn policy_tampering_cannot_enable_a_retry() {
    let issuer = WorkflowIdentityIssuer::new("execution:1".to_owned()).unwrap();
    let mut changed = policy(
        CanonicalEffectClass::External,
        CanonicalRetryClass::UserDecisionRequired,
    );
    changed.effect_class = CanonicalEffectClass::ReadOnly;
    changed.retry_class = CanonicalRetryClass::RecoverableRead;
    changed.approval_requirement = CanonicalApprovalRequirement::NotRequired;
    assert_eq!(
        issuer.issue_attempt(
            &changed,
            candidate(
                "1",
                1,
                None,
                AttemptOpeningKind::Initial,
                AttemptReconciliationState::NotRequired,
                false,
            )
        ),
        Err(WorkflowIdentityError::PolicyBindingMismatch)
    );
}

#[test]
fn required_once_approval_is_fresh_initially_and_not_reissued_automatically() {
    let issuer = WorkflowIdentityIssuer::new("execution:once".to_owned()).unwrap();
    let mut policy = policy(
        CanonicalEffectClass::ReadOnly,
        CanonicalRetryClass::RecoverableRead,
    );
    policy.approval_requirement = CanonicalApprovalRequirement::RequiredOnce;
    policy.policy_sha256 = ZERO.to_owned();
    policy.policy_sha256 = sha256(&to_canonical_json(&policy).unwrap());
    assert_eq!(
        issuer.issue_attempt(
            &policy,
            candidate(
                "1-missing",
                1,
                None,
                AttemptOpeningKind::Initial,
                AttemptReconciliationState::NotRequired,
                false,
            )
        ),
        Err(WorkflowIdentityError::FreshApprovalRequired)
    );
    let first = issuer
        .issue_attempt(
            &policy,
            candidate(
                "1",
                1,
                None,
                AttemptOpeningKind::Initial,
                AttemptReconciliationState::NotRequired,
                true,
            ),
        )
        .unwrap();
    issuer
        .issue_receipt(
            &first,
            ReceiptId::from_raw("receipt:once"),
            AttemptTerminalEffect::Failed,
        )
        .unwrap();
    assert!(
        issuer
            .issue_attempt(
                &policy,
                candidate(
                    "2",
                    2,
                    Some("attempt:1"),
                    AttemptOpeningKind::AutomaticRetry,
                    AttemptReconciliationState::NotRequired,
                    false,
                )
            )
            .is_ok()
    );
}

#[test]
fn concurrent_duplicate_issue_has_exactly_one_winner() {
    let issuer = Arc::new(WorkflowIdentityIssuer::new("execution:race".to_owned()).unwrap());
    let policy = Arc::new(policy(
        CanonicalEffectClass::ReadOnly,
        CanonicalRetryClass::RecoverableRead,
    ));
    let barrier = Arc::new(Barrier::new(16));
    let handles = (0..16)
        .map(|_| {
            let issuer = Arc::clone(&issuer);
            let policy = Arc::clone(&policy);
            let barrier = Arc::clone(&barrier);
            std::thread::spawn(move || {
                barrier.wait();
                issuer.issue_attempt(
                    &policy,
                    candidate(
                        "race",
                        1,
                        None,
                        AttemptOpeningKind::Initial,
                        AttemptReconciliationState::NotRequired,
                        false,
                    ),
                )
            })
        })
        .collect::<Vec<_>>();
    let results = handles
        .into_iter()
        .map(|handle| handle.join().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
    assert_eq!(
        results
            .iter()
            .filter(|result| result.as_ref().err()
                == Some(&WorkflowIdentityError::AttemptChainMismatch))
            .count(),
        15
    );
}
