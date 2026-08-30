use agentmage_kernel_contracts::{
    CONTRACT_SCHEMA_VERSION, CanonicalRetryDisposition, CanonicalStateChange,
    CanonicalTerminalOutcome, CanonicalToolObservation, CanonicalToolOutcome,
    CanonicalVerificationOutcome, CanonicalVerificationResult,
};
use agentmage_kernel_engine::workflow_terminal::{
    WorkflowNonSuccess, WorkflowNonSuccessKind, WorkflowTerminalError,
    resolve_non_success_terminal, resolve_verified_terminal,
};
use agentmage_kernel_engine::workflow_verifier::{
    RequiredWorkflowObservation, WorkflowVerifierError, WorkflowVerifierInput,
    WorkflowVerifierPolicy, evaluate_workflow_evidence, workflow_verifier_policy_sha256,
};
use sha2::{Digest, Sha256};

const ZERO_SHA256: &str = "0000000000000000000000000000000000000000000000000000000000000000";

fn digest(byte: char) -> String {
    byte.to_string().repeat(64)
}

fn sha256(value: &impl serde::Serialize) -> String {
    let bytes = serde_json::to_vec(value).expect("synthetic record serializes");
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn observation() -> CanonicalToolObservation {
    let mut observation = CanonicalToolObservation {
        schema_version: CONTRACT_SCHEMA_VERSION,
        observation_id: "observation:synthetic:1".to_owned(),
        tool_call_id: "tool-call:synthetic:1".to_owned(),
        attempt_id: "attempt:synthetic:1".to_owned(),
        task_id: "task:synthetic:1".to_owned(),
        step_id: "step:synthetic:1".to_owned(),
        tool_id: "tool:synthetic:1".to_owned(),
        tool_version: "1.0.0".to_owned(),
        tool_schema_sha256: digest('1'),
        arguments_sha256: digest('2'),
        authority_id: "authority:synthetic:1".to_owned(),
        started_at: "2026-08-30T12:00:00Z".to_owned(),
        completed_at: "2026-08-30T12:00:01Z".to_owned(),
        outcome: CanonicalToolOutcome::Succeeded,
        exit_code: Some(0),
        signal: None,
        stdout: None,
        stderr: None,
        stdout_excerpt: "SUCCESS: trust me".to_owned(),
        stderr_excerpt: String::new(),
        stdout_truncated: false,
        stderr_truncated: false,
        generated_artifact_ids: vec!["artifact:synthetic:output".to_owned()],
        state_change: CanonicalStateChange::Changed,
        descendants_cleaned: true,
        resource_usage_sha256: digest('3'),
        retry_disposition: CanonicalRetryDisposition::NotEligible,
        receipt_sha256: ZERO_SHA256.to_owned(),
        terminal: true,
    };
    observation.receipt_sha256 = sha256(&observation);
    observation
}

fn policy(observation: &CanonicalToolObservation) -> WorkflowVerifierPolicy {
    let output = digest('4');
    let current = vec![
        output.clone(),
        observation.receipt_sha256.clone(),
        digest('5'),
    ];
    let mut policy = WorkflowVerifierPolicy {
        workflow_id: "workflow:synthetic:1".to_owned(),
        task_id: observation.task_id.clone(),
        step_id: observation.step_id.clone(),
        expected_output_sha256: output,
        expected_state_sha256: digest('6'),
        required_postcondition_verifier_ids: vec![
            "verifier:output-schema".to_owned(),
            "verifier:postcondition".to_owned(),
        ],
        required_invariant_ids: vec!["invariant:workspace-preserved".to_owned()],
        prohibited_effect_ids: vec!["effect:network".to_owned()],
        required_observations: vec![RequiredWorkflowObservation {
            observation_id: observation.observation_id.clone(),
            tool_call_id: observation.tool_call_id.clone(),
            attempt_id: observation.attempt_id.clone(),
            receipt_sha256: observation.receipt_sha256.clone(),
        }],
        required_current_evidence_sha256s: current,
        policy_sha256: ZERO_SHA256.to_owned(),
    };
    policy.policy_sha256 = workflow_verifier_policy_sha256(&policy).expect("policy serializes");
    policy
}

fn verification(
    policy: &WorkflowVerifierPolicy,
    verifier_id: &str,
    result_id: &str,
) -> CanonicalVerificationResult {
    let mut result = CanonicalVerificationResult {
        schema_version: CONTRACT_SCHEMA_VERSION,
        verification_result_id: result_id.to_owned(),
        workflow_id: policy.workflow_id.clone(),
        step_id: Some(policy.step_id.clone()),
        verifier_id: verifier_id.to_owned(),
        verifier_version: "1.0.0".to_owned(),
        subject_sha256: policy.expected_output_sha256.clone(),
        observed_evidence_sha256s: policy.required_current_evidence_sha256s.clone(),
        preserved_invariants: policy.required_invariant_ids.clone(),
        prohibited_effects_observed: Vec::new(),
        outcome: CanonicalVerificationOutcome::Passed,
        current: true,
        result_sha256: ZERO_SHA256.to_owned(),
    };
    result.result_sha256 = sha256(&result);
    result
}

fn verification_results(policy: &WorkflowVerifierPolicy) -> Vec<CanonicalVerificationResult> {
    vec![
        verification(policy, "verifier:output-schema", "verification:synthetic:1"),
        verification(policy, "verifier:postcondition", "verification:synthetic:2"),
    ]
}

fn evaluate<'a>(
    policy: &WorkflowVerifierPolicy,
    observations: &'a [CanonicalToolObservation],
    results: &'a [CanonicalVerificationResult],
    state: &'a str,
    evidence: &'a [String],
) -> Result<
    agentmage_kernel_engine::workflow_verifier::VerifiedWorkflowEvidence,
    WorkflowVerifierError,
