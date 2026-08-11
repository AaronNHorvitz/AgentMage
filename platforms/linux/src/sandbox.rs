//! Fresh Linux worker isolation with descriptor-bound Bubblewrap mounts.

use std::ffi::OsString;
use std::fmt;
use std::io::{Read, Write};
use std::os::fd::AsRawFd;
use std::path::{Component, Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;

use agentmage_kernel_contracts::{AuthorizedWorkspaceHandle, WorkspacePath};
use rustix::fd::OwnedFd;
use rustix::fs::{FileType, Mode, OFlags, fstat, open};
use rustix::io::pread;
use rustix::process::getuid;
use rustix::rand::{GetRandomFlags, getrandom};
use seccompiler::{BpfProgram, TargetArch, compile_from_json};
use sha2::{Digest, Sha256};

use crate::LinuxAuthorizedWorkspace;

const MAX_RUNTIME_FILES: usize = 16;
const MAX_OUTPUT_BYTES: usize = 4 * 1024 * 1024;
const HASH_BUFFER_BYTES: usize = 64 * 1024;
const WORKER_GUEST_PATH: &str = "/app/worker";
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
    /// Read one canonical file beneath the already-authorized workspace.
    ReadFile(WorkspacePath),
}

/// Bounded result returned by a completed worker.
#[derive(Clone, PartialEq, Eq)]
pub struct LinuxSandboxResult {
    success: bool,
    stdout: Vec<u8>,
    stdout_sha256: [u8; 32],
    stderr_sha256: [u8; 32],
    stderr_bytes: usize,
}

impl LinuxSandboxResult {
    /// Reports whether the worker exited successfully.
    #[must_use]
    pub const fn success(&self) -> bool {
        self.success
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
            .field("success", &self.success)
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
        let systemd_run = verify_artifact(systemd_run.as_ref(), None, true)?;
        let bubblewrap = verify_artifact(bubblewrap.as_ref(), None, true)?;
        let worker = verify_artifact(worker.as_ref(), Some(Path::new(WORKER_GUEST_PATH)), false)?;
        let mut verified_runtime = Vec::with_capacity(runtime_files.len());
        for runtime in runtime_files {
            if !valid_runtime_guest_path(&runtime.guest_path)
                || runtime.guest_path == Path::new(WORKER_GUEST_PATH)
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

    /// Executes one typed read in a fresh worker with no network namespace connectivity.
    pub fn run(
        &self,
        workspace: &LinuxAuthorizedWorkspace,
        operation: &LinuxSandboxOperation,
    ) -> Result<LinuxSandboxResult, LinuxSandboxError> {
        let LinuxSandboxOperation::ReadFile(path) = operation;
        if path.workspace_id() != workspace.workspace_id() {
            return Err(error(LinuxSandboxErrorKind::WorkspaceMismatch));
        }
        self.run_arguments(workspace, &[guest_workspace_path(path).into_os_string()])
    }

    fn run_arguments(
        &self,
        workspace: &LinuxAuthorizedWorkspace,
        arguments: &[OsString],
    ) -> Result<LinuxSandboxResult, LinuxSandboxError> {
        let parent_pid = std::process::id();
        let descriptor_source =
            |descriptor: &OwnedFd| format!("/proc/{parent_pid}/fd/{}", descriptor.as_raw_fd());
        revalidate_launch_artifact(&self.manifest.systemd_run)?;
        revalidate_launch_artifact(&self.manifest.bubblewrap)?;
        let systemd_command = self
            .manifest
            .systemd_run
            .launch_path
            .as_deref()
            .ok_or_else(|| error(LinuxSandboxErrorKind::InvalidManifest))?;
        let bubblewrap_command = self
            .manifest
            .bubblewrap
            .launch_path
            .as_deref()
            .ok_or_else(|| error(LinuxSandboxErrorKind::InvalidManifest))?;
        let unit = random_unit_name()?;
        let runtime_directory = format!("/run/user/{}", getuid().as_raw());
        let session_bus = format!("unix:path={runtime_directory}/bus");
        let mut command = Command::new(systemd_command);
        command
            .env_clear()
            .env("XDG_RUNTIME_DIR", runtime_directory)
            .env("DBUS_SESSION_BUS_ADDRESS", session_bus)
            .arg("--user")
            .arg("--wait")
            .arg("--collect")
            .arg("--quiet")
            .arg("--pipe")
            .arg(format!("--unit={unit}"))
            .arg("--property=NoNewPrivileges=yes")
            .arg("--property=PrivateDevices=yes")
            .arg("--property=RestrictSUIDSGID=yes")
            .arg("--property=LockPersonality=yes")
            .arg("--property=RestrictAddressFamilies=AF_UNIX AF_NETLINK")
            .arg("--property=MemorySwapMax=0")
            .arg(format!("--property=MemoryMax={}", self.limits.memory_bytes))
            .arg(format!("--property=TasksMax={}", self.limits.task_count))
            .arg(format!("--property=CPUQuota={}%", self.limits.cpu_percent))
            .arg(format!(
                "--property=RuntimeMaxSec={}s",
                self.limits.runtime_seconds
            ))
            .arg(format!(
                "--property=OpenFile={}:workspace:read-only",
                descriptor_source(&workspace.root_descriptor)
            ))
            .arg(format!(
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
                "/workspace",
            ])
            .args(["--ro-bind-fd", "3", "/workspace"])
            .args(["--ro-bind-fd", "4", WORKER_GUEST_PATH]);
        for (index, runtime) in self.manifest.runtime_files.iter().enumerate() {
            command
                .arg("--ro-bind-fd")
                .arg((index + 5).to_string())
                .arg(runtime.guest_path.as_deref().expect("verified guest path"));
        }
        command
            .args([
                "--chdir",
                "/workspace",
                "--seccomp",
                "0",
                "--",
                WORKER_GUEST_PATH,
            ])
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
        let status = child
            .wait()
            .map_err(|_| error(LinuxSandboxErrorKind::ExecutionFailed))?;
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
            success: status.success(),
            stdout_sha256: digest_bytes(&stdout.retained),
            stderr_sha256: stderr.digest,
            stderr_bytes: stderr.total,
            stdout: stdout.retained,
        })
    }

