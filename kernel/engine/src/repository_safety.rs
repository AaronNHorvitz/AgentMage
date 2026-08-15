//! Exact repository plans, preservation manifests, and owned-worktree lifecycle.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use agentmage_kernel_contracts::{
    GrantOperation, HeldWorkspaceObject, OperationOutcome, StateChange,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::authority_transaction::{EffectAuthorization, EffectDriver, EffectLaunch, EffectResult};
use crate::propagation::CancellationToken;

const SCHEMA_VERSION: u16 = 1;
const ZERO_SHA256: &str = "0000000000000000000000000000000000000000000000000000000000000000";
const MAX_IDENTIFIER_BYTES: usize = 128;
const MAX_HOST_BYTES: usize = 253;
const MAX_REPOSITORY_BYTES: usize = 512;
const MAX_REF_BYTES: usize = 512;
const MAX_WORKTREES: usize = 256;
const MAX_CHANGED_PATHS: usize = 4_096;
const MAX_GRANTS: usize = 64;

/// Stable reason repository planning or reconciliation cannot advance.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RepositorySafetyError {
    /// A field is malformed, ambiguous, unsupported, or outside a fixed bound.
    InvalidInput,
    /// The remote is not canonical HTTPS or SSH for the exact approved host.
    RemoteDenied,
    /// A ref is not one exact full branch ref in an AgentMage-admitted namespace.
    RefDenied,
    /// A manifest is malformed, stale, hazardous, or does not match its digest.
    ManifestDenied,
    /// State changed outside the operation's exact owned surface.
    PreservationMismatch,
    /// A path is stale, colliding, renamed unexpectedly, or not task-owned.
    Collision,
    /// A worktree record is absent, duplicated, dirty, active, or not removable.
    WorktreeDenied,
    /// A local branch update is not an exact proven compare-and-swap fast-forward.
    FastForwardDenied,
    /// A platform result or receipt is internally inconsistent.
    ResultDenied,
}

impl RepositorySafetyError {
    /// Returns a stable content-free failure code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidInput => "repository.input.invalid",
            Self::RemoteDenied => "repository.remote.denied",
            Self::RefDenied => "repository.ref.denied",
            Self::ManifestDenied => "repository.manifest.denied",
            Self::PreservationMismatch => "repository.preservation.mismatch",
            Self::Collision => "repository.transfer.collision",
            Self::WorktreeDenied => "repository.worktree.denied",
            Self::FastForwardDenied => "repository.fast_forward.denied",
            Self::ResultDenied => "repository.result.denied",
        }
    }
}

impl fmt::Display for RepositorySafetyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.code())
    }
}

impl std::error::Error for RepositorySafetyError {}

/// Closed network protocol admitted by remote repository reads.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GitRemoteProtocol {
    /// TLS-authenticated HTTPS.
    Https,
    /// SSH with a separately pinned host identity.
    Ssh,
}

/// Canonical, credential-free identity of one approved remote repository.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GitRemoteIdentity {
    /// Exact admitted transport.
    pub protocol: GitRemoteProtocol,
    /// Lowercase DNS host, with an optional explicit numeric port.
    pub host: String,
    /// Exact `owner/repository` path without a trailing `.git` suffix.
    pub repository: String,
    /// Canonical credential-free transport URL.
    pub canonical_url: String,
    /// SHA-256 over the canonical URL.
    pub identity_sha256: String,
}

impl GitRemoteIdentity {
    /// Parses one strict HTTPS or SSH URL and binds it to one exact approved host.
    pub fn parse(url: &str, approved_host: &str) -> Result<Self, RepositorySafetyError> {
        if url.len() > 1_024 || !valid_host(approved_host) {
            return Err(RepositorySafetyError::RemoteDenied);
        }
        let (protocol, host, path) = if let Some(rest) = url.strip_prefix("https://") {
            if rest.contains('@') {
                return Err(RepositorySafetyError::RemoteDenied);
            }
            let (host, path) = rest
                .split_once('/')
                .ok_or(RepositorySafetyError::RemoteDenied)?;
            (GitRemoteProtocol::Https, host, path)
        } else if let Some(rest) = url.strip_prefix("ssh://git@") {
            let (host, path) = rest
                .split_once('/')
                .ok_or(RepositorySafetyError::RemoteDenied)?;
            (GitRemoteProtocol::Ssh, host, path)
        } else {
            return Err(RepositorySafetyError::RemoteDenied);
        };
        if host != approved_host || host.to_ascii_lowercase() != host || !valid_host(host) {
            return Err(RepositorySafetyError::RemoteDenied);
        }
        let repository = path.strip_suffix(".git").unwrap_or(path);
        if !valid_repository_path(repository) {
            return Err(RepositorySafetyError::RemoteDenied);
        }
        let canonical_url = match protocol {
            GitRemoteProtocol::Https => format!("https://{host}/{repository}.git"),
            GitRemoteProtocol::Ssh => format!("ssh://git@{host}/{repository}.git"),
        };
        Ok(Self {
            protocol,
            host: host.to_owned(),
            repository: repository.to_owned(),
            identity_sha256: sha256_hex(canonical_url.as_bytes()),
            canonical_url,
        })
    }

    /// Revalidates canonical representation and identity.
    pub fn verify(&self) -> Result<(), RepositorySafetyError> {
        let parsed = Self::parse(&self.canonical_url, &self.host)?;
        if parsed != *self {
            return Err(RepositorySafetyError::RemoteDenied);
        }
        Ok(())
    }
}

/// High-level state of one local branch relative to one exact fetched object.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RepositoryCurrentness {
    /// Both identities are equal and the observation is within its freshness bound.
    Current,
    /// The local branch contains commits beyond the fetched object.
    Ahead,
    /// The fetched object contains commits beyond the local branch.
    Behind,
    /// Both sides contain unique commits.
    Diverged,
    /// The observation exceeded its declared age bound.
    Stale,
    /// Worktree or index changes are present.
    Dirty,
    /// Untracked paths are present.
    Untracked,
    /// No local branch object exists yet.
    Unborn,
}

/// Trusted, bounded observations used to derive a currentness report.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CurrentnessObservation {
    /// Exact canonical remote identity.
    pub remote: GitRemoteIdentity,
    /// Exact full default branch ref.
    pub default_branch: String,
    /// Exact full local branch ref when one exists.
    pub local_branch: Option<String>,
    /// Exact full upstream ref when configured.
    pub upstream: Option<String>,
    /// Current local object identity when one exists.
    pub local_object: Option<String>,
    /// Exact fetched object identity.
    pub fetched_object: String,
    /// Count of local-only commits computed by a bounded graph query.
    pub ahead_count: u32,
    /// Count of remote-only commits computed by a bounded graph query.
    pub behind_count: u32,
    /// Whether staged or unstaged changes exist.
    pub dirty: bool,
    /// Whether untracked paths exist.
    pub untracked: bool,
    /// Kernel timestamp at which fetch evidence was observed.
    pub fetched_at_epoch_ms: u64,
    /// Kernel timestamp at which the report is built.
    pub observed_at_epoch_ms: u64,
    /// Maximum accepted age of fetch evidence.
    pub max_age_ms: u64,
    /// SHA-256 of the exact bounded fetch evidence.
    pub fetch_evidence_sha256: String,
}

/// Content-minimized, hash-bound currentness report.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RepositoryCurrentnessReport {
    /// Closed report schema version.
    pub schema_version: u16,
    /// Exact remote identity digest.
    pub remote_sha256: String,
    /// Exact default branch.
    pub default_branch: String,
    /// Exact local branch when one exists.
    pub local_branch: Option<String>,
    /// Exact upstream when one exists.
    pub upstream: Option<String>,
    /// Local object identity when one exists.
    pub local_object: Option<String>,
    /// Fetched object identity.
    pub fetched_object: String,
    /// Derived currentness state.
    pub state: RepositoryCurrentness,
    /// Bounded local-only commit count.
    pub ahead_count: u32,
    /// Bounded remote-only commit count.
    pub behind_count: u32,
    /// Whether tracked changes are present.
    pub dirty: bool,
    /// Whether untracked paths are present.
    pub untracked: bool,
    /// Exact fetch-evidence digest.
    pub fetch_evidence_sha256: String,
    /// SHA-256 over all preceding fields.
    pub report_sha256: String,
}

/// Derives currentness without accepting branch state from model narration.
pub fn report_currentness(
    observation: &CurrentnessObservation,
) -> Result<RepositoryCurrentnessReport, RepositorySafetyError> {
    observation.remote.verify()?;
    validate_branch_ref(&observation.default_branch)?;
    if let Some(branch) = &observation.local_branch {
        validate_branch_ref(branch)?;
    }
    if let Some(upstream) = &observation.upstream {
        validate_remote_tracking_ref(upstream)?;
    }
    if observation
        .local_object
        .as_deref()
        .is_some_and(|object| !valid_object_id(object))
        || !valid_object_id(&observation.fetched_object)
        || !is_sha256(&observation.fetch_evidence_sha256)
        || observation.max_age_ms == 0
        || observation.observed_at_epoch_ms < observation.fetched_at_epoch_ms
    {
        return Err(RepositorySafetyError::InvalidInput);
    }
    let age = observation.observed_at_epoch_ms - observation.fetched_at_epoch_ms;
    let state = if observation.untracked {
        RepositoryCurrentness::Untracked
    } else if observation.dirty {
        RepositoryCurrentness::Dirty
    } else if age > observation.max_age_ms {
        RepositoryCurrentness::Stale
    } else if observation.local_object.is_none() {
        RepositoryCurrentness::Unborn
    } else {
        match (observation.ahead_count, observation.behind_count) {
            (0, 0) => RepositoryCurrentness::Current,
            (_, 0) => RepositoryCurrentness::Ahead,
            (0, _) => RepositoryCurrentness::Behind,
            _ => RepositoryCurrentness::Diverged,
        }
    };
    let mut report = RepositoryCurrentnessReport {
        schema_version: SCHEMA_VERSION,
        remote_sha256: observation.remote.identity_sha256.clone(),
        default_branch: observation.default_branch.clone(),
        local_branch: observation.local_branch.clone(),
        upstream: observation.upstream.clone(),
        local_object: observation.local_object.clone(),
        fetched_object: observation.fetched_object.clone(),
        state,
        ahead_count: observation.ahead_count,
        behind_count: observation.behind_count,
        dirty: observation.dirty,
        untracked: observation.untracked,
        fetch_evidence_sha256: observation.fetch_evidence_sha256.clone(),
        report_sha256: ZERO_SHA256.to_owned(),
    };
    report.report_sha256 = canonical_sha256(&report)?;
    Ok(report)
}

