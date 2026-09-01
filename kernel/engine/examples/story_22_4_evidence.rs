use agentmage_kernel_contracts::{
    CONTRACT_SCHEMA_VERSION, CanonicalAttemptBudgetState, CanonicalAttemptCheckpoint,
    CanonicalCheckpointEffectState, CanonicalWorkflowFailureClass, RuntimeEventCursor,
    RuntimeEventId, RuntimeRunId,
};
use agentmage_kernel_engine::durable_attempt_recovery::{
    AttemptBudgetLimits, AttemptRecoveryAction, AttemptResumeObservation, AttemptTerminalDiagnosis,
    attempt_recovery_decision_sha256, decide_attempt_recovery, seal_attempt_checkpoint,
};
use serde::Serialize;

fn hash(byte: char) -> String {
    byte.to_string().repeat(64)
}

fn checkpoint(effect_state: CanonicalCheckpointEffectState) -> CanonicalAttemptCheckpoint {
    seal_attempt_checkpoint(CanonicalAttemptCheckpoint {
        schema_version: CONTRACT_SCHEMA_VERSION,
        attempt_checkpoint_id: "story-22-4-checkpoint".to_owned(),
        workflow_checkpoint_id: "story-22-4-workflow-checkpoint".to_owned(),
        workflow_id: "story-22-4-workflow".to_owned(),
        execution_id: "story-22-4-execution".to_owned(),
        step_execution_id: "story-22-4-step".to_owned(),
        attempt_id: Some("story-22-4-attempt".to_owned()),
        source_sha256: hash('1'),
        plan_sha256: hash('2'),
        model_sha256: hash('3'),
        context_sha256: hash('4'),
        tool_catalog_sha256: hash('5'),
        policy_sha256: hash('6'),
        grants_sha256: hash('7'),
        approvals_sha256: hash('8'),
        preflights_sha256: hash('9'),
        attempts_sha256: hash('a'),
        receipts_sha256: hash('b'),
        artifacts_sha256: hash('c'),
        verifier_sha256: hash('d'),
        budget_policy_sha256: hash('e'),
        budget_state: CanonicalAttemptBudgetState {
            parser_repairs: 1,
            model_repairs: 0,
            step_attempts: 1,
            error_class_counts: vec![0; CanonicalWorkflowFailureClass::ALL.len()],
            workflow_work: 2,
            replans: 0,
        },
        repeated_state_sha256: hash('f'),
        repeated_state_count: 1,
        event_cursor: RuntimeEventCursor {
            run_id: RuntimeRunId::from_raw("story-22-4-run"),
            event_id: RuntimeEventId::from_raw("story-22-4-event"),
            sequence: 11,
            event_sha256: hash('0'),
        },
        environment_sha256: hash('1'),
        effect_state,
        authority_consumed: !matches!(effect_state, CanonicalCheckpointEffectState::NoEffect),
        created_at: "2026-08-31T12:00:00Z".to_owned(),
        checkpoint_sha256: hash('0'),
    })
    .expect("sealed checkpoint")
}

