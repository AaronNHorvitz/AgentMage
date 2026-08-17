//! Pinned Git observation and local owned-worktree execution for Linux.

use std::fmt;
use std::fmt::Write as _;
use std::fs;
use std::io::{Read, Write};
use std::os::fd::AsRawFd as _;
use std::os::unix::ffi::OsStrExt as _;
use std::os::unix::fs::MetadataExt as _;
use std::os::unix::process::CommandExt as _;
use std::path::{Component, Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use agentmage_kernel_contracts::OperationOutcome;
use agentmage_kernel_engine::propagation::CancellationToken;
use agentmage_kernel_engine::repository_inspection::{
    BoundedRepositoryInspectionExecutor, RepositoryInspectionLaunchPermit,
    RepositoryInspectionPlatformResult, RepositoryInspectionTermination,
};
use agentmage_kernel_engine::repository_safety::{
    BoundedRepositoryExecutor, GitInvocationKind, HardenedGitInvocation, RepositoryLaunchPermit,
    RepositoryOperation, RepositoryOperationPlan, RepositoryPlatformResult,
    RepositoryPreservationManifest,
};
use rustix::fd::OwnedFd;
use rustix::fs::{FileType, Mode, OFlags, fstat, open};
use rustix::io::pread;
use rustix::process::{Pid, Signal, getuid, kill_process_group, test_kill_process_group};
use sha2::{Digest, Sha256};

const HASH_BUFFER_BYTES: usize = 64 * 1024;
const MAX_OBSERVATION_BYTES: usize = 4 * 1024 * 1024;
const MAX_HASHED_FILE_BYTES: u64 = 64 * 1024 * 1024;
const MAX_METADATA_ENTRIES: usize = 16_384;
const PROCESS_TIMEOUT: Duration = Duration::from_secs(30);
const INSPECTION_PROCESS_TIMEOUT: Duration = Duration::from_secs(15);
const POLL_INTERVAL: Duration = Duration::from_millis(10);
const PROCESS_GROUP_GRACE: Duration = Duration::from_millis(250);

/// Stable failure class at the Linux repository boundary.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LinuxRepositoryErrorKind {
    /// The pinned Git executable changed or is not trusted.
    InvalidGitArtifact,
    /// A repository, worktree, or management path is unsafe or mismatched.
    InvalidPath,
    /// A bounded Git observation failed or exceeded its limit.
    ObservationFailed,
    /// A plan is malformed, stale, networked, or outside the admitted local surface.
    PlanDenied,
    /// A local Git process failed, timed out, or could not be reconciled.
    ExecutionFailed,
}

impl LinuxRepositoryErrorKind {
    /// Returns a stable content-free failure code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidGitArtifact => "linux.repository.git_artifact.invalid",
            Self::InvalidPath => "linux.repository.path.invalid",
            Self::ObservationFailed => "linux.repository.observation.failed",
            Self::PlanDenied => "linux.repository.plan.denied",
            Self::ExecutionFailed => "linux.repository.execution.failed",
        }
    }
}

/// Redacted Linux repository error.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LinuxRepositoryError {
    kind: LinuxRepositoryErrorKind,
}

impl LinuxRepositoryError {
    /// Returns the stable failure class.
    #[must_use]
    pub const fn kind(self) -> LinuxRepositoryErrorKind {
        self.kind
    }
}

impl fmt::Display for LinuxRepositoryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.kind.code())
    }
}

impl std::error::Error for LinuxRepositoryError {}

/// Descriptor-held, root-owned Git executable identity.
pub struct LinuxGitArtifact {
    descriptor: OwnedFd,
    launch_path: PathBuf,
    sha256: String,
}

impl LinuxGitArtifact {
    /// Verifies and holds one exact root-owned, non-writable Git executable.
    pub fn verify(path: impl AsRef<Path>) -> Result<Self, LinuxRepositoryError> {
        verify_git_artifact(path.as_ref())
    }

    /// Returns the exact executable digest.
    #[must_use]
    pub fn sha256(&self) -> &str {
        &self.sha256
    }

    /// Returns the held launch path for sibling platform adapters.
    pub(crate) fn launch_path(&self) -> &Path {
        &self.launch_path
    }
}

impl fmt::Debug for LinuxGitArtifact {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("LinuxGitArtifact")
            .field("launch_path", &self.launch_path)
            .field("sha256", &self.sha256)
            .finish_non_exhaustive()
    }
}

/// Verified existing repository and AgentMage-owned management root.
#[derive(Clone, Debug)]
pub struct LinuxRepositoryScope {
    checkout_root: PathBuf,
    git_directory: PathBuf,
    owned_root: PathBuf,
    repository_path_sha256: String,
}

impl LinuxRepositoryScope {
    /// Verifies canonical, non-symlinked, current-user-owned repository paths.
    pub fn verify(
        checkout_root: impl AsRef<Path>,
        git_directory: impl AsRef<Path>,
        owned_root: impl AsRef<Path>,
    ) -> Result<Self, LinuxRepositoryError> {
        let checkout_root = verify_owned_directory(checkout_root.as_ref())?;
        let git_directory = verify_owned_directory(git_directory.as_ref())?;
        let owned_root = verify_owned_directory(owned_root.as_ref())?;
        if checkout_root == owned_root
            || git_directory == owned_root
            || owned_root.starts_with(&checkout_root)
            || checkout_root.starts_with(&owned_root)
            || !git_directory.starts_with(&checkout_root)
        {
            return Err(error(LinuxRepositoryErrorKind::InvalidPath));
        }
        let repository_path_sha256 = path_identity(&checkout_root)?;
        Ok(Self {
            checkout_root,
            git_directory,
            owned_root,
            repository_path_sha256,
        })
    }

    /// Returns the digest expected in an exact repository plan.
    #[must_use]
    pub fn repository_path_sha256(&self) -> &str {
        &self.repository_path_sha256
    }

    fn worktree_path(&self) -> PathBuf {
        self.owned_root.join("worktree")
    }

    /// Returns the verified checkout root for sibling platform adapters.
    pub(crate) fn checkout_root(&self) -> &Path {
        &self.checkout_root
    }

    /// Returns the verified Git directory for sibling platform adapters.
    pub(crate) fn git_directory(&self) -> &Path {
        &self.git_directory
    }

    /// Returns the AgentMage-owned management root for sibling platform adapters.
    pub(crate) fn owned_root(&self) -> &Path {
        &self.owned_root
    }
}

/// Computes the Linux canonical identity used by repository operation plans.
pub fn linux_repository_path_sha256(
    path: impl AsRef<Path>,
) -> Result<String, LinuxRepositoryError> {
    path_identity(&verify_owned_directory(path.as_ref())?)
}

/// Pinned, bounded collector for one content-minimized repository manifest.
#[derive(Debug)]
pub struct LinuxRepositoryCollector {
    git: LinuxGitArtifact,
}

impl LinuxRepositoryCollector {
    /// Creates an inert collector around one already verified Git artifact.
    #[must_use]
    pub const fn new(git: LinuxGitArtifact) -> Self {
        Self { git }
    }

    /// Returns the held Git artifact for sibling platform adapters.
    pub(crate) fn git(&self) -> &LinuxGitArtifact {
        &self.git
    }

