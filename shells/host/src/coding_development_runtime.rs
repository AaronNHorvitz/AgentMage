//! Executable development-only composition of the real coding coordinator and Linux boundaries.

use std::collections::{BTreeMap, VecDeque};
use std::fmt::Write as _;
use std::fs;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use agentmage_capability_read_only::{
    GIT_INSPECTION_TOOL_ID, GIT_INSPECTION_TOOL_VERSION, GitInspectionOperation,
    GitInspectionRequest, READ_ONLY_TOOL_VERSION, ReadOnlyEncoding, ReadOnlyLimits,
    ReadOnlyRequest, ReadOnlyToolKind,
};
use agentmage_capability_repository_map::{
    RepositoryMap, RepositoryObjectKind, StructuredArtifactClass, StructuredEdit,
    StructuredLanguage,
};
use agentmage_kernel_contracts::{
    ActorId, AdapterInstanceId, AuthorityClass, BudgetLimit, BudgetResource,
    CONTRACT_SCHEMA_VERSION, CheckedContextSummary, CheckedSummaryState, ClosedModelProposal,
    ContextAdmission, ContextBudget, ContextItemKind, ContextSensitivity, ContextSummaryId,
    ContractPayload, DataSensitivity, DecodingProfile, EvidenceKind, ExactModelProfile,
    FamilyCodecIdentity, GrantTarget, HardwareEnvelope, LocalEndpointIdentity, LocalTransport,
    ModelAdapterId, ModelArtifact, ModelCancellationProbe, ModelCapability, ModelCapabilityState,
    ModelCodecId, ModelFinishReason, ModelManifestId, ModelMessageRole, ModelModality,
    ModelProfileId, ModelProposalKind, ModelResourceReport, ModelRole, ModelRunRequest,
    ModelRunResult, ModelRunTerminalState, ModelRuntimeFailure, ModelRuntimeIdentity,
    ModelRuntimeKind, ModelStreamId, ModelTokenUsage, ModelToolCallCandidate, NetworkComponent,
    NetworkDestinationClass, NetworkObservation, PathResolutionIntent, PlanId,
    PlatformArchitecture, PlatformFamily, ProposalId, RepositorySnapshotId, RollbackPlan,
    RuntimeEventCursor, RuntimeEventKind, RuntimeIsolationObservation, RuntimeRunId,
    RuntimeRunLimits, SessionId, StopCondition, StopConditionKind, TaskId, ToolCallId,
    ToolCatalogId, ToolDefinition, ToolId, WorkPacket, WorkPacketId, WorkPacketState,
    WorkspaceAuthorizationId, WorkspaceId, WorkspacePath,
};
use agentmage_kernel_engine::{
    command_runner::{
        CommandBounds, CommandRegistry, CommandRequest, CommandRisk, CommandSpec,
        CommandWorkingDirectory,
    },
    instruction_provenance::build_instruction_ledger,
    model_runtime::{
        LocalModelController, ModelAdmissionCatalog, ModelUsePurpose, RejectedModelOutput,
    },
    repository_safety::{OwnedWorktreeRecord, WorktreeDisposition},
    runtime_artifact::{
        MAX_RUNTIME_ARTIFACT_BYTES, RUNTIME_CONTINUATION_MEDIA_TYPE, RUNTIME_REQUEST_MEDIA_TYPE,
        RuntimeArtifactReadRequest, decode_runtime_continuation_state,
    },
    runtime_coordinator::seal_runtime_run_request,
    runtime_loop::{RuntimeClock, RuntimeModelPort, RuntimePortFailure},
    strict_local::{
        AcquisitionExitDisposition, NetworkAttemptLedger, OfflinePreflightObservation,
        OfflineProofReceipt, OfflineProofWorkflow, StagedArtifactState, StrictLocalNetworkPolicy,
    },
    validation_template::{
        ValidationKind, ValidationParserKind, ValidationTemplateInput, ValidationTemplateRegistry,
        ValidationTemplateSource, seal_validation_template,
    },
};
use agentmage_platform_linux::{
    LinuxBoundedCommandExecutor, LinuxBoundedRepositoryInspectionExecutor, LinuxCommandManifest,
    LinuxDevelopmentPlatformAdapter, LinuxGitArtifact, LinuxRepositoryCollector,
    LinuxRepositoryInspectionManifest, LinuxRepositoryInventoryState, LinuxRepositoryScope,
    LinuxSandboxLimits, LinuxSandboxManifest, LinuxSandboxRunner,
    ensure_private_development_directory, linux_repository_path_sha256,
    open_linux_development_authority, resolve_development_linux_workspace_object,
    retain_rejected_development_output, select_development_linux_workspace,
};
use agentmage_platform_linux_inference::{
    GptOssHarmonyFamilyCodec, LinuxNativeModelAdapter, LlamaServerDriver, LlamaServerDriverConfig,
    LlamaServerLaunchProfile, MuseAtemFamilyCodec,
};
use sha2::{Digest, Sha256};

use crate::{
    coding_authority::{
        CodingRuntimePolicy, CodingRuntimePolicyRequest, build_coding_runtime_policy,
    },
    coding_changes::{
        CONTROLLED_CHANGE_TOOL_VERSION, CONTROLLED_CREATE_TOOL_ID, CodingWriteScope,
        ControlledFileClassification, ControlledFileCreationProposal, STRUCTURED_PATCH_TOOL_ID,
        StructuredPatchProposal, controlled_create_parent_observation_sha256,
    },
    coding_context::{
        CodingContextContinuityInput, CodingContextPort, CodingContextSource, CodingTokenCounter,
        checked_continuity_source_set_sha256,
    },
    coding_development_activation::{
        CODING_DEVELOPMENT_ACTIVATION, CodingDevelopmentActivation, CodingDevelopmentKeyProvider,
    },
    coding_harness::{DurableCodingCoordinator, compose_durable_coding_coordinator},
    coding_history::{
        CHANGE_HISTORY_OUTPUT_SCHEMA_ID, CHANGE_HISTORY_TOOL_ID, CODING_HISTORY_TOOL_VERSION,
        ChangeHistoryOutput, ChangeHistoryRequest, ROLLBACK_TOOL_ID, RollbackRequest,
    },
    coding_plan::build_coding_development_plan_binding,
    coding_run::{CodingRunRequestInput, build_ephemeral_coding_run_request},
    coding_session::{CodingSessionProfile, CodingSessionProfileInput},
    coding_tools::{
        TARGETED_VALIDATION_TOOL_ID, TARGETED_VALIDATION_TOOL_VERSION, TargetedValidationRequest,
    },
    coding_verifier::{CodingCompletionCandidate, CodingTerminalClaim, coding_completion_payload},
    linux_coding::LinuxCodingWorkspace,
    linux_coding_runtime::{
        CodingIdentitySource, LinuxCodingRuntimeBoundary, LinuxCodingRuntimeBoundaryInput,
        LinuxCodingSessionPreauthorization, OsCodingIdentitySource,
    },
    linux_repository_map::{LinuxRepositoryMapPolicy, build_development_linux_repository_map},
    native_chat_runtime::{NativeChatRuntimeError, NativeChatRuntimeFactory},
    runtime_transport::RuntimePrepareInput,
};

/// Explicitly non-qualified profile used only by the executable development fixture.
pub const SCRIPTED_PROFILE_ID: &str = "scripted-executable-fixture-32k-v1";
/// Exact Muse development candidate profile. It remains evaluation-only until campaign evidence.
pub const MUSE_DEVELOPMENT_PROFILE_ID: &str =
    "muse-glimmer-30b-q4-k-m-text-32k-fedora-coding-development";
/// Exact GPT-OSS development candidate profile. It remains evaluation-only until campaign evidence.
pub const GPT_OSS_DEVELOPMENT_PROFILE_ID: &str =
    "gpt-oss-20b-mxfp4-text-32k-fedora-coding-development";
const VALIDATION_ID: &str = "validation-unit";

/// Explicit proposal source selected for one development host process.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CodingDevelopmentModel {
    /// Deterministic non-model source used only by executable contract tests.
    Scripted,
    /// Pinned Muse Glimmer candidate through the native ATEM boundary.
    Muse,
    /// Pinned GPT-OSS candidate through the native Harmony boundary.
    GptOss,
}

impl CodingDevelopmentModel {
    /// Parses one exact CLI label without aliases or fallback.
    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "scripted" => Some(Self::Scripted),
            "muse" => Some(Self::Muse),
            "gpt-oss" => Some(Self::GptOss),
            _ => None,
        }
    }

    /// Returns the only exact profile identity accepted for this source.
    #[must_use]
    pub const fn profile_id(self) -> &'static str {
        match self {
            Self::Scripted => SCRIPTED_PROFILE_ID,
            Self::Muse => MUSE_DEVELOPMENT_PROFILE_ID,
            Self::GptOss => GPT_OSS_DEVELOPMENT_PROFILE_ID,
        }
    }

    const fn is_candidate(self) -> bool {
        !matches!(self, Self::Scripted)
    }
}

/// Closed executable-fixture scenarios. Every value is visibly non-model-qualified.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CodingDevelopmentScenario {
    /// Inspect Git state and return a verifier-backed no-op.
    NoOp,
    /// Observe a genuine failing test, repair one identifier, rerun, and verify.
    FailedTestRepair,
    /// Execute native-ordered command arguments, then require separate validation repair evidence.
    NativeCommandRepair,
    /// Hold one cancellable model call so actual signal propagation can be exercised.
    SlowCancel,
    /// Pause after the first safe checkpoint for an external host-stop resume probe.
    RestartRepair,
    /// Reject one complete protocol frame, then perform the ordinary scripted repair.
    ProtocolCorrection,
    /// Reject registered arguments before permission, then perform the scripted repair.
    ArgumentsCorrection,
    /// Reject two complete frames and prove the existing parser budget exhausts.
    RepeatedProtocolRejection,
    /// Stop at the protocol-rejection checkpoint and resume without repeating it.
    RestartProtocolCorrection,
    /// Create the exact absent source file and validate it.
    NewFile,
    /// Repair two bounded source files before one complete validation.
    MultiFile,
    /// Apply, inspect, and exactly roll back one structured source change.
    Rollback,
    /// Submit an unsupported success claim with no required evidence.
    FalseCompletion,
    /// Exhaust the exact canonical event budget before a second model turn.
    Overflow,
    /// Exhaust the predeclared run disk budget before any model or tool effect.
    DiskPressure,
    /// Exhaust the predeclared output budget without presenting partial output as complete.
    OutputPressure,
}

impl CodingDevelopmentScenario {
    /// Parses one exact CLI label.
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "no-op" => Some(Self::NoOp),
            "failed-test-repair" => Some(Self::FailedTestRepair),
            "native-command-repair" => Some(Self::NativeCommandRepair),
            "slow-cancel" => Some(Self::SlowCancel),
            "restart-repair" => Some(Self::RestartRepair),
            "protocol-correction" => Some(Self::ProtocolCorrection),
            "arguments-correction" => Some(Self::ArgumentsCorrection),
            "repeated-protocol-rejection" => Some(Self::RepeatedProtocolRejection),
            "restart-protocol-correction" => Some(Self::RestartProtocolCorrection),
            "new-file" => Some(Self::NewFile),
            "multi-file" => Some(Self::MultiFile),
            "rollback" => Some(Self::Rollback),
            "false-completion" => Some(Self::FalseCompletion),
            "overflow" => Some(Self::Overflow),
            "disk-pressure" => Some(Self::DiskPressure),
            "output-pressure" => Some(Self::OutputPressure),
            _ => None,
        }
    }
}

/// Content-free development composition failure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CodingDevelopmentRuntimeError {
    /// Explicit activation inputs changed or failed their closed contract.
    Activation,
    /// Repository inventory, mapping, or held-object binding failed closed.
    Repository,
    /// Exact session/model/tool profile composition failed closed.
    Profile,
    /// Worktree ownership identity could not be sealed.
    Worktree,
    /// Registered command or validation composition failed closed.
    Validation,
    /// Repository-derived change-plan composition failed closed.
    Plan,
    /// Workspace, repository, profile, tool, or policy composition failed closed.
    Composition,
    /// Required exact local process artifacts were absent or changed.
    Platform,
    /// The isolated development state/key boundary was unavailable.
    State,
}

impl CodingDevelopmentRuntimeError {
    /// Returns one stable diagnostic code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::Activation => "coding.development.activation-failed",
            Self::Repository => "coding.development.repository-failed",
            Self::Profile => "coding.development.profile-failed",
            Self::Worktree => "coding.development.worktree-failed",
            Self::Validation => "coding.development.validation-failed",
            Self::Plan => "coding.development.plan-failed",
            Self::Composition => "coding.development.composition-failed",
            Self::Platform => "coding.development.platform-failed",
            Self::State => "coding.development.state-failed",
        }
    }
}

type DevelopmentBoundary = LinuxCodingRuntimeBoundary<
    'static,
    'static,
    'static,
    OsCodingIdentitySource,
    LinuxBoundedCommandExecutor,
    LinuxBoundedRepositoryInspectionExecutor,
>;
/// Exact coordinator type served by the development IPC host.
pub type CodingDevelopmentCoordinator = DurableCodingCoordinator<
    CodingDevelopmentModelPort,
    CodingContextPort<DevelopmentTokenCounter>,
    DevelopmentBoundary,
    OsRuntimeClock,
>;

struct PreparedDevelopmentRun {
    request: agentmage_kernel_contracts::RuntimeRunRequest,
    policy: CodingRuntimePolicy,
    skip_scripted_steps: usize,
    preauthorization: Option<LinuxCodingSessionPreauthorization>,
    continuity: Option<CodingContextContinuityInput>,
    record_session: bool,
}

/// Factory that composes the real coordinator only for one explicit disposable activation.
pub struct CodingDevelopmentRuntimeFactory {
    activation: CodingDevelopmentActivation,
    scenario: CodingDevelopmentScenario,
    model: CodingDevelopmentModel,
    platform: &'static LinuxDevelopmentPlatformAdapter,
    profile: &'static CodingSessionProfile,
    workspace: &'static LinuxCodingWorkspace<'static, 'static>,
    supporting_sources: Vec<CodingContextSource>,
    session_id: Option<SessionId>,
    preauthorization: Option<LinuxCodingSessionPreauthorization>,
    record_session: Option<bool>,
    prior_run_id: Option<RuntimeRunId>,
    resume_requested: bool,
    prepared: BTreeMap<String, PreparedDevelopmentRun>,
}

impl CodingDevelopmentRuntimeFactory {
    /// Observes and freezes the exact repository/profile foundation before serving IPC.
    pub fn new(
        activation: CodingDevelopmentActivation,
        scenario: CodingDevelopmentScenario,
        model: CodingDevelopmentModel,
        resume_requested: bool,
    ) -> Result<Self, CodingDevelopmentRuntimeError> {
        activation
            .revalidate()
            .map_err(|_| CodingDevelopmentRuntimeError::Activation)?;
        let adapter_id = AdapterInstanceId::from_raw(format!(
            "coding-development-{}",
            &activation.marker_sha256()[..24]
        ));
        let platform = Box::leak(Box::new(
            LinuxDevelopmentPlatformAdapter::activate(
                CODING_DEVELOPMENT_ACTIVATION,
                activation.marker_sha256().to_owned(),
                adapter_id,
            )
            .map_err(|_| CodingDevelopmentRuntimeError::Platform)?,
        ));
        let (profile, workspace, supporting_sources) =
            build_repository_composition(&activation, platform, scenario, model)?;
        Ok(Self {
            activation,
            scenario,
            model,
            platform,
            profile,
            workspace,
            supporting_sources,
            session_id: None,
            preauthorization: None,
            record_session: None,
            prior_run_id: None,
            resume_requested,
            prepared: BTreeMap::new(),
        })
    }

