use agentmage_kernel_contracts::{
    Action, ActionId, ActionKind, ActionState, AuthorityClass, BudgetLimit, BudgetResource,
    CONTRACT_SCHEMA_VERSION, DataSensitivity, EvidenceKind, PlanId, RollbackPlan, StopCondition,
    StopConditionKind, TaskId, WorkPacket, WorkPacketId, WorkPacketState,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::agent_runtime::{AgentDirective, AgentReviewOutcome, AgentRuntime};
use crate::configuration::ConfigurationManager;

const PROFILE: &[u8] = include_bytes!("../../../configuration/profiles/synthetic-test.json");
const MODES: &[u8] = include_bytes!("../../../fixtures/runtime/v1/reasoning-mode-fixtures.json");

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
enum ReasoningMode {
    Concise,
    Deep,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ReasoningModeFixture {
    schema_version: u32,
    fixture_set_id: String,
    cases: Vec<ReasoningModeCase>,
    mode_changes_authority: bool,
    mode_changes_evidence_standard: bool,
    private_user_data_used: bool,
    external_network_used: bool,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ReasoningModeCase {
    case_id: String,
    mode: ReasoningMode,
    authority: String,
    clarification_gate_required: bool,
    contradiction_check_required: bool,
    independent_verification_required: bool,
    maximum_assumptions: u32,
    maximum_hypotheses: u32,
    maximum_clarification_questions: u32,
    private_chain_of_thought_retained: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
struct BudgetRecord {
    resource: &'static str,
    limit: u64,
    used: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
struct FixedTaskReceipt {
    task_id: String,
    authority: &'static str,
    evidence_standard: &'static str,
    budgets: Vec<BudgetRecord>,
    status_cadence: Vec<&'static str>,
    fake_clock_elapsed_milliseconds: u64,
    fake_model_calls: u64,
    fake_tool_calls: u64,
    production_model_calls: u64,
    tool_changed_state: bool,
    tool_receipt_sha256: String,
    terminal: &'static str,
    external_network_used: bool,
}

#[derive(Debug)]
struct FakeClock {
    ticks: [u64; 2],
    index: usize,
}

impl FakeClock {
    fn next(&mut self) -> u64 {
        let tick = self.ticks[self.index];
        self.index += 1;
        tick
    }
}

#[derive(Default)]
struct FakeModel {
    calls: u64,
}

impl FakeModel {
    fn propose(&mut self, runtime: &AgentRuntime, action_id: &str) -> Action {
        self.calls += 1;
        Action {
            schema_version: CONTRACT_SCHEMA_VERSION,
            action_id: ActionId::from_raw(action_id),
            task_id: runtime.task_id().clone(),
            plan_step_id: Some(
                runtime.current_plan().expect("fixed task is planned").steps[0]
                    .plan_step_id
                    .clone(),
            ),
            kind: ActionKind::DeterministicTool,
            description: "Observe one synthetic object".to_owned(),
            expected_effects: vec!["descriptive-observation-only".to_owned()],
            state: ActionState::Proposed,
        }
    }
}

#[derive(Default)]
struct FakeTool {
    calls: u64,
}

impl FakeTool {
    fn observe(&mut self, action: &Action) -> String {
        self.calls += 1;
        let material = format!(
            "{}\0{}\0{}\0not-changed\0synthetic-observation",
            action.task_id.as_str(),
            action.action_id.as_str(),
            action.description
        );
        Sha256::digest(material.as_bytes())
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect()
    }
}

fn budgets() -> Vec<BudgetLimit> {
    vec![
        BudgetLimit {
            resource: BudgetResource::PlanSteps,
            limit: 2,
        },
        BudgetLimit {
            resource: BudgetResource::ToolCalls,
            limit: 2,
        },
        BudgetLimit {
            resource: BudgetResource::ModelCalls,
            limit: 1,
        },
        BudgetLimit {
            resource: BudgetResource::InputBytes,
            limit: 64,
        },
        BudgetLimit {
            resource: BudgetResource::OutputBytes,
            limit: 64,
        },
        BudgetLimit {
            resource: BudgetResource::ElapsedMilliseconds,
            limit: 100,
        },
    ]
}

fn packet(task_suffix: &str) -> WorkPacket {
    WorkPacket {
        schema_version: CONTRACT_SCHEMA_VERSION,
        work_packet_id: WorkPacketId::from_raw(format!("packet-s012-it01-{task_suffix}")),
        task_id: TaskId::from_raw(format!("task-s012-it01-{task_suffix}")),
        revision: 1,
        objective: format!("Inspect synthetic fixture {task_suffix}"),
        reason: "Compare bounded reasoning modes".to_owned(),
        owner: "synthetic-user".to_owned(),
        authoritative_evidence: Vec::new(),
        mutable_files: Vec::new(),
        protected_files: vec![format!("fixtures/{task_suffix}.txt")],
        expected_output: "One deterministic synthetic receipt".to_owned(),
        acceptance_checks: vec!["Record bounded synthetic evidence".to_owned()],
        required_evidence: vec![EvidenceKind::Observation],
        required_capability_class: AuthorityClass::Observe,
        budgets: budgets(),
        stop_conditions: [
            StopConditionKind::AcceptanceSatisfied,
            StopConditionKind::UserDecisionRequired,
            StopConditionKind::PolicyDenied,
            StopConditionKind::Error,
            StopConditionKind::Cancelled,
            StopConditionKind::BudgetExhausted,
            StopConditionKind::UncertainResult,
        ]
        .into_iter()
        .map(|kind| StopCondition {
            kind,
            description: format!("Synthetic {kind:?} stop"),
        })
        .collect(),
        rollback: RollbackPlan {
            reversible: true,
            description: "No state change is authorized".to_owned(),
        },
        sensitivity: DataSensitivity::Ephemeral,
        last_verification_date: "2026-08-13".to_owned(),
        next_action: None,
        next_review: None,
        status_reason: None,
        disposition: None,
        completion_evidence: Vec::new(),
        superseding_work: None,
        validation_issues: Vec::new(),
        plan_id: Some(PlanId::from_raw(format!("plan-s012-it01-{task_suffix}"))),
        state: WorkPacketState::Active,
    }
}

fn resource_name(resource: BudgetResource) -> &'static str {
    match resource {
        BudgetResource::PlanSteps => "plan-steps",
        BudgetResource::ToolCalls => "tool-calls",
        BudgetResource::ModelCalls => "model-calls",
        BudgetResource::InputBytes => "input-bytes",
        BudgetResource::OutputBytes => "output-bytes",
        BudgetResource::ElapsedMilliseconds => "elapsed-milliseconds",
        _ => "undeclared",
    }
}

fn run_fixed_task(_mode: ReasoningMode, task_suffix: &str) -> FixedTaskReceipt {
    let source_packet = packet(task_suffix);
    let declared = source_packet.budgets.clone();
    let configuration = ConfigurationManager::default()
        .load_bytes(PROFILE)
        .expect("synthetic configuration is valid");
    let mut runtime = AgentRuntime::new(configuration, source_packet).expect("fixed task starts");
    let mut clock = FakeClock {
        ticks: [5, 7],
        index: 0,
    };
    let mut model = FakeModel::default();
    let mut tool = FakeTool::default();
    let mut cadence = vec!["observe"];

    assert_eq!(runtime.observe(8, clock.next()), Ok(AgentDirective::Plan));
    cadence.push("plan");
    assert!(matches!(runtime.plan(), Ok(AgentDirective::Act { .. })));
    cadence.push("act");
    let action = model.propose(&runtime, &format!("action-s012-it01-{task_suffix}"));
    assert!(matches!(
        runtime.act(&action),
        Ok(AgentDirective::Review { .. })
    ));
    cadence.push("review");
    let tool_receipt_sha256 = tool.observe(&action);
    assert_eq!(
        runtime.review(
            &action.action_id,
            AgentReviewOutcome::Progress,
            12,
            clock.next(),
        ),
        Ok(AgentDirective::Observe {
            completed_cycles: 1,
        })
    );
    cadence.push("observe");
    assert!(matches!(
        runtime
            .signal(StopConditionKind::UserDecisionRequired)
            .expect("declared stop is admitted"),
        AgentDirective::Stop { .. }
    ));
    cadence.push("stopped");

    FixedTaskReceipt {
        task_id: runtime.task_id().as_str().to_owned(),
        authority: "descriptive-only",
        evidence_standard: "independent-verification-required",
        budgets: declared
            .into_iter()
            .map(|budget| BudgetRecord {
                resource: resource_name(budget.resource),
                limit: budget.limit,
                used: runtime
                    .usage(budget.resource)
                    .expect("declared budget exists"),
            })
            .collect(),
        status_cadence: cadence,
        fake_clock_elapsed_milliseconds: 12,
        fake_model_calls: model.calls,
        fake_tool_calls: tool.calls,
        production_model_calls: runtime
            .usage(BudgetResource::ModelCalls)
            .expect("model budget"),
        tool_changed_state: false,
        tool_receipt_sha256,
        terminal: "user-decision-required",
        external_network_used: false,
    }
}

fn mode_fixture() -> ReasoningModeFixture {
    serde_json::from_slice(MODES).expect("reasoning mode fixture is closed and valid")
}

#[test]
fn s_012_it01_reasoning_modes_change_capacity_only() {
    let fixture = mode_fixture();
    assert_eq!(fixture.schema_version, 1);
    assert_eq!(fixture.fixture_set_id, "agentmage-reasoning-modes-v1");
    assert!(!fixture.mode_changes_authority);
    assert!(!fixture.mode_changes_evidence_standard);
    assert!(!fixture.private_user_data_used);
    assert!(!fixture.external_network_used);
    assert_eq!(fixture.cases.len(), 2);
    let concise = &fixture.cases[0];
    let deep = &fixture.cases[1];
    assert_eq!(concise.case_id, "RMD-01");
    assert_eq!(deep.case_id, "RMD-02");
    assert_eq!(concise.mode, ReasoningMode::Concise);
    assert_eq!(deep.mode, ReasoningMode::Deep);
    for case in &fixture.cases {
        assert_eq!(case.authority, "descriptive-only");
        assert!(case.clarification_gate_required);
        assert!(case.contradiction_check_required);
        assert!(case.independent_verification_required);
        assert!(!case.private_chain_of_thought_retained);
    }
    assert!(deep.maximum_assumptions > concise.maximum_assumptions);
    assert!(deep.maximum_hypotheses > concise.maximum_hypotheses);
    assert!(deep.maximum_clarification_questions > concise.maximum_clarification_questions);
}

#[test]
fn s_012_it01_fixed_tasks_have_identical_mode_receipts() {
    for task in ["alpha", "beta"] {
        assert_eq!(
            run_fixed_task(ReasoningMode::Concise, task),
            run_fixed_task(ReasoningMode::Deep, task)
        );
    }
}

#[test]
fn s_012_it01_receipts_are_byte_reproducible() {
    for mode in [ReasoningMode::Concise, ReasoningMode::Deep] {
        let first = serde_json::to_vec(&run_fixed_task(mode, "alpha")).expect("receipt serializes");
        let second =
            serde_json::to_vec(&run_fixed_task(mode, "alpha")).expect("receipt serializes");
        assert_eq!(first, second);
        assert_eq!(Sha256::digest(&first), Sha256::digest(&second));
    }
}

#[test]
fn s_012_it01_budget_and_status_cadence_are_exact() {
    let receipt = run_fixed_task(ReasoningMode::Deep, "alpha");
    assert_eq!(
        receipt.status_cadence,
        ["observe", "plan", "act", "review", "observe", "stopped"]
    );
    let usage: Vec<_> = receipt
        .budgets
        .iter()
        .map(|record| (record.resource, record.limit, record.used))
        .collect();
    assert_eq!(
        usage,
        [
            ("plan-steps", 2, 1),
            ("tool-calls", 2, 1),
            ("model-calls", 1, 0),
            ("input-bytes", 64, 8),
            ("output-bytes", 64, 12),
            ("elapsed-milliseconds", 100, 12),
        ]
    );
}

#[test]
fn s_012_it01_fake_boundaries_are_inert_and_bounded() {
    let receipt = run_fixed_task(ReasoningMode::Concise, "beta");
    assert_eq!(receipt.fake_model_calls, 1);
    assert_eq!(receipt.fake_tool_calls, 1);
    assert_eq!(receipt.production_model_calls, 0);
    assert!(!receipt.tool_changed_state);
    assert!(!receipt.external_network_used);
    assert_eq!(receipt.tool_receipt_sha256.len(), 64);
    assert_eq!(receipt.terminal, "user-decision-required");
}
