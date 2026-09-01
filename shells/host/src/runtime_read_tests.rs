use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use agentmage_capability_read_only::{
    ArtifactAttemptLedger, ArtifactExecutionSignal, ArtifactLimits, ArtifactOutcome,
    ArtifactRequest, ArtifactToolKind, GIT_INSPECTION_TOOL_ID, GitInspectionOperation,
    GitInspectionOutcome, GitInspectionRequest, NeverCancelled, ReadOnlyEncoding, ReadOnlyLimits,
    ReadOnlyOutcome, ReadOnlyRequest, ReadOnlyToolKind, SnapshotEntry, SnapshotEntryKind,
    WorkspaceSnapshot, execute_read_only, parse_git_inspection, read_only_tool_kind,
    validate_git_inspection_request,
};
use agentmage_kernel_contracts::{
    AgentStateKind, ApprovalId, AuthorityClass, BudgetLimit, BudgetResource,
    CONTRACT_SCHEMA_VERSION, CanonicalApprovalRequirement, CanonicalDiagnosticDisclosure,
    CanonicalEffectClass, CanonicalExecutionBudgets, CanonicalIdempotencyRequirement,
    CanonicalRetryClass, CanonicalSchemaBinding, CanonicalStepExecutionPolicy,
    CanonicalVerificationRequirement, CanonicalWorkflowDefinition, CanonicalWorkflowStep,
    ContextPacketId, ContractPayload, DataSensitivity, EvidenceId, EvidenceKind, EvidenceReference,
    ExactModelProfile, GrantId, ModelContextPacket, ModelMessage, ModelMessageId, ModelMessageRole,
    ModelProposalKind, ModelResourceReport, ModelRunRequest, ModelRunResult, ModelRunTerminalState,
    ModelStreamId, ModelToolCallCandidate, OperationOutcome, PlanId, PolicyId, PostconditionResult,
    ReceiptId, RepositorySnapshotId, RollbackPlan, RuntimeArtifactKind, RuntimeEvent,
    RuntimeOperationId, RuntimeOutcome, RuntimeRunId, RuntimeRunLimits, RuntimeRunRequest,
    RuntimeSessionMode, SessionId, StateChange, StopCondition, StopConditionKind, Task, TaskId,
    TaskStatus, ToolCall, ToolCatalogId, ToolDefinition, ToolId, ToolResult, VerifierCandidate,
    VerifierDisposition, VerifierId, VerifierRecordId, VerifierSource, WorkPacket, WorkPacketId,
    WorkPacketState, WorkspaceId, to_canonical_json,
};
use agentmage_kernel_engine::engineering_records::canonical_record_sha256;
use agentmage_kernel_engine::model_codec::proposal_digest;
use agentmage_kernel_engine::model_orchestration_profile::{
    AllocatedContextPartition, ContextPartitionDisposition, ExactTokenCounterBinding,
    ModelContextWindowPlan,
};
use agentmage_kernel_engine::runtime_coordinator::{
    runtime_tool_catalog_sha256, seal_runtime_run_request, verify_runtime_outcome,
};
use agentmage_kernel_engine::runtime_event::RuntimeEventSequence;
use agentmage_kernel_engine::runtime_loop::{
    ReusableRuntimeCoordinator, RuntimeClock, RuntimeContextPort, RuntimeCoordinatorStep,
    RuntimeModelPort, RuntimePermissionEvaluation, RuntimePortFailure, RuntimeToolBoundary,
    RuntimeToolExecution, RuntimeVerificationInput, RuntimeVerifierPort, runtime_tool_references,
};
use agentmage_kernel_engine::source_preparation::{
    ExactSourceTokenCounter, PreparedSourceRetention, SourceAdmissionRequest, SourceEncoding,
    SourceMediaFamily, SourcePreparationError, SourcePreparationLimits, SourcePreparationService,
};
use agentmage_kernel_engine::source_runtime_context::PreparedSourceRuntimeContext;
use agentmage_kernel_engine::verified_workflow_supervisor::{
    WorkflowAttemptRequest, WorkflowAttemptResources, WorkflowSupervisorError,
    WorkflowSupervisorState, supervise_verified_workflow,
};
use serde::Deserialize;
use sha2::{Digest, Sha256};

use crate::coding_client::DenyHeadlessApproval;
use crate::source_artifact_runtime::dispatch_native_source_artifact;
use crate::workflow_supervisor::{
    ClassifiedWorkflowOutcome, CoordinatorWorkflowAttemptPort, PreparedWorkflowCoordinatorAttempt,
    WorkflowCoordinatorAttemptFactory, WorkflowOutcomeClassifier,
};

use crate::runtime_tools::read_only_runtime_registry;

const SHA: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const SNAPSHOT_ID: &str = "snapshot-native-read-0001";

#[derive(Deserialize)]
struct TestProfileCatalog {
    profiles: Vec<ExactModelProfile>,
}

struct NativeReadFakeModel {
    profile: ExactModelProfile,
    scripts: VecDeque<NativeReadModelStep>,
    calls: u32,
    identity_prefix: String,
}