/// Content-minimized repository preservation snapshot.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RepositoryPreservationManifest {
    /// Closed manifest schema version.
    pub schema_version: u16,
    /// Canonical repository identity digest.
    pub repository_sha256: String,
    /// Canonical common-directory identity digest.
    pub common_directory_sha256: String,
    /// Exact object format, currently `sha1` or `sha256`.
    pub object_format: String,
    /// Whether ownership and write permissions satisfy platform policy.
    pub safe_ownership: bool,
    /// Exact `HEAD` object when present.
    pub head_object: Option<String>,
    /// Exact current branch when attached.
    pub current_branch: Option<String>,
    /// Exact upstream when configured.
    pub upstream: Option<String>,
    /// Whether `HEAD` is detached.
    pub detached: bool,
    /// Digest of index identity and staged dispositions.
    pub index_sha256: String,
    /// Digest of bounded path disposition records, never raw path content.
    pub path_dispositions_sha256: String,
    /// Count of untracked paths.
    pub untracked_count: u32,
    /// Count of ignored paths; ignored bytes are never retained.
    pub ignored_count: u32,
    /// Digest of local branch refs.
    pub local_branches_sha256: String,
    /// Digest of remote-tracking refs.
    pub remote_tracking_refs_sha256: String,
    /// Digest of tags.
    pub tags_sha256: String,
    /// Digest of Git notes refs.
    pub notes_sha256: String,
    /// Digest of stash identity.
    pub stash_sha256: String,
    /// Digest of replacement refs and graft state.
    pub replacement_refs_sha256: String,
    /// Digest of reflog identities and sizes.
    pub reflogs_sha256: String,
    /// Digest of AgentMage-owned transaction refs.
    pub agentmage_refs_sha256: String,
    /// Digest of registered worktree identities and states.
    pub worktrees_sha256: String,
    /// Digest of submodule declarations and local states.
    pub submodules_sha256: String,
    /// Digest of Large File Storage declarations and local state.
    pub lfs_sha256: String,
    /// Digest of object database identity and bounded inventory.
    pub object_database_sha256: String,
    /// Digest of repository, worktree, and approved configuration observations.
    pub configuration_sha256: String,
    /// Digest of hook identities and metadata; hook bytes are not retained.
    pub hooks_sha256: String,
    /// Digest of filter, attribute, diff, merge, and text-conversion declarations.
    pub content_drivers_sha256: String,
    /// Digest of remotes, refspecs, and credential-free URL identities.
    pub remotes_sha256: String,
    /// Digest of lock and in-progress-operation identities.
    pub operations_sha256: String,
    /// Whether shallow repository state exists.
    pub shallow: bool,
    /// Whether partial-clone state exists.
    pub partial: bool,
    /// Whether an unsupported or executable configuration hazard exists.
    pub hazardous_configuration: bool,
    /// SHA-256 over all preceding fields.
    pub manifest_sha256: String,
}

impl RepositoryPreservationManifest {
    /// Validates and seals a content-minimized manifest.
    pub fn seal(mut manifest: Self) -> Result<Self, RepositorySafetyError> {
        manifest.schema_version = SCHEMA_VERSION;
        manifest.manifest_sha256 = ZERO_SHA256.to_owned();
        validate_manifest_shape(&manifest)?;
        manifest.manifest_sha256 = canonical_sha256(&manifest)?;
        Ok(manifest)
    }

    /// Revalidates shape and canonical identity.
    pub fn verify(&self) -> Result<(), RepositorySafetyError> {
        validate_manifest_shape(self)?;
        let mut candidate = self.clone();
        candidate.manifest_sha256 = ZERO_SHA256.to_owned();
        if canonical_sha256(&candidate)? != self.manifest_sha256 {
            return Err(RepositorySafetyError::ManifestDenied);
        }
        Ok(())
    }
}

/// Exact preservation fields one operation is permitted to change.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RepositoryOwnedDelta {
    /// Fetch may add quarantined objects and one AgentMage transaction ref.
    Fetch,
    /// Worktree creation may add one task branch and one owned worktree.
    WorktreeCreate,
    /// Worktree removal may remove one owned worktree while preserving its task branch.
    WorktreeRemove,
    /// A compare-and-swap fast-forward may change one local branch identity.
    BranchFastForward,
    /// An observation permits no state change.
    None,
}

/// Verifies that only the declared operation-owned manifest fields changed.
pub fn reconcile_preservation(
    before: &RepositoryPreservationManifest,
    after: &RepositoryPreservationManifest,
    delta: RepositoryOwnedDelta,
) -> Result<(), RepositorySafetyError> {
    before.verify()?;
    after.verify()?;
    if before.repository_sha256 != after.repository_sha256
        || before.common_directory_sha256 != after.common_directory_sha256
        || !after.safe_ownership
        || after.hazardous_configuration
    {
        return Err(RepositorySafetyError::PreservationMismatch);
    }
    let mut normalized = after.clone();
    match delta {
        RepositoryOwnedDelta::Fetch => {
            normalized.agentmage_refs_sha256 = before.agentmage_refs_sha256.clone();
            normalized.object_database_sha256 = before.object_database_sha256.clone();
        }
        RepositoryOwnedDelta::WorktreeCreate | RepositoryOwnedDelta::WorktreeRemove => {
            normalized.agentmage_refs_sha256 = before.agentmage_refs_sha256.clone();
            normalized.worktrees_sha256 = before.worktrees_sha256.clone();
        }
        RepositoryOwnedDelta::BranchFastForward => {
            normalized.agentmage_refs_sha256 = before.agentmage_refs_sha256.clone();
        }
        RepositoryOwnedDelta::None => {}
    }
    normalized.manifest_sha256 = before.manifest_sha256.clone();
    if normalized != *before {
        return Err(RepositorySafetyError::PreservationMismatch);
    }
    Ok(())
}

/// Fixed Git subprocess class; there is deliberately no generic or pull variant.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GitInvocationKind {
    /// Initialize one new bare owned repository before an exact fetch.
    InitializeOwnedRepository,
    /// Fetch one full remote branch ref into one transaction ref.
    FetchExactRef,
    /// Add one owned task worktree and branch from one immutable object.
    WorktreeCreate,
    /// Remove one proven clean owned worktree without force.
    WorktreeRemove,
    /// Prove one object is an ancestor of another.
    ProveAncestor,
    /// Compare-and-swap one exact local branch.
    UpdateRefCompareAndSwap,
}

/// One direct hardened Git invocation with a complete replacement environment.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HardenedGitInvocation {
    /// Closed invocation class.
    pub kind: GitInvocationKind,
    /// Literal arguments after the pinned `git` executable.
    pub arguments: Vec<String>,
    /// Complete environment installed after `env_clear`.
    pub environment: BTreeMap<String, String>,
    /// Whether this invocation may contact the one approved remote.
    pub network: bool,
    /// SHA-256 over all preceding fields.
    pub invocation_sha256: String,
}

impl HardenedGitInvocation {
    fn seal(
        kind: GitInvocationKind,
        operation_arguments: Vec<String>,
        network: bool,
        protocol: Option<GitRemoteProtocol>,
    ) -> Result<Self, RepositorySafetyError> {
        let mut arguments = hardened_prefix(protocol);
        arguments.extend(operation_arguments);
        if arguments.is_empty()
            || arguments.len() > 128
            || arguments.iter().any(|argument| {
                argument.is_empty()
                    || argument.len() > 4_096
                    || argument.as_bytes().contains(&0)
                    || argument.chars().any(|value| value.is_control())
            })
            || network != protocol.is_some()
        {
            return Err(RepositorySafetyError::InvalidInput);
        }
        let mut invocation = Self {
            kind,
            arguments,
            environment: hardened_environment(),
            network,
            invocation_sha256: ZERO_SHA256.to_owned(),
        };
        invocation.invocation_sha256 = canonical_sha256(&invocation)?;
        Ok(invocation)
    }

    /// Revalidates the complete fixed invocation.
    pub fn verify(&self) -> Result<(), RepositorySafetyError> {
        if self.environment != hardened_environment() || !is_sha256(&self.invocation_sha256) {
            return Err(RepositorySafetyError::InvalidInput);
        }
        let mut candidate = self.clone();
        candidate.invocation_sha256 = ZERO_SHA256.to_owned();
        if canonical_sha256(&candidate)? != self.invocation_sha256
            || contains_prohibited_git_operation(&self.arguments)
        {
            return Err(RepositorySafetyError::InvalidInput);
        }
        Ok(())
    }
}

