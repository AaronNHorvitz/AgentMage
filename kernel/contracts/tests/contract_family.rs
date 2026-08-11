use agentmage_kernel_contracts::{
    Action, ActionId, ActionKind, ActionState, ActorId, ApprovalId, ApprovalRequest,
    AuthorityClass, AuthorityTransactionId, BoundaryFailure, BoundaryKind, BoundaryOutcomeKind,
    BudgetLimit, BudgetResource, CONTRACT_SCHEMA_VERSION, CancellationId, CancellationReason,
    CancellationSignal, CapabilityGrant, ContractError, ContractPayload, CorrelationId,
    DataSensitivity, ErrorCategory, ErrorId, EvidenceId, EvidenceKind, EvidenceReference,
    GrantClass, GrantId, GrantNonce, GrantOperation, GrantPreimage, GrantSideEffect, GrantStatus,
    GrantTarget, OperationAttemptId, OperationBinding, OperationOutcome, Plan, PlanId, PlanState,
    PlanStep, PlanStepId, PlanStepState, Prompt, PromptId, PromptMessage, PromptRole, Receipt,
    ReceiptId, RequiredGrantTemplate, RetryDisposition, RollbackPlan, SchemaId, SchemaReference,
    SessionId, StateChange, StopCondition, StopConditionKind, Task, TaskId, TaskStatus, ToolCall,
    ToolCallId, ToolDefinition, ToolId, ToolResult, ToolRiskLevel, ValidationIssue,
    ValidationSeverity, VersionedContract, WorkPacket, WorkPacketId, WorkPacketState, WorkspaceId,
    WorkspacePath, WorkspacePathErrorKind, from_json, to_canonical_json,
};
use std::fmt::Debug;

fn assert_round_trip<T>(value: &T)
where
    T: VersionedContract + Debug + PartialEq,
{
    let first = to_canonical_json(value).expect("fixture must serialize");
    let second = to_canonical_json(value).expect("fixture must serialize deterministically");
    assert_eq!(first, second);
    assert_eq!(&from_json::<T>(&first).expect("fixture must parse"), value);
}

fn assert_embedded_grant_field_is_rejected<T>(value: &T)
where
    T: VersionedContract + Debug,
{
    let mut candidate = serde_json::to_value(value).expect("fixture must become JSON");
    candidate
        .as_object_mut()
        .expect("top-level contract must be an object")
        .insert(
            "capability_grant".to_owned(),
            serde_json::json!({"claimed": true}),
        );
    let bytes = serde_json::to_vec(&candidate).expect("mutated fixture must encode");
    let error = from_json::<T>(&bytes).expect_err("unknown authority field must fail closed");
    assert_eq!(error.code, "contract.field.unknown");
}

fn assert_optional_keys_are_required<T>(value: &T, keys: &[&str])
where
    T: VersionedContract + Debug,
{
    for key in keys {
        let mut candidate = serde_json::to_value(value).expect("fixture must become JSON");
        candidate
            .as_object_mut()
            .expect("top-level contract must be an object")
            .remove(*key)
            .expect("fixture key must exist");
        let bytes = serde_json::to_vec(&candidate).expect("mutated fixture must encode");
        let error = from_json::<T>(&bytes).expect_err("omitted optional key must fail closed");
        assert_eq!(error.code, "contract.field.missing", "missing key: {key}");
    }
}

fn fixture_entry<T>(name: &str, value: &T) -> serde_json::Value
where
    T: VersionedContract,
{
    let bytes = to_canonical_json(value).expect("fixture must serialize");
    serde_json::json!({
        "canonical_json": String::from_utf8(bytes).expect("contract JSON must be UTF-8"),
        "name": name,
    })
}

#[test]
fn workspace_path_is_canonical_and_cannot_deserialize_ambient_authority() {
    let path = WorkspacePath::new(
        WorkspaceId::from_raw("workspace-0001"),
        ["fixtures", "input.txt"],
    )
    .expect("canonical workspace path");
    let first = serde_json::to_vec(&path).expect("workspace path must serialize");
    let second = serde_json::to_vec(&path).expect("workspace path must serialize again");
    assert_eq!(first, second);
    assert_eq!(
        serde_json::from_slice::<WorkspacePath>(&first).expect("workspace path must parse"),
        path
    );

    let traversal = WorkspacePath::new(WorkspaceId::from_raw("workspace-0001"), [".."])
        .expect_err("parent traversal must fail");
    assert_eq!(
        traversal.kind(),
        WorkspacePathErrorKind::ParentTraversalComponent
    );

    let injected =
        br#"{"workspace_id":"workspace-0001","components":["input.txt"],"absolute_root":"/tmp"}"#;
    assert!(serde_json::from_slice::<WorkspacePath>(injected).is_err());
}