enum NativeReadModelStep {
    Tool {
        tool_id: ToolId,
        arguments: ContractPayload,
    },
    Completion,
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
        let step = self
            .scripts
            .pop_front()
            .ok_or(RuntimePortFailure::ResourceExhausted)?;
        self.calls += 1;
        let (kind, payload, tool_call) = match step {
            NativeReadModelStep::Tool { tool_id, arguments } => (
                ModelProposalKind::ToolCall,
                None,
                Some(ModelToolCallCandidate {
                    tool_call_id: agentmage_kernel_contracts::ToolCallId::from_raw(format!(
                        "{}-call-{}",
                        self.identity_prefix, self.calls
                    )),
                    tool_id,
                    tool_version: "1.0.0".to_owned(),
                    arguments,
                }),
            ),
            NativeReadModelStep::Completion => (
                ModelProposalKind::CompletionCandidate,
                Some(payload(
                    "runtime.native-read.answer",
                    b"lib.rs contains the fixture function",
                )),
                None,
            ),
        };
        let mut proposal = agentmage_kernel_contracts::ClosedModelProposal {
            schema_version: CONTRACT_SCHEMA_VERSION,
            proposal_id: agentmage_kernel_contracts::ProposalId::from_raw(format!(
                "{}-proposal-{}",
                self.identity_prefix, self.calls
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
    executions: u32,
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
        self.executions = self
            .executions
            .checked_add(1)
            .ok_or(RuntimePortFailure::ResourceExhausted)?;
        let (bytes, source_id, object_id) =
            if let Some(kind) = read_only_tool_kind(&call.tool_id, &call.tool_version) {
                let result =
                    execute_read_only(kind, &call.arguments.bytes, &self.snapshot, &NeverCancelled);
                if !result.verify(kind)
                    || !matches!(
                        result.outcome,
                        ReadOnlyOutcome::Succeeded | ReadOnlyOutcome::NoResult
                    )
                {
                    return Err(RuntimePortFailure::Invalid);
                }
                (
                    serde_json::to_vec(&result).map_err(|_| RuntimePortFailure::Invalid)?,
                    kind.id().to_owned(),
                    "workspace-projection".to_owned(),
                )
            } else if call.tool_id.as_str() == GIT_INSPECTION_TOOL_ID {
                let git_request = validate_git_inspection_request(&call.arguments.bytes)
                    .map_err(|_| RuntimePortFailure::Invalid)?;
                let result = parse_git_inspection(
                    &git_request,
                    &"e".repeat(64),
                    &"f".repeat(64),
                    &request.repository_snapshot_sha256,
                    true,
                    b"## main\n M src/lib.rs\n",
                )
                .map_err(|_| RuntimePortFailure::Invalid)?;
                if !result.verify() || result.outcome == GitInspectionOutcome::Failed {
                    return Err(RuntimePortFailure::Invalid);
                }
                (
                    serde_json::to_vec(&result).map_err(|_| RuntimePortFailure::Invalid)?,
                    GIT_INSPECTION_TOOL_ID.to_owned(),
                    "repository-status".to_owned(),
                )
            } else {
                return Err(RuntimePortFailure::Invalid);
            };
        let evidence = EvidenceReference {
            schema_version: CONTRACT_SCHEMA_VERSION,
            evidence_id: EvidenceId::from_raw(format!(
                "native-read-evidence-{:04}",
                self.executions
            )),
            kind: EvidenceKind::ToolOutput,
            source_id,
            object_id,
            fragment: Some(format!("result:{}", self.executions)),
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
            receipt_id: ReceiptId::from_raw(format!("native-read-receipt-{:04}", self.executions)),
            receipt_sha256: sha256(
                format!("native-read-receipt-{:04}", self.executions).as_bytes(),
            ),
            result: tool_result,
            result_output_kind: Some(RuntimeArtifactKind::Report),
            artifact_candidates: Vec::new(),
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

struct SourceCounter {
    binding: ExactTokenCounterBinding,
}

impl ExactSourceTokenCounter for SourceCounter {
    fn binding(&self) -> ExactTokenCounterBinding {
        self.binding.clone()
    }

    fn count_tokens(&mut self, content: &str) -> Result<u32, SourcePreparationError> {
        u32::try_from(content.split_whitespace().count())
            .map_err(|_| SourcePreparationError::ResourceLimit)
    }
}

struct SourceAwareFakeModel {
    inner: NativeReadFakeModel,
    saw_prepared_source: Arc<AtomicBool>,
}

impl RuntimeModelPort for SourceAwareFakeModel {
    fn exact_profile(&self) -> &ExactModelProfile {
        self.inner.exact_profile()
    }

    fn run_model(
        &mut self,
        request: &ModelRunRequest,
        context: &ModelContextPacket,
        cancellation: Option<&dyn agentmage_kernel_contracts::ModelCancellationProbe>,
    ) -> Result<ModelRunResult, RuntimePortFailure> {
        if context.messages.iter().any(|message| {
            std::str::from_utf8(&message.content.bytes)
                .is_ok_and(|text| text.contains("prepared_fixture"))
        }) {
            self.saw_prepared_source.store(true, Ordering::SeqCst);
        }
        self.inner.run_model(request, context, cancellation)
    }
}

struct PreparedArtifactBoundary {
    sources: Arc<SourcePreparationService>,
    ledger: ArtifactAttemptLedger,
    executions: u32,
    identity_prefix: String,
}

impl RuntimeToolBoundary for PreparedArtifactBoundary {
    fn evaluate(
        &mut self,
        _request: &RuntimeRunRequest,
        _operation_id: &RuntimeOperationId,
        _definition: &ToolDefinition,
        call: &ToolCall,
        now_epoch_ms: u64,
    ) -> Result<RuntimePermissionEvaluation, RuntimePortFailure> {
        Ok(RuntimePermissionEvaluation::Allow {
            approval_id: ApprovalId::from_raw(format!("{}-approval", self.identity_prefix)),
            preview_sha256: sha256(&call.arguments.bytes),
            expires_at_epoch_ms: now_epoch_ms.saturating_add(10_000),
            grant_id: GrantId::from_raw(format!(
                "{}-grant-{}",
                self.identity_prefix,
                call.tool_call_id.as_str()
            )),
            decision_sha256: sha256(b"prepared source fixture allow"),
            authority_sha256: sha256(b"prepared source fixture consumed authority"),
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
        let kind = ArtifactToolKind::ALL
            .into_iter()
            .find(|kind| kind.id() == call.tool_id.as_str())
            .ok_or(RuntimePortFailure::Invalid)?;
        let result = dispatch_native_source_artifact(
            kind,
            &call.arguments.bytes,
            &self.sources,
            true,
            ArtifactExecutionSignal::Continue,
            &mut self.ledger,
        )
        .map_err(|_| RuntimePortFailure::Invalid)?;
        if !result.verify(kind) || !result.production_execution {
            return Err(RuntimePortFailure::Invalid);
        }
        self.executions = self
            .executions
            .checked_add(1)
            .ok_or(RuntimePortFailure::ResourceExhausted)?;
        let bytes = serde_json::to_vec(&result).map_err(|_| RuntimePortFailure::Invalid)?;
        let outcome = match result.outcome {
            ArtifactOutcome::Succeeded | ArtifactOutcome::NoResult | ArtifactOutcome::Truncated => {
                OperationOutcome::Succeeded
            }
            ArtifactOutcome::Denied | ArtifactOutcome::Restricted => OperationOutcome::Denied,
            ArtifactOutcome::Cancelled => OperationOutcome::Cancelled,
            ArtifactOutcome::TimedOut => OperationOutcome::TimedOut,
            ArtifactOutcome::Stale
            | ArtifactOutcome::Unsupported
            | ArtifactOutcome::OutOfRange
            | ArtifactOutcome::Failed => OperationOutcome::Failed,
        };
        let evidence = EvidenceReference {
            schema_version: CONTRACT_SCHEMA_VERSION,
            evidence_id: EvidenceId::from_raw(format!(
                "{}-evidence-{:04}",
                self.identity_prefix, self.executions
            )),
            kind: EvidenceKind::ToolOutput,
            source_id: result
                .source_id
                .clone()
                .unwrap_or_else(|| kind.id().to_owned()),
            object_id: result.output_identity.clone(),
            fragment: Some(format!(
                "artifact-result:{};freshness:{}",
                result.call_id,
                result.freshness_sha256.as_deref().unwrap_or("none")
            )),
            content_sha256: sha256(&bytes),
            observed_revision: Some(request.repository_snapshot_id.as_str().to_owned()),
        };
        Ok(RuntimeToolExecution {
            receipt_id: ReceiptId::from_raw(format!(
                "{}-receipt-{:04}",
                self.identity_prefix, self.executions
            )),
            receipt_sha256: result.receipt.receipt_sha256.clone(),
            result: ToolResult {
                schema_version: CONTRACT_SCHEMA_VERSION,
                tool_call_id: call.tool_call_id.clone(),
                correlation_id: call.correlation_id.clone(),
                outcome,
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
            },
            result_output_kind: Some(RuntimeArtifactKind::Report),
            artifact_candidates: Vec::new(),
        })
    }
}

#[test]
fn story_22_5_prepared_source_flows_through_model_tool_verifier_and_terminal_lineage() {
    let registry = read_only_runtime_registry().expect("native read registry");
    let profile = deterministic_profile();
    let (sources, source, binding) = prepared_service(&profile);
    let sources = Arc::new(sources);
    let context = PreparedSourceRuntimeContext::new(
        Arc::clone(&sources),
        source_window_plan(&profile),
        SourceCounter { binding },
        [source.source_id.clone()],
        false,
    )
    .expect("prepared source context");
    let definition = registry
        .get_tool(&ToolId::from_raw(ArtifactToolKind::Search.id()), "1.0.0")
        .expect("artifact search definition");
    let search_bytes = serde_json::to_vec(&ArtifactRequest {
        schema_version: 1,
        call_id: "prepared-source-search".to_owned(),
        source_id: Some(source.source_id.clone()),
        section_id: None,
        range: None,
        query: Some("prepared_fixture".to_owned()),
        freshness_sha256: Some(source.manifest_sha256.clone()),
        output_identity: "prepared-source-search-output".to_owned(),
        limits: ArtifactLimits::default(),
        call_depth: 0,
    })
    .expect("artifact search request");
    let saw_prepared_source = Arc::new(AtomicBool::new(false));
    let request = runtime_request(profile.clone(), &registry);
    let admitted_request = request.clone();
    let mut coordinator = ReusableRuntimeCoordinator::new(
        request,
        SourceAwareFakeModel {
            inner: NativeReadFakeModel {
                profile,
                scripts: [
                    NativeReadModelStep::Tool {
                        tool_id: definition.tool_id.clone(),
                        arguments: ContractPayload {
                            schema: definition.input_schema.clone(),
                            media_type: "application/json".to_owned(),
                            sha256: sha256(&search_bytes),
                            bytes: search_bytes,
                        },
                    },
                    NativeReadModelStep::Completion,
                ]
                .into_iter()
                .collect(),
                calls: 0,
                identity_prefix: "prepared-source".to_owned(),
            },
            saw_prepared_source: Arc::clone(&saw_prepared_source),
        },
        context,
        registry,
        PreparedArtifactBoundary {
            sources,
            ledger: ArtifactAttemptLedger::default(),
            executions: 0,
            identity_prefix: "prepared-source".to_owned(),
        },
        NativeReadVerifier(VerifierId::from_raw("prepared-source-verifier-0001")),
        TestClock(3_000),
    )
    .expect("prepared source coordinator composes");

    let RuntimeCoordinatorStep::Complete { outcome } = coordinator
        .run_until_boundary(None, None)
        .expect("prepared source vertical slice completes")
    else {
        panic!("read-only prepared source cannot pause");
    };
    assert_eq!(outcome.state, AgentStateKind::Success);
    assert_eq!(outcome.tool_call_count, 1);
    assert_eq!(outcome.receipt_ids.len(), 1);
    assert!(saw_prepared_source.load(Ordering::SeqCst));
    assert!(outcome.evidence.iter().any(|reference| {
        reference.source_id == source.source_id
            && reference
                .fragment
                .as_deref()
                .is_some_and(|value| value.contains(&source.manifest_sha256))
    }));
    assert!(outcome.answer_evidence.is_some());
    verify_runtime_outcome(&outcome, &admitted_request).expect("outcome verifies");
    let mut sequence = RuntimeEventSequence::new();
    for event in coordinator.events() {
        sequence.push(event).expect("ordered event");
    }
    assert!(sequence.is_terminal());
    if std::env::var_os("AGENTMAGE_STORY_22_5_EVIDENCE").is_some() {
        println!(
            "STORY_22_5_EVIDENCE={}",
            serde_json::to_string(&serde_json::json!({
                "schema_version": 1,
                "record_type": "agentmage-story-22-5-vertical-slice-golden",
                "source_manifest": source,
                "context_manifests": coordinator.context_port().context_manifests(),
                "events": coordinator.events(),
                "outcome": outcome,
                "production_artifact_dispatch": true,
                "model_direct_tool_authority": false,
                "model_completion_authority": false,
                "duplicate_effect_count": 0,
                "replay_count": 0
            }))
            .expect("evidence serializes")
        );
    }
}

#[test]
fn story_22_5_stale_required_source_and_tokenizer_drift_stop_before_model() {
    let registry = read_only_runtime_registry().expect("native read registry");
    let profile = deterministic_profile();
    let (mut stale_sources, source, binding) = prepared_service(&profile);
    stale_sources
        .mark_stale(&source.source_id, &source.manifest_sha256)
        .expect("source becomes stale");
    let stale_sources = Arc::new(stale_sources);
    let stale_context = PreparedSourceRuntimeContext::new(
        Arc::clone(&stale_sources),
        source_window_plan(&profile),
        SourceCounter {
            binding: binding.clone(),
        },
        [source.source_id.clone()],
        false,
    )
    .expect("stale state remains explicitly representable");
    let request = runtime_request(profile.clone(), &registry);
    let mut coordinator = ReusableRuntimeCoordinator::new(
        request,
        NativeReadFakeModel {
            profile: profile.clone(),
            scripts: [NativeReadModelStep::Completion].into_iter().collect(),
            calls: 0,
            identity_prefix: "stale-source".to_owned(),
        },
        stale_context,
        registry,
        SnapshotReadBoundary {
            snapshot: native_snapshot(),
            executions: 0,
        },
        NativeReadVerifier(VerifierId::from_raw("stale-source-verifier-0001")),
        TestClock(4_000),
    )
    .expect("stale source coordinator composes without pretending freshness");
    let RuntimeCoordinatorStep::Complete { outcome } = coordinator
        .run_until_boundary(None, None)
        .expect("stale source closes with a truthful non-success")
    else {
        panic!("stale source cannot request approval");
    };
    assert_eq!(outcome.state, AgentStateKind::Failed);
    assert_eq!(outcome.model_call_count, 0);
    assert_eq!(outcome.unresolved_codes, ["runtime.context.failed"]);

    let (current_sources, current_source, binding) = prepared_service(&profile);
    let mut drifted_plan = source_window_plan(&profile);
    drifted_plan.token_counter_sha256 = "f".repeat(64);
    let mut context = PreparedSourceRuntimeContext::new(
        Arc::new(current_sources),
        drifted_plan,
        SourceCounter { binding },
        [current_source.source_id],
        false,
    )
    .expect("drift is checked against the exact request at delivery");
    let request = runtime_request(
        profile,
        &read_only_runtime_registry().expect("second registry"),
    );
    assert_eq!(
        context.build_context(
            &request,
            ContextPacketId::from_raw("drifted-context-packet"),
            1,
            &[],
            &[],
        ),
        Err(RuntimePortFailure::Invalid)
    );
}

#[test]
fn story_50_3_artifact_heavy_steps_use_one_supervisor_and_common_coordinator() {
    let (definition, policies) = foundational_workflow();
    let mut factory = FoundationalAttemptFactory::default();
    let mut port = CoordinatorWorkflowAttemptPort::new(&mut factory, FoundationalClassifier);
    let result = supervise_verified_workflow(
        "foundational-runtime-execution",
        &definition,
        &policies,
        None,
        &mut port,
    )
    .expect("artifact-heavy workflow completes through the common coordinator");

    assert_eq!(result.state, WorkflowSupervisorState::Succeeded);
    assert_eq!(
        result.completed_step_ids,
        [
            "repository-review",
            "long-log-diagnosis",
            "mixed-artifact-plan"
        ]
    );
    assert_eq!(result.attempts.len(), 3);
    assert_eq!(result.budgets.attempts, 3);
    assert_eq!(result.budgets.tool_calls, 3);
    assert!(
        factory
            .saw_prepared_source
            .iter()
            .all(|seen| seen.load(Ordering::SeqCst))
    );
    assert_eq!(
        result
            .attempts
            .iter()
            .flat_map(|attempt| attempt.receipt_ids.iter())
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        3
    );
}

#[derive(Default)]
struct FoundationalAttemptFactory {
    saw_prepared_source: Vec<Arc<AtomicBool>>,
}

impl WorkflowCoordinatorAttemptFactory for &mut FoundationalAttemptFactory {
    type Coordinator = ReusableRuntimeCoordinator<
        SourceAwareFakeModel,
        PreparedSourceRuntimeContext<SourceCounter>,
        PreparedArtifactBoundary,
        NativeReadVerifier,
        TestClock,
    >;
    type Approvals = DenyHeadlessApproval;

    fn prepare(
        &mut self,
        attempt: WorkflowAttemptRequest<'_>,
    ) -> Result<
        PreparedWorkflowCoordinatorAttempt<Self::Coordinator, Self::Approvals>,
        WorkflowSupervisorError,
    > {
        let ordinal = attempt.attempt_ordinal;
        let prefix = format!("foundational-{}-{ordinal}", attempt.step_id);
        let registry =
            read_only_runtime_registry().map_err(|_| WorkflowSupervisorError::RuntimeFailed)?;
        let profile = deterministic_profile();
        let (sources, source, binding) = prepared_service(&profile);
        let sources = Arc::new(sources);
        let context = PreparedSourceRuntimeContext::new(
            Arc::clone(&sources),
            source_window_plan(&profile),
            SourceCounter { binding },
            [source.source_id.clone()],
            false,
        )
        .map_err(|_| WorkflowSupervisorError::RuntimeFailed)?;
        let definition = registry
            .get_tool(&ToolId::from_raw(ArtifactToolKind::Search.id()), "1.0.0")
            .ok_or(WorkflowSupervisorError::RuntimeFailed)?;
        let search_bytes = serde_json::to_vec(&ArtifactRequest {
            schema_version: 1,
            call_id: format!("{prefix}-search"),
            source_id: Some(source.source_id.clone()),
            section_id: None,
            range: None,
            query: Some("prepared_fixture".to_owned()),
            freshness_sha256: Some(source.manifest_sha256),
            output_identity: format!("{prefix}-output"),
            limits: ArtifactLimits::default(),
            call_depth: 0,
        })
        .map_err(|_| WorkflowSupervisorError::RuntimeFailed)?;
        let seen = Arc::new(AtomicBool::new(false));
        self.saw_prepared_source.push(Arc::clone(&seen));
        let request = foundational_runtime_request(profile.clone(), &registry, attempt.step_id);
        let coordinator = ReusableRuntimeCoordinator::new(
            request.clone(),
            SourceAwareFakeModel {
                inner: NativeReadFakeModel {
                    profile,
                    scripts: [
                        NativeReadModelStep::Tool {
                            tool_id: definition.tool_id.clone(),
                            arguments: ContractPayload {
                                schema: definition.input_schema.clone(),
                                media_type: "application/json".to_owned(),
                                sha256: sha256(&search_bytes),
                                bytes: search_bytes,
                            },
                        },
                        NativeReadModelStep::Completion,
                    ]
                    .into_iter()
                    .collect(),
                    calls: 0,
                    identity_prefix: prefix.clone(),
                },
                saw_prepared_source: seen,
            },
            context,
            registry,
            PreparedArtifactBoundary {
                sources,
                ledger: ArtifactAttemptLedger::default(),
                executions: 0,
                identity_prefix: prefix.clone(),
            },
            NativeReadVerifier(VerifierId::from_raw(format!("{prefix}-verifier"))),
            TestClock(10_000 * u64::from(ordinal)),
        )
        .map_err(|_| WorkflowSupervisorError::RuntimeFailed)?;
        Ok(PreparedWorkflowCoordinatorAttempt {
            attempt_id: format!("{prefix}-attempt"),
            attempt_ordinal: ordinal,
            preflight_policy_sha256: attempt.policy_sha256.to_owned(),
            preflight_current: true,
            request,
            coordinator,
            approvals: DenyHeadlessApproval,
        })
    }
}

struct FoundationalClassifier;

impl WorkflowOutcomeClassifier for FoundationalClassifier {
    fn classify(
        &mut self,
        _attempt: WorkflowAttemptRequest<'_>,
        _request: &RuntimeRunRequest,
        outcome: &RuntimeOutcome,
    ) -> Result<ClassifiedWorkflowOutcome, WorkflowSupervisorError> {
        if !outcome.state.is_success() {
            return Err(WorkflowSupervisorError::EvidenceDenied);
        }
        Ok(ClassifiedWorkflowOutcome {
            failure_class: None,
            uncertain_effect: false,
            resources: WorkflowAttemptResources {
                context_tokens: 16,
                memory_bytes: 32 * 1024,
                cost_minor_units: 0,
            },
        })
    }
}

fn foundational_runtime_request(
    profile: ExactModelProfile,
    registry: &agentmage_kernel_engine::tooling::ToolRegistry,
    step_id: &str,
) -> RuntimeRunRequest {
    let mut request = runtime_request(profile, registry);
    request.run_id = RuntimeRunId::from_raw(format!("foundational-{step_id}-run"));
    request.session_id = SessionId::from_raw("foundational-runtime-session");
    request.task.task_id = TaskId::from_raw(format!("foundational-{step_id}-task"));
    request.task.session_id = request.session_id.clone();
    request.task.objective = format!("Execute the {step_id} artifact workflow");
    request.work_packet.work_packet_id =
        WorkPacketId::from_raw(format!("foundational-{step_id}-packet"));
    request.work_packet.task_id = request.task.task_id.clone();
    request
        .work_packet
        .objective
        .clone_from(&request.task.objective);
    request.policy_id = PolicyId::from_raw(format!("foundational-{step_id}-policy"));
    request.request_sha256 = "0".repeat(64);
    seal_runtime_run_request(request).expect("step request seals")
}

fn foundational_workflow() -> (
    CanonicalWorkflowDefinition,
    Vec<CanonicalStepExecutionPolicy>,
) {
    let budgets = CanonicalExecutionBudgets {
        turns: 5,
        tokens: 1_024,
        duration_ms: 60_000,
        tool_calls: 4,
        attempts: 2,
        no_progress_events: 2,
        output_bytes: 4_096,
        memory_bytes: 1_000_000,
        cost_minor_units: 0,
    };
    let step_ids = [
        "repository-review",
        "long-log-diagnosis",
        "mixed-artifact-plan",
    ];
    let mut policies = step_ids
        .iter()
        .enumerate()
        .map(|(index, step_id)| CanonicalStepExecutionPolicy {
            schema_version: CONTRACT_SCHEMA_VERSION,
            policy_id: format!("foundational-{step_id}-execution-policy"),
            plan_id: PlanId::from_raw("foundational-runtime-plan"),
            plan_step_id: agentmage_kernel_contracts::PlanStepId::from_raw(format!(
                "foundational-runtime-plan:step:{:04}",
                index + 1
            )),
            plan_revision: 1,
            preflight_policy_id: format!("foundational-{step_id}-preflight"),
            required_preflight_ids: vec!["workspace-current".to_owned()],
            side_effect_policy_id: "foundational-read-only-effects".to_owned(),
            effect_class: CanonicalEffectClass::ReadOnly,
            approval_policy_id: "foundational-no-approval".to_owned(),
            approval_requirement: CanonicalApprovalRequirement::NotRequired,
            idempotency_policy_id: "foundational-read-idempotency".to_owned(),
            idempotency_key_requirement: CanonicalIdempotencyRequirement::NotApplicable,
            verifier_policy_id: format!("foundational-{step_id}-verifier-policy"),
            verification_requirement: CanonicalVerificationRequirement::VerifierEvidenceRequired,
            required_verifier_ids: vec![format!("foundational-{step_id}-verifier")],
            deferral_reason_code: None,
            retry_policy_id: "foundational-read-retry".to_owned(),
            retry_class: CanonicalRetryClass::RecoverableRead,
            budget_policy_id: "foundational-budgets".to_owned(),
            budgets: budgets.clone(),
            diagnostic_policy_id: "foundational-diagnostics".to_owned(),
            diagnostic_disclosure: CanonicalDiagnosticDisclosure::ContentFreeCodes,
            recorded_at: "2026-08-31T12:00:00Z".to_owned(),
            policy_sha256: "0".repeat(64),
        })
        .collect::<Vec<_>>();
    for policy in &mut policies {
        policy.policy_sha256 = canonical_record_sha256(policy).expect("policy seals");
    }
    let mut definition = CanonicalWorkflowDefinition {
        schema_version: CONTRACT_SCHEMA_VERSION,
        workflow_id: "foundational-artifact-workflow".to_owned(),
        workflow_version: 1,
        input_schema: CanonicalSchemaBinding {
            schema_id: "foundational-artifact-input".to_owned(),
            schema_version: 1,
            schema_sha256: SHA.to_owned(),
        },
        output_schema: CanonicalSchemaBinding {
            schema_id: "foundational-artifact-output".to_owned(),
            schema_version: 1,
            schema_sha256: SHA.to_owned(),
        },
        steps: step_ids
            .iter()
            .enumerate()
            .map(|(index, step_id)| CanonicalWorkflowStep {
                step_id: (*step_id).to_owned(),
                depends_on: index
                    .checked_sub(1)
                    .map(|prior| vec![step_ids[prior].to_owned()])
                    .unwrap_or_default(),
                model_role: Some("engineering-worker".to_owned()),
                tool_id: Some(ArtifactToolKind::Search.id().to_owned()),
                effect_class: CanonicalEffectClass::ReadOnly,
                retry_class: CanonicalRetryClass::RecoverableRead,
                verifier_ids: vec![format!("foundational-{step_id}-verifier")],
                budgets: budgets.clone(),
            })
            .collect(),
        entry_step_ids: vec![step_ids[0].to_owned()],
        definition_sha256: "0".repeat(64),
    };
    definition.definition_sha256 = canonical_record_sha256(&definition).expect("definition seals");
    (definition, policies)
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

#[test]
fn story_23_4_fake_model_composes_multiple_reads_search_and_read_only_git() {
    let registry = read_only_runtime_registry().expect("native read registry");
    let profile = deterministic_profile();
    let multiple = read_model_step(
        &registry,
        ReadOnlyToolKind::ReadMultiple,
        ReadOnlyRequest {
            schema_version: 1,
            paths: vec![
                vec!["src".to_owned(), "lib.rs".to_owned()],
                vec!["src".to_owned(), "other.rs".to_owned()],
            ],
            query: None,
            byte_offset: Some(0),
            byte_count: Some(256),
            encoding: ReadOnlyEncoding::Utf8,
            limits: ReadOnlyLimits::default(),
            call_depth: 0,
        },
    );
    let search = read_model_step(
        &registry,
        ReadOnlyToolKind::SearchText,
        ReadOnlyRequest {
            schema_version: 1,
            paths: vec![vec!["src".to_owned()]],
            query: Some("fixture".to_owned()),
            byte_offset: Some(0),
            byte_count: Some(256),
            encoding: ReadOnlyEncoding::Utf8,
            limits: ReadOnlyLimits::default(),
            call_depth: 0,
        },
    );
    let git = git_model_step(
        &registry,
        GitInspectionRequest {
            schema_version: 1,
            operation: GitInspectionOperation::Status,
            revision: None,
            object_id: None,
            pathspecs: Vec::new(),
            max_records: 100,
            max_output_bytes: 64 * 1024,
        },
    );
    let request = runtime_request(profile.clone(), &registry);
    let admitted_request = request.clone();
    let observed_result = Arc::new(AtomicBool::new(false));
    let mut coordinator = ReusableRuntimeCoordinator::new(
        request,
        NativeReadFakeModel {
            profile,
            scripts: [multiple, search, git, NativeReadModelStep::Completion]
                .into_iter()
                .collect(),
            calls: 0,
            identity_prefix: "native-read-multi".to_owned(),
        },
        NativeReadContext {
            observed_result: Arc::clone(&observed_result),
        },
        registry,
        SnapshotReadBoundary {
            snapshot: native_snapshot(),
            executions: 0,
        },
        NativeReadVerifier(VerifierId::from_raw("native-read-verifier-0001")),
        TestClock(2_000),
    )
    .expect("multi-tool native coordinator composes");

    let RuntimeCoordinatorStep::Complete { outcome } = coordinator
        .run_until_boundary(None, None)
        .expect("multi-tool native fixture completes")
    else {
        panic!("pre-authorized reads cannot pause");
    };
    assert_eq!(outcome.state, AgentStateKind::Success);
    assert_eq!(outcome.turn_count, 4);
    assert_eq!(outcome.model_call_count, 4);
    assert_eq!(outcome.tool_call_count, 3);
    assert_eq!(outcome.receipt_ids.len(), 3);
    assert!(observed_result.load(Ordering::SeqCst));
    assert!(
        outcome
            .evidence
            .iter()
            .any(|item| { item.source_id == ReadOnlyToolKind::ReadMultiple.id() })
    );
    assert!(
        outcome
            .evidence
            .iter()
            .any(|item| { item.source_id == ReadOnlyToolKind::SearchText.id() })
    );
    assert!(
        outcome
            .evidence
            .iter()
            .any(|item| item.source_id == GIT_INSPECTION_TOOL_ID)
    );
    verify_runtime_outcome(&outcome, &admitted_request).expect("outcome verifies");
    let mut sequence = RuntimeEventSequence::new();
    for event in coordinator.events() {
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
    let read = read_model_step(&registry, ReadOnlyToolKind::ReadText, read_request);
    let request = runtime_request(profile.clone(), &registry);
    let admitted_request = request.clone();
    let observed_result = Arc::new(AtomicBool::new(false));
    let mut coordinator = ReusableRuntimeCoordinator::new(
        request,
        NativeReadFakeModel {
            profile,
            scripts: [read, NativeReadModelStep::Completion]
                .into_iter()
                .collect(),
            calls: 0,
            identity_prefix: "native-read-single".to_owned(),
        },
        NativeReadContext {
            observed_result: Arc::clone(&observed_result),
        },
        registry,
        SnapshotReadBoundary {
            snapshot: native_snapshot(),
            executions: 0,
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

fn read_model_step(
    registry: &agentmage_kernel_engine::tooling::ToolRegistry,
    kind: ReadOnlyToolKind,
    request: ReadOnlyRequest,
) -> NativeReadModelStep {
    let definition = registry
        .get_tool(&ToolId::from_raw(kind.id()), "1.0.0")
        .expect("read tool");
    let bytes = serde_json::to_vec(&request).expect("read request");
    NativeReadModelStep::Tool {
        tool_id: definition.tool_id.clone(),
        arguments: ContractPayload {
            schema: definition.input_schema.clone(),
            media_type: "application/json".to_owned(),
            sha256: sha256(&bytes),
            bytes,
        },
    }
}

fn git_model_step(
    registry: &agentmage_kernel_engine::tooling::ToolRegistry,
    request: GitInspectionRequest,
) -> NativeReadModelStep {
    let definition = registry
        .get_tool(&ToolId::from_raw(GIT_INSPECTION_TOOL_ID), "1.0.0")
        .expect("Git inspection tool");
    let bytes = serde_json::to_vec(&request).expect("Git request");
    NativeReadModelStep::Tool {
        tool_id: definition.tool_id.clone(),
        arguments: ContractPayload {
            schema: definition.input_schema.clone(),
            media_type: "application/json".to_owned(),
            sha256: sha256(&bytes),
            bytes,
        },
    }
}

fn native_snapshot() -> WorkspaceSnapshot {
    WorkspaceSnapshot {
        entries: vec![
            SnapshotEntry {
                path: vec!["src".to_owned()],
                kind: SnapshotEntryKind::Directory,
                bytes: Vec::new(),
                executable: false,
            },
            SnapshotEntry {
                path: vec!["src".to_owned(), "lib.rs".to_owned()],
                kind: SnapshotEntryKind::RegularFile,
                bytes: b"pub fn fixture() -> u64 { 42 }\n".to_vec(),
                executable: false,
            },
            SnapshotEntry {
                path: vec!["src".to_owned(), "other.rs".to_owned()],
                kind: SnapshotEntryKind::RegularFile,
                bytes: b"pub fn other_fixture() -> bool { true }\n".to_vec(),
                executable: false,
            },
        ],
    }
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

fn source_window_plan(profile: &ExactModelProfile) -> ModelContextWindowPlan {
    let source_tokens = profile.context.max_context_tokens.saturating_sub(5);
    let included = |tokens: u32| AllocatedContextPartition {
        requested_tokens: tokens,
        minimum_tokens: u32::from(tokens > 0),
        allocated_tokens: tokens,
        disposition: ContextPartitionDisposition::Included,
        reason_code: None,
    };
    let mut plan = ModelContextWindowPlan {
        schema_version: CONTRACT_SCHEMA_VERSION,
        model_profile_id: profile.profile_id.as_str().to_owned(),
        model_manifest_sha256: profile.manifest_sha256.clone(),
        model_runtime_sha256: sha256(
            &serde_json::to_vec(&profile.runtime).expect("runtime identity serializes"),
        ),
        tokenizer_sha256: profile.codec.tokenizer_sha256.clone(),
        token_counter_sha256: profile.context.token_counter_sha256.clone(),
        total_window_tokens: profile.context.max_context_tokens,
        system_and_tool_tokens: 1,
        user_input_tokens: 1,
        source_artifacts: included(source_tokens),
        retrieved_context: included(0),
        workflow_recovery_reserve_tokens: 1,
        output_reserve_tokens: 1,
        safety_margin_tokens: 1,
        unallocated_tokens: 0,
        plan_sha256: "0".repeat(64),
    };
    plan.plan_sha256 = sha256(&serde_json::to_vec(&plan).expect("plan serializes"));
    plan
}

fn prepared_service(
    profile: &ExactModelProfile,
) -> (
    SourcePreparationService,
    agentmage_kernel_engine::source_preparation::PreparedSourceManifest,
    ExactTokenCounterBinding,
) {
    let binding = ExactTokenCounterBinding {
        token_counter_id: profile.context.token_counter.clone(),
        token_counter_sha256: profile.context.token_counter_sha256.clone(),
        tokenizer_sha256: profile.codec.tokenizer_sha256.clone(),
    };
    let mut sources = SourcePreparationService::new();
    let source = sources
        .admit(
            SourceAdmissionRequest {
                source_id: "prepared-repository-source".to_owned(),
                request_id: "prepared-repository-request".to_owned(),
                source_kind: "repository_snapshot".to_owned(),
                protected_origin_sha256: sha256(b"protected repository source"),
                media_type: "text/plain".to_owned(),
                media_family: SourceMediaFamily::PlainText,
                encoding: SourceEncoding::Utf8,
                sensitivity: agentmage_kernel_contracts::ContextSensitivity::Internal,
                retention: PreparedSourceRetention::Ephemeral,
                collected_at_epoch_ms: 1_788_134_400_000,
                limits: SourcePreparationLimits::default(),
                bytes: b"pub fn prepared_fixture() -> u64 { 42 }\n".to_vec(),
            },
            &mut SourceCounter {
                binding: binding.clone(),
            },
            &mut || false,
        )
        .expect("prepared repository source");
    (sources, source, binding)
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
            max_turns: 5,
            max_model_calls: 5,
            max_tool_calls: 4,
            max_repeated_tool_calls: 2,
            max_tool_call_depth: 1,
            max_no_progress_turns: 2,
            max_context_refreshes: 5,
            max_events: 128,
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
