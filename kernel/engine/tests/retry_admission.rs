use agentmage_kernel_contracts::{
    ActionId, ActionKind, ActorId, ApprovalId, ApprovalRequest, CanonicalApprovalRequirement,
    CanonicalDiagnosticDisclosure, CanonicalEffectClass, CanonicalExecutionAuthority,
    CanonicalExecutionBudgets, CanonicalFailureClass, CanonicalIdempotencyRequirement,
    CanonicalRecoveryAction, CanonicalRecoveryDecision, CanonicalRetryAdmission,
    CanonicalRetryClass, CanonicalStepExecutionPolicy, CanonicalVerificationRequirement,
    CapabilityGrant, ContractPayload, CorrelationId, DataSensitivity, GrantClass, GrantId,
    GrantNonce, GrantOperation, GrantPreimage, GrantSideEffect, GrantStatus, GrantTarget,
    OperationBinding, PlanId, PlanStepId, SchemaId, SchemaReference, SessionId, TaskId, ToolCall,
    ToolCallId, ToolId,
};
use agentmage_kernel_engine::retry_admission::{
    AttemptEffectOutcome, AttemptExecutionGateError, CurrentAttemptApproval,
    CurrentEffectReconciliation, CurrentPreflightEvidence, EffectReconciliationDisposition,
    FreshAttemptAdmissionError, FreshAttemptAdmissionInput, FreshSingleUseGrant,
    PriorExecutionIdentityLedger, SynchronizedAttemptExecutionGate,
    compile_execution_ready_attempt, compile_fresh_attempt_admission,
};
use sha2::{Digest, Sha256};
use std::fmt::Write as _;
use std::sync::{
    Arc, Barrier,
    atomic::{AtomicUsize, Ordering},
};
use std::thread;

const ZERO_SHA256: &str = "0000000000000000000000000000000000000000000000000000000000000000";
const PRIOR_SHA256: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const OTHER_SHA256: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

fn record_sha256(value: &impl serde::Serialize) -> String {
    let bytes = serde_json::to_vec(value).expect("record serialization");
    let mut output = String::with_capacity(64);
    for byte in Sha256::digest(bytes) {
        let _ = write!(output, "{byte:02x}");
    }
    output
}

fn seal_policy(mut value: CanonicalStepExecutionPolicy) -> CanonicalStepExecutionPolicy {
    value.policy_sha256 = ZERO_SHA256.to_owned();
    value.policy_sha256 = record_sha256(&value);
    value
}

fn seal_decision(mut value: CanonicalRecoveryDecision) -> CanonicalRecoveryDecision {
    value.decision_sha256 = ZERO_SHA256.to_owned();
    value.decision_sha256 = record_sha256(&value);
    value
}

fn seal_admission(mut value: CanonicalRetryAdmission) -> CanonicalRetryAdmission {
    value.admission_sha256 = ZERO_SHA256.to_owned();
    value.admission_sha256 = record_sha256(&value);
    value
}

fn seal_approval(mut value: ApprovalRequest) -> ApprovalRequest {
    value.confirmation_sha256 = ZERO_SHA256.to_owned();
    value.confirmation_sha256 = record_sha256(&value);
    value
}

fn policy(retry_class: CanonicalRetryClass) -> CanonicalStepExecutionPolicy {
    seal_policy(CanonicalStepExecutionPolicy {
        schema_version: 1,
        policy_id: "policy-1".to_owned(),
        plan_id: PlanId::from_raw("plan-1"),
        plan_step_id: PlanStepId::from_raw("plan-1:step:1"),
        plan_revision: 1,
        preflight_policy_id: "preflight-policy-1".to_owned(),
        required_preflight_ids: vec!["workspace-current".to_owned()],
        side_effect_policy_id: "effect-policy-1".to_owned(),
        effect_class: CanonicalEffectClass::ReadOnly,
        approval_policy_id: "approval-policy-1".to_owned(),
        approval_requirement: CanonicalApprovalRequirement::NotRequired,
        idempotency_policy_id: "idempotency-policy-1".to_owned(),
        idempotency_key_requirement: CanonicalIdempotencyRequirement::NotApplicable,
        verifier_policy_id: "verifier-policy-1".to_owned(),
        verification_requirement: CanonicalVerificationRequirement::VerifierEvidenceRequired,
        required_verifier_ids: vec!["result-current".to_owned()],
        deferral_reason_code: None,
        retry_policy_id: "retry-policy-1".to_owned(),
        retry_class,
        budget_policy_id: "budget-policy-1".to_owned(),
        budgets: CanonicalExecutionBudgets {
            turns: 3,
            tokens: 100,
            duration_ms: 1_000,
            tool_calls: 3,
            attempts: 2,
            no_progress_events: 1,
            output_bytes: 1_024,
            memory_bytes: 1_024,
            cost_minor_units: 0,
        },
        diagnostic_policy_id: "diagnostic-policy-1".to_owned(),
        diagnostic_disclosure: CanonicalDiagnosticDisclosure::ContentFreeCodes,
        recorded_at: "2026-08-30T00:00:00Z".to_owned(),
        policy_sha256: ZERO_SHA256.to_owned(),
    })
}

fn decision() -> CanonicalRecoveryDecision {
    seal_decision(CanonicalRecoveryDecision {
        schema_version: 1,
        decision_id: "decision-1".to_owned(),
        step_execution_id: "step-execution-1".to_owned(),
        attempt_id: "attempt-1".to_owned(),
        policy_id: "policy-1".to_owned(),
        failure_class: CanonicalFailureClass::Transport,
        uncertain_outcome: false,
        decision: CanonicalRecoveryAction::RetryNewAttempt,
        reason_code: "transport-transient".to_owned(),
        decided_at: "2026-08-30T00:00:01Z".to_owned(),
        established_by: CanonicalExecutionAuthority::AgentmageRuntime,
        decision_sha256: ZERO_SHA256.to_owned(),
    })
}

