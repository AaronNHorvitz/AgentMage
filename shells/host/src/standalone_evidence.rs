//! The standalone evidence development host (Decision 0150): its activation,
//! Rust-owned folder admission, the read-only runtime factory over the
//! admitted snapshot, and the service the host serves over its private IPC.
//!
//! Nothing here writes, runs a command, opens a network connection or reads
//! outside the one admitted folder. The only model is a source-controlled
//! term-match fixture that is not a language model.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::fs::{self, File};
use std::io::Read;
use std::os::unix::fs::MetadataExt;
use std::path::{Component, Path, PathBuf};
use std::sync::{Arc, Mutex};

use agentmage_capability_read_only::{
    ARTIFACT_TOOL_VERSION, ArtifactAttemptLedger, ArtifactExecutionSignal, ArtifactFragment,
    ArtifactItem, ArtifactLimits, ArtifactOutcome, ArtifactRequest, ArtifactResult,
    ArtifactToolKind, artifact_tool_definition, validate_artifact_request,
};
use agentmage_kernel_contracts::{
    ApprovalId, AuthorityClass, BudgetLimit, BudgetResource, CONTRACT_SCHEMA_VERSION,
    CancellationId, ClosedModelProposal, ContextBudget, ContextPacketId, ContextSensitivity,
    ContractPayload, DataSensitivity, DecodingProfile, EvidenceId, EvidenceKind, EvidenceReference,
    ExactModelProfile, FamilyCodecIdentity, GrantId, HardwareEnvelope, ModelAdapterId,
    ModelArtifact, ModelCancellationProbe, ModelCapability, ModelCapabilityState, ModelCodecId,
    ModelContextPacket, ModelFinishReason, ModelManifestId, ModelMessageRole, ModelModality,
    ModelProfileId, ModelProposalKind, ModelResourceReport, ModelRole, ModelRunRequest,
    ModelRunResult, ModelRunTerminalState, ModelRuntimeFailure, ModelRuntimeIdentity,
    ModelRuntimeKind, ModelStreamId, ModelTokenUsage, ModelToolCallCandidate, OperationOutcome,
    PlanId, PlatformArchitecture, PlatformFamily, PolicyId, PostconditionResult, ProposalId,
    ReceiptId, RepositorySnapshotId, RollbackPlan, RuntimeApprovalChallenge,
    RuntimeApprovalResponse, RuntimeArtifactKind, RuntimeArtifactRef, RuntimeEventCursor,
    RuntimeOperationId, RuntimeRunId, RuntimeRunLimits, RuntimeRunRequest, RuntimeSessionMode,
    SchemaId, SchemaReference, SessionId, StateChange, StopCondition, StopConditionKind, Task,
    TaskId, TaskStatus, ToolCall, ToolCallId, ToolCatalogId, ToolDefinition, ToolResult,
    ValidationIssue, ValidationSeverity, VerifierCandidate, VerifierDisposition, VerifierId,
    VerifierRecordId, VerifierSource, WorkPacket, WorkPacketId, WorkPacketState, WorkspaceId,
};
use agentmage_kernel_engine::context_inspection::{ContextInspection, inspect_context};
use agentmage_kernel_engine::job_control::JobControlRequest;
use agentmage_kernel_engine::job_ledger_store::DurableJobLedgers;
use agentmage_kernel_engine::model_orchestration_profile::{
    AllocatedContextPartition, ContextPartitionDisposition, ExactTokenCounterBinding,
    ModelContextWindowPlan,
};
use agentmage_kernel_engine::model_runtime::{ModelAdmissionCatalog, ModelUsePurpose};
use agentmage_kernel_engine::runtime_artifact::{RuntimeArtifactPage, RuntimeArtifactState};
use agentmage_kernel_engine::runtime_coordinator::{
    runtime_tool_catalog_sha256, seal_runtime_run_request,
};
use agentmage_kernel_engine::runtime_loop::{
    ReusableRuntimeCoordinator, RuntimeArtifactAccessPort, RuntimeClock, RuntimeContextPort,
    RuntimeModelPort, RuntimePermissionEvaluation, RuntimePortFailure, RuntimeToolBoundary,
    RuntimeToolExecution, RuntimeVerificationInput, RuntimeVerifierPort, runtime_tool_references,
};
use agentmage_kernel_engine::source_preparation::{
    ExactSourceTokenCounter, PreparedSourceRetention, SourceAdmissionRequest, SourceEncoding,
    SourceMediaFamily, SourcePreparationError, SourcePreparationLimits, SourcePreparationService,
};
use agentmage_kernel_engine::source_runtime_context::PreparedSourceRuntimeContext;
use agentmage_kernel_engine::tooling::{Tool, ToolRegistry};
use agentmage_platform_linux::{
    LinuxAuthorityRuntime, LinuxDevelopmentPlatformAdapter, open_linux_development_authority,
};
use rustix::fd::OwnedFd;
use rustix::fs::{Dir, FileType, Mode, OFlags, ResolveFlags, fstat, open, openat2};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::coding_action_history::{RunActionHistory, RunActionHistorySource};
use crate::coding_context::RunContextInspectionSource;
use crate::coding_live_runtime::LiveCodingRuntimeService;
use crate::coding_recoverability::{
    RecoverabilityError, RecoverabilityReport, RunRecoverabilitySource, assess_recoverability,
    assess_run_recoverability,
};
use crate::native_chat_runtime::NativeChatRuntimeFactory;
use crate::runtime_transport::{
    RuntimeClientScope, RuntimeJobControl, RuntimeJobStatus, RuntimePrepareInput,
    RuntimeRunDeclarations, RuntimeTransportError, RuntimeTransportPort, RuntimeTransportStep,
};
use crate::source_artifact_runtime::dispatch_native_source_artifact;
use crate::standalone_folder::{
    FOLDER_ADMISSION_SCHEMA_VERSION, FolderAdmissionView, FolderAnswer, FolderEntryDisposition,
    FolderEntryView, FolderLimitsView, FolderRefusal, FolderRequest, MAX_FOLDER_DEPTH,
    MAX_FOLDER_ENTRIES, MAX_FOLDER_ENTRY_PATH_BYTES, MAX_FOLDER_FILE_BYTES, MAX_FOLDER_LINE_BYTES,
    MAX_FOLDER_PATH_BYTES, MAX_FOLDER_TOTAL_BYTES, folder_workspace_id,
};

/// Exact activation identity; production startup never accepts it.
pub const STANDALONE_EVIDENCE_ACTIVATION: &str = "standalone-evidence-development-v1";
/// The only selectable profile: a term-match fixture, not a language model.
pub const STANDALONE_EVIDENCE_PROFILE_ID: &str = "standalone-evidence-fixture-v1";
/// Media type of the fixture's closed answer.
pub const STANDALONE_EVIDENCE_ANSWER_MEDIA_TYPE: &str = "application/json";
/// Schema identity of the fixture's closed answer.
pub const STANDALONE_EVIDENCE_ANSWER_SCHEMA_ID: &str = "runtime.standalone-evidence.answer";
/// Identity of the always-required folder inventory source.
pub const FOLDER_INVENTORY_SOURCE_ID: &str = "folder-inventory";

const INVENTORY_HEADER: &str = "Admitted folder inventory (standalone-evidence-development-v1)";
const TOKEN_COUNTER: &str = "bounded-byte-counter-v1";
const MAX_QUESTION_BYTES: usize = 2 * 1024;
const MAX_TERMS: usize = 4;
const MIN_TERM_CHARS: usize = 4;
const MAX_SEARCHES: usize = 16;
const MAX_STATEMENTS: usize = 6;
const SEARCH_ITEMS: u32 = 8;
const SEARCH_OUTPUT_BYTES: u64 = 16 * 1024;
const ZERO_SHA256: &str = "0000000000000000000000000000000000000000000000000000000000000000";
/// Longest development delay of the fixture before each proposal.
pub const MAX_FIXTURE_STEP_DELAY: std::time::Duration = std::time::Duration::from_millis(2_000);
const STOP_WORDS: &[&str] = &[
    "about", "after", "also", "been", "before", "being", "does", "doing", "from", "have", "into",
    "more", "most", "much", "only", "other", "over", "please", "should", "some", "such", "tell",
    "than", "that", "their", "them", "then", "there", "these", "they", "this", "those", "very",
    "what", "when", "where", "which", "while", "with", "would", "your",
];

// ---------------------------------------------------------------------------
// Activation
// ---------------------------------------------------------------------------

/// Stable content-free activation refusal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StandaloneEvidenceActivationError {
    /// A root was relative, aliased, nested in the other, or not private.
    RootDenied,
}

impl StandaloneEvidenceActivationError {
    /// One stable redacted code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::RootDenied => "standalone.evidence.root-denied",
        }
    }
}

impl fmt::Display for StandaloneEvidenceActivationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.code())
    }
}

impl std::error::Error for StandaloneEvidenceActivationError {}

/// The two private roots of one development launch.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StandaloneEvidenceActivation {
    state_root: PathBuf,
    disposable_root: PathBuf,
}

impl StandaloneEvidenceActivation {
    /// Validates two canonical, owner-only (`0700`) directories, neither
    /// inside the other.
    pub fn validate(
        state_root: &Path,
        disposable_root: &Path,
    ) -> Result<Self, StandaloneEvidenceActivationError> {
        let state_root = private_directory(state_root)?;
        let disposable_root = private_directory(disposable_root)?;
        if state_root.starts_with(&disposable_root) || disposable_root.starts_with(&state_root) {
            return Err(StandaloneEvidenceActivationError::RootDenied);
        }
        Ok(Self {
            state_root,
            disposable_root,
        })
    }

    /// Revalidates both roots before folder admission and each question.
    pub fn revalidate(&self) -> Result<(), StandaloneEvidenceActivationError> {
        if Self::validate(&self.state_root, &self.disposable_root)? != *self {
            return Err(StandaloneEvidenceActivationError::RootDenied);
        }
        Ok(())
    }

    /// The private state root, where the host's socket lives.
    #[must_use]
    pub fn state_root(&self) -> &Path {
        &self.state_root
    }