/// Closed local repository operation supported by Sprint 42.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RepositoryOperation {
    /// Create a no-checkout owned repository and fetch one exact ref.
    Clone,
    /// Fetch one exact ref into a transaction namespace.
    Fetch,
    /// Create one branch and isolated worktree for a task.
    WorktreeCreate,
    /// Remove one proven-clean owned worktree.
    WorktreeRemove,
    /// Advance one exact local branch by compare-and-swap.
    BranchFastForward,
}

impl RepositoryOperation {
    /// Returns the exact canonical grant operation required for this local effect.
    #[must_use]
    pub const fn grant_operation(self) -> GrantOperation {
        match self {
            Self::Clone => GrantOperation::GitClone,
            Self::Fetch => GrantOperation::GitFetch,
            Self::WorktreeCreate => GrantOperation::GitWorktreeCreate,
            Self::WorktreeRemove => GrantOperation::GitWorktreeRemove,
            Self::BranchFastForward => GrantOperation::GitBranchFastForward,
        }
    }
}

/// Exact plan for one repository operation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RepositoryOperationPlan {
    /// Closed plan schema version.
    pub schema_version: u16,
    /// Non-replayable transaction identity.
    pub transaction_id: String,
    /// Exact operation and grant class.
    pub operation: RepositoryOperation,
    /// Exact remote identity for clone or fetch.
    pub remote: Option<GitRemoteIdentity>,
    /// Exact full remote branch ref where applicable.
    pub source_ref: Option<String>,
    /// Exact owned fetch ref where applicable.
    pub transaction_ref: Option<String>,
    /// Exact local branch where applicable.
    pub branch_ref: Option<String>,
    /// Exact expected old local object where applicable.
    pub expected_old_object: Option<String>,
    /// Exact expected new object or immutable base where applicable.
    pub expected_new_object: Option<String>,
    /// Digest of the owned repository or destination path identity.
    pub repository_path_sha256: String,
    /// Digest of the owned worktree path identity where applicable.
    pub worktree_path_sha256: Option<String>,
    /// Manifest that must still match immediately before grant consumption.
    pub preservation_manifest_sha256: String,
    /// Exact fixed invocation sequence.
    pub invocations: Vec<HardenedGitInvocation>,
    /// SHA-256 over all preceding fields.
    pub plan_sha256: String,
}

impl RepositoryOperationPlan {
    /// Revalidates shape, operation-specific fields, invocation sequence, and digest.
    pub fn verify(&self) -> Result<(), RepositorySafetyError> {
        validate_identifier(&self.transaction_id)?;
        if !is_sha256(&self.repository_path_sha256)
            || !is_sha256(&self.preservation_manifest_sha256)
            || self
                .worktree_path_sha256
                .as_deref()
                .is_some_and(|value| !is_sha256(value))
            || self.invocations.is_empty()
            || self.invocations.iter().any(|item| item.verify().is_err())
        {
            return Err(RepositorySafetyError::InvalidInput);
        }
        validate_plan_shape(self)?;
        let mut candidate = self.clone();
        candidate.plan_sha256 = ZERO_SHA256.to_owned();
        if canonical_sha256(&candidate)? != self.plan_sha256 {
            return Err(RepositorySafetyError::InvalidInput);
        }
        Ok(())
    }
}

/// Builds an approval-ready no-checkout clone transaction into an owned destination.
pub fn plan_clone(
    transaction_id: &str,
    remote: GitRemoteIdentity,
    source_ref: &str,
    destination_path_sha256: &str,
    empty_destination_manifest_sha256: &str,
) -> Result<RepositoryOperationPlan, RepositorySafetyError> {
    remote.verify()?;
    validate_identifier(transaction_id)?;
    validate_branch_ref(source_ref)?;
    require_sha(destination_path_sha256)?;
    require_sha(empty_destination_manifest_sha256)?;
    let transaction_ref = transaction_ref(transaction_id)?;
    let short_branch = source_ref
        .strip_prefix("refs/heads/")
        .ok_or(RepositorySafetyError::RefDenied)?;
    let initialize = HardenedGitInvocation::seal(
        GitInvocationKind::InitializeOwnedRepository,
        vec![
            "init".to_owned(),
            "--bare".to_owned(),
            "--initial-branch=agentmage-unborn".to_owned(),
            "/owned/repository".to_owned(),
        ],
        false,
        None,
    )?;
    let fetch = HardenedGitInvocation::seal(
        GitInvocationKind::FetchExactRef,
        vec![
            "--git-dir=/owned/repository".to_owned(),
            "fetch".to_owned(),
            "--atomic".to_owned(),
            "--no-tags".to_owned(),
            "--no-write-fetch-head".to_owned(),
            "--no-recurse-submodules".to_owned(),
            remote.canonical_url.clone(),
            format!("refs/heads/{short_branch}:{transaction_ref}"),
        ],
        true,
        Some(remote.protocol),
    )?;
    seal_plan(RepositoryOperationPlan {
        schema_version: SCHEMA_VERSION,
        transaction_id: transaction_id.to_owned(),
        operation: RepositoryOperation::Clone,
        remote: Some(remote),
        source_ref: Some(source_ref.to_owned()),
        transaction_ref: Some(transaction_ref),
        branch_ref: None,
        expected_old_object: None,
        expected_new_object: None,
        repository_path_sha256: destination_path_sha256.to_owned(),
        worktree_path_sha256: None,
        preservation_manifest_sha256: empty_destination_manifest_sha256.to_owned(),
        invocations: vec![initialize, fetch],
        plan_sha256: ZERO_SHA256.to_owned(),
    })
}

/// Builds an exact namespaced fetch plan for an existing owned repository.
pub fn plan_fetch(
    transaction_id: &str,
    remote: GitRemoteIdentity,
    source_ref: &str,
    repository_path_sha256: &str,
    manifest: &RepositoryPreservationManifest,
) -> Result<RepositoryOperationPlan, RepositorySafetyError> {
    remote.verify()?;
    manifest.verify()?;
    if manifest.hazardous_configuration || !manifest.safe_ownership {
        return Err(RepositorySafetyError::ManifestDenied);
    }
    validate_identifier(transaction_id)?;
    validate_branch_ref(source_ref)?;
    require_sha(repository_path_sha256)?;
    let transaction_ref = transaction_ref(transaction_id)?;
    let invocation = HardenedGitInvocation::seal(
        GitInvocationKind::FetchExactRef,
        vec![
            "--git-dir=/repo".to_owned(),
            "fetch".to_owned(),
            "--atomic".to_owned(),
            "--no-tags".to_owned(),
            "--no-write-fetch-head".to_owned(),
            "--no-recurse-submodules".to_owned(),
            remote.canonical_url.clone(),
            format!("{source_ref}:{transaction_ref}"),
        ],
        true,
        Some(remote.protocol),
    )?;
    seal_plan(RepositoryOperationPlan {
        schema_version: SCHEMA_VERSION,
        transaction_id: transaction_id.to_owned(),
        operation: RepositoryOperation::Fetch,
        remote: Some(remote),
        source_ref: Some(source_ref.to_owned()),
        transaction_ref: Some(transaction_ref),
        branch_ref: None,
        expected_old_object: None,
        expected_new_object: None,
        repository_path_sha256: repository_path_sha256.to_owned(),
        worktree_path_sha256: None,
        preservation_manifest_sha256: manifest.manifest_sha256.clone(),
        invocations: vec![invocation],
        plan_sha256: ZERO_SHA256.to_owned(),
    })
}

/// Builds one task-branch worktree creation plan from an immutable base object.
pub fn plan_worktree_create(
    transaction_id: &str,
    task_id: &str,
    base_object: &str,
    repository_path_sha256: &str,
    worktree_path_sha256: &str,
    manifest: &RepositoryPreservationManifest,
) -> Result<RepositoryOperationPlan, RepositorySafetyError> {
    validate_identifier(transaction_id)?;
    validate_identifier(task_id)?;
    require_sha(repository_path_sha256)?;
    require_sha(worktree_path_sha256)?;
    if !valid_object_id(base_object) {
        return Err(RepositorySafetyError::InvalidInput);
    }
    manifest.verify()?;
    if manifest.hazardous_configuration || !manifest.safe_ownership {
        return Err(RepositorySafetyError::ManifestDenied);
    }
    let branch_ref = format!("refs/heads/agentmage/tasks/{task_id}");
    validate_branch_ref(&branch_ref)?;
    let invocation = HardenedGitInvocation::seal(
        GitInvocationKind::WorktreeCreate,
        vec![
            "--git-dir=/repo".to_owned(),
            "worktree".to_owned(),
            "add".to_owned(),
            "--no-track".to_owned(),
            "-b".to_owned(),
            branch_ref.trim_start_matches("refs/heads/").to_owned(),
            "/owned/worktree".to_owned(),
            base_object.to_owned(),
        ],
        false,
        None,
    )?;
    seal_plan(RepositoryOperationPlan {
        schema_version: SCHEMA_VERSION,
        transaction_id: transaction_id.to_owned(),
        operation: RepositoryOperation::WorktreeCreate,
        remote: None,
        source_ref: None,
        transaction_ref: None,
        branch_ref: Some(branch_ref),
        expected_old_object: None,
        expected_new_object: Some(base_object.to_owned()),
        repository_path_sha256: repository_path_sha256.to_owned(),
        worktree_path_sha256: Some(worktree_path_sha256.to_owned()),
        preservation_manifest_sha256: manifest.manifest_sha256.clone(),
        invocations: vec![invocation],
        plan_sha256: ZERO_SHA256.to_owned(),
    })
}