fn admission(required: CanonicalApprovalRequirement) -> CanonicalRetryAdmission {
    seal_admission(CanonicalRetryAdmission {
        schema_version: 1,
        admission_id: "admission-1".to_owned(),
        step_execution_id: "step-execution-1".to_owned(),
        policy_id: "policy-1".to_owned(),
        decision_id: "decision-1".to_owned(),
        prior_attempt_id: "attempt-1".to_owned(),
        prior_attempt_ordinal: 1,
        prior_call_id: "call-1".to_owned(),
        prior_tool_call_id: ToolCallId::from_raw("tool-call-1"),
        prior_grant_id: GrantId::from_raw("grant-1"),
        successor_attempt_id: "attempt-2".to_owned(),
        successor_attempt_ordinal: 2,
        successor_call_id: "call-2".to_owned(),
        successor_tool_call_id: ToolCallId::from_raw("tool-call-2"),
        successor_grant_id: GrantId::from_raw("grant-2"),
        approval_requirement: required,
        successor_approval_id: (required == CanonicalApprovalRequirement::RequiredPerAttempt)
            .then(|| "approval-2".to_owned()),
        reconciliation_required: false,
        reconciled: false,
        admitted_at: "2026-08-30T00:00:02Z".to_owned(),
        established_by: CanonicalExecutionAuthority::AgentmageRuntime,
        admission_sha256: ZERO_SHA256.to_owned(),
    })
}

fn grant(
    required: CanonicalApprovalRequirement,
    policy: &CanonicalStepExecutionPolicy,
) -> CapabilityGrant {
    CapabilityGrant {
        schema_version: 1,
        grant_id: GrantId::from_raw("grant-2"),
        revision: 1,
        grant_class: GrantClass::Operation,
        actor_id: ActorId::from_raw("actor-1"),
        approval_id: (required == CanonicalApprovalRequirement::RequiredPerAttempt)
            .then(|| ApprovalId::from_raw("approval-2")),
        session_id: SessionId::from_raw("session-1"),
        task_id: TaskId::from_raw("task-1"),
        action_id: Some(ActionId::from_raw("action-2")),
        action_kind: Some(ActionKind::DeterministicTool),
        operation: OperationBinding::new(GrantOperation::WorkspaceRead),
        tool_id: Some(ToolId::from_raw("read")),
        tool_version: Some("1".to_owned()),
        targets: Vec::new(),
        excluded_targets: Vec::new(),
        sensitivity: DataSensitivity::Ephemeral,
        argument_sha256: OTHER_SHA256.to_owned(),
        preimages: Vec::new(),
        expected_side_effects: Vec::new(),
        rollback_description: "No effect".to_owned(),
        issued_at_epoch_ms: 1_000,
        expires_at_epoch_ms: 2_000,
        nonce: GrantNonce::from_raw("nonce-2"),
        use_limit: 1,
        use_count: 0,
        parent_grant_id: Some(GrantId::from_raw("parent-1")),
        parent_grant_sha256: Some(OTHER_SHA256.to_owned()),
        preview_sha256: OTHER_SHA256.to_owned(),
        policy_sha256: policy.policy_sha256.clone(),
        status: GrantStatus::Issued,
    }
}

fn approval(policy: &CanonicalStepExecutionPolicy) -> ApprovalRequest {
    seal_approval(ApprovalRequest {
        schema_version: 1,
        approval_id: ApprovalId::from_raw("approval-2"),
        proposed_grant_id: GrantId::from_raw("grant-2"),
        parent_grant_id: GrantId::from_raw("parent-1"),
        parent_grant_sha256: OTHER_SHA256.to_owned(),
        actor_id: ActorId::from_raw("actor-1"),
        session_id: SessionId::from_raw("session-1"),
        task_id: TaskId::from_raw("task-1"),
        action_kind: ActionKind::DeterministicTool,
        operation: OperationBinding::new(GrantOperation::WorkspaceRead),
        tool_call: ToolCall {
            schema_version: 1,
            tool_call_id: ToolCallId::from_raw("tool-call-2"),
            correlation_id: CorrelationId::from_raw("correlation-2"),
            action_id: ActionId::from_raw("action-2"),
            tool_id: ToolId::from_raw("read"),
            tool_version: "1".to_owned(),
            arguments: ContractPayload {
                schema: SchemaReference {
                    schema_id: SchemaId::from_raw("read-input"),
                    schema_version: 1,
                    schema_sha256: OTHER_SHA256.to_owned(),
                },
                media_type: "application/json".to_owned(),
                bytes: b"{}".to_vec(),
                sha256: OTHER_SHA256.to_owned(),
            },
        },
        targets: Vec::new(),
        excluded_targets: Vec::new(),
        sensitivity: DataSensitivity::Ephemeral,
        preimages: Vec::new(),
        expected_side_effects: Vec::new(),
        rollback_description: "No effect".to_owned(),
        issued_at_epoch_ms: 1_000,
        expires_at_epoch_ms: 2_000,
        policy_sha256: policy.policy_sha256.clone(),
        confirmation_sha256: ZERO_SHA256.to_owned(),
    })
}

fn preflight(policy: &CanonicalStepExecutionPolicy) -> CurrentPreflightEvidence {
    CurrentPreflightEvidence::new(
        "step-execution-1".to_owned(),
        policy.policy_sha256.clone(),
        vec!["workspace-current".to_owned()],
        1_000,
        2_000,
    )
    .expect("current preflight")
}

fn prior_ledger() -> PriorExecutionIdentityLedger {
    PriorExecutionIdentityLedger::new(
        vec!["attempt-1".to_owned()],
        vec!["call-1".to_owned()],
        vec!["tool-call-1".to_owned()],
        vec!["grant-1".to_owned()],
        vec!["approval-1".to_owned()],
        vec!["receipt-1".to_owned()],
        vec![PRIOR_SHA256.to_owned()],
    )
    .expect("closed prior-use ledger")
}