    /// The private root every admitted folder lies beneath.
    #[must_use]
    pub fn disposable_root(&self) -> &Path {
        &self.disposable_root
    }
}

// The same rule as the coding development activation (Decision 0063).
fn private_directory(path: &Path) -> Result<PathBuf, StandaloneEvidenceActivationError> {
    let denied = || StandaloneEvidenceActivationError::RootDenied;
    if !path.is_absolute() {
        return Err(denied());
    }
    let canonical = path.canonicalize().map_err(|_| denied())?;
    if canonical != path {
        return Err(denied());
    }
    let metadata = fs::symlink_metadata(path).map_err(|_| denied())?;
    if !metadata.is_dir()
        || metadata.uid() != rustix::process::getuid().as_raw()
        || metadata.mode() & 0o777 != 0o700
    {
        return Err(denied());
    }
    Ok(canonical)
}

// ---------------------------------------------------------------------------
// Fixture profile and token counter
// ---------------------------------------------------------------------------

/// The exact fixture profile, admitted only for contract testing.
pub fn standalone_evidence_profile() -> Result<ExactModelProfile, RuntimeTransportError> {
    let profile = ExactModelProfile {
        schema_version: CONTRACT_SCHEMA_VERSION,
        profile_id: ModelProfileId::from_raw(STANDALONE_EVIDENCE_PROFILE_ID),
        manifest_id: ModelManifestId::from_raw("standalone-evidence-fixture-manifest-v1"),
        manifest_sha256: sha256(b"standalone-evidence-fixture-manifest-v1"),
        display_name: "Standalone evidence fixture (term match, not a model)".to_owned(),
        family: "deterministic_fake_evidence".to_owned(),
        publisher_control: "AgentMage development fixture".to_owned(),
        lineage: vec!["source-controlled-term-match-fixture-v1".to_owned()],
        license_spdx: "Apache-2.0".to_owned(),
        license_terms_sha256: sha256(include_bytes!("../../../LICENSE")),
        artifact: ModelArtifact {
            artifact_id: "standalone-evidence-fixture".to_owned(),
            publisher: "AgentMage development fixture".to_owned(),
            source_revision: "source-tree-bound".to_owned(),
            format: "in-process-term-match-proposals".to_owned(),
            bytes: 1,
            sha256: sha256(b"standalone-evidence-fixture"),
        },
        transformations: Vec::new(),
        codec: FamilyCodecIdentity {
            codec_id: ModelCodecId::from_raw("standalone-evidence-closed-proposal-v1"),
            codec_version: "1.0.0".to_owned(),
            codec_sha256: sha256(b"standalone-evidence-closed-proposal-v1"),
            tokenizer: TOKEN_COUNTER.to_owned(),
            tokenizer_sha256: sha256(TOKEN_COUNTER.as_bytes()),
            template: "standalone-evidence-proposal-v1".to_owned(),
            template_sha256: sha256(b"standalone-evidence-proposal-v1"),
            tool_protocol_version: "closed-proposal-v1".to_owned(),
            end_tokens: vec![0],
            reasoning_enabled: false,
        },
        runtime: ModelRuntimeIdentity {
            adapter_id: ModelAdapterId::from_raw("standalone-evidence-fixture-adapter-v1"),
            kind: ModelRuntimeKind::DeterministicFake,
            contract_version: 1,
            runtime_build: "agentmage-source-term-match-fixture-v1".to_owned(),
            runtime_sha256: sha256(b"agentmage-source-term-match-fixture-v1"),
            platform: PlatformFamily::DeterministicFake,
            architecture: PlatformArchitecture::X86_64,
        },
        quantization: "not-applicable".to_owned(),
        modalities: vec![ModelModality::Text],
        context: ContextBudget {
            max_context_tokens: 32_768,
            max_input_bytes: 512 * 1024,
            max_messages: 4096,
            token_counter: TOKEN_COUNTER.to_owned(),
            token_counter_sha256: sha256(TOKEN_COUNTER.as_bytes()),
        },
        decoding: DecodingProfile {
            profile_id: "standalone-evidence-exact-v1".to_owned(),
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
        capabilities: vec![ModelCapability {
            role: ModelRole::Dialogue,
            state: ModelCapabilityState::Passed,
            evaluation_profile: Some(STANDALONE_EVIDENCE_PROFILE_ID.to_owned()),
            result_sha256: Some(sha256(STANDALONE_EVIDENCE_PROFILE_ID.as_bytes())),
            limitations: vec![
                "standalone-evidence-fixture-only".to_owned(),
                "not-a-qualified-model".to_owned(),
                "term-match-not-language-model".to_owned(),
            ],
        }],
        policy_sha256: sha256(STANDALONE_EVIDENCE_ACTIVATION.as_bytes()),
        lifecycle: agentmage_kernel_contracts::ModelLifecycleState::Candidate,
        enabled: false,
        automatic_fallback: false,
    };
    ModelAdmissionCatalog::new(vec![profile.clone()])
        .and_then(|catalog| catalog.admit(&profile, ModelUsePurpose::ContractTest))
        .map(|admitted| admitted.exact_profile().clone())
        .map_err(|_| RuntimeTransportError::RequestDenied)
}

/// The fixture profile's counter: one token per four bytes, at least one.
#[derive(Clone)]
pub struct EvidenceTokenCounter {
    binding: ExactTokenCounterBinding,
}

impl EvidenceTokenCounter {
    /// The counter bound to `profile`.
    #[must_use]
    pub fn for_profile(profile: &ExactModelProfile) -> Self {
        Self {
            binding: ExactTokenCounterBinding {
                token_counter_id: profile.context.token_counter.clone(),
                token_counter_sha256: profile.context.token_counter_sha256.clone(),
                tokenizer_sha256: profile.codec.tokenizer_sha256.clone(),
            },
        }
    }
}

impl ExactSourceTokenCounter for EvidenceTokenCounter {
    fn binding(&self) -> ExactTokenCounterBinding {
        self.binding.clone()
    }

    fn count_tokens(&mut self, content: &str) -> Result<u32, SourcePreparationError> {
        u32::try_from(content.len().div_ceil(4).max(1))
            .map_err(|_| SourcePreparationError::ResourceLimit)
    }
}

// ---------------------------------------------------------------------------
// Folder admission
// ---------------------------------------------------------------------------

/// One admitted immutable folder snapshot.
pub struct FolderSnapshot {
    view: FolderAdmissionView,
    sources: Arc<SourcePreparationService>,
}

impl FolderSnapshot {
    /// What a person sees.
    #[must_use]
    pub const fn view(&self) -> &FolderAdmissionView {
        &self.view
    }
}

struct Captured {
    path: String,
    bytes: Vec<u8>,
}

struct Walk {
    root: OwnedFd,
    entries: Vec<FolderEntryView>,
    captured: Vec<Captured>,
    counted: u32,
    total_bytes: u64,
}

/// Reads one folder beneath the activation's disposable root and admits its
/// supported text as the current snapshot. Only the host calls this.
pub fn admit_folder(
    activation: &StandaloneEvidenceActivation,
    folder: &str,
    profile: &ExactModelProfile,
    collected_at_epoch_ms: u64,
) -> Result<FolderSnapshot, FolderRefusal> {
    let path = normalized_absolute(folder).ok_or(FolderRefusal::NotAbsolute)?;
    activation
        .revalidate()
        .map_err(|_| FolderRefusal::OutsideDisposableRoot)?;
    if path == activation.disposable_root() || !path.starts_with(activation.disposable_root()) {
        return Err(FolderRefusal::OutsideDisposableRoot);
    }
    let slash = open(
        "/",
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::CLOEXEC,
        Mode::empty(),
    )
    .map_err(|_| FolderRefusal::Failed)?;
    let relative = folder.trim_start_matches('/');
    let root = beneath(&slash, relative, OFlags::RDONLY | OFlags::DIRECTORY)
        .map_err(|_| FolderRefusal::Unavailable)?;
    let metadata = fstat(&root).map_err(|_| FolderRefusal::Unavailable)?;
    if FileType::from_raw_mode(metadata.st_mode) != FileType::Directory {
        return Err(FolderRefusal::Unavailable);
    }
    if metadata.st_uid != rustix::process::getuid().as_raw() || metadata.st_mode & 0o022 != 0 {
        return Err(FolderRefusal::NotPrivate);
    }
    let mut walk = Walk {
        root,
        entries: Vec::new(),
        captured: Vec::new(),
        counted: 0,
        total_bytes: 0,
    };
    walk.directory("", 0)?;
    walk.entries
        .sort_by(|left, right| left.path.cmp(&right.path));
    walk.captured
        .sort_by(|left, right| left.path.cmp(&right.path));
    build_snapshot(folder, walk, profile, collected_at_epoch_ms)
}

fn normalized_absolute(folder: &str) -> Option<PathBuf> {
    if folder.is_empty() || folder.len() > MAX_FOLDER_PATH_BYTES || folder.contains('\0') {
        return None;
    }
    let path = Path::new(folder);
    if !path.is_absolute()
        || path
            .components()
            .any(|component| matches!(component, Component::CurDir | Component::ParentDir))
    {
        return None;
    }
    let normalized: PathBuf = path.components().collect();
    // The text itself must be normal, so the same folder has one identity.
    (normalized.to_str() == Some(folder) && folder != "/").then_some(normalized)
}

fn beneath(root: &OwnedFd, path: &str, flags: OFlags) -> rustix::io::Result<OwnedFd> {
    openat2(
        root,
        path,
        flags | OFlags::CLOEXEC | OFlags::NOFOLLOW,
        Mode::empty(),
        ResolveFlags::BENEATH | ResolveFlags::NO_SYMLINKS | ResolveFlags::NO_MAGICLINKS,
    )
}

impl Walk {
    fn report(&mut self, path: String, disposition: FolderEntryDisposition, reason: &str) {
        self.entries.push(FolderEntryView {
            path,
            disposition,
            reason_code: reason.to_owned(),
            byte_len: 0,
            content_sha256: None,
            source_id: None,
        });
    }

    fn directory(&mut self, prefix: &str, depth: u32) -> Result<(), FolderRefusal> {
        let fd = beneath(
            &self.root,
            if prefix.is_empty() { "." } else { prefix },
            OFlags::RDONLY | OFlags::DIRECTORY,
        )
        .map_err(|_| FolderRefusal::Changed)?;
        let before = fstat(&fd).map_err(|_| FolderRefusal::Failed)?;
        let mut directory = Dir::read_from(&fd).map_err(|_| FolderRefusal::Failed)?;
        let mut names: Vec<(Vec<u8>, FileType)> = Vec::new();
        while let Some(entry) = directory.read() {
            let entry = entry.map_err(|_| FolderRefusal::Failed)?;
            let name = entry.file_name().to_bytes();
            if name == b"." || name == b".." {
                continue;
            }
            self.counted += 1;
            if self.counted > MAX_FOLDER_ENTRIES {
                return Err(FolderRefusal::TooManyEntries);
            }
            names.push((name.to_vec(), entry.file_type()));
        }
        names.sort_by(|left, right| left.0.cmp(&right.0));
        for (name, kind) in names {
            let display = display_name(&name);
            let path = if prefix.is_empty() {
                display.clone()
            } else {
                format!("{prefix}/{display}")
            };
            if path.len() > MAX_FOLDER_ENTRY_PATH_BYTES {
                return Err(FolderRefusal::TooDeep);
            }
            if std::str::from_utf8(&name).is_err() {
                self.report(
                    path,
                    FolderEntryDisposition::Rejected,
                    "folder.entry.name-not-utf8",
                );
                continue;
            }
            if display.starts_with('.') {
                self.report(path, FolderEntryDisposition::Skipped, "folder.entry.hidden");
                continue;
            }
            if kind == FileType::Symlink {
                self.report(
                    path,
                    FolderEntryDisposition::Rejected,
                    "folder.entry.symlink",
                );
                continue;
            }
            let Ok(entry_fd) = beneath(&self.root, &path, OFlags::RDONLY | OFlags::NONBLOCK) else {
                self.report(
                    path,
                    FolderEntryDisposition::Rejected,
                    "folder.entry.unreadable",
                );
                continue;
            };
            let metadata = fstat(&entry_fd).map_err(|_| FolderRefusal::Failed)?;
            match FileType::from_raw_mode(metadata.st_mode) {
                FileType::Directory => {
                    if depth + 1 > MAX_FOLDER_DEPTH {
                        return Err(FolderRefusal::TooDeep);
                    }
                    drop(entry_fd);
                    self.directory(&path, depth + 1)?;
                }
                FileType::RegularFile => self.file(path, &display, entry_fd, &metadata)?,
                _ => self.report(
                    path,
                    FolderEntryDisposition::Rejected,
                    "folder.entry.special-file",
                ),
            }
        }
        let after = fstat(&fd).map_err(|_| FolderRefusal::Failed)?;
        if before.st_mtime != after.st_mtime
            || before.st_mtime_nsec != after.st_mtime_nsec
            || before.st_ctime != after.st_ctime
            || before.st_ctime_nsec != after.st_ctime_nsec
        {
            return Err(FolderRefusal::Changed);
        }
        Ok(())
    }

    fn file(
        &mut self,
        path: String,
        name: &str,
        fd: OwnedFd,
        before: &rustix::fs::Stat,
    ) -> Result<(), FolderRefusal> {
        let extension = Path::new(name)
            .extension()
            .and_then(|extension| extension.to_str())
            .map(str::to_ascii_lowercase);
        if !matches!(extension.as_deref(), Some("txt" | "md" | "markdown")) {
            self.report(
                path,
                FolderEntryDisposition::Skipped,
                "folder.entry.unsupported-format",
            );
            return Ok(());
        }
        let Ok(size) = u64::try_from(before.st_size) else {
            self.report(
                path,
                FolderEntryDisposition::Rejected,
                "folder.entry.unreadable",
            );
            return Ok(());
        };
        if size > MAX_FOLDER_FILE_BYTES {
            self.report(
                path,
                FolderEntryDisposition::Rejected,
                "folder.entry.too-large",
            );
            return Ok(());
        }
        let file = File::from(fd);
        let mut bytes = Vec::new();
        if (&file)
            .take(MAX_FOLDER_FILE_BYTES + 1)
            .read_to_end(&mut bytes)
            .is_err()
        {
            self.report(
                path,
                FolderEntryDisposition::Rejected,
                "folder.entry.unreadable",
            );
            return Ok(());
        }
        let after = fstat(&file).map_err(|_| FolderRefusal::Failed)?;
        if bytes.len() as u64 > MAX_FOLDER_FILE_BYTES
            || before.st_size != after.st_size
            || before.st_mtime != after.st_mtime
            || before.st_mtime_nsec != after.st_mtime_nsec
            || before.st_ctime != after.st_ctime
            || before.st_ctime_nsec != after.st_ctime_nsec
        {
            self.report(
                path,
                FolderEntryDisposition::Rejected,
                "folder.entry.changed",
            );
            return Ok(());
        }
        let Ok(text) = std::str::from_utf8(&bytes) else {
            self.report(
                path,
                FolderEntryDisposition::Rejected,
                "folder.entry.not-utf8",
            );
            return Ok(());
        };
        if text
            .chars()
            .any(|character| character.is_control() && !matches!(character, '\n' | '\r' | '\t'))
        {
            self.report(
                path,
                FolderEntryDisposition::Rejected,
                "folder.entry.control-characters",
            );
            return Ok(());
        }
        if text.trim().is_empty() {
            self.report(path, FolderEntryDisposition::Skipped, "folder.entry.empty");
            return Ok(());
        }
        if text
            .lines()
            .any(|line| line.len() > MAX_FOLDER_LINE_BYTES as usize)
        {
            self.report(
                path,
                FolderEntryDisposition::Rejected,
                "folder.entry.long-line",
            );
            return Ok(());
        }
        self.total_bytes = self
            .total_bytes
            .checked_add(bytes.len() as u64)
            .filter(|total| *total <= MAX_FOLDER_TOTAL_BYTES)
            .ok_or(FolderRefusal::TooLarge)?;
        self.captured.push(Captured { path, bytes });
        Ok(())
    }
}

// Names are shown exactly when they are UTF-8; any other byte is escaped,
// so every entry keeps one distinct, ordered path.
fn display_name(name: &[u8]) -> String {
    match std::str::from_utf8(name) {
        Ok(text) => text.to_owned(),
        Err(_) => name
            .iter()
            .map(|byte| {
                if byte.is_ascii_graphic() && *byte != b'\\' {
                    char::from(*byte).to_string()
                } else {
                    format!("\\x{byte:02x}")
                }
            })
            .collect(),
    }
}

fn build_snapshot(
    folder: &str,
    walk: Walk,
    profile: &ExactModelProfile,
    collected_at_epoch_ms: u64,
) -> Result<FolderSnapshot, FolderRefusal> {
    let mut counter = EvidenceTokenCounter::for_profile(profile);
    let mut sources = SourcePreparationService::new();
    let mut entries = walk.entries;
    let mut inventory = vec![INVENTORY_HEADER.to_owned()];
    for (index, captured) in walk.captured.into_iter().enumerate() {
        let source_id = format!("folder-source-{:03}", index + 1);
        let content_sha256 = sha256(&captured.bytes);
        let byte_len = captured.bytes.len() as u64;
        let admitted = sources.admit(
            SourceAdmissionRequest {
                source_id: source_id.clone(),
                request_id: format!("folder-admission-{:03}", index + 1),
                source_kind: "standalone_folder_text".to_owned(),
                protected_origin_sha256: sha256(
                    format!("standalone-folder:{folder}/{}", captured.path).as_bytes(),
                ),
                media_type: "text/plain".to_owned(),
                media_family: SourceMediaFamily::PlainText,
                encoding: SourceEncoding::Utf8,
                sensitivity: ContextSensitivity::Internal,
                retention: PreparedSourceRetention::Ephemeral,
                collected_at_epoch_ms,
                limits: SourcePreparationLimits::default(),
                bytes: captured.bytes,
            },
            &mut counter,
            &mut || false,
        );
        match admitted {
            Ok(manifest) => {
                inventory.push(format!(
                    "source\t{source_id}\t{}\t{}",
                    manifest.manifest_sha256, captured.path
                ));
                entries.push(FolderEntryView {
                    path: captured.path,
                    disposition: FolderEntryDisposition::Accepted,
                    reason_code: "folder.entry.accepted".to_owned(),
                    byte_len,
                    content_sha256: Some(content_sha256),
                    source_id: Some(source_id),
                });
            }
            Err(_) => entries.push(FolderEntryView {
                path: captured.path,
                disposition: FolderEntryDisposition::Rejected,
                reason_code: "folder.entry.preparation-failed".to_owned(),
                byte_len: 0,
                content_sha256: None,
                source_id: None,
            }),
        }
    }
    entries.sort_by(|left, right| left.path.cmp(&right.path));
    for entry in &entries {
        if entry.disposition != FolderEntryDisposition::Accepted {
            inventory.push(format!(
                "{}\t{}\t{}",
                match entry.disposition {
                    FolderEntryDisposition::Skipped => "skipped",
                    _ => "rejected",
                },
                entry.reason_code,
                entry.path
            ));
        }
    }
    let count = |disposition| {
        u32::try_from(
            entries
                .iter()
                .filter(|entry| entry.disposition == disposition)
                .count(),
        )
        .map_err(|_| FolderRefusal::TooManyEntries)
    };
    let admitted_bytes = entries
        .iter()
        .filter(|entry| entry.disposition == FolderEntryDisposition::Accepted)
        .map(|entry| entry.byte_len)
        .sum();
    let mut view = FolderAdmissionView {
        schema_version: FOLDER_ADMISSION_SCHEMA_VERSION,
        admission_sha256: ZERO_SHA256.to_owned(),
        folder: folder.to_owned(),
        workspace_id: folder_workspace_id(folder),
        accepted: count(FolderEntryDisposition::Accepted)?,
        skipped: count(FolderEntryDisposition::Skipped)?,
        rejected: count(FolderEntryDisposition::Rejected)?,
        entries,
        admitted_bytes,
        limits: FolderLimitsView::current(),
    };
    view.admission_sha256 = view.compute_sha256().map_err(|_| FolderRefusal::Failed)?;
    inventory.push(format!("admission\t{}", view.admission_sha256));
    let mut inventory_text = inventory.join("\n");
    inventory_text.push('\n');
    sources
        .admit(
            SourceAdmissionRequest {
                source_id: FOLDER_INVENTORY_SOURCE_ID.to_owned(),
                request_id: "folder-admission-inventory".to_owned(),
                source_kind: "standalone_folder_inventory".to_owned(),
                protected_origin_sha256: sha256(format!("standalone-folder:{folder}").as_bytes()),
                media_type: "text/plain".to_owned(),
                media_family: SourceMediaFamily::PlainText,
                encoding: SourceEncoding::Utf8,
                sensitivity: ContextSensitivity::Internal,
                retention: PreparedSourceRetention::Ephemeral,
                collected_at_epoch_ms,
                limits: SourcePreparationLimits::default(),
                bytes: inventory_text.into_bytes(),
            },
            &mut counter,
            &mut || false,
        )
        .map_err(|_| FolderRefusal::Failed)?;
    if !view.verify() {
        return Err(FolderRefusal::Failed);
    }
    Ok(FolderSnapshot {
        view,
        sources: Arc::new(sources),
    })
}

// ---------------------------------------------------------------------------
// Answer contract
// ---------------------------------------------------------------------------

/// The closed answer the fixture proposes and the verifier checks.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvidenceAnswer {
    /// Answer schema.
    pub schema_version: u16,
    /// Cited statements in display order.
    pub statements: Vec<EvidenceStatement>,
    /// The terms the fixture searched for.
    pub terms: Vec<String>,
    /// How many source and term searches were made.
    pub searches: u32,
    /// Whether no admitted source contained a qualifying line.
    pub no_matching_text: bool,
}

