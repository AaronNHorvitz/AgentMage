// SPDX-License-Identifier: Apache-2.0
//! Synthetic full coordinator execution. No model score or coding-workflow acceptance.

use super::*;
use crate::runtime_loop::{
    RuntimeCompletionProposal, RuntimeCompletionVerificationInput,
    RuntimeEndpointCoordinatorStep as EndpointStep, RuntimeEndpointDriver,
    RuntimeEndpointPreparationInput, RuntimeEndpointToolProposal, RuntimeEndpointTurnPort,
    RuntimeNoNativeContext, RuntimePreparedEndpointTurn, RuntimeRunScope,
    endpoint::{
        RuntimeEndpointContextPacket, RuntimeEndpointMessage, RuntimeEndpointMessageRole,
        RuntimeEndpointModelPort,
    },
};
use agentmage_kernel_contracts::{
    EndpointModelBody, EndpointModelFinish, EndpointModelLimits, EndpointModelRequest,
    EndpointModelResult, EndpointModelSelection, ModelOperationControl, RuntimeEndpointRunRequest,
    RuntimeModelBinding,
};

#[derive(Clone, Copy, Debug)]
enum Response {
    Read,
    Effect(GrantOperation),
    Answer,
    Limited,
    WrongDigest,
    MalformedTool,
    MalformedCompletion,
    DuplicateCompletion,
    PositionalTool(u8),
    BadArguments(u8),
    Failure,
    Overflow,
}
struct EndpointPort {
    selection: EndpointModelSelection,
    responses: VecDeque<Response>,
    calls: Arc<AtomicUsize>,
    preparations: Arc<AtomicUsize>,
    clock: Arc<AtomicU64>,
    expire_in_prepare: bool,
    expire_in_stream: bool,
    output_limit: usize,
    observed_results: Arc<Mutex<Vec<ToolResult>>>,
}
struct Dispatch {
    request: EndpointModelRequest,
    response: Response,
    calls: Arc<AtomicUsize>,
    clock: Arc<AtomicU64>,
    expire: bool,
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
    ) -> Result<EndpointModelResult, Self::Error> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        control
            .remaining_ms()
            .map_err(|_| RuntimePortFailure::Cancelled)?;
        if matches!(self.response, Response::Failure) {
            return Err(RuntimePortFailure::Unavailable);
        }
        let text = match self.response {
            Response::Read | Response::MalformedTool => {
                let mut args = payload("fixture.input", br#"{"path":"fixture.txt"}"#);
                if matches!(self.response, Response::MalformedTool) {
                    args.sha256 = "0".repeat(64);
                }
                let arguments = if matches!(self.response, Response::MalformedTool) {
                    serde_json::to_value(args).unwrap()
                } else {
                    serde_json::from_slice::<serde_json::Value>(&args.bytes).unwrap()
                };
                serde_json::json!({"tool_call": {"tool_id":"fixture.read", "tool_version":"1.0.0", "arguments":arguments}}).to_string()
            }
            Response::PositionalTool(form) => {
                let arguments = serde_json::json!({"path":"fixture.txt"});
                let call = serde_json::json!(["fixture.read", "1.0.0", arguments]);
                match form {
                    0 => serde_json::json!([call]),
                    1 => serde_json::json!({"tool_call":call}),
                    2 => serde_json::json!([{
                        "tool_id":"fixture.read", "tool_version":"1.0.0", "arguments":arguments
                    }]),
                    _ => panic!("unknown positional tool form"),
                }.to_string()
            }
            Response::Effect(operation) => {
                let id = match operation {
                    GrantOperation::WorkspaceWrite => "fixture.write",
                    GrantOperation::CommandExecute => "fixture.command",
                    _ => panic!("unsupported synthetic effect"),
                };
                serde_json::json!({"tool_call": {"tool_id":id,"tool_version":"1.0.0","arguments":{"path":"fixture.txt"}}}).to_string()
            }
            Response::MalformedCompletion => "{".into(),
            Response::DuplicateCompletion => r#"{"answer":"done","answer":"other"}"#.into(),
            Response::Overflow => "x".repeat(self.request.limits().response_text_limit_bytes + 1),
            Response::BadArguments(kind) => match kind {
                0 => r#"{"tool_call":{"tool_id":"fixture.read","tool_version":"1.0.0","arguments":{"path":"fixture.txt","path":"elsewhere"}}}"#.into(),
                1 => r#"{"tool_call":{"tool_id":"fixture.read","tool_version":"1.0.0","arguments":{"path":"fixture.txt"},"tool_call_id":"chosen"}}"#.into(),
                2 => r#"{"tool_call":{"tool_id":"fixture.read","tool_version":"1.0.0","arguments":{"path":7}}}"#.into(),
                3 => r#"{"tool_call":{"tool_id":"fixture.read","tool_version":"1.0.0"}}"#.into(),
                4 => r#"{"tool_call":{"tool_id":"fixture.read","tool_version":"1.0.0","arguments":{"path":"fixture.txt"}},"grant_id":"chosen"}"#.into(),
                _ => {
                    let mut arguments = payload("fixture.input", br#"{"path":"fixture.txt"}"#);
                    if kind == 5 { arguments.schema = schema("foreign.input"); }
                    else if kind == 6 { arguments.media_type = "text/plain".into(); }
                    else { arguments.sha256 = "a".repeat(64); }
                    serde_json::json!({"tool_call":{"tool_id":"fixture.read","tool_version":"1.0.0","arguments":arguments}}).to_string()
                }
            },
            _ => "{\"answer\":\"done\"}".into(),
        };
        emit(&text);
        if self.expire {
            self.clock.store(1_000_000, Ordering::SeqCst);
        }
        control
            .remaining_ms()
            .map_err(|_| RuntimePortFailure::TimedOut)?;
        Ok(EndpointModelResult::from_completed_response(
            self.request,
            if matches!(self.response, Response::Limited) {
                EndpointModelFinish::OutputTokenLimit
            } else {
                EndpointModelFinish::Stop
            },
            if matches!(self.response, Response::WrongDigest) {
                "a".repeat(64)
            } else {
                sha256(text.as_bytes())
            },
            text.len(),
            Duration::from_millis(1),
            None,
        ))
    }
}
impl RuntimeEndpointTurnPort for EndpointPort {
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
        self.preparations.fetch_add(1, Ordering::SeqCst);
        *self.observed_results.lock().unwrap() = input.tool_results.to_vec();
        assert_eq!(input.request.model_selection, self.selection);
        assert_eq!(input.turn as usize, self.calls.load(Ordering::SeqCst) + 1);
        if !input.tool_results.is_empty() {
            assert_eq!(input.tool_results.len(), input.turn as usize - 1);
            assert_eq!(input.completed_tool_calls.len(), input.tool_results.len());
            assert_eq!(
                input.tool_results[0].tool_call_id,
                input.completed_tool_calls[0].tool_call_id
            );
            assert_eq!(input.evidence.len(), input.tool_results.len());
        }
        control
            .remaining_ms()
            .map_err(|_| RuntimePortFailure::TimedOut)?;
        if self.expire_in_prepare {
            self.clock.store(1_000_000, Ordering::SeqCst);
        }
        let context = RuntimeEndpointContextPacket::from_messages(&[RuntimeEndpointMessage {
            role: RuntimeEndpointMessageRole::User,
            content: if input.tool_results.is_empty() {
                "read selected file"
            } else {
                "current read received; answer"
            },
        }])?;
        let request = EndpointModelRequest::from_prepared_body(
            self.selection.clone(),
            context.messages().clone(),
            EndpointModelBody {
                request_body_sha256: sha256(b"synthetic approved body"),
                request_body_bytes: 400,
            },
            EndpointModelLimits {
                requested_max_output_tokens: 16,
                provider_usage_requested: false,
                total_timeout: Duration::from_millis(1000),
                idle_timeout: Duration::from_millis(500),
                response_text_limit_bytes: self.output_limit,
                response_body_limit_bytes: 4096,
                response_wire_limit_bytes: 8192,
            },
        );
        let response = self
            .responses
            .pop_front()
            .ok_or(RuntimePortFailure::Unavailable)?;
        Ok(RuntimePreparedEndpointTurn::new(
            context,
            Dispatch {
                request,
                response,
                calls: self.calls.clone(),
                clock: self.clock.clone(),
                expire: self.expire_in_stream,
            },
            schema("runtime.answer"),
        ))
    }
}
struct Clock(Arc<AtomicU64>);
impl RuntimeClock for Clock {
    fn now_epoch_ms(&mut self) -> Result<u64, RuntimePortFailure> {
        Ok(self.0.load(Ordering::SeqCst))
    }
}

