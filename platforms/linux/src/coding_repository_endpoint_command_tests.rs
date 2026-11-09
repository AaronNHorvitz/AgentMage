// SPDX-License-Identifier: Apache-2.0
//! Synthetic creation and command observations; real durable grant transactions.
use super::super::tests::{Fixture, spec};
use super::*;
use agentmage_kernel_contracts::*;
use agentmage_kernel_engine::{
    command_runner::{
        CommandExecutionFailure, CommandLaunchPermit, CommandPlatformResult, CommandTermination,
    },
    runtime_coordinator::{runtime_tool_catalog_sha256, seal_runtime_endpoint_run_request},
    validation_template::ValidationKind,
};
use std::{
    fs,
    sync::{
        Mutex,
        atomic::{AtomicU64, AtomicUsize, Ordering},
    },
};
const NOW: u64 = 1000;
fn request(
    workspace: &LinuxAuthorizedWorkspace,
    registry: &ToolRegistry,
) -> RuntimeEndpointRunRequest {
    let task_id = TaskId::from_raw("endpoint-policy-task");
    let session_id = SessionId::from_raw("endpoint-policy-session");
    let objective = "Apply the prepared fixture edit";
    let check = "Ground observations in current source";
    let catalog_id = ToolCatalogId::from_raw("endpoint-policy-catalog");
    let visible_tools = runtime_tool_references(registry).unwrap();
    seal_runtime_endpoint_run_request(RuntimeEndpointRunRequest {
        schema_version: CONTRACT_SCHEMA_VERSION,
        run_id: RuntimeRunId::from_raw("endpoint-policy-run"),
        session_id: session_id.clone(),
        mode: RuntimeSessionMode::ControlledWrite,
        task: Task {
            schema_version: CONTRACT_SCHEMA_VERSION,
            task_id: task_id.clone(),
            session_id,
            objective: objective.into(),
            acceptance_criteria: vec![check.into()],
            constraints: vec!["Only the complete approved diff; no network".into()],
            status: TaskStatus::Ready,
        },
        work_packet: WorkPacket {
            schema_version: CONTRACT_SCHEMA_VERSION,
            work_packet_id: WorkPacketId::from_raw("endpoint-policy-packet"),
            task_id,
            revision: 1,
            objective: objective.into(),
            reason: "Exercise the kernel authority boundary".into(),
            owner: "synthetic-local-actor".into(),
            authoritative_evidence: vec![],
            mutable_files: vec!["one.py".into(), "two.py".into()],
            protected_files: vec!["private".into()],
            expected_output: "Grounded source observations".into(),
            acceptance_checks: vec![check.into()],
            required_evidence: vec![EvidenceKind::ToolOutput],
            required_capability_class: AuthorityClass::LocalWrite,
            budgets: vec![
                BudgetLimit {
                    resource: BudgetResource::ToolCallDepth,
                    limit: 1,
                },
                BudgetLimit {
                    resource: BudgetResource::InputBytes,
                    limit: 81920,
                },
                BudgetLimit {
                    resource: BudgetResource::OutputBytes,
                    limit: 4096,
                },
                BudgetLimit {
                    resource: BudgetResource::ElapsedMilliseconds,
                    limit: 60000,
                },
                BudgetLimit {
                    resource: BudgetResource::MemoryBytes,
                    limit: 16777216,
                },
                BudgetLimit {
                    resource: BudgetResource::DiskBytes,
                    limit: 4096,
                },
                BudgetLimit {
                    resource: BudgetResource::ProcessCount,
                    limit: 1,
                },
                BudgetLimit {
                    resource: BudgetResource::PlanSteps,
                    limit: 5,
                },
                BudgetLimit {
                    resource: BudgetResource::ModelCalls,
                    limit: 5,
                },
                BudgetLimit {
                    resource: BudgetResource::ToolCalls,
                    limit: 4,
                },
            ],
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
                description: format!("Stop for {kind:?}"),
            })
            .collect(),
            rollback: RollbackPlan {
                reversible: true,
                description: "No workspace changes".into(),
            },
            sensitivity: DataSensitivity::Ephemeral,
            last_verification_date: "2026-11-08".into(),
            next_action: None,
            next_review: None,
            status_reason: None,
            disposition: None,
            completion_evidence: vec![],
            superseding_work: None,
            validation_issues: vec![],
            plan_id: Some(PlanId::from_raw("endpoint-policy-plan")),
            state: WorkPacketState::Active,
        },
        workspace_id: workspace.workspace_id().clone(),
        workspace_snapshot_sha256: "a".repeat(64),
        repository_snapshot_id: RepositorySnapshotId::from_raw("endpoint-policy-snapshot"),
        repository_snapshot_sha256: "b".repeat(64),
        model_selection: EndpointModelSelection {
            endpoint: "http://127.0.0.1:1234/v1".into(),
            model: "synthetic-model".into(),
        },
        context_budget: RuntimeEndpointContextBudget {
            max_messages: 8,
            max_content_bytes: 4096,
            max_serialized_messages_bytes: 8192,
            max_request_body_bytes: 16384,
        },
        max_output_tokens: 128,
        tool_catalog_sha256: runtime_tool_catalog_sha256(&catalog_id, &visible_tools).unwrap(),
        tool_catalog_id: catalog_id,
        visible_tools,
        policy_id: PolicyId::from_raw("unbound-startup-policy"),
        policy_sha256: "c".repeat(64),
        limits: RuntimeRunLimits {
            max_turns: 5,
            max_model_calls: 5,
            max_tool_calls: 4,
            max_repeated_tool_calls: 2,
            max_tool_call_depth: 1,
            max_no_progress_turns: 2,
            max_context_refreshes: 5,
            max_events: 128,
            max_elapsed_ms: 60000,
            max_output_bytes: 4096,
        },
        event_cursor: None,
        request_sha256: "0".repeat(64),
    })
    .unwrap()
}
#[derive(Clone)]
struct Clock(Arc<AtomicU64>);
impl RuntimeClock for Clock {
    fn now_epoch_ms(&mut self) -> std::result::Result<u64, RuntimePortFailure> {
        Ok(self.0.load(Ordering::SeqCst))
    }
}
struct Control(Mutex<std::result::Result<std::num::NonZeroU64, ModelOperationStop>>);
impl Control {
    fn new() -> Self {
        Self(Mutex::new(Ok(std::num::NonZeroU64::new(60000).unwrap())))
    }
    fn stop(&self, stop: ModelOperationStop) {
        *self.0.lock().unwrap() = Err(stop);
    }
}
impl ModelOperationControl for Control {
    fn remaining_ms(&self) -> std::result::Result<std::num::NonZeroU64, ModelOperationStop> {
        *self.0.lock().unwrap()
    }
}
fn owner(f: &Fixture, validation: bool, prior: Option<&str>) -> LinuxCodingRepository {
    let mut owner = f.open();
    f.synthetic_created(&mut owner);
    prepare(&mut owner, validation, prior);
    owner
}
fn prepare(owner: &mut LinuxCodingRepository, validation: bool, prior: Option<&str>) {
    let registry = command_registry(spec()).unwrap();
    owner.command = Some(if validation {
        owner
            .validation_preview(
                &registry,
                ValidationKind::Unit,
                2,
                Default::default(),
                prior,
                NOW,
            )
            .unwrap()
    } else {
        owner.command_preview(&registry, NOW).unwrap()
    });
}
fn binding(owner: &LinuxCodingRepository) -> (RuntimeEndpointRunRequest, ToolCall) {
    let registry = linux_coding_endpoint_command_registry().unwrap();
    let mut req = request(&owner.created.as_ref().unwrap().held, &registry);
    req.work_packet.required_capability_class = AuthorityClass::Execute;
    req.limits.max_output_bytes = 65536;
    for b in &mut req.work_packet.budgets {
        if matches!(
            b.resource,
            BudgetResource::OutputBytes | BudgetResource::DiskBytes
        ) {
            b.limit = 65536;
        }
    }
    for b in &mut req.work_packet.budgets {
        if b.resource == BudgetResource::MemoryBytes {
            b.limit = spec().bounds.memory_bytes;
        }
        if b.resource == BudgetResource::ProcessCount {
            b.limit = u64::from(spec().bounds.task_count);
        }
    }
    let req = seal_runtime_endpoint_run_request(req).unwrap();
    let def = EndpointCommandTool::new().0;
    let bytes =
        serde_json::to_vec(&json!({"preview":owner.command.as_ref().unwrap().preview})).unwrap();
    let call = ToolCall {
        schema_version: CONTRACT_SCHEMA_VERSION,
        tool_call_id: ToolCallId::from_raw("endpoint-command-call"),
        correlation_id: CorrelationId::from_raw("endpoint-command-correlation"),
        action_id: runtime_action_id(&req.run_id, 1),
        tool_id: def.tool_id,
        tool_version: def.tool_version,
        arguments: ContractPayload {
            schema: def.input_schema,
            media_type: "application/json".into(),
            sha256: hash(&bytes),
            bytes,
        },
    };
    (req, call)
}
fn handoff(owner: LinuxCodingRepository) -> LinuxCodingEndpointCommand {
    let (req, call) = binding(&owner);
    owner
        .into_endpoint_command(
            req,
            RuntimeTurnId::from_raw("endpoint-command-turn"),
            RuntimeOperationId::from_raw("endpoint-command-operation"),
            call,
        )
        .ok()
        .unwrap()
}
fn boundary(owner: LinuxCodingRepository) -> LinuxCodingEndpointCommandBoundary<Clock> {
    handoff(owner).into_boundary(
        Arc::new(Control::new()),
        Clock(Arc::new(AtomicU64::new(NOW))),
    )
}
fn ask(b: &mut LinuxCodingEndpointCommandBoundary<Clock>) -> RuntimePermissionEvaluation {
    let c = b.command.challenge.clone();
    let call = b.command.call.clone();
    let req = b.command.request.clone();
    b.evaluate_scoped(
        RuntimeRunScope::endpoint(&req).unwrap(),
        &c.operation_id,
        &EndpointCommandTool::new().0,
        &call,
        NOW,
    )
    .unwrap()
}
fn response(b: &LinuxCodingEndpointCommandBoundary<Clock>, allow: bool) -> RuntimeApprovalResponse {
    let c = &b.command.challenge;
    RuntimeApprovalResponse {
        schema_version: CONTRACT_SCHEMA_VERSION,
        run_id: c.run_id.clone(),
        approval_id: c.approval_id.clone(),
        disposition: if allow {
            RuntimeApprovalDisposition::Allow
        } else {
            RuntimeApprovalDisposition::Deny
        },
        challenge_sha256: c.challenge_sha256.clone(),
        grant_id: allow.then(|| c.proposed_grant_id.clone()),
        selection: None,
    }
}
fn resolve(
    b: &mut LinuxCodingEndpointCommandBoundary<Clock>,
    allow: bool,
) -> std::result::Result<RuntimePermissionEvaluation, RuntimePortFailure> {
    let c = b.command.challenge.clone();
    let call = b.command.call.clone();
    let req = b.command.request.clone();
    let r = response(b, allow);
    b.resolve_using(
        RuntimeRunScope::endpoint(&req).unwrap(),
        (&c, &r, &EndpointCommandTool::new().0, &call),
        NOW,
        |_| Ok(()),
    )
}
struct Synthetic {
    calls: Arc<AtomicUsize>,
    stdout: Vec<u8>,
    exit: i32,
    stop: Option<(Arc<Control>, ModelOperationStop)>,
    panic: bool,
    corrupt: bool,
}
impl Synthetic {
    fn new(calls: &Arc<AtomicUsize>, pass: bool) -> Self {
        let stdout=serde_json::to_vec(&json!({"schema_version":1,"status":if pass {"passed"} else {"assertion_failed"},
            "passed":if pass {2} else {1},"failed":if pass {0} else {1},"skipped":0,"duration_ms":1,
            "failed_names":if pass {vec![]} else {vec!["synthetic.test"]},"artifact_ids":[],"retry_count":0,
            "initial_failure_sha256":null})).unwrap();
        Self {
            calls: calls.clone(),
            stdout,
            exit: i32::from(!pass),
            stop: None,
            panic: false,
            corrupt: false,
        }
    }
}
impl BoundedCommandExecutor for Synthetic {
    type WorkingDirectory = LinuxAuthorizedWorkspace;
    fn execute(
        &mut self,
        permit: CommandLaunchPermit<'_>,
        root: &LinuxAuthorizedWorkspace,
        cancel: &dyn EffectCancellationObservation,
    ) -> std::result::Result<CommandPlatformResult, CommandExecutionFailure> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        assert_eq!(permit.command(), &spec());
        assert!(root.root_path_sha256().len() == 64);
        assert!(!self.panic, "synthetic command unwind");
        let cancelled = if let Some((control, stop)) = &self.stop {
            control.stop(*stop);
            let signal = cancel.observe_effect_cancellation().unwrap().unwrap();
            assert!(signal.task_id.as_str().starts_with("command-"));
            assert_eq!(signal.task_id.as_str(), signal.correlation_id.as_str());
            true
        } else {
            assert!(cancel.observe_effect_cancellation().unwrap().is_none());
            false
        };
        Ok(CommandPlatformResult {
            termination: if cancelled {
                CommandTermination::Cancelled
            } else {
                CommandTermination::Exited
            },
            exit_code: if cancelled { None } else { Some(self.exit) },
            signal: None,
            stdout: self.stdout.clone(),
            stdout_sha256: if self.corrupt {
                hash(b"wrong")
            } else {
                hash(&self.stdout)
            },
            stdout_total_bytes: self.stdout.len() as u64,
            stderr: vec![],
            stderr_sha256: hash(b""),
            stderr_total_bytes: 0,
            elapsed_ms: 1,
            resource_usage: None,
            descendants_terminated: true,
            platform_code: "test.synthetic.endpoint-command".into(),
        })
    }
}
fn execute(
    b: &mut LinuxCodingEndpointCommandBoundary<Clock>,
    e: &RuntimePermissionEvaluation,
    executor: Synthetic,
    probe: Option<&dyn ModelCancellationProbe>,
) -> std::result::Result<RuntimeToolExecution, RuntimePortFailure> {
    let req = b.command.request.clone();
    let call = b.command.call.clone();
    b.execute_using(
        RuntimeRunScope::endpoint(&req).unwrap(),
        (e, &EndpointCommandTool::new().0, &call),
        probe,
        |_| Ok(executor),
    )
}
fn no_receipts(b: &LinuxCodingEndpointCommandBoundary<Clock>) {
    assert!(
        b.command.repository.inspect().unwrap()["receipts"]
            .as_array()
            .unwrap()
            .is_empty()
    );
}
#[test]
fn endpoint_command_approval_issues_original_grant_without_launch_then_consumes_it_once() {
    let f = Fixture::new();
    let calls = Arc::new(AtomicUsize::new(0));
    let mut b = boundary(owner(&f, false, None));
    ask(&mut b);
    no_receipts(&b);
    let e = resolve(&mut b, true).unwrap();
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    no_receipts(&b);
    let grant = &b
        .command
        .repository
        .command
        .as_ref()
        .unwrap()
        .issued
        .as_ref()
        .unwrap()
        .grant;
    let RuntimePermissionEvaluation::Allow {
        grant_id,
        authority_sha256,
        ..
    } = &e
    else {
        panic!("allow");
    };
    assert_eq!(grant_id, &grant.grant_id);
    assert_eq!(authority_sha256, &hash(&to_canonical_json(grant).unwrap()));
    let result = execute(&mut b, &e, Synthetic::new(&calls, true), None).unwrap();
    assert_eq!(result.result.outcome, OperationOutcome::Succeeded);
    assert_eq!(result.result.state_change, StateChange::NotChanged);
    let value: Value = serde_json::from_slice(&result.result.output.unwrap().bytes).unwrap();
    assert_eq!(value["receipt"]["grant_id"], grant_id.as_str());
    assert_eq!(value["receipt"]["receipt_id"], result.receipt_id.as_str());
    assert_eq!(value["receipt"]["receipt_sha256"], result.receipt_sha256);
    assert!(value["validation"].is_null());
    assert_eq!(value["test_suite_verified"], false);
    assert!(execute(&mut b, &e, Synthetic::new(&calls, true), None).is_err());
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert_eq!(
        fs::read(f.0.join("owned/worktree/sentinel")).unwrap(),
        b"preserve"
    );
    assert_eq!(
        fs::read(f.0.join("original/sentinel")).unwrap(),
        b"preserve"
    );
}
#[test]
fn endpoint_command_validation_failure_and_exact_fresh_rerun_use_original_parser_and_budget() {
    let f = Fixture::new();
    let calls = Arc::new(AtomicUsize::new(0));
    let mut b = boundary(owner(&f, true, None));
    ask(&mut b);
    let e = resolve(&mut b, true).unwrap();
    assert_eq!(b.inspect().unwrap()["validation"]["attempts"], 0);
    let failed = execute(&mut b, &e, Synthetic::new(&calls, false), None).unwrap();
    assert_eq!(failed.result.outcome, OperationOutcome::Failed);
    let failed: Value = serde_json::from_slice(&failed.result.output.unwrap().bytes).unwrap();
    assert_eq!(failed["validation"]["validation_passed"], false);
    assert_eq!(failed["validation"]["attempt"], 1);
    let prior = failed["validation"]["receipt"]["receipt_sha256"]
        .as_str()
        .unwrap();
    let mut owner = b.into_repository();
    assert!(owner.command.is_none());
    assert!(owner.repair_feedback().is_ok());
    prepare(&mut owner, true, Some(prior));
    let mut b = boundary(owner);
    ask(&mut b);
    let next = resolve(&mut b, true).unwrap();
    assert_ne!(e, next);
    let passed = execute(&mut b, &next, Synthetic::new(&calls, true), None).unwrap();
    let passed: Value = serde_json::from_slice(&passed.result.output.unwrap().bytes).unwrap();
    assert_eq!(passed["validation"]["validation_passed"], true);
    assert_eq!(passed["validation"]["attempt"], 2);
    assert_eq!(passed["rerun_plan"]["prior_receipt_sha256"], prior);
    assert_eq!(passed["test_suite_verified"], false);
    assert_eq!(passed["validation"]["current_workspace_verified"], false);
    assert_eq!(calls.load(Ordering::SeqCst), 2);
}
#[test]
fn endpoint_command_binding_refusals_return_original_live_owner_unchanged() {
    for case in 0..9 {
        let f = Fixture::new();
        let owner = owner(&f, false, None);
        let original = owner.inspect_command().unwrap();
        let (mut req, mut call) = binding(&owner);
        match case {
            0 => call.arguments.bytes.push(b' '),
            1 => call.arguments.sha256 = hash(b"wrong"),
            2 => call.tool_version = "2.0.0".into(),
            3 => call.action_id = ActionId::from_raw("foreign-action"),
            4 => call.arguments.schema.schema_sha256 = hash(b"wrong-schema"),
            5 => call.arguments.media_type = "text/plain".into(),
            6 => {
                req.workspace_id = WorkspaceId::from_raw("foreign-workspace");
                req = seal_runtime_endpoint_run_request(req).unwrap();
            }
            7 => req.request_sha256 = hash(b"wrong"),
            8 => call.schema_version = CONTRACT_SCHEMA_VERSION + 1,
            _ => unreachable!(),
        }
        let (owner, error) = owner
            .into_endpoint_command(
                req,
                RuntimeTurnId::from_raw("turn"),
                RuntimeOperationId::from_raw("op"),
                call,
            )
            .err()
            .unwrap();
        assert_eq!(error.code(), "code.endpoint-command.binding", "case {case}");
        assert_eq!(owner.inspect_command().unwrap(), original);
        assert!(
            owner.inspect().unwrap()["receipts"]
                .as_array()
                .unwrap()
                .is_empty()
        );
    }
}
#[test]
fn endpoint_command_substitutions_are_inert_then_original_denial_spends_once() {
    let f = Fixture::new();
    let mut b = boundary(owner(&f, true, None));
    let req = b.command.request.clone();
    let def = EndpointCommandTool::new().0;
    let call = b.command.call.clone();
    let c = b.command.challenge.clone();
    let mut foreign = call.clone();
    foreign.tool_call_id = ToolCallId::from_raw("foreign");
    assert!(
        b.evaluate_scoped(
            RuntimeRunScope::endpoint(&req).unwrap(),
            &c.operation_id,
            &def,
            &foreign,
            NOW
        )
        .is_err()
    );
    ask(&mut b);
    for case in 0..8 {
        let mut r = response(&b, true);
        let mut changed = c.clone();
        let mut d = def.clone();
        let mut q = req.clone();
        match case {
            0 => r.run_id = RuntimeRunId::from_raw("foreign"),
            1 => r.grant_id = Some(GrantId::from_raw("foreign")),
            2 => r.approval_id = ApprovalId::from_raw("foreign"),
            3 => r.challenge_sha256 = hash(b"wrong"),
            4 => changed.operation_id = RuntimeOperationId::from_raw("foreign"),
            5 => d.required_grant.target_scope = "foreign".into(),
            6 => q.policy_sha256 = hash(b"foreign"),
            7 => r.schema_version = CONTRACT_SCHEMA_VERSION + 1,
            _ => unreachable!(),
        }
        if case == 6 {
            q = seal_runtime_endpoint_run_request(q).unwrap();
        }
        assert!(
            b.resolve_using(
                RuntimeRunScope::endpoint(&q).unwrap(),
                (&changed, &r, &d, &call),
                NOW,
                |_| panic!("substitution reached preflight")
            )
            .is_err()
        );
        assert!(matches!(b.stage, Stage::Asked));
        no_receipts(&b);
    }
    let deny = resolve(&mut b, false).unwrap();
    assert!(matches!(deny, RuntimePermissionEvaluation::Deny { .. }));
    assert!(b.command.repository.command.is_none());
    assert_eq!(b.inspect().unwrap()["validation"]["attempts"], 0);
    assert!(resolve(&mut b, true).is_err());
    no_receipts(&b);
}
#[test]
fn endpoint_command_invalid_issued_evaluation_cannot_consume_the_original_grant() {
    let f = Fixture::new();
    let calls = Arc::new(AtomicUsize::new(0));
    let mut b = boundary(owner(&f, false, None));
    ask(&mut b);
    let e = resolve(&mut b, true).unwrap();
    let mut changed = e.clone();
    if let RuntimePermissionEvaluation::Allow {
        authority_sha256, ..
    } = &mut changed
    {
        *authority_sha256 = hash(b"wrong");
    }
    assert!(execute(&mut b, &changed, Synthetic::new(&calls, true), None).is_err());
    no_receipts(&b);
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    execute(&mut b, &e, Synthetic::new(&calls, true), None).unwrap();
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}
#[test]
fn endpoint_command_original_stops_retire_each_matching_stage_before_dependencies() {
    for stage in 0..3 {
        for stop in [
            ModelOperationStop::Cancelled,
            ModelOperationStop::TimedOut,
            ModelOperationStop::Unavailable,
            ModelOperationStop::Invalid,
        ] {
            let f = Fixture::new();
            let calls = Arc::new(AtomicUsize::new(0));
            let control = Arc::new(Control::new());
            let mut b = handoff(owner(&f, true, None))
                .into_boundary(control.clone(), Clock(Arc::new(AtomicU64::new(NOW))));
            let mut e = None;
            if stage > 0 {
                ask(&mut b);
            }
            if stage > 1 {
                e = Some(resolve(&mut b, true).unwrap());
            }
            control.stop(stop);
            let result = match stage {
                0 => {
                    let req = b.command.request.clone();
                    let c = b.command.challenge.clone();
                    let call = b.command.call.clone();
                    b.evaluate_scoped(
                        RuntimeRunScope::endpoint(&req).unwrap(),
                        &c.operation_id,
                        &EndpointCommandTool::new().0,
                        &call,
                        NOW,
                    )
                    .map(|_| ())
                }
                1 => resolve(&mut b, true).map(|_| ()),
                _ => execute(
                    &mut b,
                    e.as_ref().unwrap(),
                    Synthetic::new(&calls, true),
                    None,
                )
                .map(|_| ()),
            };
            assert!(result.is_err());
            assert!(matches!(b.stage, Stage::Spent));
            assert!(resolve(&mut b, true).is_err());
            assert_eq!(calls.load(Ordering::SeqCst), 0);
            no_receipts(&b);
            assert_eq!(b.inspect().unwrap()["validation"]["attempts"], 0);
        }
    }
}
#[test]
fn endpoint_command_expiry_clock_and_held_root_drift_refuse_original_dispatch() {
    for case in 0..5 {
        let f = Fixture::new();
        let calls = Arc::new(AtomicUsize::new(0));
        let mut b = boundary(owner(&f, false, None));
        ask(&mut b);
        let e = resolve(&mut b, true).unwrap();
        match case {
            0 => b.clock.0.store(NOW - 1, Ordering::SeqCst),
            1 => b.clock.0.store(NOW + LIFETIME_MS, Ordering::SeqCst),
            2 => b.command.repository.command.as_mut().unwrap().deadline = Instant::now(),
            3 => {
                b.command
                    .repository
                    .created
                    .as_mut()
                    .unwrap()
                    .creation_receipt_sha256 = hash(b"foreign-creation")
            }
            4 => {
                fs::rename(f.0.join("owned/worktree"), f.0.join("owned/old-worktree")).unwrap();
                fs::create_dir(f.0.join("owned/worktree")).unwrap();
            }
            _ => unreachable!(),
        }
        assert!(
            execute(&mut b, &e, Synthetic::new(&calls, true), None).is_err(),
            "case {case}"
        );
        assert_eq!(calls.load(Ordering::SeqCst), 0);
        assert!(matches!(b.stage, Stage::Spent));
        no_receipts(&b);
    }
}
struct Probe {
    request: RuntimeEndpointRunRequest,
    call: ToolCall,
    kind: u8,
}
impl ModelCancellationProbe for Probe {
    fn observe(&self) -> std::result::Result<Option<CancellationSignal>, ModelRuntimeFailure> {
        match self.kind {
            0 => Ok(None),
            1 | 2 => Ok(Some(CancellationSignal {
                schema_version: CONTRACT_SCHEMA_VERSION,
                cancellation_id: CancellationId::from_raw("cancel-run"),
                task_id: if self.kind == 1 {
                    self.request.task.task_id.clone()
                } else {
                    TaskId::from_raw("foreign")
                },
                correlation_id: self.call.correlation_id.clone(),
                reason: CancellationReason::UserRequested,
                requested_by: BoundaryKind::Shell,
            })),
            3 => Err(ModelRuntimeFailure {
                code: "fixture.unavailable".into(),
                retryable_after_correction: false,
                dependency_recovery_required: true,
                contract_error: None,
            }),
            _ => panic!("synthetic probe unwind"),
        }
    }
}
#[test]
fn endpoint_command_cancellation_scopes_errors_and_unwind_never_reopen_dispatch() {
    for kind in 0..5 {
        let f = Fixture::new();
        let calls = Arc::new(AtomicUsize::new(0));
        let mut b = boundary(owner(&f, false, None));
        ask(&mut b);
        let e = resolve(&mut b, true).unwrap();
        let probe = Probe {
            request: b.command.request.clone(),
            call: b.command.call.clone(),
            kind,
        };
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            execute(&mut b, &e, Synthetic::new(&calls, true), Some(&probe))
        }));
        if kind == 0 {
            assert!(result.unwrap().is_ok());
            assert_eq!(calls.load(Ordering::SeqCst), 1);
        } else {
            if kind == 4 {
                assert!(result.is_err());
            } else {
                assert!(result.unwrap().is_err());
            }
            assert_eq!(calls.load(Ordering::SeqCst), 0);
            no_receipts(&b);
        }
        assert!(matches!(b.stage, Stage::Spent));
        assert!(execute(&mut b, &e, Synthetic::new(&calls, true), None).is_err());
    }
}
#[test]
fn endpoint_command_supervisor_observes_original_control_during_execution() {
    for stop in [ModelOperationStop::Cancelled, ModelOperationStop::TimedOut] {
        let f = Fixture::new();
        let calls = Arc::new(AtomicUsize::new(0));
        let control = Arc::new(Control::new());
        let mut b = handoff(owner(&f, true, None))
            .into_boundary(control.clone(), Clock(Arc::new(AtomicU64::new(NOW))));
        ask(&mut b);
        let e = resolve(&mut b, true).unwrap();
        let mut runner = Synthetic::new(&calls, true);
        runner.stop = Some((control, stop));
        let result = execute(&mut b, &e, runner, None).unwrap();
        assert_eq!(result.result.outcome, OperationOutcome::Cancelled);
        let value: Value = serde_json::from_slice(&result.result.output.unwrap().bytes).unwrap();
        assert_eq!(value["validation"]["validation_passed"], false);
        assert_eq!(value["validation"]["attempt"], 1);
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert!(execute(&mut b, &e, Synthetic::new(&calls, true), None).is_err());
    }
}
#[test]
fn endpoint_command_postdispatch_output_refusal_retains_original_result_and_no_replay() {
    let f = Fixture::new();
    let calls = Arc::new(AtomicUsize::new(0));
    let mut original = handoff(owner(&f, true, None));
    original.request.limits.max_output_bytes = 4096;
    for budget in &mut original.request.work_packet.budgets {
        if matches!(
            budget.resource,
            BudgetResource::OutputBytes | BudgetResource::DiskBytes
        ) {
            budget.limit = 4096;
        }
    }
    original.request = seal_runtime_endpoint_run_request(original.request).unwrap();
    let mut b = original.into_boundary(
        Arc::new(Control::new()),
        Clock(Arc::new(AtomicU64::new(NOW))),
    );
    ask(&mut b);
    let e = resolve(&mut b, true).unwrap();
    let mut runner = Synthetic::new(&calls, true);
    runner.stdout.extend(vec![b' '; 700]);
    assert!(matches!(
        execute(&mut b, &e, runner, None),
        Err(RuntimePortFailure::Uncertain)
    ));
    assert!(b.inspect().unwrap()["last_result"].is_object());
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert!(execute(&mut b, &e, Synthetic::new(&calls, true), None).is_err());
    let owner = b.into_repository();
    assert!(owner.command.is_none());
    assert_eq!(owner.inspect_validation().unwrap()["attempts"], 1);
}
#[test]
fn endpoint_command_native_admission_refusal_and_abandoned_grant_are_not_replay_authority() {
    let f = Fixture::new();
    let mut b = boundary(owner(&f, false, None));
    ask(&mut b);
    let req = b.command.request.clone();
    let c = b.command.challenge.clone();
    let call = b.command.call.clone();
    let r = response(&b, true);
    let native = native_executor(&command_registry(spec()).unwrap()).is_ok();
    let result = b.resolve_scoped(
        RuntimeRunScope::endpoint(&req).unwrap(),
        &c,
        &r,
        &EndpointCommandTool::new().0,
        &call,
        NOW,
    );
    assert_eq!(result.is_ok(), native);
    no_receipts(&b);
    if !native {
        assert!(matches!(result, Err(RuntimePortFailure::Unavailable)));
        assert!(matches!(b.stage, Stage::Spent));
    }
    let owner = b.into_repository();
    assert!(owner.command.is_none());
    assert!(owner.command_cancellation(&c.preview_sha256).is_err());
    assert!(
        owner.inspect().unwrap()["receipts"]
            .as_array()
            .unwrap()
            .is_empty()
    );
}
#[test]
fn endpoint_command_dependency_unwind_and_corrupt_output_stay_spent() {
    for case in 0..3 {
        let f = Fixture::new();
        let calls = Arc::new(AtomicUsize::new(0));
        let mut b = boundary(owner(&f, false, None));
        ask(&mut b);
        if case == 0 {
            let req = b.command.request.clone();
            let c = b.command.challenge.clone();
            let call = b.command.call.clone();
            let r = response(&b, true);
            assert!(
                std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| b.resolve_using(
                    RuntimeRunScope::endpoint(&req).unwrap(),
                    (&c, &r, &EndpointCommandTool::new().0, &call),
                    NOW,
                    |_| panic!("synthetic preflight unwind")
                )))
                .is_err()
            );
            no_receipts(&b);
        } else {
            let e = resolve(&mut b, true).unwrap();
            let mut runner = Synthetic::new(&calls, true);
            runner.panic = case == 1;
            runner.corrupt = case == 2;
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                execute(&mut b, &e, runner, None)
            }));
            if case == 1 {
                assert!(result.is_err());
            } else {
                assert!(matches!(
                    result.unwrap(),
                    Err(RuntimePortFailure::Uncertain)
                ));
            }
            assert_eq!(calls.load(Ordering::SeqCst), 1);
            assert!(execute(&mut b, &e, Synthetic::new(&calls, true), None).is_err());
        }
        assert!(matches!(b.stage, Stage::Spent));
        assert!(resolve(&mut b, true).is_err());
    }
}