    fn refresh_repository_composition(&mut self) -> Result<(), CodingDevelopmentRuntimeError> {
        let (profile, workspace, supporting_sources) = build_repository_composition(
            &self.activation,
            self.platform,
            self.scenario,
            self.model,
        )?;
        self.profile = profile;
        self.workspace = workspace;
        self.supporting_sources = supporting_sources;
        Ok(())
    }

    fn prepare_resume_request(
        &mut self,
        input: &RuntimePrepareInput,
    ) -> Result<agentmage_kernel_contracts::RuntimeRunRequest, NativeChatRuntimeError> {
        if input.engineering_session_id.is_some()
            || self.session_id.is_some()
            || !self.prepared.is_empty()
        {
            return Err(prepare_denied("resume-state"));
        }
        let mut key = CodingDevelopmentKeyProvider::open(&self.activation)
            .map_err(|_| prepare_denied("resume-key"))?;
        let authority = open_linux_development_authority(
            self.platform,
            self.activation.state_root(),
            &mut key,
            now_epoch_ms().map_err(|_| prepare_denied("resume-time"))?,
        )
        .map_err(|_| prepare_denied("resume-store"))?;
        let checkpoint = authority
            .authority()
            .current_session_checkpoint()
            .map_err(|_| prepare_denied("resume-checkpoint"))?
            .ok_or_else(|| prepare_denied("resume-checkpoint-absent"))?;
        let binding = authority
            .authority()
            .current_runtime_resume_binding()
            .map_err(|_| prepare_denied("resume-binding"))?
            .ok_or_else(|| prepare_denied("resume-binding-absent"))?;
        if binding.checkpoint_id != checkpoint.checkpoint_id
            || binding.checkpoint_sha256 != checkpoint.checkpoint_sha256
            || binding.session_id != checkpoint.session_id
            || binding.task_id != checkpoint.task_id
        {
            return Err(prepare_denied("resume-binding-drift"));
        }
        let request_references = binding
            .artifacts
            .iter()
            .filter(|reference| reference.media_type == RUNTIME_REQUEST_MEDIA_TYPE)
            .cloned()
            .collect::<Vec<_>>();
        let [request_reference] = request_references.as_slice() else {
            return Err(prepare_denied("resume-request-reference"));
        };
        let continuation_references = binding
            .artifacts
            .iter()
            .filter(|reference| reference.media_type == RUNTIME_CONTINUATION_MEDIA_TYPE)
            .cloned()
            .collect::<Vec<_>>();
        let [continuation_reference] = continuation_references.as_slice() else {
            return Err(prepare_denied("resume-continuation-reference"));
        };
        let request_bytes = authority
            .read_runtime_artifact(&RuntimeArtifactReadRequest {
                session_id: checkpoint.session_id.clone(),
                task_id: checkpoint.task_id.clone(),
                policy_sha256: checkpoint.policy_sha256.clone(),
                reference: request_reference.clone(),
                now_epoch_ms: now_epoch_ms().map_err(|_| prepare_denied("resume-time"))?,
                maximum_bytes: MAX_RUNTIME_ARTIFACT_BYTES,
            })
            .map_err(|_| prepare_denied("resume-request-read"))?;
        let base_request: agentmage_kernel_contracts::RuntimeRunRequest =
            serde_json::from_slice(&request_bytes)
                .map_err(|_| prepare_denied("resume-request-decode"))?;
        agentmage_kernel_engine::runtime_coordinator::verify_runtime_run_request(&base_request)
            .map_err(|_| prepare_denied("resume-request-invalid"))?;
        let continuation_bytes = authority
            .read_runtime_artifact(&RuntimeArtifactReadRequest {
                session_id: checkpoint.session_id.clone(),
                task_id: checkpoint.task_id.clone(),
                policy_sha256: checkpoint.policy_sha256.clone(),
                reference: continuation_reference.clone(),
                now_epoch_ms: now_epoch_ms().map_err(|_| prepare_denied("resume-time"))?,
                maximum_bytes: MAX_RUNTIME_ARTIFACT_BYTES,
            })
            .map_err(|_| prepare_denied("resume-continuation-read"))?;
        let continuation = decode_runtime_continuation_state(&continuation_bytes)
            .map_err(|_| prepare_denied("resume-continuation-decode"))?;
        let events = authority
            .authority()
            .runtime_events(&base_request.run_id)
            .map_err(|_| prepare_denied("resume-journal"))?;
        let last = events
            .last()
            .ok_or_else(|| prepare_denied("resume-journal-empty"))?;
        if base_request.event_cursor.is_some()
            || base_request.session_id != binding.session_id
            || base_request.task.task_id != binding.task_id
            || base_request.task.objective != input.prompt
            || base_request.model_profile != *self.profile.model_profile()
            || base_request.workspace_id != *self.profile.write_scope().workspace_id()
            || base_request.workspace_snapshot_sha256 != self.profile.worktree().record_sha256
            || base_request.repository_snapshot_id != *self.profile.repository_snapshot_id()
            || base_request.repository_snapshot_sha256 != self.profile.repository_snapshot_sha256()
            || base_request.tool_catalog_id != *self.profile.tool_catalog_id()
            || base_request.tool_catalog_sha256 != self.profile.tool_catalog_sha256()
            || base_request.visible_tools != self.profile.visible_tools()
            || base_request.limits != *self.profile.limits()
        {
            return Err(prepare_denied("resume-profile-drift"));
        }
        let policy = build_coding_runtime_policy(CodingRuntimePolicyRequest {
            actor_id: &ActorId::from_raw("coding-development-user"),
            task_id: &base_request.task.task_id,
            run_id: &base_request.run_id,
            workspace: self.workspace.workspace(),
            profile: self.profile,
            excluded_scopes: Vec::new(),
        })
        .map_err(|_| prepare_denied("resume-policy"))?;
        if base_request.policy_id != *policy.policy_id()
            || base_request.policy_sha256 != policy.engine().policy_sha256()
            || checkpoint.policy_id != base_request.policy_id
            || checkpoint.policy_sha256 != base_request.policy_sha256
        {
            return Err(prepare_denied("resume-policy-drift"));
        }
        let mut request = base_request;
        request.event_cursor = Some(RuntimeEventCursor {
            run_id: last.run_id.clone(),
            event_id: last.event_id.clone(),
            sequence: last.sequence,
            event_sha256: last.event_sha256.clone(),
        });
        request =
            seal_runtime_run_request(request).map_err(|_| prepare_denied("resume-request-seal"))?;
        self.session_id = Some(request.session_id.clone());
        self.resume_requested = false;
        self.prepared.insert(
            request.run_id.as_str().to_owned(),
            PreparedDevelopmentRun {
                request: request.clone(),
                policy,
                skip_scripted_steps: usize::try_from(continuation.model_call_count)
                    .map_err(|_| prepare_denied("resume-model-count"))?,
                preauthorization: None,
                continuity: None,
                record_session: false,
            },
        );
        Ok(request)
    }
}

fn build_repository_composition(
    activation: &CodingDevelopmentActivation,
    platform: &'static LinuxDevelopmentPlatformAdapter,
    scenario: CodingDevelopmentScenario,
    model: CodingDevelopmentModel,
) -> Result<
    (
        &'static CodingSessionProfile,
        &'static LinuxCodingWorkspace<'static, 'static>,
        Vec<CodingContextSource>,
    ),
    CodingDevelopmentRuntimeError,
> {
    let workspace_id = WorkspaceId::from_raw(format!(
        "coding-development-{}",
        &activation.marker_sha256()[..24]
    ));
    let selected = select_development_linux_workspace(
        platform,
        activation.workspace_root(),
        workspace_id.clone(),
        WorkspaceAuthorizationId::from_raw("coding-development-inventory"),
    )
    .map_err(|_| CodingDevelopmentRuntimeError::Repository)?;
    let management = activation.state_root().join("repository-management");
    ensure_private_development_directory(&management)
        .map_err(|_| CodingDevelopmentRuntimeError::State)?;
    let scope = LinuxRepositoryScope::verify(
        activation.workspace_root(),
        activation.workspace_root().join(".git"),
        &management,
    )
    .map_err(|_| CodingDevelopmentRuntimeError::Repository)?;
    let collector = LinuxRepositoryCollector::new(
        LinuxGitArtifact::verify("/usr/bin/git")
            .map_err(|_| CodingDevelopmentRuntimeError::Platform)?,
    );
    let inventory = collector
        .collect_inventory(&scope)
        .map_err(|_| CodingDevelopmentRuntimeError::Repository)?;
    if inventory.entries.iter().any(|entry| {
        !matches!(
            entry.state,
            LinuxRepositoryInventoryState::Tracked {
                staged_changed: false,
                conflicted: false,
                ..
            }
        )
    }) {
        return Err(CodingDevelopmentRuntimeError::Repository);
    }
    let policy = LinuxRepositoryMapPolicy::new(
        "coding-development-map-v1",
        vec![
            vec![".git".to_owned()],
            vec![".agentmage-development-workspace".to_owned()],
        ],
        vec![vec!["target".to_owned()]],
        vec![vec!["vendor".to_owned()]],
    )
    .map_err(|_| CodingDevelopmentRuntimeError::Repository)?;
    let repository_map =
        build_development_linux_repository_map(platform, &selected, inventory.clone(), &policy)
            .map_err(|_| CodingDevelopmentRuntimeError::Repository)?;
    let mut supporting_sources =
        build_repository_context_sources(platform, &selected, &repository_map)?;
    drop(selected);
    let profile = Box::leak(Box::new(build_profile(
        activation,
        workspace_id,
        &inventory,
        &repository_map,
        scenario,
        model,
    )?));
    let workspace = Box::leak(Box::new(
        LinuxCodingWorkspace::bind_development(
            platform,
            profile,
            repository_map,
            activation.workspace_root(),
            WorkspaceAuthorizationId::from_raw("coding-development-runtime"),
        )
        .map_err(|_| CodingDevelopmentRuntimeError::Repository)?,
    ));
    // Creation needs the platform's parent identity, not a model-invented tree
    // hash. Observe only the already authorized writable roots. The effect
    // boundary still rebinds the parent/siblings and refuses any later drift.
    supporting_sources.extend(build_creation_parent_sources(
        platform,
        profile,
        workspace.workspace(),
    )?);
    Ok((profile, workspace, supporting_sources))
}

fn build_creation_parent_sources(
    platform: &LinuxDevelopmentPlatformAdapter,
    profile: &CodingSessionProfile,
    workspace: &agentmage_platform_linux::LinuxAuthorizedWorkspace,
) -> Result<Vec<CodingContextSource>, CodingDevelopmentRuntimeError> {
    let mut sources = Vec::new();
    for components in profile.write_scope().writable_roots() {
        let (target, siblings) = if components.is_empty() {
            (
                GrantTarget::held_workspace_root(workspace)
                    .map_err(|_| CodingDevelopmentRuntimeError::Repository)?,
                workspace
                    .observe_root_names(4_096, 1024 * 1024)
                    .map_err(|_| CodingDevelopmentRuntimeError::Repository)?,
            )
        } else {
            let path = WorkspacePath::new(
                profile.write_scope().workspace_id().clone(),
                components.iter().cloned(),
            )
            .map_err(|_| CodingDevelopmentRuntimeError::Repository)?;
            let held = resolve_development_linux_workspace_object(
                platform,
                workspace,
                &path,
                PathResolutionIntent::ReadDirectory,
            )
            .map_err(|_| CodingDevelopmentRuntimeError::Repository)?;
            (
                GrantTarget::held_object(&held)
                    .map_err(|_| CodingDevelopmentRuntimeError::Repository)?,
                held.observe_directory_names(4_096, 1024 * 1024)
                    .map_err(|_| CodingDevelopmentRuntimeError::Repository)?,
            )
        };
        let parent_sha256 = controlled_create_parent_observation_sha256(&target, &siblings)
            .map_err(|_| CodingDevelopmentRuntimeError::Composition)?;
        let content = serde_json::to_string(&serde_json::json!({
            "schema_version": 1, "parent_path": components,
            "observed_sibling_names": siblings,
            "expected_parent_sha256": parent_sha256,
            "use": "Source-bound initial parent observation for controlled create-file. Copy this hash only for a direct child of this parent. It is not authority. A changed parent, sibling set, or existing destination is rejected at the native effect boundary.",
        })).map_err(|_| CodingDevelopmentRuntimeError::Composition)?;
        let digest = sha256(content.as_bytes());
        sources.push(
            CodingContextSource::new(
                format!("creation-parent-{}", &digest[..16]),
                ContextItemKind::Supporting,
                ContextSensitivity::Private,
                ContextAdmission::Eligible,
                true,
                format!("agentmage:creation-parent:{}", components.join("/")),
                parent_sha256,
                digest,
                content,
            )
            .map_err(|_| CodingDevelopmentRuntimeError::Composition)?,
        );
    }
    Ok(sources)
}

fn build_repository_context_sources(
    platform: &LinuxDevelopmentPlatformAdapter,
    workspace: &agentmage_platform_linux::LinuxAuthorizedWorkspace,
    repository_map: &RepositoryMap,
) -> Result<Vec<CodingContextSource>, CodingDevelopmentRuntimeError> {
    const MAX_SOURCE_BYTES: usize = 64 * 1024;
    let mut sources = Vec::new();
    for record in &repository_map.files {
        if !record.content_read
            || record.object_kind != RepositoryObjectKind::RegularFile
            || record.size_bytes == 0
            || record.size_bytes > MAX_SOURCE_BYTES as u64
        {
            continue;
        }
        let held = resolve_development_linux_workspace_object(
            platform,
            workspace,
            &record.path,
            PathResolutionIntent::ReadFile,
        )
        .map_err(|_| CodingDevelopmentRuntimeError::Repository)?;
        let bytes = held
            .read_exact_bytes()
            .map_err(|_| CodingDevelopmentRuntimeError::Repository)?;
        if bytes.len() > MAX_SOURCE_BYTES || sha256(&bytes) != record.content_sha256 {
            return Err(CodingDevelopmentRuntimeError::Repository);
        }
        let Ok(content) = String::from_utf8(bytes) else {
            continue;
        };
        let path = record
            .path
            .components()
            .iter()
            .map(|component| component.as_str())
            .collect::<Vec<_>>()
            .join("/");
        let excerpt = serde_json::to_string(&serde_json::json!({
            "schema_version": 1,
            "untrusted_repository_source": true,
            "path": &path,
            "content_sha256": &record.content_sha256,
            "content": &content,
        }))
        .map_err(|_| CodingDevelopmentRuntimeError::Composition)?;
        sources.push(
            CodingContextSource::new(
                format!("repository-source-{}", &sha256(path.as_bytes())[..16]),
                ContextItemKind::Supporting,
                ContextSensitivity::Private,
                ContextAdmission::Eligible,
                false,
                format!("agentmage:workspace-file:{path}"),
                record.content_sha256.clone(),
                sha256(excerpt.as_bytes()),
                excerpt,
            )
            .map_err(|_| CodingDevelopmentRuntimeError::Composition)?,
        );
    }
    Ok(sources)
}

impl NativeChatRuntimeFactory for CodingDevelopmentRuntimeFactory {
    type Coordinator = CodingDevelopmentCoordinator;