struct Boundary {
    executions: Arc<AtomicUsize>,
    ask: bool,
    scope_digest: String,
    result_override: Option<(OperationOutcome, StateChange)>,
    failure_observation: Result<bool, RuntimePortFailure>,
    failure_observation_calls: usize,
    feedback_output: bool,
}
impl Boundary {
    fn check(&self, scope: RuntimeRunScope<'_>) {
        assert_eq!(scope.request_sha256(), self.scope_digest);
        assert!(scope.native_request().is_err());
    }
}
impl RuntimeToolBoundary for Boundary {
    fn permits_failure_observation(
        &mut self,
        scope: RuntimeRunScope<'_>,
        _: &ToolDefinition,
        call: &ToolCall,
        execution: &RuntimeToolExecution,
    ) -> Result<bool, RuntimePortFailure> {
        self.check(scope);
        assert_eq!(execution.result.tool_call_id, call.tool_call_id);
        assert_eq!(execution.result.outcome, OperationOutcome::Failed);
        self.failure_observation_calls += 1;
        self.failure_observation
    }

    fn evaluate(
        &mut self,
        _: &RuntimeRunRequest,
        _: &RuntimeOperationId,
        _: &ToolDefinition,
        _: &ToolCall,
        _: u64,
    ) -> Result<RuntimePermissionEvaluation, RuntimePortFailure> {
        panic!("native entry")
    }
    fn resolve(
        &mut self,
        _: &RuntimeRunRequest,
        _: &agentmage_kernel_contracts::RuntimeApprovalChallenge,
        _: &RuntimeApprovalResponse,
        _: &ToolDefinition,
        _: &ToolCall,
        _: u64,
    ) -> Result<RuntimePermissionEvaluation, RuntimePortFailure> {
        panic!("native entry")
    }
    fn execute(
        &mut self,
        _: &RuntimeRunRequest,
        _: &RuntimePermissionEvaluation,
        _: &ToolDefinition,
        _: &ToolCall,
        _: Option<&dyn agentmage_kernel_contracts::ModelCancellationProbe>,
    ) -> Result<RuntimeToolExecution, RuntimePortFailure> {
        panic!("native entry")
    }
    fn evaluate_scoped(
        &mut self,
        scope: RuntimeRunScope<'_>,
        _: &RuntimeOperationId,
        _: &ToolDefinition,
        _: &ToolCall,
        _: u64,
    ) -> Result<RuntimePermissionEvaluation, RuntimePortFailure> {
        self.check(scope);
        if self.ask {
            Ok(RuntimePermissionEvaluation::Ask {
                approval_id: ApprovalId::from_raw("endpoint-read-approval"),
                grant_id: GrantId::from_raw("endpoint-read-grant"),
                preview_sha256: sha256(b"read selected file"),
                expires_at_epoch_ms: 10_000,
            })
        } else {
            Ok(allow())
        }
    }
    fn resolve_scoped(
        &mut self,
        scope: RuntimeRunScope<'_>,
        _: &agentmage_kernel_contracts::RuntimeApprovalChallenge,
        response: &RuntimeApprovalResponse,
        _: &ToolDefinition,
        _: &ToolCall,
        _: u64,
    ) -> Result<RuntimePermissionEvaluation, RuntimePortFailure> {
        self.check(scope);
        if response.disposition == RuntimeApprovalDisposition::Allow {
            Ok(allow())
        } else {
            Ok(RuntimePermissionEvaluation::Deny {
                approval_id: ApprovalId::from_raw("endpoint-read-approval"),
                grant_id: GrantId::from_raw("endpoint-read-grant"),
                preview_sha256: sha256(b"read selected file"),
                expires_at_epoch_ms: 10_000,
                decision_sha256: sha256(b"deny"),
                reason_code: "runtime.fixture.denied".into(),
            })
        }
    }
    fn execute_scoped(
        &mut self,
        scope: RuntimeRunScope<'_>,
        evaluation: &RuntimePermissionEvaluation,
        definition: &ToolDefinition,
        call: &ToolCall,
        _: Option<&dyn agentmage_kernel_contracts::ModelCancellationProbe>,
    ) -> Result<RuntimeToolExecution, RuntimePortFailure> {
        self.check(scope);
        assert!(matches!(
            evaluation,
            RuntimePermissionEvaluation::Allow { .. }
        ));
        self.executions.fetch_add(1, Ordering::SeqCst);
        let (outcome, state_change) = self.result_override.unwrap_or((
            OperationOutcome::Succeeded,
            if definition.required_grant.operation.operation() == GrantOperation::WorkspaceWrite {
                StateChange::Changed
            } else {
                StateChange::NotChanged
            },
        ));
        Ok(RuntimeToolExecution {
            result: ToolResult {
                schema_version: CONTRACT_SCHEMA_VERSION,
                tool_call_id: call.tool_call_id.clone(),
                correlation_id: call.correlation_id.clone(),
                outcome,
                output: self.feedback_output.then(|| {
                    payload(
                        "fixture.output",
                        br#"{"diagnostic":"synthetic failed assertion"}"#,
                    )
                }),
                validation_issues: vec![],
                evidence: if outcome == OperationOutcome::Uncertain
                    && definition.required_grant.operation.operation()
                        == GrantOperation::WorkspaceRead
                {
                    // An uncertain synthetic read supplies no trusted bytes.
                    vec![]
                } else {
                    vec![evidence(
                        call.tool_call_id.as_str(),
                        EvidenceKind::ToolOutput,
                    )]
                },
                error: None,
                elapsed_ms: 1,
                state_change,
            },
            receipt_id: ReceiptId::from_raw(format!("receipt-{}", call.tool_call_id.as_str())),
            receipt_sha256: sha256(b"actual synthetic read"),
            result_output_kind: self.feedback_output.then_some(RuntimeArtifactKind::Report),
            artifact_candidates: vec![],
        })
    }
}
fn allow() -> RuntimePermissionEvaluation {
    RuntimePermissionEvaluation::Allow {
        approval_id: ApprovalId::from_raw("endpoint-read-approval"),
        grant_id: GrantId::from_raw("endpoint-read-grant"),
        preview_sha256: sha256(b"read selected file"),
        expires_at_epoch_ms: 10_000,
        decision_sha256: sha256(b"allow"),
        authority_sha256: sha256(b"fixture authority"),
    }
}
struct Verifier {
    id: VerifierId,
    mutation: u8,
    expected_results: usize,
    disposition: VerifierDisposition,
}
impl RuntimeVerifierPort for Verifier {
    fn verifier_id(&self) -> &VerifierId {
        &self.id
    }
    fn verify(
        &mut self,
        _: RuntimeVerificationInput<'_>,
    ) -> Result<VerifierCandidate, RuntimePortFailure> {
        panic!("native verifier")
    }
    fn verify_completion(
        &mut self,
        input: RuntimeCompletionVerificationInput<'_>,
    ) -> Result<VerifierCandidate, RuntimePortFailure> {
        assert!(input.proposal.matches_scope(input.request));
        assert!(input.proposal.native_proposal().is_none());
        let good = input.tool_results.len() == self.expected_results
            && input.receipt_ids.len() == self.expected_results
            && input
                .tool_results
                .iter()
                .all(|result| result.outcome == OperationOutcome::Succeeded)
            && input.proposal.payload().unwrap().bytes == br#"{"answer":"done"}"#;
        let mut candidate = VerifierCandidate {
            schema_version: CONTRACT_SCHEMA_VERSION,
            verifier_record_id: VerifierRecordId::from_raw("endpoint-check"),
            verifier_id: self.id.clone(),
            task_id: input.request.task().task_id.clone(),
            proposal_id: input.proposal.proposal_id().clone(),
            repository_snapshot_id: input.request.repository_snapshot_id().clone(),
            state_revision: input.state_revision,
            source: VerifierSource::DeterministicPostcondition,
            disposition: self.disposition,
            postconditions: input
                .postconditions
                .iter()
                .map(|id| PostconditionResult {
                    postcondition_id: id.clone(),
                    passed: good,
                    evidence: input.evidence.to_vec(),
                })
                .collect(),
        };
        match self.mutation {
            1 => {
                candidate.proposal_id =
                    agentmage_kernel_contracts::ProposalId::from_raw("foreign-claim")
            }
            2 => candidate.state_revision += 1,
            3 => candidate.task_id = TaskId::from_raw("foreign-task"),
            4 => {
                candidate.repository_snapshot_id =
                    RepositorySnapshotId::from_raw("foreign-snapshot")
            }
            5 => candidate.verifier_id = VerifierId::from_raw("foreign-verifier"),
            6 => candidate.source = VerifierSource::ModelProse,
            7 => candidate.postconditions.clear(),
            _ => {}
        }
        Ok(candidate)
    }
}
type Coordinator = ReusableRuntimeCoordinator<
    RuntimeEndpointDriver<EndpointPort>,
    RuntimeNoNativeContext,
    Boundary,
    Verifier,
    Clock,