/// One statement and its citations.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvidenceStatement {
    /// Statement text.
    pub text: String,
    /// Exact quoted source text, when the statement quotes.
    pub quote: Option<String>,
    /// Evidence observed in this run.
    pub citations: Vec<EvidenceCitation>,
}

/// One citation of exact search evidence.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvidenceCitation {
    /// Evidence identity from this run's tool results.
    pub evidence_id: String,
    /// Prepared source.
    pub source_id: String,
    /// First cited line, zero-based as the search fragment reports it.
    pub start_line: u64,
    /// First line after the cited text.
    pub end_line_exclusive: u64,
}

fn answer_schema() -> SchemaReference {
    SchemaReference {
        schema_id: SchemaId::from_raw(STANDALONE_EVIDENCE_ANSWER_SCHEMA_ID),
        schema_version: 1,
        schema_sha256: sha256(STANDALONE_EVIDENCE_ANSWER_SCHEMA_ID.as_bytes()),
    }
}

// ---------------------------------------------------------------------------
// Tool registry
// ---------------------------------------------------------------------------

struct EvidenceArtifactTool {
    definition: ToolDefinition,
    kind: ArtifactToolKind,
}

impl Tool for EvidenceArtifactTool {
    fn definition(&self) -> &ToolDefinition {
        &self.definition
    }

