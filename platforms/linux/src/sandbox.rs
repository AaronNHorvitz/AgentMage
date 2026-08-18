//! Fresh Linux worker isolation with descriptor-bound Bubblewrap mounts.

use std::ffi::OsString;
use std::fmt;
use std::io::{Read, Write};
use std::os::fd::AsRawFd;
use std::path::{Component, Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use agentmage_kernel_contracts::{
    GrantOperation, GrantTarget, HeldWorkspaceObject, OperationOutcome, PathResolutionIntent,
    SnapshotEntry, SnapshotEntryKind, StateChange, WorkspaceObjectKind, WorkspacePath,
    WorkspaceSnapshot,
};
use agentmage_kernel_engine::authority_transaction::{
    EffectAuthorization, EffectDriver, EffectLaunch, EffectResult,
};
use rustix::fd::OwnedFd;
use rustix::fs::{
    Dir, FileType, MemfdFlags, Mode, OFlags, SealFlags, SeekFrom, fcntl_add_seals, fstat,
    memfd_create, open, seek,
};
use rustix::io::{pread, write};
use rustix::process::getuid;
use rustix::rand::{GetRandomFlags, getrandom};
use seccompiler::{BpfProgram, TargetArch, compile_from_json};
use sha2::{Digest, Sha256};

use crate::LinuxHeldObject;

const MAX_RUNTIME_FILES: usize = 16;
const MAX_OUTPUT_BYTES: usize = 4 * 1024 * 1024;
const MAX_DIRECTORY_ENTRIES: usize = 4_096;
const MAX_DIRECTORY_PROJECTION_BYTES: usize = 1024 * 1024;
const HASH_BUFFER_BYTES: usize = 64 * 1024;
const MAX_READ_ONLY_HELD_OBJECTS: usize = 256;
const MAX_READ_ONLY_REQUEST_BYTES: usize = 64 * 1024;
const MAX_READ_ONLY_SNAPSHOT_BYTES: usize = 128 * 1024 * 1024;
const WORKER_GUEST_ROOT: &str = "/app";
const PATH_EXECUTOR: &str = "/usr/bin/env";
const SYSTEMCTL: &str = "/usr/bin/systemctl";
const SECCOMP_POLICY_ID: &str = "agentmage.linux.worker.deny.v1";
const DENIED_SYSCALLS: &[&str] = &[
    "accept",
    "accept4",
    "acct",
    "add_key",
    "adjtimex",
    "bind",
    "bpf",
    "chroot",
    "clone3",
    "clock_settime",
    "connect",
    "delete_module",
    "finit_module",
    "fanotify_init",
    "fsmount",
    "fsopen",
    "fspick",
    "init_module",
    "io_uring_enter",
    "io_uring_register",
    "io_uring_setup",
    "kexec_file_load",
    "kexec_load",
    "keyctl",
    "listen",
    "lookup_dcookie",
    "memfd_create",
    "mount",
    "mount_setattr",
    "move_mount",
    "name_to_handle_at",
    "open_by_handle_at",
    "perf_event_open",
    "pidfd_getfd",
    "pidfd_open",
    "pidfd_send_signal",
    "pivot_root",
    "process_vm_readv",
    "process_vm_writev",
    "ptrace",
    "quotactl",
    "reboot",
    "recvfrom",
    "recvmmsg",
    "recvmsg",
    "request_key",
    "sendmmsg",
    "sendmsg",
    "sendto",
    "setdomainname",
    "sethostname",
    "setns",
    "shutdown",
    "socket",
    "socketpair",
    "swapoff",
    "swapon",
    "syslog",
    "umount2",
    "unshare",
    "userfaultfd",
    "vhangup",
];

/// Content-free Linux sandbox failure categories.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LinuxSandboxErrorKind {
    /// A declared trusted executable or runtime file is invalid.
    InvalidManifest,
    /// A requested workspace path belongs to a different authorization.
    WorkspaceMismatch,
    /// The operation, permit, and continuously held object do not match exactly.
    TargetMismatch,
    /// The continuously held object changed before the worker launch.
    StaleObject,
    /// An immutable file projection could not be constructed safely.
    FileProjectionFailed,
    /// A bounded directory projection could not be constructed safely.
    DirectoryProjectionFailed,
    /// A resource bound is outside the supported range.
    InvalidLimit,
    /// The seccomp policy could not be compiled for the running architecture.
    SeccompUnavailable,
    /// A required Linux isolation mechanism could not be started.
    IsolationUnavailable,
    /// The worker or isolation supervisor could not be completed.
    ExecutionFailed,
    /// Worker output exceeded its declared bound.
    OutputLimitExceeded,
}

impl LinuxSandboxErrorKind {
    /// Returns a stable machine-readable code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidManifest => "linux.sandbox.manifest.invalid",
            Self::WorkspaceMismatch => "linux.sandbox.workspace.mismatch",
            Self::TargetMismatch => "linux.sandbox.target.mismatch",
            Self::StaleObject => "linux.sandbox.target.stale",
            Self::FileProjectionFailed => "linux.sandbox.file_projection.failed",
            Self::DirectoryProjectionFailed => "linux.sandbox.directory_projection.failed",
            Self::InvalidLimit => "linux.sandbox.limit.invalid",
            Self::SeccompUnavailable => "linux.sandbox.seccomp.unavailable",
            Self::IsolationUnavailable => "linux.sandbox.isolation.unavailable",
            Self::ExecutionFailed => "linux.sandbox.execution.failed",
            Self::OutputLimitExceeded => "linux.sandbox.output.exceeded",
        }
    }
}

/// Redacted Linux sandbox error.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LinuxSandboxError {
    kind: LinuxSandboxErrorKind,
}

impl LinuxSandboxError {
    /// Returns the stable failure category.
    #[must_use]
    pub const fn kind(self) -> LinuxSandboxErrorKind {
        self.kind
    }
}

impl fmt::Display for LinuxSandboxError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.kind.code())
    }
}

impl std::error::Error for LinuxSandboxError {}

/// One runtime file mounted into the worker at an exact absolute destination.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LinuxWorkerRuntimeFile {
    host_path: PathBuf,
    guest_path: PathBuf,
}

impl LinuxWorkerRuntimeFile {
    /// Declares a host runtime file and its absolute path inside the worker.
    #[must_use]
    pub fn new(host_path: impl Into<PathBuf>, guest_path: impl Into<PathBuf>) -> Self {
        Self {
            host_path: host_path.into(),
            guest_path: guest_path.into(),
        }
    }
}

/// Per-worker cgroup and output limits.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LinuxSandboxLimits {
    memory_bytes: u64,
    task_count: u32,
    cpu_percent: u16,
    runtime_seconds: u16,
    output_bytes: usize,
}

impl LinuxSandboxLimits {
    /// Validates bounded worker limits.
    pub fn new(
        memory_bytes: u64,
        task_count: u32,
        cpu_percent: u16,
        runtime_seconds: u16,
        output_bytes: usize,
    ) -> Result<Self, LinuxSandboxError> {
        if !(16 * 1024 * 1024..=4 * 1024 * 1024 * 1024).contains(&memory_bytes)
            || !(1..=64).contains(&task_count)
            || !(1..=400).contains(&cpu_percent)
            || !(1..=300).contains(&runtime_seconds)
            || !(1..=MAX_OUTPUT_BYTES).contains(&output_bytes)
        {
            return Err(error(LinuxSandboxErrorKind::InvalidLimit));
        }
        Ok(Self {
            memory_bytes,
            task_count,
            cpu_percent,
            runtime_seconds,
            output_bytes,
        })
    }
}

impl Default for LinuxSandboxLimits {
    fn default() -> Self {
        Self {
            memory_bytes: 256 * 1024 * 1024,
            task_count: 16,
            cpu_percent: 100,
            runtime_seconds: 15,
            output_bytes: 1024 * 1024,
        }
    }
}

/// Typed operations admitted by the Linux read-only worker boundary.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LinuxSandboxOperation {
    /// Read one exact canonical file already held by the path adapter.
    ReadFile(WorkspacePath),
    /// Enumerate one exact held directory through a bounded isolated projection.
    ReadDirectory(WorkspacePath),
}

/// Bounded result returned by a completed worker.
#[derive(Clone, PartialEq, Eq)]
pub struct LinuxSandboxResult {
    outcome: OperationOutcome,
    stdout: Vec<u8>,
    stdout_sha256: [u8; 32],
    stderr_sha256: [u8; 32],
    stderr_bytes: usize,
}

impl LinuxSandboxResult {
    /// Reports whether the worker exited successfully.
    #[must_use]
    pub const fn success(&self) -> bool {
        matches!(self.outcome, OperationOutcome::Succeeded)
    }

    /// Returns the exact supervisor-observed terminal outcome.
    #[must_use]
    pub const fn outcome(&self) -> OperationOutcome {
        self.outcome
    }

    /// Returns the bounded worker output.
    #[must_use]
    pub fn stdout(&self) -> &[u8] {
        &self.stdout
    }

    /// Returns the digest of worker standard output.
    #[must_use]
    pub const fn stdout_sha256(&self) -> &[u8; 32] {
        &self.stdout_sha256
    }

    /// Returns the digest of supervisor and worker diagnostics.
    #[must_use]
    pub const fn stderr_sha256(&self) -> &[u8; 32] {
        &self.stderr_sha256
    }

    /// Returns the diagnostic byte count without disclosing diagnostic content.
    #[must_use]
    pub const fn stderr_bytes(&self) -> usize {
        self.stderr_bytes
    }
}

impl fmt::Debug for LinuxSandboxResult {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("LinuxSandboxResult")
            .field("outcome", &self.outcome)
            .field("stdout_bytes", &self.stdout.len())
            .field("stderr_bytes", &self.stderr_bytes)
            .finish_non_exhaustive()
    }
}

struct VerifiedArtifact {
    descriptor: OwnedFd,
    guest_path: Option<PathBuf>,
    launch_path: Option<PathBuf>,
    sha256: [u8; 32],
}

struct ProjectionMount<'descriptor> {
    descriptor: &'descriptor OwnedFd,
    guest_path: &'static str,
}

impl fmt::Debug for VerifiedArtifact {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("VerifiedArtifact")
            .field("guest_path", &self.guest_path)
            .field("sha256", &hex_digest(&self.sha256))
            .finish_non_exhaustive()
    }
}