    /// Collects a bounded manifest without retaining path names, config values, or file content.
    pub fn collect(
        &self,
        scope: &LinuxRepositoryScope,
    ) -> Result<RepositoryPreservationManifest, LinuxRepositoryError> {
        revalidate_git_artifact(&self.git)?;
        let object_format =
            text_required(self.observe(scope, &["rev-parse", "--show-object-format"])?)?;
        let head_object = self
            .observe_optional(scope, &["rev-parse", "--verify", "HEAD"])?
            .map(text_required)
            .transpose()?;
        let symbolic_head = self
            .observe_optional(scope, &["symbolic-ref", "--quiet", "HEAD"])?
            .map(text_required)
            .transpose()?;
        let detached = symbolic_head.is_none();
        let upstream = if let Some(branch) = &symbolic_head {
            self.observe_optional(
                scope,
                &["for-each-ref", "--format=%(upstream)", "--count=1", branch],
            )?
            .map(text_required)
            .transpose()?
            .filter(|value| !value.is_empty())
        } else {
            None
        };

        let status = self.observe(
            scope,
            &[
                "status",
                "--porcelain=v2",
                "-z",
                "--untracked-files=all",
                "--ignored=matching",
                "--no-renames",
            ],
        )?;
        let (untracked_count, ignored_count) = status_counts(&status)?;
        let config = self.observe(
            scope,
            &["config", "--local", "--no-includes", "--null", "--list"],
        )?;
        let config_lower = config
            .iter()
            .map(u8::to_ascii_lowercase)
            .collect::<Vec<_>>();

        let local_branches = self.observe(
            scope,
            &[
                "for-each-ref",
                "--format=%(refname)%00%(objectname)%00",
                "--exclude=refs/heads/agentmage/**",
                "refs/heads",
            ],
        )?;
        let agent_branches = self.observe(
            scope,
            &[
                "for-each-ref",
                "--format=%(refname)%00%(objectname)%00",
                "refs/heads/agentmage",
                "refs/agentmage",
            ],
        )?;
        let remote_refs = self.ref_observation(scope, "refs/remotes")?;
        let tags = self.ref_observation(scope, "refs/tags")?;
        let notes = self.ref_observation(scope, "refs/notes")?;
        let stash = self.ref_observation(scope, "refs/stash")?;
        let replacements = self.ref_observation(scope, "refs/replace")?;
        let worktrees = self.observe(scope, &["worktree", "list", "--porcelain", "-z"])?;
        let objects = self.observe(scope, &["count-objects", "-v"])?;

        let index_sha256 = hash_optional_file(&scope.git_directory.join("index"))?;
        let gitmodules = hash_optional_file(&scope.checkout_root.join(".gitmodules"))?;
        let attributes_path = scope.checkout_root.join(".gitattributes");
        let attributes = read_optional_bounded(&attributes_path)?;
        let hooks = hash_metadata_tree(&scope.git_directory.join("hooks"), false)?;
        let user_reflogs = hash_reflog_tree(&scope.git_directory.join("logs"), true)?;
        let agent_reflogs = hash_reflog_tree(
            &scope.git_directory.join("logs/refs/heads/agentmage"),
            false,
        )?;
        let operations = operation_metadata(&scope.git_directory)?;
        let alternates = scope.git_directory.join("objects/info/alternates").exists();
        let shallow = scope.git_directory.join("shallow").exists();
        let partial = contains_any(
            &config_lower,
            &[
                b"extensions.partialclone",
                b"promisor=true",
                b"promisor\ntrue",
            ],
        );
        let executable_config = hazardous_config(&config_lower);
        let executable_attributes = attributes.as_deref().is_some_and(hazardous_attributes);
        let hook_present = directory_has_active_hooks(&scope.git_directory.join("hooks"))?;
        let hazardous_configuration = executable_config
            || executable_attributes
            || hook_present
            || alternates
            || !replacements.is_empty();

        let mut agent_state = agent_branches;
        agent_state.extend(agent_reflogs);
        RepositoryPreservationManifest::seal(RepositoryPreservationManifest {
            schema_version: 0,
            repository_sha256: scope.repository_path_sha256.clone(),
            common_directory_sha256: path_identity(&scope.git_directory)?,
            object_format,
            safe_ownership: true,
            head_object,
            current_branch: symbolic_head,
            upstream,
            detached,
            index_sha256,
            path_dispositions_sha256: hash(&status),
            untracked_count,
            ignored_count,
            local_branches_sha256: hash(&local_branches),
            remote_tracking_refs_sha256: hash(&remote_refs),
            tags_sha256: hash(&tags),
            notes_sha256: hash(&notes),
            stash_sha256: hash(&stash),
            replacement_refs_sha256: hash(&replacements),
            reflogs_sha256: hash(&user_reflogs),
            agentmage_refs_sha256: hash(&agent_state),
            worktrees_sha256: hash(&worktrees),
            submodules_sha256: gitmodules,
            lfs_sha256: hash(attributes.as_deref().unwrap_or_default()),
            object_database_sha256: hash(&objects),
            configuration_sha256: hash(&config),
            hooks_sha256: hash(&hooks),
            content_drivers_sha256: hash(attributes.as_deref().unwrap_or_default()),
            remotes_sha256: hash(&config),
            operations_sha256: hash(&operations),
            shallow,
            partial,
            hazardous_configuration,
            manifest_sha256: String::new(),
        })
        .map_err(|_| error(LinuxRepositoryErrorKind::ObservationFailed))
    }

    fn ref_observation(
        &self,
        scope: &LinuxRepositoryScope,
        reference: &str,
    ) -> Result<Vec<u8>, LinuxRepositoryError> {
        self.observe(
            scope,
            &[
                "for-each-ref",
                "--format=%(refname)%00%(objectname)%00",
                reference,
            ],
        )
    }

    fn observe(
        &self,
        scope: &LinuxRepositoryScope,
        arguments: &[&str],
    ) -> Result<Vec<u8>, LinuxRepositoryError> {
        self.observe_status(scope, arguments)?
            .ok_or_else(|| error(LinuxRepositoryErrorKind::ObservationFailed))
    }

    fn observe_optional(
        &self,
        scope: &LinuxRepositoryScope,
        arguments: &[&str],
    ) -> Result<Option<Vec<u8>>, LinuxRepositoryError> {
        self.observe_status(scope, arguments)
    }

    fn observe_status(
        &self,
        scope: &LinuxRepositoryScope,
        arguments: &[&str],
    ) -> Result<Option<Vec<u8>>, LinuxRepositoryError> {
        revalidate_git_artifact(&self.git)?;
        let output = Command::new(&self.git.launch_path)
            .env_clear()
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_TERMINAL_PROMPT", "0")
            .env("GIT_OPTIONAL_LOCKS", "0")
            .env("GIT_LFS_SKIP_SMUDGE", "1")
            .env("GCM_INTERACTIVE", "Never")
            .env("HOME", "/nonexistent")
            .env("XDG_CONFIG_HOME", "/nonexistent")
            .env("LANG", "C")
            .env("LC_ALL", "C")
            .args(hardened_observation_prefix())
            .arg(format!("--git-dir={}", scope.git_directory.display()))
            .arg(format!("--work-tree={}", scope.checkout_root.display()))
            .args(arguments)
            .stdin(Stdio::null())
            .output()
            .map_err(|_| error(LinuxRepositoryErrorKind::ObservationFailed))?;
        if output.stdout.len() > MAX_OBSERVATION_BYTES
            || output.stderr.len() > MAX_OBSERVATION_BYTES
        {
            return Err(error(LinuxRepositoryErrorKind::ObservationFailed));
        }
        if output.status.success() {
            Ok(Some(output.stdout))
        } else if matches!(output.status.code(), Some(1 | 128)) {
            Ok(None)
        } else {
            Err(error(LinuxRepositoryErrorKind::ObservationFailed))
        }
    }
}

