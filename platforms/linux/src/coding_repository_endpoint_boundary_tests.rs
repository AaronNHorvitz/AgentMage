// SPDX-License-Identifier: MIT OR Apache-2.0
//! Synthetic creation and endpoint provenance; actual original native writes.
use super::*;
use agentmage_kernel_engine::runtime_loop::{
    RuntimeClock, RuntimePermissionEvaluation, RuntimePortFailure, RuntimeRunScope,
    RuntimeToolBoundary,
};
use std::sync::{Arc, Mutex};

#[derive(Clone)]
struct Clock(Arc<AtomicU64>);
impl RuntimeClock for Clock {
    fn now_epoch_ms(&mut self) -> std::result::Result<u64, RuntimePortFailure> {
        Ok(self.0.load(Ordering::SeqCst))
    }
}
struct Latch(Mutex<std::result::Result<std::num::NonZeroU64, ModelOperationStop>>);
impl Latch {
    fn new() -> Self {
        Self(Mutex::new(running().0))
    }
    fn stop(&self, stop: ModelOperationStop) {
        *self.0.lock().unwrap() = Err(stop);
    }
}
impl ModelOperationControl for Latch {
    fn remaining_ms(&self) -> std::result::Result<std::num::NonZeroU64, ModelOperationStop> {
        *self.0.lock().unwrap()
    }
}
fn clock() -> Clock {
    Clock(Arc::new(AtomicU64::new(NOW)))
}
fn large_handoff(owner: LinuxCodingRepository) -> LinuxCodingEndpointEdit {
    let registry = linux_coding_endpoint_edit_registry().unwrap();
    let mut req = request(&owner.created.as_ref().unwrap().held, &registry);
    req.limits.max_output_bytes = 65536;
    for budget in &mut req.work_packet.budgets {
        if matches!(
            budget.resource,
            BudgetResource::OutputBytes | BudgetResource::DiskBytes
        ) {
            budget.limit = 65536;
        }
    }
    let req = seal_runtime_endpoint_run_request(req).unwrap();
    let call = call(&owner, &req);
    owner
        .into_endpoint_edit(
            req,
            RuntimeTurnId::from_raw("endpoint-edit-turn"),
            RuntimeOperationId::from_raw("endpoint-edit-operation"),
            call,
        )
        .ok()
        .unwrap()
}
fn ask<C: RuntimeClock>(
    boundary: &mut LinuxCodingEndpointEditBoundary<C>,
    req: &RuntimeEndpointRunRequest,
    challenge: &RuntimeApprovalChallenge,
    call: &ToolCall,
) -> RuntimePermissionEvaluation {
    boundary
        .evaluate_scoped(
            RuntimeRunScope::endpoint(req).unwrap(),
            &challenge.operation_id,
            &EndpointEditTool::new().0,
            call,
            NOW,
        )
        .unwrap()
}
fn resolve<C: RuntimeClock>(
    boundary: &mut LinuxCodingEndpointEditBoundary<C>,
    req: &RuntimeEndpointRunRequest,
    challenge: &RuntimeApprovalChallenge,
    call: &ToolCall,
    response: &RuntimeApprovalResponse,
) -> std::result::Result<RuntimePermissionEvaluation, RuntimePortFailure> {
    boundary.resolve_scoped(
        RuntimeRunScope::endpoint(req).unwrap(),
        challenge,
        response,
        &EndpointEditTool::new().0,
        call,
        NOW,
    )
}
fn execute<C: RuntimeClock>(
    boundary: &mut LinuxCodingEndpointEditBoundary<C>,
    req: &RuntimeEndpointRunRequest,
    call: &ToolCall,
    evaluation: &RuntimePermissionEvaluation,
) -> std::result::Result<
    agentmage_kernel_engine::runtime_loop::RuntimeToolExecution,
    RuntimePortFailure,