/// Verified, descriptor-held Linux sandbox launch manifest.
pub struct LinuxSandboxManifest {
    systemd_run: VerifiedArtifact,
    systemctl: VerifiedArtifact,
    path_executor: VerifiedArtifact,
    bubblewrap: VerifiedArtifact,
    worker: VerifiedArtifact,
    runtime_files: Vec<VerifiedArtifact>,
}

impl LinuxSandboxManifest {
    /// Verifies and holds every executable and runtime file used by the worker.
    pub fn verify(
        systemd_run: impl AsRef<Path>,
        bubblewrap: impl AsRef<Path>,
        worker: impl AsRef<Path>,
        runtime_files: &[LinuxWorkerRuntimeFile],
    ) -> Result<Self, LinuxSandboxError> {
        if runtime_files.len() > MAX_RUNTIME_FILES {
            return Err(error(LinuxSandboxErrorKind::InvalidManifest));
        }
        let worker_guest_path =
            Path::new(WORKER_GUEST_ROOT).join(verified_worker_name(worker.as_ref())?);
        let systemd_run = verify_artifact(systemd_run.as_ref(), None, true)?;
        let systemctl_path = std::fs::canonicalize(SYSTEMCTL)
            .map_err(|_| error(LinuxSandboxErrorKind::InvalidManifest))?;
        let systemctl = verify_artifact(&systemctl_path, None, true)?;
        let path_executor_path = std::fs::canonicalize(PATH_EXECUTOR)
            .map_err(|_| error(LinuxSandboxErrorKind::InvalidManifest))?;
        let path_executor = verify_artifact(&path_executor_path, None, true)?;
        let bubblewrap = verify_artifact(bubblewrap.as_ref(), None, true)?;
        let worker = verify_artifact(worker.as_ref(), Some(&worker_guest_path), false)?;
        let mut verified_runtime = Vec::with_capacity(runtime_files.len());
        for runtime in runtime_files {
            if !valid_runtime_guest_path(&runtime.guest_path)
                || runtime.guest_path == worker_guest_path
                || verified_runtime.iter().any(|artifact: &VerifiedArtifact| {
                    artifact.guest_path.as_deref() == Some(runtime.guest_path.as_path())
                })
            {
                return Err(error(LinuxSandboxErrorKind::InvalidManifest));
            }
            verified_runtime.push(verify_artifact(
                &runtime.host_path,
                Some(&runtime.guest_path),
                false,
            )?);
        }
        Ok(Self {
            systemd_run,
            systemctl,
            path_executor,
            bubblewrap,
            worker,
            runtime_files: verified_runtime,
        })
    }
}

impl fmt::Debug for LinuxSandboxManifest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("LinuxSandboxManifest")
            .field("systemd_run", &self.systemd_run)
            .field("systemctl", &self.systemctl)
            .field("path_executor", &self.path_executor)
            .field("bubblewrap", &self.bubblewrap)
            .field("worker", &self.worker)
            .field("runtime_files", &self.runtime_files)
            .finish()
    }
}

/// Runs one fresh, offline Bubblewrap worker under a user cgroup.
#[derive(Debug)]
pub struct LinuxSandboxRunner {
    manifest: LinuxSandboxManifest,
    limits: LinuxSandboxLimits,
    seccomp_bpf: Vec<u8>,
}

/// Thread-safe cancellation signal for one bounded worker launch.
#[derive(Clone, Debug, Default)]
pub struct LinuxSandboxCancellation {
    cancelled: Arc<AtomicBool>,
}

impl LinuxSandboxCancellation {
    /// Requests cancellation. The supervisor confirms whole-unit teardown before returning.
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
    }

    fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }
}

/// Opaque sealed request and workspace projection for one read-only tool worker.
pub struct LinuxReadOnlyToolInput {
    held: Vec<LinuxHeldObject>,
    request: OwnedFd,
    snapshot: OwnedFd,
    tool_id: String,
    tool_version: String,
}

impl LinuxReadOnlyToolInput {
    /// Seals one already validated request and an ordered set of exact held objects.
    pub fn seal(
        tool_id: impl Into<String>,
        tool_version: impl Into<String>,
        request: &[u8],
        held: Vec<LinuxHeldObject>,
    ) -> Result<Self, LinuxSandboxError> {
        let tool_id = tool_id.into();
        let tool_version = tool_version.into();
        if !valid_tool_identity(&tool_id)
            || !valid_tool_version(&tool_version)
            || request.is_empty()
            || request.len() > MAX_READ_ONLY_REQUEST_BYTES
            || held.is_empty()
            || held.len() > MAX_READ_ONLY_HELD_OBJECTS
        {
            return Err(error(LinuxSandboxErrorKind::InvalidManifest));
        }
        let first = &held[0];
        let mut paths = std::collections::BTreeSet::new();
        let mut entries = Vec::with_capacity(held.len());
        for object in &held {
            if object.authorization_id() != first.authorization_id()
                || object.adapter_instance_id() != first.adapter_instance_id()
                || object.workspace_path().workspace_id() != first.workspace_path().workspace_id()
                || !paths.insert(object.workspace_path().clone())
            {
                return Err(error(LinuxSandboxErrorKind::TargetMismatch));
            }
            object
                .revalidate()
                .map_err(|_| error(LinuxSandboxErrorKind::StaleObject))?;
            let (kind, bytes) = match object.object_kind() {
                WorkspaceObjectKind::RegularFile => (
                    SnapshotEntryKind::RegularFile,
                    descriptor_bytes(&file_projection(object)?, MAX_READ_ONLY_SNAPSHOT_BYTES)?,
                ),
                WorkspaceObjectKind::Directory => (SnapshotEntryKind::Directory, Vec::new()),
            };
            entries.push(SnapshotEntry {
                path: object
                    .workspace_path()
                    .components()
                    .iter()
                    .map(|component| component.as_str().to_owned())
                    .collect(),
                kind,
                bytes,
                executable: object.object_snapshot.mode & 0o111 != 0,
            });
        }
        entries.sort_by(|left, right| left.path.cmp(&right.path));
        let snapshot_bytes = serde_json::to_vec(&WorkspaceSnapshot { entries })
            .map_err(|_| error(LinuxSandboxErrorKind::FileProjectionFailed))?;
        if snapshot_bytes.is_empty() || snapshot_bytes.len() > MAX_READ_ONLY_SNAPSHOT_BYTES {
            return Err(error(LinuxSandboxErrorKind::OutputLimitExceeded));
        }
        Ok(Self {
            held,
            request: sealed_payload("agentmage-read-only-request", request)?,
            snapshot: sealed_payload("agentmage-read-only-snapshot", &snapshot_bytes)?,
            tool_id,
            tool_version,
        })
    }
}

impl fmt::Debug for LinuxReadOnlyToolInput {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("LinuxReadOnlyToolInput")
            .field("held_count", &self.held.len())
            .field("tool_id", &self.tool_id)
            .field("tool_version", &self.tool_version)
            .finish_non_exhaustive()
    }
}

impl LinuxSandboxRunner {
    /// Compiles the fixed seccomp policy and creates a fail-closed runner.
    pub fn new(
        manifest: LinuxSandboxManifest,
        limits: LinuxSandboxLimits,
    ) -> Result<Self, LinuxSandboxError> {
        let seccomp_bpf = compile_seccomp_policy()?;
        Ok(Self {
            manifest,
            limits,
            seccomp_bpf,
        })
    }

    fn run(
        &self,
        held: &LinuxHeldObject,
        operation: &LinuxSandboxOperation,
        excluded_targets: &[GrantTarget],
    ) -> Result<LinuxSandboxResult, LinuxSandboxError> {
        let path = match operation {
            LinuxSandboxOperation::ReadFile(path) | LinuxSandboxOperation::ReadDirectory(path) => {
                path
            }
        };
        if path.workspace_id() != held.workspace_path().workspace_id() {
            return Err(error(LinuxSandboxErrorKind::WorkspaceMismatch));
        }
        if path != held.workspace_path() {
            return Err(error(LinuxSandboxErrorKind::TargetMismatch));
        }
        held.revalidate()
            .map_err(|_| error(LinuxSandboxErrorKind::StaleObject))?;
        match operation {
            LinuxSandboxOperation::ReadFile(_) => {
                if held.intent() != PathResolutionIntent::ReadFile
                    || held.object_kind() != WorkspaceObjectKind::RegularFile
                {
                    return Err(error(LinuxSandboxErrorKind::TargetMismatch));
                }
                let projection = file_projection(held)?;
                self.run_projection_arguments(
                    &[ProjectionMount {
                        descriptor: &projection,
                        guest_path: "/input/object",
                    }],
                    &[OsString::from("/input/object")],
                )
            }
            LinuxSandboxOperation::ReadDirectory(_) => {
                if held.intent() != PathResolutionIntent::ReadDirectory
                    || held.object_kind() != WorkspaceObjectKind::Directory
                {
                    return Err(error(LinuxSandboxErrorKind::TargetMismatch));
                }
                let projection = directory_projection(held, excluded_targets)?;
                self.run_projection_arguments(
                    &[ProjectionMount {
                        descriptor: &projection,
                        guest_path: "/input/object",
                    }],
                    &[OsString::from("/input/object")],
                )
            }
        }
    }