    fn prepare_runtime_request(
        &mut self,
        input: &RuntimePrepareInput,
    ) -> Result<agentmage_kernel_contracts::RuntimeRunRequest, NativeChatRuntimeError> {
        self.activation
            .revalidate()
            .map_err(|_| prepare_denied("activation"))?;
        if input.resume != self.resume_requested
            || self.resume_requested && (input.preauthorization.is_some() || input.record_session)
            || input.profile_id != self.model.profile_id()
            || input.expected_entry_sha256 != self.activation.marker_sha256()
            || input.workspace_id != self.profile.write_scope().workspace_id().as_str()
            || input.workspace_root != self.activation.workspace_root().to_string_lossy()
            || input.prompt.is_empty()
            || input.prompt.len() > 16 * 1024
            || input.slow_subscriber_probe
                && (self.model != CodingDevelopmentModel::Scripted
                    || self.scenario != CodingDevelopmentScenario::SlowCancel)
            || !self.prepared.is_empty()
        {
            return Err(prepare_denied("input-binding"));
        }
        if self.resume_requested {
            return self.prepare_resume_request(input);
        }
        let preparation_time = now_epoch_ms().map_err(|_| prepare_denied("time"))?;
        if let Some(preauthorization) = &input.preauthorization {
            preauthorization
                .verify(&input.workspace_id, preparation_time)
                .map_err(|_| prepare_denied("preauthorization"))?;
        }
        let current_session = self.session_id.clone();
        let existing_session = current_session.is_some();
        let session_id = match (current_session, &input.engineering_session_id) {
            (None, None) => {
                let session_id =
                    SessionId::from_raw(next_development_identity("coding-development-session")?);
                self.session_id = Some(session_id.clone());
                session_id
            }
            (Some(current), Some(requested)) if &current == requested => {
                self.refresh_repository_composition()
                    .map_err(|_| prepare_denied("follow-up-repository"))?;
                current
            }
            _ => return Err(prepare_denied("session-binding")),
        };
        match (&self.preauthorization, &input.preauthorization) {
            (None, Some(contract)) if !existing_session => {
                self.preauthorization =
                    Some(LinuxCodingSessionPreauthorization::new(contract.clone()));
            }
            (Some(active), Some(contract)) if active.matches(contract) => {}
            (None, None) => {}
            _ => return Err(prepare_denied("preauthorization-drift")),
        }
        match self.record_session {
            None if !existing_session => self.record_session = Some(input.record_session),
            Some(active) if active == input.record_session => {}
            _ => return Err(prepare_denied("recording-consent-drift")),
        }
        let continuity = if existing_session && input.record_session {
            let prior_run_id = self
                .prior_run_id
                .as_ref()
                .ok_or_else(|| prepare_denied("continuity-run-absent"))?;
            Some(build_follow_up_continuity(
                &self.activation,
                self.platform,
                self.profile,
                &session_id,
                prior_run_id,
            )?)
        } else {
            None
        };
        let run_id = RuntimeRunId::from_raw(next_development_identity("coding-development-run")?);
        let task_id = TaskId::from_raw(self.profile.worktree().task_id.clone());
        let policy = build_coding_runtime_policy(CodingRuntimePolicyRequest {
            actor_id: &ActorId::from_raw("coding-development-user"),
            task_id: &task_id,
            run_id: &run_id,
            workspace: self.workspace.workspace(),
            profile: self.profile,
            excluded_scopes: Vec::new(),
        })
        .map_err(|_| prepare_denied("policy"))?;
        let (acceptance, required_evidence) = match self.scenario {
            CodingDevelopmentScenario::NoOp
            | CodingDevelopmentScenario::SlowCancel
            | CodingDevelopmentScenario::Overflow
            | CodingDevelopmentScenario::DiskPressure
            | CodingDevelopmentScenario::OutputPressure => (
                vec![
                    "The exact Git status proves the disposable repository is unchanged".to_owned(),
                ],
                vec![EvidenceKind::Observation],
            ),
            CodingDevelopmentScenario::FailedTestRepair
            | CodingDevelopmentScenario::NativeCommandRepair
            | CodingDevelopmentScenario::RestartRepair
            | CodingDevelopmentScenario::ProtocolCorrection
            | CodingDevelopmentScenario::ArgumentsCorrection
            | CodingDevelopmentScenario::RepeatedProtocolRejection
            | CodingDevelopmentScenario::RestartProtocolCorrection
            | CodingDevelopmentScenario::NewFile
            | CodingDevelopmentScenario::MultiFile
            | CodingDevelopmentScenario::Rollback
            | CodingDevelopmentScenario::FalseCompletion => (
                vec!["The exact synthetic validation passes after the bounded repair".to_owned()],
                vec![EvidenceKind::Validation],
            ),
        };
        let mutable_files = match self.scenario {
            CodingDevelopmentScenario::MultiFile => {
                vec!["src/calc.py".to_owned(), "src/subtract.py".to_owned()]
            }
            _ => vec!["src/calc.py".to_owned()],
        };
        let packet = development_work_packet(
            self.profile,
            &task_id,
            &input.prompt,
            &acceptance,
            mutable_files,
            required_evidence,
            self.scenario,
        );
        let request = build_ephemeral_coding_run_request(
            self.profile,
            CodingRunRequestInput {
                run_id,
                session_id,
                objective: input.prompt.clone(),
                acceptance_criteria: acceptance,
                constraints: {
                    let mut constraints = vec![
                        "Disposable synthetic repository only".to_owned(),
                        "No network, publication, package installation, or commit".to_owned(),
                    ];
                    if let Some(preauthorization) = &self.preauthorization {
                        constraints.push(format!(
                            "Direct session preauthorization digest: {}",
                            preauthorization
                                .contract_sha256()
                                .map_err(|_| prepare_denied("preauthorization-state"))?
                        ));
                    }
                    constraints
                },
                work_packet: packet,
                policy_id: policy.policy_id().clone(),
                policy_sha256: policy.engine().policy_sha256().to_owned(),
            },
        )
        .map_err(|error| prepare_denied(error.source_code()))?;
        self.prepared.insert(
            request.run_id.as_str().to_owned(),
            PreparedDevelopmentRun {
                request: request.clone(),
                policy,
                skip_scripted_steps: 0,
                preauthorization: self.preauthorization.clone(),
                continuity,
                record_session: input.record_session,
            },
        );
        self.prior_run_id = Some(request.run_id.clone());
        Ok(request)
    }

    fn compose_runtime(
        &mut self,
        request: &agentmage_kernel_contracts::RuntimeRunRequest,
    ) -> Result<Self::Coordinator, NativeChatRuntimeError> {
        self.activation
            .revalidate()
            .map_err(|_| NativeChatRuntimeError::RequestDenied)?;
        let prepared = self
            .prepared
            .remove(request.run_id.as_str())
            .ok_or(NativeChatRuntimeError::RunUnavailable)?;
        if prepared.request != *request {
            return Err(NativeChatRuntimeError::RequestDenied);
        }
        let stop_after_checkpoint = matches!(
            self.scenario,
            CodingDevelopmentScenario::RestartRepair
                | CodingDevelopmentScenario::RestartProtocolCorrection
        ) && prepared.skip_scripted_steps == 0
            && request.event_cursor.is_none();
        let model = match self.model {
            CodingDevelopmentModel::Scripted => {
                let mut steps = scripted_steps(
                    self.scenario,
                    self.profile,
                    request,
                    self.activation.workspace_root(),
                    self.platform,
                    self.workspace.workspace(),
                )
                .map_err(|_| NativeChatRuntimeError::RequestDenied)?;
                for _ in 0..prepared.skip_scripted_steps {
                    steps
                        .pop_front()
                        .ok_or(NativeChatRuntimeError::RequestDenied)?;
                }
                let delays_ms: VecDeque<u64> = if prepared.skip_scripted_steps > 0 {
                    VecDeque::new()
                } else {
                    match self.scenario {
                        CodingDevelopmentScenario::SlowCancel => [120_000].into_iter().collect(),
                        CodingDevelopmentScenario::RestartRepair => {
                            [0, 120_000].into_iter().collect()
                        }
                        _ => VecDeque::new(),
                    }
                };
                CodingDevelopmentModelPort::Scripted(ScriptedDevelopmentModel {
                    profile: self.profile.model_profile().clone(),
                    steps,
                    calls: 0,
                    delays_ms,
                })
            }
            candidate => load_candidate_model(
                candidate,
                self.profile,
                self.activation.state_root(),
                prepared.record_session,
            )
            .map_err(|_| NativeChatRuntimeError::RuntimeFailed)?,
        };
        let mut context = CodingContextPort::for_profile(
            self.profile,
            self.supporting_sources.clone(),
            DevelopmentTokenCounter::new(
                self.profile.model_profile().context.token_counter.clone(),
            ),
        )
        .map_err(|_| NativeChatRuntimeError::RequestDenied)?;
        if let Some(continuity) = prepared.continuity {
            context = context
                .with_checked_continuity(continuity)
                .map_err(|_| NativeChatRuntimeError::RequestDenied)?;
        }
        let mut key = CodingDevelopmentKeyProvider::open(&self.activation)
            .map_err(|_| NativeChatRuntimeError::RuntimeFailed)?;
        let authority = open_linux_development_authority(
            self.platform,
            self.activation.state_root(),
            &mut key,
            now_epoch_ms().map_err(|_| NativeChatRuntimeError::RuntimeFailed)?,
        )
        .map_err(|_| NativeChatRuntimeError::RuntimeFailed)?;
        let command_manifest = LinuxCommandManifest::verify(
            "/usr/bin/systemd-run",
            "/usr/bin/systemctl",
            "/usr/bin/bwrap",
            self.profile.commands(),
        )
        .map_err(|_| NativeChatRuntimeError::RuntimeFailed)?;
        let command_executor = LinuxBoundedCommandExecutor::new(command_manifest)
            .map_err(|_| NativeChatRuntimeError::RuntimeFailed)?;
        let git_manifest = LinuxRepositoryInspectionManifest::verify(
            "/usr/bin/systemd-run",
            "/usr/bin/systemctl",
            "/usr/bin/bwrap",
            "/usr/bin/git",
        )
        .map_err(|_| NativeChatRuntimeError::RuntimeFailed)?;
        let git_executor = LinuxBoundedRepositoryInspectionExecutor::new(git_manifest)
            .map_err(|_| NativeChatRuntimeError::RuntimeFailed)?;
        let sandbox_manifest =
            LinuxSandboxManifest::verify_development_read_only_worker(self.platform)
                .map_err(|_| NativeChatRuntimeError::RuntimeFailed)?;
        let sandbox = LinuxSandboxRunner::new(sandbox_manifest, LinuxSandboxLimits::default())
            .map_err(|_| NativeChatRuntimeError::RuntimeFailed)?;
        let boundary = LinuxCodingRuntimeBoundary::new(LinuxCodingRuntimeBoundaryInput {
            workspace: self.workspace,
            authority,
            sandbox,
            command_executor,
            git_executor,
            policy: prepared.policy,
            actor_id: ActorId::from_raw("coding-development-user"),
            session_id: request.session_id.clone(),
            sensitivity: DataSensitivity::Operational,
            identities: OsCodingIdentitySource,
            preauthorization: prepared.preauthorization,
            retain_model_exchanges: prepared.record_session,
        })
        .map_err(|error| {
            eprintln!("coding.development.compose.boundary-{error:?}");
            NativeChatRuntimeError::RuntimeFailed
        })?;
        let coordinator = compose_durable_coding_coordinator(
            self.profile,
            request.clone(),
            model,
            context,
            boundary,
            OsRuntimeClock,
        )
        .map_err(|error| {
            eprintln!("coding.development.compose.coordinator-{error:?}");
            NativeChatRuntimeError::RuntimeFailed
        })?;
        Ok(if stop_after_checkpoint {
            coordinator.with_development_checkpoint_stop_probe()
        } else {
            coordinator
        })
    }

    fn revoke_session_preauthorization(
        &mut self,
        session_id: &SessionId,
        preauthorization_sha256: &str,
    ) -> Result<(), NativeChatRuntimeError> {
        if self.session_id.as_ref() != Some(session_id) {
            return Err(NativeChatRuntimeError::RequestDenied);
        }
        self.preauthorization
            .as_ref()
            .ok_or(NativeChatRuntimeError::RequestDenied)?
            .revoke(
                preauthorization_sha256,
                now_epoch_ms().map_err(|_| NativeChatRuntimeError::RuntimeFailed)?,
            )
            .map_err(|_| NativeChatRuntimeError::RequestDenied)
    }
}

fn prepare_denied(stage: &str) -> NativeChatRuntimeError {
    eprintln!("coding.development.prepare.{stage}-denied");
    NativeChatRuntimeError::RequestDenied
}

fn next_development_identity(prefix: &str) -> Result<String, NativeChatRuntimeError> {
    let mut identities = OsCodingIdentitySource;
    identities
        .next(prefix)
        .map_err(|_| prepare_denied("identity"))
}

