use agentmage_kernel_contracts::CanonicalWorkflowFailureClass;
use agentmage_kernel_engine::tool_call_repair::{
    BudgetedToolCallRepairError, ModelRepairPolicy, OrderedToolCallFragment, ToolCallRepairError,
    admit_targeted_model_repair_with_budget, normalize_tool_call_with_budget,
};
use agentmage_kernel_engine::workflow_budget::{
    ErrorClassBudgetLimit, WorkflowBudgetDimension, WorkflowBudgetError, WorkflowBudgetEvent,
    WorkflowBudgetLedger, WorkflowBudgetPolicy,
};
use agentmage_kernel_engine::workflow_progress::{
    RepeatedStateDetector, RepeatedStatePolicy, WorkflowProgressDecision, WorkflowStateComponents,
    WorkflowStateFingerprint,
};

const PROFILE: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const SCHEMA: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

fn error_limits(limit: u64) -> Vec<ErrorClassBudgetLimit> {
    CanonicalWorkflowFailureClass::ALL
        .into_iter()
        .map(|failure_class| ErrorClassBudgetLimit {
            failure_class,
            limit,
        })
        .collect()
}

fn policy(total: u64) -> WorkflowBudgetPolicy {
    WorkflowBudgetPolicy::new(
        "workflow-budget:integration".to_owned(),
        1,
        1,
        1,
        &error_limits(1),
        total,
        1,
    )
    .unwrap()
}

fn fragments() -> Vec<OrderedToolCallFragment> {
    vec![OrderedToolCallFragment {
        ordinal: 0,
        bytes: b" {\"path\":\"README.md\"} ".to_vec(),
    }]
}

fn fingerprint() -> WorkflowStateFingerprint {
    let digest = "c".repeat(64);
    WorkflowStateFingerprint::new(&WorkflowStateComponents {
        plan_sha256: digest.clone(),
        step_sha256: digest.clone(),
        observation_sha256: vec![],
        proposal_sha256: digest.clone(),
        tool_sha256: digest.clone(),
        policy_sha256: digest.clone(),
        receipt_sha256: vec![],
        artifact_sha256: vec![],
        verifier_state_sha256: digest,
    })
    .unwrap()
}

#[test]
fn parser_model_attempt_replan_repeat_and_total_limits_are_independent() {
    let budget_policy = policy(6);
    let mut ledger = WorkflowBudgetLedger::new(&budget_policy);
    let normalized = normalize_tool_call_with_budget(&fragments(), &budget_policy, &mut ledger)
        .expect("first parser repair");
    let repair_policy =
        ModelRepairPolicy::new(PROFILE.to_owned(), SCHEMA.to_owned(), true).unwrap();
    admit_targeted_model_repair_with_budget(
        &repair_policy,
        &normalized,
        PROFILE,
        SCHEMA,
        true,
        0,
        0,
        &budget_policy,
        &mut ledger,
    )
    .expect("first model repair");
    ledger
        .consume(&budget_policy, WorkflowBudgetEvent::StepAttempt, 1)
        .unwrap();
    ledger
        .consume(&budget_policy, WorkflowBudgetEvent::Replan, 1)
        .unwrap();

    assert_eq!(ledger.usage(WorkflowBudgetDimension::ParserRepair), 1);
    assert_eq!(ledger.usage(WorkflowBudgetDimension::ModelRepair), 1);
    assert_eq!(ledger.usage(WorkflowBudgetDimension::StepAttempt), 1);
    assert_eq!(ledger.usage(WorkflowBudgetDimension::Replan), 1);
    assert_eq!(ledger.usage(WorkflowBudgetDimension::WorkflowWork), 4);

    let repeat_policy = RepeatedStatePolicy::new("repeat:integration".to_owned(), 1).unwrap();
    let mut detector = RepeatedStateDetector::new(&repeat_policy);
    let fingerprint = fingerprint();
    assert!(matches!(
        detector.observe(&repeat_policy, &fingerprint).unwrap(),
        WorkflowProgressDecision::Advanced { .. }
    ));
    assert!(matches!(
        detector.observe(&repeat_policy, &fingerprint).unwrap(),
        WorkflowProgressDecision::StopRepeatedState { .. }
    ));
    assert!(detector.is_stopped());
    assert_eq!(ledger.usage(WorkflowBudgetDimension::WorkflowWork), 4);
}

#[test]
fn each_exhausted_repair_dimension_denies_without_touching_other_usage() {
    let budget_policy = policy(10);
    let mut ledger = WorkflowBudgetLedger::new(&budget_policy);
    let normalized = normalize_tool_call_with_budget(&fragments(), &budget_policy, &mut ledger)
        .expect("first parser repair");
    let before_parser_denial = ledger.clone();
    assert_eq!(
        normalize_tool_call_with_budget(&fragments(), &budget_policy, &mut ledger),
        Err(BudgetedToolCallRepairError::Budget(
            WorkflowBudgetError::BudgetExceeded {
                dimension: WorkflowBudgetDimension::ParserRepair,
                limit: 1,
                attempted_total: 2,
            }
        ))
    );
    assert_eq!(ledger, before_parser_denial);

    let repair_policy =
        ModelRepairPolicy::new(PROFILE.to_owned(), SCHEMA.to_owned(), true).unwrap();
    admit_targeted_model_repair_with_budget(
        &repair_policy,
        &normalized,
        PROFILE,
        SCHEMA,
        true,
        0,
        0,
        &budget_policy,
        &mut ledger,
    )
    .expect("first model repair");
    let before_model_denial = ledger.clone();
    assert_eq!(
        admit_targeted_model_repair_with_budget(
            &repair_policy,
            &normalized,
            PROFILE,
            SCHEMA,
            true,
            0,
            0,
            &budget_policy,
            &mut ledger,
        ),
        Err(BudgetedToolCallRepairError::Budget(
            WorkflowBudgetError::BudgetExceeded {
                dimension: WorkflowBudgetDimension::ModelRepair,
                limit: 1,
                attempted_total: 2,
            }
        ))
    );
    assert_eq!(ledger, before_model_denial);
}

#[test]
fn invalid_repairs_and_total_work_exhaustion_return_no_admission_and_no_partial_charge() {
    let budget_policy = policy(1);
    let mut ledger = WorkflowBudgetLedger::new(&budget_policy);
    assert_eq!(
        normalize_tool_call_with_budget(&[], &budget_policy, &mut ledger),
        Err(BudgetedToolCallRepairError::Repair(
            ToolCallRepairError::EmptyFragments
        ))
    );
    assert_eq!(ledger.usage(WorkflowBudgetDimension::ParserRepair), 0);
    ledger
        .consume(&budget_policy, WorkflowBudgetEvent::StepAttempt, 1)
        .unwrap();
    let before = ledger.clone();
    assert_eq!(
        normalize_tool_call_with_budget(&fragments(), &budget_policy, &mut ledger),
        Err(BudgetedToolCallRepairError::Budget(
            WorkflowBudgetError::BudgetExceeded {
                dimension: WorkflowBudgetDimension::WorkflowWork,
                limit: 1,
                attempted_total: 2,
            }
        ))
    );
    assert_eq!(ledger, before);
}