> {
    boundary.execute_scoped(
        RuntimeRunScope::endpoint(req).unwrap(),
        evaluation,
        &EndpointEditTool::new().0,
        call,
        None,
    )
}

#[test]
fn endpoint_native_boundary_issues_then_consumes_original_grant_and_receipt_once() {
    fn sendable<T: Send>() {}
    sendable::<LinuxCodingEndpointEditBoundary<Clock>>();
    let f = Fixture::new();
    let edit = large_handoff(f.prepared());
    let (req, c, call, r) = (
        edit.request().clone(),
        edit.challenge().clone(),
        edit.call().clone(),
        response(&edit, true),
    );
    let control = Arc::new(Latch::new());
    let mut b = edit.into_boundary(control.clone(), clock());
    assert!(matches!(
        ask(&mut b, &req, &c, &call),
        RuntimePermissionEvaluation::Ask { .. }
    ));
    f.assert_unwritten();
    let allowed = resolve(&mut b, &req, &c, &call, &r).unwrap();
    let RuntimePermissionEvaluation::Allow {
        ref grant_id,
        ref authority_sha256,
        ref decision_sha256,
        ..
    } = allowed
    else {
        panic!("allow")
    };
    assert_eq!(grant_id, &c.proposed_grant_id);
    assert_eq!(authority_sha256.len(), 64);
    assert_ne!(authority_sha256, &c.preview_sha256);
    assert_eq!(decision_sha256, &hash(&to_canonical_json(&r).unwrap()));
    f.assert_unwritten();
    assert!(resolve(&mut b, &req, &c, &call, &r).is_err());
    let executed = execute(&mut b, &req, &call, &allowed).unwrap();
    assert_eq!(executed.result.outcome, OperationOutcome::Succeeded);
    assert_eq!(executed.result.state_change, StateChange::Changed);
    let payload = executed.result.output.unwrap();
    let value: Value = serde_json::from_slice(&payload.bytes).unwrap();
    assert_eq!(payload.sha256, hash(&payload.bytes));
    assert_eq!(value, b.inspect().unwrap()["latest"]);
    let receipts = value["write_result"]["receipts"].as_array().unwrap();
    let last = receipts.last().unwrap();
    assert_eq!(executed.receipt_sha256, last["receipt_sha256"]);
    assert_eq!(
        executed.receipt_id.as_str(),
        format!(
            "{}:{}",
            last["transaction_id"].as_str().unwrap(),
            last["sequence"]
        )
    );
    assert!(
        receipts
            .iter()
            .all(|r| r["grant_id"] == c.proposed_grant_id.as_str())
    );
    assert_eq!(
        receipts
            .iter()
            .filter(|r| r["status"] == "verified")
            .count(),
        2
    );
    assert_eq!(executed.result.evidence[0].content_sha256, payload.sha256);
    assert_eq!(
        fs::read(f.0.join("owned/worktree/one.py")).unwrap(),
        b"repaired first\n"
    );
    assert_eq!(
        fs::read(f.0.join("owned/worktree/two.py")).unwrap(),
        b"repaired second\n"
    );
    f.assert_original();
    assert!(execute(&mut b, &req, &call, &allowed).is_err());
    let owner = b.into_repository();
    assert!(owner.edit.is_none());
    assert_eq!(owner.created.as_ref().unwrap().edits.len(), 2);
    assert_eq!(owner.inspect_edit().unwrap()["latest"], value);
}