#[allow(clippy::too_many_arguments)]
fn compile(
    candidate: CanonicalRetryAdmission,
    policy: &CanonicalStepExecutionPolicy,
    decision: &CanonicalRecoveryDecision,
    preflight: &CurrentPreflightEvidence,
    reconciliation: Option<&CurrentEffectReconciliation>,
    grant: &CapabilityGrant,
    approval: Option<&ApprovalRequest>,
    prior_approval_id: Option<&ApprovalId>,
) -> Result<CanonicalRetryAdmission, FreshAttemptAdmissionError> {
    let prior_identities = prior_ledger();
    compile_with_controls(
        candidate,
        policy,
        decision,
        preflight,
        reconciliation,
        grant,
        approval,
        prior_approval_id,
        &prior_identities,
        None,
        None,
    )
}

#[allow(clippy::too_many_arguments)]
fn compile_with_controls(
    candidate: CanonicalRetryAdmission,
    policy: &CanonicalStepExecutionPolicy,
    decision: &CanonicalRecoveryDecision,
    preflight: &CurrentPreflightEvidence,
    reconciliation: Option<&CurrentEffectReconciliation>,
    grant: &CapabilityGrant,
    approval: Option<&ApprovalRequest>,
    prior_approval_id: Option<&ApprovalId>,
    prior_identities: &PriorExecutionIdentityLedger,
    successor_idempotency_key_sha256: Option<&str>,
    presented_successor_receipt_id: Option<&str>,
) -> Result<CanonicalRetryAdmission, FreshAttemptAdmissionError> {
    compile_fresh_attempt_admission(FreshAttemptAdmissionInput {
        candidate,
        policy,
        decision,
        preflight,
        reconciliation,
        successor_grant: FreshSingleUseGrant::new(grant),
        successor_approval: approval.map(CurrentAttemptApproval::new),
        prior_approval_id,
        prior_identities,
        successor_idempotency_key_sha256,
        presented_successor_receipt_id,
        now_epoch_ms: 1_500,
    })
}

#[test]
fn fresh_attempt_requires_current_preflight_remaining_budget_and_single_use_grant() {
    let policy = policy(CanonicalRetryClass::RecoverableRead);
    let decision = decision();
    let current_preflight = preflight(&policy);
    let current_grant = grant(CanonicalApprovalRequirement::NotRequired, &policy);
    let result = compile(
        admission(CanonicalApprovalRequirement::NotRequired),
        &policy,
        &decision,
        &current_preflight,
        None,
        &current_grant,
        None,
        None,
    )
    .expect("all fresh prerequisites must admit one successor");
    assert_eq!(result.successor_attempt_id, "attempt-2");

    let stale_preflight = CurrentPreflightEvidence::new(
        "step-execution-1".to_owned(),
        policy.policy_sha256.clone(),
        vec!["workspace-current".to_owned()],
        1_000,
        1_500,
    )
    .expect("boundary-stale evidence is structurally valid");
    assert_eq!(
        compile(
            admission(CanonicalApprovalRequirement::NotRequired),
            &policy,
            &decision,
            &stale_preflight,
            None,
            &current_grant,
            None,
            None,
        ),
        Err(FreshAttemptAdmissionError::PreflightNotCurrent),
    );

    let mut exhausted = policy.clone();
    exhausted.budgets.attempts = 1;
    let exhausted = seal_policy(exhausted);
    let exhausted_preflight = preflight(&exhausted);
    let exhausted_grant = grant(CanonicalApprovalRequirement::NotRequired, &exhausted);
    assert_eq!(
        compile(
            admission(CanonicalApprovalRequirement::NotRequired),
            &exhausted,
            &decision,
            &exhausted_preflight,
            None,
            &exhausted_grant,
            None,
            None,
        ),
        Err(FreshAttemptAdmissionError::AttemptBudgetExhausted),
    );

    let mut used = current_grant.clone();
    used.use_count = 1;
    assert_eq!(
        compile(
            admission(CanonicalApprovalRequirement::NotRequired),
            &policy,
            &decision,
            &current_preflight,
            None,
            &used,
            None,
            None,
        ),
        Err(FreshAttemptAdmissionError::GrantNotFreshSingleUse),
    );
    let mut reusable = current_grant.clone();
    reusable.use_limit = 2;
    assert_eq!(
        compile(
            admission(CanonicalApprovalRequirement::NotRequired),
            &policy,
            &decision,
            &current_preflight,
            None,
            &reusable,
            None,
            None,
        ),
        Err(FreshAttemptAdmissionError::GrantNotFreshSingleUse),
    );
}

#[test]
fn conditional_retry_requires_current_safe_effect_reconciliation() {
    let mut policy = policy(CanonicalRetryClass::ConditionalAfterReconciliation);
    policy.effect_class = CanonicalEffectClass::Conditional;
    let policy = seal_policy(policy);
    let decision = decision();
    let preflight = preflight(&policy);
    let grant = grant(CanonicalApprovalRequirement::NotRequired, &policy);
    let mut candidate = admission(CanonicalApprovalRequirement::NotRequired);
    candidate.reconciliation_required = true;
    candidate.reconciled = true;
    let candidate = seal_admission(candidate);
    assert_eq!(
        compile(
            candidate.clone(),
            &policy,
            &decision,
            &preflight,
            None,
            &grant,
            None,
            None,
        ),
        Err(FreshAttemptAdmissionError::ReconciliationNotCurrent),
    );
    let evidence = CurrentEffectReconciliation::new(
        "attempt-1".to_owned(),
        policy.policy_sha256.clone(),
        OTHER_SHA256.to_owned(),
        EffectReconciliationDisposition::SafeForFreshAttempt,
        1_250,
        2_000,
    )
    .expect("current reconciliation");
    assert_eq!(evidence.observation_sha256(), OTHER_SHA256);
    assert!(
        compile(
            candidate,
            &policy,
            &decision,
            &preflight,
            Some(&evidence),
            &grant,
            None,
            None,
        )
        .is_ok()
    );
}