fn build_follow_up_continuity(
    activation: &CodingDevelopmentActivation,
    platform: &'static LinuxDevelopmentPlatformAdapter,
    profile: &CodingSessionProfile,
    session_id: &SessionId,
    prior_run_id: &RuntimeRunId,
) -> Result<CodingContextContinuityInput, NativeChatRuntimeError> {
    const MAX_CONTINUITY_ARTIFACT_READ_BYTES: u64 = 4 * 1024 * 1024;

    let mut key = CodingDevelopmentKeyProvider::open(activation)
        .map_err(|_| prepare_denied("continuity-key"))?;
    let authority = open_linux_development_authority(
        platform,
        activation.state_root(),
        &mut key,
        now_epoch_ms().map_err(|_| prepare_denied("continuity-time"))?,
    )
    .map_err(|_| prepare_denied("continuity-store"))?;
    let checkpoint = authority
        .authority()
        .current_session_checkpoint()
        .map_err(|_| prepare_denied("continuity-checkpoint"))?
        .ok_or_else(|| prepare_denied("continuity-checkpoint-absent"))?;
    let binding = authority
        .authority()
        .current_runtime_resume_binding()
        .map_err(|_| prepare_denied("continuity-binding"))?
        .ok_or_else(|| prepare_denied("continuity-binding-absent"))?;
    if &binding.run_id != prior_run_id
        || &binding.session_id != session_id
        || binding.checkpoint_id != checkpoint.checkpoint_id
        || binding.checkpoint_sha256 != checkpoint.checkpoint_sha256
    {
        return Err(prepare_denied("continuity-binding-drift"));
    }
    let events = authority
        .authority()
        .runtime_events(prior_run_id)
        .map_err(|_| prepare_denied("continuity-events"))?;
    let Some(last_event) = events.last() else {
        return Err(prepare_denied("continuity-events-absent"));
    };
    let terminal_state = match &last_event.kind {
        RuntimeEventKind::RunTerminal { state, .. } => *state,
        _ => return Err(prepare_denied("continuity-run-nonterminal")),
    };
    if events.iter().any(|event| {
        &event.run_id != prior_run_id
            || &event.session_id != session_id
            || event.task_id != checkpoint.task_id
    }) {
        return Err(prepare_denied("continuity-event-binding"));
    }

    let request_references = binding
        .artifacts
        .iter()
        .filter(|reference| reference.media_type == RUNTIME_REQUEST_MEDIA_TYPE)
        .collect::<Vec<_>>();
    let [request_reference] = request_references.as_slice() else {
        return Err(prepare_denied("continuity-request-reference"));
    };
    let read_artifact = |reference: &agentmage_kernel_contracts::RuntimeArtifactRef| {
        if reference.byte_size > MAX_CONTINUITY_ARTIFACT_READ_BYTES {
            return Err(prepare_denied("continuity-source-overflow"));
        }
        authority
            .read_runtime_artifact(&RuntimeArtifactReadRequest {
                session_id: checkpoint.session_id.clone(),
                task_id: checkpoint.task_id.clone(),
                policy_sha256: checkpoint.policy_sha256.clone(),
                reference: reference.clone(),
                now_epoch_ms: now_epoch_ms().map_err(|_| prepare_denied("continuity-time"))?,
                maximum_bytes: MAX_CONTINUITY_ARTIFACT_READ_BYTES,
            })
            .map_err(|_| prepare_denied("continuity-source-unavailable"))
    };
    let request_bytes = read_artifact(request_reference)?;
    let prior_request: agentmage_kernel_contracts::RuntimeRunRequest =
        serde_json::from_slice(&request_bytes)
            .map_err(|_| prepare_denied("continuity-request-decode"))?;
    agentmage_kernel_engine::runtime_coordinator::verify_runtime_run_request(&prior_request)
        .map_err(|_| prepare_denied("continuity-request-invalid"))?;
    if prior_request.run_id != *prior_run_id
        || prior_request.session_id != *session_id
        || prior_request.workspace_id != *profile.write_scope().workspace_id()
        || prior_request.model_profile != *profile.model_profile()
    {
        return Err(prepare_denied("continuity-request-drift"));
    }

    let mut continuations = Vec::new();
    for reference in binding
        .artifacts
        .iter()
        .filter(|reference| reference.media_type == RUNTIME_CONTINUATION_MEDIA_TYPE)
    {
        let bytes = read_artifact(reference)?;
        let continuation = decode_runtime_continuation_state(&bytes)
            .map_err(|_| prepare_denied("continuity-state-decode"))?;
        if continuation.run_id != *prior_run_id
            || continuation.session_id != *session_id
            || continuation.task_id != checkpoint.task_id
            || continuation.request_sha256 != prior_request.request_sha256
        {
            return Err(prepare_denied("continuity-state-drift"));
        }
        continuations.push((continuation.turn_count, reference, bytes, continuation));
    }
    continuations.sort_by_key(|(turn_count, _, _, _)| *turn_count);
    let Some((_, continuation_reference, continuation_bytes, continuation)) = continuations.last()
    else {
        return Err(prepare_denied("continuity-state-absent"));
    };
    let event_bytes =
        serde_json::to_vec(&events).map_err(|_| prepare_denied("continuity-event-encode"))?;
    let continuation_projection = serde_json::to_vec(&serde_json::json!({
        "schema_version": 1,
        "kind": "checked_runtime_continuation_projection",
        "artifact_id": continuation_reference.artifact_id,
        "manifest_sha256": continuation_reference.manifest_sha256,
        "payload_sha256": continuation_reference.payload_sha256,
        "continuation_sha256": continuation.continuation_sha256,
        "event_cursor": continuation.event_cursor,
        "agent_state": continuation.agent_state,
        "agent_state_revision": continuation.agent_state_revision,
        "turn_count": continuation.turn_count,
        "model_call_count": continuation.model_call_count,
        "tool_call_count": continuation.tool_call_count,
        "context_refresh_count": continuation.context_refresh_count,
        "no_progress_turns": continuation.no_progress_turns,
        "resources": continuation.resources,
        "state_transition_count": continuation.state_transitions.len(),
        "tool_attempt_count": continuation.tool_attempts.len(),
        "tool_result_count": continuation.tool_results.len(),
        "evidence": continuation.evidence,
        "receipt_ids": continuation.receipt_ids,
        "artifacts": continuation.artifacts,
        "verified_payload_byte_size": continuation_bytes.len(),
        "verified_payload_sha256": sha256(continuation_bytes),
    }))
    .map_err(|_| prepare_denied("continuity-state-projection"))?;
    let event_projection = serde_json::to_vec(&serde_json::json!({
        "schema_version": 1,
        "kind": "checked_terminal_event_chain_projection",
        "run_id": prior_run_id,
        "session_id": session_id,
        "event_count": events.len(),
        "first_event_id": events.first().map(|event| &event.event_id),
        "first_event_sha256": events.first().map(|event| event.event_sha256.as_str()),
        "terminal_event_id": last_event.event_id,
        "terminal_event_sha256": last_event.event_sha256,
        "terminal_state": terminal_state,
        "verified_chain_bytes_sha256": sha256(&event_bytes),
        "verified_chain_byte_size": event_bytes.len(),
    }))
    .map_err(|_| prepare_denied("continuity-event-projection"))?;

    let source_material = [
        (
            "request",
            request_reference.artifact_id.as_str(),
            request_reference.manifest_sha256.as_str(),
            request_bytes,
        ),
        (
            "continuation",
            continuation_reference.artifact_id.as_str(),
            continuation_reference.manifest_sha256.as_str(),
            continuation_projection,
        ),
        (
            "events",
            last_event.event_id.as_str(),
            last_event.event_sha256.as_str(),
            event_projection,
        ),
    ];
    let mut sources = Vec::with_capacity(source_material.len());
    for (kind, identity, revision, bytes) in source_material {
        let content_sha256 = sha256(&bytes);
        let content =
            String::from_utf8(bytes).map_err(|_| prepare_denied("continuity-source-encoding"))?;
        sources.push(
            CodingContextSource::new(
                format!("continuity-{kind}-{}", &content_sha256[..16]),
                ContextItemKind::Supporting,
                ContextSensitivity::Private,
                ContextAdmission::Eligible,
                true,
                format!(
                    "agentmage:coding-continuity:{}:{kind}:{identity}",
                    session_id.as_str()
                ),
                revision,
                content_sha256,
                content,
            )
            .map_err(|_| prepare_denied("continuity-source-invalid"))?,
        );
    }
    let mut source_hashes = sources
        .iter()
        .map(|source| source.content_sha256().to_owned())
        .collect::<Vec<_>>();
    source_hashes.sort();
    source_hashes.dedup();
    let source_set_sha256 = checked_continuity_source_set_sha256(&source_hashes);
    let mut identifiers = vec![
        prior_run_id.as_str().to_owned(),
        session_id.as_str().to_owned(),
        profile.write_scope().workspace_id().as_str().to_owned(),
    ];
    identifiers.sort();
    identifiers.dedup();
    let mut evidence_ids = continuation
        .evidence
        .iter()
        .map(|evidence| evidence.evidence_id.clone())
        .collect::<Vec<_>>();
    evidence_ids.sort_by(|left, right| left.as_str().cmp(right.as_str()));
    evidence_ids.dedup();
    let mut receipt_ids = continuation.receipt_ids.clone();
    receipt_ids.sort_by(|left, right| left.as_str().cmp(right.as_str()));
    receipt_ids.dedup();
    let summary = CheckedContextSummary {
        schema_version: CONTRACT_SCHEMA_VERSION,
        summary_id: ContextSummaryId::from_raw(format!(
            "summary-coding-{}",
            &source_set_sha256[..24]
        )),
        state: CheckedSummaryState::Current,
        summary: "Prior same-session work is retained; its exact request and digest-bound checked projections of the verified latest safe continuation and complete terminal event chain are reopened as untrusted sources."
            .to_owned(),
        paths: Vec::new(),
        errors: Vec::new(),
        identifiers,
        commands: Vec::new(),
        decisions: vec![format!("prior_terminal_state={terminal_state:?}")],
        unresolved_questions: vec![
            "Any claim depending on unavailable prior bytes must remain blocked.".to_owned(),
        ],
        evidence_ids,
        citation_ids: Vec::new(),
        receipt_ids,
        source_set_sha256,
    };
    CodingContextContinuityInput::new(
        session_id.clone(),
        profile.write_scope().workspace_id().clone(),
        summary,
        sources,
    )
    .map_err(|_| prepare_denied("continuity-summary-invalid"))
}