/// Pinned offline executor for one kernel-validated read-only Git inspection.
#[derive(Debug)]
pub struct LinuxBoundedRepositoryInspectionExecutor {
    git: LinuxGitArtifact,
}

impl LinuxBoundedRepositoryInspectionExecutor {
    /// Creates an inert executor around one descriptor-held Git artifact.
    #[must_use]
    pub const fn new(git: LinuxGitArtifact) -> Self {
        Self { git }
    }

    fn run(
        &self,
        permit: RepositoryInspectionLaunchPermit<'_>,
        working_directory: &crate::LinuxAuthorizedWorkspace,
        cancellation: &CancellationToken,
    ) -> RepositoryInspectionPlatformResult {
        let prepared = permit.prepared();
        if revalidate_git_artifact(&self.git).is_err() || working_directory.revalidate().is_err() {
            return failed_inspection("linux.git-inspection.preflight.failed");
        }
        let Ok(root) = working_directory.reopen_root_directory() else {
            return failed_inspection("linux.git-inspection.root.failed");
        };
        let root_path = format!("/proc/self/fd/{}", root.as_raw_fd());
        let mut command = Command::new(&self.git.launch_path);
        command
            .env_clear()
            .envs(prepared.environment().iter())
            .args(prepared.arguments())
            .current_dir(root_path)
            .process_group(0)
            .stdin(if prepared.stdin().is_empty() {
                Stdio::null()
            } else {
                Stdio::piped()
            })
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let started = Instant::now();
        let Ok(mut child) = command.spawn() else {
            return failed_inspection("linux.git-inspection.launch.failed");
        };
        let Some(process_group) = i32::try_from(child.id()).ok().and_then(Pid::from_raw) else {
            let _ = child.kill();
            let _ = child.wait();
            return failed_inspection("linux.git-inspection.process-group.failed");
        };
        if !prepared.stdin().is_empty() {
            let wrote_input = child
                .stdin
                .take()
                .is_some_and(|mut input| input.write_all(prepared.stdin()).is_ok());
            if !wrote_input {
                let cleanup = terminate_inspection_group(&mut child, process_group);
                return failed_inspection_with_cleanup(
                    "linux.git-inspection.stdin.failed",
                    cleanup,
                    started.elapsed(),
                );
            }
        }
        let Some(stdout) = child.stdout.take() else {
            let cleanup = terminate_inspection_group(&mut child, process_group);
            return failed_inspection_with_cleanup(
                "linux.git-inspection.stdout.failed",
                cleanup,
                started.elapsed(),
            );
        };
        let Some(stderr) = child.stderr.take() else {
            let cleanup = terminate_inspection_group(&mut child, process_group);
            return failed_inspection_with_cleanup(
                "linux.git-inspection.stderr.failed",
                cleanup,
                started.elapsed(),
            );
        };
        let stdout_reader = thread::spawn(move || read_inspection_output(stdout));
        let stderr_reader = thread::spawn(move || read_inspection_output(stderr));
        let deadline = started + INSPECTION_PROCESS_TIMEOUT;
        let (mut termination, mut exit_code, mut cleanup_verified, platform_code) = loop {
            match child.try_wait() {
                Ok(Some(status)) => {
                    let cleanup = reconcile_inspection_group(process_group);
                    let Some(code) = status.code() else {
                        break (
                            RepositoryInspectionTermination::LaunchFailed,
                            None,
                            cleanup,
                            "linux.git-inspection.signal",
                        );
                    };
                    break (
                        RepositoryInspectionTermination::Exited,
                        Some(code),
                        cleanup,
                        "linux.git-inspection.exited",
                    );
                }
                Ok(None) if cancellation.is_cancelled() => {
                    let cleanup = terminate_inspection_group(&mut child, process_group);
                    break (
                        RepositoryInspectionTermination::Cancelled,
                        None,
                        cleanup,
                        "linux.git-inspection.cancelled",
                    );
                }
                Ok(None) if Instant::now() >= deadline => {
                    let cleanup = terminate_inspection_group(&mut child, process_group);
                    break (
                        RepositoryInspectionTermination::TimedOut,
                        None,
                        cleanup,
                        "linux.git-inspection.timed-out",
                    );
                }
                Ok(None) => thread::sleep(POLL_INTERVAL),
                Err(_) => {
                    let cleanup = terminate_inspection_group(&mut child, process_group);
                    break (
                        RepositoryInspectionTermination::LaunchFailed,
                        None,
                        cleanup,
                        "linux.git-inspection.wait.failed",
                    );
                }
            }
        };
        let Ok(stdout) = stdout_reader.join().unwrap_or(Err(())) else {
            return failed_inspection_with_cleanup(
                "linux.git-inspection.stdout.read-failed",
                cleanup_verified,
                started.elapsed(),
            );
        };
        let Ok(stderr) = stderr_reader.join().unwrap_or(Err(())) else {
            return failed_inspection_with_cleanup(
                "linux.git-inspection.stderr.read-failed",
                cleanup_verified,
                started.elapsed(),
            );
        };
        let mut platform_code = platform_code;
        if stdout.exceeded || stderr.exceeded {
            termination = RepositoryInspectionTermination::OutputLimit;
            exit_code = None;
            platform_code = "linux.git-inspection.output-limit";
        }
        if working_directory.revalidate().is_err() {
            termination = RepositoryInspectionTermination::LaunchFailed;
            exit_code = None;
            cleanup_verified = false;
            platform_code = "linux.git-inspection.root.changed";
        }
        RepositoryInspectionPlatformResult {
            termination,
            exit_code,
            stdout_sha256: hash(&stdout.retained),
            stdout_bytes: stdout.retained.len() as u64,
            stdout: stdout.retained,
            stderr_sha256: hash(&stderr.retained),
            stderr_bytes: stderr.retained.len() as u64,
            elapsed_ms: elapsed_millis(started.elapsed()),
            descendants_terminated: cleanup_verified,
            platform_code: platform_code.to_owned(),
        }
    }
}

impl BoundedRepositoryInspectionExecutor for LinuxBoundedRepositoryInspectionExecutor {
    type WorkingDirectory = crate::LinuxAuthorizedWorkspace;

    fn execute(
        &mut self,
        permit: RepositoryInspectionLaunchPermit<'_>,
        working_directory: &Self::WorkingDirectory,
        cancellation: &CancellationToken,
    ) -> RepositoryInspectionPlatformResult {
        self.run(permit, working_directory, cancellation)
    }
}

struct InspectionOutput {
    retained: Vec<u8>,
    exceeded: bool,
}