/// Builds one non-forced removal plan for a proven clean owned worktree.
pub fn plan_worktree_remove(
    transaction_id: &str,
    record: &OwnedWorktreeRecord,
    repository_path_sha256: &str,
    manifest: &RepositoryPreservationManifest,
) -> Result<RepositoryOperationPlan, RepositorySafetyError> {
    validate_identifier(transaction_id)?;
    require_sha(repository_path_sha256)?;
    manifest.verify()?;
    record.verify()?;
    if !record.cleanup_eligible() || manifest.hazardous_configuration || !manifest.safe_ownership {
        return Err(RepositorySafetyError::WorktreeDenied);
    }
    let invocation = HardenedGitInvocation::seal(
        GitInvocationKind::WorktreeRemove,
        vec![
            "--git-dir=/repo".to_owned(),
            "worktree".to_owned(),
            "remove".to_owned(),
            "/owned/worktree".to_owned(),
        ],
        false,
        None,
    )?;
    seal_plan(RepositoryOperationPlan {
        schema_version: SCHEMA_VERSION,
        transaction_id: transaction_id.to_owned(),
        operation: RepositoryOperation::WorktreeRemove,
        remote: None,
        source_ref: None,
        transaction_ref: None,
        branch_ref: Some(record.branch_ref.clone()),
        expected_old_object: Some(record.source_object.clone()),
        expected_new_object: None,
        repository_path_sha256: repository_path_sha256.to_owned(),
        worktree_path_sha256: Some(record.worktree_path_sha256.clone()),
        preservation_manifest_sha256: manifest.manifest_sha256.clone(),
        invocations: vec![invocation],
        plan_sha256: ZERO_SHA256.to_owned(),
    })
}

/// Builds a two-step ancestry proof and compare-and-swap branch update.
pub fn plan_branch_fast_forward(
    transaction_id: &str,
    branch_ref: &str,
    expected_old_object: &str,
    expected_new_object: &str,
    descendant_proven: bool,
    repository_path_sha256: &str,
    manifest: &RepositoryPreservationManifest,
) -> Result<RepositoryOperationPlan, RepositorySafetyError> {
    validate_identifier(transaction_id)?;
    validate_branch_ref(branch_ref)?;
    require_sha(repository_path_sha256)?;
    manifest.verify()?;
    if !branch_ref.starts_with("refs/heads/agentmage/tasks/")
        || !valid_object_id(expected_old_object)
        || !valid_object_id(expected_new_object)
        || expected_old_object == expected_new_object
        || !descendant_proven
        || manifest.hazardous_configuration
        || !manifest.safe_ownership
    {
        return Err(RepositorySafetyError::FastForwardDenied);
    }
    let prove = HardenedGitInvocation::seal(
        GitInvocationKind::ProveAncestor,
        vec![
            "--git-dir=/repo".to_owned(),
            "merge-base".to_owned(),
            "--is-ancestor".to_owned(),
            expected_old_object.to_owned(),
            expected_new_object.to_owned(),
        ],
        false,
        None,
    )?;
    let update = HardenedGitInvocation::seal(
        GitInvocationKind::UpdateRefCompareAndSwap,
        vec![
            "--git-dir=/repo".to_owned(),
            "update-ref".to_owned(),
            "--no-deref".to_owned(),
            branch_ref.to_owned(),
            expected_new_object.to_owned(),
            expected_old_object.to_owned(),
        ],
        false,
        None,
    )?;
    seal_plan(RepositoryOperationPlan {
        schema_version: SCHEMA_VERSION,
        transaction_id: transaction_id.to_owned(),
        operation: RepositoryOperation::BranchFastForward,
        remote: None,
        source_ref: None,
        transaction_ref: None,
        branch_ref: Some(branch_ref.to_owned()),
        expected_old_object: Some(expected_old_object.to_owned()),
        expected_new_object: Some(expected_new_object.to_owned()),
        repository_path_sha256: repository_path_sha256.to_owned(),
        worktree_path_sha256: None,
        preservation_manifest_sha256: manifest.manifest_sha256.clone(),
        invocations: vec![prove, update],
        plan_sha256: ZERO_SHA256.to_owned(),
    })
}

/// Lifecycle disposition of one AgentMage-owned worktree.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorktreeDisposition {
    /// Available for the exact task.
    Active,
    /// Ownership has been explicitly handed to the user; automatic removal is denied.
    HandedOff,
    /// Interrupted or uncertain work is retained for recovery.
    RecoveryRequired,
    /// Proven clean and process-free after a retained recovery snapshot.
    CleanupEligible,
    /// Removal completed and postconditions were reconciled.
    Removed,
}

/// Complete content-minimized ownership record for one task worktree.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OwnedWorktreeRecord {
    /// Closed record schema version.
    pub schema_version: u16,
    /// Stable worktree identity.
    pub worktree_id: String,
    /// Exact task identity.
    pub task_id: String,
    /// Exact immutable source object.
    pub source_object: String,
    /// Exact AgentMage task branch.
    pub branch_ref: String,
    /// Digest of the canonical worktree path.
    pub worktree_path_sha256: String,
    /// Digest of the owner identity.
    pub owner_sha256: String,
    /// Ordered exact grants associated with the task.
    pub grant_ids: Vec<String>,
    /// Digest of exact task-owned file identities.
    pub file_ownership_sha256: String,
    /// Count of live process identities attributed to this worktree.
    pub live_process_count: u32,
    /// Exact resource-budget identity.
    pub resource_budget_sha256: String,
    /// Kernel timestamp through which retention is required.
    pub retain_until_epoch_ms: u64,
    /// Whether the worktree is proven clean.
    pub clean: bool,
    /// Whether a recovery artifact has been retained.
    pub recovery_retained: bool,
    /// Current explicit disposition.
    pub disposition: WorktreeDisposition,
    /// SHA-256 over all preceding fields.
    pub record_sha256: String,
}

impl OwnedWorktreeRecord {
    /// Validates and seals one ownership record.
    pub fn seal(mut record: Self) -> Result<Self, RepositorySafetyError> {
        record.schema_version = SCHEMA_VERSION;
        record.record_sha256 = ZERO_SHA256.to_owned();
        validate_worktree_record(&record)?;
        record.record_sha256 = canonical_sha256(&record)?;
        Ok(record)
    }

    /// Revalidates record shape and digest.
    pub fn verify(&self) -> Result<(), RepositorySafetyError> {
        validate_worktree_record(self)?;
        let mut candidate = self.clone();
        candidate.record_sha256 = ZERO_SHA256.to_owned();
        if canonical_sha256(&candidate)? != self.record_sha256 {
            return Err(RepositorySafetyError::WorktreeDenied);
        }
        Ok(())
    }

    /// Reports whether non-forced automatic cleanup is currently permitted.
    #[must_use]
    pub fn cleanup_eligible(&self) -> bool {
        self.disposition == WorktreeDisposition::CleanupEligible
            && self.clean
            && self.recovery_retained
            && self.live_process_count == 0
    }
}

/// In-memory exact ownership registry; durable storage is supplied by the operational store.
#[derive(Clone, Debug, Default)]
pub struct WorktreeOwnershipRegistry {
    records: BTreeMap<String, OwnedWorktreeRecord>,
}

impl WorktreeOwnershipRegistry {
    /// Registers one unique active owned worktree.
    pub fn register(&mut self, record: OwnedWorktreeRecord) -> Result<(), RepositorySafetyError> {
        record.verify()?;
        if record.disposition != WorktreeDisposition::Active
            || self.records.len() >= MAX_WORKTREES
            || self.records.contains_key(&record.worktree_id)
            || self.records.values().any(|existing| {
                existing.worktree_path_sha256 == record.worktree_path_sha256
                    || existing.branch_ref == record.branch_ref
            })
        {
            return Err(RepositorySafetyError::WorktreeDenied);
        }
        self.records.insert(record.worktree_id.clone(), record);
        Ok(())
    }

    /// Returns records in stable worktree identity order.
    #[must_use]
    pub fn list(&self) -> Vec<&OwnedWorktreeRecord> {
        self.records.values().collect()
    }

    /// Replaces one record only when the prior record digest still matches.
    pub fn compare_and_swap(
        &mut self,
        worktree_id: &str,
        expected_record_sha256: &str,
        replacement: OwnedWorktreeRecord,
    ) -> Result<(), RepositorySafetyError> {
        replacement.verify()?;
        let current = self
            .records
            .get(worktree_id)
            .ok_or(RepositorySafetyError::WorktreeDenied)?;
        if current.record_sha256 != expected_record_sha256
            || replacement.worktree_id != current.worktree_id
            || replacement.task_id != current.task_id
            || replacement.source_object != current.source_object
            || replacement.branch_ref != current.branch_ref
            || replacement.worktree_path_sha256 != current.worktree_path_sha256
            || replacement.owner_sha256 != current.owner_sha256
        {
            return Err(RepositorySafetyError::WorktreeDenied);
        }
        self.records.insert(worktree_id.to_owned(), replacement);
        Ok(())
    }
}

/// One content-minimized path observation used before change transfer.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TransferPathObservation {
    /// Digest of the canonical path.
    pub path_sha256: String,
    /// Preimage approved in the transfer preview, or zero for expected absence.
    pub approved_preimage_sha256: String,
    /// Current preimage immediately before grant consumption, or zero for absence.
    pub current_preimage_sha256: String,
    /// Whether this exact path is task-owned.
    pub task_owned: bool,
    /// Optional digest of the exact prior path for an approved rename.
    pub renamed_from_sha256: Option<String>,
}