    fn run_projection_arguments(
        &self,
        projections: &[ProjectionMount<'_>],
        arguments: &[OsString],
    ) -> Result<LinuxSandboxResult, LinuxSandboxError> {
        self.run_projection_arguments_with_cancellation(
            projections,
            arguments,
            &LinuxSandboxCancellation::default(),
        )
    }

    fn run_projection_arguments_with_cancellation(
        &self,
        projections: &[ProjectionMount<'_>],
        arguments: &[OsString],
        cancellation: &LinuxSandboxCancellation,
    ) -> Result<LinuxSandboxResult, LinuxSandboxError> {
        if projections.is_empty()
            || projections.len() > 8
            || projections
                .iter()
                .any(|projection| !valid_input_guest_path(projection.guest_path))
        {
            return Err(error(LinuxSandboxErrorKind::InvalidManifest));
        }
        let parent_pid = std::process::id();
        let descriptor_source =
            |descriptor: &OwnedFd| format!("/proc/{parent_pid}/fd/{}", descriptor.as_raw_fd());
        revalidate_launch_artifact(&self.manifest.systemd_run)?;
        revalidate_launch_artifact(&self.manifest.systemctl)?;
        revalidate_launch_artifact(&self.manifest.path_executor)?;
        revalidate_launch_artifact(&self.manifest.bubblewrap)?;
        let systemd_command = self
            .manifest
            .systemd_run
            .launch_path
            .as_deref()
            .ok_or_else(|| error(LinuxSandboxErrorKind::InvalidManifest))?;
        let systemctl_command = self
            .manifest
            .systemctl
            .launch_path
            .as_deref()
            .ok_or_else(|| error(LinuxSandboxErrorKind::InvalidManifest))?;
        let bubblewrap_command = self
            .manifest
            .bubblewrap
            .launch_path
            .as_deref()
            .ok_or_else(|| error(LinuxSandboxErrorKind::InvalidManifest))?;
        let path_executor_command = self
            .manifest
            .path_executor
            .launch_path
            .as_deref()
            .ok_or_else(|| error(LinuxSandboxErrorKind::InvalidManifest))?;
        let unit = random_unit_name()?;
        let runtime_directory = format!("/run/user/{}", getuid().as_raw());
        let session_bus = format!("unix:path={runtime_directory}/bus");
        let mut command = Command::new(systemd_command);
        command
            .env_clear()
            .env("XDG_RUNTIME_DIR", &runtime_directory)
            .env("DBUS_SESSION_BUS_ADDRESS", &session_bus)
            .arg("--user")
            .arg("--wait")
            .arg("--collect")
            .arg("--quiet")
            .arg("--pipe")
            .arg(format!("--unit={unit}"))
            .arg("--property=RestrictSUIDSGID=yes")
            .arg("--property=LockPersonality=yes")
            .arg("--property=RestrictAddressFamilies=AF_UNIX AF_NETLINK")
            .arg("--property=MemorySwapMax=0")
            .arg(format!("--property=MemoryMax={}", self.limits.memory_bytes))
            .arg(format!("--property=TasksMax={}", self.limits.task_count))
            .arg(format!("--property=CPUQuota={}%", self.limits.cpu_percent))
            .arg(format!(
                "--property=RuntimeMaxSec={}s",
                self.limits.runtime_seconds.saturating_add(1)
            ));
        for (index, projection) in projections.iter().enumerate() {
            command.arg(format!(
                "--property=OpenFile={}:projection-{index}:read-only",
                descriptor_source(projection.descriptor)
            ));
        }
        command.arg(format!(
            "--property=OpenFile={}:worker:read-only",
            descriptor_source(&self.manifest.worker.descriptor)
        ));
        for (index, runtime) in self.manifest.runtime_files.iter().enumerate() {
            command.arg(format!(
                "--property=OpenFile={}:runtime-{index}:read-only",
                descriptor_source(&runtime.descriptor)
            ));
        }
        command
            .arg(path_executor_command)
            .arg("--")
            .arg(bubblewrap_command)
            .args([
                "--unshare-all",
                "--unshare-user",
                "--disable-userns",
                "--new-session",
                "--die-with-parent",
                "--clearenv",
                "--setenv",
                "PATH",
                "/app",
                "--setenv",
                "LANG",
                "C",
                "--cap-drop",
                "ALL",
                "--proc",
                "/proc",
                "--dev",
                "/dev",
                "--size",
                "16777216",
                "--tmpfs",
                "/tmp",
                "--dir",
                "/app",
                "--dir",
                "/input",
            ]);
        for (index, projection) in projections.iter().enumerate() {
            command
                .arg("--ro-bind-data")
                .arg((index + 3).to_string())
                .arg(projection.guest_path);
        }
        let worker_descriptor = projections.len() + 3;
        command
            .arg("--ro-bind-fd")
            .arg(worker_descriptor.to_string());
        command.arg(
            self.manifest
                .worker
                .guest_path
                .as_deref()
                .expect("verified worker guest path"),
        );
        for (index, runtime) in self.manifest.runtime_files.iter().enumerate() {
            command
                .arg("--ro-bind-fd")
                .arg((index + worker_descriptor + 1).to_string())
                .arg(runtime.guest_path.as_deref().expect("verified guest path"));
        }
        command
            .args(["--chdir", "/input", "--seccomp", "0", "--"])
            .arg(
                self.manifest
                    .worker
                    .guest_path
                    .as_deref()
                    .expect("verified worker guest path"),
            )
            .args(arguments)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let mut child = command
            .spawn()
            .map_err(|_| error(LinuxSandboxErrorKind::IsolationUnavailable))?;
        let mut stdin = child
            .stdin
            .take()
            .ok_or_else(|| error(LinuxSandboxErrorKind::IsolationUnavailable))?;
        if stdin.write_all(&self.seccomp_bpf).is_err() {
            let _ = child.kill();
            let _ = child.wait();
            return Err(error(LinuxSandboxErrorKind::IsolationUnavailable));
        }
        drop(stdin);
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| error(LinuxSandboxErrorKind::ExecutionFailed))?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| error(LinuxSandboxErrorKind::ExecutionFailed))?;
        let output_limit = self.limits.output_bytes;
        let stdout_reader = thread::spawn(move || read_bounded(stdout, output_limit));
        let stderr_reader = thread::spawn(move || read_bounded(stderr, output_limit));
        let started = Instant::now();
        let deadline = Duration::from_secs(u64::from(self.limits.runtime_seconds));
        let (forced_outcome, status) = loop {
            if let Some(status) = child
                .try_wait()
                .map_err(|_| error(LinuxSandboxErrorKind::ExecutionFailed))?
            {
                break (None, status);
            }
            let forced_outcome = if cancellation.is_cancelled() {
                Some(OperationOutcome::Cancelled)
            } else if started.elapsed() >= deadline {
                Some(OperationOutcome::TimedOut)
            } else {
                None
            };
            if let Some(outcome) = forced_outcome {
                let stop_status = Command::new(systemctl_command)
                    .env_clear()
                    .env("XDG_RUNTIME_DIR", &runtime_directory)
                    .env("DBUS_SESSION_BUS_ADDRESS", &session_bus)
                    .args(["--user", "stop", unit.as_str()])
                    .stdin(Stdio::null())
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .status()
                    .map_err(|_| error(LinuxSandboxErrorKind::ExecutionFailed))?;
                if !stop_status.success() {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(error(LinuxSandboxErrorKind::ExecutionFailed));
                }
                let status = child
                    .wait()
                    .map_err(|_| error(LinuxSandboxErrorKind::ExecutionFailed))?;
                break (Some(outcome), status);
            }
            thread::sleep(Duration::from_millis(10));
        };
        let stdout = stdout_reader
            .join()
            .map_err(|_| error(LinuxSandboxErrorKind::ExecutionFailed))??;
        let stderr = stderr_reader
            .join()
            .map_err(|_| error(LinuxSandboxErrorKind::ExecutionFailed))??;
        if stdout.exceeded || stderr.exceeded {
            return Err(error(LinuxSandboxErrorKind::OutputLimitExceeded));
        }
        Ok(LinuxSandboxResult {
            outcome: forced_outcome.unwrap_or(if status.success() {
                OperationOutcome::Succeeded
            } else {
                OperationOutcome::Failed
            }),
            stdout_sha256: digest_bytes(&stdout.retained),
            stderr_sha256: stderr.digest,
            stderr_bytes: stderr.total,
            stdout: stdout.retained,
        })
    }

    fn run_read_only_tool_with_cancellation(
        &self,
        input: &LinuxReadOnlyToolInput,
        cancellation: &LinuxSandboxCancellation,
    ) -> Result<LinuxSandboxResult, LinuxSandboxError> {
        for object in &input.held {
            object
                .revalidate()
                .map_err(|_| error(LinuxSandboxErrorKind::StaleObject))?;
        }
        self.run_projection_arguments_with_cancellation(
            &[
                ProjectionMount {
                    descriptor: &input.request,
                    guest_path: "/input/request",
                },
                ProjectionMount {
                    descriptor: &input.snapshot,
                    guest_path: "/input/snapshot",
                },
            ],
            &[
                OsString::from(&input.tool_id),
                OsString::from(&input.tool_version),
            ],
            cancellation,
        )
    }

    /// Returns the fixed seccomp policy identity used by every worker.
    #[must_use]
    pub const fn seccomp_policy_id(&self) -> &'static str {
        SECCOMP_POLICY_ID
    }
}

/// Linux sandbox effect callable only with a kernel-issued authorization.
///
/// The underlying runner has no public execution method:
///
/// ```compile_fail
/// use agentmage_platform_linux::{
///     LinuxHeldObject, LinuxSandboxOperation, LinuxSandboxRunner,
/// };
/// fn bypass(
///     runner: &LinuxSandboxRunner,
///     held: &LinuxHeldObject,
///     operation: &LinuxSandboxOperation,
/// ) {
///     let _ = runner.run(held, operation, &[]);
/// }
/// ```
pub struct LinuxSandboxEffectDriver {
    runner: LinuxSandboxRunner,
    held: LinuxHeldObject,
    operation: LinuxSandboxOperation,
    result: Option<LinuxSandboxResult>,
    error: Option<LinuxSandboxError>,
}

impl LinuxSandboxEffectDriver {
    /// Creates an inert driver over one continuously held exact object.
    #[must_use]
    pub const fn new(
        runner: LinuxSandboxRunner,
        held: LinuxHeldObject,
        operation: LinuxSandboxOperation,
    ) -> Self {
        Self {
            runner,
            held,
            operation,
            result: None,
            error: None,
        }
    }

    /// Takes the bounded worker result after the authority transaction closes.
    pub fn take_result(&mut self) -> Option<LinuxSandboxResult> {
        self.result.take()
    }

    /// Takes the redacted platform error after a failed mediated attempt.
    pub fn take_error(&mut self) -> Option<LinuxSandboxError> {
        self.error.take()
    }

    /// Returns the still-verified runner after this one driver attempt closes.
    ///
    /// The runner exposes no public launch method, so recovering it does not
    /// bypass the kernel effect permit required by the next driver.
    #[must_use]
    pub fn into_runner(self) -> LinuxSandboxRunner {
        self.runner
    }
}

impl fmt::Debug for LinuxSandboxEffectDriver {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("LinuxSandboxEffectDriver")
            .field("operation", &self.operation)
            .field("has_result", &self.result.is_some())
            .field("has_error", &self.error.is_some())
            .finish_non_exhaustive()
    }
}