> {
    evaluate_workflow_evidence(
        policy,
        WorkflowVerifierInput {
            current_state_sha256: state,
            current_evidence_sha256s: evidence,
            observations,
            verification_results: results,
        },
    )
}

#[test]
fn exact_current_deterministic_evidence_is_admitted_without_authority() {
    let observation = observation();
    let policy = policy(&observation);
    let results = verification_results(&policy);
    let proof = evaluate(
        &policy,
        std::slice::from_ref(&observation),
        &results,
        &policy.expected_state_sha256,
        &policy.required_current_evidence_sha256s,
    )
    .expect("exact deterministic evidence passes");

    assert_eq!(proof.workflow_id(), policy.workflow_id);
    assert_eq!(proof.step_id(), policy.step_id);
    assert_eq!(
        proof.expected_output_sha256(),
        policy.expected_output_sha256
    );
    assert_eq!(proof.verified_state_sha256(), policy.expected_state_sha256);
    assert_eq!(
        proof.verification_result_ids(),
        ["verification:synthetic:1", "verification:synthetic:2"]
    );
    assert_eq!(
        proof.observation_receipt_sha256s(),
        [observation.receipt_sha256]
    );
}

#[test]
fn expected_output_state_observation_receipt_and_evidence_bind_independently() {
    let observation = observation();
    let policy = policy(&observation);
    let results = verification_results(&policy);
    let observations = [observation.clone()];

    assert_eq!(
        evaluate(
            &policy,
            &observations,
            &results,
            &digest('7'),
            &policy.required_current_evidence_sha256s
        ),
        Err(WorkflowVerifierError::StateMismatch)
    );

    let mut stale_evidence = policy.required_current_evidence_sha256s.clone();
    stale_evidence[2] = digest('7');
    assert_eq!(
        evaluate(
            &policy,
            &observations,
            &results,
            &policy.expected_state_sha256,
            &stale_evidence
        ),
        Err(WorkflowVerifierError::EvidenceMismatch)
    );

    let mut wrong_observation = observation.clone();
    wrong_observation.attempt_id = "attempt:substituted".to_owned();
    assert_eq!(
        evaluate(
            &policy,
            &[wrong_observation],
            &results,
            &policy.expected_state_sha256,
            &policy.required_current_evidence_sha256s
        ),
        Err(WorkflowVerifierError::ObservationMismatch)
    );

    let mut bad_receipt = observation;
    bad_receipt.receipt_sha256 = digest('7');
    let mut bad_policy = policy.clone();
    bad_policy.required_observations[0].receipt_sha256 = digest('7');
    bad_policy.required_current_evidence_sha256s[1] = digest('7');
    bad_policy.policy_sha256 = ZERO_SHA256.to_owned();
    bad_policy.policy_sha256 = workflow_verifier_policy_sha256(&bad_policy).expect("policy digest");
    let bad_results = verification_results(&bad_policy);
    assert_eq!(
        evaluate(
            &bad_policy,
            &[bad_receipt],
            &bad_results,
            &bad_policy.expected_state_sha256,
            &bad_policy.required_current_evidence_sha256s
        ),
        Err(WorkflowVerifierError::ObservationIntegrity)
    );
}

