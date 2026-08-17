use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use agentmage_kernel_contracts::{
    AgentStateKind, ApprovalId, AuthorityClass, BoundaryKind, BudgetLimit, BudgetResource,
    CONTRACT_SCHEMA_VERSION, CancellationId, CancellationReason, CancellationSignal,
    ContextPacketId, ContractPayload, CorrelationId, DataSensitivity, EvidenceId, EvidenceKind,
    EvidenceReference, ExactModelProfile, GrantId, GrantOperation, ModelContextPacket,
    ModelMessage, ModelMessageId, ModelMessageRole, ModelProposalKind, ModelResourceReport,
    ModelRunRequest, ModelRunResult, ModelRunTerminalState, ModelStreamId, ModelToolCallCandidate,
    OperationBinding, OperationOutcome, PlanId, PolicyId, PostconditionResult, ReceiptId,
    RepositorySnapshotId, RequiredGrantTemplate, RollbackPlan, RuntimeApprovalDisposition,
    RuntimeApprovalResponse, RuntimeEventKind, RuntimeOperationId, RuntimeRunId, RuntimeRunLimits,
    RuntimeRunRequest, RuntimeSessionMode, SchemaId, SchemaReference, SessionId, StateChange,
    StopCondition, StopConditionKind, Task, TaskId, TaskStatus, ToolCall, ToolCatalogId,
    ToolDefinition, ToolId, ToolResult, ToolRiskLevel, VerifierCandidate, VerifierDisposition,
    VerifierId, VerifierRecordId, VerifierSource, WorkPacket, WorkPacketId, WorkPacketState,
    WorkspaceId, to_canonical_json,
};
use sha2::{Digest, Sha256};

use super::{
    ReusableRuntimeCoordinator, RuntimeClock, RuntimeContextPort, RuntimeCoordinatorStep,
    RuntimeLoopError, RuntimeModelPort, RuntimePermissionEvaluation, RuntimePortFailure,
    RuntimeToolBoundary, RuntimeToolExecution, RuntimeVerificationInput, RuntimeVerifierPort,
    derived_id, runtime_tool_references,
};
use crate::model_codec::{proposal_digest, tests_support::profile};
use crate::runtime_coordinator::{
    runtime_tool_catalog_sha256, seal_runtime_run_request, verify_runtime_outcome,
};
use crate::runtime_event::RuntimeEventSequence;
use crate::tooling::{Tool, ToolRegistry};

const SHA: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const SNAPSHOT: &str = "snapshot-0001";

#[derive(Clone, Copy)]
enum ModelScript {
    Completion,
    Tool,
    Malformed,
}

struct FakeModel {
    profile: ExactModelProfile,
    scripts: VecDeque<ModelScript>,
    calls: u32,
}

impl FakeModel {
    fn new(profile: ExactModelProfile, scripts: impl IntoIterator<Item = ModelScript>) -> Self {
        Self {
            profile,
            scripts: scripts.into_iter().collect(),
            calls: 0,
        }
    }
}

impl RuntimeModelPort for FakeModel {
    fn exact_profile(&self) -> &ExactModelProfile {
        &self.profile
    }

