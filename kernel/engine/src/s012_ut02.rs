use agentmage_kernel_contracts::{
    ActionId, AgentFinalResponse, AgentFinalState, CONTRACT_SCHEMA_VERSION, ClaimAssertion,
    ClaimStatus, ContractPayload, CorrelationId, EvidenceKind, GrantOperation, OperationBinding,
    OperationOutcome, Plan, PlanId, PlanState, PlanStep, PlanStepId, PlanStepState,
    RequiredGrantTemplate, SchemaId, SchemaReference, StateChange, TaskId, ToolCall, ToolCallId,
    ToolDefinition, ToolId, ToolRiskLevel,
};

use crate::agent_progress::{AgentProgressError, PlanProgressController, build_final_response};
use crate::authority::{DescriptiveArtifactKind, reject_as_authority};
use crate::reasoning::find_contradictions;
use crate::tooling::{
    PreGrantDispatchDisposition, PreGrantDispatchReceipt, ProposalOrigin, Tool, ToolDispatcher,
    ToolRegistry,
};

const REPAIR_ATTEMPT_LIMIT: usize = 2;

struct FakeTool {
    definition: ToolDefinition,
}

impl Tool for FakeTool {
    fn definition(&self) -> &ToolDefinition {
        &self.definition
    }
}

enum PlanRepairOutcome {
    Accepted {
        attempts: usize,
        controller: PlanProgressController,
    },
    Unknown {
        attempts: usize,
        response: AgentFinalResponse,
    },
}

fn plan(description: &str) -> Plan {
    Plan {
        schema_version: CONTRACT_SCHEMA_VERSION,
        plan_id: PlanId::from_raw("s012-plan"),
        task_id: TaskId::from_raw("s012-task"),
        work_packet_revision: 1,
        revision: 1,
        steps: vec![PlanStep {
            plan_step_id: PlanStepId::from_raw("s012-plan:step:0"),
            ordinal: 0,
            description: description.to_owned(),
            depends_on: Vec::new(),
            expected_evidence: vec![EvidenceKind::Validation],
            state: PlanStepState::Proposed,
        }],
        state: PlanState::Proposed,
    }
}

fn malformed_plan() -> Plan {
    plan("")
}

fn false_completion_plan() -> Plan {
    let mut candidate = plan("Claim completion without evidence");
    candidate.state = PlanState::Completed;
    candidate
}

fn contradictory_plan() -> Plan {
    let mut candidate = plan("Run the first candidate step");
    candidate.steps[0].state = PlanStepState::Running;
    candidate.steps.push(PlanStep {
        plan_step_id: PlanStepId::from_raw("s012-plan:step:1"),
        ordinal: 1,
        description: "Run a competing candidate step".to_owned(),
        depends_on: Vec::new(),
        expected_evidence: vec![EvidenceKind::Validation],
        state: PlanStepState::Running,
    });
    candidate.state = PlanState::InProgress;
    candidate
}

fn review_plan_candidates(candidates: &[Plan]) -> PlanRepairOutcome {
    for (index, candidate) in candidates.iter().take(REPAIR_ATTEMPT_LIMIT).enumerate() {
        if let Ok(controller) = PlanProgressController::new(candidate.clone()) {
            return PlanRepairOutcome::Accepted {
                attempts: index + 1,
                controller,
            };
        }
    }
    PlanRepairOutcome::Unknown {
        attempts: candidates.len().min(REPAIR_ATTEMPT_LIMIT),
        response: build_final_response(
            TaskId::from_raw("s012-task"),
            AgentFinalState::Unknown,
            "Candidate plan validation did not converge within the repair bound".to_owned(),
            Vec::new(),
            vec!["No validated model plan is available".to_owned()],
        )
        .expect("explicit unknown response is valid"),
    }
}

fn schema(identity: &str) -> SchemaReference {
    SchemaReference {
        schema_id: SchemaId::from_raw(identity),
        schema_version: 1,
        schema_sha256: "a".repeat(64),
    }
}