/// Rejects stale, duplicate, unowned, or unexpected rename transfer paths.
pub fn verify_transfer_paths(
    observations: &[TransferPathObservation],
) -> Result<String, RepositorySafetyError> {
    if observations.is_empty() || observations.len() > MAX_CHANGED_PATHS {
        return Err(RepositorySafetyError::Collision);
    }
    let mut paths = BTreeSet::new();
    let mut rename_sources = BTreeSet::new();
    for item in observations {
        if !is_sha256(&item.path_sha256)
            || !is_sha256(&item.approved_preimage_sha256)
            || !is_sha256(&item.current_preimage_sha256)
            || !item.task_owned
            || item.approved_preimage_sha256 != item.current_preimage_sha256
            || !paths.insert(&item.path_sha256)
        {
            return Err(RepositorySafetyError::Collision);
        }
        if let Some(source) = &item.renamed_from_sha256
            && (!is_sha256(source) || source == &item.path_sha256 || !rename_sources.insert(source))
        {
            return Err(RepositorySafetyError::Collision);
        }
    }
    canonical_sha256(&observations).map_err(|_| RepositorySafetyError::Collision)
}

/// Trusted platform result for one exact repository operation attempt.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RepositoryPlatformResult {
    /// Terminal outcome established by platform postconditions.
    pub outcome: OperationOutcome,
    /// Manifest immediately before effect launch.
    pub before: RepositoryPreservationManifest,
    /// Manifest after success, failure, timeout, cancellation, or recovery.
    pub after: RepositoryPreservationManifest,
    /// Whether every attributed process and owned transient artifact was reconciled.
    pub cleanup_verified: bool,
    /// Stable content-free platform result code.
    pub platform_code: String,
}

/// Hash-bound terminal repository operation receipt.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RepositoryOperationReceipt {
    /// Closed receipt schema version.
    pub schema_version: u16,
    /// Exact repository transaction identity.
    pub repository_transaction_id: String,
    /// Exact authority transaction identity.
    pub authority_transaction_id: String,
    /// Exact non-replayable operation attempt identity.
    pub operation_attempt_id: String,
    /// Exact operation.
    pub operation: RepositoryOperation,
    /// Exact plan digest.
    pub plan_sha256: String,
    /// Exact pre-effect manifest digest.
    pub before_manifest_sha256: String,
    /// Exact terminal manifest digest.
    pub after_manifest_sha256: String,
    /// Deterministically established outcome.
    pub outcome: OperationOutcome,
    /// Whether cleanup and owned-artifact accounting completed.
    pub cleanup_verified: bool,
    /// Stable content-free platform code.
    pub platform_code: String,
    /// SHA-256 over all preceding fields.
    pub receipt_sha256: String,
}

/// Verifies postconditions and seals a truthful terminal receipt.
pub fn reconcile_operation(
    plan: &RepositoryOperationPlan,
    result: RepositoryPlatformResult,
    authority_transaction_id: &str,
    operation_attempt_id: &str,
) -> Result<RepositoryOperationReceipt, RepositorySafetyError> {
    plan.verify()?;
    validate_identifier(authority_transaction_id)?;
    validate_identifier(operation_attempt_id)?;
    result.before.verify()?;
    result.after.verify()?;
    if result.before.manifest_sha256 != plan.preservation_manifest_sha256
        || !valid_platform_code(&result.platform_code)
    {
        return Err(RepositorySafetyError::ResultDenied);
    }
    let delta = match (plan.operation, result.outcome) {
        (
            _,
            OperationOutcome::Failed | OperationOutcome::Cancelled | OperationOutcome::TimedOut,
        ) => RepositoryOwnedDelta::None,
        (RepositoryOperation::Fetch, OperationOutcome::Succeeded) => RepositoryOwnedDelta::Fetch,
        (RepositoryOperation::WorktreeCreate, OperationOutcome::Succeeded) => {
            RepositoryOwnedDelta::WorktreeCreate
        }
        (RepositoryOperation::WorktreeRemove, OperationOutcome::Succeeded) => {
            RepositoryOwnedDelta::WorktreeRemove
        }
        (RepositoryOperation::BranchFastForward, OperationOutcome::Succeeded) => {
            RepositoryOwnedDelta::BranchFastForward
        }
        (RepositoryOperation::Clone, OperationOutcome::Succeeded)
        | (_, OperationOutcome::Denied | OperationOutcome::Uncertain) => {
            return Err(RepositorySafetyError::ResultDenied);
        }
    };
    reconcile_preservation(&result.before, &result.after, delta)?;
    if !result.cleanup_verified {
        return Err(RepositorySafetyError::ResultDenied);
    }
    let mut receipt = RepositoryOperationReceipt {
        schema_version: SCHEMA_VERSION,
        repository_transaction_id: plan.transaction_id.clone(),
        authority_transaction_id: authority_transaction_id.to_owned(),
        operation_attempt_id: operation_attempt_id.to_owned(),
        operation: plan.operation,
        plan_sha256: plan.plan_sha256.clone(),
        before_manifest_sha256: result.before.manifest_sha256,
        after_manifest_sha256: result.after.manifest_sha256,
        outcome: result.outcome,
        cleanup_verified: result.cleanup_verified,
        platform_code: result.platform_code,
        receipt_sha256: ZERO_SHA256.to_owned(),
    };
    receipt.receipt_sha256 = canonical_sha256(&receipt)?;
    Ok(receipt)
}

/// Authority-free prepared bytes for one exact repository operation plan.
#[derive(Clone, Debug)]
pub struct PreparedRepositoryOperation {
    plan: RepositoryOperationPlan,
    request_bytes: Vec<u8>,
    request_sha256: String,
}

impl PreparedRepositoryOperation {
    /// Validates and serializes one complete operation plan before approval.
    pub fn new(plan: RepositoryOperationPlan) -> Result<Self, RepositorySafetyError> {
        plan.verify()?;
        let request_bytes =
            serde_json::to_vec(&plan).map_err(|_| RepositorySafetyError::InvalidInput)?;
        let request_sha256 = sha256_hex(&request_bytes);
        Ok(Self {
            plan,
            request_bytes,
            request_sha256,
        })
    }

    /// Returns the complete deterministic preview and execution plan.
    #[must_use]
    pub const fn plan(&self) -> &RepositoryOperationPlan {
        &self.plan
    }

    /// Returns exact canonical request bytes for grant and tool-call binding.
    #[must_use]
    pub fn request_bytes(&self) -> &[u8] {
        &self.request_bytes
    }

    /// Returns the digest of exact canonical request bytes.
    #[must_use]
    pub fn request_sha256(&self) -> &str {
        &self.request_sha256
    }
}

/// Nonforgeable launch permit created only after exact repository authority validation.
pub struct RepositoryLaunchPermit<'plan> {
    plan: &'plan RepositoryOperationPlan,
    before: &'plan RepositoryPreservationManifest,
}

impl RepositoryLaunchPermit<'_> {
    /// Returns the exact approved repository plan.
    #[must_use]
    pub const fn plan(&self) -> &RepositoryOperationPlan {
        self.plan
    }

    /// Returns the exact current pre-effect preservation manifest.
    #[must_use]
    pub const fn before(&self) -> &RepositoryPreservationManifest {
        self.before
    }
}

impl fmt::Debug for RepositoryLaunchPermit<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RepositoryLaunchPermit")
            .field("transaction_id", &self.plan.transaction_id)
            .field("operation", &self.plan.operation)
            .finish_non_exhaustive()
    }
}

/// Trusted platform executor whose launch requires one kernel-created permit.
pub trait BoundedRepositoryExecutor {
    /// Executes one exact repository plan and returns manifest-bound postconditions.
    fn execute(
        &mut self,
        permit: RepositoryLaunchPermit<'_>,
        cancellation: &CancellationToken,
    ) -> RepositoryPlatformResult;
}

/// Inert repository driver crossing the effect boundary only with consumed authority.
pub struct RepositoryEffectDriver<E, H> {
    executor: E,
    held_repository_scope: H,
    prepared: PreparedRepositoryOperation,
    before: RepositoryPreservationManifest,
    cancellation: CancellationToken,
    receipt: Option<RepositoryOperationReceipt>,
    error: Option<RepositorySafetyError>,
}

impl<E, H> RepositoryEffectDriver<E, H> {
    /// Creates an inert driver after validating the pre-effect manifest binding.
    pub fn new(
        executor: E,
        held_repository_scope: H,
        prepared: PreparedRepositoryOperation,
        before: RepositoryPreservationManifest,
        cancellation: CancellationToken,
    ) -> Result<Self, RepositorySafetyError> {
        before.verify()?;
        if before.manifest_sha256 != prepared.plan.preservation_manifest_sha256 {
            return Err(RepositorySafetyError::ManifestDenied);
        }
        Ok(Self {
            executor,
            held_repository_scope,
            prepared,
            before,
            cancellation,
            receipt: None,
            error: None,
        })
    }

    /// Takes the repository-specific terminal receipt after mediated execution.
    pub fn take_receipt(&mut self) -> Option<RepositoryOperationReceipt> {
        self.receipt.take()
    }

    /// Takes a content-free repository-boundary error.
    pub fn take_error(&mut self) -> Option<RepositorySafetyError> {
        self.error.take()
    }

    /// Returns the executor after the attempt closes.
    #[must_use]
    pub fn into_executor(self) -> E {
        self.executor
    }
}

impl<E, H> fmt::Debug for RepositoryEffectDriver<E, H> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RepositoryEffectDriver")
            .field("transaction_id", &self.prepared.plan.transaction_id)
            .field("operation", &self.prepared.plan.operation)
            .field("has_receipt", &self.receipt.is_some())
            .field("has_error", &self.error.is_some())
            .finish_non_exhaustive()
    }
}