#[test]
fn per_attempt_approval_must_be_current_exact_and_different_from_prior() {
    let mut policy = policy(CanonicalRetryClass::RecoverableRead);
    policy.approval_requirement = CanonicalApprovalRequirement::RequiredPerAttempt;
    let policy = seal_policy(policy);
    let decision = decision();
    let preflight = preflight(&policy);
    let grant = grant(CanonicalApprovalRequirement::RequiredPerAttempt, &policy);
    let approval = approval(&policy);
    let prior = ApprovalId::from_raw("approval-1");
    assert!(
        compile(
            admission(CanonicalApprovalRequirement::RequiredPerAttempt),
            &policy,
            &decision,
            &preflight,
            None,
            &grant,
            Some(&approval),
            Some(&prior),
        )
        .is_ok()
    );
    let reused = ApprovalId::from_raw("approval-2");
    assert_eq!(
        compile(
            admission(CanonicalApprovalRequirement::RequiredPerAttempt),
            &policy,
            &decision,
            &preflight,
            None,
            &grant,
            Some(&approval),
            Some(&reused),
        ),
        Err(FreshAttemptAdmissionError::FreshApprovalRequired),
    );
}

#[test]
fn unsafe_and_uncertain_effects_never_receive_automatic_new_attempts() {
    let decision = decision();
    let base_policy = policy(CanonicalRetryClass::RecoverableRead);
    let preflight = preflight(&base_policy);
    let grant = grant(CanonicalApprovalRequirement::NotRequired, &base_policy);
    for effect_class in [
        CanonicalEffectClass::NonIdempotent,
        CanonicalEffectClass::Destructive,
        CanonicalEffectClass::External,
        CanonicalEffectClass::Unknown,
    ] {
        let mut unsafe_policy = policy(CanonicalRetryClass::RecoverableRead);
        unsafe_policy.effect_class = effect_class;
        let unsafe_policy = seal_policy(unsafe_policy);
        assert_eq!(
            compile(
                admission(CanonicalApprovalRequirement::NotRequired),
                &unsafe_policy,
                &decision,
                &preflight,
                None,
                &grant,
                None,
                None,
            ),
            Err(FreshAttemptAdmissionError::AutomaticRetryForbidden),
            "{effect_class:?} must never be admitted automatically",
        );
    }

    let policy = base_policy;
    let mut uncertain = decision;
    uncertain.uncertain_outcome = true;
    let uncertain = seal_decision(uncertain);
    assert_eq!(
        compile(
            admission(CanonicalApprovalRequirement::NotRequired),
            &policy,
            &uncertain,
            &preflight,
            None,
            &grant,
            None,
            None,
        ),
        Err(FreshAttemptAdmissionError::DecisionNotRetry),
    );
}

#[test]
fn complete_prior_use_ledger_denies_every_replayed_identity_and_any_receipt() {
    let base_policy = policy(CanonicalRetryClass::RecoverableRead);
    let decision = decision();
    let preflight = preflight(&base_policy);
    let base_grant = grant(CanonicalApprovalRequirement::NotRequired, &base_policy);

    for ledger in [
        PriorExecutionIdentityLedger::new(
            vec!["attempt-2".to_owned()],
            vec![],
            vec![],
            vec![],
            vec![],
            vec![],
            vec![],
        ),
        PriorExecutionIdentityLedger::new(
            vec![],
            vec!["call-2".to_owned()],
            vec![],
            vec![],
            vec![],
            vec![],
            vec![],
        ),
        PriorExecutionIdentityLedger::new(
            vec![],
            vec![],
            vec!["tool-call-2".to_owned()],
            vec![],
            vec![],
            vec![],
            vec![],
        ),
        PriorExecutionIdentityLedger::new(
            vec![],
            vec![],
            vec![],
            vec!["grant-2".to_owned()],
            vec![],
            vec![],
            vec![],
        ),
    ] {
        let ledger = ledger.expect("single prior identity");
        assert_eq!(
            compile_with_controls(
                admission(CanonicalApprovalRequirement::NotRequired),
                &base_policy,
                &decision,
                &preflight,
                None,
                &base_grant,
                None,
                None,
                &ledger,
                None,
                None,
            ),
            Err(FreshAttemptAdmissionError::ReplayedIdentity),
        );
    }

    let ledger = prior_ledger();
    assert_eq!(
        compile_with_controls(
            admission(CanonicalApprovalRequirement::NotRequired),
            &base_policy,
            &decision,
            &preflight,
            None,
            &base_grant,
            None,
            None,
            &ledger,
            None,
            Some("receipt-1"),
        ),
        Err(FreshAttemptAdmissionError::ReplayedIdentity),
    );
    assert_eq!(
        compile_with_controls(
            admission(CanonicalApprovalRequirement::NotRequired),
            &base_policy,
            &decision,
            &preflight,
            None,
            &base_grant,
            None,
            None,
            &ledger,
            None,
            Some("receipt-2"),
        ),
        Err(FreshAttemptAdmissionError::PrematureReceipt),
    );

    let mut approval_policy = policy(CanonicalRetryClass::RecoverableRead);
    approval_policy.approval_requirement = CanonicalApprovalRequirement::RequiredPerAttempt;
    let approval_policy = seal_policy(approval_policy);
    let approval_grant = grant(
        CanonicalApprovalRequirement::RequiredPerAttempt,
        &approval_policy,
    );
    let approval = approval(&approval_policy);
    let prior_approval = ApprovalId::from_raw("approval-1");
    let approval_ledger = PriorExecutionIdentityLedger::new(
        vec![],
        vec![],
        vec![],
        vec![],
        vec!["approval-2".to_owned()],
        vec![],
        vec![],
    )
    .expect("prior approval ledger");
    assert_eq!(
        compile_with_controls(
            admission(CanonicalApprovalRequirement::RequiredPerAttempt),
            &approval_policy,
            &decision,
            &preflight,
            None,
            &approval_grant,
            Some(&approval),
            Some(&prior_approval),
            &approval_ledger,
            None,
            None,
        ),
        Err(FreshAttemptAdmissionError::ReplayedIdentity),
    );
}

