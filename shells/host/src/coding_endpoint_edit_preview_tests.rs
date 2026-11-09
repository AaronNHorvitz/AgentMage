// SPDX-License-Identifier: Apache-2.0
//! Synthetic endpoint/creation; real strict parsing, native edits and approvals.
use super::*;
use crate::coding_endpoint_edit_generation::CodingEndpointEditGeneration;
use crate::coding_endpoint_edit_previews::{
    CodingEndpointEditBoundary, CodingEndpointEditPreviews,
};
use agentmage_kernel_engine::runtime_loop::{
    ReusableRuntimeCoordinator, RuntimeClock, RuntimeCompletionVerificationInput,
    RuntimeEndpointCoordinatorStep, RuntimeEndpointPreparationInput, RuntimeEndpointTurnPort,
    RuntimePermissionEvaluation, RuntimePreparedEndpointTurn, RuntimeRunScope, RuntimeToolBoundary,
    RuntimeToolExecution, RuntimeVerificationInput, RuntimeVerifierPort,
};
use std::{collections::VecDeque, sync::atomic::AtomicUsize};

struct ObservedBoundary {
    inner: CodingEndpointEditBoundary<Clock>,
    output: Arc<AtomicUsize>,
}
impl RuntimeToolBoundary for ObservedBoundary {
    fn evaluate_scoped(
        &mut self,
        s: RuntimeRunScope<'_>,
        op: &RuntimeOperationId,
        d: &ToolDefinition,
        c: &ToolCall,
        now: u64,
    ) -> Result<RuntimePermissionEvaluation, RuntimePortFailure> {
        self.inner.evaluate_scoped(s, op, d, c, now)
    }
    fn resolve_scoped(
        &mut self,
        s: RuntimeRunScope<'_>,
        ch: &RuntimeApprovalChallenge,
        r: &RuntimeApprovalResponse,
        d: &ToolDefinition,
        c: &ToolCall,
        now: u64,
    ) -> Result<RuntimePermissionEvaluation, RuntimePortFailure> {
        self.inner.resolve_scoped(s, ch, r, d, c, now)
    }
    fn execute_scoped(
        &mut self,
        s: RuntimeRunScope<'_>,
        e: &RuntimePermissionEvaluation,
        d: &ToolDefinition,
        c: &ToolCall,
        p: Option<&dyn ModelCancellationProbe>,
    ) -> Result<RuntimeToolExecution, RuntimePortFailure> {
        let result = self.inner.execute_scoped(s, e, d, c, p)?;
        self.output.fetch_add(
            result.result.output.as_ref().unwrap().bytes.len(),
            Ordering::SeqCst,
        );
        Ok(result)
    }
    fn evaluate(
        &mut self,
        _: &RuntimeRunRequest,
        _: &RuntimeOperationId,
        _: &ToolDefinition,
        _: &ToolCall,
        _: u64,
    ) -> Result<RuntimePermissionEvaluation, RuntimePortFailure> {
        panic!("no native request")
    }
    fn resolve(
        &mut self,
        _: &RuntimeRunRequest,
        _: &RuntimeApprovalChallenge,
        _: &RuntimeApprovalResponse,
        _: &ToolDefinition,
        _: &ToolCall,
        _: u64,
    ) -> Result<RuntimePermissionEvaluation, RuntimePortFailure> {
        panic!("no native request")
    }
    fn execute(
        &mut self,
        _: &RuntimeRunRequest,
        _: &RuntimePermissionEvaluation,
        _: &ToolDefinition,
        _: &ToolCall,
        _: Option<&dyn ModelCancellationProbe>,
    ) -> Result<RuntimeToolExecution, RuntimePortFailure> {
        panic!("no native request")
    }
}