fn read_inspection_output(mut input: impl Read) -> Result<InspectionOutput, ()> {
    let mut retained = Vec::with_capacity(MAX_OBSERVATION_BYTES);
    let mut buffer = [0_u8; HASH_BUFFER_BYTES];
    loop {
        let count = input.read(&mut buffer).map_err(|_| ())?;
        if count == 0 {
            break;
        }
        if retained.len().saturating_add(count) > MAX_OBSERVATION_BYTES {
            let remaining = MAX_OBSERVATION_BYTES.saturating_sub(retained.len());
            retained.extend_from_slice(&buffer[..remaining]);
            return Ok(InspectionOutput {
                retained,
                exceeded: true,
            });
        }
        retained.extend_from_slice(&buffer[..count]);
    }
    Ok(InspectionOutput {
        retained,
        exceeded: false,
    })
}

fn terminate_inspection_group(child: &mut Child, process_group: Pid) -> bool {
    let _ = kill_process_group(process_group, Signal::TERM);
    let deadline = Instant::now() + PROCESS_GROUP_GRACE;
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if Instant::now() < deadline => thread::sleep(POLL_INTERVAL),
            Ok(None) | Err(_) => {
                let _ = kill_process_group(process_group, Signal::KILL);
                let _ = child.wait();
                break;
            }
        }
    }
    reconcile_inspection_group(process_group)
}

fn reconcile_inspection_group(process_group: Pid) -> bool {
    match test_kill_process_group(process_group) {
        Err(rustix::io::Errno::SRCH) => true,
        Ok(()) => {
            let _ = kill_process_group(process_group, Signal::KILL);
            thread::sleep(POLL_INTERVAL);
            matches!(
                test_kill_process_group(process_group),
                Err(rustix::io::Errno::SRCH)
            )
        }
        Err(_) => false,
    }
}

fn failed_inspection(code: &str) -> RepositoryInspectionPlatformResult {
    failed_inspection_with_cleanup(code, true, Duration::ZERO)
}

fn failed_inspection_with_cleanup(
    code: &str,
    cleanup_verified: bool,
    elapsed: Duration,
) -> RepositoryInspectionPlatformResult {
    RepositoryInspectionPlatformResult {
        termination: RepositoryInspectionTermination::LaunchFailed,
        exit_code: None,
        stdout: Vec::new(),
        stdout_sha256: hash(&[]),
        stdout_bytes: 0,
        stderr_sha256: hash(&[]),
        stderr_bytes: 0,
        elapsed_ms: elapsed_millis(elapsed),
        descendants_terminated: cleanup_verified,
        platform_code: code.to_owned(),
    }
}

fn elapsed_millis(elapsed: Duration) -> u64 {
    u64::try_from(elapsed.as_millis()).unwrap_or(u64::MAX)
}

/// Local-only Linux executor for exact worktree and compare-and-swap plans.
///
/// Network invocations remain denied until the later authenticated transport and
/// process-tree containment stories are complete. This executor is not registered
/// in a product profile by Sprint 42.
#[derive(Debug)]
pub struct LinuxRepositoryExecutor {
    collector: LinuxRepositoryCollector,
    scope: LinuxRepositoryScope,
}

impl LinuxRepositoryExecutor {
    /// Creates an inert local repository executor.
    #[must_use]
    pub const fn new(collector: LinuxRepositoryCollector, scope: LinuxRepositoryScope) -> Self {
        Self { collector, scope }
    }

    fn run(
        &mut self,
        plan: &RepositoryOperationPlan,
        approved_before: &RepositoryPreservationManifest,
        cancellation: &CancellationToken,
    ) -> RepositoryPlatformResult {
        let current = match self.collector.collect(&self.scope) {
            Ok(value) => value,
            Err(_) => return unchanged_failure(approved_before, "linux.git.preflight.failed"),
        };
        if plan.verify().is_err()
            || current != *approved_before
            || plan.preservation_manifest_sha256 != current.manifest_sha256
            || plan.repository_path_sha256 != self.scope.repository_path_sha256
            || plan.operation == RepositoryOperation::Clone
            || plan.invocations.iter().any(|invocation| invocation.network)
        {
            return unchanged_failure(approved_before, "linux.git.plan.denied");
        }
        if plan.operation == RepositoryOperation::WorktreeCreate
            && (self.scope.worktree_path().exists()
                || plan.worktree_path_sha256.as_deref()
                    != Some(hash(self.scope.worktree_path().as_os_str().as_bytes()).as_str()))
        {
            return unchanged_failure(approved_before, "linux.git.worktree.destination");
        }
        if plan.operation == RepositoryOperation::WorktreeRemove
            && plan.worktree_path_sha256.as_deref()
                != linux_repository_path_sha256(self.scope.worktree_path())
                    .ok()
                    .as_deref()
        {
            return unchanged_failure(approved_before, "linux.git.worktree.identity");
        }

        for invocation in &plan.invocations {
            let status = self.run_invocation(invocation, cancellation);
            if !status.is_success() {
                let after = self
                    .collector
                    .collect(&self.scope)
                    .unwrap_or_else(|_| current.clone());
                let outcome = if after == current {
                    status.outcome()
                } else {
                    OperationOutcome::Uncertain
                };
                return RepositoryPlatformResult {
                    outcome,
                    before: current,
                    after,
                    cleanup_verified: status.cleanup_verified(),
                    platform_code: status.code().to_owned(),
                };
            }
        }
        match self.collector.collect(&self.scope) {
            Ok(after) => RepositoryPlatformResult {
                outcome: OperationOutcome::Succeeded,
                before: current,
                after,
                cleanup_verified: true,
                platform_code: "linux.git.succeeded".to_owned(),
            },
            Err(_) => RepositoryPlatformResult {
                outcome: OperationOutcome::Uncertain,
                before: current.clone(),
                after: current,
                cleanup_verified: false,
                platform_code: "linux.git.postflight.failed".to_owned(),
            },
        }
    }

    fn run_invocation(
        &self,
        invocation: &HardenedGitInvocation,
        cancellation: &CancellationToken,
    ) -> InvocationStatus {
        if invocation.verify().is_err()
            || invocation.network
            || revalidate_git_artifact(&self.collector.git).is_err()
        {
            return InvocationStatus::Denied;
        }
        let Some(arguments) = map_arguments(invocation, &self.scope) else {
            return InvocationStatus::Denied;
        };
        let mut command = Command::new(&self.collector.git.launch_path);
        command
            .env_clear()
            .envs(invocation.environment.iter())
            .args(arguments)
            .current_dir(&self.scope.owned_root)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let Ok(mut child) = command.spawn() else {
            return InvocationStatus::Failed;
        };
        wait_bounded(&mut child, cancellation)
    }
}