>;
fn endpoint_request() -> RuntimeEndpointRunRequest {
    let native = request(
        profile("endpoint-coordinator-fixture"),
        &registry_for_operation(GrantOperation::WorkspaceRead),
    );
    scope_tests::endpoint(&native)
}
fn build(
    request: RuntimeEndpointRunRequest,
    responses: impl IntoIterator<Item = Response>,
    ask: bool,
) -> Result<Coordinator, RuntimeLoopError> {
    build_registry(
        request,
        responses,
        ask,
        registry_for_operation(GrantOperation::WorkspaceRead),
    )
}
fn build_registry(
    request: RuntimeEndpointRunRequest,
    responses: impl IntoIterator<Item = Response>,
    ask: bool,
    registry: ToolRegistry,
) -> Result<Coordinator, RuntimeLoopError> {
    let clock = Arc::new(AtomicU64::new(1000));
    ReusableRuntimeCoordinator::new_endpoint(
        request.clone(),
        EndpointPort {
            selection: request.model_selection.clone(),
            responses: responses.into_iter().collect(),
            calls: Arc::new(AtomicUsize::new(0)),
            preparations: Arc::new(AtomicUsize::new(0)),
            clock: clock.clone(),
            expire_in_prepare: false,
            expire_in_stream: false,
            output_limit: 2048,
            observed_results: Arc::new(Mutex::new(Vec::new())),
        },
        registry,
        Boundary {
            executions: Arc::new(AtomicUsize::new(0)),
            ask,
            scope_digest: request.request_sha256,
            result_override: None,
            failure_observation: Ok(false),
            failure_observation_calls: 0,
            feedback_output: false,
        },
        Verifier {
            id: VerifierId::from_raw("endpoint-fixture-verifier"),
            mutation: 0,
            expected_results: 1,
            disposition: VerifierDisposition::NoOp,
        },
        Clock(clock),
    )
}
fn coordinator(responses: impl IntoIterator<Item = Response>, ask: bool) -> Coordinator {
    build(endpoint_request(), responses, ask).unwrap()
}
fn terminal(coordinator: &mut Coordinator) -> crate::runtime_coordinator::RuntimeEndpointOutcome {
    let EndpointStep::Complete { outcome } =
        coordinator.run_endpoint_until_boundary(None, None).unwrap()
    else {
        panic!("terminal expected")
    };
    outcome
}

struct InterpretedPort {
    inner: EndpointPort,
    interpreted: Arc<AtomicUsize>,
    mode: u8,
}

struct CompletionOnlyPort {
    inner: EndpointPort,
    narrow_first: bool,
}
impl RuntimeEndpointTurnPort for CompletionOnlyPort {
    fn model_binding(&self) -> RuntimeModelBinding<'_> {
        self.inner.model_binding()
    }
    fn prepare<'a>(
        &mut self,
        input: RuntimeEndpointPreparationInput<'_>,
        control: &dyn ModelOperationControl,
    ) -> Result<RuntimePreparedEndpointTurn<'a>, RuntimePortFailure>
    where
        Self: 'a,
    {
        let turn = self.inner.prepare(input, control)?;
        let turn = if self.narrow_first {
            turn.completion_only()
        } else {
            turn
        };
        let turn =
            turn.with_tool_preparation(|_, _| panic!("completion must not invoke an interpreter"));
        Ok(if self.narrow_first {
            turn
        } else {
            turn.completion_only()
        })
    }
}

