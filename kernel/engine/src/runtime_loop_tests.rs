use std::collections::VecDeque;
use std::fs;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use std::thread;
use std::time::{Duration, Instant};

use agentmage_kernel_contracts::{
    AgentStateKind, ApprovalId, AuthorityClass, BoundaryKind, BudgetLimit, BudgetResource,
    CONTRACT_SCHEMA_VERSION, CancellationId, CancellationReason, CancellationSignal,
    ContextPacketId, ContractPayload, CorrelationId, DataSensitivity, EvidenceId, EvidenceKind,
    EvidenceReference, ExactModelProfile, GrantId, GrantOperation, ModelContextPacket,
    ModelMessage, ModelMessageId, ModelMessageRole, ModelProposalKind, ModelResourceReport,
    ModelRunRequest, ModelRunResult, ModelRunTerminalState, ModelRuntimeFailure, ModelStreamId,
    ModelToolCallCandidate, OperationBinding, OperationOutcome, PlanId, PlanStepId, PolicyId,
    PostconditionResult, ReceiptId, RepositorySnapshotId, RequiredGrantTemplate, RollbackPlan,
    RuntimeApprovalDisposition, RuntimeApprovalResponse, RuntimeArtifactManifest,
    RuntimeArtifactRef, RuntimeEvent, RuntimeEventKind, RuntimeEventRetentionKind,
    RuntimeOperationId, RuntimeOutput, RuntimeResourceUsage, RuntimeResumeBinding, RuntimeRunId,
    RuntimeRunLimits, RuntimeRunRequest, RuntimeSessionMode, SchemaId, SchemaReference,
    SessionCheckpoint, SessionCheckpointId, SessionId, StateChange, StopCondition,
    StopConditionKind, StorageFilesystemClass, StrictLocalStorageObservation, Task, TaskId,
    TaskStatus, ToolCall, ToolCatalogId, ToolDefinition, ToolId, ToolResult, ToolRiskLevel,
    VerifierCandidate, VerifierDisposition, VerifierId, VerifierRecordId, VerifierSource,
    WorkPacket, WorkPacketId, WorkPacketState, WorkspaceId, to_canonical_json,
};
use sha2::{Digest, Sha256};

use super::{
    MAX_RUNTIME_INLINE_OUTPUT_BYTES, ReusableRuntimeCoordinator, RuntimeArtifactPort,
    RuntimeCheckpointCommit, RuntimeCheckpointPort, RuntimeCheckpointPublication, RuntimeClock,
    RuntimeContextPort, RuntimeCoordinatorStep, RuntimeCorrectnessTransactionPort,
    RuntimeJournalPort, RuntimeLoopError, RuntimeModelPort, RuntimePermissionEvaluation,
    RuntimePortFailure, RuntimeResumeSnapshot, RuntimeToolBoundary, RuntimeToolCorrectnessCommit,
    RuntimeToolExecution, RuntimeVerificationInput, RuntimeVerifierPort, derived_id,
    runtime_action_id, runtime_event_cursor, runtime_tool_references,
};
use crate::context_management::finalize_checkpoint;
use crate::model_codec::{proposal_digest, tests_support::profile};
use crate::operational_store::{
    OperationalStore, OperationalStoreKeyError, OperationalStoreKeyProvider,
};
use crate::runtime_artifact::{
    runtime_artifact_ref, seal_runtime_resume_binding, verify_runtime_artifact_manifest,
};
use crate::runtime_coordinator::{
    runtime_tool_catalog_sha256, seal_runtime_run_request, verify_runtime_outcome,
};
use crate::runtime_event::RuntimeEventSequence;
use crate::runtime_hardening::{
    MAX_RUNTIME_CLIENT_QUEUE_BYTES, MAX_RUNTIME_EVENT_ENVELOPE_BYTES, RuntimeHardeningError,
    RuntimeHardeningLimits, RuntimeResourceLedger,
};
use crate::runtime_journal::{RuntimeJournalError, RuntimeJournalWorker};
use crate::tooling::{Tool, ToolRegistry};

const SHA: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const SNAPSHOT: &str = "snapshot-0001";
static PRESSURE_TEMP_ID: AtomicU64 = AtomicU64::new(1);

#[test]
fn story_48_2_runtime_action_ids_are_precomputable_stable_and_sequence_bound() {
    let run_id = RuntimeRunId::from_raw("run-action-policy-0001");
    let first = runtime_action_id(&run_id, 1);
    assert_eq!(first, runtime_action_id(&run_id, 1));
    assert_ne!(first, runtime_action_id(&run_id, 2));
    assert_ne!(
        first,
        runtime_action_id(&RuntimeRunId::from_raw("run-other"), 1)
    );
    assert!(first.as_str().starts_with("action:"));
    assert!(first.as_str().len() <= 128);
}