#[test]
fn required_idempotency_key_must_be_fresh_and_digest_bound() {
    let mut policy = policy(CanonicalRetryClass::ConditionalAfterReconciliation);
    policy.effect_class = CanonicalEffectClass::IdempotentWrite;
    policy.idempotency_key_requirement = CanonicalIdempotencyRequirement::Required;
    let policy = seal_policy(policy);
    let decision = decision();
    let preflight = preflight(&policy);
    let grant = grant(CanonicalApprovalRequirement::NotRequired, &policy);
    let mut candidate = admission(CanonicalApprovalRequirement::NotRequired);
    candidate.reconciliation_required = true;
    candidate.reconciled = true;
    let candidate = seal_admission(candidate);
    let reconciliation = CurrentEffectReconciliation::new(
        "attempt-1".to_owned(),
        policy.policy_sha256.clone(),
        OTHER_SHA256.to_owned(),
        EffectReconciliationDisposition::SafeForFreshAttempt,
        1_250,
        2_000,
    )
    .expect("current reconciliation");
    let ledger = prior_ledger();
    assert!(
        compile_with_controls(
            candidate.clone(),
            &policy,
            &decision,
            &preflight,
            Some(&reconciliation),
            &grant,
            None,
            None,
            &ledger,
            Some(OTHER_SHA256),
            None,
        )
        .is_ok()
    );
    for digest in [None, Some("invalid"), Some(PRIOR_SHA256)] {
        assert_eq!(
            compile_with_controls(
                candidate.clone(),
                &policy,
                &decision,
                &preflight,
                Some(&reconciliation),
                &grant,
                None,
                None,
                &ledger,
                digest,
                None,
            ),
            Err(FreshAttemptAdmissionError::FreshIdempotencyEvidenceRequired),
        );
    }
}

fn deny_before_dispatch(
    result: Result<CanonicalRetryAdmission, FreshAttemptAdmissionError>,
    expected: FreshAttemptAdmissionError,
    dispatch_probe: &mut u64,
    case: &str,
) {
    if result.is_ok() {
        *dispatch_probe += 1;
    }
    assert_eq!(result, Err(expected), "mutation case {case}");
    assert_eq!(*dispatch_probe, 0, "{case} reached dispatch");
}

#[test]
fn every_effect_failure_and_budget_mutation_denies_before_dispatch() {
    let base_policy = policy(CanonicalRetryClass::RecoverableRead);
    let base_decision = decision();
    let current_preflight = preflight(&base_policy);
    let current_grant = grant(CanonicalApprovalRequirement::NotRequired, &base_policy);
    let mut dispatch_probe = 0_u64;

    for effect_class in [
        CanonicalEffectClass::IdempotentWrite,
        CanonicalEffectClass::Conditional,
        CanonicalEffectClass::NonIdempotent,
        CanonicalEffectClass::Destructive,
        CanonicalEffectClass::External,
        CanonicalEffectClass::Unknown,
    ] {
        let mut changed = base_policy.clone();
        changed.effect_class = effect_class;
        deny_before_dispatch(
            compile(
                admission(CanonicalApprovalRequirement::NotRequired),
                &changed,
                &base_decision,
                &current_preflight,
                None,
                &current_grant,
                None,
                None,
            ),
            FreshAttemptAdmissionError::IntegrityMismatch,
            &mut dispatch_probe,
            "effect_class",
        );
    }
    for retry_class in [
        CanonicalRetryClass::Never,
        CanonicalRetryClass::ConditionalAfterReconciliation,
        CanonicalRetryClass::UserDecisionRequired,
    ] {
        let mut changed = base_policy.clone();
        changed.retry_class = retry_class;
        deny_before_dispatch(
            compile(
                admission(CanonicalApprovalRequirement::NotRequired),
                &changed,
                &base_decision,
                &current_preflight,
                None,
                &current_grant,
                None,
                None,
            ),
            FreshAttemptAdmissionError::IntegrityMismatch,
            &mut dispatch_probe,
            "retry_class",
        );
    }

    for failure_class in [
        CanonicalFailureClass::Rate,
        CanonicalFailureClass::Timeout,
        CanonicalFailureClass::Crash,
        CanonicalFailureClass::UnavailableService,
        CanonicalFailureClass::MissingCommand,
        CanonicalFailureClass::InvalidArguments,
        CanonicalFailureClass::Authentication,
        CanonicalFailureClass::Permission,
        CanonicalFailureClass::PolicyDenial,
        CanonicalFailureClass::DeterministicVerificationFailure,
        CanonicalFailureClass::MalformedModelOutput,
        CanonicalFailureClass::ContextOverflow,
        CanonicalFailureClass::UserRejection,
    ] {
        let mut changed = base_decision.clone();
        changed.failure_class = failure_class;
        deny_before_dispatch(
            compile(
                admission(CanonicalApprovalRequirement::NotRequired),
                &base_policy,
                &changed,
                &current_preflight,
                None,
                &current_grant,
                None,
                None,
            ),
            FreshAttemptAdmissionError::IntegrityMismatch,
            &mut dispatch_probe,
            "failure_class",
        );
    }
    let mut uncertain = base_decision.clone();
    uncertain.uncertain_outcome = true;
    deny_before_dispatch(
        compile(
            admission(CanonicalApprovalRequirement::NotRequired),
            &base_policy,
            &uncertain,
            &current_preflight,
            None,
            &current_grant,
            None,
            None,
        ),
        FreshAttemptAdmissionError::IntegrityMismatch,
        &mut dispatch_probe,
        "uncertain_outcome",
    );

    for budget_field in 0..9 {
        let mut changed = base_policy.clone();
        match budget_field {
            0 => changed.budgets.turns += 1,
            1 => changed.budgets.tokens += 1,
            2 => changed.budgets.duration_ms += 1,
            3 => changed.budgets.tool_calls += 1,
            4 => changed.budgets.attempts += 1,
            5 => changed.budgets.no_progress_events += 1,
            6 => changed.budgets.output_bytes += 1,
            7 => changed.budgets.memory_bytes += 1,
            8 => changed.budgets.cost_minor_units += 1,
            _ => unreachable!("closed budget field"),
        }
        deny_before_dispatch(
            compile(
                admission(CanonicalApprovalRequirement::NotRequired),
                &changed,
                &base_decision,
                &current_preflight,
                None,
                &current_grant,
                None,
                None,
            ),
            FreshAttemptAdmissionError::IntegrityMismatch,
            &mut dispatch_probe,
            "budget_field",
        );
    }
    assert_eq!(dispatch_probe, 0);
}