    fn validate_arguments(&self, arguments: &[u8]) -> Vec<ValidationIssue> {
        validate_artifact_request(self.kind, arguments).map_or_else(
            |error| {
                vec![ValidationIssue {
                    code: error.code().to_owned(),
                    severity: ValidationSeverity::Error,
                    field_path: vec!["arguments".to_owned()],
                    message: "Standalone evidence arguments failed closed validation".to_owned(),
                }]
            },
            |_| Vec::new(),
        )
    }
}

/// The run's whole catalog: the prepared-source artifact tools and nothing else.
pub fn standalone_evidence_registry() -> Result<ToolRegistry, RuntimeTransportError> {
    let mut registry = ToolRegistry::new();
    for kind in ArtifactToolKind::ALL {
        registry
            .register_tool(Box::new(EvidenceArtifactTool {
                definition: artifact_tool_definition(kind),
                kind,
            }))
            .map_err(|_| RuntimeTransportError::RequestDenied)?;
    }
    Ok(registry)
}

// ---------------------------------------------------------------------------
// Fixture model
// ---------------------------------------------------------------------------

struct PlannedSearch {
    source_id: String,
    manifest_sha256: String,
    term: String,
}

/// The term-match fixture. It reads only its context packet and proposes
/// inert calls and one closed answer; it holds no authority.
pub struct EvidenceFixtureModel {
    profile: ExactModelProfile,
    search: ToolDefinition,
    calls: u32,
    plan: Option<Vec<PlannedSearch>>,
    terms: Vec<String>,
    next: usize,
    source_order: Vec<String>,
    step_delay: std::time::Duration,
}

impl EvidenceFixtureModel {
    fn new(profile: ExactModelProfile, step_delay: std::time::Duration) -> Self {
        Self {
            profile,
            search: artifact_tool_definition(ArtifactToolKind::Search),
            calls: 0,
            plan: None,
            terms: Vec::new(),
            next: 0,
            source_order: Vec::new(),
            step_delay,
        }
    }

    fn plan_from(&mut self, context: &ModelContextPacket) {
        let task = context
            .messages
            .iter()
            .find(|message| {
                message.content.schema.schema_id.as_str() == "runtime.prepared-source.task"
            })
            .and_then(|message| std::str::from_utf8(&message.content.bytes).ok())
            .unwrap_or_default();
        self.terms = question_terms(task);
        let inventory = context
            .messages
            .iter()
            .filter(|message| {
                message.content.schema.schema_id.as_str() == "runtime.prepared-source.section"
            })
            .filter_map(|message| std::str::from_utf8(&message.content.bytes).ok())
            .find(|text| text.starts_with(INVENTORY_HEADER))
            .unwrap_or_default();
        let sources: Vec<(String, String)> = inventory
            .lines()
            .filter_map(|line| {
                let mut fields = line.split('\t');
                (fields.next() == Some("source"))
                    .then(|| Some((fields.next()?.to_owned(), fields.next()?.to_owned())))
                    .flatten()
            })
            .collect();
        self.source_order = sources.iter().map(|(id, _)| id.clone()).collect();
        let mut plan = Vec::new();
        for (source_id, manifest_sha256) in &sources {
            for term in &self.terms {
                if plan.len() < MAX_SEARCHES {
                    plan.push(PlannedSearch {
                        source_id: source_id.clone(),
                        manifest_sha256: manifest_sha256.clone(),
                        term: term.clone(),
                    });
                }
            }
        }
        self.plan = Some(plan);
    }

    fn answer(&self, context: &ModelContextPacket) -> EvidenceAnswer {
        // Every distinct line found, with the first evidence that showed it.
        let mut found: BTreeMap<(usize, String, u64, u64), (String, String)> = BTreeMap::new();
        for message in context
            .messages
            .iter()
            .filter(|message| message.role == ModelMessageRole::Tool)
        {
            let Ok(result) = serde_json::from_slice::<ToolResult>(&message.content.bytes) else {
                continue;
            };
            let (Some(output), Some(evidence)) = (result.output.as_ref(), result.evidence.first())
            else {
                continue;
            };
            let Ok(artifact) = serde_json::from_slice::<ArtifactResult>(&output.bytes) else {
                continue;
            };
            let Some(source_id) = artifact.source_id.clone() else {
                continue;
            };
            let order = self
                .source_order
                .iter()
                .position(|id| *id == source_id)
                .unwrap_or(usize::MAX);
            for item in &artifact.items {
                if let ArtifactItem::SearchHit {
                    fragment:
                        ArtifactFragment::Line {
                            start,
                            end_exclusive,
                            content,
                        },
                    ..
                } = item
                {
                    found
                        .entry((order, source_id.clone(), *start, *end_exclusive))
                        .or_insert_with(|| {
                            (content.clone(), evidence.evidence_id.as_str().to_owned())
                        });
                }
            }
        }
        let score = |content: &str| {
            self.terms
                .iter()
                .filter(|term| content.contains(term.as_str()))
                .count()
        };
        let threshold = if self.terms.len() >= 2 { 2 } else { 1 };
        let best = found
            .values()
            .map(|(content, _)| score(content))
            .max()
            .unwrap_or(0);
        let statements: Vec<EvidenceStatement> = if self.terms.is_empty() || best < threshold {
            Vec::new()
        } else {
            found
                .iter()
                .filter(|(_, (content, _))| score(content) == best)
                .take(MAX_STATEMENTS)
                .map(
                    |((_, source_id, start, end), (content, evidence_id))| EvidenceStatement {
                        text: content.trim().to_owned(),
                        quote: Some(content.clone()),
                        citations: vec![EvidenceCitation {
                            evidence_id: evidence_id.clone(),
                            source_id: source_id.clone(),
                            start_line: *start,
                            end_line_exclusive: *end,
                        }],
                    },
                )
                .collect()
        };
        EvidenceAnswer {
            schema_version: 1,
            no_matching_text: statements.is_empty(),
            statements,
            terms: self.terms.clone(),
            searches: u32::try_from(self.next).unwrap_or(u32::MAX),
        }
    }

