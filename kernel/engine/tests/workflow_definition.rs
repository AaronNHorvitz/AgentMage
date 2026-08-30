use std::fmt::Write as _;

use agentmage_kernel_contracts::{
    CONTRACT_SCHEMA_VERSION, CanonicalApprovalRequirement, CanonicalDiagnosticDisclosure,
    CanonicalEffectClass, CanonicalExecutionBudgets, CanonicalIdempotencyRequirement,
    CanonicalRetryClass, CanonicalSchemaBinding, CanonicalStepExecutionPolicy,
    CanonicalVerificationRequirement, CanonicalWorkflowDefinition, CanonicalWorkflowLifecycle,
    CanonicalWorkflowStep, PlanId, PlanStepId, to_canonical_json,
};
use agentmage_kernel_engine::workflow_definition::{
    WorkflowDefinitionAdmissionError, WorkflowStepPolicyBinding, admit_closed_workflow,
    workflow_lifecycle_is_terminal,
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

fn budgets() -> CanonicalExecutionBudgets {
    CanonicalExecutionBudgets {
        turns: 2,
        tokens: 4_096,
        duration_ms: 30_000,
        tool_calls: 2,
        attempts: 2,
        no_progress_events: 1,
        output_bytes: 65_536,
        memory_bytes: 16_777_216,
        cost_minor_units: 0,
    }
}

fn seal_definition(mut definition: CanonicalWorkflowDefinition) -> CanonicalWorkflowDefinition {
    definition.definition_sha256 = ZERO.to_owned();
    definition.definition_sha256 = sha256(&to_canonical_json(&definition).unwrap());
    definition
}

fn definition() -> CanonicalWorkflowDefinition {
    seal_definition(CanonicalWorkflowDefinition {
        schema_version: CONTRACT_SCHEMA_VERSION,
        workflow_id: "workflow:fixture".to_owned(),
        workflow_version: 3,
        input_schema: CanonicalSchemaBinding {
            schema_id: "schema:input".to_owned(),
            schema_version: 1,
            schema_sha256: "1".repeat(64),
        },
        output_schema: CanonicalSchemaBinding {
            schema_id: "schema:output".to_owned(),
            schema_version: 1,
            schema_sha256: "2".repeat(64),
        },
        steps: vec![
            CanonicalWorkflowStep {
                step_id: "step:inspect".to_owned(),
                depends_on: vec![],
                model_role: None,
                tool_id: Some("tool:inspect".to_owned()),
                effect_class: CanonicalEffectClass::ReadOnly,
                retry_class: CanonicalRetryClass::RecoverableRead,
                verifier_ids: vec!["verify:inspection".to_owned()],
                budgets: budgets(),
            },
            CanonicalWorkflowStep {
                step_id: "step:write".to_owned(),
                depends_on: vec!["step:inspect".to_owned()],
                model_role: Some("model:reviewer".to_owned()),
                tool_id: Some("tool:conditional-write".to_owned()),
                effect_class: CanonicalEffectClass::Conditional,
                retry_class: CanonicalRetryClass::ConditionalAfterReconciliation,
                verifier_ids: vec!["verify:postcondition".to_owned()],
                budgets: budgets(),
            },
        ],
        entry_step_ids: vec!["step:inspect".to_owned()],
        definition_sha256: ZERO.to_owned(),
    })
}

fn seal_policy(mut policy: CanonicalStepExecutionPolicy) -> CanonicalStepExecutionPolicy {
    policy.policy_sha256 = ZERO.to_owned();
    policy.policy_sha256 = sha256(&to_canonical_json(&policy).unwrap());
    policy
}

fn policy(
    suffix: &str,
    effect_class: CanonicalEffectClass,
    retry_class: CanonicalRetryClass,
    verifier_id: &str,
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
    seal_policy(CanonicalStepExecutionPolicy {
        schema_version: CONTRACT_SCHEMA_VERSION,
        policy_id: format!("policy:{suffix}"),
        plan_id: PlanId::from_raw("plan:fixture"),
        plan_step_id: PlanStepId::from_raw(format!("plan-step:{suffix}")),
        plan_revision: 3,
        preflight_policy_id: format!("preflight-policy:{suffix}"),
        required_preflight_ids: vec![format!("preflight:{suffix}")],
        side_effect_policy_id: format!("effect-policy:{suffix}"),
        effect_class,
        approval_policy_id: format!("approval-policy:{suffix}"),
        approval_requirement,
        idempotency_policy_id: format!("idempotency-policy:{suffix}"),
        idempotency_key_requirement,
        verifier_policy_id: format!("verifier-policy:{suffix}"),
        verification_requirement: CanonicalVerificationRequirement::VerifierEvidenceRequired,
        required_verifier_ids: vec![verifier_id.to_owned()],
        deferral_reason_code: None,
        retry_policy_id: format!("retry-policy:{suffix}"),
        retry_class,
        budget_policy_id: format!("budget-policy:{suffix}"),
        budgets: budgets(),
        diagnostic_policy_id: format!("diagnostic-policy:{suffix}"),
        diagnostic_disclosure: CanonicalDiagnosticDisclosure::ContentFreeCodes,
        recorded_at: "2026-08-30T12:00:00Z".to_owned(),
        policy_sha256: ZERO.to_owned(),
    })
}

fn policies() -> (CanonicalStepExecutionPolicy, CanonicalStepExecutionPolicy) {
    (
        policy(
            "inspect",
            CanonicalEffectClass::ReadOnly,
            CanonicalRetryClass::RecoverableRead,
            "verify:inspection",
        ),
        policy(
            "write",
            CanonicalEffectClass::Conditional,
            CanonicalRetryClass::ConditionalAfterReconciliation,
            "verify:postcondition",
        ),
    )
}

fn admit(
    definition: &CanonicalWorkflowDefinition,
    inspect: &CanonicalStepExecutionPolicy,
    write: &CanonicalStepExecutionPolicy,
) -> Result<
    agentmage_kernel_engine::workflow_definition::AdmittedWorkflowDefinition,
    WorkflowDefinitionAdmissionError,
> {
    admit_closed_workflow(
        definition,
        &[
            WorkflowStepPolicyBinding {
                step_id: "step:write",
                policy: write,
            },
            WorkflowStepPolicyBinding {
                step_id: "step:inspect",
                policy: inspect,
            },
        ],
    )
}

#[test]
fn admits_an_exact_closed_graph_and_exposes_no_execution_authority() {
    let definition = definition();
    let (inspect, write) = policies();
    let admitted = admit(&definition, &inspect, &write).unwrap();
    assert_eq!(admitted.workflow_id(), "workflow:fixture");
    assert_eq!(admitted.workflow_version(), 3);
    assert_eq!(admitted.definition_sha256(), definition.definition_sha256);
    assert_eq!(admitted.entry_step_ids(), ["step:inspect"]);
    assert_eq!(admitted.ordered_step_ids(), ["step:inspect", "step:write"]);
    assert_eq!(
        admitted.policy_sha256("step:write"),
        Some(write.policy_sha256.as_str())
    );
    assert_eq!(admitted.policy_sha256("step:absent"), None);
}

#[test]
fn definition_digest_and_exact_entry_frontier_fail_closed() {
    let (inspect, write) = policies();
    let mut changed = definition();
    changed.workflow_version += 1;
    assert_eq!(
        admit(&changed, &inspect, &write),
        Err(WorkflowDefinitionAdmissionError::DefinitionIntegrity)
    );

    let mut changed = definition();
    changed.entry_step_ids = vec!["step:write".to_owned()];
    changed = seal_definition(changed);
    assert_eq!(
        admit(&changed, &inspect, &write),
        Err(WorkflowDefinitionAdmissionError::EntryFrontierMismatch)
    );
}

#[test]
fn dependency_order_and_non_executable_steps_are_denied() {
    let (inspect, write) = policies();
    let mut changed = definition();
    changed.steps.swap(0, 1);
    changed = seal_definition(changed);
    assert_eq!(
        admit(&changed, &inspect, &write),
        Err(WorkflowDefinitionAdmissionError::StepOrderInvalid)
    );

    let mut changed = definition();
    changed.steps[0].tool_id = None;
    changed = seal_definition(changed);
    assert_eq!(
        admit(&changed, &inspect, &write),
        Err(WorkflowDefinitionAdmissionError::InvalidDefinition)
    );
}

#[test]
fn policy_set_must_be_exact_and_identity_unique() {
    let definition = definition();
    let (inspect, mut write) = policies();
    assert_eq!(
        admit_closed_workflow(
            &definition,
            &[WorkflowStepPolicyBinding {
                step_id: "step:inspect",
                policy: &inspect,
            }],
        ),
        Err(WorkflowDefinitionAdmissionError::PolicySetMismatch)
    );
    write.policy_id = inspect.policy_id.clone();
    write = seal_policy(write);
    assert_eq!(
        admit(&definition, &inspect, &write),
        Err(WorkflowDefinitionAdmissionError::PolicyIdentityReuse)
    );
}

#[test]
fn every_step_policy_surface_is_bound_to_the_definition() {
    let definition = definition();
    let (inspect, write) = policies();
    for index in 0..4 {
        let mut changed = write.clone();
        match index {
            0 => changed.effect_class = CanonicalEffectClass::IdempotentWrite,
            1 => changed.retry_class = CanonicalRetryClass::Never,
            2 => changed.required_verifier_ids = vec!["verify:different".to_owned()],
            3 => changed.budgets.tokens += 1,
            _ => unreachable!(),
        }
        changed = seal_policy(changed);
        assert_eq!(
            admit(&definition, &inspect, &changed),
            Err(WorkflowDefinitionAdmissionError::PolicyStepMismatch),
            "mutation {index}"
        );
    }
}

#[test]
fn preflight_postcondition_and_policy_integrity_mutations_are_denied() {
    let definition = definition();
    let (inspect, write) = policies();
    let mut changed = write.clone();
    changed.required_preflight_ids.clear();
    changed = seal_policy(changed);
    assert_eq!(
        admit(&definition, &inspect, &changed),
        Err(WorkflowDefinitionAdmissionError::PreflightOrPostconditionInvalid)
    );

    let mut changed = write.clone();
    changed.verification_requirement = CanonicalVerificationRequirement::PolicyDeferred;
    changed = seal_policy(changed);
    assert_eq!(
        admit(&definition, &inspect, &changed),
        Err(WorkflowDefinitionAdmissionError::PreflightOrPostconditionInvalid)
    );

    let mut changed = write.clone();
    changed.recorded_at = "2026-08-31T12:00:00Z".to_owned();
    assert_eq!(
        admit(&definition, &inspect, &changed),
        Err(WorkflowDefinitionAdmissionError::PolicyIntegrity)
    );
}

#[test]
fn effect_classes_require_conservative_approval_and_idempotency_policy() {
    let definition = definition();
    let (inspect, write) = policies();
    for effect in [
        CanonicalEffectClass::NonIdempotent,
        CanonicalEffectClass::Destructive,
        CanonicalEffectClass::External,
        CanonicalEffectClass::Unknown,
    ] {
        let mut changed = write.clone();
        changed.effect_class = effect;
        changed.retry_class = CanonicalRetryClass::UserDecisionRequired;
        changed.approval_requirement = CanonicalApprovalRequirement::NotRequired;
        changed.idempotency_key_requirement = CanonicalIdempotencyRequirement::VerifiedDesiredState;
        changed = seal_policy(changed);
        let mut matching_definition = definition.clone();
        matching_definition.steps[1].effect_class = effect;
        matching_definition.steps[1].retry_class = CanonicalRetryClass::UserDecisionRequired;
        matching_definition = seal_definition(matching_definition);
        assert_eq!(
            admit(&matching_definition, &inspect, &changed),
            Err(WorkflowDefinitionAdmissionError::ApprovalPolicyInvalid),
            "effect {effect:?}"
        );
    }

    let mut changed = write;
    changed.idempotency_key_requirement = CanonicalIdempotencyRequirement::NotApplicable;
    changed = seal_policy(changed);
    assert_eq!(
        admit(&definition, &inspect, &changed),
        Err(WorkflowDefinitionAdmissionError::IdempotencyPolicyInvalid)
    );
}

#[test]
fn terminal_state_family_is_closed_and_nonterminal_states_do_not_leak_into_it() {
    let cases = [
        (CanonicalWorkflowLifecycle::Created, false),
        (CanonicalWorkflowLifecycle::Validating, false),
        (CanonicalWorkflowLifecycle::Ready, false),
        (CanonicalWorkflowLifecycle::Running, false),
        (CanonicalWorkflowLifecycle::Verifying, false),
        (CanonicalWorkflowLifecycle::WaitingForDependency, false),
        (CanonicalWorkflowLifecycle::WaitingForApproval, false),
        (CanonicalWorkflowLifecycle::Paused, false),
        (CanonicalWorkflowLifecycle::Reconciling, false),
        (CanonicalWorkflowLifecycle::Recovering, false),
        (CanonicalWorkflowLifecycle::Succeeded, true),
        (CanonicalWorkflowLifecycle::NoOp, true),
        (CanonicalWorkflowLifecycle::Blocked, true),
        (CanonicalWorkflowLifecycle::Failed, true),
        (CanonicalWorkflowLifecycle::Cancelled, true),
        (CanonicalWorkflowLifecycle::TimedOut, true),
        (CanonicalWorkflowLifecycle::ResourceExhausted, true),
        (CanonicalWorkflowLifecycle::Uncertain, true),
    ];
    for (state, terminal) in cases {
        assert_eq!(workflow_lifecycle_is_terminal(state), terminal, "{state:?}");
    }
}