#[test]
fn complete_contract_family_preserves_linked_identities() {
    let session_id = SessionId::from_raw("session-0001");
    let task_id = TaskId::from_raw("task-0001");
    let packet_id = WorkPacketId::from_raw("packet-0001");
    let plan_id = PlanId::from_raw("plan-0001");
    let step_id = PlanStepId::from_raw("step-0001");
    let action_id = ActionId::from_raw("action-0001");
    let tool_id = ToolId::from_raw("fixture.read");
    let call_id = ToolCallId::from_raw("call-0001");
    let correlation_id = CorrelationId::from_raw("correlation-0001");
    let evidence_id = EvidenceId::from_raw("evidence-0001");
    let grant_id = GrantId::from_raw("grant-0001");

    let validation_issue = ValidationIssue {
        code: "fixture.warning".to_owned(),
        severity: ValidationSeverity::Warning,
        field_path: vec!["objective".to_owned()],
        message: "Synthetic warning for contract construction".to_owned(),
    };
    let task = Task {
        schema_version: CONTRACT_SCHEMA_VERSION,
        task_id: task_id.clone(),
        session_id: session_id.clone(),
        objective: "Inspect one synthetic fixture".to_owned(),
        acceptance_criteria: vec!["Record bounded evidence".to_owned()],
        constraints: vec!["Read only".to_owned()],
        status: TaskStatus::Ready,
    };
    let packet = WorkPacket {
        schema_version: CONTRACT_SCHEMA_VERSION,
        work_packet_id: packet_id,
        task_id: task_id.clone(),
        revision: 1,
        objective: task.objective.clone(),
        reason: "Exercise the complete contract family".to_owned(),
        owner: "fixture-user".to_owned(),
        authoritative_evidence: Vec::new(),
        mutable_files: Vec::new(),
        protected_files: vec!["fixtures/input.txt".to_owned()],
        expected_output: "One bounded observation".to_owned(),
        acceptance_checks: task.acceptance_criteria.clone(),
        required_evidence: vec![EvidenceKind::Observation],
        required_capability_class: AuthorityClass::Observe,
        budgets: vec![BudgetLimit {
            resource: BudgetResource::ToolCalls,
            limit: 1,
        }],
        stop_conditions: vec![
            StopCondition {
                kind: StopConditionKind::AcceptanceSatisfied,
                description: "Stop after the observation is evidenced".to_owned(),
            },
            StopCondition {
                kind: StopConditionKind::UserDecisionRequired,
                description: "Stop for a new user decision".to_owned(),
            },
            StopCondition {
                kind: StopConditionKind::PolicyDenied,
                description: "Stop on policy denial".to_owned(),
            },
            StopCondition {
                kind: StopConditionKind::Error,
                description: "Stop on a typed error".to_owned(),
            },
            StopCondition {
                kind: StopConditionKind::Cancelled,
                description: "Stop on cancellation".to_owned(),
            },
            StopCondition {
                kind: StopConditionKind::BudgetExhausted,
                description: "Stop on budget exhaustion".to_owned(),
            },
            StopCondition {
                kind: StopConditionKind::UncertainResult,
                description: "Stop on an uncertain result".to_owned(),
            },
        ],
        rollback: RollbackPlan {
            reversible: true,
            description: "No state change is permitted".to_owned(),
        },
        sensitivity: DataSensitivity::Ephemeral,
        last_verification_date: "2026-08-10".to_owned(),
        next_action: Some("Execute the proposed read".to_owned()),
        next_review: None,
        status_reason: None,
        disposition: None,
        completion_evidence: Vec::new(),
        superseding_work: None,
        validation_issues: Vec::new(),
        plan_id: Some(plan_id.clone()),
        state: WorkPacketState::Planned,
    };
    let plan = Plan {
        schema_version: CONTRACT_SCHEMA_VERSION,
        plan_id: plan_id.clone(),
        task_id: task_id.clone(),
        work_packet_revision: packet.revision,
        revision: 1,
        steps: vec![PlanStep {
            plan_step_id: step_id.clone(),
            ordinal: 0,
            description: "Read the fixture".to_owned(),
            depends_on: Vec::new(),
            expected_evidence: vec![EvidenceKind::Observation],
            state: PlanStepState::Ready,
        }],
        state: PlanState::Current,
    };
    let action = Action {
        schema_version: CONTRACT_SCHEMA_VERSION,
        action_id: action_id.clone(),
        task_id: task_id.clone(),
        plan_step_id: Some(step_id),
        kind: ActionKind::DeterministicTool,
        description: "Request one bounded fixture read".to_owned(),
        expected_effects: vec!["read-only-observation".to_owned()],
        state: ActionState::Ready,
    };
    let schema = SchemaReference {
        schema_id: SchemaId::from_raw("fixture.schema"),
        schema_version: 1,
        schema_sha256: "1".repeat(64),
    };
    let tool = ToolDefinition {
        schema_version: CONTRACT_SCHEMA_VERSION,
        tool_id: tool_id.clone(),
        tool_version: "1.0.0".to_owned(),
        display_name: "Fixture Reader".to_owned(),
        description: "Reads one synthetic fixture".to_owned(),
        input_schema: schema.clone(),
        output_schema: schema,
        risk_level: ToolRiskLevel::Low,
        declared_effects: vec![OperationBinding::new(GrantOperation::WorkspaceRead)],
        required_grant: RequiredGrantTemplate {
            operation: OperationBinding::new(GrantOperation::WorkspaceRead),
            target_scope: "workspace-file".to_owned(),
            single_use: true,
        },
        timeout_ms: 1_000,
    };
    let call = ToolCall {
        schema_version: CONTRACT_SCHEMA_VERSION,
        tool_call_id: call_id.clone(),
        correlation_id: correlation_id.clone(),
        action_id: action_id.clone(),
        tool_id: tool_id.clone(),
        tool_version: tool.tool_version.clone(),
        arguments: ContractPayload {
            schema: tool.input_schema.clone(),
            media_type: "application/json".to_owned(),
            bytes: b"{}".to_vec(),
            sha256: "2".repeat(64),
        },
    };
    let grant = CapabilityGrant {
        schema_version: CONTRACT_SCHEMA_VERSION,
        grant_id,
        revision: 1,
        grant_class: GrantClass::Operation,
        actor_id: ActorId::from_raw("actor-local-0001"),
        approval_id: Some(ApprovalId::from_raw("approval-0001")),
        session_id: session_id.clone(),
        task_id: task_id.clone(),
        action_id: Some(action_id.clone()),
        action_kind: Some(ActionKind::DeterministicTool),
        operation: OperationBinding::new(GrantOperation::WorkspaceRead),
        tool_id: Some(tool_id.clone()),
        tool_version: Some(tool.tool_version.clone()),
        targets: vec![GrantTarget {
            workspace_id: WorkspaceId::from_raw("workspace-0001"),
            path_components: vec!["fixtures".to_owned(), "input.txt".to_owned()],
        }],
        excluded_targets: vec![GrantTarget {
            workspace_id: WorkspaceId::from_raw("workspace-0001"),
            path_components: vec!["fixtures".to_owned(), "private".to_owned()],
        }],
        sensitivity: DataSensitivity::Ephemeral,
        argument_sha256: call.arguments.sha256.clone(),
        preimages: vec![GrantPreimage {
            target_index: 0,
            content_sha256: "7".repeat(64),
            observed_revision: Some("fixture-v1".to_owned()),
        }],
        expected_side_effects: vec![GrantSideEffect {
            operation: OperationBinding::new(GrantOperation::WorkspaceRead),
            target_indexes: vec![0],
            details_sha256: "8".repeat(64),
        }],
        rollback_description: "No state change is permitted".to_owned(),
        issued_at_epoch_ms: 1_786_320_000_000,
        expires_at_epoch_ms: 1_786_320_060_000,
        nonce: GrantNonce::from_raw("nonce-0001"),
        use_limit: 1,
        use_count: 0,
        parent_grant_id: Some(GrantId::from_raw("grant-parent-0001")),
        parent_grant_sha256: Some("9".repeat(64)),
        preview_sha256: "a".repeat(64),
        policy_sha256: "b".repeat(64),
        status: GrantStatus::Issued,
    };
    let approval = ApprovalRequest {
        schema_version: CONTRACT_SCHEMA_VERSION,
        approval_id: grant
            .approval_id
            .clone()
            .expect("fixture approval identity"),
        proposed_grant_id: grant.grant_id.clone(),
        parent_grant_id: grant
            .parent_grant_id
            .clone()
            .expect("fixture parent grant identity"),
        parent_grant_sha256: grant
            .parent_grant_sha256
            .clone()
            .expect("fixture parent grant digest"),
        actor_id: grant.actor_id.clone(),
        session_id: grant.session_id.clone(),
        task_id: grant.task_id.clone(),
        action_kind: grant.action_kind.expect("fixture action kind"),
        operation: grant.operation,
        tool_call: call.clone(),
        targets: grant.targets.clone(),
        excluded_targets: grant.excluded_targets.clone(),
        sensitivity: grant.sensitivity,
        preimages: grant.preimages.clone(),
        expected_side_effects: grant.expected_side_effects.clone(),
        rollback_description: grant.rollback_description.clone(),
        issued_at_epoch_ms: grant.issued_at_epoch_ms,
        expires_at_epoch_ms: grant.expires_at_epoch_ms,
        policy_sha256: grant.policy_sha256.clone(),
        confirmation_sha256: grant.preview_sha256.clone(),
    };
    let evidence = EvidenceReference {
        schema_version: CONTRACT_SCHEMA_VERSION,
        evidence_id,
        kind: EvidenceKind::Observation,
        source_id: "synthetic-corpus-v1".to_owned(),
        object_id: "case-0001".to_owned(),
        fragment: Some("record:1".to_owned()),
        content_sha256: "3".repeat(64),
        observed_revision: Some("fixture-v1".to_owned()),
    };
    let error = ContractError {
        schema_version: CONTRACT_SCHEMA_VERSION,
        error_id: ErrorId::from_raw("error-0001"),
        code: "fixture.denied".to_owned(),
        category: ErrorCategory::Policy,
        message: "Synthetic denial".to_owned(),
        field_path: Vec::new(),
        retry: RetryDisposition::AfterUserDecision,
        caused_by: None,
    };
    let result = ToolResult {
        schema_version: CONTRACT_SCHEMA_VERSION,
        tool_call_id: call_id.clone(),
        correlation_id: correlation_id.clone(),
        outcome: OperationOutcome::Denied,
        output: None,
        validation_issues: vec![validation_issue],
        evidence: vec![evidence.clone()],
        error: Some(error.clone()),
        elapsed_ms: 0,
        state_change: StateChange::NotChanged,
    };
    let receipt = Receipt {
        schema_version: CONTRACT_SCHEMA_VERSION,
        receipt_id: ReceiptId::from_raw("receipt-0001"),
        sequence: 1,
        correlation_id: correlation_id.clone(),
        authority_transaction_id: AuthorityTransactionId::from_raw("transaction-0001"),
        operation_attempt_id: OperationAttemptId::from_raw("attempt-0001"),
        approval_id: approval.approval_id.clone(),
        grant_id: grant.grant_id.clone(),
        session_id,
        task_id: task_id.clone(),
        action_id: action_id.clone(),
        tool_call_id: Some(call_id.clone()),
        operation: grant.operation,
        outcome: result.outcome,
        operation_sha256: "4".repeat(64),
        evidence: vec![evidence],
        error: Some(error),
        previous_receipt_sha256: "0".repeat(64),
        receipt_sha256: "5".repeat(64),
        occurred_at: "2026-08-10T00:00:00Z".to_owned(),
    };
    let cancellation = CancellationSignal {
        schema_version: CONTRACT_SCHEMA_VERSION,
        cancellation_id: CancellationId::from_raw("cancel-0001"),
        correlation_id: correlation_id.clone(),
        task_id: task_id.clone(),
        reason: CancellationReason::PolicyDenied,
        requested_by: BoundaryKind::Kernel,
    };
    let boundary_failure = BoundaryFailure {
        schema_version: CONTRACT_SCHEMA_VERSION,
        correlation_id: correlation_id.clone(),
        task_id: task_id.clone(),
        origin: BoundaryKind::Kernel,
        route: vec![BoundaryKind::Kernel],
        outcome: BoundaryOutcomeKind::Cancelled,
        error: ContractError {
            schema_version: CONTRACT_SCHEMA_VERSION,
            error_id: ErrorId::from_raw("error-cancelled"),
            code: "fixture.cancelled".to_owned(),
            category: ErrorCategory::Cancellation,
            message: "Synthetic cancellation".to_owned(),
            field_path: Vec::new(),
            retry: RetryDisposition::AfterUserDecision,
            caused_by: None,
        },
        cancellation: Some(cancellation.clone()),
    };
    let prompt = Prompt {
        schema_version: CONTRACT_SCHEMA_VERSION,
        prompt_id: PromptId::from_raw("prompt-0001"),
        correlation_id: correlation_id.clone(),
        task_id: task_id.clone(),
        purpose: "Describe one synthetic observation".to_owned(),
        messages: vec![PromptMessage {
            role: PromptRole::UserRequest,
            content: "Describe the fixture".to_owned(),
        }],
    };

    if std::env::var_os("AGENTMAGE_EMIT_CONTRACT_FIXTURES").as_deref()
        == Some(std::ffi::OsStr::new("1"))
    {
        let fixtures = vec![
            fixture_entry("action", &action),
            fixture_entry("approval_request", &approval),
            fixture_entry("boundary_failure", &boundary_failure),
            fixture_entry("capability_grant", &grant),
            fixture_entry("cancellation_signal", &cancellation),
            fixture_entry(
                "contract_error",
                result.error.as_ref().expect("fixture error"),
            ),
            fixture_entry("evidence_reference", &result.evidence[0]),
            fixture_entry("plan", &plan),
            fixture_entry("prompt", &prompt),
            fixture_entry("receipt", &receipt),
            fixture_entry("task", &task),
            fixture_entry("tool_call", &call),
            fixture_entry("tool_definition", &tool),
            fixture_entry("tool_result", &result),
            fixture_entry("work_packet", &packet),
        ];
        println!(
            "AGENTMAGE_CONTRACT_FIXTURES={}",
            serde_json::to_string(&fixtures).expect("fixture bundle must serialize")
        );
    }

    assert_round_trip(&task);
    assert_round_trip(&packet);
    assert_round_trip(&plan);
    assert_round_trip(&action);
    assert_round_trip(&tool);
    assert_round_trip(&call);
    assert_round_trip(&result);
    assert_round_trip(&result.evidence[0]);
    assert_round_trip(result.error.as_ref().expect("fixture error"));
    assert_round_trip(&receipt);
    assert_round_trip(&cancellation);
    assert_round_trip(&boundary_failure);
    assert_round_trip(&prompt);
    assert_round_trip(&grant);
    assert_round_trip(&approval);
    assert_embedded_grant_field_is_rejected(&plan);
    assert_embedded_grant_field_is_rejected(&prompt);
    assert_embedded_grant_field_is_rejected(&tool);
    assert_embedded_grant_field_is_rejected(&approval);
    assert_optional_keys_are_required(
        &packet,
        &[
            "next_action",
            "next_review",
            "status_reason",
            "disposition",
            "superseding_work",
            "plan_id",
        ],
    );
    assert_optional_keys_are_required(&action, &["plan_step_id"]);
    assert_optional_keys_are_required(&result, &["output", "error"]);
    assert_optional_keys_are_required(&result.evidence[0], &["fragment", "observed_revision"]);
    assert_optional_keys_are_required(
        result.error.as_ref().expect("fixture error"),
        &["caused_by"],
    );
    assert_optional_keys_are_required(&receipt, &["tool_call_id", "error"]);
    assert_optional_keys_are_required(&boundary_failure, &["cancellation"]);
    assert_optional_keys_are_required(
        &grant,
        &[
            "action_id",
            "action_kind",
            "tool_id",
            "tool_version",
            "parent_grant_id",
            "parent_grant_sha256",
        ],
    );

    assert_eq!(packet.task_id, task.task_id);
    assert_eq!(plan.plan_id, packet.plan_id.expect("fixture plan identity"));
    assert_eq!(action.task_id, task_id);
    assert_eq!(call.action_id, action_id);
    assert_eq!(call.tool_id, tool_id);
    assert_eq!(result.tool_call_id, call_id);
    assert_eq!(receipt.correlation_id, correlation_id);
    assert_eq!(receipt.tool_call_id, Some(result.tool_call_id));
}