impl<E, H> EffectDriver for RepositoryEffectDriver<E, H>
where
    E: BoundedRepositoryExecutor,
    H: HeldWorkspaceObject,
{
    fn execute(&mut self, authorization: EffectAuthorization<'_>) -> EffectLaunch {
        let call = authorization.call();
        if authorization.operation().operation() != self.prepared.plan.operation.grant_operation()
            || !authorization.authorizes_held_object(&self.held_repository_scope)
            || call.arguments.sha256 != self.prepared.request_sha256
            || call.arguments.bytes != self.prepared.request_bytes
            || sha256_hex(&call.arguments.bytes) != call.arguments.sha256
        {
            self.error = Some(RepositorySafetyError::InvalidInput);
            return EffectLaunch::failed();
        }

        let platform = if self.cancellation.is_cancelled() {
            RepositoryPlatformResult {
                outcome: OperationOutcome::Cancelled,
                before: self.before.clone(),
                after: self.before.clone(),
                cleanup_verified: true,
                platform_code: "repository.cancelled.before_launch".to_owned(),
            }
        } else {
            self.executor.execute(
                RepositoryLaunchPermit {
                    plan: &self.prepared.plan,
                    before: &self.before,
                },
                &self.cancellation,
            )
        };

        match reconcile_operation(
            &self.prepared.plan,
            platform,
            authorization.transaction_id().as_str(),
            authorization.attempt_id().as_str(),
        ) {
            Ok(receipt) => {
                let state_change = if receipt.outcome == OperationOutcome::Succeeded {
                    StateChange::Changed
                } else {
                    StateChange::NotChanged
                };
                let effect = EffectResult::from_redacted_material(
                    receipt.outcome,
                    receipt.receipt_sha256.as_bytes(),
                    state_change,
                );
                self.receipt = Some(receipt);
                EffectLaunch::completed(effect)
            }
            Err(error) => {
                self.error = Some(error);
                EffectLaunch::failed()
            }
        }
    }
}

fn seal_plan(
    mut plan: RepositoryOperationPlan,
) -> Result<RepositoryOperationPlan, RepositorySafetyError> {
    plan.plan_sha256 = ZERO_SHA256.to_owned();
    validate_plan_shape(&plan)?;
    plan.plan_sha256 = canonical_sha256(&plan)?;
    plan.verify()?;
    Ok(plan)
}

fn validate_plan_shape(plan: &RepositoryOperationPlan) -> Result<(), RepositorySafetyError> {
    if plan.schema_version != SCHEMA_VERSION || !is_sha256(&plan.plan_sha256) {
        return Err(RepositorySafetyError::InvalidInput);
    }
    let kinds: Vec<_> = plan.invocations.iter().map(|item| item.kind).collect();
    let valid = match plan.operation {
        RepositoryOperation::Clone => {
            plan.remote.is_some()
                && plan.source_ref.is_some()
                && plan.transaction_ref.is_some()
                && plan.branch_ref.is_none()
                && plan.expected_old_object.is_none()
                && plan.expected_new_object.is_none()
                && plan.worktree_path_sha256.is_none()
                && kinds
                    == [
                        GitInvocationKind::InitializeOwnedRepository,
                        GitInvocationKind::FetchExactRef,
                    ]
        }
        RepositoryOperation::Fetch => {
            plan.remote.is_some()
                && plan.source_ref.is_some()
                && plan.transaction_ref.is_some()
                && plan.branch_ref.is_none()
                && plan.expected_old_object.is_none()
                && plan.expected_new_object.is_none()
                && plan.worktree_path_sha256.is_none()
                && kinds == [GitInvocationKind::FetchExactRef]
        }
        RepositoryOperation::WorktreeCreate => {
            plan.remote.is_none()
                && plan.source_ref.is_none()
                && plan.transaction_ref.is_none()
                && plan.branch_ref.is_some()
                && plan.expected_old_object.is_none()
                && plan
                    .expected_new_object
                    .as_deref()
                    .is_some_and(valid_object_id)
                && plan.worktree_path_sha256.is_some()
                && kinds == [GitInvocationKind::WorktreeCreate]
        }
        RepositoryOperation::WorktreeRemove => {
            plan.remote.is_none()
                && plan.source_ref.is_none()
                && plan.transaction_ref.is_none()
                && plan.branch_ref.is_some()
                && plan
                    .expected_old_object
                    .as_deref()
                    .is_some_and(valid_object_id)
                && plan.expected_new_object.is_none()
                && plan.worktree_path_sha256.is_some()
                && kinds == [GitInvocationKind::WorktreeRemove]
        }
        RepositoryOperation::BranchFastForward => {
            plan.remote.is_none()
                && plan.source_ref.is_none()
                && plan.transaction_ref.is_none()
                && plan.branch_ref.is_some()
                && plan
                    .expected_old_object
                    .as_deref()
                    .is_some_and(valid_object_id)
                && plan
                    .expected_new_object
                    .as_deref()
                    .is_some_and(valid_object_id)
                && plan.worktree_path_sha256.is_none()
                && kinds
                    == [
                        GitInvocationKind::ProveAncestor,
                        GitInvocationKind::UpdateRefCompareAndSwap,
                    ]
        }
    };
    if !valid {
        return Err(RepositorySafetyError::InvalidInput);
    }
    if let Some(remote) = &plan.remote {
        remote.verify()?;
    }
    if let Some(source) = &plan.source_ref {
        validate_branch_ref(source)?;
    }
    if let Some(transaction) = &plan.transaction_ref {
        validate_transaction_ref(transaction, &plan.transaction_id)?;
    }
    if let Some(branch) = &plan.branch_ref {
        validate_branch_ref(branch)?;
    }
    Ok(())
}

fn validate_manifest_shape(
    manifest: &RepositoryPreservationManifest,
) -> Result<(), RepositorySafetyError> {
    let hashes = [
        &manifest.repository_sha256,
        &manifest.common_directory_sha256,
        &manifest.index_sha256,
        &manifest.path_dispositions_sha256,
        &manifest.local_branches_sha256,
        &manifest.remote_tracking_refs_sha256,
        &manifest.tags_sha256,
        &manifest.notes_sha256,
        &manifest.stash_sha256,
        &manifest.replacement_refs_sha256,
        &manifest.reflogs_sha256,
        &manifest.agentmage_refs_sha256,
        &manifest.worktrees_sha256,
        &manifest.submodules_sha256,
        &manifest.lfs_sha256,
        &manifest.object_database_sha256,
        &manifest.configuration_sha256,
        &manifest.hooks_sha256,
        &manifest.content_drivers_sha256,
        &manifest.remotes_sha256,
        &manifest.operations_sha256,
        &manifest.manifest_sha256,
    ];
    if manifest.schema_version != SCHEMA_VERSION
        || !hashes.into_iter().all(|value| is_sha256(value))
        || !matches!(manifest.object_format.as_str(), "sha1" | "sha256")
        || manifest
            .head_object
            .as_deref()
            .is_some_and(|value| !valid_object_id(value))
        || manifest
            .current_branch
            .as_deref()
            .is_some_and(|value| validate_branch_ref(value).is_err())
        || manifest
            .upstream
            .as_deref()
            .is_some_and(|value| validate_remote_tracking_ref(value).is_err())
        || manifest.detached == manifest.current_branch.is_some()
    {
        return Err(RepositorySafetyError::ManifestDenied);
    }
    Ok(())
}

fn validate_worktree_record(record: &OwnedWorktreeRecord) -> Result<(), RepositorySafetyError> {
    if record.schema_version != SCHEMA_VERSION
        || !valid_identifier(&record.worktree_id)
        || !valid_identifier(&record.task_id)
        || !valid_object_id(&record.source_object)
        || validate_branch_ref(&record.branch_ref).is_err()
        || !record.branch_ref.starts_with("refs/heads/agentmage/tasks/")
        || ![
            &record.worktree_path_sha256,
            &record.owner_sha256,
            &record.file_ownership_sha256,
            &record.resource_budget_sha256,
            &record.record_sha256,
        ]
        .into_iter()
        .all(|value| is_sha256(value))
        || record.grant_ids.len() > MAX_GRANTS
        || record
            .grant_ids
            .iter()
            .any(|value| !valid_identifier(value))
        || record.grant_ids.iter().collect::<BTreeSet<_>>().len() != record.grant_ids.len()
        || record.disposition == WorktreeDisposition::Removed
            && (!record.clean || record.live_process_count != 0 || !record.recovery_retained)
    {
        return Err(RepositorySafetyError::WorktreeDenied);
    }
    Ok(())
}

