//! Executable development-only composition of the real coding coordinator and Linux boundaries.

use std::collections::{BTreeMap, VecDeque};
use std::fmt::Write as _;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use agentmage_capability_read_only::{
    GIT_INSPECTION_TOOL_ID, GIT_INSPECTION_TOOL_VERSION, GitInspectionOperation,
    GitInspectionRequest,
};
use agentmage_capability_repository_map::{
    StructuredArtifactClass, StructuredEdit, StructuredLanguage,
};
use agentmage_kernel_contracts::{
    ActorId, AdapterInstanceId, AuthorityClass, BudgetLimit, BudgetResource,
    CONTRACT_SCHEMA_VERSION, ClosedModelProposal, ContextBudget, ContractPayload, DataSensitivity,
    DecodingProfile, EvidenceKind, ExactModelProfile, FamilyCodecIdentity, HardwareEnvelope,
    LocalEndpointIdentity, LocalTransport, ModelAdapterId, ModelArtifact, ModelCancellationProbe,
    ModelCapability, ModelCapabilityState, ModelCodecId, ModelFinishReason, ModelManifestId,
    ModelMessageRole, ModelModality, ModelProfileId, ModelProposalKind, ModelResourceReport,
    ModelRole, ModelRunRequest, ModelRunResult, ModelRunTerminalState, ModelRuntimeFailure,
    ModelRuntimeIdentity, ModelRuntimeKind, ModelStreamId, ModelTokenUsage, ModelToolCallCandidate,
    NetworkComponent, NetworkDestinationClass, NetworkObservation, PlanId, PlatformArchitecture,
    PlatformFamily, ProposalId, RepositorySnapshotId, RollbackPlan, RuntimeRunId, RuntimeRunLimits,
    SessionId, StopCondition, StopConditionKind, TaskId, ToolCallId, ToolCatalogId, ToolId,
    WorkPacket, WorkPacketId, WorkPacketState, WorkspaceAuthorizationId, WorkspaceId,
    WorkspacePath,
};
use agentmage_kernel_engine::{
    command_runner::{
        CommandBounds, CommandRegistry, CommandRisk, CommandSpec, CommandWorkingDirectory,
    },
    instruction_provenance::build_instruction_ledger,
    model_runtime::{ModelAdmissionCatalog, ModelUsePurpose},
    repository_safety::{OwnedWorktreeRecord, WorktreeDisposition},
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
    LinuxSandboxLimits, LinuxSandboxManifest, LinuxSandboxRunner, linux_repository_path_sha256,
    open_linux_development_authority, select_development_linux_workspace,
};
use sha2::{Digest, Sha256};

use crate::{
    coding_authority::{
        CodingRuntimePolicy, CodingRuntimePolicyRequest, build_coding_runtime_policy,
    },
    coding_changes::{CodingWriteScope, STRUCTURED_PATCH_TOOL_ID, StructuredPatchProposal},
    coding_context::{CodingContextPort, CodingTokenCounter},
    coding_development_activation::{
        CODING_DEVELOPMENT_ACTIVATION, CodingDevelopmentActivation, CodingDevelopmentKeyProvider,
    },
    coding_harness::{EphemeralCodingCoordinator, compose_ephemeral_coding_coordinator},
    coding_plan::build_coding_development_plan_binding,
    coding_run::{CodingRunRequestInput, build_ephemeral_coding_run_request},
    coding_session::{CodingSessionProfile, CodingSessionProfileInput},
    coding_tools::{
        TARGETED_VALIDATION_TOOL_ID, TARGETED_VALIDATION_TOOL_VERSION, TargetedValidationRequest,
    },
    coding_verifier::{CodingCompletionCandidate, CodingTerminalClaim, coding_completion_payload},
    linux_coding::LinuxCodingWorkspace,
    linux_coding_runtime::{
        LinuxCodingRuntimeBoundary, LinuxCodingRuntimeBoundaryInput, OsCodingIdentitySource,
    },
    linux_repository_map::{LinuxRepositoryMapPolicy, build_development_linux_repository_map},
    native_chat_runtime::{NativeChatRuntimeError, NativeChatRuntimeFactory},
    runtime_transport::RuntimePrepareInput,
};

/// Explicitly non-qualified profile used only by the executable development fixture.
pub const SCRIPTED_PROFILE_ID: &str = "scripted-executable-fixture-32k-v1";
const VALIDATION_ID: &str = "validation-unit";
static NEXT_RUN: AtomicU64 = AtomicU64::new(1);

/// Closed executable-fixture scenarios. Every value is visibly non-model-qualified.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CodingDevelopmentScenario {
    /// Inspect Git state and return a verifier-backed no-op.
    NoOp,
    /// Observe a genuine failing test, repair one identifier, rerun, and verify.
    FailedTestRepair,
    /// Hold one cancellable model call so actual signal propagation can be exercised.
    SlowCancel,
}