#[test]
fn endpoint_completion_only_narrows_before_interpretation_and_preserves_verification() {
    for narrow_first in [false, true] {
        for response in [
            Response::Read,
            Response::BadArguments(2),
            Response::PositionalTool(0),
            Response::PositionalTool(1),
            Response::PositionalTool(2),
            Response::Answer,
        ] {
            let mut original = coordinator([response, Response::Read], true);
            original.verifier.expected_results = 0;
            let calls = original.model.0.calls.clone();
            let executions = original.tool_boundary.executions.clone();
            let mut c = ReusableRuntimeCoordinator::new_endpoint(
                endpoint_request(),
                CompletionOnlyPort {
                    inner: original.model.0,
                    narrow_first,
                },
                original.registry,
                original.tool_boundary,
                original.verifier,
                original.clock,
            )
            .unwrap();
            let EndpointStep::Complete { outcome } =
                c.run_endpoint_until_boundary(None, None).unwrap()
            else {
                panic!("completion-only terminal")
            };
            if matches!(response, Response::Answer) {
                // The original verifier still refuses missing postcondition
                // evidence; completion-only is no substitute for that proof.
                assert_eq!(outcome.outcome.state, AgentStateKind::Failed);
                assert_eq!(
                    outcome.outcome.unresolved_codes,
                    ["runtime.verification.failed"]
                );
            } else {
                assert_eq!(outcome.outcome.state, AgentStateKind::Failed);
                assert_eq!(
                    outcome.outcome.unresolved_codes,
                    ["runtime.proposal.completion-only"]
                );
                assert!(outcome.outcome.output.is_none());
            }
            assert_eq!(calls.load(Ordering::SeqCst), 1);
            assert_eq!(executions.load(Ordering::SeqCst), 0);
            assert_eq!(outcome.outcome.tool_call_count, 0);
            assert!(c.events().iter().all(|e| !matches!(
                e.kind,
                RuntimeEventKind::PermissionRequested { .. }
                    | RuntimeEventKind::PermissionDecided { .. }
                    | RuntimeEventKind::ToolStarted { .. }
                    | RuntimeEventKind::ToolCompleted { .. }
            )));
            let mut sequence = RuntimeEventSequence::new();
            for event in c.events() {
                sequence.push(event).unwrap();
            }
            assert!(matches!(&c.events().last().unwrap().kind,
                RuntimeEventKind::RunTerminal { outcome_sha256, .. } if outcome_sha256 == &outcome.outcome.outcome_sha256));
            let events = c.events().to_vec();
            assert_eq!(
                c.run_endpoint_until_boundary(None, None).unwrap(),
                EndpointStep::Complete { outcome }
            );
            assert_eq!(c.events(), events);
            assert_eq!(calls.load(Ordering::SeqCst), 1);
        }
    }
}
// Synthetic process-held prerequisite; never an oracle or execution grant.
struct CheckedCompletionPort {
    inner: EndpointPort,
    checks: Arc<AtomicUsize>,
    mode: u8,
}
impl RuntimeEndpointTurnPort for CheckedCompletionPort {
    fn model_binding(&self) -> RuntimeModelBinding<'_> {
        self.inner.model_binding()
    }
    fn prepare<'a>(
        &mut self,
        input: RuntimeEndpointPreparationInput<'_>,
        control: &dyn ModelOperationControl,
    ) -> Result<RuntimePreparedEndpointTurn<'a>, RuntimePortFailure>
    where
        Self: 'a,
    {
        let checks = self.checks.clone();
        let calls = self.inner.calls.clone();
        let clock = self.inner.clock.clone();
        let mode = self.mode;
        Ok(self
            .inner
            .prepare(input, control)?
            .completion_only()
            .with_completion_check(move |control| {
                assert_eq!(calls.load(Ordering::SeqCst), 1, "check follows dispatch");
                control
                    .remaining_ms()
                    .map_err(|_| RuntimePortFailure::Cancelled)?;
                checks.fetch_add(1, Ordering::SeqCst);
                if mode == 2 || mode == 3 {
                    clock.store(1_000_000, Ordering::SeqCst);
                }
                match mode {
                    1 | 3 => Err(RuntimePortFailure::Invalid),
                    4 => Err(RuntimePortFailure::Cancelled),
                    5 => Err(RuntimePortFailure::TimedOut),
                    _ => Ok(()),
                }
            }))
    }
}
struct CountCompletionVerifier {
    inner: Verifier,
    calls: Arc<AtomicUsize>,
}
impl RuntimeVerifierPort for CountCompletionVerifier {
    fn verifier_id(&self) -> &VerifierId {
        self.inner.verifier_id()
    }
    fn verify(
        &mut self,
        _: RuntimeVerificationInput<'_>,
    ) -> Result<VerifierCandidate, RuntimePortFailure> {
        panic!("native verification not used")
    }
    fn verify_completion(
        &mut self,
        input: RuntimeCompletionVerificationInput<'_>,
    ) -> Result<VerifierCandidate, RuntimePortFailure> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        self.inner.verify_completion(input)
    }
}

