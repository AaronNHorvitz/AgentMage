use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use agentmage_capability_read_only::{
    NeverCancelled, ReadOnlyEncoding, ReadOnlyLimits, ReadOnlyOutcome, ReadOnlyRequest,
    ReadOnlyToolKind, SnapshotEntry, SnapshotEntryKind, WorkspaceSnapshot, execute_read_only,
    read_only_tool_kind,
};
use agentmage_kernel_contracts::{
    AgentStateKind, ApprovalId, AuthorityClass, BudgetLimit, BudgetResource,
    CONTRACT_SCHEMA_VERSION, ContextPacketId, ContractPayload, DataSensitivity, EvidenceId,
    EvidenceKind, EvidenceReference, ExactModelProfile, GrantId, ModelContextPacket, ModelMessage,
    ModelMessageId, ModelMessageRole, ModelProposalKind, ModelResourceReport, ModelRunRequest,
    ModelRunResult, ModelRunTerminalState, ModelStreamId, ModelToolCallCandidate, OperationOutcome,
    PlanId, PolicyId, PostconditionResult, ReceiptId, RepositorySnapshotId, RollbackPlan,
    RuntimeEvent, RuntimeOperationId, RuntimeOutcome, RuntimeRunId, RuntimeRunLimits,
    RuntimeRunRequest, RuntimeSessionMode, SessionId, StateChange, StopCondition,
    StopConditionKind, Task, TaskId, TaskStatus, ToolCall, ToolCatalogId, ToolDefinition, ToolId,
    ToolResult, VerifierCandidate, VerifierDisposition, VerifierId, VerifierRecordId,
    VerifierSource, WorkPacket, WorkPacketId, WorkPacketState, WorkspaceId, to_canonical_json,
};
use agentmage_kernel_engine::model_codec::proposal_digest;
use agentmage_kernel_engine::runtime_coordinator::{
    runtime_tool_catalog_sha256, seal_runtime_run_request, verify_runtime_outcome,
};
use agentmage_kernel_engine::runtime_event::RuntimeEventSequence;
use agentmage_kernel_engine::runtime_loop::{
    ReusableRuntimeCoordinator, RuntimeClock, RuntimeContextPort, RuntimeCoordinatorStep,
    RuntimeModelPort, RuntimePermissionEvaluation, RuntimePortFailure, RuntimeToolBoundary,
    RuntimeToolExecution, RuntimeVerificationInput, RuntimeVerifierPort, runtime_tool_references,
};
use serde::Deserialize;
use sha2::{Digest, Sha256};

use crate::runtime_tools::read_only_runtime_registry;

const SHA: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const SNAPSHOT_ID: &str = "snapshot-native-read-0001";

#[derive(Deserialize)]
struct TestProfileCatalog {
    profiles: Vec<ExactModelProfile>,
}

struct NativeReadFakeModel {
    profile: ExactModelProfile,
    scripts: VecDeque<ModelProposalKind>,
    arguments: ContractPayload,
    calls: u32,
}

impl RuntimeModelPort for NativeReadFakeModel {
    fn exact_profile(&self) -> &ExactModelProfile {
        &self.profile
    }