fn build_profile(
    activation: &CodingDevelopmentActivation,
    workspace_id: WorkspaceId,
    inventory: &agentmage_platform_linux::LinuxRepositoryInventory,
    repository_map: &agentmage_capability_repository_map::RepositoryMap,
    scenario: CodingDevelopmentScenario,
    model: CodingDevelopmentModel,
) -> Result<CodingSessionProfile, CodingDevelopmentRuntimeError> {
    let limits = runtime_limits(scenario, model);
    let path_sha256 = linux_repository_path_sha256(activation.workspace_root())
        .map_err(|_| CodingDevelopmentRuntimeError::Worktree)?;
    let branch = inventory
        .branch
        .clone()
        .ok_or(CodingDevelopmentRuntimeError::Worktree)?;
    let resource_budget_sha256 =
        development_sha256(&limits).map_err(|_| CodingDevelopmentRuntimeError::Worktree)?;
    let worktree = OwnedWorktreeRecord::seal(OwnedWorktreeRecord {
        schema_version: 0,
        worktree_id: format!(
            "coding-development-worktree-{}",
            &activation.marker_sha256()[..16]
        ),
        task_id: format!(
            "coding-development-task-{}",
            &activation.marker_sha256()[..16]
        ),
        source_object: inventory.commit_id.clone(),
        branch_ref: branch,
        worktree_path_sha256: path_sha256,
        owner_sha256: activation.marker_sha256().to_owned(),
        grant_ids: Vec::new(),
        file_ownership_sha256: repository_map.map_sha256.clone(),
        live_process_count: 0,
        resource_budget_sha256,
        retain_until_epoch_ms: activation
            .marker_epoch_ms()
            .saturating_add(24 * 60 * 60 * 1_000),
        clean: true,
        recovery_retained: false,
        disposition: WorktreeDisposition::Active,
        record_sha256: "0".repeat(64),
    })
    .map_err(|_| CodingDevelopmentRuntimeError::Worktree)?;
    let command = validation_command().map_err(|_| CodingDevelopmentRuntimeError::Validation)?;
    let commands = CommandRegistry::build(vec![command.clone()])
        .map_err(|_| CodingDevelopmentRuntimeError::Validation)?;
    let validation = seal_validation_template(ValidationTemplateInput {
        validation_id: VALIDATION_ID.to_owned(),
        kind: ValidationKind::Unit,
        command,
        source: ValidationTemplateSource::TrustedProjectConfiguration,
        source_sha256: file_sha256(&activation.workspace_root().join("tests/run_validation.py"))?,
        source_path: Some(
            WorkspacePath::new(workspace_id.clone(), ["tests", "run_validation.py"])
                .map_err(|_| CodingDevelopmentRuntimeError::Validation)?,
        ),
        user_input_approval_sha256: None,
        execution_scope_sha256: worktree.record_sha256.clone(),
        parser: ValidationParserKind::AgentMageJsonV1,
        parser_version: "1.0.0".to_owned(),
        parser_sha256: sha256(b"agentmage-json-validation-v1"),
        minimum_test_count: 1,
        focused: true,
        fail_fast: true,
        failed_test_rerun_allowed: true,
        setup_idempotent: true,
        expected_artifacts: Vec::new(),
    })
    .map_err(|_| CodingDevelopmentRuntimeError::Validation)?;
    let validations = ValidationTemplateRegistry::build(vec![validation])
        .map_err(|_| CodingDevelopmentRuntimeError::Validation)?;
    let instruction_ledger = build_instruction_ledger(
        worktree.record_sha256.clone(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        &BTreeMap::new(),
    )
    .map_err(|_| CodingDevelopmentRuntimeError::Profile)?;
    let write_scope = CodingWriteScope::new(workspace_id, vec![vec!["src".to_owned()]])
        .map_err(|_| CodingDevelopmentRuntimeError::Profile)?;
    let change_plan = build_coding_development_plan_binding(repository_map)
        .map_err(|_| CodingDevelopmentRuntimeError::Plan)?;
    CodingSessionProfile::build(CodingSessionProfileInput {
        profile_id: "coding-development-session-profile-v1".to_owned(),
        tool_catalog_id: ToolCatalogId::from_raw("coding-development-native-tools-v1"),
        repository_snapshot_id: RepositorySnapshotId::from_raw(format!(
            "coding-development-snapshot-{}",
            &repository_map.map_sha256[..16]
        )),
        repository_snapshot_sha256: repository_map.map_sha256.clone(),
        immutable_base_commit: inventory.commit_id.clone(),
        worktree,
        model: admitted_development_model(model)?,
        offline_proof: offline_proof(activation)?,
        session_boundary_sha256: digest_bytes(activation.marker_sha256().as_bytes()),
        write_scope,
        change_plan,
        instruction_ledger,
        commands,
        validations,
        limits,
    })
    .map_err(|_| CodingDevelopmentRuntimeError::Profile)
}

fn validation_command() -> Result<CommandSpec, CodingDevelopmentRuntimeError> {
    let executable = fs::canonicalize("/usr/bin/python3")
        .map_err(|_| CodingDevelopmentRuntimeError::Platform)?;
    let executable_text = executable
        .to_str()
        .ok_or(CodingDevelopmentRuntimeError::Platform)?
        .to_owned();
    CommandSpec::seal(
        "fixture.python-validation",
        "1.0.0",
        executable_text,
        file_sha256(&executable)?,
        vec!["tests/run_validation.py".to_owned()],
        CommandWorkingDirectory::OwnedWorktree,
        BTreeMap::from([
            ("LANG".to_owned(), "C.UTF-8".to_owned()),
            ("TZ".to_owned(), "UTC".to_owned()),
        ]),
        CommandRisk::Moderate,
        CommandBounds::new(30_000, 128 * 1024, 128 * 1024, 256 * 1024 * 1024, 16, 100)
            .map_err(|_| CodingDevelopmentRuntimeError::Composition)?,
    )
    .map_err(|_| CodingDevelopmentRuntimeError::Composition)
}

fn scripted_admitted_model() -> Result<
    agentmage_kernel_engine::model_runtime::AdmittedModelProfile,
    CodingDevelopmentRuntimeError,
> {
    let capabilities = [
        ModelRole::Dialogue,
        ModelRole::CodingPlanner,
        ModelRole::ToolSelection,
    ]
    .into_iter()
    .map(|role| ModelCapability {
        role,
        state: ModelCapabilityState::Passed,
        evaluation_profile: Some("scripted-executable-fixture-v1".to_owned()),
        result_sha256: Some(sha256(b"scripted-executable-fixture-v1")),
        limitations: vec![
            "scripted-executable-fixture-only".to_owned(),
            "not-a-qualified-model".to_owned(),
        ],
    })
    .collect();
    let profile = ExactModelProfile {
        schema_version: CONTRACT_SCHEMA_VERSION,
        profile_id: ModelProfileId::from_raw(SCRIPTED_PROFILE_ID),
        manifest_id: ModelManifestId::from_raw("scripted-executable-fixture-manifest-v1"),
        manifest_sha256: sha256(b"scripted-executable-fixture-manifest-v1"),
        display_name: "Scripted executable fixture (not a model)".to_owned(),
        family: "deterministic_fake_coding".to_owned(),
        publisher_control: "AgentMage development fixture".to_owned(),
        lineage: vec!["source-controlled-scripted-fixture-v1".to_owned()],
        license_spdx: "Apache-2.0".to_owned(),
        license_terms_sha256: sha256(include_bytes!("../../../LICENSE")),
        artifact: ModelArtifact {
            artifact_id: "scripted-executable-fixture".to_owned(),
            publisher: "AgentMage development fixture".to_owned(),
            source_revision: "source-tree-bound".to_owned(),
            format: "in-process-scripted-proposals".to_owned(),
            bytes: 1,
            sha256: sha256(b"scripted-executable-fixture"),
        },
        transformations: Vec::new(),
        codec: FamilyCodecIdentity {
            codec_id: ModelCodecId::from_raw("scripted-closed-proposal-v1"),
            codec_version: "1.0.0".to_owned(),
            codec_sha256: sha256(b"scripted-closed-proposal-v1"),
            tokenizer: "bounded-byte-counter-v1".to_owned(),
            tokenizer_sha256: sha256(b"bounded-byte-counter-v1"),
            template: "scripted-runtime-proposal-v1".to_owned(),
            template_sha256: sha256(b"scripted-runtime-proposal-v1"),
            tool_protocol_version: "closed-proposal-v1".to_owned(),
            end_tokens: vec![0],
            reasoning_enabled: false,
        },
        runtime: ModelRuntimeIdentity {
            adapter_id: ModelAdapterId::from_raw("scripted-executable-fixture-adapter-v1"),
            kind: ModelRuntimeKind::DeterministicFake,
            contract_version: 1,
            runtime_build: "agentmage-source-scripted-fixture-v1".to_owned(),
            runtime_sha256: sha256(b"agentmage-source-scripted-fixture-v1"),
            platform: PlatformFamily::DeterministicFake,
            architecture: PlatformArchitecture::X86_64,
        },
        quantization: "not-applicable".to_owned(),
        modalities: vec![ModelModality::Text],
        context: ContextBudget {
            max_context_tokens: 32_768,
            max_input_bytes: 512 * 1024,
            max_messages: 4096,
            token_counter: "bounded-byte-counter-v1".to_owned(),
            token_counter_sha256: sha256(b"bounded-byte-counter-v1"),
        },
        decoding: DecodingProfile {
            profile_id: "scripted-exact-v1".to_owned(),
            sampler_order: vec!["fixture_exact".to_owned()],
            temperature: 0.0,
            top_p: 1.0,
            top_k: 1,
            repeat_penalty: 1.0,
            seed: 1,
            max_output_tokens: 4096,
        },
        hardware: vec![HardwareEnvelope {
            platform: PlatformFamily::DeterministicFake,
            architecture: PlatformArchitecture::X86_64,
            minimum_system_memory_bytes: 1,
            minimum_accelerator_memory_bytes: 0,
            accelerator: "none".to_owned(),
            driver_constraint: "development-fixture-only".to_owned(),
        }],
        capabilities,
        policy_sha256: sha256(b"coding-development-v1"),
        lifecycle: agentmage_kernel_contracts::ModelLifecycleState::Candidate,
        enabled: false,
        automatic_fallback: false,
    };
    ModelAdmissionCatalog::new(vec![profile.clone()])
        .and_then(|catalog| catalog.admit(&profile, ModelUsePurpose::ContractTest))
        .map_err(|_| CodingDevelopmentRuntimeError::Composition)
}

fn admitted_development_model(
    model: CodingDevelopmentModel,
) -> Result<
    agentmage_kernel_engine::model_runtime::AdmittedModelProfile,
    CodingDevelopmentRuntimeError,
> {
    if model == CodingDevelopmentModel::Scripted {
        return scripted_admitted_model();
    }
    let profile = catalog_profile(model.profile_id())?;
    ModelAdmissionCatalog::new(vec![profile.clone()])
        .and_then(|catalog| catalog.admit(&profile, ModelUsePurpose::Evaluation))
        .map_err(|_| CodingDevelopmentRuntimeError::Composition)
}

fn catalog_profile(profile_id: &str) -> Result<ExactModelProfile, CodingDevelopmentRuntimeError> {
    let catalog: serde_json::Value = serde_json::from_str(include_str!(
        "../../../model-profiles/exact-profile-catalog.json"
    ))
    .map_err(|_| CodingDevelopmentRuntimeError::Composition)?;
    let value = catalog
        .get("profiles")
        .and_then(serde_json::Value::as_array)
        .and_then(|profiles| {
            profiles.iter().find(|profile| {
                profile
                    .get("profile_id")
                    .and_then(serde_json::Value::as_str)
                    == Some(profile_id)
            })
        })
        .ok_or(CodingDevelopmentRuntimeError::Composition)?;
    serde_json::from_value(value.clone()).map_err(|_| CodingDevelopmentRuntimeError::Composition)
}

fn offline_proof(
    activation: &CodingDevelopmentActivation,
) -> Result<OfflineProofReceipt, CodingDevelopmentRuntimeError> {
    let endpoint_sha256 = digest_bytes(activation.marker_sha256().as_bytes());
    let policy = StrictLocalNetworkPolicy::new(
        LocalEndpointIdentity::new(
            NetworkComponent::KernelNativeInferenceAdapter,
            LocalTransport::AuthenticatedUnixSocket,
            endpoint_sha256,
        )
        .map_err(|_| CodingDevelopmentRuntimeError::Composition)?,
    );
    let mut ledger = NetworkAttemptLedger::new();
    ledger
        .evaluate_and_record(
            &policy,
            &NetworkObservation {
                component: NetworkComponent::KernelNativeInferenceAdapter,
                destination: NetworkDestinationClass::AuthenticatedLocalSocket,
                transport: Some(LocalTransport::AuthenticatedUnixSocket),
                endpoint_sha256: Some(endpoint_sha256),
                peer_authenticated: true,
                attempted_bytes: 1,
            },
            digest_bytes(b"coding-development-local-ipc"),
            1,
        )
        .map_err(|_| CodingDevelopmentRuntimeError::Composition)?;
    let mut workflow = OfflineProofWorkflow::new(1, 1, 0);
    workflow
        .record_exit(AcquisitionExitDisposition::Completed, 2)
        .map_err(|_| CodingDevelopmentRuntimeError::Composition)?;
    workflow
        .prove(
            &OfflinePreflightObservation {
                observed_at_millis: 3,
                acquisition_process_count: 0,
                acquisition_socket_count: 0,
                external_network_rule_count: 0,
                observed_outbound_bytes: 0,
                observed_dns_attempts: 0,
                staged_artifact_state: StagedArtifactState::ActivatedVerified,
                session_boundary_sha256: endpoint_sha256,
                firewall_policy_sha256: digest_bytes(b"coding-development-no-network"),
            },
            &ledger,
        )
        .map_err(|_| CodingDevelopmentRuntimeError::Composition)
}

fn scripted_steps(
    scenario: CodingDevelopmentScenario,
    profile: &CodingSessionProfile,
    request: &agentmage_kernel_contracts::RuntimeRunRequest,
    workspace_root: &Path,
    platform: &LinuxDevelopmentPlatformAdapter,
    workspace: &agentmage_platform_linux::LinuxAuthorizedWorkspace,
) -> Result<VecDeque<ScriptedDevelopmentStep>, CodingDevelopmentRuntimeError> {
    let git = tool_candidate(
        profile,
        GIT_INSPECTION_TOOL_ID,
        GIT_INSPECTION_TOOL_VERSION,
        "scripted-git-status",
        &GitInspectionRequest {
            schema_version: 1,
            operation: GitInspectionOperation::Status,
            revision: None,
            object_id: None,
            pathspecs: Vec::new(),
            max_records: 32,
            max_output_bytes: 4_096,
        },
    )?;
    let git_diff = tool_candidate(
        profile,
        GIT_INSPECTION_TOOL_ID,
        GIT_INSPECTION_TOOL_VERSION,
        "scripted-git-diff",
        &GitInspectionRequest {
            schema_version: 1,
            operation: GitInspectionOperation::Diff,
            revision: None,
            object_id: None,
            pathspecs: Vec::new(),
            max_records: 1_000,
            max_output_bytes: 256 * 1024,
        },
    )?;
    let completion = |claim, summary: &str, checks_not_run| {
        coding_completion_payload(&CodingCompletionCandidate {
            schema_version: 1,
            objective_sha256: sha256(request.task.objective.as_bytes()),
            terminal_claim: claim,
            summary: summary.to_owned(),
            checks_not_run,
            residual_risks: Vec::new(),
        })
        .map_err(|_| CodingDevelopmentRuntimeError::Composition)
    };
    match scenario {
        CodingDevelopmentScenario::NativeCommandRepair => {
            let mut steps = scripted_steps(
                CodingDevelopmentScenario::FailedTestRepair,
                profile,
                request,
                workspace_root,
                platform,
                workspace,
            )?;
            let command = profile
                .commands()
                .commands()
                .into_iter()
                .next()
                .ok_or(CodingDevelopmentRuntimeError::Composition)?;
            // Value serialization matches the native codec's sorted argument keys,
            // not CommandRequest's internal struct order (campaign7 regression).
            let arguments = serde_json::to_value(CommandRequest::new("attempt-001", command))
                .map_err(|_| CodingDevelopmentRuntimeError::Composition)?;
            steps.push_front(ScriptedDevelopmentStep::Tool(tool_candidate(
                profile,
                crate::coding_tools::BOUNDED_COMMAND_TOOL_ID,
                crate::coding_tools::BOUNDED_COMMAND_TOOL_VERSION,
                "scripted-native-ordered-command",
                &arguments,
            )?));
            Ok(steps)
        }
        CodingDevelopmentScenario::ProtocolCorrection
        | CodingDevelopmentScenario::ArgumentsCorrection
        | CodingDevelopmentScenario::RepeatedProtocolRejection
        | CodingDevelopmentScenario::RestartProtocolCorrection => {
            let mut steps = scripted_steps(
                CodingDevelopmentScenario::FailedTestRepair,
                profile,
                request,
                workspace_root,
                platform,
                workspace,
            )?;
            if scenario == CodingDevelopmentScenario::ArgumentsCorrection {
                steps.push_front(ScriptedDevelopmentStep::Tool(tool_candidate(
                    profile, GIT_INSPECTION_TOOL_ID, GIT_INSPECTION_TOOL_VERSION,
                    "scripted-invalid-git-pathspecs", &serde_json::json!({
                        "schema_version":1,"operation":"status","pathspecs":["src"],
                        "revision":null,"object_id":null,"max_records":100,"max_output_bytes":4194304,
                    }),
                )?));
            } else {
                steps.push_front(ScriptedDevelopmentStep::ProtocolRejected);
                if scenario == CodingDevelopmentScenario::RepeatedProtocolRejection {
                    steps.push_front(ScriptedDevelopmentStep::ProtocolRejected);
                }
            }
            Ok(steps)
        }
        CodingDevelopmentScenario::NoOp
        | CodingDevelopmentScenario::SlowCancel
        | CodingDevelopmentScenario::Overflow
        | CodingDevelopmentScenario::DiskPressure
        | CodingDevelopmentScenario::OutputPressure => Ok([
            ScriptedDevelopmentStep::Tool(git),
            ScriptedDevelopmentStep::Complete(completion(
                CodingTerminalClaim::NoOp,
                "Inspected the disposable fixture; no change was requested.",
                vec!["Mutation-dependent validation was not required.".to_owned()],
            )?),
        ]
        .into_iter()
        .collect()),
        CodingDevelopmentScenario::FailedTestRepair | CodingDevelopmentScenario::RestartRepair => {
            let validation_template = profile
                .validations()
                .templates
                .first()
                .ok_or(CodingDevelopmentRuntimeError::Composition)?;
            let validation = |call_id: &str| {
                tool_candidate(
                    profile,
                    TARGETED_VALIDATION_TOOL_ID,
                    TARGETED_VALIDATION_TOOL_VERSION,
                    call_id,
                    &TargetedValidationRequest {
                        schema_version: 1,
                        validation_attempt_id: call_id.to_owned(),
                        validation_id: validation_template.validation_id.clone(),
                        template_sha256: validation_template.template_sha256.clone(),
                    },
                )
            };
            let source = fs::read(workspace_root.join("src/calc.py"))
                .map_err(|_| CodingDevelopmentRuntimeError::Composition)?;
            let patch = tool_candidate(
                profile,
                STRUCTURED_PATCH_TOOL_ID,
                crate::coding_changes::CONTROLLED_CHANGE_TOOL_VERSION,
                "scripted-repair-patch",
                &StructuredPatchProposal {
                    schema_version: 1,
                    change_id: "scripted-repair-change".to_owned(),
                    path: vec!["src".to_owned(), "calc.py".to_owned()],
                    expected_preimage_sha256: sha256(&source),
                    intent_sha256: profile.change_plan().intent_sha256().to_owned(),
                    change_plan_sha256: profile.change_plan().plan_sha256().to_owned(),
                    language: StructuredLanguage::Python,
                    artifact_class: StructuredArtifactClass::Code,
                    edits: vec![StructuredEdit::RenameIdentifier {
                        edit_id: "scripted-repair-rename".to_owned(),
                        old: "broken_add".to_owned(),
                        replacement: "add".to_owned(),
                    }],
                    additional_review_hooks: Vec::new(),
                    generated: false,
                    allow_generated: false,
                },
            )?;
            Ok([
                ScriptedDevelopmentStep::Tool(validation("scripted-validation-failing")?),
                ScriptedDevelopmentStep::Tool(tool_candidate(
                    profile, ReadOnlyToolKind::ReadText.id(), READ_ONLY_TOOL_VERSION,
                    "scripted-inspect-preimage", &ReadOnlyRequest {
                        schema_version: 1, paths: vec![vec!["src".to_owned(), "calc.py".to_owned()]],
                        query: None, byte_offset: None, byte_count: None,
                        encoding: ReadOnlyEncoding::Utf8, limits: ReadOnlyLimits::default(), call_depth: 0,
                    },
                )?),
                ScriptedDevelopmentStep::Tool(patch),
                ScriptedDevelopmentStep::Tool(validation("scripted-validation-passing")?),
                ScriptedDevelopmentStep::Tool(git_diff),
                ScriptedDevelopmentStep::Tool(git),
                ScriptedDevelopmentStep::Complete(completion(
                    CodingTerminalClaim::Changed,
                    "Observed the failing test, repaired the bounded identifier, and reran it successfully.",
                    Vec::new(),
                )?),
            ]
            .into_iter()
            .collect())
        }
        CodingDevelopmentScenario::NewFile => {
            if workspace_root.join("src/calc.py").exists() {
                return Err(CodingDevelopmentRuntimeError::Composition);
            }
            let parent_path =
                WorkspacePath::new(profile.write_scope().workspace_id().clone(), ["src"])
                    .map_err(|_| CodingDevelopmentRuntimeError::Composition)?;
            let held_parent = resolve_development_linux_workspace_object(
                platform,
                workspace,
                &parent_path,
                PathResolutionIntent::ReadDirectory,
            )
            .map_err(|_| CodingDevelopmentRuntimeError::Composition)?;
            let parent = GrantTarget::held_object(&held_parent)
                .map_err(|_| CodingDevelopmentRuntimeError::Composition)?;
            let siblings = held_parent
                .observe_directory_names(4_096, 1024 * 1024)
                .map_err(|_| CodingDevelopmentRuntimeError::Composition)?;
            let expected_parent_sha256 =
                controlled_create_parent_observation_sha256(&parent, &siblings)
                    .map_err(|_| CodingDevelopmentRuntimeError::Composition)?;
            let create = tool_candidate(
                profile,
                CONTROLLED_CREATE_TOOL_ID,
                CONTROLLED_CHANGE_TOOL_VERSION,
                "scripted-create-calc",
                &ControlledFileCreationProposal {
                    schema_version: 1,
                    creation_id: "scripted-create-calc".to_owned(),
                    path: vec!["src".to_owned(), "calc.py".to_owned()],
                    content: "def add(left, right):\n    return left + right\n".to_owned(),
                    mode: 0o600,
                    classification: ControlledFileClassification::SourceCode,
                    intent_sha256: profile.change_plan().intent_sha256().to_owned(),
                    change_plan_sha256: profile.change_plan().plan_sha256().to_owned(),
                    expected_parent_sha256,
                },
            )?;
            let validation_template = profile
                .validations()
                .templates
                .first()
                .ok_or(CodingDevelopmentRuntimeError::Composition)?;
            let validation = tool_candidate(
                profile,
                TARGETED_VALIDATION_TOOL_ID,
                TARGETED_VALIDATION_TOOL_VERSION,
                "scripted-create-validation",
                &TargetedValidationRequest {
                    schema_version: 1,
                    validation_attempt_id: "scripted-create-validation".to_owned(),
                    validation_id: validation_template.validation_id.clone(),
                    template_sha256: validation_template.template_sha256.clone(),
                },
            )?;
            Ok([
                // Regression for the actual Muse new-file proposal: it must be
                // rejected before grant/effect, retained, then permit a new turn.
                ScriptedDevelopmentStep::Tool(tool_candidate(
                    profile,
                    ReadOnlyToolKind::ReadText.id(),
                    READ_ONLY_TOOL_VERSION,
                    "scripted-read-absent-target",
                    &ReadOnlyRequest {
                        schema_version: 1,
                        paths: vec![vec!["src".to_owned(), "calc.py".to_owned()]],
                        query: None,
                        byte_offset: None,
                        byte_count: None,
                        encoding: ReadOnlyEncoding::Utf8,
                        limits: ReadOnlyLimits::default(),
                        call_depth: 0,
                    },
                )?),
                ScriptedDevelopmentStep::Tool(create),
                ScriptedDevelopmentStep::Tool(validation),
                ScriptedDevelopmentStep::Tool(git_diff),
                ScriptedDevelopmentStep::Tool(git),
                ScriptedDevelopmentStep::Complete(completion(
                    CodingTerminalClaim::Changed,
                    "Created the absent bounded source file and verified it.",
                    Vec::new(),
                )?),
            ]
            .into_iter()
            .collect())
        }
        CodingDevelopmentScenario::MultiFile => {
            let validation_template = profile
                .validations()
                .templates
                .first()
                .ok_or(CodingDevelopmentRuntimeError::Composition)?;
            let validation = |call_id: &str| {
                tool_candidate(
                    profile,
                    TARGETED_VALIDATION_TOOL_ID,
                    TARGETED_VALIDATION_TOOL_VERSION,
                    call_id,
                    &TargetedValidationRequest {
                        schema_version: 1,
                        validation_attempt_id: call_id.to_owned(),
                        validation_id: validation_template.validation_id.clone(),
                        template_sha256: validation_template.template_sha256.clone(),
                    },
                )
            };
            let patch = |call_id: &str, file: &str, old: &str, replacement: &str| {
                let source = fs::read(workspace_root.join("src").join(file))
                    .map_err(|_| CodingDevelopmentRuntimeError::Composition)?;
                tool_candidate(
                    profile,
                    STRUCTURED_PATCH_TOOL_ID,
                    CONTROLLED_CHANGE_TOOL_VERSION,
                    call_id,
                    &StructuredPatchProposal {
                        schema_version: 1,
                        change_id: call_id.to_owned(),
                        path: vec!["src".to_owned(), file.to_owned()],
                        expected_preimage_sha256: sha256(&source),
                        intent_sha256: profile.change_plan().intent_sha256().to_owned(),
                        change_plan_sha256: profile.change_plan().plan_sha256().to_owned(),
                        language: StructuredLanguage::Python,
                        artifact_class: StructuredArtifactClass::Code,
                        edits: vec![StructuredEdit::RenameIdentifier {
                            edit_id: format!("{call_id}-rename"),
                            old: old.to_owned(),
                            replacement: replacement.to_owned(),
                        }],
                        additional_review_hooks: Vec::new(),
                        generated: false,
                        allow_generated: false,
                    },
                )
            };
            Ok([
                ScriptedDevelopmentStep::Tool(validation("scripted-multi-failing")?),
                ScriptedDevelopmentStep::Tool(patch(
                    "scripted-multi-add",
                    "calc.py",
                    "broken_add",
                    "add",
                )?),
                ScriptedDevelopmentStep::Tool(patch(
                    "scripted-multi-subtract",
                    "subtract.py",
                    "broken_subtract",
                    "subtract",
                )?),
                ScriptedDevelopmentStep::Tool(validation("scripted-multi-passing")?),
                ScriptedDevelopmentStep::Tool(git_diff),
                ScriptedDevelopmentStep::Tool(git),
                ScriptedDevelopmentStep::Complete(completion(
                    CodingTerminalClaim::Changed,
                    "Repaired two bounded source files and verified the complete fixture.",
                    Vec::new(),
                )?),
            ]
            .into_iter()
            .collect())
        }
        CodingDevelopmentScenario::Rollback => {
            let validation_template = profile
                .validations()
                .templates
                .first()
                .ok_or(CodingDevelopmentRuntimeError::Composition)?;
            let source = fs::read(workspace_root.join("src/calc.py"))
                .map_err(|_| CodingDevelopmentRuntimeError::Composition)?;
            let patch = tool_candidate(
                profile,
                STRUCTURED_PATCH_TOOL_ID,
                CONTROLLED_CHANGE_TOOL_VERSION,
                "scripted-rollback-patch",
                &StructuredPatchProposal {
                    schema_version: 1,
                    change_id: "scripted-rollback-change".to_owned(),
                    path: vec!["src".to_owned(), "calc.py".to_owned()],
                    expected_preimage_sha256: sha256(&source),
                    intent_sha256: profile.change_plan().intent_sha256().to_owned(),
                    change_plan_sha256: profile.change_plan().plan_sha256().to_owned(),
                    language: StructuredLanguage::Python,
                    artifact_class: StructuredArtifactClass::Code,
                    edits: vec![StructuredEdit::RenameIdentifier {
                        edit_id: "scripted-temporary-rename".to_owned(),
                        old: "add".to_owned(),
                        replacement: "temporary_add".to_owned(),
                    }],
                    additional_review_hooks: Vec::new(),
                    generated: false,
                    allow_generated: false,
                },
            )?;
            let history = tool_candidate(
                profile,
                CHANGE_HISTORY_TOOL_ID,
                CODING_HISTORY_TOOL_VERSION,
                "scripted-change-history",
                &ChangeHistoryRequest {
                    schema_version: 1,
                    max_records: 1,
                },
            )?;
            let rollback_definition = profile
                .registry()
                .get_tool(
                    &ToolId::from_raw(ROLLBACK_TOOL_ID),
                    CODING_HISTORY_TOOL_VERSION,
                )
                .ok_or(CodingDevelopmentRuntimeError::Composition)?
                .clone();
            let validation = tool_candidate(
                profile,
                TARGETED_VALIDATION_TOOL_ID,
                TARGETED_VALIDATION_TOOL_VERSION,
                "scripted-rollback-validation",
                &TargetedValidationRequest {
                    schema_version: 1,
                    validation_attempt_id: "scripted-rollback-validation".to_owned(),
                    validation_id: validation_template.validation_id.clone(),
                    template_sha256: validation_template.template_sha256.clone(),
                },
            )?;
            Ok([
                ScriptedDevelopmentStep::Tool(patch),
                ScriptedDevelopmentStep::Tool(history),
                ScriptedDevelopmentStep::RollbackFromHistory {
                    definition: rollback_definition,
                    intent_sha256: profile.change_plan().intent_sha256().to_owned(),
                    change_plan_sha256: profile.change_plan().plan_sha256().to_owned(),
                },
                ScriptedDevelopmentStep::Tool(validation),
                ScriptedDevelopmentStep::Tool(git_diff),
                ScriptedDevelopmentStep::Tool(git),
                ScriptedDevelopmentStep::Complete(completion(
                    CodingTerminalClaim::Changed,
                    "Applied one bounded change, inspected its retained history, and restored the exact preimage through a fresh controlled write.",
                    Vec::new(),
                )?),
            ]
            .into_iter()
            .collect())
        }
        CodingDevelopmentScenario::FalseCompletion => {
            Ok([ScriptedDevelopmentStep::Complete(completion(
                CodingTerminalClaim::Changed,
                "Unsupported completion claim without tools or validation.",
                Vec::new(),
            )?)]
            .into_iter()
            .collect())
        }
    }
}

fn tool_candidate<T: serde::Serialize>(
    profile: &CodingSessionProfile,
    tool_id: &str,
    tool_version: &str,
    call_id: &str,
    value: &T,
) -> Result<ModelToolCallCandidate, CodingDevelopmentRuntimeError> {
    let definition = profile
        .registry()
        .get_tool(&ToolId::from_raw(tool_id), tool_version)
        .ok_or(CodingDevelopmentRuntimeError::Composition)?;
    let bytes =
        serde_json::to_vec(value).map_err(|_| CodingDevelopmentRuntimeError::Composition)?;
    Ok(ModelToolCallCandidate {
        tool_call_id: ToolCallId::from_raw(call_id),
        tool_id: definition.tool_id.clone(),
        tool_version: definition.tool_version.clone(),
        arguments: ContractPayload {
            schema: definition.input_schema.clone(),
            media_type: "application/json".to_owned(),
            sha256: sha256(&bytes),
            bytes,
        },
    })
}

fn development_work_packet(
    profile: &CodingSessionProfile,
    task_id: &TaskId,
    objective: &str,
    acceptance: &[String],
    mutable_files: Vec<String>,
    required_evidence: Vec<EvidenceKind>,
    scenario: CodingDevelopmentScenario,
) -> WorkPacket {
    let native_candidate = profile.model_profile().runtime.kind == ModelRuntimeKind::NativeLlamaCpp;
    WorkPacket {
        schema_version: CONTRACT_SCHEMA_VERSION,
        work_packet_id: WorkPacketId::from_raw(format!("packet-{}", task_id.as_str())),
        task_id: task_id.clone(),
        revision: 1,
        objective: objective.to_owned(),
        reason: "Exercise the real coordinator in an explicitly disposable fixture".to_owned(),
        owner: "local-user".to_owned(),
        authoritative_evidence: Vec::new(),
        mutable_files,
        protected_files: vec![".git".to_owned(), "tests/run_validation.py".to_owned()],
        expected_output: "One verifier-backed coding outcome".to_owned(),
        acceptance_checks: acceptance.to_vec(),
        required_evidence,
        required_capability_class: AuthorityClass::LocalWrite,
        budgets: [
            (BudgetResource::PlanSteps, 16),
            (BudgetResource::ToolCallDepth, 2),
            (BudgetResource::ModelCalls, 16),
            (BudgetResource::ToolCalls, 16),
            (BudgetResource::InputBytes, 8 * 1024 * 1024),
            (
                BudgetResource::OutputBytes,
                if scenario == CodingDevelopmentScenario::OutputPressure {
                    1_024
                } else {
                    4 * 1024 * 1024
                },
            ),
            (
                BudgetResource::ElapsedMilliseconds,
                if native_candidate {
                    45 * 60 * 1_000
                } else {
                    600_000
                },
            ),
            (
                BudgetResource::MemoryBytes,
                if native_candidate {
                    64 * 1024 * 1024 * 1024
                } else {
                    256 * 1024 * 1024
                },
            ),
            (
                BudgetResource::DiskBytes,
                if scenario == CodingDevelopmentScenario::DiskPressure {
                    1_024
                } else {
                    64 * 1024 * 1024
                },
            ),
            (BudgetResource::ProcessCount, 32),
        ]
        .into_iter()
        .map(|(resource, limit)| BudgetLimit { resource, limit })
        .collect(),
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
            description: "Restore only exact preimage bytes through a fresh grant".to_owned(),
        },
        sensitivity: DataSensitivity::Ephemeral,
        last_verification_date: "2026-09-21".to_owned(),
        next_action: None,
        next_review: None,
        status_reason: None,
        disposition: None,
        completion_evidence: Vec::new(),
        superseding_work: None,
        validation_issues: Vec::new(),
        plan_id: Some(PlanId::from_raw(profile.change_plan().plan_id().to_owned())),
        state: WorkPacketState::Active,
    }
}

