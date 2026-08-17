use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use agentmage_kernel_contracts::{
    AgentStateKind, AuthorityClass, CONTRACT_SCHEMA_VERSION, ContextSensitivity, CorrelationId,
    EvidenceId, EvidenceKind, EvidenceReference, GrantOperation, ModelCancellationProbe,
    ModelRunId, ReceiptId, RuntimeApprovalChallenge, RuntimeApprovalDisposition,
    RuntimeApprovalResponse, RuntimeArtifactId, RuntimeArtifactRef, RuntimeEvent, RuntimeEventId,
    RuntimeEventKind, RuntimeEventRetention, RuntimeEventRetentionKind, RuntimeOutcome,
    RuntimeOutput, RuntimePayloadReference, RuntimeRunRequest, RuntimeTurnId, SessionCheckpointId,
    ToolId,
};
use agentmage_kernel_engine::{
    runtime_coordinator::{seal_runtime_outcome, verify_runtime_outcome},
    runtime_event::{RuntimeEventSequence, runtime_event_persistence, seal_runtime_event},
    runtime_loop::RuntimeCoordinatorStep,
    workflow_authority::{
        WorkflowAuthorityLayer, WorkflowAuthorityLayerKind, intersect_workflow_authority,
    },
};

use crate::cli_runtime::{NeverCancelInteractiveCli, drive_interactive_cli_runtime};
use crate::coding_client::{
    CodingApprovalPort, CodingClientError, CodingCoordinatorPort, CodingEventSink,
};
use crate::native_chat_runtime::{
    NativeChatPrepareInput, NativeChatRuntimeError, NativeChatRuntimeFactory,
    NativeChatRuntimePort, NativeChatRuntimeService,
};
use crate::workflow_caller::{
    InMemoryWorkflowCaller, WorkflowCallerError, WorkflowCallerIdentity, WorkflowCallerState,
    WorkflowRuntimeSubmission, seal_workflow_runtime_submission,
};

const ZERO_SHA256: &str = "0000000000000000000000000000000000000000000000000000000000000000";

#[derive(Clone)]
struct ParityFixture {
    request: RuntimeRunRequest,
    events: Vec<RuntimeEvent>,
    artifacts: Vec<RuntimeArtifactRef>,
    outcome: RuntimeOutcome,
}

#[derive(Clone, Debug, PartialEq)]
struct ClientProjection {
    request: RuntimeRunRequest,
    events: Vec<RuntimeEvent>,
    artifacts: Vec<RuntimeArtifactRef>,
    evidence: Vec<EvidenceReference>,
    receipts: Vec<ReceiptId>,
    checkpoints: Vec<SessionCheckpointId>,
    outcome: RuntimeOutcome,
}

#[test]
fn story_50_2_read_only_and_coding_packets_are_equal_across_all_three_callers() {
    let (request, events, outcome, observed_result) = completed_native_read_fixture();
    assert!(observed_result);
    assert_three_client_parity(ParityFixture {
        request,
        events,
        artifacts: Vec::new(),
        outcome,
    });
    assert_three_client_parity(controlled_write_fixture());
}

#[test]
fn story_50_2_narrow_workflow_authority_rejects_every_broadening_without_execution() {
    let fixture = controlled_write_fixture();
    let submission = workflow_submission(fixture.request.clone());
    assert!(!submission.authority.unattended_approval);
    assert!(
        !submission
            .authority
            .operations
            .contains(&GrantOperation::GitPush)
    );
    assert!(
        !submission
            .authority
            .operations
            .contains(&GrantOperation::WorkspaceWrite)
    );

    let mut aggregation = submission.clone();
    aggregation
        .authority
        .operations
        .push(GrantOperation::GitPush);
    aggregation.authority.operations.sort();

    let mut simulated_approval = submission.clone();
    simulated_approval.authority.unattended_approval = true;

    let mut tool_expansion = submission.clone();
    tool_expansion
        .requested_tool_ids
        .push(ToolId::from_raw("unreviewed.tool"));
    tool_expansion.requested_tool_ids.sort();

    let mut root_expansion = submission.clone();
    root_expansion.runtime_request.workspace_snapshot_sha256 = "f".repeat(64);

    let mut model_switch = submission.clone();
    model_switch.runtime_request.model_profile.profile_id =
        agentmage_kernel_contracts::ModelProfileId::from_raw("unreviewed-model");

    let mut hidden_retry = submission.clone();
    hidden_retry.runtime_request.limits.max_model_calls += 1;

    let mut result_as_authority = submission.clone();
    result_as_authority.authority.source_sha256s[0] = fixture.outcome.outcome_sha256.clone();
    result_as_authority
        .authority
        .operations
        .push(GrantOperation::GitPush);
    result_as_authority.authority.operations.sort();

    let mut child_spawn = submission;
    child_spawn
        .requested_tool_ids
        .push(ToolId::from_raw("workflow.child.spawn"));
    child_spawn.requested_tool_ids.sort();

    for attempted in [
        aggregation,
        simulated_approval,
        tool_expansion,
        root_expansion,
        model_switch,
        hidden_retry,
        result_as_authority,
        child_spawn,
    ] {
        assert_submission_denied_without_execution(attempted);
    }
}