#[derive(Clone)]
struct Clock(Arc<AtomicU64>);
impl RuntimeClock for Clock {
    fn now_epoch_ms(&mut self) -> Result<u64, RuntimePortFailure> {
        Ok(self.0.load(Ordering::SeqCst))
    }
}
struct Probe {
    control: Arc<Control>,
    signal: CancellationSignal,
}
impl ModelCancellationProbe for Probe {
    fn observe(&self) -> Result<Option<CancellationSignal>, ModelRuntimeFailure> {
        Ok(self
            .control
            .0
            .load(Ordering::SeqCst)
            .then(|| self.signal.clone()))
    }
}
struct Turns<'c> {
    selection: EndpointModelSelection,
    turns: VecDeque<CodingEndpointEditGeneration<'c>>,
    previews: CodingEndpointEditPreviews<Clock>,
}
impl RuntimeEndpointTurnPort for Turns<'_> {
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
    ) -> Result<RuntimePreparedEndpointTurn<'a>, RuntimePortFailure>
    where
        Self: 'a,
    {
        let schema = SchemaReference {
            schema_id: SchemaId::from_raw("synthetic.strict.no-completion"),
            schema_version: 1,
            schema_sha256: "a".repeat(64),
        };
        self.turns
            .pop_front()
            .ok_or(RuntimePortFailure::Invalid)?
            .into_coordinator_edit_turn(input, control, schema, &mut self.previews)
            .map_err(|e| match e {
                GenerationError::Preparation(e) => e,
                _ => RuntimePortFailure::Invalid,
            })
    }
}
struct NeverVerify(VerifierId);
impl RuntimeVerifierPort for NeverVerify {
    fn verifier_id(&self) -> &VerifierId {
        &self.0
    }
    fn verify(
        &mut self,
        _: RuntimeVerificationInput<'_>,
    ) -> Result<VerifierCandidate, RuntimePortFailure> {
        panic!("strict tool responses cannot verify completion")
    }
    fn verify_completion(
        &mut self,
        _: RuntimeCompletionVerificationInput<'_>,
    ) -> Result<VerifierCandidate, RuntimePortFailure> {
        panic!("strict tool responses cannot verify completion")
    }
}

fn allow(ch: &RuntimeApprovalChallenge) -> RuntimeApprovalResponse {
    RuntimeApprovalResponse {
        schema_version: CONTRACT_SCHEMA_VERSION,
        run_id: ch.run_id.clone(),
        approval_id: ch.approval_id.clone(),
        disposition: RuntimeApprovalDisposition::Allow,
        challenge_sha256: ch.challenge_sha256.clone(),
        grant_id: Some(ch.proposed_grant_id.clone()),
        selection: None,
    }
}