impl EffectDriver for LinuxSandboxEffectDriver {
    fn execute(&mut self, authorization: EffectAuthorization<'_>) -> EffectLaunch {
        if authorization.operation().operation() != GrantOperation::WorkspaceRead
            || !authorization.authorizes_held_object(&self.held)
        {
            return EffectLaunch::failed();
        }
        match self.runner.run(
            &self.held,
            &self.operation,
            authorization.excluded_targets(),
        ) {
            Ok(result) => {
                let effect_result = EffectResult::from_redacted_material(
                    result.outcome(),
                    result.stdout_sha256(),
                    StateChange::NotChanged,
                );
                self.result = Some(result);
                EffectLaunch::completed(effect_result)
            }
            Err(error) => {
                let effect_result = EffectResult::from_redacted_material(
                    OperationOutcome::Failed,
                    error.kind().code().as_bytes(),
                    StateChange::NotChanged,
                );
                self.error = Some(error);
                EffectLaunch::completed(effect_result)
            }
        }
    }
}

/// Linux read-only tool worker callable only with one consumed exact grant.
pub struct LinuxReadOnlyToolEffectDriver {
    runner: LinuxSandboxRunner,
    input: LinuxReadOnlyToolInput,
    result: Option<LinuxSandboxResult>,
    error: Option<LinuxSandboxError>,
    cancellation: LinuxSandboxCancellation,
}

impl LinuxReadOnlyToolEffectDriver {
    /// Creates an inert driver over sealed input and continuously held objects.
    #[must_use]
    pub fn new(runner: LinuxSandboxRunner, input: LinuxReadOnlyToolInput) -> Self {
        Self {
            runner,
            input,
            result: None,
            error: None,
            cancellation: LinuxSandboxCancellation::default(),
        }
    }

    /// Creates an inert driver bound to an externally observable cancellation signal.
    #[must_use]
    pub const fn new_cancellable(
        runner: LinuxSandboxRunner,
        input: LinuxReadOnlyToolInput,
        cancellation: LinuxSandboxCancellation,
    ) -> Self {
        Self {
            runner,
            input,
            result: None,
            error: None,
            cancellation,
        }
    }

    /// Takes the bounded worker result after the authority transaction closes.
    pub fn take_result(&mut self) -> Option<LinuxSandboxResult> {
        self.result.take()
    }

    /// Takes the redacted platform error after a failed mediated attempt.
    pub fn take_error(&mut self) -> Option<LinuxSandboxError> {
        self.error.take()
    }

    /// Returns the verified runner after this attempt closes.
    #[must_use]
    pub fn into_runner(self) -> LinuxSandboxRunner {
        self.runner
    }
}

impl fmt::Debug for LinuxReadOnlyToolEffectDriver {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("LinuxReadOnlyToolEffectDriver")
            .field("input", &self.input)
            .field("has_result", &self.result.is_some())
            .field("has_error", &self.error.is_some())
            .finish_non_exhaustive()
    }
}

impl EffectDriver for LinuxReadOnlyToolEffectDriver {
    fn execute(&mut self, authorization: EffectAuthorization<'_>) -> EffectLaunch {
        if authorization.operation().operation() != GrantOperation::WorkspaceRead
            || authorization.call().tool_id.as_str() != self.input.tool_id
            || authorization.call().tool_version != self.input.tool_version
            || !authorization.authorizes_held_objects(&self.input.held)
        {
            return EffectLaunch::failed();
        }
        match self
            .runner
            .run_read_only_tool_with_cancellation(&self.input, &self.cancellation)
        {
            Ok(result) => {
                let effect = EffectResult::from_redacted_material(
                    result.outcome(),
                    result.stdout_sha256(),
                    StateChange::NotChanged,
                );
                self.result = Some(result);
                EffectLaunch::completed(effect)
            }
            Err(error) => {
                let effect = EffectResult::from_redacted_material(
                    OperationOutcome::Failed,
                    error.kind().code().as_bytes(),
                    StateChange::NotChanged,
                );
                self.error = Some(error);
                EffectLaunch::completed(effect)
            }
        }
    }
}

struct BoundedRead {
    retained: Vec<u8>,
    digest: [u8; 32],
    total: usize,
    exceeded: bool,
}

fn read_bounded(mut input: impl Read, limit: usize) -> Result<BoundedRead, LinuxSandboxError> {
    let mut retained = Vec::with_capacity(limit.min(HASH_BUFFER_BYTES));
    let mut digest = Sha256::new();
    let mut total = 0_usize;
    let mut buffer = [0_u8; HASH_BUFFER_BYTES];
    loop {
        let count = input
            .read(&mut buffer)
            .map_err(|_| error(LinuxSandboxErrorKind::ExecutionFailed))?;
        if count == 0 {
            break;
        }
        digest.update(&buffer[..count]);
        total = total.saturating_add(count);
        if retained.len() < limit {
            let remaining = limit - retained.len();
            retained.extend_from_slice(&buffer[..count.min(remaining)]);
        }
    }
    Ok(BoundedRead {
        retained,
        digest: digest.finalize().into(),
        total,
        exceeded: total > limit,
    })
}