#[test]
fn every_approval_field_mutation_denies_before_dispatch() {
    let mut required_policy = policy(CanonicalRetryClass::RecoverableRead);
    required_policy.approval_requirement = CanonicalApprovalRequirement::RequiredPerAttempt;
    let required_policy = seal_policy(required_policy);
    let recovery = decision();
    let current_preflight = preflight(&required_policy);
    let current_grant = grant(
        CanonicalApprovalRequirement::RequiredPerAttempt,
        &required_policy,
    );
    let base = approval(&required_policy);
    let prior = ApprovalId::from_raw("approval-1");
    let target: GrantTarget = serde_json::from_value(serde_json::json!({
        "target_kind": "held_object",
        "path": {"workspace_id": "workspace-1", "components": ["file.txt"]},
        "authorization_id": "authorization-1",
        "adapter_instance_id": "adapter-1",
        "platform": "deterministic_fake",
        "object_kind": "regular_file",
        "object_identity": {
            "platform": "deterministic_fake",
            "mount_identity_sha256": vec![1_u8; 32],
            "object_identity_sha256": vec![2_u8; 32]
        },
        "preimage": {"byte_len": 1, "content_sha256": vec![3_u8; 32]}
    }))
    .expect("synthetic held target");
    let mut mutations: Vec<(&str, ApprovalRequest)> = Vec::new();
    macro_rules! mutation {
        ($name:literal, $change:expr) => {{
            let mut value = base.clone();
            $change(&mut value);
            mutations.push(($name, value));
        }};
    }
    mutation!("schema_version", |v: &mut ApprovalRequest| v
        .schema_version +=
        1);
    mutation!("approval_id", |v: &mut ApprovalRequest| v.approval_id =
        ApprovalId::from_raw("approval-x"));
    mutation!("proposed_grant_id", |v: &mut ApprovalRequest| v
        .proposed_grant_id =
        GrantId::from_raw("grant-x"));
    mutation!("parent_grant_id", |v: &mut ApprovalRequest| v
        .parent_grant_id =
        GrantId::from_raw("parent-x"));
    mutation!("parent_grant_sha256", |v: &mut ApprovalRequest| v
        .parent_grant_sha256 =
        PRIOR_SHA256.to_owned());
    mutation!("actor_id", |v: &mut ApprovalRequest| v.actor_id =
        ActorId::from_raw("actor-x"));
    mutation!("session_id", |v: &mut ApprovalRequest| v.session_id =
        SessionId::from_raw("session-x"));
    mutation!("task_id", |v: &mut ApprovalRequest| v.task_id =
        TaskId::from_raw("task-x"));
    mutation!("action_kind", |v: &mut ApprovalRequest| v.action_kind =
        ActionKind::KernelDecision);
    mutation!("operation", |v: &mut ApprovalRequest| v.operation =
        OperationBinding::new(GrantOperation::WorkspaceWrite));
    mutation!("tool_call", |v: &mut ApprovalRequest| v
        .tool_call
        .tool_call_id =
        ToolCallId::from_raw("tool-call-x"));
    mutation!("targets", |v: &mut ApprovalRequest| v
        .targets
        .push(target.clone()));
    mutation!("excluded_targets", |v: &mut ApprovalRequest| v
        .excluded_targets
        .push(target.clone()));
    mutation!("sensitivity", |v: &mut ApprovalRequest| v.sensitivity =
        DataSensitivity::Restricted);
    mutation!("preimages", |v: &mut ApprovalRequest| v.preimages.push(
        GrantPreimage {
            target_index: 0,
            content_sha256: OTHER_SHA256.to_owned(),
            observed_revision: None
        }
    ));
    mutation!("expected_side_effects", |v: &mut ApprovalRequest| v
        .expected_side_effects
        .push(GrantSideEffect {
            operation: OperationBinding::new(GrantOperation::WorkspaceWrite),
            target_indexes: vec![0],
            details_sha256: OTHER_SHA256.to_owned()
        }));
    mutation!("rollback_description", |v: &mut ApprovalRequest| v
        .rollback_description =
        "Changed recovery".to_owned());
    mutation!("issued_at_epoch_ms", |v: &mut ApprovalRequest| v
        .issued_at_epoch_ms +=
        1);
    mutation!("expires_at_epoch_ms", |v: &mut ApprovalRequest| v
        .expires_at_epoch_ms +=
        1);
    mutation!("policy_sha256", |v: &mut ApprovalRequest| v.policy_sha256 =
        PRIOR_SHA256.to_owned());
    mutation!("confirmation_sha256", |v: &mut ApprovalRequest| v
        .confirmation_sha256 =
        PRIOR_SHA256.to_owned());

    let mut dispatch_probe = 0_u64;
    for (case, changed) in mutations {
        deny_before_dispatch(
            compile(
                admission(CanonicalApprovalRequirement::RequiredPerAttempt),
                &required_policy,
                &recovery,
                &current_preflight,
                None,
                &current_grant,
                Some(&changed),
                Some(&prior),
            ),
            FreshAttemptAdmissionError::FreshApprovalRequired,
            &mut dispatch_probe,
            case,
        );
    }
    assert_eq!(dispatch_probe, 0);
}