#[derive(Clone, Copy)]
enum ModelScript {
    Completion,
    LargeCompletion,
    Tool,
    Malformed,
    Failure(RuntimePortFailure),
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
        if let ModelScript::Failure(failure) = script {
            return Err(failure);
        }
        let (kind, payload, tool_call) = match script {
            ModelScript::Completion | ModelScript::Malformed => (
                ModelProposalKind::CompletionCandidate,
                Some(payload("runtime.answer", b"verified fixture answer")),
                None,
            ),
            ModelScript::LargeCompletion => (
                ModelProposalKind::CompletionCandidate,
                Some(payload(
                    "runtime.answer",
                    &vec![b'x'; MAX_RUNTIME_INLINE_OUTPUT_BYTES + 1],
                )),
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
            ModelScript::Failure(_) => unreachable!("failure returned before proposal creation"),
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

struct PressureStreamingModel {
    profile: ExactModelProfile,
    started: Option<mpsc::SyncSender<()>>,
    release: mpsc::Receiver<()>,
    fragments: Arc<AtomicUsize>,
    cancellation_seen: Arc<AtomicBool>,
}

impl RuntimeModelPort for PressureStreamingModel {
    fn exact_profile(&self) -> &ExactModelProfile {
        &self.profile
    }

    fn run_model(
        &mut self,
        request: &ModelRunRequest,
        _context: &ModelContextPacket,
        cancellation: Option<&dyn agentmage_kernel_contracts::ModelCancellationProbe>,
    ) -> Result<ModelRunResult, RuntimePortFailure> {
        self.started
            .take()
            .ok_or(RuntimePortFailure::Invalid)?
            .send(())
            .map_err(|_| RuntimePortFailure::Unavailable)?;
        self.release
            .recv_timeout(Duration::from_secs(1))
            .map_err(|_| RuntimePortFailure::TimedOut)?;

        let deadline = Instant::now() + Duration::from_secs(2);
        loop {
            let fragments = self.fragments.fetch_add(1, Ordering::SeqCst) + 1;
            let signal = cancellation
                .map(agentmage_kernel_contracts::ModelCancellationProbe::observe)
                .transpose()
                .map_err(|_| RuntimePortFailure::Unavailable)?
                .flatten();
            if signal.is_some() {
                self.cancellation_seen.store(true, Ordering::SeqCst);
                let fragment_count =
                    u32::try_from(fragments).map_err(|_| RuntimePortFailure::ResourceExhausted)?;
                return Ok(ModelRunResult {
                    schema_version: CONTRACT_SCHEMA_VERSION,
                    model_run_id: request.model_run_id.clone(),
                    stream_id: ModelStreamId::from_raw("pressure-stream-1"),
                    correlation_id: request.correlation_id.clone(),
                    terminal_state: ModelRunTerminalState::Cancelled,
                    fragment_count,
                    response_sha256: sha256(format!("pressure:{fragments}").as_bytes()),
                    proposal: None,
                    failure: Some(ModelRuntimeFailure {
                        code: "runtime.model.cancelled".to_owned(),
                        retryable_after_correction: false,
                        dependency_recovery_required: false,
                        contract_error: None,
                    }),
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
                });
            }
            if Instant::now() >= deadline {
                return Err(RuntimePortFailure::TimedOut);
            }
            thread::sleep(Duration::from_millis(1));
        }
    }
}

struct PressureCancellationProbe {
    requested: AtomicBool,
    signal: CancellationSignal,
}

impl agentmage_kernel_contracts::ModelCancellationProbe for PressureCancellationProbe {
    fn observe(&self) -> Result<Option<CancellationSignal>, ModelRuntimeFailure> {
        Ok(self
            .requested
            .load(Ordering::SeqCst)
            .then(|| self.signal.clone()))
    }
}

struct PressureJournalBoundary {
    worker: Arc<RuntimeJournalWorker>,
}

impl RuntimeToolBoundary for PressureJournalBoundary {
    fn evaluate(
        &mut self,
        _request: &RuntimeRunRequest,
        _operation_id: &RuntimeOperationId,
        _definition: &ToolDefinition,
        _call: &ToolCall,
        _now_epoch_ms: u64,
    ) -> Result<RuntimePermissionEvaluation, RuntimePortFailure> {
        Err(RuntimePortFailure::Unavailable)
    }

    fn resolve(
        &mut self,
        _request: &RuntimeRunRequest,
        _challenge: &agentmage_kernel_contracts::RuntimeApprovalChallenge,
        _response: &RuntimeApprovalResponse,
        _definition: &ToolDefinition,
        _call: &ToolCall,
        _now_epoch_ms: u64,
    ) -> Result<RuntimePermissionEvaluation, RuntimePortFailure> {
        Err(RuntimePortFailure::Unavailable)
    }

    fn execute(
        &mut self,
        _request: &RuntimeRunRequest,
        _evaluation: &RuntimePermissionEvaluation,
        _definition: &ToolDefinition,
        _call: &ToolCall,
        _cancellation: Option<&dyn agentmage_kernel_contracts::ModelCancellationProbe>,
    ) -> Result<RuntimeToolExecution, RuntimePortFailure> {
        Err(RuntimePortFailure::Unavailable)
    }
}

impl RuntimeJournalPort for PressureJournalBoundary {
    fn append_runtime_event(&mut self, event: &RuntimeEvent) -> Result<(), RuntimePortFailure> {
        self.worker
            .append(event.clone())
            .map(|_| ())
            .map_err(map_pressure_journal_error)
    }

    fn flush_runtime_events(&mut self) -> Result<(), RuntimePortFailure> {
        self.worker
            .flush_all()
            .map(|_| ())
            .map_err(map_pressure_journal_error)
    }

    fn load_runtime_events(
        &mut self,
        run_id: &RuntimeRunId,
    ) -> Result<Vec<RuntimeEvent>, RuntimePortFailure> {
        self.worker.load(run_id).map_err(map_pressure_journal_error)
    }
}

impl RuntimeCorrectnessTransactionPort for PressureJournalBoundary {
    fn evaluate_with_correctness_event(
        &mut self,
        _request: &RuntimeRunRequest,
        _operation_id: &RuntimeOperationId,
        _definition: &ToolDefinition,
        _call: &ToolCall,
        _now_epoch_ms: u64,
        _build_event: &mut dyn FnMut(
            &RuntimePermissionEvaluation,
        ) -> Result<RuntimeEvent, RuntimePortFailure>,
    ) -> Result<(RuntimePermissionEvaluation, RuntimeEvent), RuntimePortFailure> {
        Err(RuntimePortFailure::Unavailable)
    }

    fn resolve_with_correctness_event(
        &mut self,
        _request: &RuntimeRunRequest,
        _challenge: &agentmage_kernel_contracts::RuntimeApprovalChallenge,
        _response: &RuntimeApprovalResponse,
        _definition: &ToolDefinition,
        _call: &ToolCall,
        _now_epoch_ms: u64,
        _build_event: &mut dyn FnMut(
            &RuntimePermissionEvaluation,
        ) -> Result<RuntimeEvent, RuntimePortFailure>,
    ) -> Result<(RuntimePermissionEvaluation, RuntimeEvent), RuntimePortFailure> {
        Err(RuntimePortFailure::Unavailable)
    }

    fn execute_with_correctness_events(
        &mut self,
        _request: &RuntimeRunRequest,
        _evaluation: &RuntimePermissionEvaluation,
        _definition: &ToolDefinition,
        _call: &ToolCall,
        _cancellation: Option<&dyn agentmage_kernel_contracts::ModelCancellationProbe>,
        _started_event: RuntimeEvent,
        _build_terminal_event: &mut dyn FnMut(
            &RuntimeToolExecution,
        ) -> Result<RuntimeEvent, RuntimePortFailure>,
    ) -> Result<RuntimeToolCorrectnessCommit, RuntimePortFailure> {
        Err(RuntimePortFailure::Unavailable)
    }

    fn commit_checkpoint_with_correctness_event(
        &mut self,
        _input: RuntimeCheckpointCommit<'_>,
        _build_event: &mut dyn FnMut(
            &RuntimeCheckpointPublication,
        ) -> Result<RuntimeEvent, RuntimePortFailure>,
    ) -> Result<(RuntimeCheckpointPublication, RuntimeEvent), RuntimePortFailure> {
        Err(RuntimePortFailure::Unavailable)
    }
}

struct PressureTestKey;

impl OperationalStoreKeyProvider for PressureTestKey {
    fn with_key<T>(
        &mut self,
        operation: impl FnOnce(&[u8]) -> T,
    ) -> Result<T, OperationalStoreKeyError> {
        Ok(operation(&[81; 32]))
    }
}

fn map_pressure_journal_error(error: RuntimeJournalError) -> RuntimePortFailure {
    match error {
        RuntimeJournalError::InvalidLimits
        | RuntimeJournalError::InvalidEvent
        | RuntimeJournalError::OrderingMismatch
        | RuntimeJournalError::Serialization => RuntimePortFailure::Invalid,
        RuntimeJournalError::QueueSaturated => RuntimePortFailure::ResourceExhausted,
        RuntimeJournalError::Storage
        | RuntimeJournalError::Integrity
        | RuntimeJournalError::WorkerUnavailable => RuntimePortFailure::Unavailable,
    }
}

fn pressure_observation() -> StrictLocalStorageObservation {
    StrictLocalStorageObservation {
        filesystem: StorageFilesystemClass::Local,
        synchronization_marker: None,
        root_identity_sha256: [11; 32],
        symlink_free: true,
    }
}

fn pressure_temporary_directory() -> std::path::PathBuf {
    let sequence = PRESSURE_TEMP_ID.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!(
        "agentmage-runtime-loop-pressure-{}-{sequence}",
        std::process::id()
    ));
    fs::create_dir(&path).expect("pressure temporary directory");
    path
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

type PublishedArtifacts = Arc<Mutex<Vec<(RuntimeArtifactManifest, Vec<u8>)>>>;
type PublishedCheckpoint = Arc<Mutex<Option<RuntimeResumeSnapshot>>>;

struct FakeToolBoundary {
    script: PermissionScript,
    executions: Arc<AtomicUsize>,
    emit_evidence: bool,
    outcome: OperationOutcome,
    state_change: StateChange,
    tool_output_bytes: usize,
    journal: Arc<Mutex<Vec<RuntimeEvent>>>,
    journal_flushes: Arc<AtomicUsize>,
    artifacts: PublishedArtifacts,
    checkpoint: PublishedCheckpoint,
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
                grant_id: GrantId::from_raw("grant-0001"),
                preview_sha256,
                expires_at_epoch_ms,
            };
        }
        let disposition = response.map(|value| value.disposition);
        if disposition == Some(RuntimeApprovalDisposition::Deny) {
            RuntimePermissionEvaluation::Deny {
                approval_id,
                grant_id: GrantId::from_raw("grant-0001"),
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
        let evidence = if self.emit_evidence && self.outcome == OperationOutcome::Succeeded {
            vec![evidence(
                &format!("tool-evidence-{execution}"),
                EvidenceKind::ToolOutput,
            )]
        } else {
            Vec::new()
        };
        let output = (self.outcome == OperationOutcome::Succeeded).then(|| {
            if self.tool_output_bytes > 0 {
                vec![b't'; self.tool_output_bytes]
            } else {
                b"fixture contents".to_vec()
            }
        });
        let result = ToolResult {
            schema_version: CONTRACT_SCHEMA_VERSION,
            tool_call_id: call.tool_call_id.clone(),
            correlation_id: call.correlation_id.clone(),
            outcome: self.outcome,
            output: output.map(|bytes| payload("fixture.output", &bytes)),
            validation_issues: Vec::new(),
            evidence,
            error: None,
            elapsed_ms: 1,
            state_change: if self.outcome == OperationOutcome::Succeeded {
                self.state_change
            } else if self.outcome == OperationOutcome::Uncertain {
                StateChange::Uncertain
            } else {
                StateChange::NotChanged
            },
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

impl RuntimeJournalPort for FakeToolBoundary {
    fn append_runtime_event(&mut self, event: &RuntimeEvent) -> Result<(), RuntimePortFailure> {
        let mut events = self
            .journal
            .lock()
            .map_err(|_| RuntimePortFailure::Uncertain)?;
        let mut sequence = RuntimeEventSequence::new();
        for retained in events.iter() {
            sequence
                .push(retained)
                .map_err(|_| RuntimePortFailure::Invalid)?;
        }
        sequence
            .push(event)
            .map_err(|_| RuntimePortFailure::Invalid)?;
        events.push(event.clone());
        Ok(())
    }

    fn flush_runtime_events(&mut self) -> Result<(), RuntimePortFailure> {
        self.journal_flushes.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    fn load_runtime_events(
        &mut self,
        run_id: &RuntimeRunId,
    ) -> Result<Vec<RuntimeEvent>, RuntimePortFailure> {
        self.journal
            .lock()
            .map_err(|_| RuntimePortFailure::Uncertain)
            .map(|events| {
                events
                    .iter()
                    .filter(|event| &event.run_id == run_id)
                    .cloned()
                    .collect()
            })
    }
}

impl RuntimeCorrectnessTransactionPort for FakeToolBoundary {
    fn evaluate_with_correctness_event(
        &mut self,
        request: &RuntimeRunRequest,
        operation_id: &RuntimeOperationId,
        definition: &ToolDefinition,
        call: &ToolCall,
        now_epoch_ms: u64,
        build_event: &mut dyn FnMut(
            &RuntimePermissionEvaluation,
        ) -> Result<RuntimeEvent, RuntimePortFailure>,
    ) -> Result<(RuntimePermissionEvaluation, RuntimeEvent), RuntimePortFailure> {
        let evaluation = <Self as RuntimeToolBoundary>::evaluate(
            self,
            request,
            operation_id,
            definition,
            call,
            now_epoch_ms,
        )?;
        let event = build_event(&evaluation)?;
        self.append_runtime_event(&event)?;
        Ok((evaluation, event))
    }

    fn resolve_with_correctness_event(
        &mut self,
        request: &RuntimeRunRequest,
        challenge: &agentmage_kernel_contracts::RuntimeApprovalChallenge,
        response: &RuntimeApprovalResponse,
        definition: &ToolDefinition,
        call: &ToolCall,
        now_epoch_ms: u64,
        build_event: &mut dyn FnMut(
            &RuntimePermissionEvaluation,
        ) -> Result<RuntimeEvent, RuntimePortFailure>,
    ) -> Result<(RuntimePermissionEvaluation, RuntimeEvent), RuntimePortFailure> {
        let evaluation = <Self as RuntimeToolBoundary>::resolve(
            self,
            request,
            challenge,
            response,
            definition,
            call,
            now_epoch_ms,
        )?;
        let event = build_event(&evaluation)?;
        self.append_runtime_event(&event)?;
        Ok((evaluation, event))
    }

    fn execute_with_correctness_events(
        &mut self,
        request: &RuntimeRunRequest,
        evaluation: &RuntimePermissionEvaluation,
        definition: &ToolDefinition,
        call: &ToolCall,
        cancellation: Option<&dyn agentmage_kernel_contracts::ModelCancellationProbe>,
        started_event: RuntimeEvent,
        build_terminal_event: &mut dyn FnMut(
            &RuntimeToolExecution,
        ) -> Result<RuntimeEvent, RuntimePortFailure>,
    ) -> Result<RuntimeToolCorrectnessCommit, RuntimePortFailure> {
        self.append_runtime_event(&started_event)?;
        let execution = <Self as RuntimeToolBoundary>::execute(
            self,
            request,
            evaluation,
            definition,
            call,
            cancellation,
        )?;
        let terminal = build_terminal_event(&execution)?;
        self.append_runtime_event(&terminal)?;
        Ok(RuntimeToolCorrectnessCommit {
            execution,
            events: vec![started_event, terminal],
        })
    }

    fn commit_checkpoint_with_correctness_event(
        &mut self,
        input: RuntimeCheckpointCommit<'_>,
        build_event: &mut dyn FnMut(
            &RuntimeCheckpointPublication,
        ) -> Result<RuntimeEvent, RuntimePortFailure>,
    ) -> Result<(RuntimeCheckpointPublication, RuntimeEvent), RuntimePortFailure> {
        let publication = self.commit_runtime_checkpoint(input)?;
        let event = build_event(&publication)?;
        self.append_runtime_event(&event)?;
        Ok((publication, event))
    }
}

impl RuntimeArtifactPort for FakeToolBoundary {
    fn publish_runtime_artifact(
        &mut self,
        manifest: RuntimeArtifactManifest,
        payload: &[u8],
    ) -> Result<RuntimeArtifactRef, RuntimePortFailure> {
        verify_runtime_artifact_manifest(&manifest).map_err(|_| RuntimePortFailure::Invalid)?;
        if manifest.payload_sha256 != sha256(payload) || manifest.byte_size != payload.len() as u64
        {
            return Err(RuntimePortFailure::Invalid);
        }
        let reference = runtime_artifact_ref(&manifest).map_err(|_| RuntimePortFailure::Invalid)?;
        self.artifacts
            .lock()
            .map_err(|_| RuntimePortFailure::Uncertain)?
            .push((manifest, payload.to_vec()));
        Ok(reference)
    }
}

impl RuntimeCheckpointPort for FakeToolBoundary {
    fn commit_runtime_checkpoint(
        &mut self,
        input: RuntimeCheckpointCommit<'_>,
    ) -> Result<RuntimeCheckpointPublication, RuntimePortFailure> {
        let mut evidence_ids = input
            .continuation
            .evidence
            .iter()
            .map(|evidence| evidence.evidence_id.clone())
            .collect::<Vec<_>>();
        evidence_ids.sort_by(|left, right| left.as_str().cmp(right.as_str()));
        let plan_id = input
            .request
            .work_packet
            .plan_id
            .clone()
            .ok_or(RuntimePortFailure::Invalid)?;
        let checkpoint = finalize_checkpoint(SessionCheckpoint {
            schema_version: CONTRACT_SCHEMA_VERSION,
            checkpoint_id: SessionCheckpointId::from_raw(format!(
                "checkpoint-{}",
                input.continuation.turn_count
            )),
            session_id: input.request.session_id.clone(),
            task_id: input.request.task.task_id.clone(),
            objective_sha256: sha256(input.request.task.objective.as_bytes()),
            plan_id,
            plan_revision: input.request.work_packet.revision,
            plan_step_id: PlanStepId::from_raw(format!(
                "runtime-step-{}",
                input.continuation.turn_count
            )),
            next_action_sha256: sha256(b"continue from safe runtime boundary"),
            workspace_id: input.request.workspace_id.clone(),
            workspace_state_sha256: input.request.workspace_snapshot_sha256.clone(),
            repository_snapshot_id: input.request.repository_snapshot_id.clone(),
            repository_branch: "fixture/main".to_owned(),
            repository_map_sha256: input.request.repository_snapshot_sha256.clone(),
            files: Vec::new(),
            instruction_sha256: sha256(b"fixture instructions"),
            permission_profile_id: input.request.policy_id.as_str().to_owned(),
            permission_profile_sha256: input.request.policy_sha256.clone(),
            policy_id: input.request.policy_id.clone(),
            policy_sha256: input.request.policy_sha256.clone(),
            model_profile_id: input.request.model_profile.profile_id.clone(),
            model_manifest_sha256: input.request.model_profile.manifest_sha256.clone(),
            model_runtime_sha256: input.request.model_profile.runtime.runtime_sha256.clone(),
            evidence_ids,
            citation_set_sha256: sha256(b"fixture citation set"),
            blockers: Vec::new(),
            context_packet_sha256: input.continuation.continuation_sha256.clone(),
            action_id: None,
            action_state: None,
            consumed_grant_id: None,
            receipt_id: None,
            receipt_sha256: None,
            ephemeral: false,
            checkpoint_sha256: "0".repeat(64),
        })
        .map_err(|_| RuntimePortFailure::Invalid)?;
        let binding = seal_runtime_resume_binding(RuntimeResumeBinding {
            schema_version: CONTRACT_SCHEMA_VERSION,
            checkpoint_id: checkpoint.checkpoint_id.clone(),
            checkpoint_sha256: checkpoint.checkpoint_sha256.clone(),
            session_id: input.request.session_id.clone(),
            task_id: input.request.task.task_id.clone(),
            run_id: input.request.run_id.clone(),
            event_cursor: input.event_cursor.clone(),
            artifacts: input.artifacts.to_vec(),
            binding_sha256: "0".repeat(64),
        })
        .map_err(|_| RuntimePortFailure::Invalid)?;
        let snapshot = RuntimeResumeSnapshot {
            checkpoint: checkpoint.clone(),
            binding: binding.clone(),
            continuation: input.continuation.clone(),
            continuation_artifact: input.continuation_artifact.clone(),
        };
        *self
            .checkpoint
            .lock()
            .map_err(|_| RuntimePortFailure::Uncertain)? = Some(snapshot);
        Ok(RuntimeCheckpointPublication {
            checkpoint,
            binding,
        })
    }

    fn load_runtime_checkpoint(
        &mut self,
        _request: &RuntimeRunRequest,
    ) -> Result<Option<RuntimeResumeSnapshot>, RuntimePortFailure> {
        self.checkpoint
            .lock()
            .map_err(|_| RuntimePortFailure::Uncertain)
            .map(|snapshot| snapshot.clone())
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

fn registry_for_operation(operation: GrantOperation) -> ToolRegistry {
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
                declared_effects: vec![OperationBinding::new(operation)],
                required_grant: RequiredGrantTemplate {
                    operation: OperationBinding::new(operation),
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
    coordinator_for_mode(
        RuntimeSessionMode::EphemeralReadOnly,
        scripts,
        permission,
        emit_tool_evidence,
    )
    .expect("fixture coordinator builds")
}

fn coordinator_for_mode(
    mode: RuntimeSessionMode,
    scripts: impl IntoIterator<Item = ModelScript>,
    permission: PermissionScript,
    emit_tool_evidence: bool,
) -> Result<(FixtureCoordinator, Arc<AtomicUsize>), RuntimeLoopError> {
    coordinator_for_mode_and_operation(
        mode,
        GrantOperation::WorkspaceRead,
        scripts,
        permission,
        emit_tool_evidence,
    )
}

fn coordinator_for_mode_and_operation(
    mode: RuntimeSessionMode,
    operation: GrantOperation,
    scripts: impl IntoIterator<Item = ModelScript>,
    permission: PermissionScript,
    emit_tool_evidence: bool,
) -> Result<(FixtureCoordinator, Arc<AtomicUsize>), RuntimeLoopError> {
    coordinator_with_request_mutation(
        mode,
        operation,
        scripts,
        permission,
        emit_tool_evidence,
        |_| {},
    )
}

fn coordinator_with_request_mutation(
    mode: RuntimeSessionMode,
    operation: GrantOperation,
    scripts: impl IntoIterator<Item = ModelScript>,
    permission: PermissionScript,
    emit_tool_evidence: bool,
    mutate: impl FnOnce(&mut RuntimeRunRequest),
) -> Result<(FixtureCoordinator, Arc<AtomicUsize>), RuntimeLoopError> {
    let profile = profile("runtime-loop");
    let registry = registry_for_operation(operation);
    let mut request = request(profile.clone(), &registry);
    request.mode = mode;
    mutate(&mut request);
    request.request_sha256 = "0".repeat(64);
    let request = seal_runtime_run_request(request).expect("mode-specific request seals");
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
            outcome: OperationOutcome::Succeeded,
            state_change: if operation == GrantOperation::WorkspaceWrite {
                StateChange::Changed
            } else {
                StateChange::NotChanged
            },
            tool_output_bytes: 0,
            journal: Arc::new(Mutex::new(Vec::new())),
            journal_flushes: Arc::new(AtomicUsize::new(0)),
            artifacts: Arc::new(Mutex::new(Vec::new())),
            checkpoint: Arc::new(Mutex::new(None)),
        },
        FakeVerifier {
            verifier_id: VerifierId::from_raw("verifier-0001"),
            source: VerifierSource::DeterministicPostcondition,
        },
        FakeClock { now: 1_000 },
    )?;
    Ok((coordinator, executions))
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
                resource: BudgetResource::ToolCallDepth,
                limit: 1,
            },
            BudgetLimit {
                resource: BudgetResource::ModelCalls,
                limit: 4,
            },
            BudgetLimit {
                resource: BudgetResource::ToolCalls,
                limit: 4,
            },
            BudgetLimit {
                resource: BudgetResource::InputBytes,
                limit: 4_096,
            },
            BudgetLimit {
                resource: BudgetResource::OutputBytes,
                limit: 1024 * 1024,
            },
            BudgetLimit {
                resource: BudgetResource::ElapsedMilliseconds,
                limit: 100_000,
            },
            BudgetLimit {
                resource: BudgetResource::MemoryBytes,
                limit: 1,
            },
            BudgetLimit {
                resource: BudgetResource::DiskBytes,
                limit: 64 * 1024 * 1024,
            },
            BudgetLimit {
                resource: BudgetResource::ProcessCount,
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
fn story_48_2_controlled_write_mode_uses_the_same_ephemeral_coordinator_boundary() {
    let (mut coordinator, executions) = coordinator_for_mode(
        RuntimeSessionMode::ControlledWrite,
        [ModelScript::Completion],
        PermissionScript::Allow,
        true,
    )
    .expect("controlled-write coordinator builds");

    let RuntimeCoordinatorStep::Complete { outcome } = coordinator
        .run_until_boundary(None, None)
        .expect("controlled-write mode reaches the verifier")
    else {
        panic!("direct completion cannot request approval");
    };
    assert_eq!(
        coordinator.request.mode,
        RuntimeSessionMode::ControlledWrite
    );
    assert_eq!(outcome.state, AgentStateKind::Success);
    assert_eq!(executions.load(Ordering::SeqCst), 0);
    assert_valid_terminal_stream(&coordinator);
}

#[test]
fn story_48_2_controlled_write_accepts_only_truthful_changed_tool_results() {
    let (mut coordinator, executions) = coordinator_for_mode_and_operation(
        RuntimeSessionMode::ControlledWrite,
        GrantOperation::WorkspaceWrite,
        [ModelScript::Tool, ModelScript::Completion],
        PermissionScript::Allow,
        true,
    )
    .expect("controlled-write tool catalog builds");

    let RuntimeCoordinatorStep::Complete { outcome } = coordinator
        .run_until_boundary(None, None)
        .expect("changed result returns to observation and verification")
    else {
        panic!("pre-authorized fixture does not pause");
    };
    assert_eq!(outcome.state, AgentStateKind::Success);
    assert_eq!(outcome.tool_call_count, 1);
    assert_eq!(executions.load(Ordering::SeqCst), 1);
    assert_valid_terminal_stream(&coordinator);

    let (mut false_no_change, false_executions) = coordinator_for_mode_and_operation(
        RuntimeSessionMode::ControlledWrite,
        GrantOperation::WorkspaceWrite,
        [ModelScript::Tool],
        PermissionScript::Allow,
        true,
    )
    .expect("controlled-write tool catalog builds");
    false_no_change.tool_boundary.state_change = StateChange::NotChanged;
    assert_eq!(
        false_no_change.run_until_boundary(None, None),
        Err(RuntimeLoopError::InvalidBoundaryResult)
    );
    assert_eq!(false_executions.load(Ordering::SeqCst), 1);
}

#[test]
fn story_23_4_read_only_mode_rejects_stateful_or_command_catalogs() {
    for operation in [
        GrantOperation::WorkspaceWrite,
        GrantOperation::CommandExecute,
        GrantOperation::NetworkAccess,
    ] {
        assert!(matches!(
            coordinator_for_mode_and_operation(
                RuntimeSessionMode::EphemeralReadOnly,
                operation,
                [ModelScript::Completion],
                PermissionScript::Allow,
                true,
            ),
            Err(RuntimeLoopError::ToolCatalogBinding)
        ));
    }
}

#[test]
fn durable_mode_remains_unavailable_without_journal_and_resume_ports() {
    assert!(matches!(
        coordinator_for_mode(
            RuntimeSessionMode::DurableReadOnly,
            [ModelScript::Completion],
            PermissionScript::Allow,
            true,
        ),
        Err(RuntimeLoopError::UnsupportedMode)
    ));
}

#[test]
fn story_21_2_durable_model_progress_and_cancellation_survive_delayed_sqlcipher() {
    const FRAGMENTS_BEFORE_CANCEL: usize = 16;
    const CANCELLATION_LIMIT: Duration = Duration::from_millis(250);

    let directory = pressure_temporary_directory();
    let path = directory.join("authority.db");
    let store = Arc::new(Mutex::new(
        OperationalStore::open(&path, &pressure_observation(), &mut PressureTestKey)
            .expect("encrypted pressure store"),
    ));
    let worker = Arc::new(
        RuntimeJournalWorker::new(Arc::clone(&store)).expect("bounded pressure journal worker"),
    );
    let profile = profile("runtime-loop-pressure");
    let registry = registry_for_operation(GrantOperation::WorkspaceRead);
    let mut request = request(profile.clone(), &registry);
    request.mode = RuntimeSessionMode::DurableReadOnly;
    request.request_sha256 = "0".repeat(64);
    let request = seal_runtime_run_request(request).expect("durable pressure request seals");
    let signal = CancellationSignal {
        schema_version: CONTRACT_SCHEMA_VERSION,
        cancellation_id: CancellationId::from_raw("pressure-cancellation-0001"),
        correlation_id: CorrelationId::from_raw(derived_id(
            "correlation",
            request.run_id.as_str(),
            0,
        )),
        task_id: request.task.task_id.clone(),
        reason: CancellationReason::UserRequested,
        requested_by: BoundaryKind::Shell,
    };
    let cancellation = Arc::new(PressureCancellationProbe {
        requested: AtomicBool::new(false),
        signal,
    });
    let fragments = Arc::new(AtomicUsize::new(0));
    let cancellation_seen = Arc::new(AtomicBool::new(false));
    let (started_sender, started_receiver) = mpsc::sync_channel(1);
    let (release_sender, release_receiver) = mpsc::sync_channel(1);
    let model = PressureStreamingModel {
        profile,
        started: Some(started_sender),
        release: release_receiver,
        fragments: Arc::clone(&fragments),
        cancellation_seen: Arc::clone(&cancellation_seen),
    };
    let boundary = PressureJournalBoundary {
        worker: Arc::clone(&worker),
    };
    let mut coordinator = ReusableRuntimeCoordinator::new_with_journal(
        request,
        model,
        FakeContext,
        registry,
        boundary,
        FakeVerifier {
            verifier_id: VerifierId::from_raw("verifier-pressure-0001"),
            source: VerifierSource::DeterministicPostcondition,
        },
        FakeClock { now: 8_000 },
    )
    .expect("durable pressure coordinator builds");
    let subscription = coordinator
        .subscribe_events(16)
        .expect("pressure subscriber binds");
    let cancellation_for_run = Arc::clone(&cancellation);
    let mut delivered = Vec::new();

    let (coordinator, step, cancellation_latency, store_delay) = thread::scope(|scope| {
        let handle = scope.spawn(move || {
            let step = coordinator.run_until_boundary(None, Some(cancellation_for_run.as_ref()));
            (coordinator, step)
        });
        started_receiver
            .recv_timeout(Duration::from_secs(1))
            .expect("model reaches its stream boundary");

        let store_guard = store.lock().expect("delay the sole SQLCipher writer");
        let store_delay_started = Instant::now();
        release_sender
            .send(())
            .expect("release model stream under store delay");
        let fragment_deadline = Instant::now() + Duration::from_secs(1);
        while fragments.load(Ordering::SeqCst) < FRAGMENTS_BEFORE_CANCEL
            && Instant::now() < fragment_deadline
        {
            thread::sleep(Duration::from_millis(1));
        }
        assert!(
            fragments.load(Ordering::SeqCst) >= FRAGMENTS_BEFORE_CANCEL,
            "model progress must continue while the encrypted store is delayed"
        );

        let cancellation_started = Instant::now();
        cancellation.requested.store(true, Ordering::SeqCst);
        let cancellation_deadline = cancellation_started + CANCELLATION_LIMIT;
        while !cancellation_seen.load(Ordering::SeqCst) && Instant::now() < cancellation_deadline {
            thread::sleep(Duration::from_millis(1));
        }
        let cancellation_latency = cancellation_started.elapsed();
        assert!(cancellation_seen.load(Ordering::SeqCst));
        assert!(cancellation_latency < CANCELLATION_LIMIT);

        let client_deadline = Instant::now() + Duration::from_secs(1);
        let mut saw_model_failure = false;
        while !saw_model_failure && Instant::now() < client_deadline {
            while let Some(event) = subscription.try_next().expect("subscriber remains live") {
                saw_model_failure |= matches!(event.kind, RuntimeEventKind::ModelFailed { .. });
                delivered.push(event);
            }
            if !saw_model_failure {
                thread::sleep(Duration::from_millis(1));
            }
        }
        assert!(saw_model_failure);
        assert!(
            !delivered
                .iter()
                .any(|event| matches!(event.kind, RuntimeEventKind::RunTerminal { .. }))
        );
        assert!(
            !handle.is_finished(),
            "terminal return must wait for the blocked correctness write"
        );
        let store_delay = store_delay_started.elapsed();
        drop(store_guard);
        let (coordinator, step) = handle.join().expect("pressure coordinator joins");
        (coordinator, step, cancellation_latency, store_delay)
    });

    let RuntimeCoordinatorStep::Complete { outcome } =
        step.expect("pressure cancellation closes truthfully")
    else {
        panic!("cancelled model cannot request approval");
    };
    assert_eq!(outcome.state, AgentStateKind::Cancelled, "{outcome:#?}");
    while let Some(event) = subscription.try_next().expect("subscriber remains live") {
        delivered.push(event);
    }
    assert_eq!(delivered, coordinator.events());
    let cancellation_requested = coordinator
        .events()
        .iter()
        .position(|event| matches!(event.kind, RuntimeEventKind::CancellationRequested { .. }))
        .expect("cancellation request is journaled");
    let cancellation_observed = coordinator
        .events()
        .iter()
        .position(|event| matches!(event.kind, RuntimeEventKind::CancellationObserved { .. }))
        .expect("cancellation observation is journaled");
    let terminal = coordinator
        .events()
        .iter()
        .position(|event| matches!(event.kind, RuntimeEventKind::RunTerminal { .. }))
        .expect("terminal event is journaled");
    assert!(cancellation_requested < cancellation_observed);
    assert!(cancellation_observed < terminal);
    assert_eq!(
        worker.load(&coordinator.request.run_id),
        Ok(coordinator.events().to_vec())
    );
    let mut sequence = RuntimeEventSequence::new();
    for event in coordinator.events() {
        sequence
            .push(event)
            .expect("pressure event sequence remains valid");
    }
    assert!(sequence.is_terminal());
    assert_eq!(sequence.event_count(), coordinator.events().len() as u64);
    verify_runtime_outcome(&outcome, &coordinator.request)
        .expect("pressure outcome remains request-bound");

    println!(
        "AGENTMAGE_RUNTIME_PRESSURE_METRICS={}",
        serde_json::json!({
            "cancellation_latency_us": u64::try_from(cancellation_latency.as_micros())
                .unwrap_or(u64::MAX),
            "cancellation_limit_ms": CANCELLATION_LIMIT.as_millis(),
            "client_progress_while_store_delayed": true,
            "external_network_used": false,
            "fragment_count_before_cancellation": fragments.load(Ordering::SeqCst),
            "store_delay_ms": u64::try_from(store_delay.as_millis()).unwrap_or(u64::MAX),
            "terminal_waited_for_correctness_durability": true,
        })
    );

    drop(subscription);
    drop(coordinator);
    drop(worker);
    drop(store);
    fs::remove_dir_all(directory).expect("remove pressure directory");
}

#[test]
fn durable_mode_persists_ordered_session_events_before_terminal_return() {
    let profile = profile("runtime-loop-durable");
    let registry = registry_for_operation(GrantOperation::WorkspaceRead);
    let mut request = request(profile.clone(), &registry);
    request.mode = RuntimeSessionMode::DurableReadOnly;
    request.request_sha256 = "0".repeat(64);
    let request = seal_runtime_run_request(request).expect("durable request seals");
    let journal = Arc::new(Mutex::new(Vec::new()));
    let flushes = Arc::new(AtomicUsize::new(0));
    let mut coordinator = ReusableRuntimeCoordinator::new_with_journal(
        request,
        FakeModel::new(profile, [ModelScript::Completion]),
        FakeContext,
        registry,
        FakeToolBoundary {
            script: PermissionScript::Allow,
            executions: Arc::new(AtomicUsize::new(0)),
            emit_evidence: true,
            outcome: OperationOutcome::Succeeded,
            state_change: StateChange::NotChanged,
            tool_output_bytes: 0,
            journal: Arc::clone(&journal),
            journal_flushes: Arc::clone(&flushes),
            artifacts: Arc::new(Mutex::new(Vec::new())),
            checkpoint: Arc::new(Mutex::new(None)),
        },
        FakeVerifier {
            verifier_id: VerifierId::from_raw("verifier-durable-0001"),
            source: VerifierSource::DeterministicPostcondition,
        },
        FakeClock { now: 2_000 },
    )
    .expect("journal-backed coordinator builds");

    let RuntimeCoordinatorStep::Complete { outcome } = coordinator
        .run_until_boundary(None, None)
        .expect("durable completion")
    else {
        panic!("direct completion cannot pause");
    };
    assert_eq!(outcome.state, AgentStateKind::Success);
    assert_eq!(flushes.load(Ordering::SeqCst), 1);
    assert!(coordinator.events().iter().all(|event| {
        event.retention.kind == RuntimeEventRetentionKind::Session
            && event.retention.expires_at_epoch_ms.is_none()
    }));
    assert_eq!(
        *journal.lock().expect("journal remains available"),
        coordinator.events()
    );
    assert_valid_terminal_stream(&coordinator);
}

#[test]
fn durable_large_model_output_is_artifact_backed_and_event_referenced() {
    let profile = profile("runtime-loop-artifact");
    let registry = registry_for_operation(GrantOperation::WorkspaceRead);
    let mut request = request(profile.clone(), &registry);
    request.mode = RuntimeSessionMode::DurableReadOnly;
    request.limits.max_output_bytes = (MAX_RUNTIME_INLINE_OUTPUT_BYTES + 1) as u64;
    request.request_sha256 = "0".repeat(64);
    let request = seal_runtime_run_request(request).expect("artifact request seals");
    let journal = Arc::new(Mutex::new(Vec::new()));
    let artifacts = Arc::new(Mutex::new(Vec::new()));
    let mut coordinator = ReusableRuntimeCoordinator::new_with_persistence(
        request,
        FakeModel::new(profile, [ModelScript::LargeCompletion]),
        FakeContext,
        registry,
        FakeToolBoundary {
            script: PermissionScript::Allow,
            executions: Arc::new(AtomicUsize::new(0)),
            emit_evidence: true,
            outcome: OperationOutcome::Succeeded,
            state_change: StateChange::NotChanged,
            tool_output_bytes: 0,
            journal: Arc::clone(&journal),
            journal_flushes: Arc::new(AtomicUsize::new(0)),
            artifacts: Arc::clone(&artifacts),
            checkpoint: Arc::new(Mutex::new(None)),
        },
        FakeVerifier {
            verifier_id: VerifierId::from_raw("verifier-artifact-0001"),
            source: VerifierSource::DeterministicPostcondition,
        },
        FakeClock { now: 4_000 },
    )
    .expect("artifact-backed coordinator builds");

    let RuntimeCoordinatorStep::Complete { outcome } = coordinator
        .run_until_boundary(None, None)
        .expect("artifact-backed completion")
    else {
        panic!("direct completion cannot pause");
    };
    let RuntimeOutput::Artifact { reference } = outcome.output.expect("output exists") else {
        panic!("large durable output must not remain inline");
    };
    assert_eq!(
        reference.byte_size,
        (MAX_RUNTIME_INLINE_OUTPUT_BYTES + 1) as u64
    );
    assert_eq!(coordinator.artifact_references().len(), 1);
    assert_eq!(
        coordinator.artifact_references()[0].artifact_id,
        reference.artifact_id
    );
    let artifact_event = coordinator
        .events()
        .iter()
        .find(|event| matches!(event.kind, RuntimeEventKind::ArtifactCreated { .. }))
        .expect("artifact event exists");
    assert!(artifact_event.operation_id.is_none());
    assert_eq!(artifact_event.payload_reference.as_ref(), Some(&reference));
    let retained = artifacts
        .lock()
        .expect("artifact fixture remains available");
    assert_eq!(retained.len(), 1);
    assert_eq!(retained[0].1.len(), MAX_RUNTIME_INLINE_OUTPUT_BYTES + 1);
    assert_eq!(
        *journal.lock().expect("journal remains available"),
        coordinator.events()
    );
    assert_valid_terminal_stream(&coordinator);
}

#[test]
fn durable_tool_receipt_commits_before_its_bound_large_artifact() {
    let profile = profile("runtime-loop-tool-artifact");
    let registry = registry_for_operation(GrantOperation::WorkspaceRead);
    let mut request = request(profile.clone(), &registry);
    request.mode = RuntimeSessionMode::DurableReadOnly;
    request.limits.max_output_bytes = (MAX_RUNTIME_INLINE_OUTPUT_BYTES + 1) as u64;
    request.request_sha256 = "0".repeat(64);
    let request = seal_runtime_run_request(request).expect("tool artifact request seals");
    let journal = Arc::new(Mutex::new(Vec::new()));
    let artifacts = Arc::new(Mutex::new(Vec::new()));
    let mut coordinator = ReusableRuntimeCoordinator::new_with_persistence(
        request,
        FakeModel::new(profile, [ModelScript::Tool, ModelScript::Completion]),
        FakeContext,
        registry,
        FakeToolBoundary {
            script: PermissionScript::Allow,
            executions: Arc::new(AtomicUsize::new(0)),
            emit_evidence: true,
            outcome: OperationOutcome::Succeeded,
            state_change: StateChange::NotChanged,
            tool_output_bytes: MAX_RUNTIME_INLINE_OUTPUT_BYTES + 1,
            journal: Arc::clone(&journal),
            journal_flushes: Arc::new(AtomicUsize::new(0)),
            artifacts: Arc::clone(&artifacts),
            checkpoint: Arc::new(Mutex::new(None)),
        },
        FakeVerifier {
            verifier_id: VerifierId::from_raw("verifier-tool-artifact-0001"),
            source: VerifierSource::DeterministicPostcondition,
        },
        FakeClock { now: 5_000 },
    )
    .expect("tool artifact coordinator builds");

    let RuntimeCoordinatorStep::Complete { outcome } = coordinator
        .run_until_boundary(None, None)
        .expect("tool artifact run completes")
    else {
        panic!("allowed fixture cannot pause");
    };
    assert_eq!(outcome.state, AgentStateKind::Success);
    assert_eq!(coordinator.artifact_references().len(), 1);
    let artifact_index = coordinator
        .events()
        .iter()
        .position(|event| matches!(event.kind, RuntimeEventKind::ArtifactCreated { .. }))
        .expect("artifact event exists");
    let completion_index = coordinator
        .events()
        .iter()
        .position(|event| matches!(event.kind, RuntimeEventKind::ToolCompleted { .. }))
        .expect("tool completion exists");
    assert!(completion_index < artifact_index);
    assert!(coordinator.events()[artifact_index].operation_id.is_some());
    let retained = artifacts
        .lock()
        .expect("artifact fixture remains available");
    assert_eq!(retained.len(), 1);
    assert_eq!(
        retained[0]
            .0
            .receipt_id
            .as_ref()
            .map(|value| value.as_str()),
        Some("receipt-1")
    );
    assert_eq!(retained[0].1.len(), MAX_RUNTIME_INLINE_OUTPUT_BYTES + 1);
    assert_eq!(
        *journal.lock().expect("journal remains available"),
        coordinator.events()
    );
    assert_valid_terminal_stream(&coordinator);
}

#[test]
fn durable_checkpoint_resumes_without_replaying_the_completed_effect() {
    let profile = profile("runtime-loop-resume");
    let registry = registry_for_operation(GrantOperation::WorkspaceRead);
    let mut request = request(profile.clone(), &registry);
    request.mode = RuntimeSessionMode::DurableReadOnly;
    request.request_sha256 = "0".repeat(64);
    let request = seal_runtime_run_request(request).expect("durable request seals");
    let journal = Arc::new(Mutex::new(Vec::new()));
    let artifacts = Arc::new(Mutex::new(Vec::new()));
    let checkpoint = Arc::new(Mutex::new(None));
    let executions = Arc::new(AtomicUsize::new(0));
    let mut first = ReusableRuntimeCoordinator::new_with_durable_state(
        request.clone(),
        FakeModel::new(profile.clone(), [ModelScript::Tool]),
        FakeContext,
        registry_for_operation(GrantOperation::WorkspaceRead),
        FakeToolBoundary {
            script: PermissionScript::Allow,
            executions: Arc::clone(&executions),
            emit_evidence: true,
            outcome: OperationOutcome::Succeeded,
            state_change: StateChange::NotChanged,
            tool_output_bytes: 0,
            journal: Arc::clone(&journal),
            journal_flushes: Arc::new(AtomicUsize::new(0)),
            artifacts: Arc::clone(&artifacts),
            checkpoint: Arc::clone(&checkpoint),
        },
        FakeVerifier {
            verifier_id: VerifierId::from_raw("verifier-resume-0001"),
            source: VerifierSource::DeterministicPostcondition,
        },
        FakeClock { now: 6_000 },
    )
    .expect("durable coordinator builds");
    first.start().expect("run starts");
    first.run_turn(None).expect("first tool turn checkpoints");
    assert_eq!(executions.load(Ordering::SeqCst), 1);
    assert_eq!(first.state.current(), AgentStateKind::Observation);
    assert!(matches!(
        first.events().last().map(|event| &event.kind),
        Some(RuntimeEventKind::CheckpointCommitted { .. })
    ));
    let checkpoint_artifact = artifacts
        .lock()
        .expect("artifacts remain available")
        .last()
        .expect("continuation artifact exists")
        .0
        .clone();
    assert_eq!(
        checkpoint_artifact.media_type,
        crate::runtime_artifact::RUNTIME_CONTINUATION_MEDIA_TYPE
    );

    let cursor = runtime_event_cursor(
        journal
            .lock()
            .expect("journal remains available")
            .last()
            .expect("checkpoint marker exists"),
    );
    drop(first);
    let mut resumed_request = request;
    resumed_request.event_cursor = Some(cursor);
    resumed_request.request_sha256 = "0".repeat(64);
    let resumed_request = seal_runtime_run_request(resumed_request).expect("resume request seals");
    let mut resumed = ReusableRuntimeCoordinator::new_with_durable_state(
        resumed_request,
        FakeModel::new(profile, [ModelScript::Completion]),
        FakeContext,
        registry_for_operation(GrantOperation::WorkspaceRead),
        FakeToolBoundary {
            script: PermissionScript::Allow,
            executions: Arc::clone(&executions),
            emit_evidence: true,
            outcome: OperationOutcome::Succeeded,
            state_change: StateChange::NotChanged,
            tool_output_bytes: 0,
            journal: Arc::clone(&journal),
            journal_flushes: Arc::new(AtomicUsize::new(0)),
            artifacts,
            checkpoint,
        },
        FakeVerifier {
            verifier_id: VerifierId::from_raw("verifier-resume-0001"),
            source: VerifierSource::DeterministicPostcondition,
        },
        FakeClock { now: 6_100 },
    )
    .expect("checkpoint reconstructs a fresh coordinator");
    let RuntimeCoordinatorStep::Complete { outcome } = resumed
        .run_until_boundary(None, None)
        .expect("resumed run completes")
    else {
        panic!("completion cannot pause");
    };
    assert_eq!(outcome.state, AgentStateKind::Success);
    assert_eq!(outcome.turn_count, 2);
    assert_eq!(outcome.model_call_count, 2);
    assert_eq!(outcome.tool_call_count, 1);
    assert_eq!(executions.load(Ordering::SeqCst), 1);
    assert_eq!(
        resumed
            .events()
            .iter()
            .filter(|event| matches!(event.kind, RuntimeEventKind::RunStarted { .. }))
            .count(),
        1
    );
    assert_valid_terminal_stream(&resumed);
}

#[test]
fn ephemeral_mode_cannot_accidentally_attach_a_durable_journal() {
    let profile = profile("runtime-loop-ephemeral-journal");
    let registry = registry_for_operation(GrantOperation::WorkspaceRead);
    let request = request(profile.clone(), &registry);
    let journal = Arc::new(Mutex::new(Vec::new()));
    let result = ReusableRuntimeCoordinator::new_with_journal(
        request,
        FakeModel::new(profile, [ModelScript::Completion]),
        FakeContext,
        registry,
        FakeToolBoundary {
            script: PermissionScript::Allow,
            executions: Arc::new(AtomicUsize::new(0)),
            emit_evidence: true,
            outcome: OperationOutcome::Succeeded,
            state_change: StateChange::NotChanged,
            tool_output_bytes: 0,
            journal: Arc::clone(&journal),
            journal_flushes: Arc::new(AtomicUsize::new(0)),
            artifacts: Arc::new(Mutex::new(Vec::new())),
            checkpoint: Arc::new(Mutex::new(None)),
        },
        FakeVerifier {
            verifier_id: VerifierId::from_raw("verifier-ephemeral-0001"),
            source: VerifierSource::DeterministicPostcondition,
        },
        FakeClock { now: 3_000 },
    );
    assert!(matches!(result, Err(RuntimeLoopError::UnsupportedMode)));
    assert!(
        journal
            .lock()
            .expect("journal remains available")
            .is_empty()
    );
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
fn story_23_4_model_dependency_failures_close_without_effect_or_retry() {
    for (failure, state) in [
        (RuntimePortFailure::Invalid, AgentStateKind::Failed),
        (RuntimePortFailure::Unavailable, AgentStateKind::Failed),
        (RuntimePortFailure::Cancelled, AgentStateKind::Failed),
        (RuntimePortFailure::Uncertain, AgentStateKind::Failed),
        (RuntimePortFailure::TimedOut, AgentStateKind::Exhausted),
        (
            RuntimePortFailure::ResourceExhausted,
            AgentStateKind::Exhausted,
        ),
    ] {
        let (mut coordinator, executions) = coordinator(
            [ModelScript::Failure(failure)],
            PermissionScript::Allow,
            true,
        );
        let RuntimeCoordinatorStep::Complete { outcome } = coordinator
            .run_until_boundary(None, None)
            .expect("model dependency failure closes truthfully")
        else {
            panic!("model dependency failure cannot request approval");
        };
        assert_eq!(outcome.state, state, "{failure:?}");
        assert_eq!(outcome.unresolved_codes, [failure.code().to_owned()]);
        assert_eq!(outcome.model_call_count, 1);
        assert_eq!(outcome.tool_call_count, 0);
        assert_eq!(executions.load(Ordering::SeqCst), 0);
        assert_eq!(
            coordinator
                .events()
                .iter()
                .filter(|event| matches!(event.kind, RuntimeEventKind::ModelFailed { .. }))
                .count(),
            1
        );
        assert_valid_terminal_stream(&coordinator);
    }
}

#[test]
fn story_23_4_terminal_tool_failures_have_one_receipt_and_no_hidden_retry() {
    for (tool_outcome, state, code) in [
        (
            OperationOutcome::Denied,
            AgentStateKind::Failed,
            "runtime.tool.denied_after_launch",
        ),
        (
            OperationOutcome::TimedOut,
            AgentStateKind::Exhausted,
            "runtime.tool.timed_out",
        ),
        (
            OperationOutcome::Failed,
            AgentStateKind::Failed,
            "runtime.tool.failed",
        ),
    ] {
        let (mut coordinator, executions) =
            coordinator([ModelScript::Tool], PermissionScript::Allow, true);
        coordinator.tool_boundary.outcome = tool_outcome;
        let RuntimeCoordinatorStep::Complete { outcome } = coordinator
            .run_until_boundary(None, None)
            .expect("terminal tool result closes truthfully")
        else {
            panic!("terminal tool result cannot request approval");
        };
        assert_eq!(outcome.state, state, "{tool_outcome:?}");
        assert_eq!(outcome.unresolved_codes, [code.to_owned()]);
        assert_eq!(outcome.tool_call_count, 1);
        assert_eq!(outcome.receipt_ids.len(), 1);
        assert_eq!(executions.load(Ordering::SeqCst), 1);
        assert_eq!(
            coordinator
                .events()
                .iter()
                .filter(|event| matches!(event.kind, RuntimeEventKind::ToolStarted { .. }))
                .count(),
            1
        );
        assert_eq!(
            coordinator
                .events()
                .iter()
                .filter(|event| matches!(event.kind, RuntimeEventKind::ToolFailed { .. }))
                .count(),
            1
        );
        assert_valid_terminal_stream(&coordinator);
    }
}

#[test]
fn story_23_4_repeat_no_progress_and_turn_budgets_stop_truthfully() {
    let (mut repeated, repeated_executions) = coordinator_with_request_mutation(
        RuntimeSessionMode::EphemeralReadOnly,
        GrantOperation::WorkspaceRead,
        [ModelScript::Tool, ModelScript::Tool],
        PermissionScript::Allow,
        true,
        |request| request.limits.max_repeated_tool_calls = 1,
    )
    .expect("repeat-bounded coordinator builds");
    let RuntimeCoordinatorStep::Complete { outcome } = repeated
        .run_until_boundary(None, None)
        .expect("repeat ceiling closes truthfully")
    else {
        panic!("repeat ceiling cannot request approval");
    };
    assert_eq!(outcome.state, AgentStateKind::Exhausted);
    assert_eq!(
        outcome.unresolved_codes,
        ["tool.attempt.repeat_limit.exceeded".to_owned()]
    );
    assert_eq!(repeated_executions.load(Ordering::SeqCst), 1);
    assert_valid_terminal_stream(&repeated);

    let (mut stalled, stalled_executions) = coordinator(
        [ModelScript::Tool, ModelScript::Tool],
        PermissionScript::Allow,
        false,
    );
    let RuntimeCoordinatorStep::Complete { outcome } = stalled
        .run_until_boundary(None, None)
        .expect("no-progress ceiling closes truthfully")
    else {
        panic!("no-progress ceiling cannot request approval");
    };
    assert_eq!(outcome.state, AgentStateKind::Stalled);
    assert_eq!(
        outcome.unresolved_codes,
        ["runtime.no_progress.exhausted".to_owned()]
    );
    assert_eq!(stalled_executions.load(Ordering::SeqCst), 2);
    assert_valid_terminal_stream(&stalled);

    let (mut turn_bounded, turn_executions) = coordinator_with_request_mutation(
        RuntimeSessionMode::EphemeralReadOnly,
        GrantOperation::WorkspaceRead,
        [ModelScript::Tool, ModelScript::Completion],
        PermissionScript::Allow,
        true,
        |request| {
            request.limits.max_turns = 1;
            request.limits.max_no_progress_turns = 1;
        },
    )
    .expect("turn-bounded coordinator builds");
    let RuntimeCoordinatorStep::Complete { outcome } = turn_bounded
        .run_until_boundary(None, None)
        .expect("turn budget closes truthfully")
    else {
        panic!("turn budget cannot request approval");
    };
    assert_eq!(outcome.state, AgentStateKind::Exhausted);
    assert_eq!(
        outcome.unresolved_codes,
        ["runtime.budget.exhausted".to_owned()]
    );
    assert_eq!(turn_executions.load(Ordering::SeqCst), 1);
    assert_valid_terminal_stream(&turn_bounded);
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

fn controlled_write_hardening_request() -> RuntimeRunRequest {
    let registry = registry_for_operation(GrantOperation::WorkspaceWrite);
    let mut request = request(profile("runtime-hardening"), &registry);
    request.mode = RuntimeSessionMode::ControlledWrite;
    request
}

fn required_budget_minimum(request: &RuntimeRunRequest, resource: BudgetResource) -> u64 {
    match resource {
        BudgetResource::PlanSteps => u64::from(request.limits.max_turns),
        BudgetResource::ToolCallDepth => u64::from(request.limits.max_tool_call_depth),
        BudgetResource::ModelCalls => u64::from(request.limits.max_model_calls),
        BudgetResource::ToolCalls => u64::from(request.limits.max_tool_calls),
        BudgetResource::InputBytes => request
            .context_budget
            .max_input_bytes
            .checked_mul(u64::from(request.limits.max_context_refreshes))
            .expect("fixture input ceiling remains bounded"),
        BudgetResource::OutputBytes | BudgetResource::DiskBytes => request.limits.max_output_bytes,
        BudgetResource::ElapsedMilliseconds => request.limits.max_elapsed_ms,
        BudgetResource::MemoryBytes => 1,
        BudgetResource::ProcessCount => u64::from(request.limits.max_tool_calls.min(1)),
    }
}

#[test]
fn story_50_2_controlled_write_requires_every_bound_resource_budget() {
    let request = controlled_write_hardening_request();
    let required = [
        BudgetResource::PlanSteps,
        BudgetResource::ToolCallDepth,
        BudgetResource::ModelCalls,
        BudgetResource::ToolCalls,
        BudgetResource::InputBytes,
        BudgetResource::OutputBytes,
        BudgetResource::ElapsedMilliseconds,
        BudgetResource::MemoryBytes,
        BudgetResource::DiskBytes,
        BudgetResource::ProcessCount,
    ];

    for resource in required {
        let mut missing = request.clone();
        missing
            .work_packet
            .budgets
            .retain(|budget| budget.resource != resource);
        assert_eq!(
            RuntimeHardeningLimits::from_request(&missing),
            Err(RuntimeHardeningError::MissingBudget(resource))
        );

        let mut narrow = request.clone();
        let minimum = required_budget_minimum(&narrow, resource);
        narrow
            .work_packet
            .budgets
            .iter_mut()
            .find(|budget| budget.resource == resource)
            .expect("required fixture budget exists")
            .limit = minimum - 1;
        assert_eq!(
            RuntimeHardeningLimits::from_request(&narrow),
            Err(RuntimeHardeningError::BudgetBinding(resource))
        );
    }
}

#[test]
fn story_50_2_cumulative_budget_overage_is_non_mutating() {
    let request = controlled_write_hardening_request();
    for resource in [
        BudgetResource::PlanSteps,
        BudgetResource::ModelCalls,
        BudgetResource::ToolCalls,
        BudgetResource::InputBytes,
        BudgetResource::OutputBytes,
        BudgetResource::ElapsedMilliseconds,
        BudgetResource::DiskBytes,
        BudgetResource::ProcessCount,
    ] {
        let limit = request
            .work_packet
            .budgets
            .iter()
            .find(|budget| budget.resource == resource)
            .expect("fixture budget exists")
            .limit;
        let mut ledger = RuntimeResourceLedger::new(&request).expect("ledger admits fixture");
        ledger.consume(resource, limit).expect("exact limit admits");
        let exact = ledger.snapshot().clone();
        assert_eq!(
            ledger.consume(resource, 1),
            Err(RuntimeHardeningError::ResourceExhausted(resource))
        );
        assert_eq!(ledger.snapshot(), &exact);
    }
}

#[test]
fn story_50_2_peak_memory_and_failure_ceilings_are_non_mutating() {
    let request = controlled_write_hardening_request();
    let memory_limit = request
        .work_packet
        .budgets
        .iter()
        .find(|budget| budget.resource == BudgetResource::MemoryBytes)
        .expect("memory budget exists")
        .limit;
    let mut ledger = RuntimeResourceLedger::new(&request).expect("ledger admits fixture");
    ledger
        .observe_memory_peak(memory_limit)
        .expect("exact memory peak admits");
    ledger
        .observe_memory_peak(memory_limit.saturating_sub(1))
        .expect("lower observation does not accumulate");
    let exact_memory = ledger.snapshot().clone();
    assert_eq!(
        ledger.observe_memory_peak(memory_limit + 1),
        Err(RuntimeHardeningError::ResourceExhausted(
            BudgetResource::MemoryBytes
        ))
    );
    assert_eq!(ledger.snapshot(), &exact_memory);

    ledger.record_denial().expect("first denial admits");
    let exact_denial = ledger.snapshot().clone();
    assert_eq!(
        ledger.record_denial(),
        Err(RuntimeHardeningError::InvalidLimits)
    );
    assert_eq!(ledger.snapshot(), &exact_denial);

    ledger
        .record_parser_failure()
        .expect("first parser failure admits");
    let exact_parser = ledger.snapshot().clone();
    assert_eq!(
        ledger.record_parser_failure(),
        Err(RuntimeHardeningError::InvalidLimits)
    );
    assert_eq!(ledger.snapshot(), &exact_parser);

    let before_retry = ledger.snapshot().clone();
    assert_eq!(
        ledger.record_retry(),
        Err(RuntimeHardeningError::RetryDenied)
    );
    assert_eq!(ledger.snapshot(), &before_retry);
}

#[test]
fn story_50_2_subscription_capacity_is_bounded_by_count_and_bytes() {
    let mut request = controlled_write_hardening_request();
    request.limits.max_events = 4_096;
    let ledger = RuntimeResourceLedger::new(&request).expect("ledger admits fixture");
    let byte_capacity =
        usize::try_from(MAX_RUNTIME_CLIENT_QUEUE_BYTES / MAX_RUNTIME_EVENT_ENVELOPE_BYTES)
            .expect("fixture capacity fits usize");

    ledger
        .validate_subscription_capacity(byte_capacity)
        .expect("exact byte-bounded capacity admits");
    assert_eq!(
        ledger.validate_subscription_capacity(0),
        Err(RuntimeHardeningError::InvalidLimits)
    );
    assert_eq!(
        ledger.validate_subscription_capacity(byte_capacity + 1),
        Err(RuntimeHardeningError::InvalidLimits)
    );
}

#[test]
fn story_50_2_event_and_artifact_overage_is_non_mutating() {
    let mut event_request = controlled_write_hardening_request();
    event_request.limits.max_events = 2;
    let mut event_ledger =
        RuntimeResourceLedger::new(&event_request).expect("event ledger admits fixture");
    event_ledger
        .admit_event(MAX_RUNTIME_EVENT_ENVELOPE_BYTES)
        .expect("first exact envelope admits");
    event_ledger
        .admit_event(MAX_RUNTIME_EVENT_ENVELOPE_BYTES)
        .expect("run event ceiling admits");
    let exact_events = event_ledger.snapshot().clone();
    assert_eq!(
        event_ledger.admit_event(1),
        Err(RuntimeHardeningError::EventEnvelopeExhausted)
    );
    assert_eq!(event_ledger.snapshot(), &exact_events);

    let mut artifact_request = controlled_write_hardening_request();
    artifact_request
        .work_packet
        .budgets
        .iter_mut()
        .find(|budget| budget.resource == BudgetResource::DiskBytes)
        .expect("disk budget exists")
        .limit = artifact_request.limits.max_output_bytes;
    let mut byte_ledger =
        RuntimeResourceLedger::new(&artifact_request).expect("artifact ledger admits fixture");
    byte_ledger
        .admit_artifact(artifact_request.limits.max_output_bytes)
        .expect("exact aggregate artifact bytes admit");
    let exact_artifact_bytes = byte_ledger.snapshot().clone();
    assert_eq!(
        byte_ledger.admit_artifact(1),
        Err(RuntimeHardeningError::ArtifactExhausted)
    );
    assert_eq!(byte_ledger.snapshot(), &exact_artifact_bytes);

    let mut count_ledger = RuntimeResourceLedger::new(&controlled_write_hardening_request())
        .expect("artifact count ledger admits fixture");
    let artifact_count = count_ledger.limits().artifact_count;
    for _ in 0..artifact_count {
        count_ledger
            .admit_artifact(1)
            .expect("artifact count within ceiling admits");
    }
    let exact_artifact_count = count_ledger.snapshot().clone();
    assert_eq!(
        count_ledger.admit_artifact(1),
        Err(RuntimeHardeningError::ArtifactExhausted)
    );
    assert_eq!(count_ledger.snapshot(), &exact_artifact_count);
}

#[test]
fn story_50_2_durable_restore_rejects_usage_drift() {
    let request = controlled_write_hardening_request();
    let mut ledger = RuntimeResourceLedger::new(&request).expect("ledger admits fixture");
    ledger
        .consume(BudgetResource::PlanSteps, 1)
        .expect("plan usage admits");
    ledger.admit_event(128).expect("event admits");
    ledger.admit_artifact(64).expect("artifact admits");
    let usage = ledger.durable_usage();
    assert_eq!(
        RuntimeResourceLedger::restore(&request, &usage)
            .expect("exact durable usage restores")
            .durable_usage(),
        usage
    );

    let mut artifact_drift = usage.clone();
    artifact_drift.artifact_bytes = artifact_drift.disk_bytes + 1;
    assert_eq!(
        RuntimeResourceLedger::restore(&request, &artifact_drift),
        Err(RuntimeHardeningError::InvalidLimits)
    );

    let mut event_drift = usage.clone();
    event_drift.event_bytes = u64::from(event_drift.event_count)
        .checked_mul(MAX_RUNTIME_EVENT_ENVELOPE_BYTES)
        .expect("fixture event product fits")
        + 1;
    assert_eq!(
        RuntimeResourceLedger::restore(&request, &event_drift),
        Err(RuntimeHardeningError::InvalidLimits)
    );

    let retry_drift = RuntimeResourceUsage {
        retry_count: 1,
        ..usage
    };
    assert_eq!(
        RuntimeResourceLedger::restore(&request, &retry_drift),
        Err(RuntimeHardeningError::InvalidLimits)
    );
}