#[test]
fn endpoint_native_boundary_invalid_decisions_are_inert_and_denial_is_final() {
    for kind in 0..9 {
        let f = Fixture::new();
        let edit = large_handoff(f.prepared());
        let (req, c, call, mut bad, r) = (
            edit.request().clone(),
            edit.challenge().clone(),
            edit.call().clone(),
            response(&edit, true),
            response(&edit, false),
        );
        let control = Arc::new(Latch::new());
        let mut b = edit.into_boundary(control.clone(), clock());
        ask(&mut b, &req, &c, &call);
        match kind {
            0 => bad.schema_version += 1,
            1 => bad.run_id = RuntimeRunId::from_raw("other-run"),
            2 => bad.approval_id = ApprovalId::from_raw("other-approval"),
            3 => bad.challenge_sha256 = "a".repeat(64),
            4 => bad.grant_id = Some(GrantId::from_raw("other-grant")),
            5 => bad.grant_id = None,
            6 => bad.disposition = RuntimeApprovalDisposition::Narrow,
            7 => bad.disposition = RuntimeApprovalDisposition::Deny,
            _ => bad.challenge_sha256.clear(),
        }
        assert!(
            resolve(&mut b, &req, &c, &call, &bad).is_err(),
            "case {kind}"
        );
        f.assert_unwritten();
        assert!(matches!(
            resolve(&mut b, &req, &c, &call, &r).unwrap(),
            RuntimePermissionEvaluation::Deny { .. }
        ));
        assert!(resolve(&mut b, &req, &c, &call, &r).is_err());
        assert_eq!(b.inspect().unwrap()["latest"]["outcome"], "denied");
        assert!(b.into_repository().edit.is_none());
        f.assert_unwritten();
    }
}

#[test]
fn endpoint_native_boundary_exact_scope_call_definition_operation_and_evaluation() {
    for kind in 0..10 {
        let f = Fixture::new();
        let edit = large_handoff(f.prepared());
        let (req, c, call, r) = (
            edit.request().clone(),
            edit.challenge().clone(),
            edit.call().clone(),
            response(&edit, true),
        );
        let control = Arc::new(Latch::new());
        let mut b = edit.into_boundary(control.clone(), clock());
        let mut foreign = req.clone();
        foreign.task.objective.push_str(" replacement");
        foreign.work_packet.objective = foreign.task.objective.clone();
        foreign = seal_runtime_endpoint_run_request(foreign).unwrap();
        let mut changed_call = call.clone();
        let mut definition = EndpointEditTool::new().0;
        let mut operation = c.operation_id.clone();
        match kind {
            0 => (),
            1 => changed_call.tool_call_id = ToolCallId::from_raw("other-call"),
            2 => changed_call.correlation_id = CorrelationId::from_raw("other-correlation"),
            3 => changed_call.action_id = ActionId::from_raw("other-action"),
            4 => changed_call.arguments.bytes.push(b' '),
            5 => changed_call.arguments.sha256 = "b".repeat(64),
            6 => definition.display_name.push_str(" replacement"),
            7 => definition
                .required_grant
                .target_scope
                .push_str(" replacement"),
            8 => operation = RuntimeOperationId::from_raw("other-operation"),
            _ => changed_call.tool_version = "2.0.0".into(),
        }
        let scope = RuntimeRunScope::endpoint(if kind == 0 { &foreign } else { &req }).unwrap();
        assert!(
            b.evaluate_scoped(scope, &operation, &definition, &changed_call, NOW)
                .is_err(),
            "case {kind}"
        );
        ask(&mut b, &req, &c, &call);
        assert!(
            b.evaluate_scoped(
                RuntimeRunScope::endpoint(&req).unwrap(),
                &c.operation_id,
                &EndpointEditTool::new().0,
                &call,
                NOW
            )
            .is_err()
        );
        let mut changed_challenge = c.clone();
        changed_challenge.turn_id = RuntimeTurnId::from_raw("other-turn");
        let changed_challenge =
            agentmage_kernel_engine::runtime_coordinator::seal_runtime_approval_challenge(
                changed_challenge,
            )
            .unwrap();
        assert!(resolve(&mut b, &req, &changed_challenge, &call, &r).is_err());
        let allowed = resolve(&mut b, &req, &c, &call, &r).unwrap();
        let mut bad_allowed = allowed.clone();
        if let RuntimePermissionEvaluation::Allow {
            authority_sha256, ..
        } = &mut bad_allowed
        {
            *authority_sha256 = "c".repeat(64);
        }
        assert!(execute(&mut b, &req, &call, &bad_allowed).is_err());
        assert!(execute(&mut b, &foreign, &call, &allowed).is_err());
        f.assert_unwritten();
        assert!(execute(&mut b, &req, &call, &allowed).is_ok());
        f.assert_original();
    }
}