pub(crate) fn compile_seccomp_policy() -> Result<Vec<u8>, LinuxSandboxError> {
    let rules = DENIED_SYSCALLS
        .iter()
        .map(|syscall| format!(r#"{{"syscall":"{syscall}"}}"#))
        .collect::<Vec<_>>()
        .join(",");
    let policy = format!(
        r#"{{"worker":{{"mismatch_action":"allow","match_action":{{"errno":1}},"filter":[{rules}]}}}}"#
    );
    let arch: TargetArch = std::env::consts::ARCH
        .try_into()
        .map_err(|_| error(LinuxSandboxErrorKind::SeccompUnavailable))?;
    let mut filters = compile_from_json(policy.as_bytes(), arch)
        .map_err(|_| error(LinuxSandboxErrorKind::SeccompUnavailable))?;
    let program = filters
        .remove("worker")
        .ok_or_else(|| error(LinuxSandboxErrorKind::SeccompUnavailable))?;
    serialize_bpf(&program)
}

fn serialize_bpf(program: &BpfProgram) -> Result<Vec<u8>, LinuxSandboxError> {
    if program.is_empty() || program.len() > u16::MAX.into() {
        return Err(error(LinuxSandboxErrorKind::SeccompUnavailable));
    }
    let mut bytes = Vec::with_capacity(program.len() * 8);
    for instruction in program {
        bytes.extend_from_slice(&instruction.code.to_ne_bytes());
        bytes.push(instruction.jt);
        bytes.push(instruction.jf);
        bytes.extend_from_slice(&instruction.k.to_ne_bytes());
    }
    Ok(bytes)
}

fn verify_artifact(
    path: &Path,
    guest_path: Option<&Path>,
    launch_by_path: bool,
) -> Result<VerifiedArtifact, LinuxSandboxError> {
    if !path.is_absolute() || guest_path.is_some_and(|guest| !valid_guest_path(guest)) {
        return Err(error(LinuxSandboxErrorKind::InvalidManifest));
    }
    verify_root_owned_path(path)?;
    let descriptor = open(
        path,
        OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::CLOEXEC | OFlags::NONBLOCK,
        Mode::empty(),
    )
    .map_err(|_| error(LinuxSandboxErrorKind::InvalidManifest))?;
    let stat = fstat(&descriptor).map_err(|_| error(LinuxSandboxErrorKind::InvalidManifest))?;
    let executable_guest = guest_path
        .and_then(Path::parent)
        .is_some_and(|parent| parent == Path::new(WORKER_GUEST_ROOT));
    if FileType::from_raw_mode(stat.st_mode) != FileType::RegularFile
        || stat.st_uid != 0
        || stat.st_mode & 0o022 != 0
        || ((launch_by_path || executable_guest) && stat.st_mode & 0o111 == 0)
    {
        return Err(error(LinuxSandboxErrorKind::InvalidManifest));
    }
    let sha256 = hash_descriptor(&descriptor, stat.st_size)?;
    Ok(VerifiedArtifact {
        descriptor,
        guest_path: guest_path.map(Path::to_path_buf),
        launch_path: launch_by_path.then(|| path.to_path_buf()),
        sha256,
    })
}

fn verified_worker_name(path: &Path) -> Result<OsString, LinuxSandboxError> {
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| {
            !name.is_empty()
                && name.len() <= 64
                && !name.starts_with('-')
                && name
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
        })
        .ok_or_else(|| error(LinuxSandboxErrorKind::InvalidManifest))?;
    Ok(OsString::from(name))
}

fn verify_root_owned_path(path: &Path) -> Result<(), LinuxSandboxError> {
    let mut current = PathBuf::from("/");
    for component in path.components().skip(1) {
        let Component::Normal(component) = component else {
            return Err(error(LinuxSandboxErrorKind::InvalidManifest));
        };
        current.push(component);
        let metadata = std::fs::symlink_metadata(&current)
            .map_err(|_| error(LinuxSandboxErrorKind::InvalidManifest))?;
        use std::os::unix::fs::MetadataExt as _;
        if metadata.uid() != 0 || metadata.mode() & 0o022 != 0 || metadata.file_type().is_symlink()
        {
            return Err(error(LinuxSandboxErrorKind::InvalidManifest));
        }
    }
    Ok(())
}

fn revalidate_launch_artifact(artifact: &VerifiedArtifact) -> Result<(), LinuxSandboxError> {
    let path = artifact
        .launch_path
        .as_deref()
        .ok_or_else(|| error(LinuxSandboxErrorKind::InvalidManifest))?;
    let current = verify_artifact(path, None, true)?;
    if current.sha256 != artifact.sha256 {
        return Err(error(LinuxSandboxErrorKind::InvalidManifest));
    }
    Ok(())
}

fn hash_descriptor(descriptor: &OwnedFd, size: i64) -> Result<[u8; 32], LinuxSandboxError> {
    let size = u64::try_from(size).map_err(|_| error(LinuxSandboxErrorKind::InvalidManifest))?;
    if size > 256 * 1024 * 1024 {
        return Err(error(LinuxSandboxErrorKind::InvalidManifest));
    }
    let mut digest = Sha256::new();
    let mut buffer = [0_u8; HASH_BUFFER_BYTES];
    let mut offset = 0_u64;
    while offset < size {
        let requested = usize::try_from((size - offset).min(HASH_BUFFER_BYTES as u64))
            .map_err(|_| error(LinuxSandboxErrorKind::InvalidManifest))?;
        let count = pread(descriptor, &mut buffer[..requested], offset)
            .map_err(|_| error(LinuxSandboxErrorKind::InvalidManifest))?;
        if count == 0 {
            return Err(error(LinuxSandboxErrorKind::InvalidManifest));
        }
        digest.update(&buffer[..count]);
        offset = offset
            .checked_add(count as u64)
            .ok_or_else(|| error(LinuxSandboxErrorKind::InvalidManifest))?;
    }
    Ok(digest.finalize().into())
}

fn valid_guest_path(path: &Path) -> bool {
    path.is_absolute()
        && path.components().next() == Some(Component::RootDir)
        && path
            .components()
            .skip(1)
            .all(|component| matches!(component, Component::Normal(_)))
}

fn valid_runtime_guest_path(path: &Path) -> bool {
    if !valid_guest_path(path) {
        return false;
    }
    let components = path
        .components()
        .filter_map(|component| match component {
            Component::Normal(value) => value.to_str(),
            _ => None,
        })
        .collect::<Vec<_>>();
    matches!(
        components.as_slice(),
        ["lib" | "lib64", _, ..] | ["usr", "lib" | "lib64", _, ..]
    )
}

fn valid_input_guest_path(path: &str) -> bool {
    matches!(path, "/input/object" | "/input/request" | "/input/snapshot")
}

fn file_projection(held: &LinuxHeldObject) -> Result<OwnedFd, LinuxSandboxError> {
    let expected = held
        .preimage()
        .ok_or_else(|| error(LinuxSandboxErrorKind::TargetMismatch))?;
    let descriptor = projection_descriptor(
        "agentmage-file-projection",
        LinuxSandboxErrorKind::FileProjectionFailed,
    )?;
    let mut digest = Sha256::new();
    let mut buffer = [0_u8; HASH_BUFFER_BYTES];
    let mut offset = 0_u64;
    while offset < expected.byte_len() {
        let requested =
            usize::try_from((expected.byte_len() - offset).min(HASH_BUFFER_BYTES as u64))
                .map_err(|_| error(LinuxSandboxErrorKind::FileProjectionFailed))?;
        let count = pread(&held.object_descriptor, &mut buffer[..requested], offset)
            .map_err(|_| error(LinuxSandboxErrorKind::FileProjectionFailed))?;
        if count == 0 {
            return Err(error(LinuxSandboxErrorKind::StaleObject));
        }
        write_all_projection(
            &descriptor,
            &buffer[..count],
            LinuxSandboxErrorKind::FileProjectionFailed,
        )?;
        digest.update(&buffer[..count]);
        offset = offset
            .checked_add(count as u64)
            .ok_or_else(|| error(LinuxSandboxErrorKind::FileProjectionFailed))?;
    }
    let observed: [u8; 32] = digest.finalize().into();
    if &observed != expected.content_sha256() {
        return Err(error(LinuxSandboxErrorKind::StaleObject));
    }
    held.revalidate()
        .map_err(|_| error(LinuxSandboxErrorKind::StaleObject))?;
    seal_projection(&descriptor, LinuxSandboxErrorKind::FileProjectionFailed)?;
    Ok(descriptor)
}

fn directory_projection(
    held: &LinuxHeldObject,
    excluded_targets: &[GrantTarget],
) -> Result<OwnedFd, LinuxSandboxError> {
    let held_path = held.workspace_path();
    let held_components = held_path.components();
    let mut excluded_children = Vec::new();
    for excluded in excluded_targets {
        if excluded.authorization_id() != held.authorization_id()
            || excluded.adapter_instance_id() != held.adapter_instance_id()
            || excluded.platform() != held.object_identity().platform()
        {
            return Err(error(LinuxSandboxErrorKind::TargetMismatch));
        }
        let Some(scope) = excluded.scope_path() else {
            return Err(error(LinuxSandboxErrorKind::TargetMismatch));
        };
        if scope.workspace_id() != held_path.workspace_id() {
            return Err(error(LinuxSandboxErrorKind::WorkspaceMismatch));
        }
        let scope_components = scope.components();
        if held_components.starts_with(scope_components) {
            return Err(error(LinuxSandboxErrorKind::TargetMismatch));
        }
        if scope_components.starts_with(held_components)
            && let Some(component) = scope_components.get(held_components.len())
        {
            excluded_children.push(component.as_str().as_bytes().to_vec());
        }
    }

    let mut entries = Vec::new();
    let directory = Dir::read_from(&held.object_descriptor)
        .map_err(|_| error(LinuxSandboxErrorKind::DirectoryProjectionFailed))?;
    for entry in directory {
        let entry = entry.map_err(|_| error(LinuxSandboxErrorKind::DirectoryProjectionFailed))?;
        let name = entry.file_name().to_bytes();
        if matches!(name, b"." | b"..") || excluded_children.iter().any(|item| item == name) {
            continue;
        }
        if entries.len() >= MAX_DIRECTORY_ENTRIES {
            return Err(error(LinuxSandboxErrorKind::DirectoryProjectionFailed));
        }
        entries.push(name.to_vec());
    }
    entries.sort_unstable();

    let projected_size = entries
        .iter()
        .try_fold(0_usize, |total, entry| total.checked_add(entry.len() + 1))
        .filter(|total| *total <= MAX_DIRECTORY_PROJECTION_BYTES)
        .ok_or_else(|| error(LinuxSandboxErrorKind::DirectoryProjectionFailed))?;
    let mut projection = Vec::with_capacity(projected_size);
    for entry in entries {
        projection.extend_from_slice(&entry);
        projection.push(0);
    }

    held.revalidate()
        .map_err(|_| error(LinuxSandboxErrorKind::StaleObject))?;

    let descriptor = projection_descriptor(
        "agentmage-directory-projection",
        LinuxSandboxErrorKind::DirectoryProjectionFailed,
    )?;
    write_all_projection(
        &descriptor,
        &projection,
        LinuxSandboxErrorKind::DirectoryProjectionFailed,
    )?;
    seal_projection(
        &descriptor,
        LinuxSandboxErrorKind::DirectoryProjectionFailed,
    )?;
    Ok(descriptor)
}

fn projection_descriptor(
    name: &str,
    error_kind: LinuxSandboxErrorKind,
) -> Result<OwnedFd, LinuxSandboxError> {
    memfd_create(name, MemfdFlags::CLOEXEC | MemfdFlags::ALLOW_SEALING)
        .map_err(|_| error(error_kind))
}

fn write_all_projection(
    descriptor: &OwnedFd,
    bytes: &[u8],
    error_kind: LinuxSandboxErrorKind,
) -> Result<(), LinuxSandboxError> {
    let mut remaining = bytes;
    while !remaining.is_empty() {
        let count = write(descriptor, remaining).map_err(|_| error(error_kind))?;
        if count == 0 {
            return Err(error(error_kind));
        }
        remaining = &remaining[count..];
    }
    Ok(())
}

fn seal_projection(
    descriptor: &OwnedFd,
    error_kind: LinuxSandboxErrorKind,
) -> Result<(), LinuxSandboxError> {
    seek(descriptor, SeekFrom::Start(0)).map_err(|_| error(error_kind))?;
    fcntl_add_seals(
        descriptor,
        SealFlags::SEAL | SealFlags::SHRINK | SealFlags::GROW | SealFlags::WRITE,
    )
    .map_err(|_| error(error_kind))
}

fn sealed_payload(name: &str, bytes: &[u8]) -> Result<OwnedFd, LinuxSandboxError> {
    let descriptor = projection_descriptor(name, LinuxSandboxErrorKind::FileProjectionFailed)?;
    write_all_projection(
        &descriptor,
        bytes,
        LinuxSandboxErrorKind::FileProjectionFailed,
    )?;
    seal_projection(&descriptor, LinuxSandboxErrorKind::FileProjectionFailed)?;
    Ok(descriptor)
}

fn descriptor_bytes(descriptor: &OwnedFd, maximum: usize) -> Result<Vec<u8>, LinuxSandboxError> {
    let stat = fstat(descriptor).map_err(|_| error(LinuxSandboxErrorKind::FileProjectionFailed))?;
    let size = usize::try_from(stat.st_size)
        .ok()
        .filter(|size| *size <= maximum)
        .ok_or_else(|| error(LinuxSandboxErrorKind::OutputLimitExceeded))?;
    let mut bytes = vec![0_u8; size];
    let mut offset = 0_usize;
    while offset < size {
        let count = pread(descriptor, &mut bytes[offset..], offset as u64)
            .map_err(|_| error(LinuxSandboxErrorKind::FileProjectionFailed))?;
        if count == 0 {
            return Err(error(LinuxSandboxErrorKind::FileProjectionFailed));
        }
        offset += count;
    }
    Ok(bytes)
}

fn valid_tool_identity(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b':' | b'-'))
}

fn valid_tool_version(value: &str) -> bool {
    let parts = value.split('.').collect::<Vec<_>>();
    value.len() <= 64
        && parts.len() == 3
        && parts
            .iter()
            .all(|part| !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit()))
}

fn random_unit_name() -> Result<String, LinuxSandboxError> {
    let mut random = [0_u8; 12];
    getrandom(&mut random, GetRandomFlags::empty())
        .map_err(|_| error(LinuxSandboxErrorKind::IsolationUnavailable))?;
    Ok(format!("agentmage-worker-{}", hex_digest(&random)))
}

fn digest_bytes(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}

fn hex_digest(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        use fmt::Write as _;
        let _ = write!(output, "{byte:02x}");
    }
    output
}

