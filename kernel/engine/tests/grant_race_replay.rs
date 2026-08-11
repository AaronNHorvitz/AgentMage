use std::{
    collections::BTreeSet,
    env,
    sync::{Arc, Barrier, Mutex},
    thread,
};

use agentmage_kernel_contracts::{
    ActionId, ActionKind, ActorId, ApprovalId, CapabilityGrant, DataSensitivity, GrantId,
    GrantNonce, GrantOperation, GrantPreimage, GrantSideEffect, GrantStatus, GrantTarget,
    OperationBinding, SessionId, TaskId, ToolId, WorkspaceId,
};
use agentmage_kernel_engine::{
    grants::{
        DerivedOperationGrantRequest, GrantConsumeError, GrantIssuer, SessionReadGrantRequest,
    },
    policy::{
        PolicyDenialScope, PolicyEngine, PolicyEvaluationContext, StrictLocalReadOnlyScope,
        ToolPolicyBinding,
    },
};
use serde::Serialize;

const EMIT_ENV: &str = "AGENTMAGE_EMIT_GRANT_RACE_REPLAY";
const MARKER: &str = "AGENTMAGE_GRANT_RACE_REPLAY=";

#[derive(Clone, Copy)]
enum Scenario {
    Success,
    Denial,
    Timeout,
    Crash,
    Uncertain,
}

impl Scenario {
    const ALL: [Self; 5] = [
        Self::Success,
        Self::Denial,
        Self::Timeout,
        Self::Crash,
        Self::Uncertain,
    ];

    const fn name(self) -> &'static str {
        match self {
            Self::Success => "success",
            Self::Denial => "denial",
            Self::Timeout => "timeout",
            Self::Crash => "crash",
            Self::Uncertain => "uncertain",
        }
    }

    const fn outcome(self) -> &'static str {
        match self {
            Self::Success => "succeeded",
            Self::Denial => "denied",
            Self::Timeout => "timed_out",
            Self::Crash => "failed",
            Self::Uncertain => "uncertain",
        }
    }

    const fn effect_count(self) -> u32 {
        match self {
            Self::Success | Self::Crash | Self::Uncertain => 1,
            Self::Denial | Self::Timeout => 0,
        }
    }

    const fn needs_uncertain_transition(self) -> bool {
        matches!(self, Self::Timeout | Self::Crash | Self::Uncertain)
    }
}

#[derive(Serialize)]
struct RaceReplayTrace {
    schema_version: u16,
    scenario: &'static str,
    result_outcome: &'static str,
    racing_consumer_count: u32,
    atomic_consumption_count: u32,
    race_denial_scopes: Vec<&'static str>,
    worker_start_count: u32,
    simulated_effect_count: u32,
    post_attempt_uncertain_transition_count: u32,
    terminal_status: &'static str,
    terminal_revision: u32,
    terminal_use_count: u32,
    replay_allowed: bool,
    replay_denial_scope: &'static str,
    effect_count_after_replay: u32,
}

struct Fixture {
    issuer: GrantIssuer,
    policy: PolicyEngine,
    grant: CapabilityGrant,
    context: PolicyEvaluationContext,
}

fn target(path: &[&str]) -> GrantTarget {
    GrantTarget {
        workspace_id: WorkspaceId::from_raw("workspace-0001"),
        path_components: path.iter().map(|value| (*value).to_owned()).collect(),
    }
}

