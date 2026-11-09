// SPDX-License-Identifier: Apache-2.0
//! Synthetic endpoint replies; actual retained coordinator accounting/control.
use super::*;
use crate::coding_endpoint_edit_generation::CodingEndpointEditGeneration;
use agentmage_capability_repository_map::coding_proposal::CODING_PROPOSAL_INSTRUCTION;
use agentmage_kernel_engine::runtime_loop::{
    ReusableRuntimeCoordinator, RuntimeClock, RuntimeCompletionProposal,
    RuntimeCompletionVerificationInput, RuntimeEndpointCoordinatorStep,
    RuntimeEndpointPreparationInput, RuntimeEndpointTurnPort, RuntimePermissionEvaluation,
    RuntimePreparedEndpointTurn, RuntimeRunScope, RuntimeToolBoundary, RuntimeToolExecution,
    RuntimeVerificationInput, RuntimeVerifierPort,
};
use sha2::{Digest, Sha256};
use std::{collections::VecDeque, sync::atomic::AtomicUsize};

fn schema() -> SchemaReference {
    SchemaReference {
        schema_id: SchemaId::from_raw("synthetic.strict-edit.candidate"),
        schema_version: 1,
        schema_sha256: Sha256::digest(b"synthetic strict candidate schema")
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect(),
    }
}

fn input(request: &RuntimeEndpointRunRequest, turn: u32) -> RuntimeEndpointPreparationInput<'_> {
    RuntimeEndpointPreparationInput {
        request,
        turn,
        completed_tool_calls: &[],
        tool_results: &[],
        evidence: &[],
        rejected_tool_calls: &[],
    }
}

struct Turns<'config> {
    selection: EndpointModelSelection,
    turns: VecDeque<CodingEndpointEditGeneration<'config>>,
    prepared: Arc<AtomicUsize>,
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
        self.prepared.fetch_add(1, Ordering::SeqCst);
        if input.turn == 2 {
            assert!(input.rejected_tool_calls.is_empty());
            assert_eq!(input.completed_tool_calls.len(), 1);
            assert_eq!(input.tool_results.len(), 1);
        }
        self.turns
            .pop_front()
            .ok_or(RuntimePortFailure::Invalid)?
            .into_coordinator_turn(input, control, schema())
            .map_err(|e| match e {
                GenerationError::Preparation(e) => e,
                _ => RuntimePortFailure::Invalid,
            })
    }
}

struct Clock(Arc<AtomicU64>);
impl RuntimeClock for Clock {
    fn now_epoch_ms(&mut self) -> Result<u64, RuntimePortFailure> {
        Ok(self.0.load(Ordering::SeqCst))
    }
}

struct CancelAfterEdit {
    calls: Arc<AtomicUsize>,
    signal: CancellationSignal,
}
impl ModelCancellationProbe for CancelAfterEdit {
    fn observe(&self) -> Result<Option<CancellationSignal>, ModelRuntimeFailure> {
        Ok((self.calls.load(Ordering::SeqCst) > 0).then(|| self.signal.clone()))
    }
}