#[test]
fn endpoint_native_boundary_original_stop_at_every_stage_is_terminal() {
    for stage in 0..3 {
        for stop in [
            ModelOperationStop::Cancelled,
            ModelOperationStop::TimedOut,
            ModelOperationStop::Unavailable,
            ModelOperationStop::Invalid,
        ] {
            let f = Fixture::new();
            let edit = large_handoff(f.prepared());
            let (req, c, call, r) = (
                edit.request().clone(),
                edit.challenge().clone(),
                edit.call().clone(),
                response(&edit, true),
            );
            let control = Arc::new(Latch::new());
            let mut b = edit.into_boundary(control.clone(), clock());
            if stage == 0 {
                control.stop(stop);
                assert!(
                    b.evaluate_scoped(
                        RuntimeRunScope::endpoint(&req).unwrap(),
                        &c.operation_id,
                        &EndpointEditTool::new().0,
                        &call,
                        NOW
                    )
                    .is_err()
                );
            } else {
                ask(&mut b, &req, &c, &call);
                if stage == 1 {
                    control.stop(stop);
                    assert!(resolve(&mut b, &req, &c, &call, &r).is_err());
                } else {
                    let allowed = resolve(&mut b, &req, &c, &call, &r).unwrap();
                    control.stop(stop);
                    assert!(execute(&mut b, &req, &call, &allowed).is_err());
                    assert!(execute(&mut b, &req, &call, &allowed).is_err());
                }
            }
            *control.0.lock().unwrap() = running().0;
            assert!(
                b.evaluate_scoped(
                    RuntimeRunScope::endpoint(&req).unwrap(),
                    &c.operation_id,
                    &EndpointEditTool::new().0,
                    &call,
                    NOW
                )
                .is_err()
            );
            assert!(resolve(&mut b, &req, &c, &call, &r).is_err());
            assert!(b.into_repository().edit.is_none());
            f.assert_unwritten();
        }
    }
}

#[test]
fn endpoint_native_boundary_expiry_preimage_link_and_clock_drift_never_replay() {
    for kind in 0..5 {
        let f = Fixture::new();
        let edit = large_handoff(f.prepared());
        let (req, c, call, r) = (
            edit.request().clone(),
            edit.challenge().clone(),
            edit.call().clone(),
            response(&edit, true),
        );
        let control = Arc::new(Latch::new());
        let clock = clock();
        let mut b = edit.into_boundary(control.clone(), clock.clone());
        ask(&mut b, &req, &c, &call);
        let allowed = resolve(&mut b, &req, &c, &call, &r).unwrap();
        match kind {
            0 => clock.0.store(c.expires_at_epoch_ms, Ordering::SeqCst),
            1 => clock.0.store(NOW - 1, Ordering::SeqCst),
            2 => fs::write(f.0.join("owned/worktree/one.py"), b"person changed this\n").unwrap(),
            3 => {
                fs::remove_file(f.0.join("owned/worktree/one.py")).unwrap();
                symlink(
                    f.0.join("original/one.py"),
                    f.0.join("owned/worktree/one.py"),
                )
                .unwrap();
            }
            _ => fs::rename(f.0.join("owned/worktree"), f.0.join("owned/moved")).unwrap(),
        }
        assert!(
            execute(&mut b, &req, &call, &allowed).is_err(),
            "case {kind}"
        );
        assert!(execute(&mut b, &req, &call, &allowed).is_err());
        f.assert_original();
        if kind < 2 {
            f.assert_unwritten();
        }
        if kind == 2 {
            assert_eq!(
                fs::read(f.0.join("owned/worktree/one.py")).unwrap(),
                b"person changed this\n"
            );
        }
        assert!(b.into_repository().edit.is_none());
    }
}