fn fixture() -> Fixture {
    let actor_id = ActorId::from_raw("actor-local-0001");
    let session_id = SessionId::from_raw("session-0001");
    let task_id = TaskId::from_raw("task-0001");
    let action_id = ActionId::from_raw("action-0001");
    let tool_id = ToolId::from_raw("fixture.read");
    let operation_target = target(&["src", "fixture.txt"]);
    let policy = PolicyEngine::strict_local_read_only(StrictLocalReadOnlyScope {
        revision: 1,
        actors: BTreeSet::from([actor_id.clone()]),
        tasks: BTreeSet::from([task_id.clone()]),
        actions: BTreeSet::from([action_id.clone()]),
        tools: BTreeSet::from([ToolPolicyBinding {
            tool_id: tool_id.clone(),
            tool_version: "1.0.0".to_owned(),
        }]),
        targets: BTreeSet::from([operation_target.clone()]),
    })
    .expect("strict fixture policy must build");
    let mut issuer = GrantIssuer::new();
    let parent = issuer
        .issue_session_read(SessionReadGrantRequest {
            grant_id: GrantId::from_raw("grant-parent-0001"),
            actor_id: actor_id.clone(),
            session_id: session_id.clone(),
            task_id: task_id.clone(),
            targets: vec![target(&[])],
            excluded_targets: vec![target(&["private"])],
            sensitivity: DataSensitivity::Ephemeral,
            issued_at_epoch_ms: 1_000,
            expires_at_epoch_ms: 60_000,
            nonce: GrantNonce::from_raw("nonce-parent-0001"),
            maximum_derived_operations: 1,
            preview_sha256: "1".repeat(64),
            policy_sha256: policy.policy_sha256().to_owned(),
        })
        .expect("session parent must issue");
    let grant = issuer
        .derive_operation(
            &parent.grant_id,
            DerivedOperationGrantRequest {
                grant_id: GrantId::from_raw("grant-operation-0001"),
                approval_id: ApprovalId::from_raw("approval-0001"),
                action_id: action_id.clone(),
                action_kind: ActionKind::DeterministicTool,
                operation: OperationBinding::new(GrantOperation::WorkspaceRead),
                tool_id: tool_id.clone(),
                tool_version: "1.0.0".to_owned(),
                targets: vec![operation_target],
                argument_sha256: "2".repeat(64),
                preimages: vec![GrantPreimage {
                    target_index: 0,
                    content_sha256: "3".repeat(64),
                    observed_revision: Some("fixture-v1".to_owned()),
                }],
                expected_side_effects: vec![GrantSideEffect {
                    operation: OperationBinding::new(GrantOperation::WorkspaceRead),
                    target_indexes: vec![0],
                    details_sha256: "4".repeat(64),
                }],
                rollback_description: "No state change is permitted".to_owned(),
                issued_at_epoch_ms: 2_000,
                expires_at_epoch_ms: 30_000,
                nonce: GrantNonce::from_raw("nonce-operation-0001"),
                preview_sha256: "5".repeat(64),
                policy_sha256: policy.policy_sha256().to_owned(),
            },
        )
        .expect("operation grant must derive");
    let context = PolicyEvaluationContext {
        actor_id,
        session_id,
        task_id,
        action_id,
        action_kind: ActionKind::DeterministicTool,
        tool_id,
        tool_version: "1.0.0".to_owned(),
        targets: grant.targets.clone(),
        argument_sha256: grant.argument_sha256.clone(),
        preimages: grant.preimages.clone(),
        expected_side_effects: grant.expected_side_effects.clone(),
        preview_sha256: grant.preview_sha256.clone(),
        now_epoch_ms: 3_000,
        network_scope: None,
        credential_scope: None,
        publication_scope: None,
    };
    Fixture {
        issuer,
        policy,
        grant,
        context,
    }
}

fn scope_name(scope: PolicyDenialScope) -> &'static str {
    match scope {
        PolicyDenialScope::Grant => "grant",
        PolicyDenialScope::Actor => "actor",
        PolicyDenialScope::Session => "session",
        PolicyDenialScope::Task => "task",
        PolicyDenialScope::Action => "action",
        PolicyDenialScope::Tool => "tool",
        PolicyDenialScope::Operation => "operation",
        PolicyDenialScope::Path => "path",
        PolicyDenialScope::Argument => "argument",
        PolicyDenialScope::Preimage => "preimage",
        PolicyDenialScope::SideEffect => "side_effect",
        PolicyDenialScope::Preview => "preview",
        PolicyDenialScope::Network => "network",
        PolicyDenialScope::Credential => "credential",
        PolicyDenialScope::Publication => "publication",
    }
}

fn status_name(status: GrantStatus) -> &'static str {
    match status {
        GrantStatus::Issued => "issued",
        GrantStatus::Consumed => "consumed",
        GrantStatus::Revoked => "revoked",
        GrantStatus::Expired => "expired",
        GrantStatus::Invalidated => "invalidated",
        GrantStatus::Uncertain => "uncertain",
    }
}