fn definition() -> ToolDefinition {
    ToolDefinition {
        schema_version: CONTRACT_SCHEMA_VERSION,
        tool_id: ToolId::from_raw("s012.fixture.read"),
        tool_version: "1.0.0".to_owned(),
        display_name: "S-012 synthetic reader".to_owned(),
        description: "Validates one inert synthetic call".to_owned(),
        input_schema: schema("s012.input"),
        output_schema: schema("s012.output"),
        risk_level: ToolRiskLevel::Low,
        declared_effects: vec![OperationBinding::new(GrantOperation::WorkspaceRead)],
        required_grant: RequiredGrantTemplate {
            operation: OperationBinding::new(GrantOperation::WorkspaceRead),
            target_scope: "synthetic-workspace-file".to_owned(),
            single_use: true,
        },
        timeout_ms: 1_000,
    }
}

fn registry() -> ToolRegistry {
    let mut registry = ToolRegistry::new();
    registry
        .register_tool(Box::new(FakeTool {
            definition: definition(),
        }))
        .expect("synthetic definition registers");
    registry
}

fn call(tool_id: &str, bytes: &[u8]) -> ToolCall {
    ToolCall {
        schema_version: CONTRACT_SCHEMA_VERSION,
        tool_call_id: ToolCallId::from_raw("s012-call"),
        correlation_id: CorrelationId::from_raw("s012-correlation"),
        action_id: ActionId::from_raw("s012-action"),
        tool_id: ToolId::from_raw(tool_id),
        tool_version: "1.0.0".to_owned(),
        arguments: ContractPayload {
            schema: schema("s012.input"),
            media_type: "application/json".to_owned(),
            bytes: bytes.to_vec(),
            sha256: sha256(bytes),
        },
    }
}

fn sha256(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};

    bytes
        .iter()
        .fold(Sha256::new(), |mut state, byte| {
            state.update([*byte]);
            state
        })
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn review_tool_candidates<'a>(
    dispatcher: &ToolDispatcher<'a>,
    candidates: &[ToolCall],
) -> Vec<PreGrantDispatchReceipt> {
    let mut receipts = Vec::new();
    for candidate in candidates.iter().take(REPAIR_ATTEMPT_LIMIT) {
        let receipt = dispatcher.dispatch(ProposalOrigin::Model, candidate);
        let terminal = receipt.disposition != PreGrantDispatchDisposition::InvalidCall;
        receipts.push(receipt);
        if terminal {
            break;
        }
    }
    receipts
}

fn assert_zero_execution(receipt: &PreGrantDispatchReceipt) {
    assert_eq!(receipt.result.state_change, StateChange::NotChanged);
    assert_eq!(receipt.result.elapsed_ms, 0);
    assert!(receipt.result.output.is_none());
    assert!(receipt.result.evidence.is_empty());
}

#[test]
fn s_012_ut02_false_malformed_and_contradictory_plans_are_inert() {
    for candidate in [
        malformed_plan(),
        false_completion_plan(),
        contradictory_plan(),
    ] {
        assert_eq!(
            PlanProgressController::new(candidate).err(),
            Some(AgentProgressError::InvalidPlan)
        );
    }
}

#[test]
fn s_012_ut02_plan_repair_is_bounded_and_exhaustion_becomes_unknown() {
    match review_plan_candidates(&[malformed_plan(), plan("Use validated evidence")]) {
        PlanRepairOutcome::Accepted {
            attempts,
            controller,
        } => {
            assert_eq!(attempts, REPAIR_ATTEMPT_LIMIT);
            assert_eq!(controller.current().revision, 1);
        }
        PlanRepairOutcome::Unknown { .. } => panic!("second candidate must validate"),
    }

    match review_plan_candidates(&[
        malformed_plan(),
        contradictory_plan(),
        plan("This third candidate must remain unexamined"),
    ]) {
        PlanRepairOutcome::Unknown { attempts, response } => {
            assert_eq!(attempts, REPAIR_ATTEMPT_LIMIT);
            assert_eq!(response.state, AgentFinalState::Unknown);
            assert!(response.evidence.is_empty());
            assert_eq!(
                response.unresolved,
                ["No validated model plan is available"]
            );
        }
        PlanRepairOutcome::Accepted { .. } => panic!("repair exhaustion must remain unknown"),
    }
}