use agentmage_kernel_engine::runtime_loop::{
    ReusableRuntimeCoordinator, RuntimeEndpointCoordinatorStep, RuntimeEndpointPreparationInput,
    RuntimeEndpointTurnPort, RuntimePreparedEndpointTurn, RuntimeVerificationInput,
    RuntimeVerifierPort,
    endpoint::{
        RuntimeEndpointContextPacket, RuntimeEndpointMessage, RuntimeEndpointMessageRole,
        RuntimeEndpointModelPort,
    },
};
struct SyntheticPort {
    selection: EndpointModelSelection,
    arguments: Value,
    calls: Arc<AtomicU64>,
    preparations: Arc<AtomicU64>,
}
struct Dispatch {
    request: EndpointModelRequest,
    text: String,
    calls: Arc<AtomicU64>,
}
impl RuntimeEndpointModelPort for Dispatch {
    type Error = RuntimePortFailure;
    fn request(&self) -> &EndpointModelRequest {
        &self.request
    }
    fn dispatch(
        self,
        control: &dyn ModelOperationControl,
        emit: &mut dyn FnMut(&str),
    ) -> std::result::Result<EndpointModelResult, RuntimePortFailure> {
        control
            .remaining_ms()
            .map_err(|_| RuntimePortFailure::Cancelled)?;
        self.calls.fetch_add(1, Ordering::SeqCst);
        emit(&self.text);
        Ok(EndpointModelResult::from_completed_response(
            self.request,
            EndpointModelFinish::Stop,
            hash(self.text.as_bytes()),
            self.text.len(),
            Duration::from_millis(1),
            None,
        ))
    }
}
impl RuntimeEndpointTurnPort for SyntheticPort {
    fn model_binding(&self) -> RuntimeModelBinding<'_> {
        RuntimeModelBinding::SelectedEndpoint {
            endpoint: &self.selection.endpoint,
            model: &self.selection.model,
        }
    }
    fn prepare<'a>(
        &mut self,
        input: RuntimeEndpointPreparationInput<'_>,
        control: &dyn ModelOperationControl,
    ) -> std::result::Result<RuntimePreparedEndpointTurn<'a>, RuntimePortFailure>
    where
        Self: 'a,
    {
        self.preparations.fetch_add(1, Ordering::SeqCst);
        control
            .remaining_ms()
            .map_err(|_| RuntimePortFailure::Cancelled)?;
        if input.turn != 1 {
            assert_eq!(input.tool_results.len(), 1);
            assert_eq!(input.tool_results[0].outcome, OperationOutcome::Succeeded);
            assert_eq!(input.tool_results[0].state_change, StateChange::NotChanged);
            assert_eq!(input.completed_tool_calls.len(), 1);
            // Deliberate upstream refusal after observing the synthetic command transaction.
            // This does not manufacture task completion or a coding score.
            return Err(RuntimePortFailure::Unavailable);
        }
        let context = RuntimeEndpointContextPacket::from_messages(&[RuntimeEndpointMessage {
            role: RuntimeEndpointMessageRole::User,
            content: "Run only the separately prepared synthetic command",
        }])?;
        let request = EndpointModelRequest::from_prepared_body(
            self.selection.clone(),
            context.messages().clone(),
            EndpointModelBody {
                request_body_sha256: hash(b"synthetic boundary transport"),
                request_body_bytes: 400,
            },
            EndpointModelLimits {
                requested_max_output_tokens: 128,
                provider_usage_requested: false,
                total_timeout: Duration::from_millis(1000),
                idle_timeout: Duration::from_millis(500),
                response_text_limit_bytes: 16384,
                response_body_limit_bytes: 32768,
                response_wire_limit_bytes: 65536,
            },
        );
        let text=json!({"tool_call":{"tool_id":"agentmage.code.run-prepared-command","tool_version":"1.0.0","arguments":self.arguments}}).to_string();
        Ok(RuntimePreparedEndpointTurn::new(
            context,
            Dispatch {
                request,
                text,
                calls: self.calls.clone(),
            },
            EndpointCommandTool::new().0.output_schema,
        ))
    }
}
struct NoCompletion(VerifierId);
impl RuntimeVerifierPort for NoCompletion {
    fn verifier_id(&self) -> &VerifierId {
        &self.0
    }
    fn verify(
        &mut self,
        _: RuntimeVerificationInput<'_>,
    ) -> std::result::Result<VerifierCandidate, RuntimePortFailure> {
        panic!("native completion must not be called")
    }
}
fn derived(prefix: &str, req: &RuntimeEndpointRunRequest, seq: u64) -> String {
    format!(
        "{prefix}:{}",
        &hash(format!("{}:{seq}", req.run_id.as_str()).as_bytes())[..32]
    )
}
// Explicitly test-only observation seam; public adapter entry points never use it.
struct SyntheticBoundary {
    inner: LinuxCodingEndpointCommandBoundary<Clock>,
    calls: Arc<AtomicUsize>,
}
impl RuntimeToolBoundary for SyntheticBoundary {
    fn evaluate_scoped(
        &mut self,
        scope: RuntimeRunScope<'_>,
        op: &RuntimeOperationId,
        def: &ToolDefinition,
        call: &ToolCall,
        now: u64,
    ) -> std::result::Result<RuntimePermissionEvaluation, RuntimePortFailure> {
        self.inner.evaluate_scoped(scope, op, def, call, now)
    }
    fn resolve_scoped(
        &mut self,
        scope: RuntimeRunScope<'_>,
        c: &RuntimeApprovalChallenge,
        r: &RuntimeApprovalResponse,
        def: &ToolDefinition,
        call: &ToolCall,
        now: u64,
    ) -> std::result::Result<RuntimePermissionEvaluation, RuntimePortFailure> {
        self.inner
            .resolve_using(scope, (c, r, def, call), now, |_| Ok(()))
    }
    fn execute_scoped(
        &mut self,
        scope: RuntimeRunScope<'_>,
        e: &RuntimePermissionEvaluation,
        def: &ToolDefinition,
        call: &ToolCall,
        cancel: Option<&dyn ModelCancellationProbe>,
    ) -> std::result::Result<RuntimeToolExecution, RuntimePortFailure> {
        let runner = Synthetic::new(&self.calls, true);
        self.inner
            .execute_using(scope, (e, def, call), cancel, |_| Ok(runner))
    }
    fn evaluate(
        &mut self,
        q: &RuntimeRunRequest,
        op: &RuntimeOperationId,
        d: &ToolDefinition,
        c: &ToolCall,
        n: u64,
    ) -> std::result::Result<RuntimePermissionEvaluation, RuntimePortFailure> {
        self.inner.evaluate(q, op, d, c, n)
    }
    fn resolve(
        &mut self,
        q: &RuntimeRunRequest,
        c: &RuntimeApprovalChallenge,
        r: &RuntimeApprovalResponse,
        d: &ToolDefinition,
        call: &ToolCall,
        n: u64,
    ) -> std::result::Result<RuntimePermissionEvaluation, RuntimePortFailure> {
        self.inner.resolve(q, c, r, d, call, n)
    }
    fn execute(
        &mut self,
        q: &RuntimeRunRequest,
        e: &RuntimePermissionEvaluation,
        d: &ToolDefinition,
        c: &ToolCall,
        p: Option<&dyn ModelCancellationProbe>,
    ) -> std::result::Result<RuntimeToolExecution, RuntimePortFailure> {
        self.inner.execute(q, e, d, c, p)
    }
}
#[test]
fn endpoint_command_actual_coordinator_uses_exact_challenge_and_original_command_receipt() {
    for allow in [false, true] {
        let f = Fixture::new();
        let owner = owner(&f, true, None);
        let (req, mut call) = binding(&owner);
        call.tool_call_id = ToolCallId::from_raw(derived("endpoint-call", &req, 1));
        call.correlation_id = CorrelationId::from_raw(derived("correlation", &req, 0));
        let handoff = owner
            .into_endpoint_command(
                req.clone(),
                RuntimeTurnId::from_raw(derived("turn", &req, 1)),
                RuntimeOperationId::from_raw(derived("operation", &req, 1)),
                call.clone(),
            )
            .ok()
            .unwrap();
        let c = handoff.challenge.clone();
        let control = Arc::new(Control::new());
        let inner = handoff.into_boundary(control, Clock(Arc::new(AtomicU64::new(NOW))));
        let r = response(&inner, allow);
        let calls = Arc::new(AtomicUsize::new(0));
        let boundary = SyntheticBoundary {
            inner,
            calls: calls.clone(),
        };
        let models = Arc::new(AtomicU64::new(0));
        let preparations = Arc::new(AtomicU64::new(0));
        let port = SyntheticPort {
            selection: req.model_selection.clone(),
            arguments: serde_json::from_slice(&call.arguments.bytes).unwrap(),
            calls: models.clone(),
            preparations: preparations.clone(),
        };
        let mut coordinator = ReusableRuntimeCoordinator::new_endpoint(
            req,
            port,
            linux_coding_endpoint_command_registry().unwrap(),
            boundary,
            NoCompletion(VerifierId::from_raw("fixture-no-completion")),
            Clock(Arc::new(AtomicU64::new(NOW))),
        )
        .unwrap();
        let RuntimeEndpointCoordinatorStep::AwaitingApproval { challenge: actual } =
            coordinator.run_endpoint_until_boundary(None, None).unwrap()
        else {
            panic!("original challenge");
        };
        assert_eq!(actual, c);
        assert_eq!(calls.load(Ordering::SeqCst), 0);
        let RuntimeEndpointCoordinatorStep::Complete { outcome } = coordinator
            .run_endpoint_until_boundary(Some(&r), None)
            .unwrap()
        else {
            panic!("terminal");
        };
        assert!(!outcome.outcome.state.is_success());
        assert_eq!(outcome.outcome.receipt_ids.len(), usize::from(allow));
        assert_eq!(calls.load(Ordering::SeqCst), usize::from(allow));
        assert_eq!(models.load(Ordering::SeqCst), 1);
        assert_eq!(
            preparations.load(Ordering::SeqCst),
            if allow { 2 } else { 1 }
        );
        assert_eq!(
            coordinator.run_endpoint_until_boundary(None, None).unwrap(),
            RuntimeEndpointCoordinatorStep::Complete { outcome }
        );
        assert_eq!(calls.load(Ordering::SeqCst), usize::from(allow));
    }
}
#[test]
fn endpoint_command_sealed_run_cannot_understate_command_resource_ceilings() {
    for case in 0..3 {
        let f = Fixture::new();
        let owner = owner(&f, false, None);
        let (mut req, call) = binding(&owner);
        match case {
            0 => {
                for b in &mut req.work_packet.budgets {
                    if b.resource == BudgetResource::MemoryBytes {
                        b.limit = spec().bounds.memory_bytes - 1;
                    }
                }
            }
            1 => {
                for b in &mut req.work_packet.budgets {
                    if b.resource == BudgetResource::ProcessCount {
                        b.limit = u64::from(spec().bounds.task_count) - 1;
                    }
                }
            }
            2 => {
                req.limits.max_elapsed_ms = spec().bounds.timeout_ms - 1;
                for b in &mut req.work_packet.budgets {
                    if b.resource == BudgetResource::ElapsedMilliseconds {
                        b.limit = req.limits.max_elapsed_ms;
                    }
                }
            }
            _ => unreachable!(),
        }
        let req = seal_runtime_endpoint_run_request(req).unwrap();
        let (owner, e) = owner
            .into_endpoint_command(
                req,
                RuntimeTurnId::from_raw("turn"),
                RuntimeOperationId::from_raw("op"),
                call,
            )
            .err()
            .unwrap();
        assert_eq!(e.code(), "code.endpoint-command.bounds");
        assert!(owner.command.as_ref().unwrap().issued.is_none());
    }
}
#[test]
fn endpoint_command_live_bridge_preserves_deadline_reason_and_rejects_late_foreign_signals() {
    for stop in [
        ModelOperationStop::Cancelled,
        ModelOperationStop::TimedOut,
        ModelOperationStop::Unavailable,
        ModelOperationStop::Invalid,
    ] {
        let f = Fixture::new();
        let owner = owner(&f, false, None);
        let original = handoff(owner);
        let control = Control::new();
        let signal = original
            .repository
            .command_cancellation(&original.challenge.preview_sha256)
            .unwrap();
        let bridge = CommandControl {
            control: &control,
            probe: None,
            task: &original.request.task.task_id,
            correlation: &original.call.correlation_id,
            signal,
        };
        assert!(bridge.observe_effect_cancellation().unwrap().is_none());
        control.stop(stop);
        let observed = bridge.observe_effect_cancellation();
        match stop {
            ModelOperationStop::Cancelled | ModelOperationStop::TimedOut => {
                let signal = observed.unwrap().unwrap();
                assert_eq!(
                    signal.reason,
                    if stop == ModelOperationStop::TimedOut {
                        CancellationReason::DeadlineReached
                    } else {
                        CancellationReason::UserRequested
                    }
                );
                assert_eq!(
                    signal.task_id.as_str(),
                    original
                        .repository
                        .command
                        .as_ref()
                        .unwrap()
                        .prepared
                        .preview()
                        .command_attempt_id
                );
            }
            ModelOperationStop::Unavailable => {
                assert_eq!(observed, Err(CancellationObservationError::Unavailable))
            }
            ModelOperationStop::Invalid => {
                assert_eq!(observed, Err(CancellationObservationError::InvalidSignal))
            }
        }
    }
    struct LateProbe {
        polls: AtomicUsize,
        signal: CancellationSignal,
    }
    impl ModelCancellationProbe for LateProbe {
        fn observe(&self) -> std::result::Result<Option<CancellationSignal>, ModelRuntimeFailure> {
            Ok((self.polls.fetch_add(1, Ordering::SeqCst) > 0).then(|| self.signal.clone()))
        }
    }
    for foreign in [false, true] {
        let f = Fixture::new();
        let original = handoff(owner(&f, false, None));
        let control = Control::new();
        let signal = original
            .repository
            .command_cancellation(&original.challenge.preview_sha256)
            .unwrap();
        let probe = LateProbe {
            polls: AtomicUsize::new(0),
            signal: CancellationSignal {
                schema_version: CONTRACT_SCHEMA_VERSION,
                cancellation_id: CancellationId::from_raw("late-cancel"),
                task_id: if foreign {
                    TaskId::from_raw("foreign")
                } else {
                    original.request.task.task_id.clone()
                },
                correlation_id: original.call.correlation_id.clone(),
                reason: CancellationReason::Shutdown,
                requested_by: BoundaryKind::Shell,
            },
        };
        let bridge = CommandControl {
            control: &control,
            probe: Some(&probe),
            task: &original.request.task.task_id,
            correlation: &original.call.correlation_id,
            signal,
        };
        assert!(bridge.observe_effect_cancellation().unwrap().is_none());
        let next = bridge.observe_effect_cancellation();
        if foreign {
            assert_eq!(next, Err(CancellationObservationError::ScopeMismatch));
        } else {
            assert_eq!(next.unwrap().unwrap().reason, CancellationReason::Shutdown);
        }
    }
}