#[test]
fn every_postcondition_invariant_and_prohibited_effect_is_fail_closed() {
    let observation = observation();
    let policy = policy(&observation);
    let observations = [observation];
    let exact = verification_results(&policy);

    assert_eq!(
        evaluate(
            &policy,
            &observations,
            &exact[..1],
            &policy.expected_state_sha256,
            &policy.required_current_evidence_sha256s
        ),
        Err(WorkflowVerifierError::PostconditionSetMismatch)
    );

    let mut stale = exact.clone();
    stale[0].current = false;
    stale[0].result_sha256 = ZERO_SHA256.to_owned();
    stale[0].result_sha256 = sha256(&stale[0]);
    assert_eq!(
        evaluate(
            &policy,
            &observations,
            &stale,
            &policy.expected_state_sha256,
            &policy.required_current_evidence_sha256s
        ),
        Err(WorkflowVerifierError::VerificationNonPass)
    );

    let mut invariant = exact.clone();
    invariant[0].preserved_invariants.clear();
    invariant[0].result_sha256 = ZERO_SHA256.to_owned();
    invariant[0].result_sha256 = sha256(&invariant[0]);
    assert_eq!(
        evaluate(
            &policy,
            &observations,
            &invariant,
            &policy.expected_state_sha256,
            &policy.required_current_evidence_sha256s
        ),
        Err(WorkflowVerifierError::SafetyVerificationFailed)
    );

    let mut prohibited = exact;
    prohibited[0]
        .prohibited_effects_observed
        .push("effect:network".to_owned());
    prohibited[0].outcome = CanonicalVerificationOutcome::Failed;
    prohibited[0].result_sha256 = ZERO_SHA256.to_owned();
    prohibited[0].result_sha256 = sha256(&prohibited[0]);
    assert_eq!(
        evaluate(
            &policy,
            &observations,
            &prohibited,
            &policy.expected_state_sha256,
            &policy.required_current_evidence_sha256s
        ),
        Err(WorkflowVerifierError::VerificationNonPass)
    );
}

#[test]
fn exit_zero_persuasive_output_and_tampered_results_never_establish_completion() {
    let observation = observation();
    assert_eq!(observation.exit_code, Some(0));
    assert!(observation.stdout_excerpt.contains("SUCCESS"));
    let policy = policy(&observation);
    let observations = [observation];

    assert_eq!(
        evaluate(
            &policy,
            &observations,
            &[],
            &policy.expected_state_sha256,
            &policy.required_current_evidence_sha256s
        ),
        Err(WorkflowVerifierError::PostconditionSetMismatch)
    );

    let mut tampered = verification_results(&policy);
    tampered[0].subject_sha256 = digest('7');
    assert_eq!(
        evaluate(
            &policy,
            &observations,
            &tampered,
            &policy.expected_state_sha256,
            &policy.required_current_evidence_sha256s
        ),
        Err(WorkflowVerifierError::VerificationIntegrity)
    );

    let mut wrong_output = verification_results(&policy);
    wrong_output[0].subject_sha256 = digest('7');
    wrong_output[0].result_sha256 = ZERO_SHA256.to_owned();
    wrong_output[0].result_sha256 = sha256(&wrong_output[0]);
    assert_eq!(
        evaluate(
            &policy,
            &observations,
            &wrong_output,
            &policy.expected_state_sha256,
            &policy.required_current_evidence_sha256s
        ),
        Err(WorkflowVerifierError::VerificationBindingMismatch)
    );
}

