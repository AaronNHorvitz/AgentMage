//! Pinned Git observation and local owned-worktree execution for Linux.

use std::fmt;
use std::fmt::Write as _;
use std::fs;
use std::io::Read;
use std::os::fd::AsRawFd as _;
use std::os::unix::ffi::OsStrExt as _;
use std::os::unix::fs::MetadataExt as _;
use std::path::{Component, Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Stdio};
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
use rustix::fs::{
    FileType, MemfdFlags, Mode, OFlags, SealFlags, SeekFrom, fchmod, fcntl_add_seals, fstat,
    memfd_create, open, seek,
};
use rustix::io::{pread, write as rustix_write};
use rustix::process::getuid;
use rustix::rand::{GetRandomFlags, getrandom};
use sha2::{Digest, Sha256};

use crate::sandbox::compile_seccomp_policy;

const HASH_BUFFER_BYTES: usize = 64 * 1024;
const MAX_OBSERVATION_BYTES: usize = 4 * 1024 * 1024;
const MAX_HASHED_FILE_BYTES: u64 = 64 * 1024 * 1024;
const MAX_METADATA_ENTRIES: usize = 16_384;
const PROCESS_TIMEOUT: Duration = Duration::from_secs(30);
const INSPECTION_PROCESS_TIMEOUT: Duration = Duration::from_secs(15);
const POLL_INTERVAL: Duration = Duration::from_millis(10);
const INSPECTION_UNIT_GRACE: Duration = Duration::from_secs(3);
const GUEST_GIT_EXECUTABLE: &str = "/app/git";

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

    pub(crate) const fn from_kind(kind: LinuxRepositoryErrorKind) -> Self {
        Self { kind }
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

/// Descriptor-held launchers for one isolated read-only Git worker.
pub struct LinuxRepositoryInspectionManifest {
    systemd_run: LinuxGitArtifact,
    systemctl: LinuxGitArtifact,
    bubblewrap: LinuxGitArtifact,
    git: LinuxGitArtifact,
}

impl LinuxRepositoryInspectionManifest {
    /// Verifies and holds the exact systemd, Bubblewrap, and Git executables.
    pub fn verify(
        systemd_run: impl AsRef<Path>,
        systemctl: impl AsRef<Path>,
        bubblewrap: impl AsRef<Path>,
        git: impl AsRef<Path>,
    ) -> Result<Self, LinuxRepositoryError> {
        Ok(Self {
            systemd_run: LinuxGitArtifact::verify(systemd_run)?,
            systemctl: LinuxGitArtifact::verify(systemctl)?,
            bubblewrap: LinuxGitArtifact::verify(bubblewrap)?,
            git: LinuxGitArtifact::verify(git)?,
        })
    }

    /// Returns the exact verified Git executable digest.
    #[must_use]
    pub fn git_sha256(&self) -> &str {
        self.git.sha256()
    }
}

impl fmt::Debug for LinuxRepositoryInspectionManifest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("LinuxRepositoryInspectionManifest")
            .field("systemd_run", &self.systemd_run)
            .field("systemctl", &self.systemctl)
            .field("bubblewrap", &self.bubblewrap)
            .field("git", &self.git)
            .finish()
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

        let index_paths = self.observe(scope, &["ls-files", "--stage", "-z"])?;
        let worktree_state = hash_worktree_tree(&scope.checkout_root)?;
        let untracked_paths =
            self.observe(scope, &["ls-files", "--others", "--exclude-standard", "-z"])?;
        let ignored_paths = self.observe(
            scope,
            &[
                "ls-files",
                "--others",
                "--ignored",
                "--exclude-standard",
                "-z",
            ],
        )?;
        let untracked_count = nul_record_count(&untracked_paths)?;
        let ignored_count = nul_record_count(&ignored_paths)?;
        let path_dispositions = framed_observations(&[
            ("index", &index_paths),
            ("worktree", &worktree_state),
            ("untracked", &untracked_paths),
            ("ignored", &ignored_paths),
        ])?;
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
            path_dispositions_sha256: hash(&path_dispositions),
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

    pub(crate) fn observe_inventory(
        &self,
        scope: &LinuxRepositoryScope,
        arguments: &[&str],
        maximum_bytes: usize,
        cancellation: Option<&CancellationToken>,
    ) -> Result<Vec<u8>, LinuxRepositoryError> {
        self.observe_inventory_optional(scope, arguments, maximum_bytes, cancellation)?
            .ok_or_else(|| error(LinuxRepositoryErrorKind::ObservationFailed))
    }

    pub(crate) fn observe_inventory_optional(
        &self,
        scope: &LinuxRepositoryScope,
        arguments: &[&str],
        maximum_bytes: usize,
        cancellation: Option<&CancellationToken>,
    ) -> Result<Option<Vec<u8>>, LinuxRepositoryError> {
        if maximum_bytes == 0 || maximum_bytes > 32 * 1024 * 1024 {
            return Err(error(LinuxRepositoryErrorKind::ObservationFailed));
        }
        revalidate_git_artifact(&self.git)?;
        if cancellation.is_some_and(CancellationToken::is_cancelled) {
            return Err(error(LinuxRepositoryErrorKind::ObservationFailed));
        }
        let mut command = Command::new(&self.git.launch_path);
        command
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
            .stdin(Stdio::null());
        let (status, stdout) =
            run_inventory_process(&mut command, maximum_bytes, cancellation, PROCESS_TIMEOUT)?;
        if status.success() {
            Ok(Some(stdout))
        } else if matches!(status.code(), Some(1 | 128)) {
            Ok(None)
        } else {
            Err(error(LinuxRepositoryErrorKind::ObservationFailed))
        }
    }
}

fn run_inventory_process(
    command: &mut Command,
    maximum_bytes: usize,
    cancellation: Option<&CancellationToken>,
    timeout: Duration,
) -> Result<(ExitStatus, Vec<u8>), LinuxRepositoryError> {
    command.stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = command
        .spawn()
        .map_err(|_| error(LinuxRepositoryErrorKind::ObservationFailed))?;
    let Some(stdout) = child.stdout.take() else {
        let _ = child.kill();
        let _ = child.wait();
        return Err(error(LinuxRepositoryErrorKind::ObservationFailed));
    };
    let Some(stderr) = child.stderr.take() else {
        let _ = child.kill();
        let _ = child.wait();
        return Err(error(LinuxRepositoryErrorKind::ObservationFailed));
    };
    let stdout_reader = thread::spawn(move || read_inventory_output(stdout, maximum_bytes));
    let stderr_reader = thread::spawn(move || read_inventory_output(stderr, MAX_OBSERVATION_BYTES));
    let deadline = Instant::now() + timeout;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break Ok(status),
            Ok(None)
                if cancellation.is_some_and(CancellationToken::is_cancelled)
                    || Instant::now() >= deadline =>
            {
                let _ = child.kill();
                let _ = child.wait();
                break Err(error(LinuxRepositoryErrorKind::ObservationFailed));
            }
            Ok(None) => thread::sleep(POLL_INTERVAL),
            Err(_) => {
                let _ = child.kill();
                let _ = child.wait();
                break Err(error(LinuxRepositoryErrorKind::ObservationFailed));
            }
        }
    };
    let stdout = stdout_reader
        .join()
        .map_err(|_| error(LinuxRepositoryErrorKind::ObservationFailed))??;
    let stderr = stderr_reader
        .join()
        .map_err(|_| error(LinuxRepositoryErrorKind::ObservationFailed))??;
    let status = status?;
    if stdout.len() > maximum_bytes || stderr.len() > MAX_OBSERVATION_BYTES {
        return Err(error(LinuxRepositoryErrorKind::ObservationFailed));
    }
    Ok((status, stdout))
}

fn read_inventory_output(
    input: impl Read,
    maximum_bytes: usize,
) -> Result<Vec<u8>, LinuxRepositoryError> {
    let limit = u64::try_from(maximum_bytes)
        .ok()
        .and_then(|value| value.checked_add(1))
        .ok_or_else(|| error(LinuxRepositoryErrorKind::ObservationFailed))?;
    let mut bytes = Vec::new();
    input
        .take(limit)
        .read_to_end(&mut bytes)
        .map_err(|_| error(LinuxRepositoryErrorKind::ObservationFailed))?;
    if bytes.len() > maximum_bytes {
        return Err(error(LinuxRepositoryErrorKind::ObservationFailed));
    }
    Ok(bytes)
}

/// Pinned offline executor for one kernel-validated read-only Git inspection.
#[derive(Debug)]
pub struct LinuxBoundedRepositoryInspectionExecutor {
    manifest: LinuxRepositoryInspectionManifest,
    seccomp_descriptor: OwnedFd,
}

impl LinuxBoundedRepositoryInspectionExecutor {
    /// Creates an inert executor around a verified offline worker manifest.
    pub fn new(manifest: LinuxRepositoryInspectionManifest) -> Result<Self, LinuxRepositoryError> {
        let seccomp = compile_seccomp_policy()
            .map_err(|_| error(LinuxRepositoryErrorKind::InvalidGitArtifact))?;
        Ok(Self {
            manifest,
            seccomp_descriptor: sealed_inspection_seccomp(&seccomp)?,
        })
    }