#[test]
fn endpoint_command_abandon_original_prepared_handle_returns_same_unissued_owner() {
    for validation in [false, true] {
        let f = Fixture::new();
        let owner = owner(&f, validation, None);
        let before = owner.inspect().unwrap();
        let prepared = handoff(owner);
        let owner = prepared.into_repository();
        assert_eq!(owner.inspect().unwrap(), before);
        assert!(owner.inspect_command().unwrap()["pending"].is_null());
        assert!(owner.inspect_validation().unwrap()["pending"].is_null());
        owner.verify_created().unwrap();
        assert_eq!(
            fs::read(f.0.join("owned/worktree/sentinel")).unwrap(),
            b"preserve"
        );
    }
}

#[test]
fn endpoint_command_original_handles_share_one_signal_without_explicit_direct_attachment() {
    for validation in [false, true] {
        let f = Fixture::new();
        let mut owner = owner(&f, validation, None);
        let digest = owner.command.as_ref().unwrap().hash.clone();
        let first = owner.command_cancellation(&digest).unwrap();
        let second = if validation {
            owner.validation_cancellation(&digest).unwrap()
        } else {
            owner.command_cancellation(&digest).unwrap()
        };
        assert!(second.observe_effect_cancellation().unwrap().is_none());
        first.shutdown().unwrap();
        second.cancel().unwrap();
        for handle in [&first, &second] {
            let signal = handle.observe_effect_cancellation().unwrap().unwrap();
            assert_eq!(signal.reason, CancellationReason::Shutdown);
            assert_eq!(signal.task_id.as_str(), first.attempt_id);
            assert_eq!(signal.correlation_id.as_str(), first.attempt_id);
        }
        let calls = Arc::new(AtomicUsize::new(0));
        let pending = owner.take_command(&digest, true, NOW).unwrap().unwrap();
        // Same default observer used by direct decisions with no explicit handle.
        let result = owner
            .run_command(pending, Synthetic::new(&calls, true), NOW)
            .unwrap();
        assert_eq!(calls.load(Ordering::SeqCst), 0);
        assert_eq!(result["receipt"]["outcome"], "cancelled");
        assert_eq!(result["command_receipt"]["termination"], "cancelled");
        if validation {
            assert_eq!(result["validation"]["validation_passed"], false);
            assert_eq!(owner.inspect_validation().unwrap()["attempts"], 1);
        }
        assert!(owner.command_cancellation(&digest).is_err());
    }
}