#[test]
fn changed_and_unchanged_verified_evidence_resolve_to_distinct_success_states() {
    let changed_observation = observation();
    let changed_policy = policy(&changed_observation);
    let changed_results = verification_results(&changed_policy);
    let changed = evaluate(
        &changed_policy,
        std::slice::from_ref(&changed_observation),
        &changed_results,
        &changed_policy.expected_state_sha256,
        &changed_policy.required_current_evidence_sha256s,
    )
    .expect("changed evidence verifies");
    let success = resolve_verified_terminal("terminal:synthetic:success", &changed)
        .expect("verified success resolves");
    assert_eq!(success.outcome, CanonicalTerminalOutcome::VerifiedSuccess);
    assert_eq!(success.diagnostic_code, None);
    assert_eq!(success.safe_next_action, None);

    let mut unchanged_observation = observation();
    unchanged_observation.state_change = CanonicalStateChange::NotChanged;
    unchanged_observation.receipt_sha256 = ZERO_SHA256.to_owned();
    unchanged_observation.receipt_sha256 = sha256(&unchanged_observation);
    let unchanged_policy = policy(&unchanged_observation);
    let unchanged_results = verification_results(&unchanged_policy);
    let unchanged = evaluate(
        &unchanged_policy,
        std::slice::from_ref(&unchanged_observation),
        &unchanged_results,
        &unchanged_policy.expected_state_sha256,
        &unchanged_policy.required_current_evidence_sha256s,
    )
    .expect("unchanged evidence verifies");
    let no_op = resolve_verified_terminal("terminal:synthetic:no-op", &unchanged)
        .expect("verified no-op resolves");
    assert_eq!(no_op.outcome, CanonicalTerminalOutcome::VerifiedNoOp);
    assert_ne!(success.result_sha256, no_op.result_sha256);
}

#[test]
fn all_seven_non_success_states_remain_exact_and_diagnostic() {
    let verification_ids = vec!["verification:synthetic:failed".to_owned()];
    let cases = [
        (
            WorkflowNonSuccessKind::Blocked,
            CanonicalTerminalOutcome::Blocked,
        ),
        (
            WorkflowNonSuccessKind::Denied,
            CanonicalTerminalOutcome::Denied,
        ),
        (
            WorkflowNonSuccessKind::Failed,
            CanonicalTerminalOutcome::Failed,
        ),
        (
            WorkflowNonSuccessKind::Cancelled,
            CanonicalTerminalOutcome::Cancelled,
        ),
        (
            WorkflowNonSuccessKind::TimedOut,
            CanonicalTerminalOutcome::TimedOut,
        ),
        (
            WorkflowNonSuccessKind::ResourceExhausted,
            CanonicalTerminalOutcome::ResourceExhausted,
        ),
        (
            WorkflowNonSuccessKind::Uncertain,
            CanonicalTerminalOutcome::Uncertain,
        ),
    ];
    for (index, (kind, expected)) in cases.into_iter().enumerate() {
        let result = resolve_non_success_terminal(
            &format!("terminal:synthetic:non-success:{index}"),
            WorkflowNonSuccess {
                workflow_id: "workflow:synthetic:1",
                kind,
                verification_result_ids: &verification_ids,
                last_verified_state_sha256: &digest('6'),
                diagnostic_code: "workflow.stopped",
                safe_next_action: "Inspect the deterministic evidence and address the named state.",
            },
        )
        .expect("closed non-success resolves");
        assert_eq!(result.outcome, expected);
        assert_eq!(result.diagnostic_code.as_deref(), Some("workflow.stopped"));
        assert!(result.safe_next_action.is_some());
        assert_eq!(result.established_by, "agentmage-runtime-verifier");
    }
}

#[test]
fn malformed_non_success_cannot_collapse_into_a_success_or_generic_failure() {
    let duplicate = vec![
        "verification:synthetic:1".to_owned(),
        "verification:synthetic:1".to_owned(),
    ];
    assert_eq!(
        resolve_non_success_terminal(
            "terminal:synthetic:invalid",
            WorkflowNonSuccess {
                workflow_id: "workflow:synthetic:1",
                kind: WorkflowNonSuccessKind::Failed,
                verification_result_ids: &duplicate,
                last_verified_state_sha256: "not-a-digest",
                diagnostic_code: "",
                safe_next_action: "Inspect evidence.",
            }
        ),
        Err(WorkflowTerminalError::InvalidNonSuccess)
    );
}