    fn run_model(
        &mut self,
        request: &ModelRunRequest,
        _context: &ModelContextPacket,
        _cancellation: Option<&dyn agentmage_kernel_contracts::ModelCancellationProbe>,
    ) -> Result<ModelRunResult, RuntimePortFailure> {
        let script = self
            .scripts
            .pop_front()
            .ok_or(RuntimePortFailure::ResourceExhausted)?;
        self.calls += 1;
        let (kind, payload, tool_call) = match script {
            ModelScript::Completion | ModelScript::Malformed => (
                ModelProposalKind::CompletionCandidate,
                Some(payload("runtime.answer", b"verified fixture answer")),
                None,
            ),
            ModelScript::Tool => (
                ModelProposalKind::ToolCall,
                None,
                Some(ModelToolCallCandidate {
                    tool_call_id: agentmage_kernel_contracts::ToolCallId::from_raw(format!(
                        "tool-call-{}",
                        self.calls
                    )),
                    tool_id: ToolId::from_raw("fixture.read"),
                    tool_version: "1.0.0".to_owned(),
                    arguments: payload("fixture.input", br#"{"path":"fixture.txt"}"#),
                }),
            ),
        };
        let mut proposal = agentmage_kernel_contracts::ClosedModelProposal {
            schema_version: CONTRACT_SCHEMA_VERSION,
            proposal_id: agentmage_kernel_contracts::ProposalId::from_raw(format!(
                "proposal-{}",
                self.calls
            )),
            model_run_id: request.model_run_id.clone(),
            context_packet_id: request.context_packet_id.clone(),
            profile_id: request.profile_id.clone(),
            codec_id: self.profile.codec.codec_id.clone(),
            correlation_id: request.correlation_id.clone(),
            kind,
            payload,
            tool_call,
            proposal_sha256: "0".repeat(64),
        };
        proposal.proposal_sha256 =
            proposal_digest(&proposal).map_err(|_| RuntimePortFailure::Invalid)?;
        if matches!(script, ModelScript::Malformed) {
            proposal.proposal_sha256 = "f".repeat(64);
        }
        Ok(ModelRunResult {
            schema_version: CONTRACT_SCHEMA_VERSION,
            model_run_id: request.model_run_id.clone(),
            stream_id: ModelStreamId::from_raw(format!("stream-{}", self.calls)),
            correlation_id: request.correlation_id.clone(),
            terminal_state: ModelRunTerminalState::Proposed,
            fragment_count: 1,
            response_sha256: sha256(proposal.proposal_sha256.as_bytes()),
            proposal: Some(proposal),
            failure: None,
            resources: ModelResourceReport {
                adapter_id: request.adapter_id.clone(),
                profile_id: request.profile_id.clone(),
                model_run_id: Some(request.model_run_id.clone()),
                resident_memory_bytes: 1,
                accelerator_memory_bytes: 0,
                input_tokens: 1,
                output_tokens: 1,
                elapsed_ms: 1,
            },
        })
    }
}

struct FakeContext;

impl RuntimeContextPort for FakeContext {
    fn build_context(
        &mut self,
        request: &RuntimeRunRequest,
        context_packet_id: ContextPacketId,
        turn: u32,
        _tool_results: &[ToolResult],
        _evidence: &[EvidenceReference],
    ) -> Result<ModelContextPacket, RuntimePortFailure> {
        let content = payload(
            "runtime.context",
            format!("turn={turn}; objective={}", request.task.objective).as_bytes(),
        );
        let input_bytes =
            u64::try_from(content.bytes.len()).map_err(|_| RuntimePortFailure::Invalid)?;
        let mut packet = ModelContextPacket {
            schema_version: CONTRACT_SCHEMA_VERSION,
            context_packet_id,
            session_id: request.session_id.clone(),
            task_id: request.task.task_id.clone(),
            profile_id: request.model_profile.profile_id.clone(),
            manifest_sha256: request.model_profile.manifest_sha256.clone(),
            tool_catalog_id: request.tool_catalog_id.clone(),
            messages: vec![ModelMessage {
                message_id: ModelMessageId::from_raw(format!("message-{turn}")),
                role: ModelMessageRole::User,
                content,
            }],
            input_bytes,
            input_tokens: 1,
            packet_sha256: "0".repeat(64),
        };
        packet.packet_sha256 = contract_sha256(&packet)?;
        Ok(packet)
    }
}

struct FakeClock {
    now: u64,
}

impl RuntimeClock for FakeClock {
    fn now_epoch_ms(&mut self) -> Result<u64, RuntimePortFailure> {
        self.now += 1;
        Ok(self.now)
    }
}

#[derive(Clone, Copy)]
enum PermissionScript {
    Allow,
    Ask,
    Expired,
}

struct FakeToolBoundary {
    script: PermissionScript,
    executions: Arc<AtomicUsize>,
    emit_evidence: bool,
}

impl FakeToolBoundary {
    fn evaluation(
        &self,
        challenge: Option<&agentmage_kernel_contracts::RuntimeApprovalChallenge>,
        response: Option<&RuntimeApprovalResponse>,
        now: u64,
    ) -> RuntimePermissionEvaluation {
        let approval_id = challenge.map_or_else(
            || ApprovalId::from_raw("approval-0001"),
            |value| value.approval_id.clone(),
        );
        let preview_sha256 = challenge.map_or_else(
            || sha256(b"fixture preview"),
            |value| value.preview_sha256.clone(),
        );
        let expires_at_epoch_ms =
            challenge.map_or_else(|| now + 10_000, |value| value.expires_at_epoch_ms);
        if matches!(self.script, PermissionScript::Expired) {
            return RuntimePermissionEvaluation::Allow {
                approval_id,
                preview_sha256,
                expires_at_epoch_ms: now,
                grant_id: GrantId::from_raw("grant-0001"),
                decision_sha256: sha256(b"expired decision"),
                authority_sha256: sha256(b"expired authority"),
            };
        }
        if challenge.is_none() && matches!(self.script, PermissionScript::Ask) {
            return RuntimePermissionEvaluation::Ask {
                approval_id,
                preview_sha256,
                expires_at_epoch_ms,
            };
        }
        let disposition = response.map(|value| value.disposition);
        if disposition == Some(RuntimeApprovalDisposition::Deny) {
            RuntimePermissionEvaluation::Deny {
                approval_id,
                preview_sha256,
                expires_at_epoch_ms,
                decision_sha256: sha256(b"deny decision"),
                reason_code: "runtime.fixture.denied".to_owned(),
            }
        } else {
            RuntimePermissionEvaluation::Allow {
                approval_id,
                preview_sha256,
                expires_at_epoch_ms,
                grant_id: response
                    .and_then(|value| value.grant_id.clone())
                    .unwrap_or_else(|| GrantId::from_raw("grant-0001")),
                decision_sha256: sha256(b"allow decision"),
                authority_sha256: sha256(b"allow authority"),
            }
        }
    }
}

impl RuntimeToolBoundary for FakeToolBoundary {
    fn evaluate(
        &mut self,
        _request: &RuntimeRunRequest,
        _operation_id: &RuntimeOperationId,
        _definition: &ToolDefinition,
        _call: &ToolCall,
        now_epoch_ms: u64,
    ) -> Result<RuntimePermissionEvaluation, RuntimePortFailure> {
        Ok(self.evaluation(None, None, now_epoch_ms))
    }