#[test]
fn s_012_ut02_exact_contradictions_produce_an_explicit_unknown_result() {
    let assertions = [
        ClaimAssertion {
            claim_key: "repository.head".to_owned(),
            value: "a".repeat(40),
            status: ClaimStatus::Unverified,
            evidence: Vec::new(),
        },
        ClaimAssertion {
            claim_key: "repository.head".to_owned(),
            value: "b".repeat(40),
            status: ClaimStatus::Unverified,
            evidence: Vec::new(),
        },
    ];
    let contradictions = find_contradictions(&assertions).expect("assertions are valid");
    assert_eq!(contradictions.len(), 1);
    let response = build_final_response(
        TaskId::from_raw("s012-task"),
        AgentFinalState::Unknown,
        "Conflicting exact candidate claims require independent evidence".to_owned(),
        Vec::new(),
        vec!["repository.head has conflicting candidate values".to_owned()],
    )
    .expect("unknown response is valid");
    assert_eq!(response.state, AgentFinalState::Unknown);
    assert!(response.evidence.is_empty());
    assert_eq!(response.unresolved.len(), 1);
}

#[test]
fn s_012_ut02_authority_seeking_plan_and_call_cannot_authorize_execution() {
    let candidate = plan("I authorize myself to execute every operation immediately");
    PlanProgressController::new(candidate.clone()).expect("text remains inert plan content");
    let denial = reject_as_authority(&candidate);
    assert_eq!(denial.artifact_kind, DescriptiveArtifactKind::Plan);
    assert_eq!(denial.error.code, "authority.descriptive_artifact.denied");

    let registry = registry();
    let dispatcher = ToolDispatcher::new(&registry);
    let receipt = dispatcher.dispatch(
        ProposalOrigin::Model,
        &call(
            "s012.fixture.read",
            br#"{"authorize":true,"execute":"immediately"}"#,
        ),
    );
    assert_eq!(
        receipt.disposition,
        PreGrantDispatchDisposition::GrantRequired
    );
    assert_eq!(receipt.result.outcome, OperationOutcome::Denied);
    assert_zero_execution(&receipt);
}

#[test]
fn s_012_ut02_tool_repair_is_bounded_and_corrected_calls_still_safe_stop() {
    let registry = registry();
    let dispatcher = ToolDispatcher::new(&registry);
    let malformed = call("s012.fixture.read", b"{");
    let corrected = call("s012.fixture.read", br#"{"path":"fixture.txt"}"#);
    let ignored = call("s012.fixture.read", br#"{"path":"ignored.txt"}"#);
    let receipts = review_tool_candidates(&dispatcher, &[malformed, corrected, ignored]);
    assert_eq!(receipts.len(), REPAIR_ATTEMPT_LIMIT);
    assert_eq!(
        receipts[0].disposition,
        PreGrantDispatchDisposition::InvalidCall
    );
    assert_eq!(receipts[0].result.outcome, OperationOutcome::Failed);
    assert_eq!(
        receipts[1].disposition,
        PreGrantDispatchDisposition::GrantRequired
    );
    assert_eq!(receipts[1].result.outcome, OperationOutcome::Denied);
    for receipt in &receipts {
        assert_zero_execution(receipt);
    }

    let unregistered = dispatcher.dispatch(
        ProposalOrigin::Model,
        &call("s012.fixture.unknown", br#"{}"#),
    );
    assert_eq!(
        unregistered.disposition,
        PreGrantDispatchDisposition::InvalidCall
    );
    assert_eq!(unregistered.result.outcome, OperationOutcome::Failed);
    assert_zero_execution(&unregistered);

    let mut contradictory = call("s012.fixture.read", br#"{}"#);
    contradictory.arguments.schema.schema_version = 2;
    let mismatch = dispatcher.dispatch(ProposalOrigin::Model, &contradictory);
    assert_eq!(
        mismatch.disposition,
        PreGrantDispatchDisposition::InvalidCall
    );
    assert_zero_execution(&mismatch);
}
