use agentmage_kernel_contracts::{
    ActionId, ActionKind, ActorId, ApprovalId, ApprovalRequest, CanonicalApprovalRequirement,
    CanonicalDiagnosticDisclosure, CanonicalEffectClass, CanonicalExecutionAuthority,
    CanonicalExecutionBudgets, CanonicalFailureClass, CanonicalIdempotencyRequirement,
    CanonicalRecoveryAction, CanonicalRecoveryDecision, CanonicalRetryAdmission,
    CanonicalRetryClass, CanonicalStepExecutionPolicy, CanonicalVerificationRequirement,
    CapabilityGrant, ContractPayload, CorrelationId, DataSensitivity, GrantClass, GrantId,
    GrantNonce, GrantOperation, GrantStatus, OperationBinding, PlanId, PlanStepId, SchemaId,
    SchemaReference, SessionId, TaskId, ToolCall, ToolCallId, ToolId,
};
use agentmage_kernel_engine::retry_admission::{
    CurrentAttemptApproval, CurrentEffectReconciliation, CurrentPreflightEvidence,
    EffectReconciliationDisposition, FreshAttemptAdmissionError, FreshAttemptAdmissionInput,
    FreshSingleUseGrant, compile_fresh_attempt_admission,
};

const POLICY_SHA256: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const OTHER_SHA256: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

fn policy(retry_class: CanonicalRetryClass) -> CanonicalStepExecutionPolicy {
    CanonicalStepExecutionPolicy {
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
        policy_sha256: POLICY_SHA256.to_owned(),
    }
}

fn decision() -> CanonicalRecoveryDecision {
    CanonicalRecoveryDecision {
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
        decision_sha256: OTHER_SHA256.to_owned(),
    }
}

fn admission(required: CanonicalApprovalRequirement) -> CanonicalRetryAdmission {
    CanonicalRetryAdmission {
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
        admission_sha256: OTHER_SHA256.to_owned(),
    }
}

fn grant(required: CanonicalApprovalRequirement) -> CapabilityGrant {
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
        policy_sha256: POLICY_SHA256.to_owned(),
        status: GrantStatus::Issued,
    }
}

fn approval() -> ApprovalRequest {
    ApprovalRequest {
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
        policy_sha256: POLICY_SHA256.to_owned(),
        confirmation_sha256: OTHER_SHA256.to_owned(),
    }
}

fn preflight() -> CurrentPreflightEvidence {
    CurrentPreflightEvidence::new(
        "step-execution-1".to_owned(),
        POLICY_SHA256.to_owned(),
        vec!["workspace-current".to_owned()],
        1_000,
        2_000,
    )
    .expect("current preflight")
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
    compile_fresh_attempt_admission(FreshAttemptAdmissionInput {
        candidate,
        policy,
        decision,
        preflight,
        reconciliation,
        successor_grant: FreshSingleUseGrant::new(grant),
        successor_approval: approval.map(CurrentAttemptApproval::new),
        prior_approval_id,
        now_epoch_ms: 1_500,
    })
}

#[test]
fn fresh_attempt_requires_current_preflight_remaining_budget_and_single_use_grant() {
    let policy = policy(CanonicalRetryClass::RecoverableRead);
    let decision = decision();
    let preflight = preflight();
    let grant = grant(CanonicalApprovalRequirement::NotRequired);
    let result = compile(
        admission(CanonicalApprovalRequirement::NotRequired),
        &policy,
        &decision,
        &preflight,
        None,
        &grant,
        None,
        None,
    )
    .expect("all fresh prerequisites must admit one successor");
    assert_eq!(result.successor_attempt_id, "attempt-2");

    let stale_preflight = CurrentPreflightEvidence::new(
        "step-execution-1".to_owned(),
        POLICY_SHA256.to_owned(),
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
            &grant,
            None,
            None,
        ),
        Err(FreshAttemptAdmissionError::PreflightNotCurrent),
    );

    let mut exhausted = policy.clone();
    exhausted.budgets.attempts = 1;
    assert_eq!(
        compile(
            admission(CanonicalApprovalRequirement::NotRequired),
            &exhausted,
            &decision,
            &preflight,
            None,
            &grant,
            None,
            None,
        ),
        Err(FreshAttemptAdmissionError::AttemptBudgetExhausted),
    );

    let mut used = grant.clone();
    used.use_count = 1;
    assert_eq!(
        compile(
            admission(CanonicalApprovalRequirement::NotRequired),
            &policy,
            &decision,
            &preflight,
            None,
            &used,
            None,
            None,
        ),
        Err(FreshAttemptAdmissionError::GrantNotFreshSingleUse),
    );
    let mut reusable = grant.clone();
    reusable.use_limit = 2;
    assert_eq!(
        compile(
            admission(CanonicalApprovalRequirement::NotRequired),
            &policy,
            &decision,
            &preflight,
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
    let decision = decision();
    let preflight = preflight();
    let grant = grant(CanonicalApprovalRequirement::NotRequired);
    let mut candidate = admission(CanonicalApprovalRequirement::NotRequired);
    candidate.reconciliation_required = true;
    candidate.reconciled = true;
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
        POLICY_SHA256.to_owned(),
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
    let decision = decision();
    let preflight = preflight();
    let grant = grant(CanonicalApprovalRequirement::RequiredPerAttempt);
    let approval = approval();
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