    fn run(
        &self,
        prepared: &agentmage_kernel_engine::repository_inspection::PreparedRepositoryInspection,
        working_directory: &crate::LinuxAuthorizedWorkspace,
        cancellation: &CancellationToken,
    ) -> RepositoryInspectionPlatformResult {
        if revalidate_git_artifact(&self.manifest.systemd_run).is_err()
            || revalidate_git_artifact(&self.manifest.systemctl).is_err()
            || revalidate_git_artifact(&self.manifest.bubblewrap).is_err()
            || revalidate_git_artifact(&self.manifest.git).is_err()
            || working_directory.revalidate().is_err()
            || !prepared.stdin().is_empty()
        {
            return failed_inspection("linux.git-inspection.preflight.failed");
        }
        let Ok(root) = working_directory.reopen_root_directory() else {
            return failed_inspection("linux.git-inspection.root.failed");
        };
        let Ok(unit_stem) = random_inspection_unit() else {
            return failed_inspection("linux.git-inspection.unit.failed");
        };
        let unit = format!("{unit_stem}.service");
        let parent_pid = std::process::id();
        let git_descriptor = format!(
            "/proc/{parent_pid}/fd/{}",
            self.manifest.git.descriptor.as_raw_fd()
        );
        let seccomp_descriptor = format!(
            "/proc/{parent_pid}/fd/{}",
            self.seccomp_descriptor.as_raw_fd()
        );
        let worktree_descriptor = format!("/proc/{parent_pid}/fd/{}", root.as_raw_fd());
        let runtime_directory = format!("/run/user/{}", getuid().as_raw());
        let session_bus = format!("unix:path={runtime_directory}/bus");
        let mut command = Command::new(&self.manifest.systemd_run.launch_path);
        command
            .env_clear()
            .env("XDG_RUNTIME_DIR", &runtime_directory)
            .env("DBUS_SESSION_BUS_ADDRESS", &session_bus)
            .args([
                "--user",
                "--wait",
                "--quiet",
                "--pipe",
                "--collect",
                "--expand-environment=no",
            ])
            .arg(format!("--unit={unit}"))
            .arg("--property=NoNewPrivileges=yes")
            .arg("--property=RestrictSUIDSGID=yes")
            .arg("--property=LockPersonality=yes")
            .arg("--property=RestrictAddressFamilies=AF_UNIX AF_NETLINK")
            .arg("--property=MemorySwapMax=0")
            .arg("--property=MemoryMax=268435456")
            .arg("--property=TasksMax=16")
            .arg("--property=CPUQuota=100%")
            .arg("--property=RuntimeMaxSec=16s")
            .arg("--property=KillMode=control-group")
            .arg("--property=SendSIGKILL=yes")
            .arg("--property=TimeoutStopSec=2s")
            .arg(format!(
                "--property=OpenFile={git_descriptor}:git:read-only"
            ))
            .arg(format!(
                "--property=OpenFile={seccomp_descriptor}:seccomp:read-only"
            ))
            .arg(format!(
                "--property=OpenFile={worktree_descriptor}:worktree:read-only"
            ))
            .arg(&self.manifest.bubblewrap.launch_path)
            .args([
                "--unshare-all",
                "--unshare-user",
                "--disable-userns",
                "--new-session",
                "--die-with-parent",
                "--clearenv",
            ]);
        for (name, value) in prepared.environment() {
            command.arg("--setenv").arg(name).arg(value);
        }
        command.args([
            "--cap-drop",
            "ALL",
            "--proc",
            "/proc",
            "--dev",
            "/dev",
            "--tmpfs",
            "/tmp",
            "--dir",
            "/app",
            "--dir",
            "/work",
            "--dir",
            "/usr",
            "--dir",
            "/etc",
        ]);
        add_inspection_runtime_mounts(&mut command);
        command
            .args(["--ro-bind-fd", "3", GUEST_GIT_EXECUTABLE])
            .args(["--ro-bind-fd", "5", "/work"])
            .args([
                "--chdir",
                "/work",
                "--seccomp",
                "4",
                "--",
                GUEST_GIT_EXECUTABLE,
            ])
            .args(prepared.arguments())
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let started = Instant::now();
        let Ok(mut child) = command.spawn() else {
            return failed_inspection("linux.git-inspection.launch.failed");
        };
        let Some(stdout) = child.stdout.take() else {
            let cleanup = terminate_inspection_unit(
                &self.manifest,
                &unit,
                &runtime_directory,
                &session_bus,
                &mut child,
            );
            return failed_inspection_with_cleanup(
                "linux.git-inspection.stdout.failed",
                cleanup,
                started.elapsed(),
            );
        };
        let Some(stderr) = child.stderr.take() else {
            let cleanup = terminate_inspection_unit(
                &self.manifest,
                &unit,
                &runtime_directory,
                &session_bus,
                &mut child,
            );
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
                    let cleanup = cleanup_inspection_unit(
                        &self.manifest,
                        &unit,
                        &runtime_directory,
                        &session_bus,
                    );
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
                    let cleanup = terminate_inspection_unit(
                        &self.manifest,
                        &unit,
                        &runtime_directory,
                        &session_bus,
                        &mut child,
                    );
                    break (
                        RepositoryInspectionTermination::Cancelled,
                        None,
                        cleanup,
                        "linux.git-inspection.cancelled",
                    );
                }
                Ok(None) if Instant::now() >= deadline => {
                    let cleanup = terminate_inspection_unit(
                        &self.manifest,
                        &unit,
                        &runtime_directory,
                        &session_bus,
                        &mut child,
                    );
                    break (
                        RepositoryInspectionTermination::TimedOut,
                        None,
                        cleanup,
                        "linux.git-inspection.timed-out",
                    );
                }
                Ok(None) => thread::sleep(POLL_INTERVAL),
                Err(_) => {
                    let cleanup = terminate_inspection_unit(
                        &self.manifest,
                        &unit,
                        &runtime_directory,
                        &session_bus,
                        &mut child,
                    );
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
        self.run(permit.prepared(), working_directory, cancellation)
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

fn sealed_inspection_seccomp(bytes: &[u8]) -> Result<OwnedFd, LinuxRepositoryError> {
    if bytes.is_empty() {
        return Err(error(LinuxRepositoryErrorKind::InvalidGitArtifact));
    }
    let descriptor = memfd_create(
        "agentmage-git-seccomp",
        MemfdFlags::CLOEXEC | MemfdFlags::ALLOW_SEALING,
    )
    .map_err(|_| error(LinuxRepositoryErrorKind::InvalidGitArtifact))?;
    let mut remaining = bytes;
    while !remaining.is_empty() {
        let count = rustix_write(&descriptor, remaining)
            .map_err(|_| error(LinuxRepositoryErrorKind::InvalidGitArtifact))?;
        if count == 0 {
            return Err(error(LinuxRepositoryErrorKind::InvalidGitArtifact));
        }
        remaining = &remaining[count..];
    }
    seek(&descriptor, SeekFrom::Start(0))
        .map_err(|_| error(LinuxRepositoryErrorKind::InvalidGitArtifact))?;
    fcntl_add_seals(
        &descriptor,
        SealFlags::SEAL | SealFlags::SHRINK | SealFlags::GROW | SealFlags::WRITE,
    )
    .map_err(|_| error(LinuxRepositoryErrorKind::InvalidGitArtifact))?;
    Ok(descriptor)
}

fn add_inspection_runtime_mounts(command: &mut Command) {
    for path in ["/usr/lib", "/usr/lib64", "/usr/libexec", "/lib", "/lib64"] {
        if Path::new(path).exists() {
            command.args(["--ro-bind", path, path]);
        }
    }
    if Path::new("/etc/ld.so.cache").is_file() {
        command.args(["--ro-bind", "/etc/ld.so.cache", "/etc/ld.so.cache"]);
    }
}

fn random_inspection_unit() -> Result<String, LinuxRepositoryError> {
    let mut random = [0_u8; 12];
    getrandom(&mut random, GetRandomFlags::empty())
        .map_err(|_| error(LinuxRepositoryErrorKind::ExecutionFailed))?;
    Ok(format!("agentmage-git-inspection-{}", hex(&random)))
}

fn terminate_inspection_unit(
    manifest: &LinuxRepositoryInspectionManifest,
    unit: &str,
    runtime_directory: &str,
    session_bus: &str,
    child: &mut Child,
) -> bool {
    let _ = inspection_systemctl(
        manifest,
        runtime_directory,
        session_bus,
        ["kill", "--signal=KILL", "--kill-whom=all", unit],
    );
    let _ = inspection_systemctl(
        manifest,
        runtime_directory,
        session_bus,
        ["stop", unit, "--no-block", "--no-ask-password"],
    );
    let deadline = Instant::now() + INSPECTION_UNIT_GRACE;
    let waited = loop {
        match child.try_wait() {
            Ok(Some(_)) => break true,
            Ok(None) if Instant::now() < deadline => thread::sleep(POLL_INTERVAL),
            Ok(None) | Err(_) => {
                let _ = child.kill();
                break child.wait().is_ok();
            }
        }
    };
    let inactive = inspection_unit_is_inactive(manifest, unit, runtime_directory, session_bus);
    let _ = cleanup_inspection_unit(manifest, unit, runtime_directory, session_bus);
    waited && inactive
}

fn cleanup_inspection_unit(
    manifest: &LinuxRepositoryInspectionManifest,
    unit: &str,
    runtime_directory: &str,
    session_bus: &str,
) -> bool {
    let inactive = inspection_unit_is_inactive(manifest, unit, runtime_directory, session_bus);
    let _ = inspection_systemctl(
        manifest,
        runtime_directory,
        session_bus,
        ["reset-failed", unit, "--no-ask-password"],
    );
    inactive
}

fn inspection_unit_is_inactive(
    manifest: &LinuxRepositoryInspectionManifest,
    unit: &str,
    runtime_directory: &str,
    session_bus: &str,
) -> bool {
    if revalidate_git_artifact(&manifest.systemctl).is_err() {
        return false;
    }
    Command::new(&manifest.systemctl.launch_path)
        .env_clear()
        .env("XDG_RUNTIME_DIR", runtime_directory)
        .env("DBUS_SESSION_BUS_ADDRESS", session_bus)
        .args(["--user", "is-active", "--quiet", unit])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .ok()
        .and_then(|status| status.code())
        .is_some_and(|code| matches!(code, 3 | 4))
}

fn inspection_systemctl<const N: usize>(
    manifest: &LinuxRepositoryInspectionManifest,
    runtime_directory: &str,
    session_bus: &str,
    arguments: [&str; N],
) -> bool {
    if revalidate_git_artifact(&manifest.systemctl).is_err() {
        return false;
    }
    Command::new(&manifest.systemctl.launch_path)
        .env_clear()
        .env("XDG_RUNTIME_DIR", runtime_directory)
        .env("DBUS_SESSION_BUS_ADDRESS", session_bus)
        .arg("--user")
        .args(arguments)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
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
            if invocation.kind == GitInvocationKind::WorktreeCreate
                && !make_created_worktree_private(&self.scope.worktree_path())
            {
                let after = self
                    .collector
                    .collect(&self.scope)
                    .unwrap_or_else(|_| current.clone());
                return RepositoryPlatformResult {
                    outcome: OperationOutcome::Uncertain,
                    before: current,
                    after,
                    cleanup_verified: false,
                    platform_code: "linux.git.worktree.permissions".to_owned(),
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
        // Reported while the Git child is live, so a test can act strictly inside
        // the execution window instead of relying on thread scheduling.
        report_invocation_spawned(child.id());
        wait_bounded(&mut child, cancellation)
    }
}

/// Test-only seam reporting that a hardened Git child is running.
///
/// It observes an already-spawned process identifier and can neither choose nor
/// alter any invocation. It compiles to nothing outside tests.
#[cfg(test)]
mod invocation_observer {
    use std::sync::Mutex;

    static OBSERVER: Mutex<Option<fn(u32)>> = Mutex::new(None);

    pub(super) fn install(observer: fn(u32)) {
        *OBSERVER.lock().expect("invocation observer") = Some(observer);
    }

    pub(super) fn clear() {
        *OBSERVER.lock().expect("invocation observer") = None;
    }

    pub(super) fn report(pid: u32) {
        let observer = *OBSERVER.lock().expect("invocation observer");
        if let Some(observer) = observer {
            observer(pid);
        }
    }
}

#[cfg(test)]
fn report_invocation_spawned(pid: u32) {
    invocation_observer::report(pid);
}

#[cfg(not(test))]
const fn report_invocation_spawned(_pid: u32) {}

fn make_created_worktree_private(path: &Path) -> bool {
    let Ok(descriptor) = open(
        path,
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::empty(),
    ) else {
        return false;
    };
    let Ok(before) = fstat(&descriptor) else {
        return false;
    };
    if FileType::from_raw_mode(before.st_mode) != FileType::Directory
        || before.st_uid != getuid().as_raw()
    {
        return false;
    }
    if fchmod(&descriptor, Mode::from_raw_mode(0o700)).is_err() {
        return false;
    }
    fstat(&descriptor).is_ok_and(|after| {
        FileType::from_raw_mode(after.st_mode) == FileType::Directory
            && after.st_uid == getuid().as_raw()
            && after.st_mode & 0o777 == 0o700
    })
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

fn nul_record_count(bytes: &[u8]) -> Result<u32, LinuxRepositoryError> {
    if !bytes.is_empty() && !bytes.ends_with(&[0]) {
        return Err(error(LinuxRepositoryErrorKind::ObservationFailed));
    }
    u32::try_from(
        bytes
            .split(|byte| *byte == 0)
            .filter(|record| !record.is_empty())
            .count(),
    )
    .map_err(|_| error(LinuxRepositoryErrorKind::ObservationFailed))
}

fn framed_observations(observations: &[(&str, &[u8])]) -> Result<Vec<u8>, LinuxRepositoryError> {
    let mut framed = Vec::new();
    for (label, bytes) in observations {
        let length = u64::try_from(bytes.len())
            .map_err(|_| error(LinuxRepositoryErrorKind::ObservationFailed))?;
        framed.extend_from_slice(&(label.len() as u64).to_be_bytes());
        framed.extend_from_slice(label.as_bytes());
        framed.extend_from_slice(&length.to_be_bytes());
        framed.extend_from_slice(bytes);
        if framed.len() > MAX_OBSERVATION_BYTES {
            return Err(error(LinuxRepositoryErrorKind::ObservationFailed));
        }
    }
    Ok(framed)
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

fn hash_worktree_tree(path: &Path) -> Result<Vec<u8>, LinuxRepositoryError> {
    let mut pending = vec![path.to_path_buf()];
    let mut records = Vec::new();
    let mut total_bytes = 0_u64;
    while let Some(current) = pending.pop() {
        for entry in fs::read_dir(&current)
            .map_err(|_| error(LinuxRepositoryErrorKind::ObservationFailed))?
        {
            let entry = entry.map_err(|_| error(LinuxRepositoryErrorKind::ObservationFailed))?;
            let child = entry.path();
            let relative = child
                .strip_prefix(path)
                .map_err(|_| error(LinuxRepositoryErrorKind::ObservationFailed))?;
            if relative == Path::new(".git") {
                continue;
            }
            let metadata = fs::symlink_metadata(&child)
                .map_err(|_| error(LinuxRepositoryErrorKind::ObservationFailed))?;
            let mut record = Vec::new();
            record.extend_from_slice(relative.as_os_str().as_bytes());
            record.extend_from_slice(&metadata.mode().to_be_bytes());
            record.extend_from_slice(&metadata.len().to_be_bytes());
            record.extend_from_slice(&metadata.mtime().to_be_bytes());
            if metadata.is_dir() && !metadata.file_type().is_symlink() {
                record.push(b'd');
                pending.push(child);
            } else if metadata.is_file() {
                if metadata.len() > MAX_HASHED_FILE_BYTES {
                    return Err(error(LinuxRepositoryErrorKind::ObservationFailed));
                }
                total_bytes = total_bytes
                    .checked_add(metadata.len())
                    .filter(|total| *total <= MAX_HASHED_FILE_BYTES)
                    .ok_or_else(|| error(LinuxRepositoryErrorKind::ObservationFailed))?;
                record.push(b'f');
                record.extend_from_slice(
                    hash(
                        &fs::read(&child)
                            .map_err(|_| error(LinuxRepositoryErrorKind::ObservationFailed))?,
                    )
                    .as_bytes(),
                );
            } else if metadata.file_type().is_symlink() {
                record.push(b'l');
                record.extend_from_slice(
                    fs::read_link(&child)
                        .map_err(|_| error(LinuxRepositoryErrorKind::ObservationFailed))?
                        .as_os_str()
                        .as_bytes(),
                );
            } else {
                record.push(b's');
            }
            records.push(record);
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
    use agentmage_kernel_contracts::{
        AdapterInstanceId, BoundaryKind, CONTRACT_SCHEMA_VERSION, CancellationId,
        CancellationReason, CancellationSignal, CorrelationId, TaskId, WorkspaceAuthorizationId,
        WorkspaceId,
    };
    use agentmage_kernel_engine::propagation::CancellationToken;
    use agentmage_kernel_engine::repository_inspection::{
        RepositoryInspectionOperation, RepositoryInspectionRequest, prepare_repository_inspection,
    };
    use agentmage_kernel_engine::repository_safety::{
        OwnedWorktreeRecord, RepositorySafetyError, WorktreeDisposition, plan_branch_fast_forward,
        plan_worktree_create, plan_worktree_remove, reconcile_operation,
    };
    use std::collections::BTreeMap;
    use std::net::TcpListener;
    use std::os::unix::fs::PermissionsExt as _;
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

    use rustix::process::{Pid, Signal, kill_process};

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
            fs::set_permissions(&repository, fs::Permissions::from_mode(0o700))
                .expect("private repository directory");
            fs::set_permissions(&owned, fs::Permissions::from_mode(0o700))
                .expect("private owned directory");
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
            fs::set_permissions(&git_directory, fs::Permissions::from_mode(0o700))
                .expect("private Git directory");
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

    #[test]
    fn inspection_manifest_rejects_untrusted_executable_paths() {
        let fixture = Fixture::new();
        let untrusted = fixture.root.join("systemd-run");
        fs::write(&untrusted, b"#!/bin/sh\nexit 0\n").expect("fixture executable writes");
        fs::set_permissions(&untrusted, fs::Permissions::from_mode(0o755))
            .expect("fixture executable becomes executable");

        let error = LinuxRepositoryInspectionManifest::verify(
            &untrusted,
            "/usr/bin/systemctl",
            "/usr/bin/bwrap",
            "/usr/bin/git",
        )
        .expect_err("a user-owned launcher must fail closed");
        assert_eq!(error.kind(), LinuxRepositoryErrorKind::InvalidGitArtifact);
    }

    #[test]
    #[ignore = "requires a supported Linux user systemd session and Bubblewrap"]
    fn live_read_only_git_inspection_runs_inside_the_offline_worker() {
        let fixture = Fixture::new();
        let before = fixture
            .collector()
            .collect(&fixture.scope())
            .expect("pre-inspection manifest");
        let head = object(&fixture.repository, "HEAD");
        let workspace = crate::authorize_workspace_root(
            &fixture.repository,
            WorkspaceId::from_raw("workspace-linux-git-inspection-0001"),
            WorkspaceAuthorizationId::from_raw("authorization-linux-git-inspection-0001"),
            AdapterInstanceId::from_raw("adapter-linux-git-inspection-0001"),
        )
        .expect("workspace root authorizes");
        let manifest = LinuxRepositoryInspectionManifest::verify(
            "/usr/bin/systemd-run",
            "/usr/bin/systemctl",
            "/usr/bin/bwrap",
            "/usr/bin/git",
        )
        .expect("Git worker manifest verifies");
        let executor = LinuxBoundedRepositoryInspectionExecutor::new(manifest)
            .expect("Git worker executor constructs");
        for (index, operation) in RepositoryInspectionOperation::ALL.into_iter().enumerate() {
            let prepared = prepare_repository_inspection(
                RepositoryInspectionRequest {
                    schema_version: 1,
                    operation,
                    revision: matches!(
                        operation,
                        RepositoryInspectionOperation::Show | RepositoryInspectionOperation::Ref
                    )
                    .then(|| "HEAD".to_owned()),
                    object_id: (operation == RepositoryInspectionOperation::Object)
                        .then(|| head.clone()),
                    pathspecs: Vec::new(),
                    max_records: 32,
                    max_output_bytes: 4_096,
                },
                "a".repeat(64),
                hash(format!("operation-{index}").as_bytes()),
            )
            .expect("Git inspection prepares");
            let result = executor.run(
                &prepared,
                &workspace,
                &cancellation(&format!("live-git-inspection-{index}")),
            );

            assert_eq!(
                result.termination,
                RepositoryInspectionTermination::Exited,
                "{operation:?}: {result:?}"
            );
            if operation == RepositoryInspectionOperation::Upstream {
                assert!(
                    matches!(result.exit_code, Some(1 | 128)),
                    "{operation:?}: {result:?}"
                );
            } else {
                assert_eq!(result.exit_code, Some(0), "{operation:?}: {result:?}");
            }
            assert!(result.descendants_terminated, "{operation:?}: {result:?}");
            if operation == RepositoryInspectionOperation::Object {
                assert_eq!(result.stdout, b"commit\n", "{result:?}");
            }
        }
        assert_eq!(
            fixture
                .collector()
                .collect(&fixture.scope())
                .expect("post-inspection manifest"),
            before
        );
    }

    #[test]
    #[ignore = "requires a supported Linux user systemd session and Bubblewrap"]
    fn live_hostile_git_configuration_cannot_execute_or_contact_loopback() {
        let fixture = Fixture::new();
        let listener =
            TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0)).expect("loopback listener binds");
        listener
            .set_nonblocking(true)
            .expect("listener becomes nonblocking");
        let port = listener.local_addr().expect("listener address").port();
        let canaries = [
            "hook",
            "pager",
            "diff",
            "credential",
            "alias",
            "filter",
            "editor",
            "signer",
            "sequence",
            "textconv",
            "submodule",
            "lfs",
        ]
        .map(|name| fixture.root.join(format!("{name}-executed")));
        let hook = fixture.git_directory.join("hooks/post-index-change");
        fs::write(
            &hook,
            format!("#!/bin/sh\ntouch {}\n", canaries[0].display()),
        )
        .expect("hostile hook writes");
        fs::set_permissions(&hook, fs::Permissions::from_mode(0o755))
            .expect("hostile hook becomes executable");
        fs::write(
            fixture.repository.join(".gitattributes"),
            "*.md diff=hostile filter=hostile\n*.bin filter=lfs\n",
        )
        .expect("hostile attributes write");
        // A hostile submodule declaration pointing at the loopback listener.
        fs::write(
            fixture.repository.join(".gitmodules"),
            format!(
                "[submodule \"hostile\"]\n\tpath = hostile\n\turl = ssh://127.0.0.1:{port}/sub\n"
            ),
        )
        .expect("hostile submodule declaration writes");
        for arguments in [
            vec![
                "config".to_owned(),
                "core.pager".to_owned(),
                format!("touch {}", canaries[1].display()),
            ],
            vec![
                "config".to_owned(),
                "diff.hostile.command".to_owned(),
                format!("touch {}", canaries[2].display()),
            ],
            vec![
                "config".to_owned(),
                "credential.helper".to_owned(),
                format!("!touch {}", canaries[3].display()),
            ],
            vec![
                "config".to_owned(),
                "alias.inspect".to_owned(),
                format!("!touch {}", canaries[4].display()),
            ],
            vec![
                "config".to_owned(),
                "filter.hostile.smudge".to_owned(),
                format!("touch {}", canaries[5].display()),
            ],
            vec![
                "config".to_owned(),
                "filter.hostile.clean".to_owned(),
                format!("touch {}", canaries[5].display()),
            ],
            vec![
                "config".to_owned(),
                "core.editor".to_owned(),
                format!("touch {}", canaries[6].display()),
            ],
            vec![
                "config".to_owned(),
                "gpg.program".to_owned(),
                format!("touch {}", canaries[7].display()),
            ],
            vec![
                "config".to_owned(),
                "sequence.editor".to_owned(),
                format!("touch {}", canaries[8].display()),
            ],
            vec![
                "config".to_owned(),
                "diff.hostile.textconv".to_owned(),
                format!("touch {}", canaries[9].display()),
            ],
            // Rewrites any fetched URL onto the loopback listener.
            vec![
                "config".to_owned(),
                format!("url.ssh://127.0.0.1:{port}/.insteadOf"),
                "https://example.invalid/".to_owned(),
            ],
            vec![
                "config".to_owned(),
                "submodule.hostile.update".to_owned(),
                format!("!touch {}", canaries[10].display()),
            ],
            vec![
                "config".to_owned(),
                "filter.lfs.smudge".to_owned(),
                format!("touch {}", canaries[11].display()),
            ],
            vec![
                "config".to_owned(),
                "filter.lfs.process".to_owned(),
                format!("touch {}", canaries[11].display()),
            ],
            // Unsafe ownership relaxation must not grant anything.
            vec![
                "config".to_owned(),
                "safe.directory".to_owned(),
                "*".to_owned(),
            ],
            vec![
                "remote".to_owned(),
                "add".to_owned(),
                "origin".to_owned(),
                format!("ssh://127.0.0.1:{port}/repository"),
            ],
        ] {
            let borrowed = arguments.iter().map(String::as_str).collect::<Vec<_>>();
            run_fixture(&fixture.repository, &borrowed);
        }
        run_fixture(
            &fixture.repository,
            &[
                "update-ref",
                "refs/replace/0000000000000000000000000000000000000000",
                "HEAD",
            ],
        );
        // A hostile object alternate pointing outside the repository.
        fs::create_dir_all(fixture.git_directory.join("objects/info"))
            .expect("objects info directory");
        fs::write(
            fixture.git_directory.join("objects/info/alternates"),
            format!("{}\n", fixture.root.join("hostile-objects").display()),
        )
        .expect("hostile alternates write");
        let before = fixture
            .collector()
            .collect(&fixture.scope())
            .expect("hostile pre-inspection manifest");
        assert!(before.hazardous_configuration);

        let workspace = crate::authorize_workspace_root(
            &fixture.repository,
            WorkspaceId::from_raw("workspace-linux-hostile-git-0001"),
            WorkspaceAuthorizationId::from_raw("authorization-linux-hostile-git-0001"),
            AdapterInstanceId::from_raw("adapter-linux-hostile-git-0001"),
        )
        .expect("workspace root authorizes");
        let manifest = LinuxRepositoryInspectionManifest::verify(
            "/usr/bin/systemd-run",
            "/usr/bin/systemctl",
            "/usr/bin/bwrap",
            "/usr/bin/git",
        )
        .expect("Git worker manifest verifies");
        let executor = LinuxBoundedRepositoryInspectionExecutor::new(manifest)
            .expect("Git worker executor constructs");
        for (index, operation) in [
            RepositoryInspectionOperation::Status,
            RepositoryInspectionOperation::Diff,
            RepositoryInspectionOperation::Log,
            RepositoryInspectionOperation::Show,
            RepositoryInspectionOperation::Upstream,
        ]
        .into_iter()
        .enumerate()
        {
            let prepared = prepare_repository_inspection(
                RepositoryInspectionRequest {
                    schema_version: 1,
                    operation,
                    revision: (operation == RepositoryInspectionOperation::Show)
                        .then(|| "HEAD".to_owned()),
                    object_id: None,
                    pathspecs: Vec::new(),
                    max_records: 32,
                    max_output_bytes: 4_096,
                },
                "c".repeat(64),
                hash(format!("hostile-operation-{index}").as_bytes()),
            )
            .expect("hostile Git inspection prepares");
            let result = executor.run(
                &prepared,
                &workspace,
                &cancellation(&format!("hostile-git-{index}")),
            );
            assert_eq!(
                result.termination,
                RepositoryInspectionTermination::Exited,
                "{operation:?}: {result:?}"
            );
            assert!(result.descendants_terminated, "{operation:?}: {result:?}");
        }

        let executed_canaries = canaries
            .iter()
            .filter(|path| path.exists())
            .map(|path| path.file_name().expect("canary name").to_owned())
            .collect::<Vec<_>>();
        assert!(
            executed_canaries.is_empty(),
            "hostile Git canaries executed: {executed_canaries:?}"
        );
        assert!(matches!(
            listener.accept(),
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock
        ));
        assert_eq!(
            fixture
                .collector()
                .collect(&fixture.scope())
                .expect("hostile post-inspection manifest"),
            before
        );
    }

    // Names of ambient variables carrying the harness payload into the child. Each
    // is set only inside the child process, so the outer suite can distinguish a
    // spawned harness invocation from an ordinary run of the same #[test].
    const AMBIENT_HARNESS_REPO: &str = "AGENTMAGE_TEST_AMBIENT_HARNESS_REPO";
    const AMBIENT_HARNESS_GIT_DIR: &str = "AGENTMAGE_TEST_AMBIENT_HARNESS_GIT_DIR";
    const AMBIENT_HARNESS_OWNED: &str = "AGENTMAGE_TEST_AMBIENT_HARNESS_OWNED";
    const AMBIENT_HARNESS_RESULT: &str = "AGENTMAGE_TEST_AMBIENT_HARNESS_RESULT";

    /// Child half of the ambient-variable harness. When the outer suite runs it
    /// without the harness payload it is a no-op; when the parent test re-invokes
    /// this binary with the payload variables set, it collects the manifest
    /// under the hostile ambient environment and writes the exact digest.
    #[test]
    fn ambient_git_variables_harness_child() {
        let (Ok(repository), Ok(git_directory), Ok(owned), Ok(result)) = (
            std::env::var(AMBIENT_HARNESS_REPO),
            std::env::var(AMBIENT_HARNESS_GIT_DIR),
            std::env::var(AMBIENT_HARNESS_OWNED),
            std::env::var(AMBIENT_HARNESS_RESULT),
        ) else {
            return;
        };
        let scope = LinuxRepositoryScope::verify(
            PathBuf::from(&repository),
            PathBuf::from(&git_directory),
            PathBuf::from(&owned),
        )
        .expect("harness scope verifies under hostile ambient environment");
        let collector = LinuxRepositoryCollector::new(
            LinuxGitArtifact::verify("/usr/bin/git").expect("harness git artifact"),
        );
        let manifest = collector
            .collect(&scope)
            .expect("harness manifest collects under hostile ambient environment");
        fs::write(PathBuf::from(&result), manifest.manifest_sha256).expect("harness result writes");
    }

    /// Live end-to-end proof that ambient `GIT_*` variables cannot influence
    /// anything the collector spawns. The parent captures a clean baseline,
    /// re-invokes this test binary with a broad set of hostile ambient
    /// variables (including path-hijacking `GIT_DIR`, execution vectors like
    /// `GIT_EXTERNAL_DIFF`, and trace files), and asserts that the child's
    /// manifest digest is byte-for-byte identical to the baseline and that no
    /// canary or trace file was written.
    #[test]
    #[ignore = "requires re-invoking the test binary and a root-owned /usr/bin/git"]
    fn ambient_git_variables_never_leak_into_launched_git_children() {
        let fixture = Fixture::new();
        let baseline = fixture
            .collector()
            .collect(&fixture.scope())
            .expect("baseline manifest");

        let canary = fixture.root.join("ambient-canary");
        let trace = fixture.root.join("ambient-trace.log");
        let result = fixture.root.join("ambient-child-manifest.sha256");
        let hostile_git_dir = fixture.root.join("hostile-git-dir");
        let hostile_worktree = fixture.root.join("hostile-worktree");
        let hostile_index = fixture.root.join("hostile-index");
        let hostile_objects = fixture.root.join("hostile-objects");
        let hostile_alternates = fixture.root.join("hostile-alternates");
        let hostile_common = fixture.root.join("hostile-common");
        let hostile_config = fixture.root.join("hostile-config");
        let hostile_home = fixture.root.join("hostile-home");
        let hostile_template = fixture.root.join("hostile-template");

        let touch_canary = format!("touch {}", canary.display());
        let current_exe = std::env::current_exe().expect("test binary path");
        let output = Command::new(&current_exe)
            .env(AMBIENT_HARNESS_REPO, &fixture.repository)
            .env(AMBIENT_HARNESS_GIT_DIR, &fixture.git_directory)
            .env(AMBIENT_HARNESS_OWNED, &fixture.owned)
            .env(AMBIENT_HARNESS_RESULT, &result)
            // Path-hijacking ambient variables: if they leak, the collector's
            // observations would target the hostile paths instead of the
            // fixture and the manifest digest would change or the observation
            // would fail outright.
            .env("GIT_DIR", &hostile_git_dir)
            .env("GIT_WORK_TREE", &hostile_worktree)
            .env("GIT_INDEX_FILE", &hostile_index)
            .env("GIT_OBJECT_DIRECTORY", &hostile_objects)
            .env("GIT_ALTERNATE_OBJECT_DIRECTORIES", &hostile_alternates)
            .env("GIT_COMMON_DIR", &hostile_common)
            .env("GIT_NAMESPACE", "hostile")
            .env("GIT_CEILING_DIRECTORIES", "/")
            .env("GIT_DISCOVERY_ACROSS_FILESYSTEM", "1")
            // Execution vectors: any of these would run the canary command if
            // the ambient environment leaked and the observation touched the
            // matching Git surface.
            .env("GIT_EXTERNAL_DIFF", &touch_canary)
            .env("GIT_PAGER", &touch_canary)
            .env("GIT_EDITOR", &touch_canary)
            .env("GIT_SEQUENCE_EDITOR", &touch_canary)
            .env("GIT_ASKPASS", &touch_canary)
            .env("GIT_SSH", &touch_canary)
            .env("GIT_SSH_COMMAND", &touch_canary)
            .env("SSH_ASKPASS", &touch_canary)
            // Trace variables: if leaked, Git writes trace output to the
            // named file the moment it starts up.
            .env("GIT_TRACE", &trace)
            .env("GIT_TRACE_SETUP", &trace)
            .env("GIT_TRACE_PACKET", &trace)
            .env("GIT_TRACE_PERFORMANCE", &trace)
            .env("GIT_TRACE_PACK_ACCESS", &trace)
            // Configuration overrides that would relax the collector's
            // hardening if the executor forwarded ambient values instead of
            // replacing them.
            .env("GIT_CONFIG_NOSYSTEM", "0")
            .env("GIT_CONFIG_SYSTEM", &hostile_config)
            .env("GIT_CONFIG_GLOBAL", &hostile_config)
            .env("GIT_TERMINAL_PROMPT", "1")
            .env("GIT_OPTIONAL_LOCKS", "1")
            .env("GIT_LFS_SKIP_SMUDGE", "0")
            .env("GIT_TEMPLATE_DIR", &hostile_template)
            .env("GIT_ATTR_SOURCE", "hostile")
            .env("GIT_AUTHOR_NAME", "Hostile Author")
            .env("GIT_AUTHOR_EMAIL", "hostile@example.invalid")
            .env("GIT_COMMITTER_NAME", "Hostile Committer")
            .env("GIT_COMMITTER_EMAIL", "hostile@example.invalid")
            .env("HOME", &hostile_home)
            .env("XDG_CONFIG_HOME", &hostile_home)
            .env("GCM_INTERACTIVE", "Always")
            .args([
                "repository_safety::tests::ambient_git_variables_harness_child",
                "--exact",
                "--include-ignored",
                "--test-threads=1",
                "--nocapture",
            ])
            .stdin(Stdio::null())
            .output()
            .expect("harness subprocess spawns");
        assert!(
            output.status.success(),
            "harness subprocess failed: status={:?} stdout={} stderr={}",
            output.status.code(),
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr),
        );

        let child_digest = fs::read_to_string(&result).expect("harness result reads");
        assert_eq!(
            child_digest.trim(),
            baseline.manifest_sha256,
            "ambient GIT_* variables changed the manifest observed by the child"
        );
        assert!(
            !canary.exists(),
            "ambient GIT_* execution vector fired: {canary:?}"
        );
        assert!(
            !trace.exists(),
            "ambient GIT_TRACE leaked into a hardened Git child: {trace:?}"
        );

        // The hostile paths were only ever named through the environment; the
        // executor must not have created any of them on the filesystem.
        for path in [
            &hostile_git_dir,
            &hostile_worktree,
            &hostile_index,
            &hostile_objects,
            &hostile_alternates,
            &hostile_common,
            &hostile_config,
            &hostile_home,
            &hostile_template,
        ] {
            assert!(
                !path.exists(),
                "hostile ambient path materialized on disk: {path:?}"
            );
        }
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
    fn repository_inventory_preserves_exact_git_states_and_nonregular_hints() {
        let fixture = Fixture::new();
        fs::write(fixture.repository.join(".gitignore"), "ignored.log\n")
            .expect("ignore rule writes");
        fs::write(fixture.repository.join("tracked.rs"), "fn tracked() {}\n")
            .expect("tracked source writes");
        std::os::unix::fs::symlink("tracked.rs", fixture.repository.join("tracked-link"))
            .expect("tracked symlink writes");
        run_fixture(
            &fixture.repository,
            &["add", ".gitignore", "tracked.rs", "tracked-link"],
        );
        let head = object(&fixture.repository, "HEAD");
        run_fixture(
            &fixture.repository,
            &[
                "update-index",
                "--add",
                "--cacheinfo",
                &format!("160000,{head},vendor/dependency"),
            ],
        );
        run_fixture(
            &fixture.repository,
            &[
                "-c",
                "user.name=AgentMage Fixture",
                "-c",
                "user.email=fixture@example.test",
                "commit",
                "-m",
                "inventory fixture",
            ],
        );
        fs::write(fixture.repository.join("staged.rs"), "fn staged() {}\n")
            .expect("staged source writes");
        run_fixture(&fixture.repository, &["add", "staged.rs"]);
        fs::write(fixture.repository.join("untracked.txt"), "untracked\n")
            .expect("untracked source writes");
        fs::write(fixture.repository.join("ignored.log"), "ignored\n")
            .expect("ignored source writes");

        let collector = fixture.collector();
        let first = collector
            .collect_inventory(&fixture.scope())
            .expect("live inventory");
        let second = collector
            .collect_inventory(&fixture.scope())
            .expect("stable live inventory");
        assert_eq!(first, second);
        assert_eq!(first.object_format, "sha1");
        assert_eq!(first.branch.as_deref(), Some("refs/heads/main"));
        assert_eq!(first.entries.len(), 8);

        let entry = |path: &str| {
            first
                .entries
                .iter()
                .find(|entry| entry.path.join("/") == path)
                .expect("inventory path")
        };
        assert_eq!(
            entry("tracked-link").object_hint,
            crate::LinuxRepositoryObjectHint::SymbolicLink
        );
        assert_eq!(
            entry("vendor/dependency").object_hint,
            crate::LinuxRepositoryObjectHint::Gitlink
        );
        assert!(matches!(
            entry("staged.rs").state,
            crate::LinuxRepositoryInventoryState::Tracked {
                staged_changed: true,
                conflicted: false,
                ..
            }
        ));
        assert!(matches!(
            entry("untracked.txt").state,
            crate::LinuxRepositoryInventoryState::Untracked
        ));
        assert!(matches!(
            entry("ignored.log").state,
            crate::LinuxRepositoryInventoryState::Ignored
        ));
    }

    #[test]
    fn repository_inventory_cancellation_fails_before_git_launch() {
        let fixture = Fixture::new();
        let task_id = TaskId::from_raw("task-inventory-cancel-0001");
        let correlation_id = CorrelationId::from_raw("correlation-inventory-cancel-0001");
        let cancellation = CancellationToken::root(
            BoundaryKind::Kernel,
            task_id.clone(),
            correlation_id.clone(),
        );
        cancellation
            .cancel(CancellationSignal {
                schema_version: CONTRACT_SCHEMA_VERSION,
                cancellation_id: CancellationId::from_raw("cancel-inventory-0001"),
                correlation_id,
                task_id,
                reason: CancellationReason::UserRequested,
                requested_by: BoundaryKind::Kernel,
            })
            .expect("inventory cancellation");
        let error = fixture
            .collector()
            .collect_inventory_cancellable(&fixture.scope(), &cancellation)
            .expect_err("pre-cancelled inventory fails closed");
        assert_eq!(error.kind(), LinuxRepositoryErrorKind::ObservationFailed);
    }

    #[test]
    fn repository_inventory_timeout_kills_and_reaps_the_stalled_process() {
        let mut command = Command::new("/usr/bin/sleep");
        command.arg("10").stdin(Stdio::null());
        let started = Instant::now();
        assert!(
            run_inventory_process(&mut command, 1_024, None, Duration::from_millis(20)).is_err()
        );
        assert!(started.elapsed() < Duration::from_secs(2));
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
        assert_eq!(
            fs::metadata(&worktree_path)
                .expect("worktree metadata")
                .permissions()
                .mode()
                & 0o777,
            0o700
        );

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

    /// Exactly what the user's active checkout looked like, for byte-for-byte comparison.
    #[derive(Debug, PartialEq, Eq)]
    struct ActiveCheckoutState {
        head: String,
        abbreviated_ref: String,
        index: Vec<u8>,
        status: Vec<u8>,
        readme: Vec<u8>,
        untracked: Option<Vec<u8>>,
    }

    fn capture_active_checkout(fixture: &Fixture) -> ActiveCheckoutState {
        ActiveCheckoutState {
            head: object(&fixture.repository, "HEAD"),
            abbreviated_ref: String::from_utf8(run_fixture(
                &fixture.repository,
                &["rev-parse", "--abbrev-ref", "HEAD"],
            ))
            .expect("ref UTF-8")
            .trim()
            .to_owned(),
            index: fs::read(fixture.git_directory.join("index")).expect("index bytes"),
            status: run_fixture(&fixture.repository, &["status", "--porcelain"]),
            readme: fs::read(fixture.repository.join("README.md")).expect("readme bytes"),
            untracked: fs::read(fixture.repository.join("untracked.txt")).ok(),
        }
    }

    /// Creates one owned worktree and returns its path and sealed ownership record.
    fn create_owned_worktree(
        fixture: &Fixture,
        executor: &mut LinuxRepositoryExecutor,
        scope: &LinuxRepositoryScope,
        cancellation: &CancellationToken,
        label: &str,
    ) -> (PathBuf, OwnedWorktreeRecord) {
        let source = object(&fixture.repository, "HEAD");
        // The executor pins the destination to the scope's own worktree path.
        let worktree_path = fixture.owned.join("worktree");
        let before = executor.collector.collect(scope).expect("before create");
        let plan = plan_worktree_create(
            "transaction-create",
            "task-1",
            &source,
            scope.repository_path_sha256(),
            &hash(worktree_path.as_os_str().as_bytes()),
            &before,
        )
        .expect("create plan");
        let created = executor.run(&plan, &before, cancellation);
        assert_eq!(created.outcome, OperationOutcome::Succeeded, "{label}");
        reconcile_operation(&plan, created, "authority-create", "attempt-create")
            .expect("create reconciles");
        let record = eligible_record(
            linux_repository_path_sha256(&worktree_path).expect("worktree identity"),
            source,
        );
        (worktree_path, record)
    }

    /// The owned worktree lifecycle must never disturb the user's active checkout, in any
    /// of its states, and ownership identity must stay stable across the whole lifecycle.
    #[test]
    fn worktree_lifecycle_preserves_a_dirty_active_checkout() {
        let fixture = Fixture::new();
        // Unstaged modification, a staged change, and an untracked file.
        fs::write(fixture.repository.join("README.md"), b"fixture dirty\n").expect("dirty write");
        fs::write(fixture.repository.join("staged.txt"), b"staged\n").expect("staged write");
        run_fixture(&fixture.repository, &["add", "staged.txt"]);
        fs::write(fixture.repository.join("untracked.txt"), b"untracked\n")
            .expect("untracked write");
        let before_state = capture_active_checkout(&fixture);
        assert!(!before_state.status.is_empty(), "fixture is not dirty");

        let scope = fixture.scope();
        let cancellation = cancellation("worktree-dirty");
        let mut executor = LinuxRepositoryExecutor::new(fixture.collector(), scope.clone());
        let (worktree_path, record) = create_owned_worktree(
            &fixture,
            &mut executor,
            &scope,
            &cancellation,
            "worktree-dirty",
        );
        assert_eq!(capture_active_checkout(&fixture), before_state);

        let before_remove = executor.collector.collect(&scope).expect("before remove");
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
        assert_eq!(capture_active_checkout(&fixture), before_state);
    }

    #[test]
    fn worktree_lifecycle_preserves_a_detached_active_checkout() {
        let fixture = Fixture::new();
        let detached_at = object(&fixture.repository, "HEAD");
        run_fixture(&fixture.repository, &["checkout", "--detach"]);
        let before_state = capture_active_checkout(&fixture);
        assert_eq!(
            before_state.abbreviated_ref, "HEAD",
            "checkout is not detached"
        );
        assert_eq!(before_state.head, detached_at);

        let scope = fixture.scope();
        let cancellation = cancellation("worktree-detached");
        let mut executor = LinuxRepositoryExecutor::new(fixture.collector(), scope.clone());
        let (worktree_path, record) = create_owned_worktree(
            &fixture,
            &mut executor,
            &scope,
            &cancellation,
            "worktree-detached",
        );
        assert_eq!(capture_active_checkout(&fixture), before_state);

        let before_remove = executor.collector.collect(&scope).expect("before remove");
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
        assert_eq!(capture_active_checkout(&fixture), before_state);
    }

    /// Everything the user owns that an interrupted AgentMage operation must never touch.
    #[derive(Debug, PartialEq, Eq)]
    struct UserGitState {
        checkout: ActiveCheckoutState,
        refs: Vec<u8>,
        tags: Vec<u8>,
        notes: Vec<u8>,
        stash: Vec<u8>,
        config: Vec<u8>,
        hook: Vec<u8>,
    }

    /// Seeds refs, tags, notes, stash, configuration, a hook, and dirty working state.
    fn seed_user_git_state(fixture: &Fixture) {
        let repository = &fixture.repository;
        let author: &[&str] = &[
            "-c",
            "user.name=AgentMage Fixture",
            "-c",
            "user.email=fixture@example.test",
        ];
        run_fixture(repository, &["branch", "user/topic"]);
        run_fixture(repository, &["tag", "user-tag"]);
        let mut note = author.to_vec();
        note.extend_from_slice(&["notes", "add", "-m", "user note", "HEAD"]);
        run_fixture(repository, &note);
        fs::write(repository.join("README.md"), b"user stashed\n").expect("stash source");
        let mut stash = author.to_vec();
        stash.extend_from_slice(&["stash", "push", "-m", "user stash"]);
        run_fixture(repository, &stash);
        run_fixture(
            repository,
            &["config", "user.agentmageFixture", "preserved"],
        );
        let hooks = fixture.git_directory.join("hooks");
        fs::create_dir_all(&hooks).expect("hooks directory");
        fs::write(hooks.join("pre-commit"), b"#!/bin/sh\nexit 1\n").expect("hook writes");
        // Dirty working state on top of everything above.
        fs::write(repository.join("README.md"), b"user dirty\n").expect("dirty write");
        fs::write(repository.join("untracked.txt"), b"untracked\n").expect("untracked write");
    }

    fn capture_user_git_state(fixture: &Fixture) -> UserGitState {
        UserGitState {
            checkout: capture_active_checkout(fixture),
            refs: run_fixture(&fixture.repository, &["show-ref"]),
            tags: run_fixture(&fixture.repository, &["tag", "--list"]),
            notes: run_fixture(&fixture.repository, &["notes", "list"]),
            stash: run_fixture(&fixture.repository, &["stash", "list"]),
            config: run_fixture(
                &fixture.repository,
                &["config", "--get", "user.agentmageFixture"],
            ),
            hook: fs::read(fixture.git_directory.join("hooks/pre-commit")).expect("hook bytes"),
        }
    }

    fn cancelled_token(label: &str) -> CancellationToken {
        let token = cancellation(label);
        token
            .cancel(CancellationSignal {
                schema_version: CONTRACT_SCHEMA_VERSION,
                cancellation_id: CancellationId::from_raw(format!("cancel-{label}")),
                correlation_id: CorrelationId::from_raw(format!("correlation-{label}")),
                task_id: TaskId::from_raw(format!("task-{label}")),
                reason: CancellationReason::UserRequested,
                requested_by: BoundaryKind::Kernel,
            })
            .expect("cancellation");
        token
    }

    /// An interrupted worktree creation must leave every user-owned artifact identical
    /// and must report an indeterminate outcome whenever repository state actually moved.
    #[test]
    fn interrupted_worktree_create_preserves_every_user_artifact() {
        let fixture = Fixture::new();
        seed_user_git_state(&fixture);
        let before_state = capture_user_git_state(&fixture);

        let scope = fixture.scope();
        let collector = fixture.collector();
        let before = collector.collect(&scope).expect("before");
        let plan = plan_worktree_create(
            "transaction-create",
            "task-1",
            &object(&fixture.repository, "HEAD"),
            scope.repository_path_sha256(),
            &hash(scope.owned_root.join("worktree").as_os_str().as_bytes()),
            &before,
        )
        .expect("create plan");
        let mut executor = LinuxRepositoryExecutor::new(collector, scope.clone());
        let result = executor.run(&plan, &before, &cancelled_token("worktree-interrupt"));

        // Whatever the race decided, the report must match what actually happened.
        if result.after != result.before {
            assert_eq!(result.outcome, OperationOutcome::Uncertain, "{result:?}");
        } else {
            assert_ne!(result.outcome, OperationOutcome::Uncertain, "{result:?}");
        }
        assert_eq!(capture_user_git_state(&fixture), before_state);

        // A partially created worktree is owned scratch, never user state.
        let worktree = scope.owned_root.join("worktree");
        if worktree.exists() {
            fs::remove_dir_all(&worktree).expect("owned scratch removes");
        }
    }

    /// Coordinates the concurrent-mutation case: the observer callback is a plain
    /// function pointer, so shared state lives here.
    static CONCURRENT_REPOSITORY: Mutex<Option<PathBuf>> = Mutex::new(None);
    /// Only the thread running this fixture's removal may trigger the mutation, so a
    /// parallel test's Git invocation can never consume or misfire the observer.
    static CONCURRENT_THREAD: Mutex<Option<std::thread::ThreadId>> = Mutex::new(None);
    static CONCURRENT_STOPPED: AtomicBool = AtomicBool::new(false);
    static CONCURRENT_MUTATED: AtomicBool = AtomicBool::new(false);

    /// Resumes the exact stopped child on every exit path, including a panic.
    struct ResumeChildGuard(Pid);

    impl Drop for ResumeChildGuard {
        fn drop(&mut self) {
            let _ = kill_process(self.0, Signal::CONT);
        }
    }

    /// Clears the observer and every shared campaign value on every exit path.
    struct ConcurrentStateGuard;

    impl Drop for ConcurrentStateGuard {
        fn drop(&mut self) {
            super::invocation_observer::clear();
            *CONCURRENT_REPOSITORY.lock().expect("concurrent repository") = None;
            *CONCURRENT_THREAD.lock().expect("concurrent thread") = None;
        }
    }

    /// Reads the scheduler state letter of one exact process.
    fn process_state(pid: u32) -> Option<char> {
        let stat = fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
        stat.rsplit_once(')')?
            .1
            .split_whitespace()
            .next()?
            .chars()
            .next()
    }

    /// Stops the exact hardened Git child, proves it is stopped, mutates the user's
    /// repository, and resumes it. While stopped the child provably cannot complete,
    /// so the overlap is guaranteed rather than observed after the fact.
    fn mutate_during_removal(git_pid: u32) {
        let expected = *CONCURRENT_THREAD.lock().expect("concurrent thread");
        if expected != Some(std::thread::current().id()) {
            return;
        }
        if CONCURRENT_MUTATED.swap(true, Ordering::SeqCst) {
            return;
        }
        let pid = Pid::from_raw(i32::try_from(git_pid).expect("child pid fits"))
            .expect("child pid is valid");
        kill_process(pid, Signal::STOP).expect("hardened Git child stops");
        let _resume = ResumeChildGuard(pid);
        let deadline = Instant::now() + Duration::from_secs(5);
        let mut stopped = false;
        while Instant::now() < deadline {
            if process_state(git_pid) == Some('T') {
                stopped = true;
                break;
            }
            thread::sleep(Duration::from_millis(1));
        }
        CONCURRENT_STOPPED.store(stopped, Ordering::SeqCst);

        let repository = CONCURRENT_REPOSITORY
            .lock()
            .expect("concurrent repository")
            .clone()
            .expect("concurrent repository set");
        fs::write(
            repository.join("concurrent.txt"),
            b"written during removal\n",
        )
        .expect("concurrent file");
        fs::write(
            repository.join("README.md"),
            b"user edited during removal\n",
        )
        .expect("concurrent edit");
        run_fixture(&repository, &["branch", "user/concurrent"]);
        run_fixture(&repository, &["tag", "user-concurrent-tag"]);
        run_fixture(
            &repository,
            &["config", "user.agentmageConcurrent", "written"],
        );
    }

    /// Exact ref name to object identifier, for comparison without substring matching.
    fn ref_map(fixture: &Fixture) -> BTreeMap<String, String> {
        String::from_utf8(run_fixture(&fixture.repository, &["show-ref"]))
            .expect("refs UTF-8")
            .lines()
            .filter_map(|line| line.split_once(' '))
            .map(|(object, name)| (name.to_owned(), object.to_owned()))
            .collect()
    }

    /// The user works inside the exact window in which AgentMage removes its own
    /// worktree, with the removal's Git child held stopped so it cannot finish first.
    #[test]
    fn concurrent_user_changes_survive_worktree_removal_and_cleanup() {
        let fixture = Fixture::new();
        seed_user_git_state(&fixture);
        let seeded = capture_user_git_state(&fixture);
        let seeded_refs = ref_map(&fixture);
        let head = object(&fixture.repository, "HEAD");
        let scope = fixture.scope();
        let cancellation = cancellation("worktree-concurrent");
        let mut executor = LinuxRepositoryExecutor::new(fixture.collector(), scope.clone());
        let (worktree_path, record) = create_owned_worktree(
            &fixture,
            &mut executor,
            &scope,
            &cancellation,
            "worktree-concurrent",
        );
        assert!(worktree_path.is_dir(), "worktree was not created");

        let before_remove = executor.collector.collect(&scope).expect("before remove");
        let removal = plan_worktree_remove(
            "transaction-remove",
            &record,
            scope.repository_path_sha256(),
            &before_remove,
        )
        .expect("remove plan");

        let removed = {
            let _state = ConcurrentStateGuard;
            *CONCURRENT_REPOSITORY.lock().expect("concurrent repository") =
                Some(fixture.repository.clone());
            *CONCURRENT_THREAD.lock().expect("concurrent thread") =
                Some(std::thread::current().id());
            CONCURRENT_STOPPED.store(false, Ordering::SeqCst);
            CONCURRENT_MUTATED.store(false, Ordering::SeqCst);
            super::invocation_observer::install(mutate_during_removal);
            executor.run(&removal, &before_remove, &cancellation)
        };

        // The mutation ran, and the exact Git child was held stopped throughout it.
        assert!(
            CONCURRENT_MUTATED.load(Ordering::SeqCst),
            "mutation never ran"
        );
        assert!(
            CONCURRENT_STOPPED.load(Ordering::SeqCst),
            "the hardened Git child was not stopped during the mutation"
        );

        // Exact postconditions, not a menu of permitted outcomes.
        assert_eq!(removed.outcome, OperationOutcome::Succeeded, "{removed:?}");
        assert!(removed.cleanup_verified, "{removed:?}");
        assert_eq!(removed.platform_code, "linux.git.succeeded");
        assert!(!worktree_path.exists(), "owned worktree survived removal");
        assert_ne!(removed.after, removed.before, "{removed:?}");
        // Protected state moved underneath the operation, so no receipt may claim a
        // clean removal: reconciliation refuses rather than attesting to it.
        assert_eq!(
            reconcile_operation(&removal, removed, "authority-remove", "attempt-remove")
                .expect_err("concurrent movement must not reconcile"),
            RepositorySafetyError::PreservationMismatch
        );

        // Seeded artifacts are identical except where the user themselves changed them.
        let after = capture_user_git_state(&fixture);
        assert_eq!(after.notes, seeded.notes);
        assert_eq!(after.stash, seeded.stash);
        assert_eq!(after.hook, seeded.hook);
        assert_eq!(after.config, seeded.config);
        assert_eq!(after.checkout.head, seeded.checkout.head);
        assert_eq!(
            after.checkout.abbreviated_ref,
            seeded.checkout.abbreviated_ref
        );
        assert_eq!(after.checkout.index, seeded.checkout.index);
        assert_eq!(after.checkout.untracked, seeded.checkout.untracked);

        // Every seeded ref keeps its exact name and object identifier.
        let after_refs = ref_map(&fixture);
        for (name, object_id) in &seeded_refs {
            assert_eq!(
                after_refs.get(name),
                Some(object_id),
                "seeded ref {name} changed or vanished"
            );
        }
        // Refs created inside the window exist under their exact names and objects.
        assert_eq!(
            after_refs.get("refs/heads/user/concurrent"),
            Some(&head),
            "{after_refs:?}"
        );
        assert_eq!(
            after_refs.get("refs/tags/user-concurrent-tag"),
            Some(&head),
            "{after_refs:?}"
        );
        // Nothing else appeared. The only permitted addition outside the two user refs
        // is AgentMage's own task branch, which the worktree creation owns.
        let unexpected: Vec<&String> = after_refs
            .keys()
            .filter(|name| {
                !seeded_refs.contains_key(*name)
                    && name.as_str() != "refs/heads/user/concurrent"
                    && name.as_str() != "refs/tags/user-concurrent-tag"
                    && !name.starts_with("refs/heads/agentmage/tasks/")
            })
            .collect();
        assert!(
            unexpected.is_empty(),
            "unexpected refs appeared: {unexpected:?}"
        );

        assert_eq!(
            fs::read(fixture.repository.join("concurrent.txt")).expect("concurrent file"),
            b"written during removal\n"
        );
        assert_eq!(after.checkout.readme, b"user edited during removal\n");
        assert_eq!(
            run_fixture(
                &fixture.repository,
                &["config", "--get", "user.agentmageConcurrent"],
            ),
            b"written\n"
        );
    }

    /// An interrupted compare-and-swap must leave the branch either exactly where it was
    /// or exactly at the proven descendant, and must never disturb any other ref.
    #[test]
    fn interrupted_branch_fast_forward_preserves_every_user_artifact() {
        let fixture = Fixture::new();
        let first = object(&fixture.repository, "HEAD");
        run_fixture(
            &fixture.repository,
            &["branch", "agentmage/tasks/interrupt", &first],
        );
        fs::write(fixture.repository.join("README.md"), b"second\n").expect("second write");
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
        seed_user_git_state(&fixture);
        let before_state = capture_user_git_state(&fixture);

        let scope = fixture.scope();
        let collector = fixture.collector();
        let before = collector.collect(&scope).expect("before");
        let plan = plan_branch_fast_forward(
            "transaction-cas",
            "refs/heads/agentmage/tasks/interrupt",
            &first,
            &second,
            true,
            scope.repository_path_sha256(),
            &before,
        )
        .expect("fast-forward plan");
        let mut executor = LinuxRepositoryExecutor::new(collector, scope.clone());
        let result = executor.run(&plan, &before, &cancelled_token("cas-interrupt"));

        if result.after != result.before {
            assert_eq!(result.outcome, OperationOutcome::Uncertain, "{result:?}");
        } else {
            assert_ne!(result.outcome, OperationOutcome::Uncertain, "{result:?}");
        }
        // The task branch may hold only its old or its exact proven new object.
        let moved = object(&fixture.repository, "agentmage/tasks/interrupt");
        assert!(moved == first || moved == second, "{moved}");
        // Every user-owned artifact other than the AgentMage task branch is identical.
        let after_state = capture_user_git_state(&fixture);
        assert_eq!(after_state.checkout, before_state.checkout);
        assert_eq!(after_state.tags, before_state.tags);
        assert_eq!(after_state.notes, before_state.notes);
        assert_eq!(after_state.stash, before_state.stash);
        assert_eq!(after_state.config, before_state.config);
        assert_eq!(after_state.hook, before_state.hook);
    }

    /// A worktree that vanished underneath the owner can no longer be identified, so
    /// removal must fail closed rather than act on whatever now occupies the path.
    #[test]
    fn worktree_removal_fails_closed_when_the_owned_directory_is_missing() {
        let fixture = Fixture::new();
        let scope = fixture.scope();
        let cancellation = cancellation("worktree-missing");
        let mut executor = LinuxRepositoryExecutor::new(fixture.collector(), scope.clone());
        let (worktree_path, record) = create_owned_worktree(
            &fixture,
            &mut executor,
            &scope,
            &cancellation,
            "worktree-missing",
        );
        fs::remove_dir_all(&worktree_path).expect("worktree removed outside AgentMage");
        let before_state = capture_active_checkout(&fixture);

        let before_remove = executor.collector.collect(&scope).expect("before remove");
        let removal = plan_worktree_remove(
            "transaction-remove",
            &record,
            scope.repository_path_sha256(),
            &before_remove,
        )
        .expect("remove plan");
        let removed = executor.run(&removal, &before_remove, &cancellation);
        assert_eq!(removed.outcome, OperationOutcome::Denied, "{removed:?}");
        assert_eq!(removed.platform_code, "linux.git.worktree.identity");
        assert_eq!(removed.before, removed.after);
        assert_eq!(capture_active_checkout(&fixture), before_state);
    }

    /// A renamed worktree no longer matches its sealed ownership identity, so removal
    /// must fail closed and leave the renamed directory untouched.
    #[test]
    fn worktree_removal_fails_closed_when_the_owned_directory_is_renamed() {
        let fixture = Fixture::new();
        let scope = fixture.scope();
        let cancellation = cancellation("worktree-renamed");
        let mut executor = LinuxRepositoryExecutor::new(fixture.collector(), scope.clone());
        let (worktree_path, record) = create_owned_worktree(
            &fixture,
            &mut executor,
            &scope,
            &cancellation,
            "worktree-renamed",
        );
        let renamed = fixture.owned.join("renamed-worktree");
        fs::rename(&worktree_path, &renamed).expect("worktree renamed outside AgentMage");
        let before_state = capture_active_checkout(&fixture);

        let before_remove = executor.collector.collect(&scope).expect("before remove");
        let removal = plan_worktree_remove(
            "transaction-remove",
            &record,
            scope.repository_path_sha256(),
            &before_remove,
        )
        .expect("remove plan");
        let removed = executor.run(&removal, &before_remove, &cancellation);
        assert_eq!(removed.outcome, OperationOutcome::Denied, "{removed:?}");
        assert_eq!(removed.platform_code, "linux.git.worktree.identity");
        assert_eq!(removed.before, removed.after);
        assert!(
            renamed.join("README.md").is_file(),
            "renamed tree was disturbed"
        );
        assert!(!worktree_path.exists());
        assert_eq!(capture_active_checkout(&fixture), before_state);
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