fn hardened_prefix(protocol: Option<GitRemoteProtocol>) -> Vec<String> {
    let mut arguments = vec![
        "--no-pager",
        "-c",
        "alias.agentmage=!false",
        "-c",
        "core.hooksPath=/dev/null",
        "-c",
        "core.fsmonitor=false",
        "-c",
        "core.askPass=",
        "-c",
        "credential.helper=",
        "-c",
        "credential.interactive=never",
        "-c",
        "diff.external=",
        "-c",
        "diff.trustExitCode=false",
        "-c",
        "core.pager=cat",
        "-c",
        "pager.branch=false",
        "-c",
        "sequence.editor=false",
        "-c",
        "core.editor=false",
        "-c",
        "gpg.program=false",
        "-c",
        "maintenance.auto=false",
        "-c",
        "gc.auto=0",
        "-c",
        "fetch.writeCommitGraph=false",
        "-c",
        "fetch.recurseSubmodules=false",
        "-c",
        "submodule.recurse=false",
        "-c",
        "filter.lfs.smudge=",
        "-c",
        "filter.lfs.required=false",
        "-c",
        "protocol.allow=never",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect::<Vec<_>>();
    if let Some(protocol) = protocol {
        arguments.extend([
            "-c".to_owned(),
            match protocol {
                GitRemoteProtocol::Https => "protocol.https.allow=always".to_owned(),
                GitRemoteProtocol::Ssh => "protocol.ssh.allow=always".to_owned(),
            },
        ]);
    }
    arguments
}

fn hardened_environment() -> BTreeMap<String, String> {
    BTreeMap::from([
        ("GCM_INTERACTIVE".to_owned(), "Never".to_owned()),
        ("GIT_CONFIG_GLOBAL".to_owned(), "/dev/null".to_owned()),
        ("GIT_CONFIG_NOSYSTEM".to_owned(), "1".to_owned()),
        ("GIT_LFS_SKIP_SMUDGE".to_owned(), "1".to_owned()),
        ("GIT_OPTIONAL_LOCKS".to_owned(), "0".to_owned()),
        ("GIT_TERMINAL_PROMPT".to_owned(), "0".to_owned()),
        ("HOME".to_owned(), "/nonexistent".to_owned()),
        ("LANG".to_owned(), "C".to_owned()),
        ("LC_ALL".to_owned(), "C".to_owned()),
        ("NO_COLOR".to_owned(), "1".to_owned()),
        ("XDG_CONFIG_HOME".to_owned(), "/nonexistent".to_owned()),
    ])
}

fn contains_prohibited_git_operation(arguments: &[String]) -> bool {
    const PROHIBITED: [&str; 22] = [
        "pull",
        "merge",
        "rebase",
        "reset",
        "clean",
        "checkout",
        "restore",
        "stash",
        "tag",
        "notes",
        "remote",
        "push",
        "am",
        "cherry-pick",
        "revert",
        "replace",
        "gc",
        "maintenance",
        "prune",
        "submodule",
        "filter-branch",
        "fast-import",
    ];
    arguments
        .iter()
        .any(|argument| PROHIBITED.contains(&argument.as_str()))
        || arguments.iter().any(|argument| {
            matches!(
                argument.as_str(),
                "--force" | "-f" | "--mirror" | "--tags" | "--all" | "--prune"
            ) || argument.starts_with("--force-with-lease")
                || argument.starts_with("--upload-pack")
                || argument.starts_with("--receive-pack")
                || argument.starts_with("--exec-path")
        })
}

fn transaction_ref(transaction_id: &str) -> Result<String, RepositorySafetyError> {
    validate_identifier(transaction_id)?;
    Ok(format!("refs/agentmage/fetch/{transaction_id}/source"))
}

fn validate_transaction_ref(
    reference: &str,
    transaction_id: &str,
) -> Result<(), RepositorySafetyError> {
    if reference != transaction_ref(transaction_id)? {
        return Err(RepositorySafetyError::RefDenied);
    }
    Ok(())
}

fn validate_branch_ref(reference: &str) -> Result<(), RepositorySafetyError> {
    validate_ref(reference, "refs/heads/")
}

fn validate_remote_tracking_ref(reference: &str) -> Result<(), RepositorySafetyError> {
    validate_ref(reference, "refs/remotes/")
}

fn validate_ref(reference: &str, prefix: &str) -> Result<(), RepositorySafetyError> {
    let suffix = reference
        .strip_prefix(prefix)
        .ok_or(RepositorySafetyError::RefDenied)?;
    if reference.len() > MAX_REF_BYTES
        || suffix.is_empty()
        || suffix.starts_with('.')
        || suffix.ends_with('.')
        || suffix.ends_with('/')
        || suffix.ends_with(".lock")
        || suffix.contains("..")
        || suffix.contains("//")
        || suffix.contains("@{")
        || suffix.chars().any(|value| {
            value.is_control()
                || value.is_whitespace()
                || matches!(value, '~' | '^' | ':' | '?' | '*' | '[' | '\\')
        })
    {
        return Err(RepositorySafetyError::RefDenied);
    }
    Ok(())
}

fn validate_identifier(value: &str) -> Result<(), RepositorySafetyError> {
    if valid_identifier(value) {
        Ok(())
    } else {
        Err(RepositorySafetyError::InvalidInput)
    }
}

fn valid_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_IDENTIFIER_BYTES
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
}

fn valid_host(value: &str) -> bool {
    if value.is_empty()
        || value.len() > MAX_HOST_BYTES
        || value.starts_with('.')
        || value.ends_with('.')
        || value.contains("..")
    {
        return false;
    }
    let (host, port) = value
        .rsplit_once(':')
        .map_or((value, None), |(host, port)| {
            if port.bytes().all(|byte| byte.is_ascii_digit()) {
                (host, Some(port))
            } else {
                (value, None)
            }
        });
    if port.is_some_and(|port| port.parse::<u16>().ok().is_none_or(|value| value == 0)) {
        return false;
    }
    host.split('.').all(|label| {
        !label.is_empty()
            && label.len() <= 63
            && !label.starts_with('-')
            && !label.ends_with('-')
            && label
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
    })
}

fn valid_repository_path(value: &str) -> bool {
    value.len() <= MAX_REPOSITORY_BYTES
        && value.split('/').count() == 2
        && value.split('/').all(|component| {
            !component.is_empty()
                && component != "."
                && component != ".."
                && !component.starts_with('.')
                && !component.ends_with('.')
                && component
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
        })
}

fn valid_object_id(value: &str) -> bool {
    matches!(value.len(), 40 | 64)
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}

fn require_sha(value: &str) -> Result<(), RepositorySafetyError> {
    if is_sha256(value) {
        Ok(())
    } else {
        Err(RepositorySafetyError::InvalidInput)
    }
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}

fn valid_platform_code(value: &str) -> bool {
    valid_identifier(value) && value.contains('.')
}