    fn proposal(
        &mut self,
        request: &ModelRunRequest,
        kind: ModelProposalKind,
        payload: Option<ContractPayload>,
        tool_call: Option<ModelToolCallCandidate>,
    ) -> Result<ModelRunResult, RuntimePortFailure> {
        let mut proposal = ClosedModelProposal {
            schema_version: CONTRACT_SCHEMA_VERSION,
            proposal_id: ProposalId::from_raw(format!(
                "standalone-evidence-proposal-{}-{}",
                request.model_run_id.as_str(),
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
            proposal_sha256: ZERO_SHA256.to_owned(),
        };
        proposal.proposal_sha256 = agentmage_kernel_engine::model_codec::proposal_digest(&proposal)
            .map_err(|_| RuntimePortFailure::Invalid)?;
        Ok(ModelRunResult {
            schema_version: CONTRACT_SCHEMA_VERSION,
            model_run_id: request.model_run_id.clone(),
            stream_id: ModelStreamId::from_raw(format!(
                "standalone-evidence-stream-{}",
                self.calls
            )),
            correlation_id: request.correlation_id.clone(),
            terminal_state: ModelRunTerminalState::Proposed,
            finish_reason: ModelFinishReason::EndOfSequence,
            fragment_count: 1,
            response_sha256: sha256(proposal.proposal_sha256.as_bytes()),
            proposal: Some(proposal),
            failure: None,
            usage: usage(request, 0),
            resources: resources(request, 0),
        })
    }
}

impl RuntimeModelPort for EvidenceFixtureModel {
    fn exact_profile(&self) -> &ExactModelProfile {
        &self.profile
    }

    fn run_model(
        &mut self,
        request: &ModelRunRequest,
        context: &ModelContextPacket,
        cancellation: Option<&dyn ModelCancellationProbe>,
    ) -> Result<ModelRunResult, RuntimePortFailure> {
        // The development delay before each proposal; like a model
        // mid-generation, the fixture observes cancellation while it waits.
        let waited = std::time::Instant::now();
        while waited.elapsed() < self.step_delay {
            if cancellation.is_some_and(|probe| matches!(probe.observe(), Ok(Some(_)))) {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        if cancellation.is_some_and(|probe| matches!(probe.observe(), Ok(Some(_)) | Err(_))) {
            return Ok(cancelled(request, context.input_tokens));
        }
        self.calls = self
            .calls
            .checked_add(1)
            .ok_or(RuntimePortFailure::ResourceExhausted)?;
        if self.plan.is_none() {
            self.plan_from(context);
        }
        let planned = self
            .plan
            .as_ref()
            .and_then(|plan| plan.get(self.next))
            .map(|search| {
                (
                    search.source_id.clone(),
                    search.manifest_sha256.clone(),
                    search.term.clone(),
                )
            });
        if let Some((source_id, manifest_sha256, term)) = planned {
            self.next += 1;
            let arguments = serde_json::to_vec(&ArtifactRequest {
                schema_version: 1,
                call_id: format!("standalone-evidence-search-{}", self.next),
                source_id: Some(source_id),
                section_id: None,
                range: None,
                query: Some(term),
                freshness_sha256: Some(manifest_sha256),
                output_identity: format!("standalone-evidence-search-{}-output", self.next),
                limits: ArtifactLimits {
                    items: SEARCH_ITEMS,
                    output_bytes: SEARCH_OUTPUT_BYTES,
                    ..ArtifactLimits::default()
                },
                call_depth: 0,
            })
            .map_err(|_| RuntimePortFailure::Invalid)?;
            let candidate = ModelToolCallCandidate {
                tool_call_id: ToolCallId::from_raw(format!(
                    "standalone-evidence-call-{}",
                    self.next
                )),
                tool_id: self.search.tool_id.clone(),
                tool_version: ARTIFACT_TOOL_VERSION.to_owned(),
                arguments: ContractPayload {
                    schema: self.search.input_schema.clone(),
                    media_type: "application/json".to_owned(),
                    sha256: sha256(&arguments),
                    bytes: arguments,
                },
            };
            return self.proposal(request, ModelProposalKind::ToolCall, None, Some(candidate));
        }
        let bytes =
            serde_json::to_vec(&self.answer(context)).map_err(|_| RuntimePortFailure::Invalid)?;
        let payload = ContractPayload {
            schema: answer_schema(),
            media_type: STANDALONE_EVIDENCE_ANSWER_MEDIA_TYPE.to_owned(),
            sha256: sha256(&bytes),
            bytes,
        };
        self.proposal(
            request,
            ModelProposalKind::CompletionCandidate,
            Some(payload),
            None,
        )
    }
}

/// The question's search terms: words of at least four characters, as
/// written, that are not common words; at most four, first occurrence first.
#[must_use]
pub fn question_terms(question: &str) -> Vec<String> {
    let mut seen = BTreeSet::new();
    let mut terms = Vec::new();
    for word in
        question.split(|character: char| !(character.is_alphanumeric() || character == '\''))
    {
        let word = word.trim_matches('\'');
        if word.chars().count() < MIN_TERM_CHARS
            || STOP_WORDS.contains(&word.to_lowercase().as_str())
            || !seen.insert(word.to_lowercase())
        {
            continue;
        }
        terms.push(word.to_owned());
        if terms.len() == MAX_TERMS {
            break;
        }
    }
    terms
}

fn usage(request: &ModelRunRequest, input_tokens: u32) -> ModelTokenUsage {
    ModelTokenUsage {
        rendered_prompt_tokens: input_tokens.max(1),
        cached_input_tokens: None,
        evaluated_input_tokens: None,
        generated_output_tokens: 1,
        reasoning_output_tokens: None,
        output_token_reserve: request.max_output_tokens,
        remaining_capacity_tokens: None,
    }
}

fn resources(request: &ModelRunRequest, input_tokens: u32) -> ModelResourceReport {
    ModelResourceReport {
        adapter_id: request.adapter_id.clone(),
        profile_id: request.profile_id.clone(),
        model_run_id: Some(request.model_run_id.clone()),
        resident_memory_bytes: 1,
        accelerator_memory_bytes: 0,
        input_tokens: input_tokens.max(1),
        output_tokens: 1,
        elapsed_ms: 1,
    }
}

fn cancelled(request: &ModelRunRequest, input_tokens: u32) -> ModelRunResult {
    ModelRunResult {
        schema_version: CONTRACT_SCHEMA_VERSION,
        model_run_id: request.model_run_id.clone(),
        stream_id: ModelStreamId::from_raw("standalone-evidence-cancelled-stream"),
        correlation_id: request.correlation_id.clone(),
        terminal_state: ModelRunTerminalState::Cancelled,
        finish_reason: ModelFinishReason::Cancelled,
        fragment_count: 1,
        response_sha256: sha256(b"standalone-evidence-cancelled"),
        proposal: None,
        failure: Some(ModelRuntimeFailure {
            code: "runtime.model.cancelled".to_owned(),
            retryable_after_correction: false,
            dependency_recovery_required: false,
            contract_error: None,
        }),
        usage: ModelTokenUsage {
            generated_output_tokens: 0,
            ..usage(request, input_tokens)
        },
        resources: ModelResourceReport {
            output_tokens: 0,
            ..resources(request, input_tokens)
        },
    }
}

// ---------------------------------------------------------------------------
// Context port
// ---------------------------------------------------------------------------

/// The prepared-source context over the snapshot, with a content-free view of
/// every packet it returned.
pub struct EvidenceContext {
    inner: PreparedSourceRuntimeContext<EvidenceTokenCounter>,
    inspections: Vec<ContextInspection>,
    complete: bool,
}

impl RuntimeContextPort for EvidenceContext {
    fn build_context(
        &mut self,
        request: &RuntimeRunRequest,
        context_packet_id: ContextPacketId,
        turn: u32,
        completed_tool_calls: &[ToolCall],
        tool_results: &[ToolResult],
        evidence: &[EvidenceReference],
    ) -> Result<ModelContextPacket, RuntimePortFailure> {
        let packet = self.inner.build_context(
            request,
            context_packet_id,
            turn,
            completed_tool_calls,
            tool_results,
            evidence,
        )?;
        match self
            .inner
            .context_manifests()
            .last()
            .map(|manifest| inspect_context(&manifest.packet))
        {
            Some(Ok(inspection)) if self.inspections.len() < 64 => {
                self.inspections.push(inspection);
            }
            _ => self.complete = false,
        }
        Ok(packet)
    }
}

impl RunContextInspectionSource for EvidenceContext {
    fn run_context_inspections(&self) -> Option<&[ContextInspection]> {
        self.complete.then_some(self.inspections.as_slice())
    }
}

fn window_plan(
    profile: &ExactModelProfile,
) -> Result<ModelContextWindowPlan, RuntimeTransportError> {
    let partition = |tokens: u32| AllocatedContextPartition {
        requested_tokens: tokens,
        minimum_tokens: u32::from(tokens > 0),
        allocated_tokens: tokens,
        disposition: ContextPartitionDisposition::Included,
        reason_code: None,
    };
    let total = profile.context.max_context_tokens;
    let output = 4_096;
    let reserved = 1_024 * 4 + output;
    let mut plan = ModelContextWindowPlan {
        schema_version: CONTRACT_SCHEMA_VERSION,
        model_profile_id: profile.profile_id.as_str().to_owned(),
        model_manifest_sha256: profile.manifest_sha256.clone(),
        model_runtime_sha256: sha256(
            &serde_json::to_vec(&profile.runtime)
                .map_err(|_| RuntimeTransportError::RequestDenied)?,
        ),
        tokenizer_sha256: profile.codec.tokenizer_sha256.clone(),
        token_counter_sha256: profile.context.token_counter_sha256.clone(),
        total_window_tokens: total,
        system_and_tool_tokens: 1_024,
        user_input_tokens: 1_024,
        source_artifacts: partition(total.saturating_sub(reserved) / 2),
        retrieved_context: partition(0),
        workflow_recovery_reserve_tokens: 1_024,
        output_reserve_tokens: output,
        safety_margin_tokens: 1_024,
        unallocated_tokens: total
            .saturating_sub(reserved)
            .saturating_sub(total.saturating_sub(reserved) / 2),
        plan_sha256: ZERO_SHA256.to_owned(),
    };
    plan.plan_sha256 =
        sha256(&serde_json::to_vec(&plan).map_err(|_| RuntimeTransportError::RequestDenied)?);
    Ok(plan)
}

// ---------------------------------------------------------------------------
// Tool boundary
// ---------------------------------------------------------------------------

/// Read-only artifact calls over the snapshot. Selecting the folder is the
/// read authorization; any other tool is denied.
pub struct EvidenceToolBoundary {
    sources: Arc<SourcePreparationService>,
    admission_sha256: String,
    ledger: ArtifactAttemptLedger,
    executions: u32,
    // The run's development store, which keeps its job; held for the run.
    _store: Option<LinuxAuthorityRuntime>,
}

impl EvidenceToolBoundary {
    fn artifact_kind(definition: &ToolDefinition, call: &ToolCall) -> Option<ArtifactToolKind> {
        ArtifactToolKind::ALL.into_iter().find(|kind| {
            kind.id() == call.tool_id.as_str()
                && definition.tool_id == call.tool_id
                && call.tool_version == ARTIFACT_TOOL_VERSION
        })
    }

    fn authority(&self, request: &RuntimeRunRequest, call: &ToolCall) -> String {
        sha256(
            format!(
                "{}\n{}\n{}\n{}",
                self.admission_sha256,
                request.run_id.as_str(),
                call.tool_call_id.as_str(),
                call.arguments.sha256
            )
            .as_bytes(),
        )
    }
}

impl RuntimeToolBoundary for EvidenceToolBoundary {
    fn evaluate(
        &mut self,
        request: &RuntimeRunRequest,
        _operation_id: &RuntimeOperationId,
        definition: &ToolDefinition,
        call: &ToolCall,
        now_epoch_ms: u64,
    ) -> Result<RuntimePermissionEvaluation, RuntimePortFailure> {
        let authority = self.authority(request, call);
        let approval_id =
            ApprovalId::from_raw(format!("standalone-evidence-decision-{}", &authority[..24]));
        let grant_id = GrantId::from_raw(format!("standalone-evidence-grant-{}", &authority[..24]));
        let preview_sha256 = sha256(&call.arguments.bytes);
        let expires_at_epoch_ms = now_epoch_ms.saturating_add(60_000);
        if Self::artifact_kind(definition, call).is_none()
            || request.mode != RuntimeSessionMode::EphemeralReadOnly
        {
            return Ok(RuntimePermissionEvaluation::Deny {
                approval_id,
                grant_id,
                preview_sha256,
                expires_at_epoch_ms,
                decision_sha256: sha256(format!("deny\n{authority}").as_bytes()),
                reason_code: "standalone.evidence.tool-denied".to_owned(),
            });
        }
        Ok(RuntimePermissionEvaluation::Allow {
            approval_id,
            preview_sha256,
            expires_at_epoch_ms,
            grant_id,
            decision_sha256: sha256(format!("allow\n{authority}").as_bytes()),
            authority_sha256: authority,
        })
    }

    fn resolve(
        &mut self,
        _request: &RuntimeRunRequest,
        _challenge: &RuntimeApprovalChallenge,
        _response: &RuntimeApprovalResponse,
        _definition: &ToolDefinition,
        _call: &ToolCall,
        _now_epoch_ms: u64,
    ) -> Result<RuntimePermissionEvaluation, RuntimePortFailure> {
        // No evidence call ever asks a person; there is nothing to resolve.
        Err(RuntimePortFailure::Invalid)
    }

    fn execute(
        &mut self,
        request: &RuntimeRunRequest,
        evaluation: &RuntimePermissionEvaluation,
        definition: &ToolDefinition,
        call: &ToolCall,
        cancellation: Option<&dyn ModelCancellationProbe>,
    ) -> Result<RuntimeToolExecution, RuntimePortFailure> {
        let kind = Self::artifact_kind(definition, call).ok_or(RuntimePortFailure::Invalid)?;
        let RuntimePermissionEvaluation::Allow {
            authority_sha256, ..
        } = evaluation
        else {
            return Err(RuntimePortFailure::Invalid);
        };
        if *authority_sha256 != self.authority(request, call) {
            return Err(RuntimePortFailure::Invalid);
        }
        let signal = if cancellation.is_some_and(|probe| matches!(probe.observe(), Ok(Some(_)))) {
            ArtifactExecutionSignal::Cancelled
        } else {
            ArtifactExecutionSignal::Continue
        };
        let result = dispatch_native_source_artifact(
            kind,
            &call.arguments.bytes,
            &self.sources,
            true,
            signal,
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
                "standalone-evidence-{}-{:04}",
                &self.admission_sha256[..16],
                self.executions
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
                "standalone-evidence-receipt-{}-{:04}",
                &self.admission_sha256[..16],
                self.executions
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

impl RuntimeArtifactAccessPort for EvidenceToolBoundary {
    fn read_runtime_artifact_page(
        &mut self,
        _request: &RuntimeRunRequest,
        _reference: &RuntimeArtifactRef,
        _offset: u64,
        _maximum_bytes: u32,
        _now_epoch_ms: u64,
    ) -> Result<RuntimeArtifactPage, RuntimePortFailure> {
        // Evidence runs retain no artifact store.
        Err(RuntimePortFailure::Unavailable)
    }

    fn release_runtime_artifact(
        &mut self,
        _request: &RuntimeRunRequest,
        _reference: &RuntimeArtifactRef,
        _now_epoch_ms: u64,
    ) -> Result<RuntimeArtifactState, RuntimePortFailure> {
        Err(RuntimePortFailure::Unavailable)
    }
}

impl RunRecoverabilitySource for EvidenceToolBoundary {
    fn declare_run_recoverability(
        &self,
        request: &RuntimeRunRequest,
    ) -> Result<RecoverabilityReport, RecoverabilityError> {
        // A read-only run makes no effect, so it declares none.
        assess_run_recoverability(
            request.session_id.as_str(),
            request.task.task_id.as_str(),
            request.run_id.as_str(),
            &[],
            &|_| None,
        )
    }

    fn declare_session_recoverability(
        &self,
        request: &RuntimeRunRequest,
        _now_epoch_ms: u64,
    ) -> Result<RecoverabilityReport, RecoverabilityError> {
        assess_recoverability(
            request.session_id.as_str(),
            request.task.task_id.as_str(),
            &[],
            &|_| None,
        )
    }
}

impl RunActionHistorySource for EvidenceToolBoundary {
    fn declare_run_action_history(&self, _request: &RuntimeRunRequest) -> Option<RunActionHistory> {
        // Evidence runs keep no action chain; the run's events record each call.
        None
    }
}

// ---------------------------------------------------------------------------
// Verifier
// ---------------------------------------------------------------------------

/// Admits success only for a closed answer whose every statement cites
/// evidence observed in this run, with every quote exact.
pub struct EvidenceAnswerVerifier {
    verifier_id: VerifierId,
    records: u32,
}

impl EvidenceAnswerVerifier {
    fn new() -> Self {
        Self {
            verifier_id: VerifierId::from_raw("standalone-evidence-citation-verifier-v1"),
            records: 0,
        }
    }
}

/// Checks one answer against the run's observed evidence and tool results.
/// Returns the cited evidence, or a stable code. It checks citation
/// membership and exact quotes, not that the text answers the question.
pub fn check_evidence_answer(
    payload: Option<&ContractPayload>,
    evidence: &[EvidenceReference],
    tool_results: &[ToolResult],
) -> Result<Vec<EvidenceReference>, &'static str> {
    let payload = payload.ok_or("standalone.evidence.answer-absent")?;
    if payload.media_type != STANDALONE_EVIDENCE_ANSWER_MEDIA_TYPE
        || payload.schema != answer_schema()
        || payload.sha256 != sha256(&payload.bytes)
    {
        return Err("standalone.evidence.answer-schema");
    }
    let answer: EvidenceAnswer =
        serde_json::from_slice(&payload.bytes).map_err(|_| "standalone.evidence.answer-schema")?;
    if answer.schema_version != 1 {
        return Err("standalone.evidence.answer-schema");
    }
    if answer.statements.is_empty() || answer.no_matching_text {
        return Err("standalone.evidence.no-cited-statement");
    }
    let mut cited = Vec::new();
    for statement in &answer.statements {
        if statement.text.trim().is_empty() || statement.citations.is_empty() {
            return Err("standalone.evidence.uncited-statement");
        }
        for citation in &statement.citations {
            let reference = evidence
                .iter()
                .find(|reference| reference.evidence_id.as_str() == citation.evidence_id)
                .ok_or("standalone.evidence.citation-unobserved")?;
            let fragment = tool_results
                .iter()
                .filter(|result| {
                    result
                        .evidence
                        .iter()
                        .any(|item| item.evidence_id == reference.evidence_id)
                })
                .filter_map(|result| result.output.as_ref())
                .filter_map(|output| serde_json::from_slice::<ArtifactResult>(&output.bytes).ok())
                .filter(|result| result.source_id.as_deref() == Some(citation.source_id.as_str()))
                .flat_map(|result| result.items)
                .find_map(|item| match item {
                    ArtifactItem::SearchHit {
                        fragment:
                            ArtifactFragment::Line {
                                start,
                                end_exclusive,
                                content,
                            },
                        ..
                    }
                    | ArtifactItem::Content {
                        fragment:
                            ArtifactFragment::Line {
                                start,
                                end_exclusive,
                                content,
                            },
                    } if start == citation.start_line
                        && end_exclusive == citation.end_line_exclusive =>
                    {
                        Some(content)
                    }
                    _ => None,
                })
                .ok_or("standalone.evidence.citation-range")?;
            if statement
                .quote
                .as_deref()
                .is_some_and(|quote| quote.is_empty() || !fragment.contains(quote))
            {
                return Err("standalone.evidence.quote-mismatch");
            }
            if !cited
                .iter()
                .any(|item: &EvidenceReference| item.evidence_id == reference.evidence_id)
            {
                cited.push(reference.clone());
            }
        }
    }
    Ok(cited)
}

impl RuntimeVerifierPort for EvidenceAnswerVerifier {
    fn verifier_id(&self) -> &VerifierId {
        &self.verifier_id
    }

    fn verify(
        &mut self,
        input: RuntimeVerificationInput<'_>,
    ) -> Result<VerifierCandidate, RuntimePortFailure> {
        self.records = self
            .records
            .checked_add(1)
            .ok_or(RuntimePortFailure::ResourceExhausted)?;
        let checked = check_evidence_answer(
            input.proposal.payload.as_ref(),
            input.evidence,
            input.tool_results,
        );
        let (disposition, cited) = match checked {
            Ok(cited) if !cited.is_empty() => (VerifierDisposition::Success, cited),
            _ => (VerifierDisposition::Failed, Vec::new()),
        };
        Ok(VerifierCandidate {
            schema_version: CONTRACT_SCHEMA_VERSION,
            verifier_record_id: VerifierRecordId::from_raw(format!(
                "standalone-evidence-verification-{}-{}",
                input.request.run_id.as_str(),
                self.records
            )),
            verifier_id: self.verifier_id.clone(),
            task_id: input.request.task.task_id.clone(),
            proposal_id: input.proposal.proposal_id.clone(),
            repository_snapshot_id: input.request.repository_snapshot_id.clone(),
            state_revision: input.state_revision,
            source: VerifierSource::DeterministicPostcondition,
            disposition,
            postconditions: input
                .postconditions
                .iter()
                .map(|postcondition_id| PostconditionResult {
                    postcondition_id: postcondition_id.clone(),
                    passed: disposition == VerifierDisposition::Success,
                    evidence: cited.clone(),
                })
                .collect(),
        })
    }
}

// ---------------------------------------------------------------------------
// Runtime factory
// ---------------------------------------------------------------------------

/// Wall clock for trusted deadlines and event times.
pub struct EvidenceClock;

impl RuntimeClock for EvidenceClock {
    fn now_epoch_ms(&mut self) -> Result<u64, RuntimePortFailure> {
        now_epoch_ms().ok_or(RuntimePortFailure::Unavailable)
    }
}

/// The coordinator one evidence question runs on.
pub type StandaloneEvidenceCoordinator = ReusableRuntimeCoordinator<
    EvidenceFixtureModel,
    EvidenceContext,
    EvidenceToolBoundary,
    EvidenceAnswerVerifier,
    EvidenceClock,
>;

/// State the service and its factory share: the current snapshot and the
/// runs prepared or running over it.
#[derive(Default)]
pub struct EvidenceShared {
    snapshot: Option<Arc<FolderSnapshot>>,
    in_flight: BTreeSet<String>,
}

/// Frames and composes read-only evidence runs over the current snapshot.
pub struct StandaloneEvidenceRuntimeFactory {
    activation: StandaloneEvidenceActivation,
    shared: Arc<Mutex<EvidenceShared>>,
    platform: LinuxDevelopmentPlatformAdapter,
    composed_job_ledgers: Option<(RuntimeRunId, DurableJobLedgers)>,
    profile: ExactModelProfile,
    session_id: Option<SessionId>,
    sequence: u64,
    prepared: BTreeMap<String, (RuntimeRunRequest, Arc<FolderSnapshot>)>,
    step_delay: std::time::Duration,
}

impl StandaloneEvidenceRuntimeFactory {
    fn new(
        activation: StandaloneEvidenceActivation,
        shared: Arc<Mutex<EvidenceShared>>,
        step_delay: std::time::Duration,
    ) -> Result<Self, RuntimeTransportError> {
        let activation_sha256 = sha256(
            format!(
                "{STANDALONE_EVIDENCE_ACTIVATION}\n{}\n{}",
                activation.state_root().display(),
                activation.disposable_root().display()
            )
            .as_bytes(),
        );
        let platform = LinuxDevelopmentPlatformAdapter::activate(
            STANDALONE_EVIDENCE_ACTIVATION,
            activation_sha256.clone(),
            agentmage_kernel_contracts::AdapterInstanceId::from_raw(format!(
                "standalone-evidence-{}",
                &activation_sha256[..24]
            )),
        )
        .map_err(|_| RuntimeTransportError::RequestDenied)?;
        Ok(Self {
            activation,
            shared,
            platform,
            composed_job_ledgers: None,
            profile: standalone_evidence_profile()?,
            session_id: None,
            sequence: 0,
            prepared: BTreeMap::new(),
            step_delay,
        })
    }

    fn next_identity(&mut self, prefix: &str) -> String {
        self.sequence += 1;
        format!(
            "{prefix}-{}-{}",
            now_epoch_ms().unwrap_or_default(),
            self.sequence
        )
    }
}

impl NativeChatRuntimeFactory for StandaloneEvidenceRuntimeFactory {
    type Coordinator = StandaloneEvidenceCoordinator;

    fn prepare_runtime_request(
        &mut self,
        input: &RuntimePrepareInput,
    ) -> Result<RuntimeRunRequest, RuntimeTransportError> {
        self.activation
            .revalidate()
            .map_err(|_| RuntimeTransportError::RequestDenied)?;
        let snapshot = {
            let shared = self
                .shared
                .lock()
                .map_err(|_| RuntimeTransportError::RuntimeFailed)?;
            // A prepared run the service released without starting is gone.
            self.prepared
                .retain(|run_id, _| shared.in_flight.contains(run_id));
            shared
                .snapshot
                .clone()
                .ok_or(RuntimeTransportError::RequestDenied)?
        };
        let view = snapshot.view();
        if input.resume
            || input.record_session
            || input.slow_subscriber_probe
            || input.preauthorization.is_some()
            || input.recipe.is_some()
            || input.profile_id != STANDALONE_EVIDENCE_PROFILE_ID
            || input.expected_entry_sha256 != view.admission_sha256
            || input.workspace_id != view.workspace_id
            || input.workspace_root != view.folder
            || input.prompt.trim().is_empty()
            || input.prompt.len() > MAX_QUESTION_BYTES
            || view.accepted == 0
            || !self.prepared.is_empty()
        {
            return Err(RuntimeTransportError::RequestDenied);
        }
        let session_id = match (&self.session_id, &input.engineering_session_id) {
            (None, None) => {
                let session_id =
                    SessionId::from_raw(self.next_identity("standalone-evidence-session"));
                self.session_id = Some(session_id.clone());
                session_id
            }
            (Some(current), Some(requested)) if current == requested => current.clone(),
            _ => return Err(RuntimeTransportError::RequestDenied),
        };
        let run_id = RuntimeRunId::from_raw(self.next_identity("standalone-evidence-run"));
        let task_id = TaskId::from_raw(format!("{}-task", run_id.as_str()));
        let registry = standalone_evidence_registry()?;
        let tool_catalog_id = ToolCatalogId::from_raw("standalone-evidence-artifact-catalog-v1");
        let visible_tools =
            runtime_tool_references(&registry).map_err(|_| RuntimeTransportError::RequestDenied)?;
        let snapshot_id = format!("folder-{}", &view.admission_sha256[..24]);
        let acceptance =
            vec!["Every answer statement cites search evidence observed in this run".to_owned()];
        let request = seal_runtime_run_request(RuntimeRunRequest {
            schema_version: CONTRACT_SCHEMA_VERSION,
            run_id: run_id.clone(),
            session_id: session_id.clone(),
            mode: RuntimeSessionMode::EphemeralReadOnly,
            task: Task {
                schema_version: CONTRACT_SCHEMA_VERSION,
                task_id: task_id.clone(),
                session_id,
                objective: input.prompt.clone(),
                acceptance_criteria: acceptance.clone(),
                constraints: vec![
                    "Read only: the admitted folder snapshot".to_owned(),
                    "No network, writes or commands".to_owned(),
                    format!(
                        "Model {STANDALONE_EVIDENCE_PROFILE_ID} is a term-match fixture, not a language model"
                    ),
                ],
                status: TaskStatus::Ready,
            },
            work_packet: work_packet(&task_id, &input.prompt, &acceptance),
            workspace_id: WorkspaceId::from_raw(view.workspace_id.clone()),
            workspace_snapshot_sha256: view.admission_sha256.clone(),
            repository_snapshot_id: RepositorySnapshotId::from_raw(snapshot_id),
            repository_snapshot_sha256: view.admission_sha256.clone(),
            context_budget: self.profile.context.clone(),
            model_profile: self.profile.clone(),
            tool_catalog_sha256: runtime_tool_catalog_sha256(&tool_catalog_id, &visible_tools)
                .map_err(|_| RuntimeTransportError::RequestDenied)?,
            tool_catalog_id,
            visible_tools,
            policy_id: PolicyId::from_raw("standalone-evidence-read-policy-v1"),
            policy_sha256: sha256(
                format!("{STANDALONE_EVIDENCE_ACTIVATION}\n{}", view.admission_sha256).as_bytes(),
            ),
            limits: RuntimeRunLimits {
                max_turns: 24,
                max_model_calls: 24,
                max_tool_calls: 20,
                max_repeated_tool_calls: 2,
                max_tool_call_depth: 1,
                max_no_progress_turns: 24,
                max_context_refreshes: 24,
                max_events: 1_024,
                max_elapsed_ms: 120_000,
                max_output_bytes: 64 * 1024,
            },
            event_cursor: None,
            request_sha256: ZERO_SHA256.to_owned(),
        })
        .map_err(|_| RuntimeTransportError::RequestDenied)?;
        self.shared
            .lock()
            .map_err(|_| RuntimeTransportError::RuntimeFailed)?
            .in_flight
            .insert(run_id.as_str().to_owned());
        self.prepared.insert(
            run_id.as_str().to_owned(),
            (request.clone(), Arc::clone(&snapshot)),
        );
        Ok(request)
    }

    fn compose_runtime(
        &mut self,
        request: &RuntimeRunRequest,
    ) -> Result<Self::Coordinator, RuntimeTransportError> {
        let (prepared, snapshot) = self
            .prepared
            .remove(request.run_id.as_str())
            .ok_or(RuntimeTransportError::RequestDenied)?;
        let current = self
            .shared
            .lock()
            .map_err(|_| RuntimeTransportError::RuntimeFailed)?
            .snapshot
            .clone();
        if prepared != *request || !current.is_some_and(|current| Arc::ptr_eq(&current, &snapshot))
        {
            return Err(RuntimeTransportError::RequestDenied);
        }
        self.activation
            .revalidate()
            .map_err(|_| RuntimeTransportError::RequestDenied)?;
        // Decision 0120: over the channel a run is cancelled only through its
        // job's ledger, so each run is one job in this host's development
        // store. The store holds job control records, never folder content.
        self.composed_job_ledgers = None;
        let mut key =
            crate::coding_development_activation::CodingDevelopmentKeyProvider::open_in_state_root(
                self.activation.state_root(),
            )
            .map_err(|_| RuntimeTransportError::RuntimeFailed)?;
        let store = open_linux_development_authority(
            &self.platform,
            self.activation.state_root(),
            &mut key,
            now_epoch_ms().ok_or(RuntimeTransportError::RuntimeFailed)?,
        )
        .map_err(|_| {
            eprintln!("standalone.evidence.store-unavailable");
            RuntimeTransportError::RuntimeFailed
        })?;
        let job_ledgers = store.authority().job_ledgers();
        let context = EvidenceContext {
            inner: PreparedSourceRuntimeContext::new(
                Arc::clone(&snapshot.sources),
                window_plan(&self.profile)?,
                EvidenceTokenCounter::for_profile(&self.profile),
                [FOLDER_INVENTORY_SOURCE_ID.to_owned()],
                false,
            )
            .map_err(|_| RuntimeTransportError::RequestDenied)?,
            inspections: Vec::new(),
            complete: true,
        };
        ReusableRuntimeCoordinator::new(
            request.clone(),
            EvidenceFixtureModel::new(self.profile.clone(), self.step_delay),
            context,
            standalone_evidence_registry()?,
            EvidenceToolBoundary {
                sources: Arc::clone(&snapshot.sources),
                admission_sha256: snapshot.view().admission_sha256.clone(),
                ledger: ArtifactAttemptLedger::default(),
                executions: 0,
                _store: Some(store),
            },
            EvidenceAnswerVerifier::new(),
            EvidenceClock,
        )
        .inspect(|_| {
            self.composed_job_ledgers = Some((request.run_id.clone(), job_ledgers));
        })
        .map_err(|_| RuntimeTransportError::RequestDenied)
    }

    fn take_job_ledgers(&mut self, run_id: &RuntimeRunId) -> Option<DurableJobLedgers> {
        match self.composed_job_ledgers.take() {
            Some((composed, ledgers)) if composed == *run_id => Some(ledgers),
            _ => None,
        }
    }
}

fn work_packet(task_id: &TaskId, question: &str, acceptance: &[String]) -> WorkPacket {
    WorkPacket {
        schema_version: CONTRACT_SCHEMA_VERSION,
        work_packet_id: WorkPacketId::from_raw(format!("{}-packet", task_id.as_str())),
        task_id: task_id.clone(),
        revision: 1,
        objective: question.to_owned(),
        reason: "Answer one question from the admitted folder snapshot".to_owned(),
        owner: "standalone-evidence-user".to_owned(),
        authoritative_evidence: Vec::new(),
        mutable_files: Vec::new(),
        protected_files: Vec::new(),
        expected_output: "Cited statements, or a truthful non-success".to_owned(),
        acceptance_checks: acceptance.to_vec(),
        required_evidence: vec![EvidenceKind::ToolOutput],
        required_capability_class: AuthorityClass::Observe,
        budgets: vec![
            BudgetLimit {
                resource: BudgetResource::PlanSteps,
                limit: 24,
            },
            BudgetLimit {
                resource: BudgetResource::ModelCalls,
                limit: 24,
            },
            BudgetLimit {
                resource: BudgetResource::ToolCalls,
                limit: 20,
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
        last_verification_date: "2026-10-04".to_owned(),
        next_action: None,
        next_review: None,
        status_reason: None,
        disposition: None,
        completion_evidence: Vec::new(),
        superseding_work: None,
        validation_issues: Vec::new(),
        plan_id: Some(PlanId::from_raw(format!("{}-plan", task_id.as_str()))),
        state: WorkPacketState::Active,
    }
}

// ---------------------------------------------------------------------------
// Service
// ---------------------------------------------------------------------------

/// What the evidence host serves: folder admission and read-only runs over
/// the shared live runtime service.
pub struct StandaloneEvidenceService {
    activation: StandaloneEvidenceActivation,
    shared: Arc<Mutex<EvidenceShared>>,
    profile: ExactModelProfile,
    runs: LiveCodingRuntimeService<StandaloneEvidenceRuntimeFactory>,
}

impl StandaloneEvidenceService {
    /// Composes the service for one validated activation. The fixture waits
    /// `step_delay`, at most two seconds, before each proposal, so a
    /// cancellation can be exercised between processes.
    pub fn new(
        activation: StandaloneEvidenceActivation,
        step_delay: std::time::Duration,
    ) -> Result<Self, RuntimeTransportError> {
        if step_delay > MAX_FIXTURE_STEP_DELAY {
            return Err(RuntimeTransportError::RequestDenied);
        }
        let shared = Arc::new(Mutex::new(EvidenceShared::default()));
        let factory = StandaloneEvidenceRuntimeFactory::new(
            activation.clone(),
            Arc::clone(&shared),
            step_delay,
        )?;
        Ok(Self {
            activation,
            shared,
            profile: standalone_evidence_profile()?,
            runs: LiveCodingRuntimeService::new(factory),
        })
    }

    fn admit(&mut self, folder: &str) -> FolderAnswer {
        let refused = |refusal| FolderAnswer::Refused { refusal };
        let Ok(shared) = self.shared.lock() else {
            return refused(FolderRefusal::Failed);
        };
        if !shared.in_flight.is_empty() {
            return refused(FolderRefusal::Busy);
        }
        drop(shared);
        let Some(now) = now_epoch_ms() else {
            return refused(FolderRefusal::Failed);
        };
        match admit_folder(&self.activation, folder, &self.profile, now) {
            Ok(snapshot) => {
                let admission = snapshot.view().clone();
                let Ok(mut shared) = self.shared.lock() else {
                    return refused(FolderRefusal::Failed);
                };
                if !shared.in_flight.is_empty() {
                    return refused(FolderRefusal::Busy);
                }
                shared.snapshot = Some(Arc::new(snapshot));
                FolderAnswer::Admitted { admission }
            }
            Err(refusal) => refused(refusal),
        }
    }

    fn finished(&self, run_id: &RuntimeRunId) {
        if let Ok(mut shared) = self.shared.lock() {
            shared.in_flight.remove(run_id.as_str());
        }
    }
}

impl RuntimeTransportPort for StandaloneEvidenceService {
    fn prepare(
        &mut self,
        input: RuntimePrepareInput,
    ) -> Result<RuntimeRunRequest, RuntimeTransportError> {
        self.runs.prepare(input)
    }

    fn start(
        &mut self,
        request: RuntimeRunRequest,
    ) -> Result<RuntimeTransportStep, RuntimeTransportError> {
        self.runs.start(request)
    }

    fn advance(
        &mut self,
        run_id: &RuntimeRunId,
        request_sha256: &str,
        after_event_cursor: Option<&RuntimeEventCursor>,
        response: Option<&RuntimeApprovalResponse>,
    ) -> Result<RuntimeTransportStep, RuntimeTransportError> {
        self.runs
            .advance(run_id, request_sha256, after_event_cursor, response)
    }

    fn cancel(
        &mut self,
        run_id: &RuntimeRunId,
        request_sha256: &str,
        cancellation_id: CancellationId,
        after_event_cursor: Option<&RuntimeEventCursor>,
    ) -> Result<RuntimeTransportStep, RuntimeTransportError> {
        self.runs
            .cancel(run_id, request_sha256, cancellation_id, after_event_cursor)
    }

    fn read_artifact_page(
        &mut self,
        run_id: &RuntimeRunId,
        request_sha256: &str,
        reference: &RuntimeArtifactRef,
        offset: u64,
        maximum_bytes: u32,
    ) -> Result<RuntimeArtifactPage, RuntimeTransportError> {
        self.runs
            .read_artifact_page(run_id, request_sha256, reference, offset, maximum_bytes)
    }

    fn release_artifact(
        &mut self,
        run_id: &RuntimeRunId,
        request_sha256: &str,
        reference: &RuntimeArtifactRef,
    ) -> Result<RuntimeArtifactState, RuntimeTransportError> {
        self.runs
            .release_artifact(run_id, request_sha256, reference)
    }

    fn run_declarations(
        &mut self,
        run_id: &RuntimeRunId,
        request_sha256: &str,
    ) -> Result<RuntimeRunDeclarations, RuntimeTransportError> {
        self.runs.run_declarations(run_id, request_sha256)
    }

    fn job_status(
        &mut self,
        run_id: &RuntimeRunId,
        request_sha256: &str,
    ) -> Result<RuntimeJobStatus, RuntimeTransportError> {
        self.runs.job_status(run_id, request_sha256)
    }

    fn control_job_for_client(
        &mut self,
        client: &RuntimeClientScope,
        run_id: &RuntimeRunId,
        request_sha256: &str,
        request: &JobControlRequest,
    ) -> Result<RuntimeJobControl, RuntimeTransportError> {
        self.runs
            .control_job_for_client(client, run_id, request_sha256, request)
    }

    fn folder(&mut self, request: FolderRequest) -> Result<FolderAnswer, RuntimeTransportError> {
        match request {
            FolderRequest::Admit { folder } => Ok(self.admit(&folder)),
        }
    }

    fn release(
        &mut self,
        run_id: &RuntimeRunId,
        request_sha256: &str,
    ) -> Result<(), RuntimeTransportError> {
        let result = self.runs.release(run_id, request_sha256);
        // Released, or no longer held at all (its start failed): either way
        // it no longer pins the snapshot.
        if matches!(result, Ok(()) | Err(RuntimeTransportError::RunUnavailable)) {
            self.finished(run_id);
        }
        result
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn now_epoch_ms() -> Option<u64> {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()
        .and_then(|elapsed| u64::try_from(elapsed.as_millis()).ok())
        .filter(|value| *value > 0)
}

fn sha256(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[cfg(test)]
#[path = "standalone_evidence_tests.rs"]
mod tests;