#[test]
fn endpoint_completion_check_runs_once_after_accounting_and_before_verification() {
    for mode in 0..6 {
        for response in [
            Response::Answer,
            Response::Read,
            Response::PositionalTool(0),
            Response::Limited,
            Response::WrongDigest,
            Response::Failure,
            Response::MalformedCompletion,
            Response::DuplicateCompletion,
        ] {
            let original = coordinator([response], false);
            let checks = Arc::new(AtomicUsize::new(0));
            let verifier_calls = Arc::new(AtomicUsize::new(0));
            let mut c = ReusableRuntimeCoordinator::new_endpoint(
                endpoint_request(),
                CheckedCompletionPort {
                    inner: original.model.0,
                    checks: checks.clone(),
                    mode,
                },
                original.registry,
                original.tool_boundary,
                CountCompletionVerifier {
                    inner: original.verifier,
                    calls: verifier_calls.clone(),
                },
                original.clock,
            )
            .unwrap();
            let EndpointStep::Complete { outcome } =
                c.run_endpoint_until_boundary(None, None).unwrap()
            else {
                panic!("terminal")
            };
            let answer = matches!(response, Response::Answer);
            assert_eq!(
                checks.load(Ordering::SeqCst),
                usize::from(answer),
                "mode {mode}, response {response:?}"
            );
            assert_eq!(
                verifier_calls.load(Ordering::SeqCst),
                usize::from(answer && mode == 0)
            );
            assert_eq!(outcome.outcome.model_call_count, 1);
            assert_eq!(outcome.outcome.tool_call_count, 0);
            assert!(outcome.outcome.output.is_none());
            if answer && mode == 1 {
                assert_eq!(
                    outcome.outcome.unresolved_codes,
                    ["runtime.proposal.completion-check-refused"]
                );
            }
            if answer && (mode == 2 || mode == 5) {
                assert_eq!(outcome.outcome.state, AgentStateKind::Exhausted);
            }
            if answer && mode == 3 {
                assert_eq!(outcome.outcome.unresolved_codes, ["runtime.port.invalid"]);
            }
            if answer && mode == 4 {
                assert_eq!(outcome.outcome.unresolved_codes, ["runtime.port.cancelled"]);
            }
            if answer && mode == 5 {
                assert_eq!(outcome.outcome.unresolved_codes, ["runtime.port.timed_out"]);
            }
            assert!(!outcome.outcome.state.is_success());
            assert_eq!(
                c.resource_snapshot().cumulative[&BudgetResource::ModelCalls],
                1
            );
            if answer {
                assert!(c.resource_snapshot().cumulative[&BudgetResource::OutputBytes] >= 17);
                assert!(
                    c.events()
                        .iter()
                        .any(|e| matches!(e.kind, RuntimeEventKind::ModelCompleted { .. }))
                );
            }
            assert!(c.events().iter().all(|e| !matches!(
                e.kind,
                RuntimeEventKind::PermissionRequested { .. } | RuntimeEventKind::ToolStarted { .. }
            )));
            let mut sequence = RuntimeEventSequence::new();
            for event in c.events() {
                sequence.push(event).unwrap();
            }
            let events = c.events().to_vec();
            assert_eq!(
                c.run_endpoint_until_boundary(None, None).unwrap(),
                EndpointStep::Complete { outcome }
            );
            assert_eq!(c.events(), events);
            assert_eq!(
                checks.load(Ordering::SeqCst),
                usize::from(answer),
                "mode {mode}, response {response:?}"
            );
        }
    }
}

impl RuntimeEndpointTurnPort for InterpretedPort {
    fn model_binding(&self) -> RuntimeModelBinding<'_> {
        self.inner.model_binding()
    }
    fn prepare<'a>(
        &mut self,
        input: RuntimeEndpointPreparationInput<'_>,
        control: &dyn ModelOperationControl,
    ) -> Result<RuntimePreparedEndpointTurn<'a>, RuntimePortFailure>
    where
        Self: 'a,
    {
        let first = input.turn == 1;
        let request = input.request.clone();
        let mode = self.mode;
        let interpreted = self.interpreted.clone();
        let clock = self.inner.clock.clone();
        let turn = self.inner.prepare(input, control)?;
        if !first {
            return Ok(turn);
        }
        Ok(turn.with_tool_preparation(move |input, control| {
            assert_eq!(input.request, &request);
            assert_eq!(input.text, r#"{"answer":"done"}"#);
            assert_eq!(
                input.turn_id.as_str(),
                derived_id("turn", request.run_id.as_str(), 1)
            );
            assert_eq!(
                input.operation_id.as_str(),
                derived_id("operation", request.run_id.as_str(), 1)
            );
            assert_eq!(
                input.tool_call_id.as_str(),
                derived_id("endpoint-call", request.run_id.as_str(), 1)
            );
            assert_eq!(input.action_id, &runtime_action_id(&request.run_id, 1));
            control.remaining_ms().unwrap();
            interpreted.fetch_add(1, Ordering::SeqCst);
            if mode == 1 {
                return Err(RuntimePortFailure::Invalid);
            }
            if mode == 2 {
                clock.store(1_000_000, Ordering::SeqCst);
            }
            Ok(RuntimeEndpointToolProposal {
                tool_id: ToolId::from_raw(if mode == 3 {
                    "unoffered.tool"
                } else {
                    "fixture.read"
                }),
                tool_version: "1.0.0".into(),
                arguments: if mode == 4 {
                    serde_json::json!({"path":7})
                } else {
                    serde_json::json!({"path":"fixture.txt"})
                },
            })
        }))
    }
}

#[test]
fn endpoint_interpretation_is_after_accounting_and_before_original_tool_admission() {
    for mode in 0..9 {
        let response = match mode {
            5 => Response::Failure,
            6 => Response::Limited,
            7 => Response::Overflow,
            _ => Response::Answer,
        };
        let mut request = endpoint_request();
        if mode == 8 {
            request.limits.max_tool_calls = 0;
            request =
                crate::runtime_coordinator::seal_runtime_endpoint_run_request(request).unwrap();
        }
        let base = build(request.clone(), [response, Response::Answer], true).unwrap();
        let interpreted = Arc::new(AtomicUsize::new(0));
        let executions = base.tool_boundary.executions.clone();
        let mut c = ReusableRuntimeCoordinator::new_endpoint(
            request,
            InterpretedPort {
                inner: base.model.0,
                interpreted: interpreted.clone(),
                mode,
            },
            registry_for_operation(GrantOperation::WorkspaceRead),
            base.tool_boundary,
            base.verifier,
            base.clock,
        )
        .unwrap();
        let step = c.run_endpoint_until_boundary(None, None).unwrap();
        assert_eq!(interpreted.load(Ordering::SeqCst), usize::from(mode < 5));
        assert_eq!(executions.load(Ordering::SeqCst), 0);
        if mode == 0 {
            let EndpointStep::AwaitingApproval { challenge } = step else {
                panic!("original permission required")
            };
            assert_eq!(
                c.resource_snapshot().cumulative[&BudgetResource::ModelCalls],
                1
            );
            assert_eq!(
                c.resource_snapshot().cumulative[&BudgetResource::OutputBytes],
                17
            );
            let response = RuntimeApprovalResponse {
                schema_version: CONTRACT_SCHEMA_VERSION,
                run_id: challenge.run_id.clone(),
                approval_id: challenge.approval_id.clone(),
                disposition: RuntimeApprovalDisposition::Allow,
                challenge_sha256: challenge.challenge_sha256.clone(),
                grant_id: Some(challenge.proposed_grant_id.clone()),
                selection: None,
            };
            let EndpointStep::Complete { outcome } = c
                .run_endpoint_until_boundary(Some(&response), None)
                .unwrap()
            else {
                panic!("terminal expected")
            };
            assert_eq!(executions.load(Ordering::SeqCst), 1);
            assert_eq!(outcome.outcome.model_call_count, 2);
        } else {
            assert!(matches!(step, EndpointStep::Complete { .. }));
        }
        let mut sequence = RuntimeEventSequence::new();
        for event in c.events() {
            sequence.push(event).unwrap();
        }
    }
}