#[test]
fn endpoint_native_boundary_output_refusal_retains_actual_effect_and_lineage() {
    let f = Fixture::new();
    // The original small output budget is intentionally insufficient for the
    // native receipt chain. Refusal AFTER dispatch must never imply no effect.
    let edit = handoff(f.prepared());
    let (req, c, call, r) = (
        edit.request().clone(),
        edit.challenge().clone(),
        edit.call().clone(),
        response(&edit, true),
    );
    let control = Arc::new(Latch::new());
    let mut b = edit.into_boundary(control.clone(), clock());
    ask(&mut b, &req, &c, &call);
    let allowed = resolve(&mut b, &req, &c, &call, &r).unwrap();
    assert_eq!(
        execute(&mut b, &req, &call, &allowed),
        Err(RuntimePortFailure::Uncertain)
    );
    assert!(execute(&mut b, &req, &call, &allowed).is_err());
    assert_eq!(b.inspect().unwrap()["latest"]["outcome"], "committed");
    let owner = b.into_repository();
    assert_eq!(owner.created.as_ref().unwrap().edits.len(), 2);
    assert!(owner.edit.is_none());
    assert_eq!(
        fs::read(f.0.join("owned/worktree/one.py")).unwrap(),
        b"repaired first\n"
    );
    f.assert_original();
}