#[test]
fn endpoint_command_original_handle_cancels_after_handoff_before_native_preflight() {
    for validation in [false, true] {
        for shutdown in [false, true] {
            let f = Fixture::new();
            let owner = owner(&f, validation, None);
            let digest = owner.command.as_ref().unwrap().hash.clone();
            let handle = owner.command_cancellation(&digest).unwrap();
            let mut b = boundary(owner);
            ask(&mut b);
            let evaluation = resolve(&mut b, true).unwrap();
            if shutdown {
                handle.shutdown().unwrap();
            } else {
                handle.cancel().unwrap();
            }
            let req = b.command.request.clone();
            let call = b.command.call.clone();
            let result = b.execute_using::<Synthetic>(
                RuntimeRunScope::endpoint(&req).unwrap(),
                (&evaluation, &EndpointCommandTool::new().0, &call),
                None,
                |_| panic!("original cancellation must stop before native preflight"),
            );
            assert_eq!(result.err(), Some(RuntimePortFailure::Cancelled));
            assert!(matches!(b.stage, Stage::Spent));
            assert!(b.command.repository.command.is_none());
            assert!(b.inspect().unwrap()["last_result"].is_null());
            assert_eq!(b.inspect().unwrap()["validation"]["attempts"], 0);
            no_receipts(&b);
            let calls = Arc::new(AtomicUsize::new(0));
            assert!(execute(&mut b, &evaluation, Synthetic::new(&calls, true), None).is_err());
            assert_eq!(calls.load(Ordering::SeqCst), 0);
            assert!(resolve(&mut b, true).is_err());
        }
    }
}