impl BoundedRepositoryExecutor for LinuxRepositoryExecutor {
    fn execute(
        &mut self,
        permit: RepositoryLaunchPermit<'_>,
        cancellation: &CancellationToken,
    ) -> RepositoryPlatformResult {
        self.run(permit.plan(), permit.before(), cancellation)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum InvocationStatus {
    Succeeded,
    Failed,
    Denied,
    Cancelled,
    TimedOut,
    OutputLimit,
    CleanupFailed,
}

impl InvocationStatus {
    const fn is_success(self) -> bool {
        matches!(self, Self::Succeeded)
    }

    const fn cleanup_verified(self) -> bool {
        !matches!(self, Self::CleanupFailed)
    }

    const fn outcome(self) -> OperationOutcome {
        match self {
            Self::Succeeded => OperationOutcome::Succeeded,
            Self::Denied => OperationOutcome::Denied,
            Self::Cancelled => OperationOutcome::Cancelled,
            Self::TimedOut => OperationOutcome::TimedOut,
            Self::Failed | Self::OutputLimit | Self::CleanupFailed => OperationOutcome::Failed,
        }
    }

    const fn code(self) -> &'static str {
        match self {
            Self::Succeeded => "linux.git.invocation.succeeded",
            Self::Failed => "linux.git.invocation.failed",
            Self::Denied => "linux.git.invocation.denied",
            Self::Cancelled => "linux.git.invocation.cancelled",
            Self::TimedOut => "linux.git.invocation.timed_out",
            Self::OutputLimit => "linux.git.invocation.output_limit",
            Self::CleanupFailed => "linux.git.invocation.cleanup_failed",
        }
    }
}

fn wait_bounded(child: &mut Child, cancellation: &CancellationToken) -> InvocationStatus {
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    let Some(stdout) = stdout else {
        return terminate_child(child, InvocationStatus::CleanupFailed);
    };
    let Some(stderr) = stderr else {
        return terminate_child(child, InvocationStatus::CleanupFailed);
    };
    let stdout_reader = thread::spawn(move || read_bounded(stdout));
    let stderr_reader = thread::spawn(move || read_bounded(stderr));
    let deadline = Instant::now() + PROCESS_TIMEOUT;
    let (status, exit) = loop {
        match child.try_wait() {
            Ok(Some(exit)) => break (InvocationStatus::Succeeded, Some(exit)),
            Ok(None) if cancellation.is_cancelled() => {
                break (terminate_child(child, InvocationStatus::Cancelled), None);
            }
            Ok(None) if Instant::now() >= deadline => {
                break (terminate_child(child, InvocationStatus::TimedOut), None);
            }
            Ok(None) => thread::sleep(POLL_INTERVAL),
            Err(_) => {
                break (
                    terminate_child(child, InvocationStatus::CleanupFailed),
                    None,
                );
            }
        }
    };
    let stdout = stdout_reader.join().unwrap_or(false);
    let stderr = stderr_reader.join().unwrap_or(false);
    if !stdout || !stderr {
        InvocationStatus::OutputLimit
    } else if status != InvocationStatus::Succeeded {
        status
    } else if exit.is_some_and(|value| value.success()) {
        InvocationStatus::Succeeded
    } else {
        InvocationStatus::Failed
    }
}

fn terminate_child(child: &mut Child, outcome: InvocationStatus) -> InvocationStatus {
    if child.kill().is_ok() && child.wait().is_ok() {
        outcome
    } else {
        InvocationStatus::CleanupFailed
    }
}

fn read_bounded(mut input: impl Read) -> bool {
    let mut total = 0_usize;
    let mut buffer = [0_u8; HASH_BUFFER_BYTES];
    loop {
        let Ok(count) = input.read(&mut buffer) else {
            return false;
        };
        if count == 0 {
            return true;
        }
        total = total.saturating_add(count);
        if total > MAX_OBSERVATION_BYTES {
            return false;
        }
    }
}

fn map_arguments(
    invocation: &HardenedGitInvocation,
    scope: &LinuxRepositoryScope,
) -> Option<Vec<String>> {
    let worktree = scope.worktree_path();
    let git_directory = scope.git_directory.to_str()?;
    let worktree = worktree.to_str()?;
    let mut mapped = Vec::with_capacity(invocation.arguments.len());
    for argument in &invocation.arguments {
        let value = if argument == "--git-dir=/repo" {
            format!("--git-dir={git_directory}")
        } else if argument == "/owned/worktree" {
            worktree.to_owned()
        } else if argument == "/owned/repository"
            || argument.contains("/repo")
            || argument.contains("/owned/")
        {
            return None;
        } else {
            argument.clone()
        };
        mapped.push(value);
    }
    let expected_kind = match invocation.kind {
        GitInvocationKind::WorktreeCreate => {
            mapped.iter().any(|value| value == "worktree")
                && mapped.iter().any(|value| value == "add")
        }
        GitInvocationKind::WorktreeRemove => {
            mapped.iter().any(|value| value == "worktree")
                && mapped.iter().any(|value| value == "remove")
        }
        GitInvocationKind::ProveAncestor => mapped.iter().any(|value| value == "merge-base"),
        GitInvocationKind::UpdateRefCompareAndSwap => {
            mapped.iter().any(|value| value == "update-ref")
        }
        GitInvocationKind::InitializeOwnedRepository | GitInvocationKind::FetchExactRef => false,
    };
    expected_kind.then_some(mapped)
}

fn unchanged_failure(
    before: &RepositoryPreservationManifest,
    code: &str,
) -> RepositoryPlatformResult {
    RepositoryPlatformResult {
        outcome: OperationOutcome::Denied,
        before: before.clone(),
        after: before.clone(),
        cleanup_verified: true,
        platform_code: code.to_owned(),
    }
}

fn hardened_observation_prefix() -> Vec<&'static str> {
    vec![
        "--no-pager",
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
}

fn verify_git_artifact(path: &Path) -> Result<LinuxGitArtifact, LinuxRepositoryError> {
    if !path.is_absolute() {
        return Err(error(LinuxRepositoryErrorKind::InvalidGitArtifact));
    }
    verify_root_owned_path(path)?;
    let descriptor = open(
        path,
        OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::CLOEXEC | OFlags::NONBLOCK,
        Mode::empty(),
    )
    .map_err(|_| error(LinuxRepositoryErrorKind::InvalidGitArtifact))?;
    let stat =
        fstat(&descriptor).map_err(|_| error(LinuxRepositoryErrorKind::InvalidGitArtifact))?;
    if FileType::from_raw_mode(stat.st_mode) != FileType::RegularFile
        || stat.st_uid != 0
        || stat.st_mode & 0o022 != 0
        || stat.st_mode & 0o111 == 0
        || stat.st_size <= 0
    {
        return Err(error(LinuxRepositoryErrorKind::InvalidGitArtifact));
    }
    let sha256 = hash_descriptor(&descriptor, stat.st_size)?;
    Ok(LinuxGitArtifact {
        descriptor,
        launch_path: path.to_path_buf(),
        sha256,
    })
}

fn revalidate_git_artifact(artifact: &LinuxGitArtifact) -> Result<(), LinuxRepositoryError> {
    let held = fstat(&artifact.descriptor)
        .map_err(|_| error(LinuxRepositoryErrorKind::InvalidGitArtifact))?;
    if held.st_size <= 0 || hash_descriptor(&artifact.descriptor, held.st_size)? != artifact.sha256
    {
        return Err(error(LinuxRepositoryErrorKind::InvalidGitArtifact));
    }
    let current = verify_git_artifact(&artifact.launch_path)?;
    if current.sha256 != artifact.sha256 {
        return Err(error(LinuxRepositoryErrorKind::InvalidGitArtifact));
    }
    Ok(())
}

fn verify_root_owned_path(path: &Path) -> Result<(), LinuxRepositoryError> {
    let mut current = PathBuf::from("/");
    for component in path.components().skip(1) {
        let Component::Normal(component) = component else {
            return Err(error(LinuxRepositoryErrorKind::InvalidGitArtifact));
        };
        current.push(component);
        let metadata = fs::symlink_metadata(&current)
            .map_err(|_| error(LinuxRepositoryErrorKind::InvalidGitArtifact))?;
        if metadata.uid() != 0 || metadata.mode() & 0o022 != 0 || metadata.file_type().is_symlink()
        {
            return Err(error(LinuxRepositoryErrorKind::InvalidGitArtifact));
        }
    }
    Ok(())
}

fn verify_owned_directory(path: &Path) -> Result<PathBuf, LinuxRepositoryError> {
    if !path.is_absolute() {
        return Err(error(LinuxRepositoryErrorKind::InvalidPath));
    }
    let canonical = path
        .canonicalize()
        .map_err(|_| error(LinuxRepositoryErrorKind::InvalidPath))?;
    let metadata = fs::symlink_metadata(&canonical)
        .map_err(|_| error(LinuxRepositoryErrorKind::InvalidPath))?;
    if !metadata.is_dir()
        || metadata.file_type().is_symlink()
        || metadata.uid() != getuid().as_raw()
        || metadata.mode() & 0o022 != 0
    {
        return Err(error(LinuxRepositoryErrorKind::InvalidPath));
    }
    Ok(canonical)
}

fn path_identity(path: &Path) -> Result<String, LinuxRepositoryError> {
    let metadata = fs::metadata(path).map_err(|_| error(LinuxRepositoryErrorKind::InvalidPath))?;
    let mut material = Vec::new();
    material.extend_from_slice(path.as_os_str().as_bytes());
    material.extend_from_slice(&metadata.dev().to_be_bytes());
    material.extend_from_slice(&metadata.ino().to_be_bytes());
    Ok(hash(&material))
}

fn hash_descriptor(descriptor: &OwnedFd, size: i64) -> Result<String, LinuxRepositoryError> {
    let size =
        usize::try_from(size).map_err(|_| error(LinuxRepositoryErrorKind::InvalidGitArtifact))?;
    let mut digest = Sha256::new();
    let mut buffer = vec![0_u8; HASH_BUFFER_BYTES];
    let mut offset = 0_usize;
    while offset < size {
        let count = pread(descriptor, &mut buffer, offset as u64)
            .map_err(|_| error(LinuxRepositoryErrorKind::InvalidGitArtifact))?;
        if count == 0 {
            return Err(error(LinuxRepositoryErrorKind::InvalidGitArtifact));
        }
        digest.update(&buffer[..count]);
        offset = offset.saturating_add(count);
    }
    Ok(hex(&digest.finalize()))
}

fn text_required(bytes: Vec<u8>) -> Result<String, LinuxRepositoryError> {
    let value = String::from_utf8(bytes)
        .map_err(|_| error(LinuxRepositoryErrorKind::ObservationFailed))?
        .trim()
        .to_owned();
    if value.len() > 1_024 || value.chars().any(|character| character.is_control()) {
        return Err(error(LinuxRepositoryErrorKind::ObservationFailed));
    }
    Ok(value)
}

fn status_counts(bytes: &[u8]) -> Result<(u32, u32), LinuxRepositoryError> {
    let mut untracked = 0_u32;
    let mut ignored = 0_u32;
    for record in bytes
        .split(|byte| *byte == 0)
        .filter(|record| !record.is_empty())
    {
        match record.first() {
            Some(b'?') => untracked = untracked.saturating_add(1),
            Some(b'!') => ignored = ignored.saturating_add(1),
            Some(b'1' | b'2' | b'u' | b'#') => {}
            _ => return Err(error(LinuxRepositoryErrorKind::ObservationFailed)),
        }
    }
    Ok((untracked, ignored))
}

fn read_optional_bounded(path: &Path) -> Result<Option<Vec<u8>>, LinuxRepositoryError> {
    if !path.exists() {
        return Ok(None);
    }
    let metadata = fs::symlink_metadata(path)
        .map_err(|_| error(LinuxRepositoryErrorKind::ObservationFailed))?;
    if !metadata.is_file()
        || metadata.file_type().is_symlink()
        || metadata.len() > MAX_HASHED_FILE_BYTES
    {
        return Err(error(LinuxRepositoryErrorKind::ObservationFailed));
    }
    fs::read(path)
        .map(Some)
        .map_err(|_| error(LinuxRepositoryErrorKind::ObservationFailed))
}

fn hash_optional_file(path: &Path) -> Result<String, LinuxRepositoryError> {
    Ok(hash(
        read_optional_bounded(path)?.as_deref().unwrap_or(b"absent"),
    ))
}

fn hash_metadata_tree(
    path: &Path,
    exclude_agentmage: bool,
) -> Result<Vec<u8>, LinuxRepositoryError> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let mut pending = vec![path.to_path_buf()];
    let mut records = Vec::new();
    while let Some(current) = pending.pop() {
        let entries = fs::read_dir(&current)
            .map_err(|_| error(LinuxRepositoryErrorKind::ObservationFailed))?;
        for entry in entries {
            let entry = entry.map_err(|_| error(LinuxRepositoryErrorKind::ObservationFailed))?;
            let child = entry.path();
            let relative = child
                .strip_prefix(path)
                .map_err(|_| error(LinuxRepositoryErrorKind::ObservationFailed))?;
            if exclude_agentmage && relative.to_string_lossy().contains("agentmage") {
                continue;
            }
            let metadata = fs::symlink_metadata(&child)
                .map_err(|_| error(LinuxRepositoryErrorKind::ObservationFailed))?;
            let mut record = Vec::new();
            record.extend_from_slice(relative.as_os_str().as_bytes());
            record.extend_from_slice(&metadata.mode().to_be_bytes());
            record.extend_from_slice(&metadata.len().to_be_bytes());
            record.extend_from_slice(&metadata.mtime().to_be_bytes());
            records.push(record);
            if metadata.is_dir() && !metadata.file_type().is_symlink() {
                pending.push(child);
            }
            if records.len() > MAX_METADATA_ENTRIES {
                return Err(error(LinuxRepositoryErrorKind::ObservationFailed));
            }
        }
    }
    records.sort();
    Ok(records.concat())
}