fn run_scenario(scenario: Scenario) -> RaceReplayTrace {
    let fixture = fixture();
    let grant_id = fixture.grant.grant_id.clone();
    let replay_context = fixture.context.clone();
    let mut race_context = fixture.context;
    if matches!(scenario, Scenario::Denial) {
        race_context.argument_sha256 = "f".repeat(64);
    }
    let issuer = Arc::new(Mutex::new(fixture.issuer));
    let policy = Arc::new(fixture.policy);
    let barrier = Arc::new(Barrier::new(3));
    let handles = (0..2)
        .map(|_| {
            let issuer = Arc::clone(&issuer);
            let policy = Arc::clone(&policy);
            let barrier = Arc::clone(&barrier);
            let grant_id = grant_id.clone();
            let context = race_context.clone();
            thread::spawn(move || {
                barrier.wait();
                issuer
                    .lock()
                    .expect("issuer lock must not be poisoned")
                    .consume_for_execution(&grant_id, &policy, &context)
            })
        })
        .collect::<Vec<_>>();
    barrier.wait();
    let outcomes = handles
        .into_iter()
        .map(|handle| handle.join().expect("consumer must finish"))
        .collect::<Vec<_>>();
    let consumption_count =
        u32::try_from(outcomes.iter().filter(|outcome| outcome.is_ok()).count())
            .expect("consumer count is bounded");
    let mut denial_scopes = outcomes
        .iter()
        .filter_map(|outcome| match outcome {
            Err(GrantConsumeError::PolicyDenied(scope)) => Some(scope_name(*scope)),
            Err(other) => panic!("unexpected race error: {other:?}"),
            Ok(_) => None,
        })
        .collect::<Vec<_>>();
    denial_scopes.sort_unstable();
    if matches!(scenario, Scenario::Denial) {
        assert_eq!(consumption_count, 0);
        assert_eq!(denial_scopes, ["argument", "grant"]);
    } else {
        assert_eq!(consumption_count, 1);
        assert_eq!(denial_scopes, ["grant"]);
    }

    let worker_start_count = consumption_count;
    let simulated_effect_count = if consumption_count == 1 {
        scenario.effect_count()
    } else {
        0
    };
    let mut uncertain_transition_count = 0;
    let mut issuer = issuer.lock().expect("issuer lock must not be poisoned");
    if scenario.needs_uncertain_transition() {
        let consumed_hash = outcomes
            .iter()
            .find_map(|outcome| {
                outcome
                    .as_ref()
                    .ok()
                    .map(|record| record.consumed_grant_sha256.as_str())
            })
            .expect("attempt scenario must have one consumed hash");
        let transition = issuer
            .mark_execution_uncertain(&grant_id, consumed_hash, 4_000)
            .expect("unreconciled attempt must become uncertain");
        assert_eq!(transition.status, GrantStatus::Uncertain);
        uncertain_transition_count = 1;
    }
    assert_eq!(
        issuer.consume_for_execution(&grant_id, &policy, &replay_context),
        Err(GrantConsumeError::PolicyDenied(PolicyDenialScope::Grant))
    );
    let retained = issuer
        .current(&grant_id)
        .expect("terminal grant must remain");
    let expected_status = match scenario {
        Scenario::Success => GrantStatus::Consumed,
        Scenario::Denial => GrantStatus::Invalidated,
        Scenario::Timeout | Scenario::Crash | Scenario::Uncertain => GrantStatus::Uncertain,
    };
    assert_eq!(retained.status, expected_status);
    assert_eq!(retained.use_count, consumption_count);
    assert_eq!(
        retained.revision,
        if scenario.needs_uncertain_transition() {
            3
        } else {
            2
        }
    );
    RaceReplayTrace {
        schema_version: 1,
        scenario: scenario.name(),
        result_outcome: scenario.outcome(),
        racing_consumer_count: 2,
        atomic_consumption_count: consumption_count,
        race_denial_scopes: denial_scopes,
        worker_start_count,
        simulated_effect_count,
        post_attempt_uncertain_transition_count: uncertain_transition_count,
        terminal_status: status_name(retained.status),
        terminal_revision: retained.revision,
        terminal_use_count: retained.use_count,
        replay_allowed: false,
        replay_denial_scope: "grant",
        effect_count_after_replay: simulated_effect_count,
    }
}

#[test]
fn race_and_replay_preserve_exactly_once_admission_and_effect_bounds() {
    let traces = Scenario::ALL.map(run_scenario);
    if env::var(EMIT_ENV).as_deref() == Ok("1") {
        println!(
            "{MARKER}{}",
            serde_json::to_string(&traces).expect("race trace must serialize")
        );
    }
}