#[test]
fn every_preflight_and_reconciliation_field_mutation_denies_before_dispatch() {
    let mut policy = policy(CanonicalRetryClass::ConditionalAfterReconciliation);
    policy.effect_class = CanonicalEffectClass::Conditional;
    let policy = seal_policy(policy);
    let recovery = decision();
    let current_preflight = preflight(&policy);
    let current_grant = grant(CanonicalApprovalRequirement::NotRequired, &policy);
    let mut candidate = admission(CanonicalApprovalRequirement::NotRequired);
    candidate.reconciliation_required = true;
    candidate.reconciled = true;
    let candidate = seal_admission(candidate);
    let current_reconciliation = CurrentEffectReconciliation::new(
        "attempt-1".to_owned(),
        policy.policy_sha256.clone(),
        OTHER_SHA256.to_owned(),
        EffectReconciliationDisposition::SafeForFreshAttempt,
        1_250,
        2_000,
    )
    .expect("current reconciliation");
    let preflight_cases = [
        CurrentPreflightEvidence::new(
            "step-execution-x".to_owned(),
            policy.policy_sha256.clone(),
            vec!["workspace-current".to_owned()],
            1_000,
            2_000,
        )
        .expect("changed step"),
        CurrentPreflightEvidence::new(
            "step-execution-1".to_owned(),
            PRIOR_SHA256.to_owned(),
            vec!["workspace-current".to_owned()],
            1_000,
            2_000,
        )
        .expect("changed policy"),
        CurrentPreflightEvidence::new(
            "step-execution-1".to_owned(),
            policy.policy_sha256.clone(),
            vec!["workspace-other".to_owned()],
            1_000,
            2_000,
        )
        .expect("changed result"),
        CurrentPreflightEvidence::new(
            "step-execution-1".to_owned(),
            policy.policy_sha256.clone(),
            vec!["workspace-current".to_owned()],
            1_501,
            2_000,
        )
        .expect("future observation"),
        CurrentPreflightEvidence::new(
            "step-execution-1".to_owned(),
            policy.policy_sha256.clone(),
            vec!["workspace-current".to_owned()],
            1_000,
            1_500,
        )
        .expect("expired observation"),
    ];
    let mut dispatch_probe = 0_u64;
    for (index, changed) in preflight_cases.iter().enumerate() {
        let expected = if index == 0 {
            FreshAttemptAdmissionError::BindingMismatch
        } else {
            FreshAttemptAdmissionError::PreflightNotCurrent
        };
        deny_before_dispatch(
            compile(
                candidate.clone(),
                &policy,
                &recovery,
                changed,
                Some(&current_reconciliation),
                &current_grant,
                None,
                None,
            ),
            expected,
            &mut dispatch_probe,
            "preflight_field",
        );
    }
    let reconciliation_cases = [
        CurrentEffectReconciliation::new(
            "attempt-x".to_owned(),
            policy.policy_sha256.clone(),
            OTHER_SHA256.to_owned(),
            EffectReconciliationDisposition::SafeForFreshAttempt,
            1_250,
            2_000,
        )
        .expect("changed attempt"),
        CurrentEffectReconciliation::new(
            "attempt-1".to_owned(),
            PRIOR_SHA256.to_owned(),
            OTHER_SHA256.to_owned(),
            EffectReconciliationDisposition::SafeForFreshAttempt,
            1_250,
            2_000,
        )
        .expect("changed policy"),
        CurrentEffectReconciliation::new(
            "attempt-1".to_owned(),
            policy.policy_sha256.clone(),
            PRIOR_SHA256.to_owned(),
            EffectReconciliationDisposition::EffectUncertain,
            1_250,
            2_000,
        )
        .expect("changed observation and disposition"),
        CurrentEffectReconciliation::new(
            "attempt-1".to_owned(),
            policy.policy_sha256.clone(),
            OTHER_SHA256.to_owned(),
            EffectReconciliationDisposition::DesiredStateAlreadyPresent,
            1_250,
            2_000,
        )
        .expect("changed disposition"),
        CurrentEffectReconciliation::new(
            "attempt-1".to_owned(),
            policy.policy_sha256.clone(),
            OTHER_SHA256.to_owned(),
            EffectReconciliationDisposition::SafeForFreshAttempt,
            1_501,
            2_000,
        )
        .expect("future observation"),
        CurrentEffectReconciliation::new(
            "attempt-1".to_owned(),
            policy.policy_sha256.clone(),
            OTHER_SHA256.to_owned(),
            EffectReconciliationDisposition::SafeForFreshAttempt,
            1_000,
            1_500,
        )
        .expect("expired observation"),
    ];
    for changed in &reconciliation_cases {
        deny_before_dispatch(
            compile(
                candidate.clone(),
                &policy,
                &recovery,
                &current_preflight,
                Some(changed),
                &current_grant,
                None,
                None,
            ),
            FreshAttemptAdmissionError::ReconciliationNotCurrent,
            &mut dispatch_probe,
            "reconciliation_field",
        );
    }
    assert_eq!(dispatch_probe, 0);
}