const fn error(kind: LinuxSandboxErrorKind) -> LinuxSandboxError {
    LinuxSandboxError { kind }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;
    use std::ffi::OsString;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::process::Command;
    use std::thread;
    use std::time::{Duration, Instant};

    use agentmage_kernel_contracts::{
        AdapterInstanceId, GrantTarget, HeldWorkspaceObject, OperationOutcome,
        PathResolutionIntent, PlatformPathAdapter, WorkspaceAuthorizationId, WorkspaceId,
        WorkspacePath, WorkspaceScopePath, WorkspaceSnapshot,
    };
    use rustix::fs::{SealFlags, fcntl_get_seals};
    use rustix::io::{pread, write};

    use super::{
        LinuxReadOnlyToolInput, LinuxSandboxCancellation, LinuxSandboxError, LinuxSandboxErrorKind,
        LinuxSandboxLimits, LinuxSandboxManifest, LinuxSandboxOperation, LinuxSandboxResult,
        LinuxSandboxRunner, LinuxWorkerRuntimeFile, MAX_READ_ONLY_REQUEST_BYTES,
        compile_seccomp_policy, directory_projection, file_projection, verified_worker_name,
    };
    use crate::{
        DEFAULT_MAX_PREIMAGE_BYTES, LinuxAuthorizedWorkspace, LinuxHeldObject, LinuxPathAdapter,
        authorize_workspace_root,
    };

    fn temp_directory(label: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "agentmage-sandbox-{label}-{}-{}",
            std::process::id(),
            super::random_unit_name().expect("random suffix")
        ));
        fs::create_dir(&root).expect("test root");
        root
    }

    fn authorize(root: &Path) -> LinuxAuthorizedWorkspace {
        authorize_workspace_root(
            root,
            WorkspaceId::from_raw("workspace-sandbox"),
            WorkspaceAuthorizationId::from_raw("authorization-sandbox"),
            AdapterInstanceId::from_raw("adapter-linux"),
        )
        .expect("workspace authorization")
    }

    fn hold(
        workspace: &LinuxAuthorizedWorkspace,
        name: &str,
        intent: PathResolutionIntent,
    ) -> LinuxHeldObject {
        let path = WorkspacePath::new(workspace.workspace_id.clone(), [name])
            .expect("canonical test path");
        LinuxPathAdapter::new(
            workspace.adapter_instance_id.clone(),
            DEFAULT_MAX_PREIMAGE_BYTES,
        )
        .resolve(workspace, &path, intent)
        .expect("held test object")
    }

    fn exclusion(workspace: &LinuxAuthorizedWorkspace, components: &[&str]) -> GrantTarget {
        GrantTarget::workspace_scope(
            workspace,
            WorkspaceScopePath::new(workspace.workspace_id.clone(), components.iter().copied())
                .expect("canonical exclusion"),
        )
        .expect("bound exclusion")
    }

    fn descriptor_bytes(descriptor: &rustix::fd::OwnedFd) -> Vec<u8> {
        let mut output = Vec::new();
        let mut offset = 0_u64;
        let mut buffer = [0_u8; 1024];
        loop {
            let count = pread(descriptor, &mut buffer, offset).expect("projection read");
            if count == 0 {
                break;
            }
            output.extend_from_slice(&buffer[..count]);
            offset += count as u64;
        }
        output
    }

    fn runtime_files(executable: &Path) -> Vec<LinuxWorkerRuntimeFile> {
        let output = Command::new("/usr/bin/ldd")
            .arg(executable)
            .output()
            .expect("ldd available");
        assert!(output.status.success());
        String::from_utf8(output.stdout)
            .expect("ldd UTF-8")
            .lines()
            .filter_map(|line| {
                let candidate = line.split_once("=>").map_or_else(
                    || line.split_whitespace().next(),
                    |(_, right)| right.split_whitespace().next(),
                )?;
                if !candidate.starts_with('/') {
                    return None;
                }
                let canonical = fs::canonicalize(candidate).expect("canonical runtime");
                Some(LinuxWorkerRuntimeFile::new(canonical, candidate))
            })
            .collect()
    }

    fn runner_for(executable: &str, limits: LinuxSandboxLimits) -> LinuxSandboxRunner {
        let executable = fs::canonicalize(executable).expect("canonical worker executable");
        let systemd_run =
            fs::canonicalize("/usr/bin/systemd-run").expect("canonical systemd-run executable");
        let bubblewrap =
            fs::canonicalize("/usr/bin/bwrap").expect("canonical Bubblewrap executable");
        let manifest = LinuxSandboxManifest::verify(
            systemd_run,
            bubblewrap,
            &executable,
            &runtime_files(&executable),
        )
        .expect("verified host manifest");
        LinuxSandboxRunner::new(manifest, limits).expect("sandbox runner")
    }

    fn runner() -> LinuxSandboxRunner {
        runner_for("/usr/bin/cat", LinuxSandboxLimits::default())
    }

    fn lifecycle_fixture(case: &str) -> PathBuf {
        let root = std::env::var_os("AGENTMAGE_LIFECYCLE_FIXTURE_ROOT")
            .map(PathBuf::from)
            .expect("lifecycle fixture root");
        root.join(format!("agentmage-lifecycle-{case}"))
    }

    fn lifecycle_units() -> BTreeSet<String> {
        let output = Command::new("/usr/bin/systemctl")
            .args([
                "--user",
                "list-units",
                "--all",
                "--plain",
                "--no-legend",
                "agentmage-worker-*",
            ])
            .output()
            .expect("systemctl unit inventory");
        assert!(output.status.success());
        String::from_utf8(output.stdout)
            .expect("unit inventory UTF-8")
            .lines()
            .filter_map(|line| line.split_whitespace().next().map(str::to_owned))
            .collect()
    }

    fn lifecycle_processes(name: &str) -> Vec<u32> {
        let mut processes = fs::read_dir("/proc")
            .expect("proc inventory")
            .filter_map(Result::ok)
            .filter_map(|entry| entry.file_name().to_string_lossy().parse::<u32>().ok())
            .filter(|pid| {
                fs::read(format!("/proc/{pid}/cmdline")).is_ok_and(|command| {
                    command
                        .windows(name.len())
                        .any(|part| part == name.as_bytes())
                })
            })
            .collect::<Vec<_>>();
        processes.sort_unstable();
        processes
    }

    fn lifecycle_scratch_visible(name: &str) -> bool {
        lifecycle_processes(name).into_iter().any(|pid| {
            PathBuf::from(format!("/proc/{pid}/root/tmp/agentmage-lifecycle-scratch")).is_file()
        })
    }

    fn wait_for_lifecycle_cleanup(name: &str, baseline_units: &BTreeSet<String>) {
        let deadline = Instant::now() + Duration::from_secs(5);
        while Instant::now() < deadline {
            if lifecycle_processes(name).is_empty() && lifecycle_units() == *baseline_units {
                return;
            }
            thread::sleep(Duration::from_millis(25));
        }
        panic!("lifecycle process or unit residue remains for {name}");
    }

    fn lifecycle_input(workspace: &LinuxAuthorizedWorkspace) -> LinuxReadOnlyToolInput {
        let request = br#"{"schema_version":1,"paths":[["allowed.txt"]],"query":null,"byte_offset":null,"byte_count":null,"encoding":"utf8","limits":{"files":128,"input_bytes":4194304,"depth":16,"matches":256,"output_bytes":1048576},"call_depth":0}"#;
        LinuxReadOnlyToolInput::seal(
            "agentmage.workspace.read-file",
            "1.0.0",
            request,
            vec![hold(
                workspace,
                "allowed.txt",
                PathResolutionIntent::ReadFile,
            )],
        )
        .expect("lifecycle input seals")
    }

    fn run_held_arguments(
        runner: &LinuxSandboxRunner,
        held: &LinuxHeldObject,
        arguments: &[OsString],
    ) -> Result<LinuxSandboxResult, LinuxSandboxError> {
        let projection = file_projection(held)?;
        runner.run_projection_arguments(
            &[super::ProjectionMount {
                descriptor: &projection,
                guest_path: "/input/object",
            }],
            arguments,
        )
    }

    #[test]
    fn policy_compiles_to_nonempty_classic_bpf() {
        let policy = compile_seccomp_policy().expect("compiled policy");
        assert!(!policy.is_empty());
        assert_eq!(policy.len() % 8, 0);
    }

    #[test]
    fn policy_denies_every_worker_socket_and_connection_syscall() {
        for syscall in [
            "accept",
            "accept4",
            "bind",
            "connect",
            "listen",
            "recvfrom",
            "recvmmsg",
            "recvmsg",
            "sendmmsg",
            "sendmsg",
            "sendto",
            "shutdown",
            "socket",
            "socketpair",
        ] {
            assert!(
                super::DENIED_SYSCALLS.contains(&syscall),
                "worker syscall policy admitted {syscall}"
            );
        }
    }

    #[test]
    fn limits_and_manifests_fail_closed() {
        assert_eq!(
            LinuxSandboxLimits::new(1, 1, 1, 1, 1)
                .expect_err("tiny memory must fail")
                .kind(),
            LinuxSandboxErrorKind::InvalidLimit
        );
        let invalid_runtime = [LinuxWorkerRuntimeFile::new(
            "/usr/lib64/libc.so.6",
            "/workspace/libc.so.6",
        )];
        assert_eq!(
            LinuxSandboxManifest::verify(
                "/usr/bin/systemd-run",
                "/usr/bin/bwrap",
                "/usr/bin/cat",
                &invalid_runtime
            )
            .expect_err("runtime destination must fail")
            .kind(),
            LinuxSandboxErrorKind::InvalidManifest
        );
        assert_eq!(
            LinuxSandboxManifest::verify(
                "/missing/systemd-run",
                "/missing/bwrap",
                "/usr/bin/cat",
                &[]
            )
            .expect_err("missing launch artifacts must fail")
            .kind(),
            LinuxSandboxErrorKind::InvalidManifest
        );
        assert_eq!(
            verified_worker_name(Path::new("/usr/lib/coreutils/cat")).expect("bounded applet name"),
            OsString::from("cat")
        );
        assert!(verified_worker_name(Path::new("/usr/bin/-worker")).is_err());
        assert!(verified_worker_name(Path::new("/usr/bin/worker name")).is_err());
    }

    #[test]
    fn directory_projection_never_contains_excluded_or_nested_workspace_content() {
        let root = temp_directory("projection");
        fs::create_dir(root.join("folder")).expect("held directory");
        fs::write(root.join("folder").join("allowed.txt"), b"allowed").expect("allowed child");
        fs::write(root.join("folder").join("secret.txt"), b"secret").expect("secret child");
        let workspace = authorize(&root);
        let held = hold(&workspace, "folder", PathResolutionIntent::ReadDirectory);
        let excluded = exclusion(&workspace, &["folder", "secret.txt"]);
        let projection = directory_projection(&held, &[excluded]).expect("bounded projection");
        assert_eq!(descriptor_bytes(&projection), b"allowed.txt\0");
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn file_projection_is_the_approved_preimage_and_cannot_be_modified() {
        let root = temp_directory("file-projection");
        fs::write(root.join("allowed.txt"), b"approved bytes").expect("fixture");
        let workspace = authorize(&root);
        let held = hold(&workspace, "allowed.txt", PathResolutionIntent::ReadFile);
        let projection = file_projection(&held).expect("immutable file projection");
        assert_eq!(descriptor_bytes(&projection), b"approved bytes");
        assert_eq!(
            fcntl_get_seals(&projection).expect("projection seals"),
            SealFlags::SEAL | SealFlags::SHRINK | SealFlags::GROW | SealFlags::WRITE
        );
        assert!(write(&projection, b"changed").is_err());
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn read_only_tool_input_seals_exact_deterministic_snapshot_and_request() {
        let root = temp_directory("read-only-input");
        fs::write(root.join("zeta.txt"), b"last").expect("zeta fixture");
        fs::write(root.join("alpha.txt"), b"first").expect("alpha fixture");
        fs::create_dir(root.join("folder")).expect("directory fixture");
        let workspace = authorize(&root);
        let request = br#"{"arguments":{"path":"alpha.txt"},"call_id":"call-1"}"#;
        let input = LinuxReadOnlyToolInput::seal(
            "agentmage.workspace.read-file",
            "1.0.0",
            request,
            vec![
                hold(&workspace, "zeta.txt", PathResolutionIntent::ReadFile),
                hold(&workspace, "folder", PathResolutionIntent::ReadDirectory),
                hold(&workspace, "alpha.txt", PathResolutionIntent::ReadFile),
            ],
        )
        .expect("sealed input");

        assert_eq!(descriptor_bytes(&input.request), request);
        let snapshot: WorkspaceSnapshot =
            serde_json::from_slice(&descriptor_bytes(&input.snapshot)).expect("snapshot JSON");
        assert_eq!(snapshot.entries.len(), 3);
        assert_eq!(snapshot.entries[0].path, ["alpha.txt"]);
        assert_eq!(snapshot.entries[0].bytes, b"first");
        assert_eq!(snapshot.entries[1].path, ["folder"]);
        assert!(snapshot.entries[1].bytes.is_empty());
        assert_eq!(snapshot.entries[2].path, ["zeta.txt"]);
        assert_eq!(snapshot.entries[2].bytes, b"last");
        for descriptor in [&input.request, &input.snapshot] {
            assert_eq!(
                fcntl_get_seals(descriptor).expect("payload seals"),
                SealFlags::SEAL | SealFlags::SHRINK | SealFlags::GROW | SealFlags::WRITE
            );
            assert!(write(descriptor, b"changed").is_err());
        }
        let debug = format!("{input:?}");
        assert!(!debug.contains("alpha.txt"));
        assert!(!debug.contains("first"));
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn read_only_tool_input_rejects_duplicate_mixed_stale_and_oversized_inputs() {
        let root = temp_directory("read-only-rejections");
        let other_root = temp_directory("read-only-foreign");
        fs::write(root.join("allowed.txt"), b"approved").expect("fixture");
        fs::write(other_root.join("foreign.txt"), b"foreign").expect("foreign fixture");
        let workspace = authorize(&root);
        let mut foreign_workspace = authorize(&other_root);
        foreign_workspace.authorization_id =
            WorkspaceAuthorizationId::from_raw("authorization-foreign");

        let duplicate = LinuxReadOnlyToolInput::seal(
            "agentmage.workspace.read-file",
            "1.0.0",
            b"{}",
            vec![
                hold(&workspace, "allowed.txt", PathResolutionIntent::ReadFile),
                hold(&workspace, "allowed.txt", PathResolutionIntent::ReadFile),
            ],
        )
        .expect_err("duplicate path must fail");
        assert_eq!(duplicate.kind(), LinuxSandboxErrorKind::TargetMismatch);

        let mixed = LinuxReadOnlyToolInput::seal(
            "agentmage.workspace.read-file",
            "1.0.0",
            b"{}",
            vec![
                hold(&workspace, "allowed.txt", PathResolutionIntent::ReadFile),
                hold(
                    &foreign_workspace,
                    "foreign.txt",
                    PathResolutionIntent::ReadFile,
                ),
            ],
        )
        .expect_err("mixed authority must fail");
        assert_eq!(mixed.kind(), LinuxSandboxErrorKind::TargetMismatch);

        let stale = hold(&workspace, "allowed.txt", PathResolutionIntent::ReadFile);
        fs::write(root.join("allowed.txt"), b"changed").expect("mutated fixture");
        let stale_error = LinuxReadOnlyToolInput::seal(
            "agentmage.workspace.read-file",
            "1.0.0",
            b"{}",
            vec![stale],
        )
        .expect_err("stale held object must fail");
        assert_eq!(stale_error.kind(), LinuxSandboxErrorKind::StaleObject);

        let oversized = vec![b'x'; MAX_READ_ONLY_REQUEST_BYTES + 1];
        let oversized_error = LinuxReadOnlyToolInput::seal(
            "agentmage.workspace.read-file",
            "1.0.0",
            &oversized,
            vec![hold(
                &workspace,
                "allowed.txt",
                PathResolutionIntent::ReadFile,
            )],
        )
        .expect_err("oversized request must fail before projection");
        assert_eq!(
            oversized_error.kind(),
            LinuxSandboxErrorKind::InvalidManifest
        );
        fs::remove_dir_all(root).expect("cleanup");
        fs::remove_dir_all(other_root).expect("cleanup");
    }

    #[test]
    fn stale_held_file_fails_before_any_worker_process_can_start() {
        let root = temp_directory("stale");
        fs::write(root.join("allowed.txt"), b"first").expect("fixture");
        let workspace = authorize(&root);
        let held = hold(&workspace, "allowed.txt", PathResolutionIntent::ReadFile);
        fs::write(root.join("allowed.txt"), b"changed").expect("mutated fixture");
        let error = file_projection(&held)
            .expect_err("stale object must fail before manifest or worker launch");
        assert_eq!(error.kind(), LinuxSandboxErrorKind::StaleObject);
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn stale_held_directory_fails_projection_before_any_worker_process_can_start() {
        let root = temp_directory("stale-directory");
        fs::create_dir(root.join("folder")).expect("held directory");
        let workspace = authorize(&root);
        let held = hold(&workspace, "folder", PathResolutionIntent::ReadDirectory);
        fs::write(root.join("folder").join("changed.txt"), b"changed").expect("mutation");
        let error = directory_projection(&held, &[])
            .expect_err("stale directory must fail before manifest launch");
        assert_eq!(error.kind(), LinuxSandboxErrorKind::StaleObject);
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    #[ignore = "requires a supported Linux systemd user session and Bubblewrap runtime"]
    fn fresh_worker_reads_only_the_canonical_workspace_file() {
        let root = temp_directory("read");
        fs::write(root.join("allowed.txt"), b"bounded worker output\n").expect("fixture");
        let workspace = authorize(&root);
        let path = WorkspacePath::new(workspace.workspace_id.clone(), ["allowed.txt"])
            .expect("workspace path");
        let held = hold(&workspace, "allowed.txt", PathResolutionIntent::ReadFile);

        let result = runner()
            .run(&held, &LinuxSandboxOperation::ReadFile(path), &[])
            .expect("sandbox execution");
        assert!(result.success());
        assert_eq!(result.stdout(), b"bounded worker output\n");
        assert_eq!(result.stderr_bytes(), 0);
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    #[ignore = "requires a supported Linux systemd user session and Bubblewrap runtime"]
    fn directory_worker_receives_only_the_bounded_exclusion_safe_projection() {
        let root = temp_directory("directory-worker");
        fs::create_dir(root.join("folder")).expect("held directory");
        fs::write(root.join("folder").join("allowed.txt"), b"allowed").expect("allowed child");
        fs::write(root.join("folder").join("secret.txt"), b"secret").expect("secret child");
        let workspace = authorize(&root);
        let held = hold(&workspace, "folder", PathResolutionIntent::ReadDirectory);
        let path = held.workspace_path().clone();
        let excluded = exclusion(&workspace, &["folder", "secret.txt"]);
        let result = runner()
            .run(
                &held,
                &LinuxSandboxOperation::ReadDirectory(path),
                &[excluded],
            )
            .expect("directory projection worker");
        assert!(result.success());
        assert_eq!(result.stdout(), b"allowed.txt\0");
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    #[ignore = "requires a supported Linux systemd user session and Bubblewrap runtime"]
    fn foreign_workspace_identity_never_starts_a_worker() {
        let root = temp_directory("foreign");
        fs::write(root.join("allowed.txt"), b"content").expect("fixture");
        let workspace = authorize(&root);
        let held = hold(&workspace, "allowed.txt", PathResolutionIntent::ReadFile);
        let foreign =
            WorkspacePath::new(WorkspaceId::from_raw("workspace-foreign"), ["allowed.txt"])
                .expect("foreign path");

        let error = runner()
            .run(&held, &LinuxSandboxOperation::ReadFile(foreign), &[])
            .expect_err("foreign workspace must fail");
        assert_eq!(error.kind(), LinuxSandboxErrorKind::WorkspaceMismatch);
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    #[ignore = "requires a supported Linux systemd user session and Bubblewrap runtime"]
    fn worker_cannot_write_the_read_only_workspace() {
        let root = temp_directory("read-only");
        let fixture = root.join("allowed.txt");
        fs::write(&fixture, b"original").expect("fixture");
        let workspace = authorize(&root);
        let held = hold(&workspace, "allowed.txt", PathResolutionIntent::ReadFile);
        let touch = runner_for("/usr/bin/touch", LinuxSandboxLimits::default());

        let result = run_held_arguments(&touch, &held, &["/input/object".into()])
            .expect("contained worker result");
        assert!(!result.success());
        assert_eq!(fs::read(&fixture).expect("unchanged fixture"), b"original");
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    #[ignore = "requires a supported Linux systemd user session and Bubblewrap runtime"]
    fn worker_has_no_ambient_host_paths_devices_or_processes() {
        let root = temp_directory("ambient");
        fs::write(root.join("allowed.txt"), b"fixture").expect("fixture");
        let workspace = authorize(&root);
        let held = hold(&workspace, "allowed.txt", PathResolutionIntent::ReadFile);
        let bash = runner_for("/usr/bin/bash", LinuxSandboxLimits::default());
        let probe = concat!(
            "for path in /home /etc /sys /run /dev/sda; do ",
            "test ! -e \"$path\" || echo \"$path\"; done; ",
            "set -- /proc/[0-9]*; echo \"processes:$#\""
        );

        let result = run_held_arguments(&bash, &held, &["-c".into(), probe.into()])
            .expect("contained worker result");
        assert!(result.success());
        assert_eq!(result.stdout(), b"processes:2\n");
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    #[ignore = "requires a supported Linux systemd user session and Bubblewrap runtime"]
    fn worker_cannot_resolve_sibling_parent_or_hidden_descriptor_content() {
        let root = temp_directory("sibling");
        fs::write(root.join("allowed.txt"), b"allowed").expect("allowed fixture");
        fs::write(root.join("secret.txt"), b"secret").expect("secret fixture");
        let workspace = authorize(&root);
        let held = hold(&workspace, "allowed.txt", PathResolutionIntent::ReadFile);
        let bash = runner_for("/usr/bin/bash", LinuxSandboxLimits::default());
        let probe = concat!(
            "test -r /input/object || exit 10; ",
            "test ! -e /input/secret.txt || exit 11; ",
            "test ! -e /workspace || exit 12; ",
            "test ! -e /input/../secret.txt || exit 13; ",
            "for fd in /proc/self/fd/*; do ",
            "target=$(readlink \"$fd\" 2>/dev/null || true); ",
            "case \"$target\" in *secret.txt*) exit 14;; esac; done; ",
            "printf isolated"
        );
        let result = run_held_arguments(&bash, &held, &["-c".into(), probe.into()])
            .expect("contained worker result");
        assert!(result.success());
        assert_eq!(result.stdout(), b"isolated");
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    #[ignore = "requires a supported Linux systemd user session and Bubblewrap runtime"]
    fn bounded_scratch_cannot_escape_into_the_held_object_or_host_workspace() {
        let root = temp_directory("scratch");
        fs::write(root.join("allowed.txt"), b"original").expect("fixture");
        let workspace = authorize(&root);
        let held = hold(&workspace, "allowed.txt", PathResolutionIntent::ReadFile);
        let bash = runner_for("/usr/bin/bash", LinuxSandboxLimits::default());
        let probe = concat!(
            "printf 'scratch\\n' >/tmp/value; ",
            "IFS= read -r value </tmp/value; test \"$value\" = scratch || exit 20; ",
            "printf changed >/input/object 2>/dev/null && exit 21; ",
            "test ! -e /tmp/../input/secret.txt || exit 22; ",
            "printf bounded"
        );
        let result = run_held_arguments(&bash, &held, &["-c".into(), probe.into()])
            .expect("contained scratch result");
        assert!(result.success());
        assert_eq!(result.stdout(), b"bounded");
        assert_eq!(
            fs::read(root.join("allowed.txt")).expect("host read"),
            b"original"
        );
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    #[ignore = "requires a supported Linux systemd user session and Bubblewrap runtime"]
    fn worker_receives_only_the_fixed_environment_and_no_network() {
        let root = temp_directory("environment");
        fs::write(root.join("allowed.txt"), b"fixture").expect("fixture");
        let workspace = authorize(&root);
        let held = hold(&workspace, "allowed.txt", PathResolutionIntent::ReadFile);
        let environment_runner = runner_for("/usr/bin/env", LinuxSandboxLimits::default());
        let environment =
            run_held_arguments(&environment_runner, &held, &[]).expect("environment worker");
        assert!(environment.success());
        let environment_text =
            String::from_utf8(environment.stdout().to_vec()).expect("environment UTF-8");
        let mut variables = environment_text.lines().collect::<Vec<_>>();
        variables.sort_unstable();
        assert_eq!(variables, ["LANG=C", "PATH=/app", "PWD=/input"]);

        let bash = runner_for("/usr/bin/bash", LinuxSandboxLimits::default());
        let network = run_held_arguments(
            &bash,
            &held,
            &[
                "-c".into(),
                "if exec 9<>/dev/tcp/127.0.0.1/9; then exit 1; else exit 0; fi".into(),
            ],
        )
        .expect("network denial result");
        assert!(network.success());
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    #[ignore = "requires a supported Linux systemd user session and Bubblewrap runtime"]
    fn tool_worker_cannot_connect_to_raw_inference_loopback() {
        let root = temp_directory("raw-inference-connect");
        fs::write(root.join("allowed.txt"), b"fixture").expect("fixture");
        let workspace = authorize(&root);
        let held = hold(&workspace, "allowed.txt", PathResolutionIntent::ReadFile);
        let bash = runner_for("/usr/bin/bash", LinuxSandboxLimits::default());
        let network = run_held_arguments(
            &bash,
            &held,
            &[
                "-c".into(),
                "if exec 9<>/dev/tcp/127.0.0.1/12434; then exit 1; else exit 0; fi".into(),
            ],
        )
        .expect("raw inference connection denial");
        assert!(network.success());
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    #[ignore = "requires a supported Linux systemd user session and Bubblewrap runtime"]
    fn worker_output_is_drained_but_never_retained_past_the_bound() {
        let root = temp_directory("output");
        fs::write(root.join("large.txt"), vec![b'x'; 4096]).expect("fixture");
        let workspace = authorize(&root);
        let held = hold(&workspace, "large.txt", PathResolutionIntent::ReadFile);
        let limits =
            LinuxSandboxLimits::new(32 * 1024 * 1024, 4, 100, 5, 128).expect("bounded limits");
        let cat = runner_for("/usr/bin/cat", limits);

        let error = run_held_arguments(&cat, &held, &["/input/object".into()])
            .expect_err("oversized output must fail closed");
        assert_eq!(error.kind(), LinuxSandboxErrorKind::OutputLimitExceeded);
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    #[ignore = "requires a supported Linux systemd user session and Bubblewrap runtime"]
    fn worker_kernel_status_confirms_no_new_privileges_and_seccomp() {
        let root = temp_directory("kernel-status");
        fs::write(root.join("allowed.txt"), b"fixture").expect("fixture");
        let workspace = authorize(&root);
        let held = hold(&workspace, "allowed.txt", PathResolutionIntent::ReadFile);
        let result = run_held_arguments(&runner(), &held, &["/proc/self/status".into()])
            .expect("kernel status result");
        assert!(result.success());
        let status = String::from_utf8(result.stdout().to_vec()).expect("status UTF-8");
        assert!(status.lines().any(|line| line == "NoNewPrivs:\t1"));
        assert!(status.lines().any(|line| line == "Seccomp:\t2"));
        println!("agentmage-kernel-control no-new-privileges=1 seccomp=2");
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    #[ignore = "requires a supported Linux systemd user session and Bubblewrap runtime"]
    fn transient_service_terminates_an_unbounded_worker() {
        let root = temp_directory("runtime-limit");
        fs::write(root.join("allowed.txt"), b"fixture").expect("fixture");
        let workspace = authorize(&root);
        let held = hold(&workspace, "allowed.txt", PathResolutionIntent::ReadFile);
        let limits =
            LinuxSandboxLimits::new(32 * 1024 * 1024, 4, 100, 1, 4096).expect("bounded limits");
        let bash = runner_for("/usr/bin/bash", limits);
        let started = Instant::now();
        let result = run_held_arguments(&bash, &held, &["-c".into(), "while :; do :; done".into()])
            .expect("limited worker result");

        assert!(!result.success());
        assert!(started.elapsed() < Duration::from_secs(5));
        println!("agentmage-resource-control runtime-limit=enforced");
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    #[ignore = "requires root-owned lifecycle fixtures, a systemd user session, and Bubblewrap"]
    fn lifecycle_matrix_cleans_descendants_scratch_and_never_accepts_interrupted_output() {
        let root = temp_directory("lifecycle-matrix");
        fs::write(root.join("allowed.txt"), b"fixture").expect("fixture");
        let workspace = authorize(&root);
        let host_scratch = Path::new("/tmp/agentmage-lifecycle-scratch");
        assert!(!host_scratch.exists());

        for termination in ["cancel", "timeout", "kill", "crash"] {
            for phase in ["before", "during", "after"] {
                let case = format!("{termination}-{phase}");
                let fixture = lifecycle_fixture(&case);
                let fixture_name = fixture
                    .file_name()
                    .and_then(|value| value.to_str())
                    .expect("fixture name")
                    .to_owned();
                let runtime_seconds = if termination == "timeout" { 1 } else { 5 };
                let limits = LinuxSandboxLimits::new(
                    64 * 1024 * 1024,
                    16,
                    100,
                    runtime_seconds,
                    4 * 1024 * 1024,
                )
                .expect("lifecycle limits");
                let runner = runner_for(fixture.to_str().expect("fixture path"), limits);
                let input = lifecycle_input(&workspace);
                let cancellation = LinuxSandboxCancellation::default();
                let control_cancellation = cancellation.clone();
                let baseline_units = lifecycle_units();
                let control_baseline = baseline_units.clone();
                let control_name = fixture_name.clone();
                let control_termination = termination.to_owned();
                let controller = thread::spawn(move || {
                    let deadline = Instant::now() + Duration::from_secs(5);
                    let unit = loop {
                        let units = lifecycle_units()
                            .difference(&control_baseline)
                            .cloned()
                            .collect::<Vec<_>>();
                        if units.len() == 1
                            && lifecycle_processes(&control_name).len() >= 2
                            && lifecycle_scratch_visible(&control_name)
                        {
                            break units[0].clone();
                        }
                        assert!(
                            Instant::now() < deadline,
                            "lifecycle fixture did not become ready"
                        );
                        thread::sleep(Duration::from_millis(5));
                    };
                    thread::sleep(Duration::from_millis(50));
                    match control_termination.as_str() {
                        "cancel" => control_cancellation.cancel(),
                        "kill" => {
                            let status = Command::new("/usr/bin/systemctl")
                                .args([
                                    "--user",
                                    "kill",
                                    "--kill-whom=all",
                                    "--signal=KILL",
                                    unit.as_str(),
                                ])
                                .status()
                                .expect("kill lifecycle unit");
                            assert!(status.success());
                        }
                        "timeout" | "crash" => {}
                        _ => panic!("unknown lifecycle termination"),
                    }
                });

                let result = runner
                    .run_read_only_tool_with_cancellation(&input, &cancellation)
                    .expect("lifecycle result");
                controller.join().expect("lifecycle controller");
                let expected = match termination {
                    "cancel" => OperationOutcome::Cancelled,
                    "timeout" => OperationOutcome::TimedOut,
                    "kill" | "crash" => OperationOutcome::Failed,
                    _ => unreachable!(),
                };
                assert_eq!(result.outcome(), expected, "{case} outcome");
                assert!(!result.success(), "{case} false completion");
                match phase {
                    "before" => assert!(result.stdout().is_empty(), "{case} output"),
                    "during" => assert_eq!(result.stdout(), b"{", "{case} output"),
                    "after" => {
                        let value: serde_json::Value =
                            serde_json::from_slice(result.stdout()).expect("complete output JSON");
                        assert_eq!(value["outcome"], "succeeded", "{case} fixture output");
                    }
                    _ => unreachable!(),
                }
                wait_for_lifecycle_cleanup(&fixture_name, &baseline_units);
                assert!(!host_scratch.exists(), "{case} host scratch leak");
            }
        }
        fs::remove_dir_all(root).expect("cleanup");
    }
}