    fn resolve(
        &mut self,
        _request: &RuntimeRunRequest,
        challenge: &agentmage_kernel_contracts::RuntimeApprovalChallenge,
        response: &RuntimeApprovalResponse,
        _definition: &ToolDefinition,
        _call: &ToolCall,
        now_epoch_ms: u64,
    ) -> Result<RuntimePermissionEvaluation, RuntimePortFailure> {
        Ok(self.evaluation(Some(challenge), Some(response), now_epoch_ms))
    }

    fn execute(
        &mut self,
        request: &RuntimeRunRequest,
        _evaluation: &RuntimePermissionEvaluation,
        _definition: &ToolDefinition,
        call: &ToolCall,
        _cancellation: Option<&dyn agentmage_kernel_contracts::ModelCancellationProbe>,
    ) -> Result<RuntimeToolExecution, RuntimePortFailure> {
        let execution = self.executions.fetch_add(1, Ordering::SeqCst) + 1;
        let evidence = if self.emit_evidence {
            vec![evidence(
                &format!("tool-evidence-{execution}"),
                EvidenceKind::ToolOutput,
            )]
        } else {
            Vec::new()
        };
        let result = ToolResult {
            schema_version: CONTRACT_SCHEMA_VERSION,
            tool_call_id: call.tool_call_id.clone(),
            correlation_id: call.correlation_id.clone(),
            outcome: OperationOutcome::Succeeded,
            output: Some(payload("fixture.output", b"fixture contents")),
            validation_issues: Vec::new(),
            evidence,
            error: None,
            elapsed_ms: 1,
            state_change: StateChange::NotChanged,
        };
        let receipt_id = ReceiptId::from_raw(format!("receipt-{execution}"));
        let receipt_sha256 = sha256(
            format!(
                "{}:{}:{}",
                request.run_id.as_str(),
                call.tool_call_id.as_str(),
                receipt_id.as_str()
            )
            .as_bytes(),
        );
        Ok(RuntimeToolExecution {
            receipt_id,
            receipt_sha256,
            result,
        })
    }
}

struct FakeVerifier {
    verifier_id: VerifierId,
    source: VerifierSource,
}

impl RuntimeVerifierPort for FakeVerifier {
    fn verifier_id(&self) -> &VerifierId {
        &self.verifier_id
    }