#[test]
fn every_attempt_identity_field_mutation_denies_before_dispatch() {
    let policy = policy(CanonicalRetryClass::RecoverableRead);
    let recovery = decision();
    let current_preflight = preflight(&policy);
    let current_grant = grant(CanonicalApprovalRequirement::NotRequired, &policy);
    let base = admission(CanonicalApprovalRequirement::NotRequired);
    let mut mutations: Vec<(&str, CanonicalRetryAdmission)> = Vec::new();
    macro_rules! mutation {
        ($name:literal, $change:expr) => {{
            let mut value = base.clone();
            $change(&mut value);
            mutations.push(($name, value));
        }};
    }
    mutation!("admission_id", |v: &mut CanonicalRetryAdmission| v
        .admission_id =
        "admission-x".to_owned());
    mutation!("step_execution_id", |v: &mut CanonicalRetryAdmission| v
        .step_execution_id =
        "step-execution-x".to_owned());
    mutation!("policy_id", |v: &mut CanonicalRetryAdmission| v.policy_id =
        "policy-x".to_owned());
    mutation!("decision_id", |v: &mut CanonicalRetryAdmission| v
        .decision_id =
        "decision-x".to_owned());
    mutation!("prior_attempt_id", |v: &mut CanonicalRetryAdmission| v
        .prior_attempt_id =
        "attempt-x".to_owned());
    mutation!(
        "prior_attempt_ordinal",
        |v: &mut CanonicalRetryAdmission| v.prior_attempt_ordinal += 1
    );
    mutation!("prior_call_id", |v: &mut CanonicalRetryAdmission| v
        .prior_call_id =
        "call-x".to_owned());
    mutation!("prior_tool_call_id", |v: &mut CanonicalRetryAdmission| v
        .prior_tool_call_id =
        ToolCallId::from_raw("tool-call-x"));
    mutation!("prior_grant_id", |v: &mut CanonicalRetryAdmission| v
        .prior_grant_id =
        GrantId::from_raw("grant-x"));
    mutation!("successor_attempt_id", |v: &mut CanonicalRetryAdmission| {
        v.successor_attempt_id = "attempt-x".to_owned()
    });
    mutation!(
        "successor_attempt_ordinal",
        |v: &mut CanonicalRetryAdmission| v.successor_attempt_ordinal += 1
    );
    mutation!("successor_call_id", |v: &mut CanonicalRetryAdmission| v
        .successor_call_id =
        "call-x".to_owned());
    mutation!(
        "successor_tool_call_id",
        |v: &mut CanonicalRetryAdmission| v.successor_tool_call_id =
            ToolCallId::from_raw("tool-call-x")
    );
    mutation!("successor_grant_id", |v: &mut CanonicalRetryAdmission| v
        .successor_grant_id =
        GrantId::from_raw("grant-x"));
    mutation!(
        "successor_approval_id",
        |v: &mut CanonicalRetryAdmission| v.successor_approval_id = Some("approval-x".to_owned())
    );
    let mut dispatch_probe = 0_u64;
    for (case, changed) in mutations {
        deny_before_dispatch(
            compile(
                changed,
                &policy,
                &recovery,
                &current_preflight,
                None,
                &current_grant,
                None,
                None,
            ),
            FreshAttemptAdmissionError::IntegrityMismatch,
            &mut dispatch_probe,
            case,
        );
    }
    assert_eq!(dispatch_probe, 0);
}

#[test]
fn racing_eligible_attempts_execute_once_and_uncertainty_is_sticky() {
    const RACERS: usize = 16;
    let policy = policy(CanonicalRetryClass::RecoverableRead);
    let recovery = decision();
    let current_preflight = preflight(&policy);
    let current_grant = grant(CanonicalApprovalRequirement::NotRequired, &policy);
    let prior_identities = prior_ledger();
    let mut admitted = Vec::new();
    for _ in 0..RACERS {
        admitted.push(
            compile_execution_ready_attempt(FreshAttemptAdmissionInput {
                candidate: admission(CanonicalApprovalRequirement::NotRequired),
                policy: &policy,
                decision: &recovery,
                preflight: &current_preflight,
                reconciliation: None,
                successor_grant: FreshSingleUseGrant::new(&current_grant),
                successor_approval: None,
                prior_approval_id: None,
                prior_identities: &prior_identities,
                successor_idempotency_key_sha256: None,
                presented_successor_receipt_id: None,
                now_epoch_ms: 1_500,
            })
            .expect("each racer independently satisfies pure admission"),
        );
    }

    let gate = SynchronizedAttemptExecutionGate::new();
    let barrier = Arc::new(Barrier::new(RACERS));
    let effect_probe = Arc::new(AtomicUsize::new(0));
    let handles: Vec<_> = admitted
        .into_iter()
        .map(|permit| {
            let gate = gate.clone();
            let barrier = barrier.clone();
            let effect_probe = effect_probe.clone();
            thread::spawn(move || {
                barrier.wait();
                gate.execute(permit, || {
                    effect_probe.fetch_add(1, Ordering::SeqCst);
                    AttemptEffectOutcome::Uncertain
                })
            })
        })
        .collect();
    let results: Vec<_> = handles
        .into_iter()
        .map(|handle| handle.join().expect("racer joins"))
        .collect();
    assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
    assert_eq!(
        results
            .iter()
            .filter(|result| {
                **result == Err(AttemptExecutionGateError::PriorAttemptAlreadyEntered)
            })
            .count(),
        RACERS - 1,
    );
    assert_eq!(effect_probe.load(Ordering::SeqCst), 1);
    let receipt = results
        .iter()
        .find_map(|result| result.as_ref().ok())
        .expect("one receipt");
    assert_eq!(receipt.successor_attempt_id, "attempt-2");
    assert_eq!(receipt.outcome, AttemptEffectOutcome::Uncertain);
    assert_eq!(
        gate.outcome("step-execution-1", "attempt-1"),
        Ok(Some(AttemptEffectOutcome::Uncertain))
    );

    let replay = compile_execution_ready_attempt(FreshAttemptAdmissionInput {
        candidate: admission(CanonicalApprovalRequirement::NotRequired),
        policy: &policy,
        decision: &recovery,
        preflight: &current_preflight,
        reconciliation: None,
        successor_grant: FreshSingleUseGrant::new(&current_grant),
        successor_approval: None,
        prior_approval_id: None,
        prior_identities: &prior_identities,
        successor_idempotency_key_sha256: None,
        presented_successor_receipt_id: None,
        now_epoch_ms: 1_500,
    })
    .expect("pure admission does not mutate execution state");
    assert_eq!(
        gate.execute(replay, || {
            effect_probe.fetch_add(1, Ordering::SeqCst);
            AttemptEffectOutcome::Succeeded
        }),
        Err(AttemptExecutionGateError::PriorAttemptAlreadyEntered)
    );
    assert_eq!(effect_probe.load(Ordering::SeqCst), 1);
    assert_eq!(
        gate.outcome("step-execution-1", "attempt-1"),
        Ok(Some(AttemptEffectOutcome::Uncertain))
    );
}