// Observe the actual original native writer. This fixture uses synthetic
// creation provenance, never a replacement native grant or a claimed Git run.
struct Native {
    inner: agentmage_platform_linux::LinuxCodingEndpointEditBoundary<Clock>,
    calls: Arc<AtomicUsize>,
    output: Arc<AtomicUsize>,
    expire: Option<Arc<AtomicU64>>,
}
impl RuntimeToolBoundary for Native {
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
        assert_eq!(result.result.outcome, OperationOutcome::Succeeded);
        self.output.store(
            result.result.output.as_ref().unwrap().bytes.len(),
            Ordering::SeqCst,
        );
        self.calls.fetch_add(1, Ordering::SeqCst);
        if let Some(clock) = &self.expire {
            clock.store(100_000, Ordering::SeqCst);
        }
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

fn hash(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
fn id(prefix: &str, request: &RuntimeEndpointRunRequest, ordinal: u64) -> String {
    format!(
        "{prefix}:{}",
        &hash(format!("{}:{ordinal}", request.run_id.as_str()).as_bytes())[..32]
    )
}

struct Reject(VerifierId, Arc<AtomicUsize>);
impl RuntimeVerifierPort for Reject {
    fn verifier_id(&self) -> &VerifierId {
        &self.0
    }
    fn verify(
        &mut self,
        _: RuntimeVerificationInput<'_>,
    ) -> Result<VerifierCandidate, RuntimePortFailure> {
        panic!("no native verifier")
    }
    fn verify_completion(
        &mut self,
        input: RuntimeCompletionVerificationInput<'_>,
    ) -> Result<VerifierCandidate, RuntimePortFailure> {
        self.1.fetch_add(1, Ordering::SeqCst);
        assert_eq!(input.proposal.payload().unwrap().bytes, good().as_bytes());
        assert_eq!(input.tool_results.len(), 1);
        assert_eq!(input.receipt_ids.len(), 1);
        Ok(VerifierCandidate {
            schema_version: CONTRACT_SCHEMA_VERSION,
            verifier_record_id: VerifierRecordId::from_raw("synthetic-strict-refusal"),
            verifier_id: self.0.clone(),
            task_id: input.request.task().task_id.clone(),
            proposal_id: input.proposal.proposal_id().clone(),
            repository_snapshot_id: input.request.repository_snapshot_id().clone(),
            state_revision: input.state_revision,
            source: VerifierSource::DeterministicPostcondition,
            disposition: VerifierDisposition::NoOp,
            postconditions: input
                .postconditions
                .iter()
                .map(|id| PostconditionResult {
                    postcondition_id: id.clone(),
                    passed: false,
                    evidence: vec![],
                })
                .collect(),
        })
    }
}

#[test]
fn strict_coordinator_conversion_refuses_foreign_invalid_turn_and_stopped_input_inertly() {
    let f = generation_fixture();
    let (listener, endpoint) = socket();
    let startup = generation_startup(&f, &endpoint, &f.empty_ledger());
    for case in 0..5 {
        let mut request = startup.request().clone();
        let mut turn = 1;
        let control = Control::new();
        match case {
            0 => request.task.objective.push_str("tampered"),
            1 => {
                request.task.objective.push_str("foreign seal");
                request = seal_runtime_endpoint_run_request(request).unwrap();
            }
            2 => turn = 0,
            3 => turn = request.limits.max_turns + 1,
            4 => control.0.store(true, Ordering::SeqCst),
            _ => unreachable!(),
        }
        let result = startup
            .prepare_edit_generation(
                connection(configured_transport(endpoint.clone()), endpoint.clone()),
                context(&startup),
                true,
            )
            .unwrap()
            .into_coordinator_turn(input(&request, turn), &control, schema());
        assert_eq!(
            result.err(),
            Some(if case == 4 {
                GenerationError::Preparation(RuntimePortFailure::Cancelled)
            } else {
                GenerationError::Scope
            })
        );
    }
    assert!(matches!(listener.accept(), Err(e) if e.kind()==std::io::ErrorKind::WouldBlock));
}

// One original coordinator performs an exactly approved native edit, then a
// strict request. The second answer never self-approves a second edit or claims
// verified completion. Only unchanged two.py is disclosed in both strict turns.
#[test]
fn strict_generation_uses_original_coordinator_ledger_and_never_self_approves() {
    for case in 0..8 {
        for shared in [false, true] {
            let f = generation_fixture();
            let mut repository = owner(&f);
            let (listener, endpoint) = socket();
            let startup = generation_startup(&f, &endpoint, &f.empty_ledger());
            let mut request = startup.request().clone();
            match case {
                1 => request.limits.max_output_bytes = 64 * 1024,
                2 => request.limits.max_elapsed_ms = 2000,
                3 => request.limits.max_model_calls = 1,
                _ => {}
            }
            let startup = Startup::prepare(
                seal_runtime_endpoint_run_request(request).unwrap(),
                Kind::Edit,
                &f.empty_ledger(),
            )
            .unwrap();
            let preview = startup
                .prepare_edit_proposal(&mut repository, &context(&startup), &good(), 1000)
                .unwrap();
            let expected_requests = if matches!(case, 1..=4 | 7) { 1 } else { 2 };
            let first = json!({"tool_call":{"tool_id":"agentmage.code.apply-prepared-edit",
            "tool_version":"1.0.0","arguments":{"preview":preview.clone()}}})
            .to_string();
            let second = if case == 6 { "x".repeat(65537) } else { good() };
            let expected_output = first.len() + if case == 0 { second.len() } else { 0 };
            let expected_user =
                proposal_context(&startup.request().task.objective, &[("two.py", "two\n")])
                    .user_message()
                    .to_owned();
            let peer = thread::spawn(move || {
                for i in 0..expected_requests {
                    let mut stream = accept(&listener);
                    let request = body(&mut stream);
                    assert_eq!(
                        request["messages"][0]["content"],
                        CODING_PROPOSAL_INSTRUCTION
                    );
                    assert_eq!(request["model"], "synthetic-model");
                    assert_eq!(request["messages"][1]["content"], expected_user);
                    assert!(!expected_user.contains("one.py"));
                    assert!(!expected_user.contains("first"));
                    respond(
                        &mut stream,
                        if i == 0 { &first } else { &second },
                        "stop",
                        !(case == 5 && i == 1),
                    );
                }
                listener
            });
            let mut turns = VecDeque::new();
            let mut bodies = Vec::new();
            let transport = configured_transport(endpoint.clone());
            let approval = LocalChatApproval::for_endpoint(endpoint.clone());
            for _ in 0..2 {
                let connection = if shared {
                    connection(transport.clone(), endpoint.clone())
                } else {
                    CodingEndpointConnection::Borrowed {
                        transport: &transport,
                        approval: &approval,
                    }
                };
                let generation = startup
                    .prepare_edit_generation(
                        connection,
                        proposal_context(&startup.request().task.objective, &[("two.py", "two\n")]),
                        true,
                    )
                    .unwrap();
                bodies.push(generation.binding().body().request_body_bytes as u64);
                turns.push_back(generation);
            }
            let prepared = Arc::new(AtomicUsize::new(0));
            let executed = Arc::new(AtomicUsize::new(0));
            let tool_output = Arc::new(AtomicUsize::new(0));
            let verified = Arc::new(AtomicUsize::new(0));
            let clock = Arc::new(AtomicU64::new(1000));
            let registry = linux_coding_endpoint_edit_registry().unwrap();
            let definition = registry.list_tools()[0];
            let arguments = serde_json::to_vec(&json!({"preview":preview})).unwrap();
            let argument_bytes = arguments.len() as u64;
            let call = ToolCall {
                schema_version: CONTRACT_SCHEMA_VERSION,
                tool_call_id: ToolCallId::from_raw(id("endpoint-call", startup.request(), 1)),
                correlation_id: CorrelationId::from_raw(id("correlation", startup.request(), 0)),
                action_id: agentmage_kernel_engine::runtime_loop::runtime_action_id(
                    &startup.request().run_id,
                    1,
                ),
                tool_id: definition.tool_id.clone(),
                tool_version: definition.tool_version.clone(),
                arguments: ContractPayload {
                    schema: definition.input_schema.clone(),
                    media_type: "application/json".into(),
                    sha256: hash(&arguments),
                    bytes: arguments,
                },
            };
            let native = repository
                .into_endpoint_edit(
                    startup.request().clone(),
                    RuntimeTurnId::from_raw(id("turn", startup.request(), 1)),
                    RuntimeOperationId::from_raw(id("operation", startup.request(), 1)),
                    call,
                )
                .ok()
                .unwrap();
            let mut coordinator = ReusableRuntimeCoordinator::new_endpoint(
                startup.request().clone(),
                Turns {
                    selection: startup.request().model_selection.clone(),
                    turns,
                    prepared: prepared.clone(),
                },
                registry,
                Native {
                    inner: native.into_boundary(Arc::new(Control::new()), Clock(clock.clone())),
                    calls: executed.clone(),
                    output: tool_output.clone(),
                    expire: (case == 4).then(|| clock.clone()),
                },
                Reject(
                    VerifierId::from_raw("synthetic-strict-verifier"),
                    verified.clone(),
                ),
                Clock(clock),
            )
            .unwrap();
            let probe = CancelAfterEdit {
                calls: executed.clone(),
                signal: CancellationSignal {
                    schema_version: CONTRACT_SCHEMA_VERSION,
                    cancellation_id: CancellationId::from_raw("synthetic-strict-cancel"),
                    correlation_id: coordinator.correlation_id().clone(),
                    task_id: startup.request().task.task_id.clone(),
                    reason: CancellationReason::UserRequested,
                    requested_by: BoundaryKind::Shell,
                },
            };
            let cancellation = (case == 7).then_some(&probe as &dyn ModelCancellationProbe);
            let first_boundary = coordinator
                .run_endpoint_until_boundary(None, cancellation)
                .unwrap();
            let RuntimeEndpointCoordinatorStep::AwaitingApproval { challenge } = first_boundary
            else {
                panic!("exact native approval required: {first_boundary:?}")
            };
            assert_eq!(executed.load(Ordering::SeqCst), 0);
            assert_eq!(
                fs::read(f.root.join("owned/worktree/one.py")).unwrap(),
                b"one\n"
            );
            let mut response = RuntimeApprovalResponse {
                schema_version: CONTRACT_SCHEMA_VERSION,
                run_id: challenge.run_id.clone(),
                approval_id: challenge.approval_id.clone(),
                disposition: RuntimeApprovalDisposition::Allow,
                challenge_sha256: "0".repeat(64),
                grant_id: Some(challenge.proposed_grant_id.clone()),
                selection: None,
            };
            let before = coordinator.events().to_vec();
            assert!(
                coordinator
                    .run_endpoint_until_boundary(Some(&response), cancellation)
                    .is_err()
            );
            assert_eq!(coordinator.events(), before);
            assert_eq!(executed.load(Ordering::SeqCst), 0);
            response.challenge_sha256 = challenge.challenge_sha256;
            let terminal = coordinator
                .run_endpoint_until_boundary(Some(&response), cancellation)
                .unwrap();
            let RuntimeEndpointCoordinatorStep::Complete { outcome } = &terminal else {
                panic!("terminal expected")
            };
            assert_eq!(
                outcome.outcome.model_call_count, expected_requests as u32,
                "case={case} shared={shared}: {outcome:?}"
            );
            assert_eq!(outcome.outcome.tool_call_count, 1);
            assert_eq!(executed.load(Ordering::SeqCst), 1);
            assert_eq!(verified.load(Ordering::SeqCst), usize::from(case == 0));
            assert_eq!(
                outcome.outcome.state,
                if case == 7 {
                    AgentStateKind::Cancelled
                } else if matches!(case, 1..=4 | 6) {
                    AgentStateKind::Exhausted
                } else {
                    AgentStateKind::Failed
                }
            );
            let usage = &coordinator.resource_snapshot().cumulative;
            assert_eq!(usage[&BudgetResource::ModelCalls], expected_requests as u64);
            assert_eq!(
                usage[&BudgetResource::InputBytes],
                bodies[..expected_requests].iter().sum::<u64>() + argument_bytes
            );
            assert_eq!(
                usage[&BudgetResource::OutputBytes],
                (expected_output + tool_output.load(Ordering::SeqCst)) as u64
            );
            assert!(usage[&BudgetResource::ElapsedMilliseconds] > 0);
            assert!(outcome.outcome.output.is_none());
            let snapshot = coordinator.resource_snapshot().clone();
            let events = coordinator.events().to_vec();
            let mut sequence = agentmage_kernel_engine::runtime_event::RuntimeEventSequence::new();
            for event in &events {
                sequence.push(event).unwrap();
            }
            assert_eq!(
                coordinator.run_endpoint_until_boundary(None, None).unwrap(),
                terminal
            );
            assert_eq!(coordinator.resource_snapshot(), &snapshot);
            assert_eq!(coordinator.events(), events);
            let native = coordinator
                .ended_tool_boundary()
                .unwrap()
                .inner
                .inspect()
                .unwrap();
            assert!(native["pending"].is_null());
            assert!(!native["latest"].is_null());
            assert_eq!(
                fs::read(f.root.join("owned/worktree/one.py")).unwrap(),
                b"first\n"
            );
            let listener = peer.join().unwrap();
            assert!(
                matches!(listener.accept(), Err(e) if e.kind()==std::io::ErrorKind::WouldBlock)
            );
            assert_eq!(
                prepared.load(Ordering::SeqCst),
                if matches!(case, 3 | 4 | 7) { 1 } else { 2 }
            );
        }
    }
}