impl CodingDevelopmentScenario {
    /// Parses one exact CLI label.
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "no-op" => Some(Self::NoOp),
            "failed-test-repair" => Some(Self::FailedTestRepair),
            "slow-cancel" => Some(Self::SlowCancel),
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
pub type CodingDevelopmentCoordinator = EphemeralCodingCoordinator<
    ScriptedDevelopmentModel,
    CodingContextPort<DevelopmentTokenCounter>,
    DevelopmentBoundary,
    OsRuntimeClock,
>;

struct PreparedDevelopmentRun {
    request: agentmage_kernel_contracts::RuntimeRunRequest,
    policy: CodingRuntimePolicy,
}

/// Factory that composes the real coordinator only for one explicit disposable activation.
pub struct CodingDevelopmentRuntimeFactory {
    activation: CodingDevelopmentActivation,
    scenario: CodingDevelopmentScenario,
    platform: &'static LinuxDevelopmentPlatformAdapter,
    profile: &'static CodingSessionProfile,
    workspace: &'static LinuxCodingWorkspace<'static, 'static>,
    prepared: BTreeMap<String, PreparedDevelopmentRun>,
}

impl CodingDevelopmentRuntimeFactory {
    /// Observes and freezes the exact repository/profile foundation before serving IPC.
    pub fn new(
        activation: CodingDevelopmentActivation,
        scenario: CodingDevelopmentScenario,
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
        ensure_private_directory(&management)?;
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
        drop(selected);
        let profile = Box::leak(Box::new(build_profile(
            &activation,
            workspace_id,
            &inventory,
            &repository_map,
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
        Ok(Self {
            activation,
            scenario,
            platform,
            profile,
            workspace,
            prepared: BTreeMap::new(),
        })
    }
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
        if input.engineering_session_id.is_some()
            || input.profile_id != SCRIPTED_PROFILE_ID
            || input.expected_entry_sha256 != self.activation.marker_sha256()
            || input.workspace_id != self.profile.write_scope().workspace_id().as_str()
            || input.workspace_root != self.activation.workspace_root().to_string_lossy()
            || input.prompt.is_empty()
            || input.prompt.len() > 16 * 1024
            || !self.prepared.is_empty()
        {
            return Err(prepare_denied("input-binding"));
        }
        let sequence = NEXT_RUN.fetch_add(1, Ordering::Relaxed);
        let run_id = RuntimeRunId::from_raw(format!("coding-development-run-{sequence:016x}"));
        let session_id = SessionId::from_raw(format!("coding-development-session-{sequence:016x}"));
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
        let acceptance =
            vec!["The exact synthetic validation passes after the bounded repair".to_owned()];
        let packet = development_work_packet(self.profile, &task_id, &input.prompt, &acceptance);
        let request = build_ephemeral_coding_run_request(
            self.profile,
            CodingRunRequestInput {
                run_id,
                session_id,
                objective: input.prompt.clone(),
                acceptance_criteria: acceptance,
                constraints: vec![
                    "Disposable synthetic repository only".to_owned(),
                    "No network, publication, package installation, or commit".to_owned(),
                ],
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
            },
        );
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
        let steps = scripted_steps(
            self.scenario,
            self.profile,
            request,
            self.activation.workspace_root(),
        )
        .map_err(|_| NativeChatRuntimeError::RequestDenied)?;
        let model = ScriptedDevelopmentModel {
            profile: self.profile.model_profile().clone(),
            steps,
            calls: 0,
            initial_delay_ms: (self.scenario == CodingDevelopmentScenario::SlowCancel)
                .then_some(120_000),
        };
        let context =
            CodingContextPort::for_profile(self.profile, Vec::new(), DevelopmentTokenCounter)
                .map_err(|_| NativeChatRuntimeError::RequestDenied)?;
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
        let sandbox_manifest = LinuxSandboxManifest::verify(
            "/usr/bin/systemd-run",
            "/usr/bin/bwrap",
            "/usr/bin/true",
            &[],
        )
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
        })
        .map_err(|_| NativeChatRuntimeError::RuntimeFailed)?;
        compose_ephemeral_coding_coordinator(
            self.profile,
            request.clone(),
            model,
            context,
            boundary,
            OsRuntimeClock,
        )
        .map_err(|_| NativeChatRuntimeError::RuntimeFailed)
    }
}

fn prepare_denied(stage: &str) -> NativeChatRuntimeError {
    eprintln!("coding.development.prepare.{stage}-denied");
    NativeChatRuntimeError::RequestDenied
}

fn build_profile(
    activation: &CodingDevelopmentActivation,
    workspace_id: WorkspaceId,
    inventory: &agentmage_platform_linux::LinuxRepositoryInventory,
    repository_map: &agentmage_capability_repository_map::RepositoryMap,
) -> Result<CodingSessionProfile, CodingDevelopmentRuntimeError> {
    let limits = runtime_limits();
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
        retain_until_epoch_ms: now_epoch_ms()?.saturating_add(24 * 60 * 60 * 1_000),
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
        model: scripted_admitted_model()?,
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
        CodingDevelopmentScenario::NoOp | CodingDevelopmentScenario::SlowCancel => Ok([
            ScriptedDevelopmentStep::Tool(git),
            ScriptedDevelopmentStep::Complete(completion(
                CodingTerminalClaim::NoOp,
                "Inspected the disposable fixture; no change was requested.",
                vec!["Mutation-dependent validation was not required.".to_owned()],
            )?),
        ]
        .into_iter()
        .collect()),
        CodingDevelopmentScenario::FailedTestRepair => {
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
                ScriptedDevelopmentStep::Tool(patch),
                ScriptedDevelopmentStep::Tool(validation("scripted-validation-passing")?),
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
) -> WorkPacket {
    WorkPacket {
        schema_version: CONTRACT_SCHEMA_VERSION,
        work_packet_id: WorkPacketId::from_raw(format!("packet-{}", task_id.as_str())),
        task_id: task_id.clone(),
        revision: 1,
        objective: objective.to_owned(),
        reason: "Exercise the real coordinator in an explicitly disposable fixture".to_owned(),
        owner: "local-user".to_owned(),
        authoritative_evidence: Vec::new(),
        mutable_files: vec!["src/calc.py".to_owned()],
        protected_files: vec![".git".to_owned(), "tests/run_validation.py".to_owned()],
        expected_output: "One verifier-backed coding outcome".to_owned(),
        acceptance_checks: acceptance.to_vec(),
        required_evidence: vec![EvidenceKind::Validation],
        required_capability_class: AuthorityClass::LocalWrite,
        budgets: [
            (BudgetResource::PlanSteps, 16),
            (BudgetResource::ToolCallDepth, 2),
            (BudgetResource::ModelCalls, 16),
            (BudgetResource::ToolCalls, 16),
            (BudgetResource::InputBytes, 8 * 1024 * 1024),
            (BudgetResource::OutputBytes, 4 * 1024 * 1024),
            (BudgetResource::ElapsedMilliseconds, 600_000),
            (BudgetResource::MemoryBytes, 256 * 1024 * 1024),
            (BudgetResource::DiskBytes, 64 * 1024 * 1024),
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
    Tool(ModelToolCallCandidate),
    Complete(ContractPayload),
}

/// Explicit scripted proposal source; it is never exposed as a qualified local model.
pub struct ScriptedDevelopmentModel {
    profile: ExactModelProfile,
    steps: VecDeque<ScriptedDevelopmentStep>,
    calls: u32,
    initial_delay_ms: Option<u64>,
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
        if let Some(delay_ms) = self.initial_delay_ms.take() {
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
            ScriptedDevelopmentStep::Tool(call) => (ModelProposalKind::ToolCall, None, Some(call)),
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

/// Exact bounded token counter for the scripted executable fixture only.
pub struct DevelopmentTokenCounter;

impl CodingTokenCounter for DevelopmentTokenCounter {
    fn counter_id(&self) -> &str {
        "bounded-byte-counter-v1"
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

fn runtime_limits() -> RuntimeRunLimits {
    RuntimeRunLimits {
        max_turns: 16,
        max_model_calls: 16,
        max_tool_calls: 16,
        max_repeated_tool_calls: 4,
        max_tool_call_depth: 2,
        max_no_progress_turns: 4,
        max_context_refreshes: 16,
        max_events: 2_048,
        max_elapsed_ms: 600_000,
        max_output_bytes: 4 * 1024 * 1024,
    }
}

fn ensure_private_directory(path: &Path) -> Result<(), CodingDevelopmentRuntimeError> {
    match fs::create_dir(path) {
        Ok(()) => fs::set_permissions(path, fs::Permissions::from_mode(0o700))
            .map_err(|_| CodingDevelopmentRuntimeError::State),
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            let metadata =
                fs::symlink_metadata(path).map_err(|_| CodingDevelopmentRuntimeError::State)?;
            if metadata.is_dir() && metadata.permissions().mode() & 0o777 == 0o700 {
                Ok(())
            } else {
                Err(CodingDevelopmentRuntimeError::State)
            }
        }
        Err(_) => Err(CodingDevelopmentRuntimeError::State),
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