enum ScriptedDevelopmentStep {
    ProtocolRejected,
    Tool(ModelToolCallCandidate),
    RollbackFromHistory {
        definition: ToolDefinition,
        intent_sha256: String,
        change_plan_sha256: String,
    },
    Complete(ContractPayload),
}

/// Explicit scripted proposal source; it is never exposed as a qualified local model.
pub struct ScriptedDevelopmentModel {
    profile: ExactModelProfile,
    steps: VecDeque<ScriptedDevelopmentStep>,
    calls: u32,
    delays_ms: VecDeque<u64>,
}

type NativeDevelopmentRuntime = LinuxNativeModelAdapter<LlamaServerDriver>;
type MuseDevelopmentController =
    LocalModelController<NativeDevelopmentRuntime, MuseAtemFamilyCodec>;
type GptOssDevelopmentController =
    LocalModelController<NativeDevelopmentRuntime, GptOssHarmonyFamilyCodec>;

/// Exact development proposal port. Candidate variants remain evaluation-only.
pub enum CodingDevelopmentModelPort {
    /// Deterministic non-model executable fixture.
    Scripted(ScriptedDevelopmentModel),
    /// Native Muse ATEM candidate controller.
    Muse {
        /// Exact admitted controller.
        controller: MuseDevelopmentController,
        /// Private raw rejection retention root.
        rejection_root: PathBuf,
        /// Explicit user consent to retain exact native model exchanges.
        record_session: bool,
    },
    /// Native GPT-OSS Harmony candidate controller.
    GptOss {
        /// Exact admitted controller.
        controller: GptOssDevelopmentController,
        /// Private raw rejection retention root.
        rejection_root: PathBuf,
        /// Explicit user consent to retain exact native model exchanges.
        record_session: bool,
    },
}

impl RuntimeModelPort for CodingDevelopmentModelPort {
    fn exact_profile(&self) -> &ExactModelProfile {
        match self {
            Self::Scripted(model) => model.exact_profile(),
            Self::Muse { controller, .. } => controller.exact_profile(),
            Self::GptOss { controller, .. } => controller.exact_profile(),
        }
    }

    fn bind_context_tokens(
        &self,
        packet: &mut agentmage_kernel_contracts::ModelContextPacket,
    ) -> Result<(), RuntimePortFailure> {
        let result = match self {
            Self::Scripted(_) => return Ok(()),
            Self::Muse { controller, .. } => controller.bind_dispatch_context(packet),
            Self::GptOss { controller, .. } => controller.bind_dispatch_context(packet),
        };
        result.map(|_| ()).map_err(|error| {
            if error == agentmage_kernel_engine::model_runtime::ModelRuntimeGateError::DispatchCapacityExceeded {
                RuntimePortFailure::ResourceExhausted
            } else {
                RuntimePortFailure::Invalid
            }
        })
    }

    fn run_model(
        &mut self,
        request: &ModelRunRequest,
        context: &agentmage_kernel_contracts::ModelContextPacket,
        cancellation: Option<&dyn ModelCancellationProbe>,
    ) -> Result<ModelRunResult, RuntimePortFailure> {
        match self {
            Self::Scripted(model) => model.run_model(request, context, cancellation),
            Self::Muse {
                controller,
                rejection_root,
                record_session,
            } => run_native_candidate(
                controller,
                rejection_root,
                *record_session,
                request,
                context,
                cancellation,
            ),
            Self::GptOss {
                controller,
                rejection_root,
                record_session,
            } => run_native_candidate(
                controller,
                rejection_root,
                *record_session,
                request,
                context,
                cancellation,
            ),
        }
    }
}