#[test]
fn endpoint_coordinator_runs_read_approval_observation_verification_and_whole_terminal() {
    let mut coordinator = coordinator([Response::Read, Response::Answer], true);
    assert_eq!(
        coordinator.run_until_boundary(None, None),
        Err(RuntimeLoopError::UnsupportedMode)
    );
    assert!(coordinator.events().is_empty());
    let EndpointStep::AwaitingApproval { challenge } =
        coordinator.run_endpoint_until_boundary(None, None).unwrap()
    else {
        panic!("read approval")
    };
    assert_eq!(
        coordinator.tool_boundary.executions.load(Ordering::SeqCst),
        0
    );
    let mut invalid = decide(&challenge, RuntimeApprovalDisposition::Allow);
    invalid.challenge_sha256 = "0".repeat(64);
    assert!(
        coordinator
            .run_endpoint_until_boundary(Some(&invalid), None)
            .is_err()
    );
    assert_eq!(
        coordinator.tool_boundary.executions.load(Ordering::SeqCst),
        0
    );
    let response = decide(&challenge, RuntimeApprovalDisposition::Allow);
    let EndpointStep::Complete { outcome } = coordinator
        .run_endpoint_until_boundary(Some(&response), None)
        .unwrap()
    else {
        panic!("complete")
    };
    assert_eq!(outcome.outcome.state, AgentStateKind::NoOp);
    assert_eq!(coordinator.model.0.calls.load(Ordering::SeqCst), 2);
    assert_eq!(
        coordinator.tool_boundary.executions.load(Ordering::SeqCst),
        1
    );
    assert_eq!(
        (
            outcome.outcome.turn_count,
            outcome.outcome.model_call_count,
            outcome.outcome.tool_call_count
        ),
        (2, 2, 1)
    );
    assert!(outcome.outcome.answer_evidence.is_none());
    assert!(outcome.answer_evidence.is_some());
    assert!(coordinator.outcome().is_none());
    assert_eq!(coordinator.endpoint_outcome(), Some(&outcome));
    let claim = coordinator
        .endpoint_claim
        .as_ref()
        .unwrap()
        .claim(coordinator.request.scope().unwrap())
        .unwrap();
    crate::runtime_coordinator::verify_runtime_endpoint_outcome(
        &outcome,
        coordinator.request.endpoint().unwrap(),
        Some(&claim),
    )
    .unwrap();
    assert!(
        crate::runtime_coordinator::verify_runtime_endpoint_outcome(
            &outcome,
            coordinator.request.endpoint().unwrap(),
            None
        )
        .is_err()
    );
    assert!(
        matches!(&coordinator.events().last().unwrap().kind,RuntimeEventKind::RunTerminal { outcome_sha256,.. } if outcome_sha256 == &outcome.outcome.outcome_sha256)
    );
    let mut sequence = RuntimeEventSequence::new();
    for event in coordinator.events() {
        sequence.push(event).unwrap();
    }
    let replay = coordinator
        .replay_events(
            None,
            crate::runtime_event::RuntimeEventBatchLimits {
                max_events: 128,
                max_bytes: 1024 * 1024,
            },
        )
        .unwrap();
    assert_eq!(replay.events, coordinator.events());
    assert_eq!(terminal(&mut coordinator), outcome);
    assert_eq!(coordinator.model.0.calls.load(Ordering::SeqCst), 2);
}

#[test]
fn endpoint_coordinator_owning_registry_rejects_foreign_or_advisory_candidates() {
    for mutation in 1..=7 {
        let mut c = coordinator([Response::Read, Response::Answer], false);
        c.verifier.mutation = mutation;
        let result = terminal(&mut c);
        assert_eq!(
            result.outcome.state,
            AgentStateKind::Failed,
            "mutation {mutation}"
        );
        assert!(result.answer_evidence.is_none());
        assert!(result.outcome.output.is_none());
        assert_eq!(c.tool_boundary.executions.load(Ordering::SeqCst), 1);
    }
}

#[test]
fn endpoint_coordinator_unverified_or_incomplete_generation_never_completes_task() {
    for response in [
        Response::Answer,
        Response::Limited,
        Response::WrongDigest,
        Response::Failure,
        Response::Overflow,
    ] {
        let mut c = coordinator([response], false);
        let result = terminal(&mut c);
        assert!(!result.outcome.state.is_success());
        assert!(result.answer_evidence.is_none());
        assert_eq!(c.tool_boundary.executions.load(Ordering::SeqCst), 0);
        assert_eq!(c.model.0.calls.load(Ordering::SeqCst), 1);
        let after = terminal(&mut c);
        assert_eq!(after, result);
        assert_eq!(c.model.0.calls.load(Ordering::SeqCst), 1);
    }
}

#[test]
fn endpoint_coordinator_dispatch_and_preparation_share_original_deadline() {
    for preparation in [true, false] {
        let mut c = coordinator([Response::Read, Response::Answer], false);
        c.model.0.expire_in_prepare = preparation;
        c.model.0.expire_in_stream = !preparation;
        let result = terminal(&mut c);
        assert_eq!(result.outcome.state, AgentStateKind::Exhausted);
        assert_eq!(
            c.model.0.calls.load(Ordering::SeqCst),
            usize::from(!preparation)
        );
        assert_eq!(c.tool_boundary.executions.load(Ordering::SeqCst), 0);
        assert!(result.answer_evidence.is_none());
    }
}

#[test]
fn endpoint_coordinator_run_counts_and_cumulative_bytes_refuse_before_second_dispatch() {
    for case in 0..5 {
        let mut request = endpoint_request();
        match case {
            0 => request.limits.max_model_calls = 1,
            1 => {
                request.limits.max_turns = 1;
                request.limits.max_no_progress_turns = 1;
            }
            2 => request.work_packet.budgets.push(BudgetLimit {
                resource: BudgetResource::InputBytes,
                limit: 600,
            }),
            3 => {
                request
                    .work_packet
                    .budgets
                    .retain(|b| b.resource != BudgetResource::OutputBytes);
                request.work_packet.budgets.push(BudgetLimit {
                    resource: BudgetResource::OutputBytes,
                    limit: 2048,
                });
            }
            4 => {
                request
                    .work_packet
                    .budgets
                    .retain(|b| b.resource != BudgetResource::PlanSteps);
                request.work_packet.budgets.push(BudgetLimit {
                    resource: BudgetResource::PlanSteps,
                    limit: 1,
                });
            }
            _ => unreachable!(),
        }
        if case == 2 {
            let mut seen = false;
            request.work_packet.budgets.retain(|b| {
                if b.resource == BudgetResource::InputBytes {
                    if b.limit == 600 && !seen {
                        seen = true;
                        true
                    } else {
                        false
                    }
                } else {
                    true
                }
            });
        }
        request = crate::runtime_coordinator::seal_runtime_endpoint_run_request(request).unwrap();
        let mut c = build(request, [Response::Read, Response::Answer], false).unwrap();
        let outcome = terminal(&mut c);
        assert_eq!(
            outcome.outcome.state,
            AgentStateKind::Exhausted,
            "case {case}"
        );
        assert_eq!(c.model.0.calls.load(Ordering::SeqCst), 1, "case {case}");
        assert_eq!(
            c.tool_boundary.executions.load(Ordering::SeqCst),
            1,
            "case {case}"
        );
    }
}