struct OriginalHandleCancellation {
    inner: Synthetic,
    entered: std::sync::mpsc::SyncSender<()>,
    cancelled: std::sync::mpsc::Receiver<()>,
    expected: CancellationSignal,
}
impl BoundedCommandExecutor for OriginalHandleCancellation {
    type WorkingDirectory = LinuxAuthorizedWorkspace;
    fn execute(
        &mut self,
        permit: CommandLaunchPermit<'_>,
        root: &LinuxAuthorizedWorkspace,
        observer: &dyn EffectCancellationObservation,
    ) -> std::result::Result<CommandPlatformResult, CommandExecutionFailure> {
        // The synthetic executor has entered with no cancellation. The public
        // pre-handoff handle is then signalled by another thread, without sleeps.
        let mut result = self.inner.execute(permit, root, observer)?;
        self.entered.send(()).unwrap();
        self.cancelled.recv_timeout(Duration::from_secs(5)).unwrap();
        assert_eq!(
            observer.observe_effect_cancellation().unwrap(),
            Some(self.expected.clone())
        );
        result.termination = CommandTermination::Cancelled;
        result.exit_code = None;
        Ok(result)
    }
}

#[test]
fn endpoint_command_original_handle_reaches_live_supervisor_observation_from_another_thread() {
    for validation in [false, true] {
        for shutdown in [false, true] {
            let f = Fixture::new();
            let owner = owner(&f, validation, None);
            let handle = owner
                .command_cancellation(&owner.command.as_ref().unwrap().hash)
                .unwrap();
            let expected = CancellationSignal {
                schema_version: CONTRACT_SCHEMA_VERSION,
                cancellation_id: CancellationId::from_raw(format!("cancel-{}", handle.attempt_id)),
                correlation_id: CorrelationId::from_raw(&handle.attempt_id),
                task_id: TaskId::from_raw(&handle.attempt_id),
                reason: if shutdown {
                    CancellationReason::Shutdown
                } else {
                    CancellationReason::UserRequested
                },
                requested_by: BoundaryKind::Shell,
            };
            let mut b = boundary(owner);
            ask(&mut b);
            let evaluation = resolve(&mut b, true).unwrap();
            let calls = Arc::new(AtomicUsize::new(0));
            let (entered, wait_entered) = std::sync::mpsc::sync_channel(0);
            let (cancelled, wait_cancelled) = std::sync::mpsc::sync_channel(0);
            std::thread::scope(|scope| {
                let canceller = scope.spawn(move || {
                    wait_entered.recv_timeout(Duration::from_secs(5)).unwrap();
                    if shutdown {
                        handle.shutdown().unwrap();
                    } else {
                        handle.cancel().unwrap();
                    }
                    cancelled.send(()).unwrap();
                });
                let req = b.command.request.clone();
                let call = b.command.call.clone();
                let result = b
                    .execute_using(
                        RuntimeRunScope::endpoint(&req).unwrap(),
                        (&evaluation, &EndpointCommandTool::new().0, &call),
                        None,
                        |_| {
                            Ok(OriginalHandleCancellation {
                                inner: Synthetic::new(&calls, true),
                                entered,
                                cancelled: wait_cancelled,
                                expected,
                            })
                        },
                    )
                    .unwrap();
                canceller.join().unwrap();
                assert_eq!(result.result.outcome, OperationOutcome::Cancelled);
                let value: Value =
                    serde_json::from_slice(&result.result.output.unwrap().bytes).unwrap();
                assert_eq!(value["receipt"]["outcome"], "cancelled");
                assert_eq!(value["command_receipt"]["termination"], "cancelled");
                if validation {
                    assert_eq!(value["validation"]["validation_passed"], false);
                    assert_eq!(value["validation"]["attempt"], 1);
                }
            });
            assert!(matches!(b.stage, Stage::Spent));
            assert_eq!(calls.load(Ordering::SeqCst), 1);
            assert!(execute(&mut b, &evaluation, Synthetic::new(&calls, true), None).is_err());
            assert_eq!(calls.load(Ordering::SeqCst), 1);
            assert!(b.inspect().unwrap()["last_result"].is_object());
        }
    }
}