fn run_native_candidate<R, C>(
    controller: &mut LocalModelController<R, C>,
    rejection_root: &Path,
    record_session: bool,
    request: &ModelRunRequest,
    context: &agentmage_kernel_contracts::ModelContextPacket,
    cancellation: Option<&dyn ModelCancellationProbe>,
) -> Result<ModelRunResult, RuntimePortFailure>
where
    R: agentmage_kernel_contracts::LocalModelRuntime,
    C: agentmage_kernel_contracts::ModelFamilyCodec,
{
    // The context owner has already selected sources against exact native rendering.
    // Rebind independently at dispatch; no approximate count reaches llama.cpp.
    let mut exact_context = context.clone();
    controller
        .bind_token_count(&mut exact_context)
        .map_err(|error| {
            eprintln!(
                "coding.development.candidate.context-binding.{}",
                error.code()
            );
            RuntimePortFailure::Invalid
        })?;
    let prepared = controller
        .prepare(request, &exact_context)
        .map_err(|error| {
            eprintln!(
                "coding.development.candidate.preflight.{} rendered_tokens={} output_reserve={} profile_capacity={}",
                error.code(), exact_context.input_tokens, request.max_output_tokens,
                controller.exact_profile().context.max_context_tokens,
            );
            RuntimePortFailure::Invalid
        })?;
    if record_session {
        let metadata = serde_json::to_vec(&serde_json::json!({
            "schema_version": 1, "kind": "rendered-prompt", "request": request,
            "profile": controller.exact_profile(), "preflight": prepared.preflight(),
        }))
        .map_err(|_| RuntimePortFailure::Invalid)?;
        let identity = sha256(format!("prompt:{}", request.model_run_id.as_str()).as_bytes());
        retain_rejected_development_output(
            rejection_root,
            &identity[..24],
            prepared.rendered_context(),
            &metadata,
        )
        .map_err(|_| RuntimePortFailure::Unavailable)?;
    }
    let output = match controller.dispatch_with_output(prepared, cancellation) {
        Ok(output) => output,
        Err(error) => {
            eprintln!("coding.development.candidate.dispatch.{}", error.code());
            let Some(mut rejected) = controller.take_rejected_output() else {
                return Err(RuntimePortFailure::Unavailable);
            };
            retain_rejected_candidate(rejection_root, &rejected).map_err(|retention_error| {
                eprintln!("coding.development.candidate.rejection-retention.{retention_error}");
                RuntimePortFailure::Unavailable
            })?;
            eprintln!(
                "coding.development.candidate.rejection-retained:codec={}:sha256={}:bytes={}",
                rejected.codec_failure_code,
                rejected.response_sha256,
                rejected.response_bytes.len()
            );
            if error
                != agentmage_kernel_engine::model_runtime::ModelRuntimeGateError::ProposalInvalid
                || !correctable_native_protocol_code(&rejected.codec_failure_code)
                || !rejected.result.finish_reason.is_complete()
                || rejected.result.proposal.is_some()
                || rejected.result.terminal_state != ModelRunTerminalState::Rejected
                || rejected.result.response_sha256 != rejected.response_sha256
                || rejected.result.failure.as_ref().is_none_or(|failure| {
                    failure.code != rejected.codec_failure_code
                        || failure.dependency_recovery_required
                        || failure.contract_error.is_some()
                })
            {
                return Err(RuntimePortFailure::Unavailable);
            }
            rejected.result.failure = Some(agentmage_kernel_contracts::ModelRuntimeFailure {
                code: "runtime.model.protocol_rejected".to_owned(),
                retryable_after_correction: true,
                dependency_recovery_required: false,
                contract_error: None,
            });
            agentmage_kernel_engine::model_runtime::VerifiedModelOutput {
                result: rejected.result,
                response_bytes: rejected.response_bytes,
            }
        }
    };
    if record_session {
        let metadata = serde_json::to_vec(&serde_json::json!({
            "schema_version": 1, "kind": "model-response", "result": &output.result,
            "canonical_result": serde_json::to_string(&output.result).map_err(|_| RuntimePortFailure::Invalid)?,
            "result_sha256": runtime_sha256(&output.result)?,
        }))
        .map_err(|_| RuntimePortFailure::Invalid)?;
        let identity = sha256(format!("response:{}", request.model_run_id.as_str()).as_bytes());
        retain_rejected_development_output(
            rejection_root,
            &identity[..24],
            &output.response_bytes,
            &metadata,
        )
        .map_err(|_| RuntimePortFailure::Unavailable)?;
    }
    Ok(output.result)
}

fn correctable_native_protocol_code(code: &str) -> bool {
    matches!(
        code,
        "model.muse-codec.native-channel-invalid"
            | "model.muse-codec.native-json-invalid"
            | "model.muse-codec.final-channel-invalid"
            | "model.gpt-oss-codec.tool-channel-invalid"
            | "model.gpt-oss-codec.tool-arguments-invalid"
            | "model.gpt-oss-codec.final-channel-invalid"
    )
}

fn retain_rejected_candidate(
    rejection_root: &Path,
    rejected: &RejectedModelOutput,
) -> Result<(), &'static str> {
    let identity = sha256(rejected.model_run_id.as_str().as_bytes());
    let metadata = serde_json::to_vec(&serde_json::json!({
        "schema_version": 1,
        "model_run_id": rejected.model_run_id.as_str(),
        "response_sha256": rejected.response_sha256,
        "response_bytes": rejected.response_bytes.len(),
        "codec_failure_code": rejected.codec_failure_code,
        "validated_result": rejected.result,
        "disposition": "untrusted-codec-rejected-no-authority"
    }))
    .map_err(|_| "metadata-invalid")?;
    retain_rejected_development_output(
        rejection_root,
        &identity[..24],
        &rejected.response_bytes,
        &metadata,
    )
    .map_err(|_| "retention-failed")
}

fn load_candidate_model(
    model: CodingDevelopmentModel,
    session_profile: &CodingSessionProfile,
    state_root: &Path,
    record_session: bool,
) -> Result<CodingDevelopmentModelPort, CodingDevelopmentRuntimeError> {
    let expected_profile = session_profile.model_profile();
    if !model.is_candidate() || expected_profile.profile_id.as_str() != model.profile_id() {
        eprintln!("coding.development.candidate.profile-binding-denied");
        return Err(CodingDevelopmentRuntimeError::Profile);
    }
    let admitted = admitted_development_model(model).inspect_err(|_| {
        eprintln!("coding.development.candidate.evaluation-admission-denied");
    })?;
    if admitted.exact_profile() != expected_profile {
        eprintln!("coding.development.candidate.profile-substitution-denied");
        return Err(CodingDevelopmentRuntimeError::Profile);
    }
    let (runtime_root, model_path, kwargs) =
        candidate_paths(model, expected_profile).inspect_err(|_| {
            eprintln!("coding.development.candidate.preparation-tuple-denied");
        })?;
    let socket_root = state_root.join(match model {
        CodingDevelopmentModel::Muse => "candidate-muse",
        CodingDevelopmentModel::GptOss => "candidate-gpt-oss",
        CodingDevelopmentModel::Scripted => return Err(CodingDevelopmentRuntimeError::Profile),
    });
    ensure_private_development_directory(&socket_root).map_err(|_| {
        eprintln!("coding.development.candidate.socket-root-denied");
        CodingDevelopmentRuntimeError::State
    })?;
    let rejection_parent = state_root.join("candidate-rejections");
    ensure_private_development_directory(&rejection_parent).map_err(|_| {
        eprintln!("coding.development.candidate.rejection-root-denied");
        CodingDevelopmentRuntimeError::State
    })?;
    let rejection_root = rejection_parent.join(match model {
        CodingDevelopmentModel::Muse => "muse",
        CodingDevelopmentModel::GptOss => "gpt-oss",
        CodingDevelopmentModel::Scripted => return Err(CodingDevelopmentRuntimeError::Profile),
    });
    ensure_private_development_directory(&rejection_root).map_err(|_| {
        eprintln!("coding.development.candidate.rejection-root-denied");
        CodingDevelopmentRuntimeError::State
    })?;
    let launch = LlamaServerLaunchProfile::new(32_768, 4, 256, 128, "q8_0", "q8_0", Some(kwargs))
        .map_err(|error| {
        eprintln!("coding.development.candidate.launch-profile.{}", error.code);
        CodingDevelopmentRuntimeError::Profile
    })?;
    let driver = LlamaServerDriver::new(
        LlamaServerDriverConfig::new(
            runtime_root,
            model_path,
            socket_root.join("llama-server.sock"),
            expected_profile.runtime.clone(),
            Duration::from_secs(600),
        )
        .map_err(|error| {
            eprintln!("coding.development.candidate.driver-config.{}", error.code);
            CodingDevelopmentRuntimeError::Profile
        })?
        .with_launch_profile(launch),
    );
    let isolation = RuntimeIsolationObservation {
        adapter_id: expected_profile.runtime.adapter_id.clone(),
        profile_id: expected_profile.profile_id.clone(),
        network_available: false,
        workspace_available: false,
        authority_material_available: false,
        credential_material_available: false,
        observation_sha256: sha256(
            format!(
                "coding-development-native-isolation-v1:{}",
                expected_profile.profile_id.as_str()
            )
            .as_bytes(),
        ),
    };
    let runtime = LinuxNativeModelAdapter::new(expected_profile.runtime.clone(), isolation, driver)
        .map_err(|error| {
            eprintln!("coding.development.candidate.adapter.{}", error.code);
            CodingDevelopmentRuntimeError::Platform
        })?;
    match model {
        CodingDevelopmentModel::Muse => {
            let schemas =
                crate::coding_tools::model_visible_coding_tools(session_profile.registry())
                    .map_err(|_| CodingDevelopmentRuntimeError::Profile)?
                    .into_iter()
                    .map(|tool| (tool.definition, tool.input_schema))
                    .collect();
            let codec = MuseAtemFamilyCodec::new(expected_profile.codec.clone())
                .and_then(|codec| {
                    codec
                        .with_native_contracts(
                            session_profile
                                .registry()
                                .list_tools()
                                .into_iter()
                                .cloned()
                                .collect(),
                            crate::coding_verifier::coding_completion_schema(),
                        )?
                        .with_native_parameter_schemas(schemas)
                })
                .map_err(|error| {
                    eprintln!("coding.development.candidate.muse-codec.{}", error.code);
                    CodingDevelopmentRuntimeError::Profile
                })?;
            let mut controller =
                LocalModelController::new(runtime, codec, admitted, 256).map_err(|error| {
                    eprintln!("coding.development.candidate.controller.{}", error.code());
                    CodingDevelopmentRuntimeError::Profile
                })?;
            load_and_verify_candidate(&mut controller)?;
            Ok(CodingDevelopmentModelPort::Muse {
                controller,
                rejection_root,
                record_session,
            })
        }
        CodingDevelopmentModel::GptOss => {
            let schemas =
                crate::coding_tools::model_visible_coding_tools(session_profile.registry())
                    .map_err(|_| CodingDevelopmentRuntimeError::Profile)?
                    .into_iter()
                    .map(|tool| (tool.definition, tool.input_schema))
                    .collect();
            let codec = GptOssHarmonyFamilyCodec::new(expected_profile.codec.clone())
                .and_then(|codec| {
                    codec
                        .with_native_tools(
                            session_profile
                                .registry()
                                .list_tools()
                                .into_iter()
                                .cloned()
                                .collect(),
                        )?
                        .with_native_parameter_schemas(schemas)?
                        .with_completion_schema(crate::coding_verifier::coding_completion_schema())
                })
                .map_err(|error| {
                    eprintln!("coding.development.candidate.gpt-oss-codec.{}", error.code);
                    CodingDevelopmentRuntimeError::Profile
                })?;
            let mut controller =
                LocalModelController::new(runtime, codec, admitted, 256).map_err(|error| {
                    eprintln!("coding.development.candidate.controller.{}", error.code());
                    CodingDevelopmentRuntimeError::Profile
                })?;
            load_and_verify_candidate(&mut controller)?;
            Ok(CodingDevelopmentModelPort::GptOss {
                controller,
                rejection_root,
                record_session,
            })
        }
        CodingDevelopmentModel::Scripted => Err(CodingDevelopmentRuntimeError::Profile),
    }
}

fn load_and_verify_candidate<R, C>(
    controller: &mut LocalModelController<R, C>,
) -> Result<(), CodingDevelopmentRuntimeError>
where
    R: agentmage_kernel_contracts::LocalModelRuntime,
    C: agentmage_kernel_contracts::ModelFamilyCodec,
{
    controller.load().map_err(|error| {
        eprintln!("coding.development.candidate.load.{}", error.code());
        CodingDevelopmentRuntimeError::Platform
    })?;
    let served = controller.serving_capabilities().map_err(|error| {
        eprintln!("coding.development.candidate.capabilities.{}", error.code());
        CodingDevelopmentRuntimeError::Platform
    })?;
    if served.context_capacity_tokens != 32_768 || served.parallel_slots != 1 {
        eprintln!("coding.development.candidate.served-profile-denied");
        return Err(CodingDevelopmentRuntimeError::Profile);
    }
    Ok(())
}

fn candidate_paths(
    model: CodingDevelopmentModel,
    profile: &ExactModelProfile,
) -> Result<(PathBuf, PathBuf, String), CodingDevelopmentRuntimeError> {
    let lab: serde_json::Value = serde_json::from_str(include_str!(
        "../../../model-profiles/development/coding-model-lab.json"
    ))
    .map_err(|_| CodingDevelopmentRuntimeError::Profile)?;
    let runtime: serde_json::Value = serde_json::from_str(include_str!("../../../demo/model.json"))
        .map_err(|_| CodingDevelopmentRuntimeError::Profile)?;
    if lab.get("scope").and_then(serde_json::Value::as_str)
        != Some("synthetic-development-preparation-only")
        || lab
            .get("product_enabled")
            .and_then(serde_json::Value::as_bool)
            != Some(false)
        || lab
            .get("context_tokens")
            .and_then(serde_json::Value::as_u64)
            != Some(32_768)
        || lab
            .get("parallel_slots")
            .and_then(serde_json::Value::as_u64)
            != Some(1)
    {
        return Err(CodingDevelopmentRuntimeError::Profile);
    }
    let label = match model {
        CodingDevelopmentModel::Muse => "muse",
        CodingDevelopmentModel::GptOss => "gpt-oss",
        CodingDevelopmentModel::Scripted => return Err(CodingDevelopmentRuntimeError::Profile),
    };
    let candidate = lab
        .pointer(&format!("/models/{label}"))
        .ok_or(CodingDevelopmentRuntimeError::Profile)?;
    if candidate.get("sha256").and_then(serde_json::Value::as_str)
        != Some(profile.artifact.sha256.as_str())
        || candidate
            .get("size_bytes")
            .and_then(serde_json::Value::as_u64)
            != Some(profile.artifact.bytes)
        || candidate
            .get("coding_qualification")
            .and_then(serde_json::Value::as_str)
            != Some("not-qualified")
    {
        return Err(CodingDevelopmentRuntimeError::Profile);
    }
    let runtime_root = runtime
        .get("runtime_root")
        .and_then(serde_json::Value::as_str)
        .map(PathBuf::from)
        .ok_or(CodingDevelopmentRuntimeError::Profile)?;
    let model_path = expand_preparation_path(
        candidate
            .get("path")
            .and_then(serde_json::Value::as_str)
            .ok_or(CodingDevelopmentRuntimeError::Profile)?,
    )?;
    let kwargs = serde_json::to_string(
        candidate
            .get("chat_template_kwargs")
            .ok_or(CodingDevelopmentRuntimeError::Profile)?,
    )
    .map_err(|_| CodingDevelopmentRuntimeError::Profile)?;
    Ok((runtime_root, model_path, kwargs))
}