#[test]
fn endpoint_coordinator_refuses_authority_modes_catalog_drift_and_native_entry() {
    let original = endpoint_request();
    let mut durable = original.clone();
    durable.mode = RuntimeSessionMode::DurableReadOnly;
    let durable = crate::runtime_coordinator::seal_runtime_endpoint_run_request(durable).unwrap();
    assert!(matches!(
        build(durable, [Response::Read], false),
        Err(RuntimeLoopError::UnsupportedMode)
    ));
    let mut bad = original.clone();
    bad.visible_tools.clear();
    bad.tool_catalog_sha256 =
        runtime_tool_catalog_sha256(&bad.tool_catalog_id, &bad.visible_tools).unwrap();
    let bad = crate::runtime_coordinator::seal_runtime_endpoint_run_request(bad).unwrap();
    assert!(matches!(
        build(bad, [Response::Read], false),
        Err(RuntimeLoopError::ToolCatalogBinding)
    ));
    let mut c = coordinator([Response::Read], false);
    c.model.0.selection.model.push_str("-foreign");
    let calls = c.model.0.calls.clone();
    let mismatch = ReusableRuntimeCoordinator::new_endpoint(
        original.clone(),
        c.model.0,
        c.registry,
        c.tool_boundary,
        c.verifier,
        c.clock,
    );
    assert!(matches!(mismatch, Err(RuntimeLoopError::ModelBinding)));
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    let mut short = original;
    short.limits.max_events = 7;
    let short = crate::runtime_coordinator::seal_runtime_endpoint_run_request(short).unwrap();
    let mut short = build(short, [Response::Read], false).unwrap();
    assert_eq!(
        terminal(&mut short).outcome.state,
        AgentStateKind::Exhausted
    );
    assert_eq!(short.model.0.preparations.load(Ordering::SeqCst), 0);
    assert_eq!(short.model.0.calls.load(Ordering::SeqCst), 0);
    let mut native = super::coordinator([], PermissionScript::Allow, true).0;
    assert_eq!(
        native.run_endpoint_until_boundary(None, None),
        Err(RuntimeLoopError::UnsupportedMode)
    );
    assert!(native.events().is_empty());
}

#[test]
fn endpoint_coordinator_denial_and_malformed_tool_never_execute() {
    let mut c = coordinator([Response::Read, Response::Answer], true);
    let EndpointStep::AwaitingApproval { challenge } =
        c.run_endpoint_until_boundary(None, None).unwrap()
    else {
        panic!("approval")
    };
    let denied = decide(&challenge, RuntimeApprovalDisposition::Deny);
    let EndpointStep::Complete { outcome } =
        c.run_endpoint_until_boundary(Some(&denied), None).unwrap()
    else {
        panic!("terminal")
    };
    assert!(!outcome.outcome.state.is_success());
    assert_eq!(c.tool_boundary.executions.load(Ordering::SeqCst), 0);
    let mut c = coordinator([Response::MalformedTool, Response::Answer], false);
    let outcome = terminal(&mut c);
    assert_eq!(outcome.outcome.state, AgentStateKind::Failed);
    assert_eq!(
        outcome.outcome.unresolved_codes,
        ["runtime.proposal.arguments-invalid"]
    );
    assert!(outcome.answer_evidence.is_none());
    assert!(outcome.outcome.output.is_none());
    assert_eq!(c.endpoint_outcome(), Some(&outcome));
    assert!(
        matches!(&c.events().last().unwrap().kind, RuntimeEventKind::RunTerminal { outcome_sha256, .. } if outcome_sha256 == &outcome.outcome.outcome_sha256)
    );
    assert_eq!(c.tool_boundary.executions.load(Ordering::SeqCst), 0);
    assert_eq!(c.model.0.calls.load(Ordering::SeqCst), 1);
    assert_eq!(terminal(&mut c), outcome);
    assert_eq!(c.model.0.calls.load(Ordering::SeqCst), 1);
}

#[test]
fn endpoint_coordinator_terminal_envelope_faults_latch_even_after_the_envelope_is_restored() {
    for missing in [true, false] {
        let mut c = coordinator([Response::MalformedTool], false);
        let original = terminal(&mut c);
        let events = c.events().to_vec();
        assert!(!c.requires_reconciliation());
        if missing {
            c.endpoint_outcome = None;
        } else {
            c.endpoint_outcome
                .as_mut()
                .unwrap()
                .outcome
                .model_call_count += 1;
        }
        assert_eq!(
            c.run_endpoint_until_boundary(None, None),
            Err(RuntimeLoopError::InvalidBoundaryResult)
        );
        assert!(c.requires_reconciliation());
        // Repairing metadata cannot authorize another advance after a fault.
        c.endpoint_outcome = Some(original);
        assert_eq!(
            c.run_endpoint_until_boundary(None, None),
            Err(RuntimeLoopError::InvalidBoundaryResult)
        );
        assert_eq!(c.events(), events);
        assert_eq!(c.model.0.calls.load(Ordering::SeqCst), 1);
        assert_eq!(c.tool_boundary.executions.load(Ordering::SeqCst), 0);
    }
}

#[test]
fn endpoint_coordinator_model_argument_defects_publish_terminal_refusal_without_dispatching_again()
{
    for kind in 0..8 {
        let mut c = coordinator([Response::BadArguments(kind), Response::Read], false);
        let outcome = terminal(&mut c);
        assert_eq!(outcome.outcome.state, AgentStateKind::Failed, "case {kind}");
        assert_eq!(
            outcome.outcome.unresolved_codes,
            ["runtime.proposal.arguments-invalid"]
        );
        assert!(outcome.outcome.output.is_none());
        assert!(outcome.answer_evidence.is_none());
        assert_eq!(c.tool_boundary.executions.load(Ordering::SeqCst), 0);
        assert_eq!(c.model.0.calls.load(Ordering::SeqCst), 1);
        assert!(
            matches!(&c.events().last().unwrap().kind, RuntimeEventKind::RunTerminal { outcome_sha256, .. } if outcome_sha256 == &outcome.outcome.outcome_sha256)
        );
        let mut sequence = RuntimeEventSequence::new();
        for event in c.events() {
            sequence.push(event).unwrap();
        }
        assert_eq!(terminal(&mut c), outcome);
        assert_eq!(c.model.0.calls.load(Ordering::SeqCst), 1);
    }
}

#[test]
fn endpoint_coordinator_waiting_approval_spends_original_clock_without_another_call() {
    let mut c = coordinator([Response::Read, Response::Answer], true);
    assert!(matches!(
        c.run_endpoint_until_boundary(None, None).unwrap(),
        EndpointStep::AwaitingApproval { .. }
    ));
    c.clock.0.store(1_000_000, Ordering::SeqCst);
    let result = terminal(&mut c);
    assert_eq!(result.outcome.state, AgentStateKind::Exhausted);
    assert_eq!(c.model.0.calls.load(Ordering::SeqCst), 1);
    assert_eq!(c.tool_boundary.executions.load(Ordering::SeqCst), 0);
    assert!(result.answer_evidence.is_none());
}