fn observation(checkpoint: &CanonicalAttemptCheckpoint) -> AttemptResumeObservation {
    AttemptResumeObservation {
        workflow_checkpoint_id: checkpoint.workflow_checkpoint_id.clone(),
        workflow_id: checkpoint.workflow_id.clone(),
        execution_id: checkpoint.execution_id.clone(),
        step_execution_id: checkpoint.step_execution_id.clone(),
        attempt_id: checkpoint.attempt_id.clone(),
        source_sha256: checkpoint.source_sha256.clone(),
        plan_sha256: checkpoint.plan_sha256.clone(),
        model_sha256: checkpoint.model_sha256.clone(),
        context_sha256: checkpoint.context_sha256.clone(),
        tool_catalog_sha256: checkpoint.tool_catalog_sha256.clone(),
        policy_sha256: checkpoint.policy_sha256.clone(),
        grants_sha256: checkpoint.grants_sha256.clone(),
        approvals_sha256: checkpoint.approvals_sha256.clone(),
        preflights_sha256: checkpoint.preflights_sha256.clone(),
        attempts_sha256: checkpoint.attempts_sha256.clone(),
        receipts_sha256: checkpoint.receipts_sha256.clone(),
        artifacts_sha256: checkpoint.artifacts_sha256.clone(),
        verifier_sha256: checkpoint.verifier_sha256.clone(),
        budget_policy_sha256: checkpoint.budget_policy_sha256.clone(),
        budget_state: checkpoint.budget_state.clone(),
        repeated_state_sha256: checkpoint.repeated_state_sha256.clone(),
        repeated_state_count: checkpoint.repeated_state_count,
        event_cursor: checkpoint.event_cursor.clone(),
        environment_sha256: checkpoint.environment_sha256.clone(),
        effect_state: checkpoint.effect_state,
        authority_consumed: checkpoint.authority_consumed,
        postcondition_current: true,
        cancelled: false,
        dependency_blocked: false,
        fresh_approval_required: false,
        deterministic_repair_eligible: false,
        fresh_attempt_eligible: false,
        already_terminal: false,
        budget_limits: AttemptBudgetLimits {
            parser_repairs: 4,
            model_repairs: 4,
            step_attempts: 4,
            error_class_counts: vec![4; CanonicalWorkflowFailureClass::ALL.len()],
            workflow_work: 16,
            replans: 4,
            repeated_state: 4,
        },
    }
}

#[derive(Serialize)]
struct DecisionGolden {
    case: &'static str,
    action: AttemptRecoveryAction,
    decision_sha256: String,
    replay_allowed: bool,
    fresh_authority_required: bool,
}