#[test]
fn endpoint_command_abandoned_handle_never_cancels_a_fresh_attempt() {
    for validation in [false, true] {
        for issued in [false, true] {
            let f = Fixture::new();
            let owner = owner(&f, validation, None);
            let old = owner
                .command_cancellation(&owner.command.as_ref().unwrap().hash)
                .unwrap();
            let mut b = boundary(owner);
            if issued {
                ask(&mut b);
                resolve(&mut b, true).unwrap();
            }
            let mut owner = b.into_repository();
            prepare(&mut owner, validation, None);
            let digest = owner.command.as_ref().unwrap().hash.clone();
            let fresh = owner.command_cancellation(&digest).unwrap();
            old.shutdown().unwrap();
            assert!(fresh.observe_effect_cancellation().unwrap().is_none());
            assert_ne!(fresh.attempt_id, old.attempt_id);
            assert_eq!(
                owner
                    .verify_command_cancellation(&digest, Some(&old))
                    .unwrap_err()
                    .code(),
                "code.command.cancellation-stale"
            );
            let calls = Arc::new(AtomicUsize::new(0));
            let mut b = boundary(owner);
            ask(&mut b);
            let e = resolve(&mut b, true).unwrap();
            let result = execute(&mut b, &e, Synthetic::new(&calls, true), None).unwrap();
            assert_eq!(result.result.outcome, OperationOutcome::Succeeded);
            assert_eq!(calls.load(Ordering::SeqCst), 1);
        }
    }
}