#[test]
fn endpoint_coordinator_native_only_tool_boundary_refuses_before_legacy_entry() {
    let c = coordinator([Response::Read, Response::Answer], false);
    let (native, executions) = super::coordinator([], PermissionScript::Allow, true);
    let calls = c.model.0.calls.clone();
    let mut c = ReusableRuntimeCoordinator::new_endpoint(
        c.request.endpoint().unwrap().clone(),
        c.model.0,
        c.registry,
        native.tool_boundary,
        c.verifier,
        c.clock,
    )
    .unwrap();
    assert_eq!(
        c.run_endpoint_until_boundary(None, None),
        Err(RuntimeLoopError::Dependency(RuntimePortFailure::Invalid))
    );
    assert_eq!(executions.load(Ordering::SeqCst), 0);
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert_eq!(
        c.run_endpoint_until_boundary(None, None),
        Err(RuntimeLoopError::InvalidBoundaryResult)
    );
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}

#[test]
fn endpoint_coordinator_cancellation_wins_before_pending_allow() {
    let mut c = coordinator([Response::Read, Response::Answer], true);
    let EndpointStep::AwaitingApproval { challenge } =
        c.run_endpoint_until_boundary(None, None).unwrap()
    else {
        panic!("approval")
    };
    let response = decide(&challenge, RuntimeApprovalDisposition::Allow);
    let cancellation = CancellationSignal {
        schema_version: CONTRACT_SCHEMA_VERSION,
        cancellation_id: CancellationId::from_raw("endpoint-cancel"),
        correlation_id: CorrelationId::from_raw(derived_id(
            "correlation",
            c.request.run_id.as_str(),
            0,
        )),
        task_id: c.request.task.task_id.clone(),
        reason: CancellationReason::UserRequested,
        requested_by: BoundaryKind::Shell,
    };
    let EndpointStep::Complete { outcome } = c
        .run_endpoint_until_boundary(Some(&response), Some(&cancellation))
        .unwrap()
    else {
        panic!("terminal")
    };
    assert_eq!(outcome.outcome.state, AgentStateKind::Cancelled);
    assert_eq!(c.tool_boundary.executions.load(Ordering::SeqCst), 0);
    assert_eq!(c.model.0.calls.load(Ordering::SeqCst), 1);
    assert!(outcome.answer_evidence.is_none());
    assert!(
        !c.events()
            .iter()
            .any(|event| matches!(event.kind, RuntimeEventKind::PermissionDecided { .. }))
    );
}

#[path = "runtime_loop_endpoint_write_tests.rs"]
mod controlled_write_tests;

#[test]
fn endpoint_terminal_tool_return_preserves_original_owner_and_refuses_live_approvals() {
    let mut c = Box::new(coordinator([Response::Read, Response::Answer], true));
    let original = &*c as *const Coordinator;
    let executions = c.tool_boundary.executions.clone();
    let calls = c.model.0.calls.clone();
    c = c.try_into_endpoint_tool_boundary().err().unwrap();
    assert_eq!(&*c as *const Coordinator, original);
    assert!(c.events().is_empty());
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    let EndpointStep::AwaitingApproval { challenge } =
        c.run_endpoint_until_boundary(None, None).unwrap()
    else {
        panic!("approval expected")
    };
    let events = c.events().to_vec();
    c = c.try_into_endpoint_tool_boundary().err().unwrap();
    assert_eq!(&*c as *const Coordinator, original);
    assert_eq!(c.events(), events);
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert_eq!(executions.load(Ordering::SeqCst), 0);
    let response = decide(&challenge, RuntimeApprovalDisposition::Allow);
    assert!(matches!(
        c.run_endpoint_until_boundary(Some(&response), None)
            .unwrap(),
        EndpointStep::Complete { .. }
    ));
    let boundary = c.try_into_endpoint_tool_boundary().ok().unwrap();
    assert!(Arc::ptr_eq(&boundary.executions, &executions));
    assert_eq!(executions.load(Ordering::SeqCst), 1);
    assert_eq!(calls.load(Ordering::SeqCst), 2);
}

#[test]
fn endpoint_terminal_tool_return_refuses_uncertainty_faults_and_incomplete_publication() {
    for defect in 0..6 {
        let mut c = Box::new(coordinator([Response::Read, Response::Answer], false));
        if defect == 0 {
            c.tool_boundary.result_override =
                Some((OperationOutcome::Uncertain, StateChange::Uncertain));
        }
        let outcome = terminal(&mut c);
        if defect == 0 {
            assert_eq!(outcome.outcome.state, AgentStateKind::Uncertain);
        }
        match defect {
            0 => {}
            1 => c.correctness_reconciliation_required = true,
            2 => c.suspended = true,
            3 => c.endpoint_outcome = None,
            4 => {
                c.endpoint_outcome
                    .as_mut()
                    .unwrap()
                    .outcome
                    .model_call_count += 1
            }
            5 => {
                c.events.pop();
            }
            _ => unreachable!(),
        }
        let events = c.events().to_vec();
        let executions = c.tool_boundary.executions.load(Ordering::SeqCst);
        let calls = c.model.0.calls.load(Ordering::SeqCst);
        let original = &*c as *const Coordinator;
        let c = c.try_into_endpoint_tool_boundary().err().unwrap();
        assert_eq!(&*c as *const Coordinator, original);
        assert_eq!(c.events(), events);
        assert_eq!(
            c.tool_boundary.executions.load(Ordering::SeqCst),
            executions
        );
        assert_eq!(c.model.0.calls.load(Ordering::SeqCst), calls);
    }
}

#[test]
fn endpoint_terminal_tool_return_allows_known_denial_and_model_failure_without_dispatch() {
    for deny in [false, true] {
        let mut c = Box::new(coordinator(
            if deny {
                vec![Response::Read]
            } else {
                vec![Response::Failure]
            },
            true,
        ));
        let calls = c.model.0.calls.clone();
        let executions = c.tool_boundary.executions.clone();
        if deny {
            let EndpointStep::AwaitingApproval { challenge } =
                c.run_endpoint_until_boundary(None, None).unwrap()
            else {
                panic!("approval expected")
            };
            let response = decide(&challenge, RuntimeApprovalDisposition::Deny);
            assert!(matches!(
                c.run_endpoint_until_boundary(Some(&response), None)
                    .unwrap(),
                EndpointStep::Complete { .. }
            ));
        } else {
            terminal(&mut c);
        }
        let boundary = c.try_into_endpoint_tool_boundary().ok().unwrap();
        assert!(Arc::ptr_eq(&boundary.executions, &executions));
        assert_eq!(executions.load(Ordering::SeqCst), 0);
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }
}