fn canonical_sha256(value: &impl Serialize) -> Result<String, RepositorySafetyError> {
    serde_json::to_vec(value)
        .map(|bytes| sha256_hex(&bytes))
        .map_err(|_| RepositorySafetyError::InvalidInput)
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut encoded = String::with_capacity(64);
    for byte in digest {
        use fmt::Write as _;
        write!(&mut encoded, "{byte:02x}").expect("writing to a String cannot fail");
    }
    encoded
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hash(label: &str) -> String {
        sha256_hex(label.as_bytes())
    }

    fn object(byte: char) -> String {
        std::iter::repeat_n(byte, 40).collect()
    }

    fn manifest() -> RepositoryPreservationManifest {
        RepositoryPreservationManifest::seal(RepositoryPreservationManifest {
            schema_version: 0,
            repository_sha256: hash("repository"),
            common_directory_sha256: hash("common"),
            object_format: "sha1".to_owned(),
            safe_ownership: true,
            head_object: Some(object('a')),
            current_branch: Some("refs/heads/main".to_owned()),
            upstream: Some("refs/remotes/origin/main".to_owned()),
            detached: false,
            index_sha256: hash("index"),
            path_dispositions_sha256: hash("paths"),
            untracked_count: 1,
            ignored_count: 2,
            local_branches_sha256: hash("branches"),
            remote_tracking_refs_sha256: hash("remote-refs"),
            tags_sha256: hash("tags"),
            notes_sha256: hash("notes"),
            stash_sha256: hash("stash"),
            replacement_refs_sha256: hash("replace"),
            reflogs_sha256: hash("reflogs"),
            agentmage_refs_sha256: hash("agentmage-refs"),
            worktrees_sha256: hash("worktrees"),
            submodules_sha256: hash("submodules"),
            lfs_sha256: hash("lfs"),
            object_database_sha256: hash("objects"),
            configuration_sha256: hash("config"),
            hooks_sha256: hash("hooks"),
            content_drivers_sha256: hash("drivers"),
            remotes_sha256: hash("remotes"),
            operations_sha256: hash("operations"),
            shallow: false,
            partial: false,
            hazardous_configuration: false,
            manifest_sha256: ZERO_SHA256.to_owned(),
        })
        .expect("manifest seals")
    }

    fn worktree(disposition: WorktreeDisposition) -> OwnedWorktreeRecord {
        OwnedWorktreeRecord::seal(OwnedWorktreeRecord {
            schema_version: 0,
            worktree_id: "worktree-1".to_owned(),
            task_id: "task-1".to_owned(),
            source_object: object('a'),
            branch_ref: "refs/heads/agentmage/tasks/task-1".to_owned(),
            worktree_path_sha256: hash("worktree-path"),
            owner_sha256: hash("owner"),
            grant_ids: vec!["grant-1".to_owned()],
            file_ownership_sha256: hash("files"),
            live_process_count: u32::from(disposition == WorktreeDisposition::Active),
            resource_budget_sha256: hash("budget"),
            retain_until_epoch_ms: 123,
            clean: disposition == WorktreeDisposition::CleanupEligible,
            recovery_retained: disposition == WorktreeDisposition::CleanupEligible,
            disposition,
            record_sha256: ZERO_SHA256.to_owned(),
        })
        .expect("worktree record seals")
    }

    #[test]
    fn remotes_are_canonical_credential_free_and_host_bound() {
        let https =
            GitRemoteIdentity::parse("https://github.com/AgentMage/fixture.git", "github.com")
                .expect("HTTPS parses");
        assert_eq!(https.repository, "AgentMage/fixture");
        let ssh = GitRemoteIdentity::parse(
            "ssh://git@git.example.test/team/repository.git",
            "git.example.test",
        )
        .expect("SSH parses");
        assert_eq!(ssh.protocol, GitRemoteProtocol::Ssh);

        for denied in [
            "http://github.com/AgentMage/fixture.git",
            "file:///tmp/repository",
            "git://github.com/AgentMage/fixture.git",
            "ext::command",
            "https://token@github.com/AgentMage/fixture.git",
            "git@github.com:AgentMage/fixture.git",
            "https://github.com/AgentMage/fixture.git?token=value",
        ] {
            assert_eq!(
                GitRemoteIdentity::parse(denied, "github.com"),
                Err(RepositorySafetyError::RemoteDenied),
                "{denied} must fail"
            );
        }
        assert_eq!(
            GitRemoteIdentity::parse(
                "https://github.com/AgentMage/fixture.git",
                "enterprise.example.test"
            ),
            Err(RepositorySafetyError::RemoteDenied)
        );
    }

    #[test]
    fn clone_and_fetch_use_only_exact_transaction_refs() {
        let remote =
            GitRemoteIdentity::parse("https://github.com/AgentMage/fixture.git", "github.com")
                .expect("remote parses");
        let clone = plan_clone(
            "transaction-1",
            remote.clone(),
            "refs/heads/agentmage/tasks/review",
            &hash("destination"),
            &hash("empty"),
        )
        .expect("clone plans");
        clone.verify().expect("clone verifies");
        assert_eq!(
            clone.transaction_ref.as_deref(),
            Some("refs/agentmage/fetch/transaction-1/source")
        );
        assert!(
            clone.invocations[0]
                .arguments
                .contains(&"--bare".to_owned())
        );
        let fetch = plan_fetch(
            "transaction-2",
            remote,
            "refs/heads/main",
            &hash("repository-path"),
            &manifest(),
        )
        .expect("fetch plans");
        let arguments = &fetch.invocations[0].arguments;
        assert!(arguments.contains(&"--atomic".to_owned()));
        assert!(arguments.contains(&"--no-tags".to_owned()));
        assert!(arguments.contains(&"--no-write-fetch-head".to_owned()));
        assert!(arguments.contains(&"--no-recurse-submodules".to_owned()));
        assert!(arguments.iter().all(|value| !value.starts_with('+')));
        assert!(
            arguments
                .iter()
                .all(|value| !value.contains("refs/remotes/"))
        );
    }

    #[test]
    fn hardened_invocations_have_no_generic_or_ambient_git_authority() {
        let plan = plan_fetch(
            "transaction-3",
            GitRemoteIdentity::parse(
                "ssh://git@git.example.test/team/repository.git",
                "git.example.test",
            )
            .expect("remote parses"),
            "refs/heads/review",
            &hash("repository-path"),
            &manifest(),
        )
        .expect("fetch plans");
        let invocation = &plan.invocations[0];
        assert_eq!(invocation.environment, hardened_environment());
        for variable in [
            "GIT_DIR",
            "GIT_WORK_TREE",
            "GIT_INDEX_FILE",
            "GIT_OBJECT_DIRECTORY",
            "GIT_ALTERNATE_OBJECT_DIRECTORIES",
            "GIT_SSH_COMMAND",
            "GIT_ASKPASS",
            "GIT_PROXY_COMMAND",
            "GIT_PAGER",
            "GIT_EDITOR",
            "GIT_TRACE",
        ] {
            assert!(!invocation.environment.contains_key(variable));
        }
        assert!(!contains_prohibited_git_operation(&invocation.arguments));
        assert!(
            invocation
                .arguments
                .contains(&"protocol.allow=never".to_owned())
        );
        assert!(
            invocation
                .arguments
                .contains(&"protocol.ssh.allow=always".to_owned())
        );
    }

    #[test]
    fn currentness_reports_dirty_untracked_stale_and_divergence() {
        let base = CurrentnessObservation {
            remote: GitRemoteIdentity::parse(
                "https://github.com/AgentMage/fixture.git",
                "github.com",
            )
            .expect("remote parses"),
            default_branch: "refs/heads/main".to_owned(),
            local_branch: Some("refs/heads/main".to_owned()),
            upstream: Some("refs/remotes/origin/main".to_owned()),
            local_object: Some(object('a')),
            fetched_object: object('b'),
            ahead_count: 0,
            behind_count: 1,
            dirty: false,
            untracked: false,
            fetched_at_epoch_ms: 100,
            observed_at_epoch_ms: 110,
            max_age_ms: 50,
            fetch_evidence_sha256: hash("fetch"),
        };
        assert_eq!(
            report_currentness(&base).expect("report").state,
            RepositoryCurrentness::Behind
        );
        let cases = [
            (true, false, 110, 0, 1, RepositoryCurrentness::Dirty),
            (false, true, 110, 0, 1, RepositoryCurrentness::Untracked),
            (false, false, 200, 0, 1, RepositoryCurrentness::Stale),
            (false, false, 110, 1, 0, RepositoryCurrentness::Ahead),
            (false, false, 110, 1, 1, RepositoryCurrentness::Diverged),
        ];
        for (dirty, untracked, observed, ahead, behind, expected) in cases {
            let value = CurrentnessObservation {
                dirty,
                untracked,
                observed_at_epoch_ms: observed,
                ahead_count: ahead,
                behind_count: behind,
                ..base.clone()
            };
            assert_eq!(report_currentness(&value).expect("report").state, expected);
        }
    }

    #[test]
    fn preservation_reconciliation_allows_only_operation_owned_fields() {
        let before = manifest();
        let mut after = before.clone();
        after.agentmage_refs_sha256 = hash("new-agentmage-ref");
        after.object_database_sha256 = hash("new-objects");
        after = RepositoryPreservationManifest::seal(after).expect("after seals");
        reconcile_preservation(&before, &after, RepositoryOwnedDelta::Fetch)
            .expect("fetch delta accepted");

        let mut changed_tag = after;
        changed_tag.tags_sha256 = hash("changed-tag");
        changed_tag = RepositoryPreservationManifest::seal(changed_tag).expect("tag manifest");
        assert_eq!(
            reconcile_preservation(&before, &changed_tag, RepositoryOwnedDelta::Fetch),
            Err(RepositorySafetyError::PreservationMismatch)
        );
    }

    #[test]
    fn worktree_registry_requires_stable_ownership_and_safe_cleanup() {
        let mut registry = WorktreeOwnershipRegistry::default();
        let active = worktree(WorktreeDisposition::Active);
        registry.register(active.clone()).expect("record registers");
        assert_eq!(registry.list().len(), 1);
        assert_eq!(
            registry.register(active.clone()),
            Err(RepositorySafetyError::WorktreeDenied)
        );

        let cleanup = worktree(WorktreeDisposition::CleanupEligible);
        registry
            .compare_and_swap("worktree-1", &active.record_sha256, cleanup.clone())
            .expect("record advances");
        assert!(cleanup.cleanup_eligible());
        plan_worktree_remove(
            "transaction-remove",
            &cleanup,
            &hash("repository-path"),
            &manifest(),
        )
        .expect("safe removal plans");
        assert_eq!(
            plan_worktree_remove(
                "transaction-remove",
                &active,
                &hash("repository-path"),
                &manifest(),
            ),
            Err(RepositorySafetyError::WorktreeDenied)
        );
    }

    #[test]
    fn fast_forward_is_proven_and_compare_and_swap_only() {
        let plan = plan_branch_fast_forward(
            "transaction-ff",
            "refs/heads/agentmage/tasks/review",
            &object('a'),
            &object('b'),
            true,
            &hash("repository-path"),
            &manifest(),
        )
        .expect("fast-forward plans");
        assert_eq!(
            plan.invocations
                .iter()
                .map(|item| item.kind)
                .collect::<Vec<_>>(),
            [
                GitInvocationKind::ProveAncestor,
                GitInvocationKind::UpdateRefCompareAndSwap,
            ]
        );
        let update = &plan.invocations[1].arguments;
        assert!(update.ends_with(&[
            "refs/heads/agentmage/tasks/review".to_owned(),
            object('b'),
            object('a'),
        ]));
        assert_eq!(
            plan_branch_fast_forward(
                "transaction-ff",
                "refs/heads/agentmage/tasks/review",
                &object('a'),
                &object('b'),
                false,
                &hash("repository-path"),
                &manifest(),
            ),
            Err(RepositorySafetyError::FastForwardDenied)
        );
    }

    #[test]
    fn transfer_paths_fail_on_stale_unowned_duplicate_or_rename_collision() {
        let accepted = vec![TransferPathObservation {
            path_sha256: hash("path"),
            approved_preimage_sha256: hash("preimage"),
            current_preimage_sha256: hash("preimage"),
            task_owned: true,
            renamed_from_sha256: Some(hash("old-path")),
        }];
        assert!(verify_transfer_paths(&accepted).is_ok());
        for changed in [
            TransferPathObservation {
                current_preimage_sha256: hash("changed"),
                ..accepted[0].clone()
            },
            TransferPathObservation {
                task_owned: false,
                ..accepted[0].clone()
            },
            TransferPathObservation {
                renamed_from_sha256: Some(hash("path")),
                ..accepted[0].clone()
            },
        ] {
            assert_eq!(
                verify_transfer_paths(&[changed]),
                Err(RepositorySafetyError::Collision)
            );
        }
        assert_eq!(
            verify_transfer_paths(&[accepted[0].clone(), accepted[0].clone()]),
            Err(RepositorySafetyError::Collision)
        );
    }

    #[test]
    fn failed_operation_requires_a_byte_identical_manifest() {
        let plan = plan_fetch(
            "transaction-failed",
            GitRemoteIdentity::parse("https://github.com/AgentMage/fixture.git", "github.com")
                .expect("remote parses"),
            "refs/heads/main",
            &hash("repository-path"),
            &manifest(),
        )
        .expect("fetch plans");
        let before = manifest();
        let receipt = reconcile_operation(
            &plan,
            RepositoryPlatformResult {
                before: before.clone(),
                after: before,
                outcome: OperationOutcome::Failed,
                cleanup_verified: true,
                platform_code: "linux.git.failed".to_owned(),
            },
            "authority-transaction-1",
            "operation-attempt-1",
        )
        .expect("failed attempt receipts");
        assert_eq!(receipt.outcome, OperationOutcome::Failed);
        assert!(is_sha256(&receipt.receipt_sha256));
    }
}