#[test]
fn strict_preview_coordinator_keeps_original_owner_and_each_exact_approval() {
    // Budget, malformed/partial/stale/protected/outside input, denial, stop,
    // deadline and tool-shaped answer all exercise the same production path.
    for case in 0..13 {
        for shared in [false, true] {
            let f = generation_fixture();
            let repository = owner(&f);
            let (listener, endpoint) = socket();
            let startup = generation_startup(&f, &endpoint, &f.empty_ledger());
            let mut request = startup.request().clone();
            request.limits.max_turns = 2;
            request.limits.max_model_calls = if case == 1 { 1 } else { 2 };
            let startup = Startup::prepare(
                seal_runtime_endpoint_run_request(request).unwrap(),
                Kind::Edit,
                &f.empty_ledger(),
            )
            .unwrap();
            let request = startup.request().clone();
            let clock = Arc::new(AtomicU64::new(1000));
            let control = Arc::new(Control::new());
            let (previews, boundary) = CodingEndpointEditPreviews::new(
                startup,
                repository,
                control.clone(),
                Clock(clock.clone()),
            )
            .unwrap();
            let transport = configured_transport(endpoint.clone());
            let approval = LocalChatApproval::for_endpoint(endpoint.clone());
            let mut turns = VecDeque::new();
            let mut bodies = vec![];
            let mut users = vec![];
            for (path, content) in [
                ("one.py", "one\n"),
                if case == 12 {
                    ("one.py", "one\n")
                } else {
                    ("two.py", "two\n")
                },
            ] {
                let connection = if shared {
                    connection(transport.clone(), endpoint.clone())
                } else {
                    CodingEndpointConnection::Borrowed {
                        transport: &transport,
                        approval: &approval,
                    }
                };
                let context = proposal_context(&request.task.objective, &[(path, content)]);
                users.push(context.user_message().to_owned());
                let generation = previews
                    .startup()
                    .prepare_edit_generation(connection, context, true)
                    .unwrap();
                bodies.push(generation.binding().body().request_body_bytes as u64);
                turns.push_back(generation);
            }
            if case == 5 {
                fs::write(f.root.join("owned/worktree/one.py"), "changed\n").unwrap();
            }
            let first = match case {
                2 => "{\"edits\":[]}".into(),
                3 => "x".repeat(65537),
                6 => json!({"edits":[edit("private/secret.py","secret","leak")]}).to_string(),
                7 => json!({"edits":[edit("../outside","one","escape")]}).to_string(),
                11 => json!({"tool_call":{"tool_id":"agentmage.code.apply-prepared-edit","tool_version":"1.0.0","arguments":{}}}).to_string(),
                _ => good(),
            };
            let second = if case == 12 {
                good()
            } else {
                json!({"edits":[edit("two.py","two","second")]}).to_string()
            };
            let expected_requests = if case == 0 || case == 12 { 2 } else { 1 };
            let model_output = if case == 3 || case == 4 {
                0
            } else {
                first.len()
                    + if expected_requests == 2 {
                        second.len()
                    } else {
                        0
                    }
            };
            let peer = thread::spawn(move || {
                for (ordinal, user) in users.iter().take(expected_requests).enumerate() {
                    let mut stream = accept(&listener);
                    let b = body(&mut stream);
                    assert_eq!(b["messages"][1]["content"].as_str(), Some(user.as_str()));
                    assert_eq!(b["messages"][0]["content"], agentmage_capability_repository_map::coding_proposal::CODING_PROPOSAL_INSTRUCTION);
                    respond(
                        &mut stream,
                        if ordinal == 0 { &first } else { &second },
                        "stop",
                        case != 4,
                    );
                }
                listener
            });
            let native_output = Arc::new(AtomicUsize::new(0));
            let mut coordinator = ReusableRuntimeCoordinator::new_endpoint(
                request.clone(),
                Turns {
                    selection: request.model_selection.clone(),
                    turns,
                    previews,
                },
                linux_coding_endpoint_edit_registry().unwrap(),
                ObservedBoundary {
                    inner: boundary,
                    output: native_output.clone(),
                },
                NeverVerify(VerifierId::from_raw("synthetic-strict-never-complete")),
                Clock(clock.clone()),
            )
            .unwrap();
            let probe = Probe {
                control: control.clone(),
                signal: CancellationSignal {
                    schema_version: CONTRACT_SCHEMA_VERSION,
                    cancellation_id: CancellationId::from_raw("synthetic-strict-preview-cancel"),
                    correlation_id: coordinator.correlation_id().clone(),
                    task_id: request.task.task_id.clone(),
                    reason: CancellationReason::UserRequested,
                    requested_by: BoundaryKind::Shell,
                },
            };
            let mut step = coordinator
                .run_endpoint_until_boundary(None, Some(&probe))
                .unwrap();
            let mut approved = vec![];
            let mut argument_bytes = 0;
            while let RuntimeEndpointCoordinatorStep::AwaitingApproval { challenge } = &step {
                assert_eq!(
                    challenge.presentation.tool_id.as_str(),
                    "agentmage.code.apply-prepared-edit"
                );
                assert_eq!(challenge.operation, GrantOperation::WorkspaceWrite);
                assert_eq!(
                    fs::read(f.root.join("owned/worktree/two.py")).unwrap(),
                    b"two\n"
                );
                if approved.is_empty() {
                    assert_eq!(
                        fs::read(f.root.join("owned/worktree/one.py")).unwrap(),
                        b"one\n"
                    );
                } else {
                    assert_eq!(
                        fs::read(f.root.join("owned/worktree/one.py")).unwrap(),
                        b"first\n"
                    );
                }
                argument_bytes += challenge.presentation.arguments.bytes.len() as u64;
                let mut response = allow(challenge);
                let before = coordinator.events().to_vec();
                response.challenge_sha256 = "0".repeat(64);
                assert!(
                    coordinator
                        .run_endpoint_until_boundary(Some(&response), None)
                        .is_err()
                );
                assert_eq!(coordinator.events(), before);
                response = allow(challenge);
                if let Some(previous) = approved.last() {
                    assert!(
                        coordinator
                            .run_endpoint_until_boundary(Some(previous), None)
                            .is_err()
                    );
                    assert_eq!(coordinator.events(), before);
                    assert_ne!(previous.approval_id, response.approval_id);
                }
                if case == 8 {
                    response.disposition = RuntimeApprovalDisposition::Deny;
                    response.grant_id = None;
                }
                if case == 9 {
                    control.0.store(true, Ordering::SeqCst);
                }
                if case == 10 {
                    clock.store(challenge.expires_at_epoch_ms.max(100_000), Ordering::SeqCst);
                    // Expired response input is inert. Advance without it to
                    // observe the original run deadline, never renew approval.
                    assert!(
                        coordinator
                            .run_endpoint_until_boundary(Some(&response), Some(&probe))
                            .is_err()
                    );
                    assert_eq!(coordinator.events(), before);
                }
                approved.push(response.clone());
                step = coordinator
                    .run_endpoint_until_boundary((case != 10).then_some(&response), Some(&probe))
                    .unwrap_or_else(|e| panic!("case={case} shared={shared}: {e:?}"));
            }
            let RuntimeEndpointCoordinatorStep::Complete { outcome } = &step else {
                unreachable!()
            };
            let edited = matches!(case, 0 | 1 | 12);
            assert_eq!(
                fs::read(f.root.join("owned/worktree/one.py")).unwrap(),
                if edited {
                    b"first\n".as_slice()
                } else if case == 5 {
                    b"changed\n".as_slice()
                } else {
                    b"one\n".as_slice()
                }
            );
            assert_eq!(
                fs::read(f.root.join("owned/worktree/two.py")).unwrap(),
                if case == 0 {
                    b"second\n".as_slice()
                } else {
                    b"two\n"
                }
            );
            assert_eq!(
                outcome.outcome.state,
                match case {
                    0 | 1 | 3 | 10 => AgentStateKind::Exhausted,
                    8 => AgentStateKind::Declined,
                    9 => AgentStateKind::Cancelled,
                    _ => AgentStateKind::Failed,
                },
                "case={case} shared={shared}"
            );
            assert_eq!(
                outcome.outcome.tool_call_count,
                if case == 0 {
                    2
                } else {
                    u32::from(matches!(case, 1 | 8..=10 | 12))
                }
            );
            assert!(outcome.outcome.output.is_none());
            assert_eq!(outcome.outcome.model_call_count, expected_requests as u32);
            let usage = &coordinator.resource_snapshot().cumulative;
            assert_eq!(usage[&BudgetResource::ModelCalls], expected_requests as u64);
            assert_eq!(
                usage[&BudgetResource::InputBytes],
                bodies[..expected_requests].iter().sum::<u64>() + argument_bytes
            );
            let tools: Vec<_> = coordinator
                .events()
                .iter()
                .filter(|event| matches!(&event.kind, RuntimeEventKind::ToolCompleted { .. }))
                .collect();
            assert_eq!(tools.len(), usize::from(edited) + usize::from(case == 0));
            assert_eq!(
                usage
                    .get(&BudgetResource::OutputBytes)
                    .copied()
                    .unwrap_or(0),
                (model_output + native_output.load(Ordering::SeqCst)) as u64
            );
            if case == 3 || case == 4 {
                assert!(!usage.contains_key(&BudgetResource::OutputBytes));
            }
            let snapshot = coordinator.resource_snapshot().clone();
            let events = coordinator.events().to_vec();
            let mut sequence = agentmage_kernel_engine::runtime_event::RuntimeEventSequence::new();
            for event in &events {
                sequence.push(event).unwrap();
            }
            assert_eq!(
                coordinator.run_endpoint_until_boundary(None, None).unwrap(),
                step
            );
            assert_eq!(coordinator.resource_snapshot(), &snapshot);
            assert_eq!(coordinator.events(), events);
            let view = coordinator
                .ended_tool_boundary()
                .unwrap()
                .inner
                .inspect()
                .unwrap();
            if matches!(case, 2..=7 | 11) {
                assert!(view["pending"].is_null());
            }
            if edited {
                assert!(!view["latest"].is_null());
            }
            let listener = peer.join().unwrap();
            assert!(
                matches!(listener.accept(), Err(e) if e.kind() == std::io::ErrorKind::WouldBlock)
            );
        }
    }
}

#[test]
fn strict_owner_return_refuses_live_previews_and_completion_callbacks_without_losing_owner() {
    let f = generation_fixture();
    let repository = owner(&f);
    let before = repository.inspect_edit().unwrap();
    let (_listener, endpoint) = socket();
    let startup = generation_startup(&f, &endpoint, &f.empty_ledger());
    let (previews, boundary) = CodingEndpointEditPreviews::new(
        startup,
        repository,
        Arc::new(Control::new()),
        Clock(Arc::new(AtomicU64::new(1000))),
    )
    .unwrap();
    let boundary = boundary
        .try_into_repository()
        .err()
        .expect("live previews must refuse");
    assert_eq!(boundary.inspect().unwrap(), before);
    let callback = previews.completion_check();
    drop(previews);
    let boundary = boundary
        .try_into_repository()
        .err()
        .expect("live callback must refuse");
    assert_eq!(boundary.inspect().unwrap(), before);
    drop(callback);
    let repository = boundary
        .try_into_repository()
        .ok()
        .expect("sole original owner");
    assert_eq!(repository.inspect_edit().unwrap(), before);
    assert_eq!(
        fs::read(f.root.join("owned/worktree/one.py")).unwrap(),
        b"one\n"
    );
}