#[test]
fn endpoint_command_failed_validation_handle_cannot_cancel_the_exact_fresh_retry() {
    let f = Fixture::new();
    let owner = owner(&f, true, None);
    let old = owner
        .validation_cancellation(&owner.command.as_ref().unwrap().hash)
        .unwrap();
    let calls = Arc::new(AtomicUsize::new(0));
    let mut b = boundary(owner);
    ask(&mut b);
    let e = resolve(&mut b, true).unwrap();
    let result = execute(&mut b, &e, Synthetic::new(&calls, false), None).unwrap();
    let result: Value = serde_json::from_slice(&result.result.output.unwrap().bytes).unwrap();
    let prior = result["validation"]["receipt"]["receipt_sha256"]
        .as_str()
        .unwrap();
    let mut owner = b.into_repository();
    prepare(&mut owner, true, Some(prior));
    let fresh = owner
        .validation_cancellation(&owner.command.as_ref().unwrap().hash)
        .unwrap();
    old.cancel().unwrap();
    assert!(fresh.observe_effect_cancellation().unwrap().is_none());
    assert_ne!(old.attempt_id, fresh.attempt_id);
    let mut b = boundary(owner);
    ask(&mut b);
    let e = resolve(&mut b, true).unwrap();
    let passed = execute(&mut b, &e, Synthetic::new(&calls, true), None).unwrap();
    let passed: Value = serde_json::from_slice(&passed.result.output.unwrap().bytes).unwrap();
    assert_eq!(passed["validation"]["validation_passed"], true);
    assert_eq!(passed["validation"]["attempt"], 2);
    assert_eq!(passed["rerun_plan"]["prior_receipt_sha256"], prior);
    assert_eq!(passed["validation"]["current_workspace_verified"], false);
    assert_eq!(calls.load(Ordering::SeqCst), 2);
}