    fn verify(
        &mut self,
        input: RuntimeVerificationInput<'_>,
    ) -> Result<VerifierCandidate, RuntimePortFailure> {
        Ok(VerifierCandidate {
            schema_version: CONTRACT_SCHEMA_VERSION,
            verifier_record_id: VerifierRecordId::from_raw(format!(
                "verifier-record-{}",
                input.state_revision
            )),
            verifier_id: self.verifier_id.clone(),
            task_id: input.request.task.task_id.clone(),
            proposal_id: input.proposal.proposal_id.clone(),
            repository_snapshot_id: input.request.repository_snapshot_id.clone(),
            state_revision: input.state_revision,
            source: self.source,
            disposition: VerifierDisposition::Success,
            postconditions: input
                .postconditions
                .iter()
                .enumerate()
                .map(|(index, postcondition_id)| PostconditionResult {
                    postcondition_id: postcondition_id.clone(),
                    passed: true,
                    evidence: vec![evidence(
                        &format!("verifier-evidence-{index}"),
                        EvidenceKind::Validation,
                    )],
                })
                .collect(),
        })
    }
}

struct FixtureTool {
    definition: ToolDefinition,
}

impl Tool for FixtureTool {
    fn definition(&self) -> &ToolDefinition {
        &self.definition
    }
}

fn registry() -> ToolRegistry {
    let mut registry = ToolRegistry::new();
    registry
        .register_tool(Box::new(FixtureTool {
            definition: ToolDefinition {
                schema_version: CONTRACT_SCHEMA_VERSION,
                tool_id: ToolId::from_raw("fixture.read"),
                tool_version: "1.0.0".to_owned(),
                display_name: "Fixture reader".to_owned(),
                description: "Reads one deterministic fixture without changing state".to_owned(),
                input_schema: schema("fixture.input"),
                output_schema: schema("fixture.output"),
                risk_level: ToolRiskLevel::Low,
                declared_effects: vec![OperationBinding::new(GrantOperation::WorkspaceRead)],
                required_grant: RequiredGrantTemplate {
                    operation: OperationBinding::new(GrantOperation::WorkspaceRead),
                    target_scope: "fixture.txt".to_owned(),
                    single_use: true,
                },
                timeout_ms: 1_000,
            },
        }))
        .expect("fixture tool registers");
    registry
}

type FixtureCoordinator =
    ReusableRuntimeCoordinator<FakeModel, FakeContext, FakeToolBoundary, FakeVerifier, FakeClock>;

fn coordinator(
    scripts: impl IntoIterator<Item = ModelScript>,
    permission: PermissionScript,
    emit_tool_evidence: bool,
) -> (FixtureCoordinator, Arc<AtomicUsize>) {
    let profile = profile("runtime-loop");
    let registry = registry();
    let request = request(profile.clone(), &registry);
    let executions = Arc::new(AtomicUsize::new(0));
    let coordinator = ReusableRuntimeCoordinator::new(
        request,
        FakeModel::new(profile, scripts),
        FakeContext,
        registry,
        FakeToolBoundary {
            script: permission,
            executions: Arc::clone(&executions),
            emit_evidence: emit_tool_evidence,
        },
        FakeVerifier {
            verifier_id: VerifierId::from_raw("verifier-0001"),
            source: VerifierSource::DeterministicPostcondition,
        },
        FakeClock { now: 1_000 },
    )
    .expect("fixture coordinator builds");
    (coordinator, executions)
}

fn request(profile: ExactModelProfile, registry: &ToolRegistry) -> RuntimeRunRequest {
    let tool_catalog_id = ToolCatalogId::from_raw("catalog-0001");
    let visible_tools = runtime_tool_references(registry).expect("tool references");
    let request = RuntimeRunRequest {
        schema_version: CONTRACT_SCHEMA_VERSION,
        run_id: RuntimeRunId::from_raw("runtime-run-0001"),
        session_id: SessionId::from_raw("session-0001"),
        mode: RuntimeSessionMode::EphemeralReadOnly,
        task: Task {
            schema_version: CONTRACT_SCHEMA_VERSION,
            task_id: TaskId::from_raw("task-0001"),
            session_id: SessionId::from_raw("session-0001"),
            objective: "Inspect one deterministic fixture".to_owned(),
            acceptance_criteria: vec!["Return evidence bound to the fixture snapshot".to_owned()],
            constraints: vec!["Read only".to_owned()],
            status: TaskStatus::Ready,
        },
        work_packet: packet(),
        workspace_id: WorkspaceId::from_raw("workspace-0001"),
        workspace_snapshot_sha256: SHA.to_owned(),
        repository_snapshot_id: RepositorySnapshotId::from_raw(SNAPSHOT),
        repository_snapshot_sha256: "b".repeat(64),
        context_budget: profile.context.clone(),
        model_profile: profile,
        tool_catalog_sha256: runtime_tool_catalog_sha256(&tool_catalog_id, &visible_tools)
            .expect("catalog digest"),
        tool_catalog_id,
        visible_tools,
        policy_id: PolicyId::from_raw("policy-0001"),
        policy_sha256: "c".repeat(64),
        limits: RuntimeRunLimits {
            max_turns: 4,
            max_model_calls: 4,
            max_tool_calls: 4,
            max_repeated_tool_calls: 4,
            max_tool_call_depth: 1,
            max_no_progress_turns: 2,
            max_context_refreshes: 4,
            max_events: 128,
            max_elapsed_ms: 100_000,
            max_output_bytes: 4_096,
        },
        event_cursor: None,
        request_sha256: "0".repeat(64),
    };
    seal_runtime_run_request(request).expect("request seals")
}

fn packet() -> WorkPacket {
    WorkPacket {
        schema_version: CONTRACT_SCHEMA_VERSION,
        work_packet_id: WorkPacketId::from_raw("packet-0001"),
        task_id: TaskId::from_raw("task-0001"),
        revision: 1,
        objective: "Inspect one deterministic fixture".to_owned(),
        reason: "Verify the reusable runtime coordinator".to_owned(),
        owner: "fixture-user".to_owned(),
        authoritative_evidence: Vec::new(),
        mutable_files: Vec::new(),
        protected_files: vec!["fixture.txt".to_owned()],
        expected_output: "A grounded bounded answer".to_owned(),
        acceptance_checks: vec!["Return evidence bound to the fixture snapshot".to_owned()],
        required_evidence: vec![EvidenceKind::Observation],
        required_capability_class: AuthorityClass::Observe,
        budgets: vec![
            BudgetLimit {
                resource: BudgetResource::PlanSteps,
                limit: 4,
            },
            BudgetLimit {
                resource: BudgetResource::ModelCalls,
                limit: 4,
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
            description: "No state change is permitted".to_owned(),
        },
        sensitivity: DataSensitivity::Ephemeral,
        last_verification_date: "2026-08-17".to_owned(),
        next_action: None,
        next_review: None,
        status_reason: None,
        disposition: None,
        completion_evidence: Vec::new(),
        superseding_work: None,
        validation_issues: Vec::new(),
        plan_id: Some(PlanId::from_raw("plan-0001")),
        state: WorkPacketState::Active,
    }
}

fn evidence(id: &str, kind: EvidenceKind) -> EvidenceReference {
    EvidenceReference {
        schema_version: CONTRACT_SCHEMA_VERSION,
        evidence_id: EvidenceId::from_raw(id),
        kind,
        source_id: "fixture".to_owned(),
        object_id: "fixture.txt".to_owned(),
        fragment: Some("record:1".to_owned()),
        content_sha256: SHA.to_owned(),
        observed_revision: Some(SNAPSHOT.to_owned()),
    }
}

fn schema(id: &str) -> SchemaReference {
    SchemaReference {
        schema_id: SchemaId::from_raw(id),
        schema_version: 1,
        schema_sha256: SHA.to_owned(),
    }
}

fn payload(schema_id: &str, bytes: &[u8]) -> ContractPayload {
    ContractPayload {
        schema: schema(schema_id),
        media_type: if bytes.starts_with(b"{") {
            "application/json".to_owned()
        } else {
            "text/plain".to_owned()
        },
        bytes: bytes.to_vec(),
        sha256: sha256(bytes),
    }
}

fn contract_sha256<T: agentmage_kernel_contracts::VersionedContract>(
    value: &T,
) -> Result<String, RuntimePortFailure> {
    to_canonical_json(value)
        .map(|bytes| sha256(&bytes))
        .map_err(|_| RuntimePortFailure::Invalid)
}

fn sha256(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn assert_valid_terminal_stream(coordinator: &FixtureCoordinator) {
    let mut sequence = RuntimeEventSequence::new();
    for event in coordinator.events() {
        sequence.push(event).expect("event sequence remains valid");
    }
    assert!(sequence.is_terminal());
    assert_eq!(sequence.event_count(), coordinator.events().len() as u64);
    verify_runtime_outcome(
        coordinator.outcome().expect("terminal outcome exists"),
        &coordinator.request,
    )
    .expect("outcome remains request-bound");
}

#[test]
fn story_23_4_direct_answer_is_verifier_backed_and_streamed_in_exact_order() {
    let (mut coordinator, executions) =
        coordinator([ModelScript::Completion], PermissionScript::Allow, true);
    let subscription = coordinator.subscribe_events(16).expect("subscriber binds");

    let RuntimeCoordinatorStep::Complete { outcome } = coordinator
        .run_until_boundary(None, None)
        .expect("direct answer completes")
    else {
        panic!("direct answer cannot request tool approval");
    };

    assert_eq!(outcome.state, AgentStateKind::Success);
    assert_eq!(outcome.turn_count, 1);
    assert_eq!(outcome.model_call_count, 1);
    assert_eq!(outcome.tool_call_count, 0);
    assert_eq!(executions.load(Ordering::SeqCst), 0);
    assert_eq!(coordinator.events().len(), 6);
    assert!(matches!(
        coordinator.events()[0].kind,
        RuntimeEventKind::RunStarted { .. }
    ));
    assert!(matches!(
        coordinator.events()[1].kind,
        RuntimeEventKind::TurnStarted
    ));
    assert!(matches!(
        coordinator.events()[2].kind,
        RuntimeEventKind::ModelRequested { .. }
    ));
    assert!(matches!(
        coordinator.events()[3].kind,
        RuntimeEventKind::ModelCompleted { .. }
    ));
    assert!(matches!(
        coordinator.events()[4].kind,
        RuntimeEventKind::TurnCompleted { .. }
    ));
    assert!(matches!(
        coordinator.events()[5].kind,
        RuntimeEventKind::RunTerminal { .. }
    ));

    let mut delivered = Vec::new();
    while let Some(event) = subscription.try_next().expect("subscription remains live") {
        delivered.push(event);
    }
    assert_eq!(delivered, coordinator.events());
    assert_valid_terminal_stream(&coordinator);
}

#[test]
fn story_23_4_model_or_classifier_claims_cannot_mint_success() {
    let (mut coordinator, _) =
        coordinator([ModelScript::Completion], PermissionScript::Allow, true);
    coordinator.verifier.source = VerifierSource::ModelProse;

    let RuntimeCoordinatorStep::Complete { outcome } = coordinator
        .run_until_boundary(None, None)
        .expect("advisory verifier claim closes truthfully")
    else {
        panic!("completion candidate cannot request approval");
    };

    assert_eq!(outcome.state, AgentStateKind::Failed);
    assert_eq!(
        outcome.unresolved_codes,
        ["runtime.verification.failed".to_owned()]
    );
    assert_valid_terminal_stream(&coordinator);
}

#[test]
fn story_23_4_ask_pauses_before_effect_and_exact_allow_resumes_once() {
    let (mut coordinator, executions) = coordinator(
        [ModelScript::Tool, ModelScript::Completion],
        PermissionScript::Ask,
        true,
    );

    let RuntimeCoordinatorStep::AwaitingApproval { challenge } = coordinator
        .run_until_boundary(None, None)
        .expect("tool request pauses")
    else {
        panic!("ASK must pause before tool launch");
    };
    assert_eq!(executions.load(Ordering::SeqCst), 0);
    assert!(matches!(
        coordinator.events().last().map(|event| &event.kind),
        Some(RuntimeEventKind::PermissionRequested { .. })
    ));

    let response = RuntimeApprovalResponse {
        schema_version: CONTRACT_SCHEMA_VERSION,
        run_id: challenge.run_id.clone(),
        approval_id: challenge.approval_id.clone(),
        disposition: RuntimeApprovalDisposition::Allow,
        challenge_sha256: challenge.challenge_sha256.clone(),
        grant_id: Some(GrantId::from_raw("grant-0001")),
    };
    let RuntimeCoordinatorStep::Complete { outcome } = coordinator
        .run_until_boundary(Some(&response), None)
        .expect("exact approval resumes")
    else {
        panic!("consumed approval cannot pause twice");
    };

    assert_eq!(outcome.state, AgentStateKind::Success);
    assert_eq!(outcome.tool_call_count, 1);
    assert_eq!(outcome.receipt_ids.len(), 1);
    assert_eq!(executions.load(Ordering::SeqCst), 1);
    assert_valid_terminal_stream(&coordinator);
}

#[test]
fn story_23_4_deny_closes_without_launching_the_tool() {
    let (mut coordinator, executions) =
        coordinator([ModelScript::Tool], PermissionScript::Ask, true);
    let RuntimeCoordinatorStep::AwaitingApproval { challenge } = coordinator
        .run_until_boundary(None, None)
        .expect("tool request pauses")
    else {
        panic!("ASK must pause");
    };
    let response = RuntimeApprovalResponse {
        schema_version: CONTRACT_SCHEMA_VERSION,
        run_id: challenge.run_id.clone(),
        approval_id: challenge.approval_id.clone(),
        disposition: RuntimeApprovalDisposition::Deny,
        challenge_sha256: challenge.challenge_sha256.clone(),
        grant_id: None,
    };

    let RuntimeCoordinatorStep::Complete { outcome } = coordinator
        .run_until_boundary(Some(&response), None)
        .expect("denial closes")
    else {
        panic!("denial cannot leave approval pending");
    };
    assert_eq!(outcome.state, AgentStateKind::Declined);
    assert_eq!(executions.load(Ordering::SeqCst), 0);
    assert!(
        !coordinator
            .events()
            .iter()
            .any(|event| matches!(event.kind, RuntimeEventKind::ToolStarted { .. }))
    );
    assert_valid_terminal_stream(&coordinator);
}

#[test]
fn story_23_4_cancellation_wins_over_a_pending_allow_response() {
    let (mut coordinator, executions) =
        coordinator([ModelScript::Tool], PermissionScript::Ask, true);
    let RuntimeCoordinatorStep::AwaitingApproval { challenge } = coordinator
        .run_until_boundary(None, None)
        .expect("tool request pauses")
    else {
        panic!("ASK must pause");
    };
    let response = RuntimeApprovalResponse {
        schema_version: CONTRACT_SCHEMA_VERSION,
        run_id: challenge.run_id.clone(),
        approval_id: challenge.approval_id.clone(),
        disposition: RuntimeApprovalDisposition::Allow,
        challenge_sha256: challenge.challenge_sha256.clone(),
        grant_id: Some(GrantId::from_raw("grant-0001")),
    };
    let cancellation = CancellationSignal {
        schema_version: CONTRACT_SCHEMA_VERSION,
        cancellation_id: CancellationId::from_raw("cancellation-0001"),
        correlation_id: CorrelationId::from_raw(derived_id(
            "correlation",
            coordinator.request.run_id.as_str(),
            0,
        )),
        task_id: coordinator.request.task.task_id.clone(),
        reason: CancellationReason::UserRequested,
        requested_by: BoundaryKind::Shell,
    };

    let RuntimeCoordinatorStep::Complete { outcome } = coordinator
        .run_until_boundary(Some(&response), Some(&cancellation))
        .expect("cancellation closes pending approval")
    else {
        panic!("cancellation is terminal");
    };
    assert_eq!(outcome.state, AgentStateKind::Cancelled);
    assert_eq!(executions.load(Ordering::SeqCst), 0);
    assert!(
        !coordinator
            .events()
            .iter()
            .any(|event| matches!(event.kind, RuntimeEventKind::PermissionDecided { .. }))
    );
    assert_valid_terminal_stream(&coordinator);
}

#[test]
fn story_23_4_malformed_model_result_fails_closed_with_terminal_evidence() {
    let (mut coordinator, executions) =
        coordinator([ModelScript::Malformed], PermissionScript::Allow, true);

    let RuntimeCoordinatorStep::Complete { outcome } = coordinator
        .run_until_boundary(None, None)
        .expect("malformed result closes")
    else {
        panic!("malformed completion cannot request approval");
    };
    assert_eq!(outcome.state, AgentStateKind::Failed);
    assert_eq!(executions.load(Ordering::SeqCst), 0);
    assert!(coordinator.events().iter().any(|event| matches!(
        &event.kind,
        RuntimeEventKind::ModelFailed { failure_code, .. }
            if failure_code == "runtime.model.result_invalid"
    )));
    assert_valid_terminal_stream(&coordinator);
}

#[test]
fn story_23_4_expired_allow_is_rejected_before_worker_launch() {
    let (mut coordinator, executions) =
        coordinator([ModelScript::Tool], PermissionScript::Expired, true);

    assert_eq!(
        coordinator.run_until_boundary(None, None),
        Err(RuntimeLoopError::InvalidBoundaryResult)
    );
    assert_eq!(executions.load(Ordering::SeqCst), 0);
    assert!(
        !coordinator
            .events()
            .iter()
            .any(|event| matches!(event.kind, RuntimeEventKind::ToolStarted { .. }))
    );
}