struct Probe(u8, CancellationSignal);
impl ModelCancellationProbe for Probe {
    fn observe(&self) -> std::result::Result<Option<CancellationSignal>, ModelRuntimeFailure> {
        match self.0 {
            0 => Ok(None),
            1 | 2 => Ok(Some(self.1.clone())),
            3 => Err(ModelRuntimeFailure {
                code: "fixture.unavailable".into(),
                retryable_after_correction: false,
                dependency_recovery_required: true,
                contract_error: None,
            }),
            _ => panic!("synthetic cancellation dependency unwind"),
        }
    }
}
#[test]
fn endpoint_native_boundary_cancellation_failure_and_unwind_spend_dispatch() {
    for kind in 0..5 {
        let f = Fixture::new();
        let edit = large_handoff(f.prepared());
        let (req, c, call, r) = (
            edit.request().clone(),
            edit.challenge().clone(),
            edit.call().clone(),
            response(&edit, true),
        );
        let control = Arc::new(Latch::new());
        let mut b = edit.into_boundary(control.clone(), clock());
        ask(&mut b, &req, &c, &call);
        let allowed = resolve(&mut b, &req, &c, &call, &r).unwrap();
        let probe = Probe(
            kind,
            CancellationSignal {
                schema_version: CONTRACT_SCHEMA_VERSION,
                cancellation_id: CancellationId::from_raw("boundary-cancellation"),
                correlation_id: call.correlation_id.clone(),
                task_id: if kind == 2 {
                    TaskId::from_raw("other-task")
                } else {
                    req.task.task_id.clone()
                },
                reason: CancellationReason::UserRequested,
                requested_by: BoundaryKind::Shell,
            },
        );
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            b.execute_scoped(
                RuntimeRunScope::endpoint(&req).unwrap(),
                &allowed,
                &EndpointEditTool::new().0,
                &call,
                Some(&probe),
            )
        }));
        if kind == 0 {
            assert!(result.unwrap().is_ok());
        } else {
            if kind == 4 {
                assert!(result.is_err());
            } else {
                assert!(result.unwrap().is_err());
            }
            f.assert_unwritten();
        }
        assert!(execute(&mut b, &req, &call, &allowed).is_err());
        assert!(b.into_repository().edit.is_none());
        f.assert_original();
    }
}
struct PanickingClock;
impl RuntimeClock for PanickingClock {
    fn now_epoch_ms(&mut self) -> std::result::Result<u64, RuntimePortFailure> {
        panic!("synthetic clock unwind")
    }
}
#[test]
fn endpoint_native_boundary_clock_unwind_and_prior_spent_handle_cannot_reopen() {
    let f = Fixture::new();
    let edit = large_handoff(f.prepared());
    let (req, c, call, r) = (
        edit.request().clone(),
        edit.challenge().clone(),
        edit.call().clone(),
        response(&edit, true),
    );
    let control = Arc::new(Latch::new());
    let mut b = edit.into_boundary(control.clone(), PanickingClock);
    ask(&mut b, &req, &c, &call);
    let allowed = resolve(&mut b, &req, &c, &call, &r).unwrap();
    assert!(
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| execute(
            &mut b, &req, &call, &allowed
        )))
        .is_err()
    );
    assert!(execute(&mut b, &req, &call, &allowed).is_err());
    f.assert_unwritten();
    drop(b);
    let mut edit = large_handoff(f.prepared());
    let (req, c, call, r) = (
        edit.request().clone(),
        edit.challenge().clone(),
        edit.call().clone(),
        response(&edit, true),
    );
    assert!(
        edit.decide(&r, NOW, &Control(Err(ModelOperationStop::Cancelled)))
            .is_err()
    );
    let mut b = edit.into_boundary(control.clone(), clock());
    assert!(
        b.evaluate_scoped(
            RuntimeRunScope::endpoint(&req).unwrap(),
            &c.operation_id,
            &EndpointEditTool::new().0,
            &call,
            NOW
        )
        .is_err()
    );
    assert!(resolve(&mut b, &req, &c, &call, &r).is_err());
    f.assert_unwritten();
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
            assert_eq!(input.tool_results[0].state_change, StateChange::Changed);
            assert_eq!(input.completed_tool_calls.len(), 1);
            // Deliberate upstream refusal after observing the real native write.
            // This does not manufacture task completion or a coding score.
            return Err(RuntimePortFailure::Unavailable);
        }
        let context = RuntimeEndpointContextPacket::from_messages(&[RuntimeEndpointMessage {
            role: RuntimeEndpointMessageRole::User,
            content: "Apply only the separately prepared fixture diff",
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
        let text=json!({"tool_call":{"tool_id":ENDPOINT_EDIT,"tool_version":"1.0.0","arguments":self.arguments}}).to_string();
        Ok(RuntimePreparedEndpointTurn::new(
            context,
            Dispatch {
                request,
                text,
                calls: self.calls.clone(),
            },
            EndpointEditTool::new().0.output_schema,
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
#[test]
fn endpoint_native_boundary_actual_coordinator_challenge_and_native_write() {
    for allow in [false, true] {
        let f = Fixture::new();
        let edit = large_handoff(f.prepared());
        let req = edit.request().clone();
        let mut call = edit.call().clone();
        call.tool_call_id = ToolCallId::from_raw(derived("endpoint-call", &req, 1));
        call.correlation_id = CorrelationId::from_raw(derived("correlation", &req, 0));
        let edit = edit.into_repository();
        // into_repository intentionally abandons its preview; prepare anew with
        // the same live creation owner before binding the actual coordinator IDs.
        let mut owner = edit;
        let scope = owner
            .select_created_sources(&["one.py".into(), "two.py".into()])
            .unwrap();
        owner
            .prepare_edit(
                scope,
                &[
                    LinuxCodingReplacement {
                        path: "one.py",
                        before_sha256: &hash(b"first\n"),
                        content: "repaired first\n",
                    },
                    LinuxCodingReplacement {
                        path: "two.py",
                        before_sha256: &hash(b"second\n"),
                        content: "repaired second\n",
                    },
                ],
                "Apply the prepared fixture edit",
                NOW,
            )
            .unwrap();
        call.arguments.bytes =
            serde_json::to_vec(&json!({"preview":owner.inspect_edit().unwrap()["pending"]}))
                .unwrap();
        call.arguments.sha256 = hash(&call.arguments.bytes);
        let edit = owner
            .into_endpoint_edit(
                req.clone(),
                RuntimeTurnId::from_raw(derived("turn", &req, 1)),
                RuntimeOperationId::from_raw(derived("operation", &req, 1)),
                call.clone(),
            )
            .ok()
            .unwrap();
        let challenge = edit.challenge().clone();
        let response = response(&edit, allow);
        let calls = Arc::new(AtomicU64::new(0));
        let preparations = Arc::new(AtomicU64::new(0));
        let port = SyntheticPort {
            selection: req.model_selection.clone(),
            arguments: serde_json::from_slice(&call.arguments.bytes).unwrap(),
            calls: calls.clone(),
            preparations: preparations.clone(),
        };
        let control = Arc::new(Latch::new());
        let boundary = edit.into_boundary(control.clone(), clock());
        let mut coordinator = ReusableRuntimeCoordinator::new_endpoint(
            req,
            port,
            linux_coding_endpoint_edit_registry().unwrap(),
            boundary,
            NoCompletion(VerifierId::from_raw("fixture-no-completion")),
            clock(),
        )
        .unwrap();
        let RuntimeEndpointCoordinatorStep::AwaitingApproval { challenge: actual } =
            coordinator.run_endpoint_until_boundary(None, None).unwrap()
        else {
            panic!("challenge")
        };
        assert_eq!(actual, challenge);
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        f.assert_unwritten();
        let mut bad = response.clone();
        bad.challenge_sha256 = "f".repeat(64);
        assert!(
            coordinator
                .run_endpoint_until_boundary(Some(&bad), None)
                .is_err()
        );
        f.assert_unwritten();
        let RuntimeEndpointCoordinatorStep::Complete { outcome } = coordinator
            .run_endpoint_until_boundary(Some(&response), None)
            .unwrap()
        else {
            panic!("terminal")
        };
        assert!(!outcome.outcome.state.is_success());
        assert_eq!(
            preparations.load(Ordering::SeqCst),
            if allow { 2 } else { 1 }
        );
        if allow {
            assert_eq!(outcome.outcome.receipt_ids.len(), 1);
            assert_eq!(
                fs::read(f.0.join("owned/worktree/one.py")).unwrap(),
                b"repaired first\n"
            );
            assert_eq!(
                fs::read(f.0.join("owned/worktree/two.py")).unwrap(),
                b"repaired second\n"
            );
        } else {
            assert!(outcome.outcome.receipt_ids.is_empty());
            f.assert_unwritten();
        }
        let terminal = coordinator.run_endpoint_until_boundary(None, None).unwrap();
        assert_eq!(
            terminal,
            RuntimeEndpointCoordinatorStep::Complete { outcome }
        );
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        f.assert_original();
        // Return the actual native owner after terminal publication. A known
        // later model refusal does not erase the already committed edit lineage.
        let boundary = Box::new(coordinator)
            .try_into_endpoint_tool_boundary()
            .ok()
            .unwrap();
        let mut owner = boundary.into_repository();
        assert!(owner.inspect_edit().unwrap()["pending"].is_null());
        assert_eq!(
            owner.created.as_ref().unwrap().edits.len(),
            if allow { 2 } else { 0 }
        );
        if allow {
            let status = owner.inspect_apply().unwrap();
            let preview = owner
                .prepare_apply(
                    &["one.py".into(), "two.py".into()],
                    "Separately review the retained original-checkout diff",
                    status["repository_root_sha256"].as_str().unwrap(),
                    status["worktree_root_sha256"].as_str().unwrap(),
                    NOW,
                )
                .unwrap();
            assert!(preview.is_object());
            f.assert_original();
        }
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }
}