    /// Returns the fixed seccomp policy identity used by every worker.
    #[must_use]
    pub const fn seccomp_policy_id(&self) -> &'static str {
        SECCOMP_POLICY_ID
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

fn compile_seccomp_policy() -> Result<Vec<u8>, LinuxSandboxError> {
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
    if FileType::from_raw_mode(stat.st_mode) != FileType::RegularFile
        || stat.st_uid != 0
        || stat.st_mode & 0o022 != 0
        || ((launch_by_path || guest_path == Some(Path::new(WORKER_GUEST_PATH)))
            && stat.st_mode & 0o111 == 0)
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

fn guest_workspace_path(path: &WorkspacePath) -> PathBuf {
    let mut guest = PathBuf::from("/workspace");
    for component in path.components() {
        guest.push(component.as_str());
    }
    guest
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
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::process::Command;
    use std::time::{Duration, Instant};

    use agentmage_kernel_contracts::{
        AdapterInstanceId, WorkspaceAuthorizationId, WorkspaceId, WorkspacePath,
    };
    use rustix::fs::{Mode, OFlags, open};

    use super::{
        LinuxSandboxErrorKind, LinuxSandboxLimits, LinuxSandboxManifest, LinuxSandboxOperation,
        LinuxSandboxRunner, LinuxWorkerRuntimeFile, compile_seccomp_policy,
    };
    use crate::{LinuxAuthorizedWorkspace, snapshot};

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
        let root_descriptor = open(
            root,
            OFlags::PATH | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
            Mode::empty(),
        )
        .expect("root descriptor");
        let root_snapshot = snapshot(&root_descriptor, None).expect("root snapshot");
        LinuxAuthorizedWorkspace {
            workspace_id: WorkspaceId::from_raw("workspace-sandbox"),
            authorization_id: WorkspaceAuthorizationId::from_raw("authorization-sandbox"),
            adapter_instance_id: AdapterInstanceId::from_raw("adapter-linux"),
            root_descriptor,
            root_snapshot,
        }
    }

    fn runtime_files(executable: &str) -> Vec<LinuxWorkerRuntimeFile> {
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
        let manifest = LinuxSandboxManifest::verify(
            "/usr/bin/systemd-run",
            "/usr/bin/bwrap",
            executable,
            &runtime_files(executable),
        )
        .expect("verified host manifest");
        LinuxSandboxRunner::new(manifest, limits).expect("sandbox runner")
    }

    fn runner() -> LinuxSandboxRunner {
        runner_for("/usr/bin/cat", LinuxSandboxLimits::default())
    }

    #[test]
    fn policy_compiles_to_nonempty_classic_bpf() {
        let policy = compile_seccomp_policy().expect("compiled policy");
        assert!(!policy.is_empty());
        assert_eq!(policy.len() % 8, 0);
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
    }

    #[test]
    fn fresh_worker_reads_only_the_canonical_workspace_file() {
        let root = temp_directory("read");
        fs::write(root.join("allowed.txt"), b"bounded worker output\n").expect("fixture");
        let workspace = authorize(&root);
        let path = WorkspacePath::new(workspace.workspace_id.clone(), ["allowed.txt"])
            .expect("workspace path");

        let result = runner()
            .run(&workspace, &LinuxSandboxOperation::ReadFile(path))
            .expect("sandbox execution");
        assert!(result.success());
        assert_eq!(result.stdout(), b"bounded worker output\n");
        assert_eq!(result.stderr_bytes(), 0);
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn foreign_workspace_identity_never_starts_a_worker() {
        let root = temp_directory("foreign");
        fs::write(root.join("allowed.txt"), b"content").expect("fixture");
        let workspace = authorize(&root);
        let foreign =
            WorkspacePath::new(WorkspaceId::from_raw("workspace-foreign"), ["allowed.txt"])
                .expect("foreign path");

        let error = runner()
            .run(&workspace, &LinuxSandboxOperation::ReadFile(foreign))
            .expect_err("foreign workspace must fail");
        assert_eq!(error.kind(), LinuxSandboxErrorKind::WorkspaceMismatch);
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn worker_cannot_write_the_read_only_workspace() {
        let root = temp_directory("read-only");
        let fixture = root.join("allowed.txt");
        fs::write(&fixture, b"original").expect("fixture");
        let workspace = authorize(&root);
        let touch = runner_for("/usr/bin/touch", LinuxSandboxLimits::default());

        let result = touch
            .run_arguments(&workspace, &["/workspace/allowed.txt".into()])
            .expect("contained worker result");
        assert!(!result.success());
        assert_eq!(fs::read(&fixture).expect("unchanged fixture"), b"original");
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn worker_has_no_ambient_host_paths_devices_or_processes() {
        let root = temp_directory("ambient");
        fs::write(root.join("allowed.txt"), b"fixture").expect("fixture");
        let workspace = authorize(&root);
        let bash = runner_for("/usr/bin/bash", LinuxSandboxLimits::default());
        let probe = concat!(
            "for path in /home /etc /sys /run /dev/sda; do ",
            "test ! -e \"$path\" || echo \"$path\"; done; ",
            "set -- /proc/[0-9]*; echo \"processes:$#\""
        );

        let result = bash
            .run_arguments(&workspace, &["-c".into(), probe.into()])
            .expect("contained worker result");
        assert!(result.success());
        assert_eq!(result.stdout(), b"processes:2\n");
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn worker_receives_only_the_fixed_environment_and_no_network() {
        let root = temp_directory("environment");
        fs::write(root.join("allowed.txt"), b"fixture").expect("fixture");
        let workspace = authorize(&root);
        let environment = runner_for("/usr/bin/env", LinuxSandboxLimits::default())
            .run_arguments(&workspace, &[])
            .expect("environment worker");
        assert!(environment.success());
        let environment_text =
            String::from_utf8(environment.stdout().to_vec()).expect("environment UTF-8");
        let mut variables = environment_text.lines().collect::<Vec<_>>();
        variables.sort_unstable();
        assert_eq!(variables, ["LANG=C", "PATH=/app", "PWD=/workspace"]);

        let bash = runner_for("/usr/bin/bash", LinuxSandboxLimits::default());
        let network = bash
            .run_arguments(
                &workspace,
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
    fn worker_output_is_drained_but_never_retained_past_the_bound() {
        let root = temp_directory("output");
        fs::write(root.join("large.txt"), vec![b'x'; 4096]).expect("fixture");
        let workspace = authorize(&root);
        let limits =
            LinuxSandboxLimits::new(32 * 1024 * 1024, 4, 100, 5, 128).expect("bounded limits");
        let cat = runner_for("/usr/bin/cat", limits);

        let error = cat
            .run_arguments(&workspace, &["/workspace/large.txt".into()])
            .expect_err("oversized output must fail closed");
        assert_eq!(error.kind(), LinuxSandboxErrorKind::OutputLimitExceeded);
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn worker_kernel_status_confirms_no_new_privileges_and_seccomp() {
        let root = temp_directory("kernel-status");
        fs::write(root.join("allowed.txt"), b"fixture").expect("fixture");
        let workspace = authorize(&root);
        let result = runner()
            .run_arguments(&workspace, &["/proc/self/status".into()])
            .expect("kernel status result");
        assert!(result.success());
        let status = String::from_utf8(result.stdout().to_vec()).expect("status UTF-8");
        assert!(status.lines().any(|line| line == "NoNewPrivs:\t1"));
        assert!(status.lines().any(|line| line == "Seccomp:\t2"));
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn transient_service_terminates_an_unbounded_worker() {
        let root = temp_directory("runtime-limit");
        fs::write(root.join("allowed.txt"), b"fixture").expect("fixture");
        let workspace = authorize(&root);
        let limits =
            LinuxSandboxLimits::new(32 * 1024 * 1024, 4, 100, 1, 4096).expect("bounded limits");
        let bash = runner_for("/usr/bin/bash", limits);
        let started = Instant::now();
        let result = bash
            .run_arguments(&workspace, &["-c".into(), "while :; do :; done".into()])
            .expect("limited worker result");

        assert!(!result.success());
        assert!(started.elapsed() < Duration::from_secs(5));
        fs::remove_dir_all(root).expect("cleanup");
    }
}