fn hash_reflog_tree(path: &Path, exclude_agentmage: bool) -> Result<Vec<u8>, LinuxRepositoryError> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let mut pending = vec![path.to_path_buf()];
    let mut records = Vec::new();
    while let Some(current) = pending.pop() {
        for entry in fs::read_dir(&current)
            .map_err(|_| error(LinuxRepositoryErrorKind::ObservationFailed))?
        {
            let entry = entry.map_err(|_| error(LinuxRepositoryErrorKind::ObservationFailed))?;
            let child = entry.path();
            let relative = child
                .strip_prefix(path)
                .map_err(|_| error(LinuxRepositoryErrorKind::ObservationFailed))?;
            if exclude_agentmage && relative.to_string_lossy().contains("agentmage") {
                continue;
            }
            let metadata = fs::symlink_metadata(&child)
                .map_err(|_| error(LinuxRepositoryErrorKind::ObservationFailed))?;
            if metadata.is_dir() && !metadata.file_type().is_symlink() {
                pending.push(child);
            } else {
                let bytes = read_optional_bounded(&child)?
                    .ok_or_else(|| error(LinuxRepositoryErrorKind::ObservationFailed))?;
                let mut record = Vec::new();
                record.extend_from_slice(relative.as_os_str().as_bytes());
                record.extend_from_slice(hash(&bytes).as_bytes());
                records.push(record);
            }
            if records.len() > MAX_METADATA_ENTRIES {
                return Err(error(LinuxRepositoryErrorKind::ObservationFailed));
            }
        }
    }
    records.sort();
    Ok(records.concat())
}