    fn run_model(
        &mut self,
        request: &ModelRunRequest,
        _context: &ModelContextPacket,
        _cancellation: Option<&dyn agentmage_kernel_contracts::ModelCancellationProbe>,
    ) -> Result<ModelRunResult, RuntimePortFailure> {
        let kind = self
            .scripts
            .pop_front()
            .ok_or(RuntimePortFailure::ResourceExhausted)?;
        self.calls += 1;
        let (payload, tool_call) = if kind == ModelProposalKind::ToolCall {
            (
                None,
                Some(ModelToolCallCandidate {
                    tool_call_id: agentmage_kernel_contracts::ToolCallId::from_raw(
                        "native-read-call-0001",
                    ),
                    tool_id: ToolId::from_raw(ReadOnlyToolKind::ReadText.id()),
                    tool_version: "1.0.0".to_owned(),
                    arguments: self.arguments.clone(),
                }),
            )
        } else {
            (
                Some(payload(
                    "runtime.native-read.answer",
                    b"lib.rs contains the fixture function",
                )),
                None,
            )
        };
        let mut proposal = agentmage_kernel_contracts::ClosedModelProposal {
            schema_version: CONTRACT_SCHEMA_VERSION,
            proposal_id: agentmage_kernel_contracts::ProposalId::from_raw(format!(
                "native-read-proposal-{}",
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
        Ok(ModelRunResult {
            schema_version: CONTRACT_SCHEMA_VERSION,
            model_run_id: request.model_run_id.clone(),
            stream_id: ModelStreamId::from_raw(format!("native-read-stream-{}", self.calls)),
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

struct NativeReadContext {
    observed_result: Arc<AtomicBool>,
}

impl RuntimeContextPort for NativeReadContext {
    fn build_context(
        &mut self,
        request: &RuntimeRunRequest,
        context_packet_id: ContextPacketId,
        turn: u32,
        tool_results: &[ToolResult],
        evidence: &[EvidenceReference],
    ) -> Result<ModelContextPacket, RuntimePortFailure> {
        if turn == 2 && !tool_results.is_empty() && !evidence.is_empty() {
            self.observed_result.store(true, Ordering::SeqCst);
        }
        let content = payload(
            "runtime.native-read.context",
            format!(
                "turn={turn}; results={}; evidence={}",
                tool_results.len(),
                evidence.len()
            )
            .as_bytes(),
        );
        let mut packet = ModelContextPacket {
            schema_version: CONTRACT_SCHEMA_VERSION,
            context_packet_id,
            session_id: request.session_id.clone(),
            task_id: request.task.task_id.clone(),
            profile_id: request.model_profile.profile_id.clone(),
            manifest_sha256: request.model_profile.manifest_sha256.clone(),
            tool_catalog_id: request.tool_catalog_id.clone(),
            input_bytes: content.bytes.len() as u64,
            input_tokens: 1,
            messages: vec![ModelMessage {
                message_id: ModelMessageId::from_raw(format!("native-read-message-{turn}")),
                role: ModelMessageRole::User,
                content,
            }],
            packet_sha256: "0".repeat(64),
        };
        packet.packet_sha256 = contract_sha256(&packet)?;
        Ok(packet)
    }
}

struct TestClock(u64);

impl RuntimeClock for TestClock {
    fn now_epoch_ms(&mut self) -> Result<u64, RuntimePortFailure> {
        self.0 += 1;
        Ok(self.0)
    }
}

struct SnapshotReadBoundary {
    snapshot: WorkspaceSnapshot,
}

impl RuntimeToolBoundary for SnapshotReadBoundary {
    fn evaluate(
        &mut self,
        _request: &RuntimeRunRequest,
        _operation_id: &RuntimeOperationId,
        _definition: &ToolDefinition,
        call: &ToolCall,
        now_epoch_ms: u64,
    ) -> Result<RuntimePermissionEvaluation, RuntimePortFailure> {
        Ok(RuntimePermissionEvaluation::Allow {
            approval_id: ApprovalId::from_raw("native-read-approval-0001"),
            preview_sha256: sha256(&call.arguments.bytes),
            expires_at_epoch_ms: now_epoch_ms + 10_000,
            grant_id: GrantId::from_raw("native-read-grant-0001"),
            decision_sha256: sha256(b"native read allow"),
            authority_sha256: sha256(b"native read consumed authority"),
        })
    }

    fn resolve(
        &mut self,
        _request: &RuntimeRunRequest,
        _challenge: &agentmage_kernel_contracts::RuntimeApprovalChallenge,
        _response: &agentmage_kernel_contracts::RuntimeApprovalResponse,
        _definition: &ToolDefinition,
        _call: &ToolCall,
        _now_epoch_ms: u64,
    ) -> Result<RuntimePermissionEvaluation, RuntimePortFailure> {
        Err(RuntimePortFailure::Invalid)
    }

    fn execute(
        &mut self,
        request: &RuntimeRunRequest,
        _evaluation: &RuntimePermissionEvaluation,
        definition: &ToolDefinition,
        call: &ToolCall,
        _cancellation: Option<&dyn agentmage_kernel_contracts::ModelCancellationProbe>,
    ) -> Result<RuntimeToolExecution, RuntimePortFailure> {
        let kind = read_only_tool_kind(&call.tool_id, &call.tool_version)
            .ok_or(RuntimePortFailure::Invalid)?;
        let result =
            execute_read_only(kind, &call.arguments.bytes, &self.snapshot, &NeverCancelled);
        if !result.verify(kind) || result.outcome != ReadOnlyOutcome::Succeeded {
            return Err(RuntimePortFailure::Invalid);
        }
        let bytes = serde_json::to_vec(&result).map_err(|_| RuntimePortFailure::Invalid)?;
        let evidence = EvidenceReference {
            schema_version: CONTRACT_SCHEMA_VERSION,
            evidence_id: EvidenceId::from_raw("native-read-evidence-0001"),
            kind: EvidenceKind::ToolOutput,
            source_id: kind.id().to_owned(),
            object_id: "src/lib.rs".to_owned(),
            fragment: Some("bytes:0-64".to_owned()),
            content_sha256: sha256(&bytes),
            observed_revision: Some(request.repository_snapshot_id.as_str().to_owned()),
        };
        let tool_result = ToolResult {
            schema_version: CONTRACT_SCHEMA_VERSION,
            tool_call_id: call.tool_call_id.clone(),
            correlation_id: call.correlation_id.clone(),
            outcome: OperationOutcome::Succeeded,
            output: Some(ContractPayload {
                schema: definition.output_schema.clone(),
                media_type: "application/json".to_owned(),
                sha256: sha256(&bytes),
                bytes,
            }),
            validation_issues: Vec::new(),
            evidence: vec![evidence],
            error: None,
            elapsed_ms: 1,
            state_change: StateChange::NotChanged,
        };
        Ok(RuntimeToolExecution {
            receipt_id: ReceiptId::from_raw("native-read-receipt-0001"),
            receipt_sha256: sha256(b"native-read-receipt-0001"),
            result: tool_result,
        })
    }
}

struct NativeReadVerifier(VerifierId);

impl RuntimeVerifierPort for NativeReadVerifier {
    fn verifier_id(&self) -> &VerifierId {
        &self.0
    }

    fn verify(
        &mut self,
        input: RuntimeVerificationInput<'_>,
    ) -> Result<VerifierCandidate, RuntimePortFailure> {
        Ok(VerifierCandidate {
            schema_version: CONTRACT_SCHEMA_VERSION,
            verifier_record_id: VerifierRecordId::from_raw("native-read-verifier-record-0001"),
            verifier_id: self.0.clone(),
            task_id: input.request.task.task_id.clone(),
            proposal_id: input.proposal.proposal_id.clone(),
            repository_snapshot_id: input.request.repository_snapshot_id.clone(),
            state_revision: input.state_revision,
            source: VerifierSource::DeterministicPostcondition,
            disposition: VerifierDisposition::Success,
            postconditions: input
                .postconditions
                .iter()
                .map(|postcondition_id| PostconditionResult {
                    postcondition_id: postcondition_id.clone(),
                    passed: true,
                    evidence: vec![EvidenceReference {
                        schema_version: CONTRACT_SCHEMA_VERSION,
                        evidence_id: EvidenceId::from_raw("native-read-verifier-evidence-0001"),
                        kind: EvidenceKind::Validation,
                        source_id: "native-read-verifier".to_owned(),
                        object_id: "runtime outcome".to_owned(),
                        fragment: None,
                        content_sha256: SHA.to_owned(),
                        observed_revision: Some(SNAPSHOT_ID.to_owned()),
                    }],
                })
                .collect(),
        })
    }
}

#[test]
fn story_23_4_fake_model_uses_existing_native_read_tool_then_verifies_completion() {
    let (request, events, outcome, observed_result) = completed_native_read_fixture();
    assert_eq!(outcome.state, AgentStateKind::Success);
    assert_eq!(outcome.turn_count, 2);
    assert_eq!(outcome.model_call_count, 2);
    assert_eq!(outcome.tool_call_count, 1);
    assert_eq!(outcome.receipt_ids.len(), 1);
    assert!(observed_result);
    assert!(outcome.evidence.iter().any(|item| {
        item.evidence_id.as_str() == "native-read-evidence-0001"
            && item.observed_revision.as_deref() == Some(SNAPSHOT_ID)
    }));
    verify_runtime_outcome(&outcome, &request).expect("outcome verifies");
    let mut sequence = RuntimeEventSequence::new();
    for event in &events {
        sequence.push(event).expect("ordered runtime event");
    }
    assert!(sequence.is_terminal());
}

pub(crate) fn completed_native_read_fixture()
-> (RuntimeRunRequest, Vec<RuntimeEvent>, RuntimeOutcome, bool) {
    let registry = read_only_runtime_registry().expect("native read registry");
    let profile = deterministic_profile();
    let read_request = ReadOnlyRequest {
        schema_version: 1,
        paths: vec![vec!["src".to_owned(), "lib.rs".to_owned()]],
        query: None,
        byte_offset: Some(0),
        byte_count: Some(256),
        encoding: ReadOnlyEncoding::Utf8,
        limits: ReadOnlyLimits::default(),
        call_depth: 0,
    };
    let read_bytes = serde_json::to_vec(&read_request).expect("read request");
    let read_definition = registry
        .get_tool(&ToolId::from_raw(ReadOnlyToolKind::ReadText.id()), "1.0.0")
        .expect("read tool");
    let arguments = ContractPayload {
        schema: read_definition.input_schema.clone(),
        media_type: "application/json".to_owned(),
        sha256: sha256(&read_bytes),
        bytes: read_bytes,
    };
    let request = runtime_request(profile.clone(), &registry);
    let admitted_request = request.clone();
    let observed_result = Arc::new(AtomicBool::new(false));
    let mut coordinator = ReusableRuntimeCoordinator::new(
        request,
        NativeReadFakeModel {
            profile,
            scripts: [
                ModelProposalKind::ToolCall,
                ModelProposalKind::CompletionCandidate,
            ]
            .into_iter()
            .collect(),
            arguments,
            calls: 0,
        },
        NativeReadContext {
            observed_result: Arc::clone(&observed_result),
        },
        registry,
        SnapshotReadBoundary {
            snapshot: WorkspaceSnapshot {
                entries: vec![SnapshotEntry {
                    path: vec!["src".to_owned(), "lib.rs".to_owned()],
                    kind: SnapshotEntryKind::RegularFile,
                    bytes: b"pub fn fixture() -> u64 { 42 }\n".to_vec(),
                    executable: false,
                }],
            },
        },
        NativeReadVerifier(VerifierId::from_raw("native-read-verifier-0001")),
        TestClock(1_000),
    )
    .expect("coordinator composes native read tool");

    let RuntimeCoordinatorStep::Complete { outcome } = coordinator
        .run_until_boundary(None, None)
        .expect("native read vertical slice completes")
    else {
        panic!("pre-authorized deterministic read cannot pause");
    };
    verify_runtime_outcome(&outcome, &admitted_request).expect("outcome verifies");
    let mut sequence = RuntimeEventSequence::new();
    for event in coordinator.events() {
        sequence.push(event).expect("ordered runtime event");
    }
    assert!(sequence.is_terminal());
    (
        admitted_request,
        coordinator.events().to_vec(),
        outcome,
        observed_result.load(Ordering::SeqCst),
    )
}

fn deterministic_profile() -> ExactModelProfile {
    let catalog: TestProfileCatalog = serde_json::from_str(include_str!(
        "../../../model-profiles/exact-profile-catalog.json"
    ))
    .expect("checked-in profile catalog");
    catalog
        .profiles
        .into_iter()
        .find(|profile| {
            profile.runtime.kind == agentmage_kernel_contracts::ModelRuntimeKind::DeterministicFake
        })
        .expect("deterministic profile")
}

fn runtime_request(
    profile: ExactModelProfile,
    registry: &agentmage_kernel_engine::tooling::ToolRegistry,
) -> RuntimeRunRequest {
    let tool_catalog_id = ToolCatalogId::from_raw("native-read-catalog-0001");
    let visible_tools = runtime_tool_references(registry).expect("runtime references");
    seal_runtime_run_request(RuntimeRunRequest {
        schema_version: CONTRACT_SCHEMA_VERSION,
        run_id: RuntimeRunId::from_raw("native-read-run-0001"),
        session_id: SessionId::from_raw("native-read-session-0001"),
        mode: RuntimeSessionMode::EphemeralReadOnly,
        task: Task {
            schema_version: CONTRACT_SCHEMA_VERSION,
            task_id: TaskId::from_raw("native-read-task-0001"),
            session_id: SessionId::from_raw("native-read-session-0001"),
            objective: "Read src/lib.rs and report the fixture function".to_owned(),
            acceptance_criteria: vec!["Ground the answer in current read evidence".to_owned()],
            constraints: vec!["Read only".to_owned(), "No network".to_owned()],
            status: TaskStatus::Ready,
        },
        work_packet: work_packet(),
        workspace_id: WorkspaceId::from_raw("native-read-workspace-0001"),
        workspace_snapshot_sha256: "b".repeat(64),
        repository_snapshot_id: RepositorySnapshotId::from_raw(SNAPSHOT_ID),
        repository_snapshot_sha256: "c".repeat(64),
        context_budget: profile.context.clone(),
        model_profile: profile,
        tool_catalog_sha256: runtime_tool_catalog_sha256(&tool_catalog_id, &visible_tools)
            .expect("catalog digest"),
        tool_catalog_id,
        visible_tools,
        policy_id: PolicyId::from_raw("native-read-policy-0001"),
        policy_sha256: "d".repeat(64),
        limits: RuntimeRunLimits {
            max_turns: 3,
            max_model_calls: 3,
            max_tool_calls: 2,
            max_repeated_tool_calls: 2,
            max_tool_call_depth: 1,
            max_no_progress_turns: 2,
            max_context_refreshes: 3,
            max_events: 64,
            max_elapsed_ms: 60_000,
            max_output_bytes: 4_096,
        },
        event_cursor: None,
        request_sha256: "0".repeat(64),
    })
    .expect("runtime request seals")
}

fn work_packet() -> WorkPacket {
    WorkPacket {
        schema_version: CONTRACT_SCHEMA_VERSION,
        work_packet_id: WorkPacketId::from_raw("native-read-packet-0001"),
        task_id: TaskId::from_raw("native-read-task-0001"),
        revision: 1,
        objective: "Read src/lib.rs and report the fixture function".to_owned(),
        reason: "Prove the reusable runtime with the existing native read pack".to_owned(),
        owner: "fixture-user".to_owned(),
        authoritative_evidence: Vec::new(),
        mutable_files: Vec::new(),
        protected_files: vec!["src/lib.rs".to_owned()],
        expected_output: "A current evidence-backed answer".to_owned(),
        acceptance_checks: vec!["Ground the answer in current read evidence".to_owned()],
        required_evidence: vec![EvidenceKind::ToolOutput],
        required_capability_class: AuthorityClass::Observe,
        budgets: vec![
            BudgetLimit {
                resource: BudgetResource::PlanSteps,
                limit: 3,
            },
            BudgetLimit {
                resource: BudgetResource::ModelCalls,
                limit: 3,
            },
            BudgetLimit {
                resource: BudgetResource::ToolCalls,
                limit: 2,
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
        plan_id: Some(PlanId::from_raw("native-read-plan-0001")),
        state: WorkPacketState::Active,
    }
}

fn payload(schema_id: &str, bytes: &[u8]) -> ContractPayload {
    ContractPayload {
        schema: agentmage_kernel_contracts::SchemaReference {
            schema_id: agentmage_kernel_contracts::SchemaId::from_raw(schema_id),
            schema_version: 1,
            schema_sha256: SHA.to_owned(),
        },
        media_type: "text/plain".to_owned(),
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