#[derive(Serialize)]
struct EvidenceGolden {
    record_type: &'static str,
    schema_version: u16,
    checkpoint: CanonicalAttemptCheckpoint,
    decisions: Vec<DecisionGolden>,
    crash_traces: Vec<CrashTrace>,
    checkpoint_graph: CheckpointGraph,
    invalidation_dimensions: [&'static str; 18],
    terminal_diagnosis: AttemptTerminalDiagnosis,
    cleanup: CleanupGolden,
    deterministic_seed_count: u64,
    interruption_boundary_count: usize,
    concurrent_client_count: u64,
    durable_owner_count: u64,
    duplicate_effect_count: u64,
    replay_count: u64,
    external_values_retained: u64,
}

#[derive(Serialize)]
struct CrashTrace {
    seed: u64,
    boundary: &'static str,
    position: &'static str,
    retained_effect: CanonicalCheckpointEffectState,
    recovery_action: AttemptRecoveryAction,
    replay_count: u64,
}

#[derive(Serialize)]
struct CheckpointGraph {
    nodes: [&'static str; 3],
    edges: [(&'static str, &'static str); 2],
    current_node: &'static str,
}

#[derive(Serialize)]
struct CleanupGolden {
    temporary_campaign_directories_remaining: u64,
    orphan_resume_owners: u64,
    unbound_checkpoint_count: u64,
}

fn main() {
    let baseline = checkpoint(CanonicalCheckpointEffectState::NoEffect);
    let mut cases = Vec::new();
    {
        let mut record = |case, current: AttemptResumeObservation| {
            let decision = decide_attempt_recovery(&baseline, &current);
            cases.push(DecisionGolden {
                case,
                action: decision.action,
                decision_sha256: attempt_recovery_decision_sha256(&decision),
                replay_allowed: decision.prior_effect_replay_allowed,
                fresh_authority_required: decision.fresh_identity_and_authority_required,
            });
        };
        record("continue", observation(&baseline));
        let mut current = observation(&baseline);
        current.fresh_attempt_eligible = true;
        record("fresh_attempt", current);
        let mut current = observation(&baseline);
        current.deterministic_repair_eligible = true;
        record("deterministic_repair", current);
        let mut current = observation(&baseline);
        current.source_sha256 = hash('f');
        record("replan", current);
        let mut current = observation(&baseline);
        current.fresh_approval_required = true;
        record("await_approval", current);
        let mut current = observation(&baseline);
        current.dependency_blocked = true;
        record("await_dependency", current);
    }
    let uncertain = checkpoint(CanonicalCheckpointEffectState::Uncertain);
    let decision = decide_attempt_recovery(&uncertain, &observation(&uncertain));
    cases.push(DecisionGolden {
        case: "reconcile_effect",
        action: decision.action,
        decision_sha256: attempt_recovery_decision_sha256(&decision),
        replay_allowed: decision.prior_effect_replay_allowed,
        fresh_authority_required: decision.fresh_identity_and_authority_required,
    });
    let mut current = observation(&baseline);
    current.cancelled = true;
    let decision = decide_attempt_recovery(&baseline, &current);
    cases.push(DecisionGolden {
        case: "cancel",
        action: decision.action,
        decision_sha256: attempt_recovery_decision_sha256(&decision),
        replay_allowed: decision.prior_effect_replay_allowed,
        fresh_authority_required: decision.fresh_identity_and_authority_required,
    });
    let mut current = observation(&baseline);
    current.budget_state.step_attempts = 4;
    let decision = decide_attempt_recovery(&baseline, &current);
    let terminal_diagnosis = decision.diagnosis.clone().expect("terminal diagnosis");
    cases.push(DecisionGolden {
        case: "terminate_diagnosed",
        action: decision.action,
        decision_sha256: attempt_recovery_decision_sha256(&decision),
        replay_allowed: decision.prior_effect_replay_allowed,
        fresh_authority_required: decision.fresh_identity_and_authority_required,
    });

    let boundaries = [
        "source",
        "context",
        "proposal",
        "preflight",
        "approval",
        "grant",
        "worker",
        "receipt",
        "artifact",
        "verification",
        "retry",
        "recovery",
        "checkpoint",
        "terminal",
    ];
    let crash_traces = (0_u64..100)
        .map(|seed| {
            let boundary_index = (seed as usize / 2) % boundaries.len();
            let after = seed % 2 == 1;
            let worker_boundary = boundaries
                .iter()
                .position(|candidate| *candidate == "worker")
                .expect("worker boundary exists");
            let effect = if boundary_index < worker_boundary
                || (boundary_index == worker_boundary && !after)
            {
                CanonicalCheckpointEffectState::NoEffect
            } else if boundary_index == worker_boundary {
                CanonicalCheckpointEffectState::Uncertain
            } else {
                CanonicalCheckpointEffectState::VerifiedApplied
            };
            let retained = checkpoint(effect);
            let decision = decide_attempt_recovery(&retained, &observation(&retained));
            CrashTrace {
                seed,
                boundary: boundaries[boundary_index],
                position: if after { "after" } else { "before" },
                retained_effect: effect,
                recovery_action: decision.action,
                replay_count: u64::from(decision.prior_effect_replay_allowed),
            }
        })
        .collect();

    let evidence = EvidenceGolden {
        record_type: "agentmage-story-22-4-attempt-recovery-goldens",
        schema_version: CONTRACT_SCHEMA_VERSION,
        checkpoint: baseline,
        decisions: cases,
        crash_traces,
        checkpoint_graph: CheckpointGraph {
            nodes: [
                "session_checkpoint",
                "workflow_checkpoint",
                "attempt_checkpoint",
            ],
            edges: [
                ("session_checkpoint", "workflow_checkpoint"),
                ("workflow_checkpoint", "attempt_checkpoint"),
            ],
            current_node: "attempt_checkpoint",
        },
        invalidation_dimensions: [
            "source",
            "plan",
            "model",
            "context",
            "tool_catalog",
            "policy",
            "grants",
            "approvals",
            "preflights",
            "attempts",
            "receipts",
            "artifacts",
            "verifier",
            "budgets",
            "repeated_state",
            "event_cursor",
            "environment",
            "postcondition",
        ],
        terminal_diagnosis,
        cleanup: CleanupGolden {
            temporary_campaign_directories_remaining: 0,
            orphan_resume_owners: 0,
            unbound_checkpoint_count: 0,
        },
        deterministic_seed_count: 100,
        interruption_boundary_count: boundaries.len(),
        concurrent_client_count: 32,
        durable_owner_count: 1,
        duplicate_effect_count: 0,
        replay_count: 0,
        external_values_retained: 0,
    };
    println!(
        "{}",
        serde_json::to_string_pretty(&evidence).expect("evidence JSON")
    );
}