fn operation_metadata(git_directory: &Path) -> Result<Vec<u8>, LinuxRepositoryError> {
    const NAMES: [&str; 14] = [
        "MERGE_HEAD",
        "CHERRY_PICK_HEAD",
        "REVERT_HEAD",
        "BISECT_START",
        "rebase-apply",
        "rebase-merge",
        "sequencer",
        "index.lock",
        "config.lock",
        "packed-refs.lock",
        "shallow.lock",
        "gc.pid",
        "gc.log",
        "maintenance.lock",
    ];
    let mut records = Vec::new();
    for name in NAMES {
        let path = git_directory.join(name);
        if path.exists() {
            let metadata = fs::symlink_metadata(&path)
                .map_err(|_| error(LinuxRepositoryErrorKind::ObservationFailed))?;
            records.extend_from_slice(name.as_bytes());
            records.extend_from_slice(&metadata.mode().to_be_bytes());
            records.extend_from_slice(&metadata.len().to_be_bytes());
        }
    }
    Ok(records)
}

fn directory_has_active_hooks(path: &Path) -> Result<bool, LinuxRepositoryError> {
    if !path.exists() {
        return Ok(false);
    }
    for entry in
        fs::read_dir(path).map_err(|_| error(LinuxRepositoryErrorKind::ObservationFailed))?
    {
        let entry = entry.map_err(|_| error(LinuxRepositoryErrorKind::ObservationFailed))?;
        let metadata = fs::symlink_metadata(entry.path())
            .map_err(|_| error(LinuxRepositoryErrorKind::ObservationFailed))?;
        if metadata.file_type().is_symlink()
            || (metadata.is_file()
                && metadata.mode() & 0o111 != 0
                && !entry.file_name().as_bytes().ends_with(b".sample"))
        {
            return Ok(true);
        }
    }
    Ok(false)
}

fn hazardous_config(lower: &[u8]) -> bool {
    lower.split(|byte| *byte == 0).any(|record| {
        let (name, value) = record
            .iter()
            .position(|byte| *byte == b'\n')
            .map_or((record, &[][..]), |index| {
                (&record[..index], &record[index + 1..])
            });
        name.starts_with(b"alias.")
            || name.starts_with(b"url.")
            || name.starts_with(b"credential.")
            || name.starts_with(b"filter.")
            || matches!(
                name,
                b"core.hookspath"
                    | b"core.sshcommand"
                    | b"core.askpass"
                    | b"core.fsmonitor"
                    | b"core.pager"
                    | b"core.editor"
                    | b"diff.external"
                    | b"gpg.program"
                    | b"gpg.ssh.program"
            )
            || (name.starts_with(b"diff.")
                && (name.ends_with(b".command") || name.ends_with(b".textconv")))
            || (name.starts_with(b"merge.") && name.ends_with(b".driver"))
            || (name.starts_with(b"remote.")
                && (name.ends_with(b".uploadpack") || name.ends_with(b".receivepack")))
            || (name.starts_with(b"protocol.") && name.ends_with(b".allow") && value == b"always")
            || (name == b"safe.directory" && value == b"*")
    })
}

fn hazardous_attributes(bytes: &[u8]) -> bool {
    let lower = bytes.iter().map(u8::to_ascii_lowercase).collect::<Vec<_>>();
    contains_any(
        &lower,
        &[
            b"filter=", b"diff=", b"merge=", b"!filter", b"!diff", b"!merge",
        ],
    )
}

fn contains_any(haystack: &[u8], needles: &[&[u8]]) -> bool {
    needles.iter().any(|needle| {
        haystack
            .windows(needle.len())
            .any(|window| window == *needle)
    })
}

fn hash(bytes: &[u8]) -> String {
    hex(&Sha256::digest(bytes))
}

fn hex(bytes: &[u8]) -> String {
    let mut encoded = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        write!(&mut encoded, "{byte:02x}").expect("writing to a String cannot fail");
    }
    encoded
}

const fn error(kind: LinuxRepositoryErrorKind) -> LinuxRepositoryError {
    LinuxRepositoryError { kind }
}

#[cfg(test)]
mod tests {
    use super::*;
    use agentmage_kernel_contracts::{BoundaryKind, CorrelationId, TaskId};
    use agentmage_kernel_engine::propagation::CancellationToken;
    use agentmage_kernel_engine::repository_safety::{
        OwnedWorktreeRecord, WorktreeDisposition, plan_branch_fast_forward, plan_worktree_create,
        plan_worktree_remove, reconcile_operation,
    };
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_ID: AtomicU64 = AtomicU64::new(1);

    struct Fixture {
        root: PathBuf,
        repository: PathBuf,
        git_directory: PathBuf,
        owned: PathBuf,
    }

    impl Fixture {
        fn new() -> Self {
            let root = std::env::temp_dir().join(format!(
                "agentmage-git-{}-{}",
                std::process::id(),
                NEXT_ID.fetch_add(1, Ordering::Relaxed)
            ));
            let repository = root.join("repository");
            let owned = root.join("owned");
            fs::create_dir_all(&repository).expect("repository directory");
            fs::create_dir_all(&owned).expect("owned directory");
            run_fixture(&repository, &["init", "--initial-branch=main"]);
            fs::write(repository.join("README.md"), b"fixture\n").expect("fixture write");
            run_fixture(&repository, &["add", "README.md"]);
            run_fixture(
                &repository,
                &[
                    "-c",
                    "user.name=AgentMage Fixture",
                    "-c",
                    "user.email=fixture@example.test",
                    "commit",
                    "-m",
                    "fixture",
                ],
            );
            let git_directory = repository.join(".git");
            Self {
                root,
                repository,
                git_directory,
                owned,
            }
        }

        fn scope(&self) -> LinuxRepositoryScope {
            LinuxRepositoryScope::verify(&self.repository, &self.git_directory, &self.owned)
                .expect("scope")
        }