fn expand_preparation_path(value: &str) -> Result<PathBuf, CodingDevelopmentRuntimeError> {
    if let Some(relative) = value.strip_prefix("~/") {
        let home = std::env::var_os("HOME")
            .map(PathBuf::from)
            .ok_or(CodingDevelopmentRuntimeError::State)?;
        let canonical = home
            .canonicalize()
            .map_err(|_| CodingDevelopmentRuntimeError::State)?;
        let metadata =
            fs::symlink_metadata(&canonical).map_err(|_| CodingDevelopmentRuntimeError::State)?;
        if !canonical.is_absolute()
            || !metadata.is_dir()
            || metadata.uid() != rustix::process::getuid().as_raw()
            || metadata.mode() & 0o077 != 0
            || relative.split('/').any(|part| part == "..")
        {
            return Err(CodingDevelopmentRuntimeError::State);
        }
        return Ok(canonical.join(relative));
    }
    let path = PathBuf::from(value);
    if !path.is_absolute() {
        return Err(CodingDevelopmentRuntimeError::State);
    }
    Ok(path)
}

impl RuntimeModelPort for ScriptedDevelopmentModel {
    fn exact_profile(&self) -> &ExactModelProfile {
        &self.profile
    }

    fn run_model(
        &mut self,
        request: &ModelRunRequest,
        context: &agentmage_kernel_contracts::ModelContextPacket,
        cancellation: Option<&dyn ModelCancellationProbe>,
    ) -> Result<ModelRunResult, RuntimePortFailure> {
        if let Some(delay_ms) = self.delays_ms.pop_front().filter(|delay| *delay > 0) {
            let mut remaining = delay_ms;
            while remaining > 0 {
                if cancellation
                    .map(|probe| probe.observe())
                    .transpose()
                    .map_err(|_| RuntimePortFailure::Invalid)?
                    .flatten()
                    .is_some()
                {
                    return Ok(cancelled_model_result(
                        request,
                        context.input_tokens,
                        delay_ms,
                    ));
                }
                let slice = remaining.min(10);
                std::thread::sleep(Duration::from_millis(slice));
                remaining -= slice;
            }
        }
        if cancellation
            .map(|probe| probe.observe())
            .transpose()
            .map_err(|_| RuntimePortFailure::Invalid)?
            .flatten()
            .is_some()
            || !context
                .messages
                .iter()
                .any(|message| message.role == ModelMessageRole::System)
        {
            return Ok(cancelled_model_result(request, context.input_tokens, 1));
        }
        let step = self
            .steps
            .pop_front()
            .ok_or(RuntimePortFailure::ResourceExhausted)?;
        self.calls = self
            .calls
            .checked_add(1)
            .ok_or(RuntimePortFailure::ResourceExhausted)?;
        let (kind, payload, tool_call) = match step {
            ScriptedDevelopmentStep::ProtocolRejected => {
                let mut result = cancelled_model_result(request, context.input_tokens, 1);
                result.terminal_state = ModelRunTerminalState::Rejected;
                result.finish_reason = ModelFinishReason::EndOfSequence;
                result.response_sha256 = sha256(b"explicit-scripted-malformed-native-frame");
                result.usage.generated_output_tokens = 1;
                result.resources.output_tokens = 1;
                result.failure = Some(agentmage_kernel_contracts::ModelRuntimeFailure {
                    code: "runtime.model.protocol_rejected".to_owned(),
                    retryable_after_correction: true,
                    dependency_recovery_required: false,
                    contract_error: None,
                });
                return Ok(result);
            }
            ScriptedDevelopmentStep::Tool(call) => (ModelProposalKind::ToolCall, None, Some(call)),
            ScriptedDevelopmentStep::RollbackFromHistory {
                definition,
                intent_sha256,
                change_plan_sha256,
            } => (
                ModelProposalKind::ToolCall,
                None,
                Some(rollback_from_history_candidate(
                    context,
                    &definition,
                    intent_sha256,
                    change_plan_sha256,
                )?),
            ),
            ScriptedDevelopmentStep::Complete(payload) => {
                (ModelProposalKind::CompletionCandidate, Some(payload), None)
            }
        };
        let mut proposal = ClosedModelProposal {
            schema_version: CONTRACT_SCHEMA_VERSION,
            proposal_id: ProposalId::from_raw(format!("scripted-proposal-{}", self.calls)),
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
        proposal.proposal_sha256 = runtime_sha256(&proposal)?;
        Ok(ModelRunResult {
            schema_version: CONTRACT_SCHEMA_VERSION,
            model_run_id: request.model_run_id.clone(),
            stream_id: ModelStreamId::from_raw(format!("scripted-stream-{}", self.calls)),
            correlation_id: request.correlation_id.clone(),
            terminal_state: ModelRunTerminalState::Proposed,
            finish_reason: ModelFinishReason::EndOfSequence,
            fragment_count: 1,
            response_sha256: sha256(proposal.proposal_sha256.as_bytes()),
            proposal: Some(proposal),
            failure: None,
            usage: ModelTokenUsage {
                rendered_prompt_tokens: context.input_tokens,
                cached_input_tokens: None,
                evaluated_input_tokens: None,
                generated_output_tokens: 1,
                reasoning_output_tokens: None,
                output_token_reserve: request.max_output_tokens,
                remaining_capacity_tokens: None,
            },
            resources: ModelResourceReport {
                adapter_id: request.adapter_id.clone(),
                profile_id: request.profile_id.clone(),
                model_run_id: Some(request.model_run_id.clone()),
                resident_memory_bytes: 1,
                accelerator_memory_bytes: 0,
                input_tokens: context.input_tokens,
                output_tokens: 1,
                elapsed_ms: 1,
            },
        })
    }
}

fn rollback_from_history_candidate(
    context: &agentmage_kernel_contracts::ModelContextPacket,
    definition: &ToolDefinition,
    intent_sha256: String,
    change_plan_sha256: String,
) -> Result<ModelToolCallCandidate, RuntimePortFailure> {
    let history = context
        .messages
        .iter()
        .rev()
        .filter(|message| message.role == ModelMessageRole::Tool)
        .filter_map(|message| {
            (message.content.sha256 == sha256(&message.content.bytes))
                .then(|| serde_json::from_slice::<serde_json::Value>(&message.content.bytes).ok())
                .flatten()
        })
        .find(|value| value["completed_call"]["tool_id"] == CHANGE_HISTORY_TOOL_ID)
        .ok_or(RuntimePortFailure::Invalid)?;
    let history = projected_change_history(&history)?;
    let source = history
        .records
        .last()
        .cloned()
        .ok_or(RuntimePortFailure::Invalid)?;
    let request = RollbackRequest {
        schema_version: 1,
        rollback_id: "scripted-exact-rollback".to_owned(),
        source,
        intent_sha256,
        change_plan_sha256,
    };
    let bytes = serde_json::to_vec(&request).map_err(|_| RuntimePortFailure::Invalid)?;
    Ok(ModelToolCallCandidate {
        tool_call_id: ToolCallId::from_raw("scripted-exact-rollback"),
        tool_id: definition.tool_id.clone(),
        tool_version: definition.tool_version.clone(),
        arguments: ContractPayload {
            schema: definition.input_schema.clone(),
            media_type: "application/json".to_owned(),
            sha256: sha256(&bytes),
            bytes,
        },
    })
}

// Scripted fixture consumption follows the same readable, paired observation
// shape used by native models. It remains an untrusted proposal; the history
// owner and exact rollback effect boundary still validate the retained source.
fn projected_change_history(
    value: &serde_json::Value,
) -> Result<ChangeHistoryOutput, RuntimePortFailure> {
    let call = &value["completed_call"];
    let result = &value["result"];
    let output = &result["output"];
    if value["untrusted_tool_observation"] != true
        || call["tool_id"] != CHANGE_HISTORY_TOOL_ID
        || call["tool_version"] != CODING_HISTORY_TOOL_VERSION
        || call["tool_call_id"].as_str().is_none_or(str::is_empty)
        || call["tool_call_id"] != result["tool_call_id"]
        || output["schema"]["schema_id"] != CHANGE_HISTORY_OUTPUT_SCHEMA_ID
        || output["media_type"] != "application/json"
    {
        return Err(RuntimePortFailure::Invalid);
    }
    let history: ChangeHistoryOutput = serde_json::from_value(output["content"].clone())
        .map_err(|_| RuntimePortFailure::Invalid)?;
    let bytes = serde_json::to_vec(&history).map_err(|_| RuntimePortFailure::Invalid)?;
    if output["sha256"] != sha256(&bytes) {
        return Err(RuntimePortFailure::Invalid);
    }
    Ok(history)
}

fn cancelled_model_result(
    request: &ModelRunRequest,
    input_tokens: u32,
    elapsed_ms: u64,
) -> ModelRunResult {
    ModelRunResult {
        schema_version: CONTRACT_SCHEMA_VERSION,
        model_run_id: request.model_run_id.clone(),
        stream_id: ModelStreamId::from_raw("scripted-cancelled-stream"),
        correlation_id: request.correlation_id.clone(),
        terminal_state: ModelRunTerminalState::Cancelled,
        finish_reason: ModelFinishReason::Cancelled,
        fragment_count: 1,
        response_sha256: sha256(b"scripted-cancelled"),
        proposal: None,
        failure: Some(ModelRuntimeFailure {
            code: "runtime.model.cancelled".to_owned(),
            retryable_after_correction: false,
            dependency_recovery_required: false,
            contract_error: None,
        }),
        usage: ModelTokenUsage {
            rendered_prompt_tokens: input_tokens,
            cached_input_tokens: None,
            evaluated_input_tokens: None,
            generated_output_tokens: 0,
            reasoning_output_tokens: None,
            output_token_reserve: request.max_output_tokens,
            remaining_capacity_tokens: None,
        },
        resources: ModelResourceReport {
            adapter_id: request.adapter_id.clone(),
            profile_id: request.profile_id.clone(),
            model_run_id: Some(request.model_run_id.clone()),
            resident_memory_bytes: 1,
            accelerator_memory_bytes: 0,
            input_tokens,
            output_tokens: 0,
            elapsed_ms: elapsed_ms.min(request.timeout_ms),
        },
    }
}

/// Approximate planning counter; native rendering is exactly bound during source selection.
pub struct DevelopmentTokenCounter {
    counter_id: String,
}

impl DevelopmentTokenCounter {
    fn new(counter_id: String) -> Self {
        Self { counter_id }
    }
}

impl CodingTokenCounter for DevelopmentTokenCounter {
    fn counter_id(&self) -> &str {
        &self.counter_id
    }

    fn count_tokens(&mut self, bytes: &[u8]) -> Result<u32, RuntimePortFailure> {
        u32::try_from(bytes.len().div_ceil(4).max(1))
            .map_err(|_| RuntimePortFailure::ResourceExhausted)
    }
}

/// Operating-system wall clock used only for trusted deadlines and event times.
pub struct OsRuntimeClock;

impl RuntimeClock for OsRuntimeClock {
    fn now_epoch_ms(&mut self) -> Result<u64, RuntimePortFailure> {
        now_epoch_ms().map_err(|_| RuntimePortFailure::Unavailable)
    }
}

fn runtime_limits(
    scenario: CodingDevelopmentScenario,
    model: CodingDevelopmentModel,
) -> RuntimeRunLimits {
    RuntimeRunLimits {
        max_turns: 16,
        max_model_calls: 16,
        max_tool_calls: 16,
        max_repeated_tool_calls: 4,
        max_tool_call_depth: 2,
        max_no_progress_turns: 4,
        max_context_refreshes: 16,
        max_events: if scenario == CodingDevelopmentScenario::Overflow {
            13
        } else {
            2_048
        },
        max_elapsed_ms: if model.is_candidate() {
            45 * 60 * 1_000
        } else {
            600_000
        },
        max_output_bytes: if matches!(
            scenario,
            CodingDevelopmentScenario::DiskPressure | CodingDevelopmentScenario::OutputPressure
        ) {
            1_024
        } else {
            4 * 1024 * 1024
        },
    }
}

fn file_sha256(path: &Path) -> Result<String, CodingDevelopmentRuntimeError> {
    let bytes = fs::read(path).map_err(|_| CodingDevelopmentRuntimeError::Platform)?;
    Ok(sha256(&bytes))
}

fn development_sha256<T: serde::Serialize>(
    value: &T,
) -> Result<String, CodingDevelopmentRuntimeError> {
    serde_json::to_vec(value)
        .map(|bytes| sha256(&bytes))
        .map_err(|_| CodingDevelopmentRuntimeError::Composition)
}

fn runtime_sha256<T: serde::Serialize>(value: &T) -> Result<String, RuntimePortFailure> {
    serde_json::to_vec(value)
        .map(|bytes| sha256(&bytes))
        .map_err(|_| RuntimePortFailure::Invalid)
}

fn sha256(bytes: &[u8]) -> String {
    let mut value = String::with_capacity(64);
    for byte in Sha256::digest(bytes) {
        write!(&mut value, "{byte:02x}").expect("String writes cannot fail");
    }
    value
}

fn digest_bytes(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}

fn now_epoch_ms() -> Result<u64, CodingDevelopmentRuntimeError> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|duration| u64::try_from(duration.as_millis()).ok())
        .filter(|value| *value > 0)
        .ok_or(CodingDevelopmentRuntimeError::State)
}

#[cfg(test)]
mod tests {
    #[test]
    fn correction_allowlist_never_recovers_identity_capacity_truncation_or_unknown_errors() {
        for code in [
            "model.muse-codec.native-json-invalid",
            "model.gpt-oss-codec.tool-channel-invalid",
        ] {
            assert!(super::correctable_native_protocol_code(code));
        }
        for code in [
            "model.muse-codec.request-mismatch",
            "model.muse-codec.proposal-mismatch",
            "model.muse-codec.final-channel-incomplete",
            "model.muse-codec.response-oversized",
            "model.muse-codec.native-tool-unknown",
            "model.gpt-oss-codec.identity-mismatch",
            "model.gpt-oss-codec.proposal-preimage-invalid",
            "model.gpt-oss-codec.context-invalid",
            "model.gpt-oss-codec.response-size",
            "model.gpt-oss-codec.final-channel-incomplete",
            "model.gpt-oss-codec.tool-unknown",
            "future.codec.failure",
        ] {
            assert!(!super::correctable_native_protocol_code(code), "{code}");
        }
    }
    use super::*;

    #[test]
    fn scripted_rollback_reads_paired_history_without_accepting_identity_or_digest_drift() {
        let history = ChangeHistoryOutput {
            schema_version: 1,
            records: vec![],
            result_sha256: "a".repeat(64),
        };
        let bytes = serde_json::to_vec(&history).unwrap();
        let observation = serde_json::json!({
            "untrusted_tool_observation": true,
            "completed_call": {"tool_id": CHANGE_HISTORY_TOOL_ID, "tool_version": CODING_HISTORY_TOOL_VERSION, "tool_call_id":"history-call"},
            "result": {"tool_call_id":"history-call", "output": {
                "schema":{"schema_id": CHANGE_HISTORY_OUTPUT_SCHEMA_ID}, "media_type":"application/json",
                "content":history, "sha256":sha256(&bytes)
            }}
        });
        assert_eq!(projected_change_history(&observation).unwrap(), history);
        for pointer in [
            "/completed_call/tool_id",
            "/completed_call/tool_version",
            "/result/tool_call_id",
            "/result/output/schema/schema_id",
            "/result/output/sha256",
            "/result/output/media_type",
        ] {
            let mut wrong = observation.clone();
            *wrong.pointer_mut(pointer).unwrap() = serde_json::json!("foreign");
            assert!(projected_change_history(&wrong).is_err(), "{pointer}");
        }
        let mut wrong = observation;
        wrong["result"]["output"]["content"]["records"] = serde_json::json!([{}]);
        assert!(projected_change_history(&wrong).is_err());
    }
}