fn assert_submission_denied_without_execution(submission: WorkflowRuntimeSubmission) {
    let advances = Arc::new(AtomicUsize::new(0));
    let runtime = CountingCoordinator {
        advances: Arc::clone(&advances),
    };
    assert!(matches!(
        InMemoryWorkflowCaller::submit(runtime, submission),
        Err(WorkflowCallerError::SubmissionDenied)
    ));
    assert_eq!(advances.load(Ordering::SeqCst), 0);
}

struct CountingCoordinator {
    advances: Arc<AtomicUsize>,
}

impl CodingCoordinatorPort for CountingCoordinator {
    fn advance(
        &mut self,
        _response: Option<&RuntimeApprovalResponse>,
        _cancellation: Option<&dyn ModelCancellationProbe>,
    ) -> Result<RuntimeCoordinatorStep, CodingClientError> {
        self.advances.fetch_add(1, Ordering::SeqCst);
        Err(CodingClientError::Runtime)
    }

    fn runtime_events(&self) -> &[RuntimeEvent] {
        &[]
    }

    fn runtime_artifacts(&self) -> &[RuntimeArtifactRef] {
        &[]
    }
}

fn assert_three_client_parity(fixture: ParityFixture) {
    let native = run_native_chat(&fixture);
    let cli = run_interactive_cli(&fixture);
    let workflow = run_workflow_caller(&fixture);

    assert_eq!(native, cli);
    assert_eq!(native, workflow);
    assert_eq!(native.request, fixture.request);
    assert_eq!(native.events, fixture.events);
    assert_eq!(native.artifacts, fixture.artifacts);
    assert_eq!(native.outcome, fixture.outcome);
    assert_eq!(native.evidence, fixture.outcome.evidence);
    assert_eq!(native.receipts, fixture.outcome.receipt_ids);

    let expected_checkpoints = fixture
        .events
        .iter()
        .filter_map(|event| match &event.kind {
            RuntimeEventKind::CheckpointCommitted { checkpoint_id, .. } => {
                Some(checkpoint_id.clone())
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(native.checkpoints, expected_checkpoints);
}

fn run_native_chat(fixture: &ParityFixture) -> ClientProjection {
    let input = prepare_input(&fixture.request);
    let mut service = NativeChatRuntimeService::new(ReplayFactory::new(input.clone(), fixture));
    let request = service.prepare(input).expect("native request prepared");
    let step = service.start(request.clone()).expect("native run started");
    service
        .release(&request.run_id, &request.request_sha256)
        .expect("native terminal released");
    projection(
        request,
        step.events,
        step.artifacts,
        step.outcome.expect("native terminal outcome"),
    )
}

fn run_interactive_cli(fixture: &ParityFixture) -> ClientProjection {
    let input = prepare_input(&fixture.request);
    let mut service = NativeChatRuntimeService::new(ReplayFactory::new(input.clone(), fixture));
    let mut sink = RecordingSink::default();
    let result = drive_interactive_cli_runtime(
        &mut service,
        input,
        &mut UnexpectedApproval,
        &mut sink,
        &mut NeverCancelInteractiveCli,
    )
    .expect("interactive CLI reaches the same terminal boundary");
    projection(
        result.request,
        sink.events,
        result.artifacts,
        result.outcome,
    )
}

fn run_workflow_caller(fixture: &ParityFixture) -> ClientProjection {
    let submission = workflow_submission(fixture.request.clone());
    let mut caller = InMemoryWorkflowCaller::submit(
        ReplayCoordinator::from_fixture(fixture),
        submission.clone(),
    )
    .expect("workflow submission admitted");
    let step = caller.advance(None).expect("workflow caller completes");
    assert_eq!(step.state, WorkflowCallerState::Terminal);
    assert_eq!(step.evidence, fixture.outcome.evidence);
    projection(
        submission.runtime_request,
        step.events,
        step.artifacts,
        step.outcome.expect("workflow terminal outcome"),
    )
}

fn projection(
    request: RuntimeRunRequest,
    events: Vec<RuntimeEvent>,
    artifacts: Vec<RuntimeArtifactRef>,
    outcome: RuntimeOutcome,
) -> ClientProjection {
    ClientProjection {
        request,
        checkpoints: events
            .iter()
            .filter_map(|event| match &event.kind {
                RuntimeEventKind::CheckpointCommitted { checkpoint_id, .. } => {
                    Some(checkpoint_id.clone())
                }
                _ => None,
            })
            .collect(),
        events,
        artifacts,
        evidence: outcome.evidence.clone(),
        receipts: outcome.receipt_ids.clone(),
        outcome,
    }
}

#[derive(Clone)]
struct ReplayCoordinator {
    events: Vec<RuntimeEvent>,
    artifacts: Vec<RuntimeArtifactRef>,
    outcome: RuntimeOutcome,
}

impl ReplayCoordinator {
    fn from_fixture(fixture: &ParityFixture) -> Self {
        Self {
            events: fixture.events.clone(),
            artifacts: fixture.artifacts.clone(),
            outcome: fixture.outcome.clone(),
        }
    }
}

impl CodingCoordinatorPort for ReplayCoordinator {
    fn advance(
        &mut self,
        response: Option<&RuntimeApprovalResponse>,
        cancellation: Option<&dyn ModelCancellationProbe>,
    ) -> Result<RuntimeCoordinatorStep, CodingClientError> {
        if response.is_some() || cancellation.is_some() {
            return Err(CodingClientError::Runtime);
        }
        Ok(RuntimeCoordinatorStep::Complete {
            outcome: self.outcome.clone(),
        })
    }

    fn runtime_events(&self) -> &[RuntimeEvent] {
        &self.events
    }

    fn runtime_artifacts(&self) -> &[RuntimeArtifactRef] {
        &self.artifacts
    }
}

struct ReplayFactory {
    input: NativeChatPrepareInput,
    request: RuntimeRunRequest,
    coordinator: ReplayCoordinator,
}

impl ReplayFactory {
    fn new(input: NativeChatPrepareInput, fixture: &ParityFixture) -> Self {
        Self {
            input,
            request: fixture.request.clone(),
            coordinator: ReplayCoordinator::from_fixture(fixture),
        }
    }
}

impl NativeChatRuntimeFactory for ReplayFactory {
    type Coordinator = ReplayCoordinator;

    fn prepare_runtime_request(
        &mut self,
        input: &NativeChatPrepareInput,
    ) -> Result<RuntimeRunRequest, NativeChatRuntimeError> {
        if input != &self.input {
            return Err(NativeChatRuntimeError::RequestDenied);
        }
        Ok(self.request.clone())
    }

    fn compose_runtime(
        &mut self,
        request: &RuntimeRunRequest,
    ) -> Result<Self::Coordinator, NativeChatRuntimeError> {
        if request != &self.request {
            return Err(NativeChatRuntimeError::RequestDenied);
        }
        Ok(self.coordinator.clone())
    }
}

#[derive(Default)]
struct RecordingSink {
    events: Vec<RuntimeEvent>,
}

impl CodingEventSink for RecordingSink {
    fn present(&mut self, event: &RuntimeEvent) -> Result<(), CodingClientError> {
        self.events.push(event.clone());
        Ok(())
    }
}

struct UnexpectedApproval;

impl CodingApprovalPort for UnexpectedApproval {
    fn decide(
        &mut self,
        _challenge: &RuntimeApprovalChallenge,
    ) -> Result<RuntimeApprovalDisposition, CodingClientError> {
        Err(CodingClientError::Approval)
    }
}

fn prepare_input(request: &RuntimeRunRequest) -> NativeChatPrepareInput {
    NativeChatPrepareInput {
        profile_id: request.model_profile.profile_id.as_str().to_owned(),
        expected_entry_sha256: "a".repeat(64),
        workspace_id: request.workspace_id.as_str().to_owned(),
        workspace_root: "/tmp/agentmage-runtime-parity".to_owned(),
        prompt: request.task.objective.clone(),
    }
}

fn workflow_submission(request: RuntimeRunRequest) -> WorkflowRuntimeSubmission {
    let tool_ids = request
        .visible_tools
        .iter()
        .map(|tool| tool.tool_id.clone())
        .collect::<Vec<_>>();
    let mut targets = vec![
        request.workspace_snapshot_sha256.clone(),
        request.repository_snapshot_sha256.clone(),
    ];
    targets.sort();
    targets.dedup();
    let layers = WorkflowAuthorityLayerKind::ALL
        .into_iter()
        .enumerate()
        .map(|(index, kind)| WorkflowAuthorityLayer {
            kind,
            operations: vec![
                GrantOperation::WorkspaceRead,
                GrantOperation::ModelInference,
            ],
            tool_ids: tool_ids.clone(),
            target_scope_sha256s: targets.clone(),
            source_sha256: format!("{}", index + 1).repeat(64),
        })
        .collect();
    seal_workflow_runtime_submission(WorkflowRuntimeSubmission {
        schema_version: CONTRACT_SCHEMA_VERSION,
        caller: WorkflowCallerIdentity {
            caller_id: "parity-caller".to_owned(),
            workflow_id: "parity-workflow".to_owned(),
            node_id: "parity-node".to_owned(),
            parent_invocation_id: "parity-parent".to_owned(),
        },
        work_packet: request.work_packet.clone(),
        requested_tool_ids: tool_ids,
        runtime_request: request,
        authority: intersect_workflow_authority(layers).expect("narrow parity authority"),
        submission_sha256: ZERO_SHA256.to_owned(),
    })
    .expect("workflow submission seals")
}

fn controlled_write_fixture() -> ParityFixture {
    let (_, request) = crate::coding_run::tests::fixture_profile_and_request();
    assert_eq!(
        request.work_packet.required_capability_class,
        AuthorityClass::LocalWrite
    );
    let turn_id = RuntimeTurnId::from_raw("parity-coding-turn-0001");
    let artifact_id = RuntimeArtifactId::from_raw("parity-coding-artifact-0001");
    let payload = RuntimePayloadReference {
        artifact_id: artifact_id.clone(),
        sha256: "d".repeat(64),
        byte_size: 31,
        media_type: "text/plain".to_owned(),
    };
    let artifact = RuntimeArtifactRef {
        schema_version: CONTRACT_SCHEMA_VERSION,
        artifact_id: artifact_id.clone(),
        manifest_sha256: "e".repeat(64),
        payload_sha256: payload.sha256.clone(),
        byte_size: payload.byte_size,
        media_type: payload.media_type.clone(),
    };
    let mut stream = FixtureStream::new(&request, "parity-coding");
    let mut events = vec![
        stream.event(
            RuntimeEventKind::RunStarted {
                request_sha256: request.request_sha256.clone(),
            },
            None,
            None,
        ),
        stream.event(RuntimeEventKind::TurnStarted, Some(&turn_id), None),
        stream.event(
            RuntimeEventKind::ModelRequested {
                model_run_id: ModelRunId::from_raw("parity-coding-model-0001"),
                request_sha256: "b".repeat(64),
            },
            Some(&turn_id),
            None,
        ),
        stream.event(
            RuntimeEventKind::ModelCompleted {
                model_run_id: ModelRunId::from_raw("parity-coding-model-0001"),
                result_sha256: "c".repeat(64),
            },
            Some(&turn_id),
            None,
        ),
        stream.artifact_event(
            RuntimeEventKind::ArtifactCreated {
                artifact_id,
                manifest_sha256: artifact.manifest_sha256.clone(),
            },
            payload.clone(),
            Some(&turn_id),
        ),
        stream.event(
            RuntimeEventKind::TurnCompleted {
                outcome_sha256: "f".repeat(64),
            },
            Some(&turn_id),
            None,
        ),
        stream.event(
            RuntimeEventKind::CheckpointCommitted {
                checkpoint_id: SessionCheckpointId::from_raw("parity-checkpoint-0001"),
                checkpoint_sha256: "a".repeat(64),
            },
            None,
            None,
        ),
    ];
    let outcome = stream.outcome(payload);
    events.push(stream.event(
        RuntimeEventKind::RunTerminal {
            state: outcome.state,
            outcome_sha256: outcome.outcome_sha256.clone(),
        },
        None,
        None,
    ));
    let mut sequence = RuntimeEventSequence::new();
    for event in &events {
        sequence.push(event).expect("coding parity event sequence");
    }
    assert!(sequence.is_terminal());
    verify_runtime_outcome(&outcome, &request).expect("coding parity outcome");
    ParityFixture {
        request,
        events,
        artifacts: vec![artifact],
        outcome,
    }
}

#[derive(Clone)]
struct FixtureStream {
    request: RuntimeRunRequest,
    prefix: String,
    correlation_id: CorrelationId,
    sequence: u64,
    previous_sha256: String,
    causation_event_id: Option<RuntimeEventId>,
    occurred_at_epoch_ms: u64,
}

impl FixtureStream {
    fn new(request: &RuntimeRunRequest, prefix: &str) -> Self {
        Self {
            request: request.clone(),
            prefix: prefix.to_owned(),
            correlation_id: CorrelationId::from_raw(format!("{prefix}-correlation-0001")),
            sequence: 0,
            previous_sha256: ZERO_SHA256.to_owned(),
            causation_event_id: None,
            occurred_at_epoch_ms: 1_000,
        }
    }

    fn event(
        &mut self,
        kind: RuntimeEventKind,
        turn_id: Option<&RuntimeTurnId>,
        operation_id: Option<&agentmage_kernel_contracts::RuntimeOperationId>,
    ) -> RuntimeEvent {
        self.seal_event(kind, None, turn_id, operation_id)
    }

    fn artifact_event(
        &mut self,
        kind: RuntimeEventKind,
        payload_reference: RuntimePayloadReference,
        turn_id: Option<&RuntimeTurnId>,
    ) -> RuntimeEvent {
        self.seal_event(kind, Some(payload_reference), turn_id, None)
    }

    fn seal_event(
        &mut self,
        kind: RuntimeEventKind,
        payload_reference: Option<RuntimePayloadReference>,
        turn_id: Option<&RuntimeTurnId>,
        operation_id: Option<&agentmage_kernel_contracts::RuntimeOperationId>,
    ) -> RuntimeEvent {
        self.occurred_at_epoch_ms += 1;
        let event_id =
            RuntimeEventId::from_raw(format!("{}-event-{:04}", self.prefix, self.sequence));
        let persistence = runtime_event_persistence(&kind);
        let event = seal_runtime_event(RuntimeEvent {
            schema_version: CONTRACT_SCHEMA_VERSION,
            event_id: event_id.clone(),
            run_id: self.request.run_id.clone(),
            session_id: self.request.session_id.clone(),
            task_id: self.request.task.task_id.clone(),
            turn_id: turn_id.cloned(),
            operation_id: operation_id.cloned(),
            correlation_id: self.correlation_id.clone(),
            causation_event_id: self.causation_event_id.clone(),
            sequence: self.sequence,
            occurred_at_epoch_ms: self.occurred_at_epoch_ms,
            sensitivity: ContextSensitivity::Internal,
            retention: RuntimeEventRetention {
                kind: RuntimeEventRetentionKind::Session,
                expires_at_epoch_ms: None,
            },
            persistence,
            policy_id: self.request.policy_id.clone(),
            payload_reference,
            kind,
            previous_event_sha256: self.previous_sha256.clone(),
            event_sha256: ZERO_SHA256.to_owned(),
        })
        .expect("parity event seals");
        self.sequence += 1;
        self.previous_sha256.clone_from(&event.event_sha256);
        self.causation_event_id = Some(event_id);
        event
    }

    fn outcome(&self, payload: RuntimePayloadReference) -> RuntimeOutcome {
        seal_runtime_outcome(
            RuntimeOutcome {
                schema_version: CONTRACT_SCHEMA_VERSION,
                run_id: self.request.run_id.clone(),
                session_id: self.request.session_id.clone(),
                task_id: self.request.task.task_id.clone(),
                request_sha256: self.request.request_sha256.clone(),
                state: AgentStateKind::Success,
                turn_count: 1,
                model_call_count: 1,
                tool_call_count: 0,
                prior_event_id: self
                    .causation_event_id
                    .clone()
                    .expect("checkpoint event exists"),
                prior_event_sha256: self.previous_sha256.clone(),
                evidence: vec![EvidenceReference {
                    schema_version: CONTRACT_SCHEMA_VERSION,
                    evidence_id: EvidenceId::from_raw("parity-coding-evidence-0001"),
                    kind: EvidenceKind::Validation,
                    source_id: "parity-verifier".to_owned(),
                    object_id: "controlled-write-fixture".to_owned(),
                    fragment: None,
                    content_sha256: "9".repeat(64),
                    observed_revision: Some(
                        self.request.repository_snapshot_id.as_str().to_owned(),
                    ),
                }],
                receipt_ids: Vec::new(),
                unresolved_codes: Vec::new(),
                output: Some(RuntimeOutput::Artifact { reference: payload }),
                outcome_sha256: ZERO_SHA256.to_owned(),
            },
            &self.request,
        )
        .expect("parity outcome seals")
    }
}

fn completed_native_read_fixture() -> (RuntimeRunRequest, Vec<RuntimeEvent>, RuntimeOutcome, bool) {
    crate::runtime_read_tests::completed_native_read_fixture()
}