        fn collector(&self) -> LinuxRepositoryCollector {
            LinuxRepositoryCollector::new(
                LinuxGitArtifact::verify("/usr/bin/git").expect("git artifact"),
            )
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    fn run_fixture(repository: &Path, arguments: &[&str]) -> Vec<u8> {
        let output = Command::new("/usr/bin/git")
            .env_clear()
            .env("HOME", "/nonexistent")
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .args(arguments)
            .current_dir(repository)
            .output()
            .expect("fixture Git runs");
        assert!(output.status.success(), "fixture Git failed");
        output.stdout
    }

    fn object(repository: &Path, revision: &str) -> String {
        String::from_utf8(run_fixture(repository, &["rev-parse", revision]))
            .expect("object UTF-8")
            .trim()
            .to_owned()
    }

    fn hash_label(label: &str) -> String {
        hash(label.as_bytes())
    }

    fn cancellation(label: &str) -> CancellationToken {
        CancellationToken::root(
            BoundaryKind::Kernel,
            TaskId::from_raw(format!("task-{label}")),
            CorrelationId::from_raw(format!("correlation-{label}")),
        )
    }

    fn eligible_record(path_sha256: String, source_object: String) -> OwnedWorktreeRecord {
        OwnedWorktreeRecord::seal(OwnedWorktreeRecord {
            schema_version: 0,
            worktree_id: "worktree-1".to_owned(),
            task_id: "task-1".to_owned(),
            source_object,
            branch_ref: "refs/heads/agentmage/tasks/task-1".to_owned(),
            worktree_path_sha256: path_sha256,
            owner_sha256: hash_label("owner"),
            grant_ids: vec!["grant-1".to_owned()],
            file_ownership_sha256: hash_label("files"),
            live_process_count: 0,
            resource_budget_sha256: hash_label("budget"),
            retain_until_epoch_ms: 1,
            clean: true,
            recovery_retained: true,
            disposition: WorktreeDisposition::CleanupEligible,
            record_sha256: String::new(),
        })
        .expect("record")
    }

    #[test]
    fn manifest_is_minimized_stable_and_detects_untracked_and_hazards() {
        let fixture = Fixture::new();
        let collector = fixture.collector();
        let scope = fixture.scope();
        let clean = collector.collect(&scope).expect("clean manifest");
        assert_eq!(clean.untracked_count, 0);
        assert!(!clean.hazardous_configuration);
        assert_eq!(collector.collect(&scope).expect("stable manifest"), clean);

        fs::write(fixture.repository.join("untracked.txt"), b"private fixture")
            .expect("untracked fixture");
        let untracked = collector.collect(&scope).expect("untracked manifest");
        assert_eq!(untracked.untracked_count, 1);
        assert_ne!(
            untracked.path_dispositions_sha256,
            clean.path_dispositions_sha256
        );
        fs::remove_file(fixture.repository.join("untracked.txt")).expect("fixture removal");
        run_fixture(
            &fixture.repository,
            &["config", "alias.agentmage", "!touch should-never-exist"],
        );
        assert!(
            collector
                .collect(&scope)
                .expect("hazard manifest")
                .hazardous_configuration
        );
    }

    #[test]
    fn exact_worktree_lifecycle_preserves_active_checkout() {
        let fixture = Fixture::new();
        let scope = fixture.scope();
        let collector = fixture.collector();
        let before = collector.collect(&scope).expect("before");
        let active_head = object(&fixture.repository, "HEAD");
        let active_branch = run_fixture(&fixture.repository, &["symbolic-ref", "HEAD"]);
        let active_index = fs::read(fixture.git_directory.join("index")).expect("index bytes");
        let worktree_path = fixture.owned.join("worktree");
        let worktree_path_sha256 = hash(worktree_path.as_os_str().as_bytes());
        let plan = plan_worktree_create(
            "transaction-create",
            "task-1",
            &active_head,
            scope.repository_path_sha256(),
            &worktree_path_sha256,
            &before,
        )
        .expect("create plan");
        let cancellation = cancellation("worktree");
        let mut executor = LinuxRepositoryExecutor::new(collector, scope.clone());
        let created = executor.run(&plan, &before, &cancellation);
        assert_eq!(created.outcome, OperationOutcome::Succeeded);
        let receipt = reconcile_operation(&plan, created, "authority-create", "attempt-create")
            .expect("create reconciles");
        assert_eq!(receipt.outcome, OperationOutcome::Succeeded);
        assert_eq!(object(&fixture.repository, "HEAD"), active_head);
        assert_eq!(
            run_fixture(&fixture.repository, &["symbolic-ref", "HEAD"]),
            active_branch
        );
        assert_eq!(
            fs::read(fixture.git_directory.join("index")).expect("index"),
            active_index
        );
        assert!(worktree_path.join("README.md").is_file());

        let before_remove = executor.collector.collect(&scope).expect("before remove");
        let record = eligible_record(
            linux_repository_path_sha256(&worktree_path).expect("worktree identity"),
            active_head,
        );
        let removal = plan_worktree_remove(
            "transaction-remove",
            &record,
            scope.repository_path_sha256(),
            &before_remove,
        )
        .expect("remove plan");
        let removed = executor.run(&removal, &before_remove, &cancellation);
        assert_eq!(removed.outcome, OperationOutcome::Succeeded);
        reconcile_operation(&removal, removed, "authority-remove", "attempt-remove")
            .expect("removal reconciles");
        assert!(!worktree_path.exists());
        assert_eq!(
            object(&fixture.repository, "HEAD"),
            object(&fixture.repository, "main")
        );
    }

    #[test]
    fn compare_and_swap_updates_only_one_unchecked_branch_and_stale_old_fails() {
        let fixture = Fixture::new();
        let first = object(&fixture.repository, "HEAD");
        run_fixture(
            &fixture.repository,
            &["branch", "agentmage/tasks/review", &first],
        );
        fs::write(fixture.repository.join("README.md"), b"fixture two\n").expect("second write");
        run_fixture(&fixture.repository, &["add", "README.md"]);
        run_fixture(
            &fixture.repository,
            &[
                "-c",
                "user.name=AgentMage Fixture",
                "-c",
                "user.email=fixture@example.test",
                "commit",
                "-m",
                "second",
            ],
        );
        let second = object(&fixture.repository, "HEAD");
        let scope = fixture.scope();
        let collector = fixture.collector();
        let before = collector.collect(&scope).expect("before");
        let plan = plan_branch_fast_forward(
            "transaction-ff",
            "refs/heads/agentmage/tasks/review",
            &first,
            &second,
            true,
            scope.repository_path_sha256(),
            &before,
        )
        .expect("ff plan");
        let cancellation = cancellation("ff");
        let mut executor = LinuxRepositoryExecutor::new(collector, scope.clone());
        let result = executor.run(&plan, &before, &cancellation);
        assert_eq!(result.outcome, OperationOutcome::Succeeded);
        reconcile_operation(&plan, result, "authority-ff", "attempt-ff").expect("ff reconciles");
        assert_eq!(
            object(&fixture.repository, "agentmage/tasks/review"),
            second
        );

        let stale_before = executor.collector.collect(&scope).expect("stale before");
        let stale_plan = plan_branch_fast_forward(
            "transaction-stale",
            "refs/heads/agentmage/tasks/review",
            &first,
            &object(&fixture.repository, "main"),
            true,
            scope.repository_path_sha256(),
            &stale_before,
        )
        .expect("stale plan");
        let stale = executor.run(&stale_plan, &stale_before, &cancellation);
        assert_eq!(stale.outcome, OperationOutcome::Failed);
        assert_eq!(stale.before, stale.after);
        assert_eq!(
            object(&fixture.repository, "agentmage/tasks/review"),
            second
        );
    }

    #[test]
    fn network_clone_fetch_and_unmapped_tokens_never_launch_locally() {
        let fixture = Fixture::new();
        let scope = fixture.scope();
        let collector = fixture.collector();
        let before = collector.collect(&scope).expect("before");
        let plan = agentmage_kernel_engine::repository_safety::plan_fetch(
            "transaction-fetch",
            agentmage_kernel_engine::repository_safety::GitRemoteIdentity::parse(
                "https://github.com/AgentMage/fixture.git",
                "github.com",
            )
            .expect("remote"),
            "refs/heads/main",
            scope.repository_path_sha256(),
            &before,
        )
        .expect("fetch plan");
        let cancellation = cancellation("fetch");
        let mut executor = LinuxRepositoryExecutor::new(collector, scope);
        let denied = executor.run(&plan, &before, &cancellation);
        assert_eq!(denied.outcome, OperationOutcome::Denied);
        assert_eq!(denied.before, denied.after);
    }
}
