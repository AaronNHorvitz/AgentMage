use agentmage_kernel_contracts::{
    Action, ActionId, ActionKind, ActionState, BudgetLimit, BudgetResource,
    CONTRACT_SCHEMA_VERSION, ContractError, ContractPayload, CorrelationId, DataSensitivity,
    ErrorCategory, ErrorId, EvidenceId, EvidenceKind, EvidenceReference, OperationOutcome, Plan,
    PlanId, PlanState, PlanStep, PlanStepId, PlanStepState, Receipt, ReceiptId, RetryDisposition,
    RollbackPlan, SchemaId, SchemaReference, SessionId, StopCondition, StopConditionKind, Task,
    TaskId, TaskStatus, ToolCall, ToolCallId, ToolDefinition, ToolId, ToolResult, ValidationIssue,
    ValidationSeverity, VersionedContract, WorkPacket, WorkPacketId, WorkPacketState, from_json,
    to_canonical_json,
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
        required_capability_class: "read-only".to_owned(),
        budgets: vec![BudgetLimit {
            resource: BudgetResource::ToolCalls,
            limit: 1,
        }],
        stop_conditions: vec![StopCondition {
            kind: StopConditionKind::AcceptanceSatisfied,
            description: "Stop after the observation is evidenced".to_owned(),
        }],
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
        declared_effects: vec!["read-only-observation".to_owned()],
    };
    let call = ToolCall {
        schema_version: CONTRACT_SCHEMA_VERSION,
        tool_call_id: call_id.clone(),
        correlation_id: correlation_id.clone(),
        action_id: action_id.clone(),
        tool_id: tool_id.clone(),
        tool_version: tool.tool_version.clone(),
        arguments: ContractPayload {
            media_type: "application/json".to_owned(),
            bytes: b"{}".to_vec(),
            sha256: "2".repeat(64),
        },
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
    };
    let receipt = Receipt {
        schema_version: CONTRACT_SCHEMA_VERSION,
        receipt_id: ReceiptId::from_raw("receipt-0001"),
        sequence: 1,
        correlation_id: correlation_id.clone(),
        session_id,
        task_id: task_id.clone(),
        action_id: action_id.clone(),
        tool_call_id: Some(call_id.clone()),
        outcome: result.outcome,
        operation_sha256: "4".repeat(64),
        evidence: vec![evidence],
        error: Some(error),
        previous_receipt_sha256: "0".repeat(64),
        receipt_sha256: "5".repeat(64),
        occurred_at: "2026-08-10T00:00:00Z".to_owned(),
    };

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

    assert_eq!(packet.task_id, task.task_id);
    assert_eq!(plan.plan_id, packet.plan_id.expect("fixture plan identity"));
    assert_eq!(action.task_id, task_id);
    assert_eq!(call.action_id, action_id);
    assert_eq!(call.tool_id, tool_id);
    assert_eq!(result.tool_call_id, call_id);
    assert_eq!(receipt.correlation_id, correlation_id);
    assert_eq!(receipt.tool_call_id, Some(result.tool_call_id));
}
