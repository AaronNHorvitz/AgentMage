//! Descriptor-bound offline Linux execution for exact command templates.

use std::collections::BTreeMap;
use std::fmt;
use std::fs;
use std::io::Read;
use std::os::fd::{AsRawFd, RawFd};
use std::os::unix::fs::MetadataExt as _;
use std::os::unix::process::ExitStatusExt as _;
use std::path::{Component, Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use agentmage_kernel_engine::command_runner::{
    BoundedCommandExecutor, CommandLaunchPermit, CommandPlatformResult, CommandRegistry,
    CommandResourceUsage, CommandSpec, CommandTermination, CommandWorkingDirectory,
};
use agentmage_kernel_engine::propagation::CancellationToken;
use rustix::fd::OwnedFd;
use rustix::fs::{
    FileType, MemfdFlags, Mode, OFlags, SealFlags, SeekFrom, fcntl_add_seals, fstat, memfd_create,
    open, seek,
};
use rustix::io::{pread, write};
use rustix::process::getuid;
use rustix::rand::{GetRandomFlags, getrandom};
use sha2::{Digest, Sha256};

use crate::{LinuxAuthorizedWorkspace, sandbox::compile_seccomp_policy};

const MAX_ARTIFACT_BYTES: u64 = 256 * 1024 * 1024;
const HASH_BUFFER_BYTES: usize = 64 * 1024;
const POLL_INTERVAL: Duration = Duration::from_millis(10);
const TERMINATION_GRACE: Duration = Duration::from_secs(3);
const GUEST_EXECUTABLE: &str = "/app/command";
/// systemd passes `OpenFile=` descriptors to the unit from this number upward,
/// in the exact order the properties are declared.
const LISTEN_FDS_START: u32 = 3;
/// Holds the verified command executable.
const COMMAND_DESCRIPTOR: u32 = LISTEN_FDS_START;
/// Holds the sealed seccomp policy.
const SECCOMP_DESCRIPTOR: u32 = LISTEN_FDS_START + 1;
/// Holds the reopened owned worktree; absent for empty scratch.
const WORKTREE_DESCRIPTOR: u32 = LISTEN_FDS_START + 2;

/// Content-free failure categories at the Linux command boundary.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LinuxCommandRunnerErrorKind {
    /// A launcher, executable, registry binding, or path failed verification.
    InvalidManifest,
    /// The fixed seccomp policy could not be compiled.
    SeccompUnavailable,
    /// The systemd/Bubblewrap isolation stack could not start.
    IsolationUnavailable,
}

impl LinuxCommandRunnerErrorKind {
    /// Returns a stable machine-readable code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidManifest => "linux.command.manifest.invalid",
            Self::SeccompUnavailable => "linux.command.seccomp.unavailable",
            Self::IsolationUnavailable => "linux.command.isolation.unavailable",
        }
    }
}

/// Redacted Linux command-runner failure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LinuxCommandRunnerError {
    kind: LinuxCommandRunnerErrorKind,
}

impl LinuxCommandRunnerError {
    /// Returns the stable failure category.
    #[must_use]
    pub const fn kind(self) -> LinuxCommandRunnerErrorKind {
        self.kind
    }
}

impl fmt::Display for LinuxCommandRunnerError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.kind.code())
    }
}

impl std::error::Error for LinuxCommandRunnerError {}

struct VerifiedCommandArtifact {
    descriptor: OwnedFd,
    launch_path: PathBuf,
    sha256: String,
}

impl fmt::Debug for VerifiedCommandArtifact {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("VerifiedCommandArtifact")
            .field("launch_path", &self.launch_path)
            .field("sha256", &self.sha256)
            .finish_non_exhaustive()
    }
}

/// Descriptor-held Linux launcher and exact command inventory.
pub struct LinuxCommandManifest {
    systemd_run: VerifiedCommandArtifact,
    systemctl: VerifiedCommandArtifact,
    bubblewrap: VerifiedCommandArtifact,
    commands: BTreeMap<(String, String), VerifiedCommandArtifact>,
}

impl LinuxCommandManifest {
    /// Verifies every fixed launcher and every executable in one frozen registry.
    pub fn verify(
        systemd_run: impl AsRef<Path>,
        systemctl: impl AsRef<Path>,
        bubblewrap: impl AsRef<Path>,
        registry: &CommandRegistry,
    ) -> Result<Self, LinuxCommandRunnerError> {
        let systemd_run = verify_executable(systemd_run.as_ref())?;
        let systemctl = verify_executable(systemctl.as_ref())?;
        let bubblewrap = verify_executable(bubblewrap.as_ref())?;
        let mut commands = BTreeMap::new();
        for command in registry.commands() {
            command
                .verify()
                .map_err(|_| error(LinuxCommandRunnerErrorKind::InvalidManifest))?;
            let artifact = verify_executable(Path::new(&command.executable))?;
            if artifact.sha256 != command.executable_sha256 {
                return Err(error(LinuxCommandRunnerErrorKind::InvalidManifest));
            }
            commands.insert(
                (
                    command.template_id.clone(),
                    command.template_version.clone(),
                ),
                artifact,
            );
        }
        if commands.len() != registry.commands().len() {
            return Err(error(LinuxCommandRunnerErrorKind::InvalidManifest));
        }
        Ok(Self {
            systemd_run,
            systemctl,
            bubblewrap,
            commands,
        })
    }
}

impl fmt::Debug for LinuxCommandManifest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("LinuxCommandManifest")
            .field("systemd_run", &self.systemd_run)
            .field("systemctl", &self.systemctl)
            .field("bubblewrap", &self.bubblewrap)
            .field("command_count", &self.commands.len())
            .finish()
    }
}

/// Linux implementation of the nonforgeable bounded-command executor.
#[derive(Debug)]
pub struct LinuxBoundedCommandExecutor {
    manifest: LinuxCommandManifest,
    seccomp_descriptor: OwnedFd,
}

impl LinuxBoundedCommandExecutor {
    /// Compiles the fixed seccomp policy and creates an inert executor.
    pub fn new(manifest: LinuxCommandManifest) -> Result<Self, LinuxCommandRunnerError> {
        let seccomp_bpf = compile_seccomp_policy()
            .map_err(|_| error(LinuxCommandRunnerErrorKind::SeccompUnavailable))?;
        let seccomp_descriptor = sealed_seccomp_descriptor(&seccomp_bpf)?;
        Ok(Self {
            manifest,
            seccomp_descriptor,
        })
    }

    fn run(
        &self,
        command: &CommandSpec,
        held_working_directory: &LinuxAuthorizedWorkspace,
        cancellation: &CancellationToken,
    ) -> CommandPlatformResult {
        let Some(artifact) = self.manifest.commands.get(&(
            command.template_id.clone(),
            command.template_version.clone(),
        )) else {
            return failed("linux.command.template.absent");
        };
        if command.verify().is_err()
            || artifact.sha256 != command.executable_sha256
            || revalidate(&self.manifest.systemd_run).is_err()
            || revalidate(&self.manifest.systemctl).is_err()
            || revalidate(&self.manifest.bubblewrap).is_err()
            || revalidate(artifact).is_err()
        {
            return failed("linux.command.artifact.changed");
        }
        if held_working_directory.revalidate().is_err() {
            return failed("linux.command.worktree.changed");
        }

        let Ok(unit_stem) = random_unit_name() else {
            return failed("linux.command.unit.identity");
        };
        let unit = format!("{unit_stem}.service");
        let parent_pid = std::process::id();
        let held_worktree = match command.working_directory {
            CommandWorkingDirectory::EmptyScratch => None,
            CommandWorkingDirectory::OwnedWorktree => {
                let Ok(descriptor) = held_working_directory.reopen_root_directory() else {
                    return failed("linux.command.worktree.descriptor");
                };
                Some(descriptor)
            }
        };
        let open_files = open_file_properties(
            parent_pid,
            artifact.descriptor.as_raw_fd(),
            self.seccomp_descriptor.as_raw_fd(),
            held_worktree.as_ref().map(AsRawFd::as_raw_fd),
        );
        let command_descriptor_argument = COMMAND_DESCRIPTOR.to_string();
        let seccomp_descriptor_argument = SECCOMP_DESCRIPTOR.to_string();
        let worktree_descriptor_argument = WORKTREE_DESCRIPTOR.to_string();
        let runtime_directory = format!("/run/user/{}", getuid().as_raw());
        let session_bus = format!("unix:path={runtime_directory}/bus");
        let timeout_seconds = command.bounds.timeout_ms.div_ceil(1_000).max(1);

        let mut process = Command::new(&self.manifest.systemd_run.launch_path);
        process
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
            .arg("--property=KillMode=control-group")
            .arg("--property=SendSIGKILL=yes")
            .arg("--property=TimeoutStopSec=2s")
            .arg(format!(
                "--property=MemoryMax={}",
                command.bounds.memory_bytes
            ))
            .arg(format!("--property=TasksMax={}", command.bounds.task_count))
            .arg(format!(
                "--property=CPUQuota={}%",
                command.bounds.cpu_percent
            ))
            .arg(format!("--property=RuntimeMaxSec={timeout_seconds}s"));
        for property in &open_files {
            process.arg(property);
        }
        process.arg(&self.manifest.bubblewrap.launch_path).args([
            "--unshare-all",
            "--unshare-user",
            "--disable-userns",
            "--new-session",
            "--die-with-parent",
            "--clearenv",
        ]);
        for (name, value) in &command.environment {
            process.arg("--setenv").arg(name).arg(value);
        }
        process.args([
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
        ]);
        match command.working_directory {
            CommandWorkingDirectory::EmptyScratch => {
                process.args(["--tmpfs", "/work"]);
            }
            CommandWorkingDirectory::OwnedWorktree => {
                process.args(["--dir", "/work"]);
            }
        }
        process.args(["--dir", "/usr"]);
        add_runtime_mounts(&mut process);
        process.args([
            "--ro-bind-fd",
            command_descriptor_argument.as_str(),
            GUEST_EXECUTABLE,
        ]);
        if command.working_directory == CommandWorkingDirectory::OwnedWorktree {
            process.args([
                "--ro-bind-fd",
                worktree_descriptor_argument.as_str(),
                "/work",
            ]);
        }
        process
            .args([
                "--chdir",
                guest_working_directory(command.working_directory),
            ])
            .args([
                "--seccomp",
                seccomp_descriptor_argument.as_str(),
                "--",
                GUEST_EXECUTABLE,
            ])
            .args(&command.arguments)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let started = Instant::now();
        let Ok(mut child) = process.spawn() else {
            return failed("linux.command.isolation.start");
        };
        let Some(stdout) = child.stdout.take() else {
            terminate_unit(
                &self.manifest,
                &unit,
                &runtime_directory,
                &session_bus,
                &mut child,
            );
            return failed("linux.command.stdout.pipe");
        };
        let Some(stderr) = child.stderr.take() else {
            terminate_unit(
                &self.manifest,
                &unit,
                &runtime_directory,
                &session_bus,
                &mut child,
            );
            return failed("linux.command.stderr.pipe");
        };
        let stdout_limit = command.bounds.stdout_bytes;
        let stderr_limit = command.bounds.stderr_bytes;
        let stdout_reader = thread::spawn(move || read_bounded(stdout, stdout_limit));
        let stderr_reader = thread::spawn(move || read_bounded(stderr, stderr_limit));
        let deadline = started + Duration::from_millis(command.bounds.timeout_ms);
        let mut resource_observation = ResourceObservation::default();
        let (termination, status, cleanup_verified, platform_code) = loop {
            resource_observation.merge(observe_unit_resources(
                &self.manifest,
                &unit,
                &runtime_directory,
                &session_bus,
            ));
            match child.try_wait() {
                Ok(Some(status)) => {
                    let cleanup =
                        cleanup_unit(&self.manifest, &unit, &runtime_directory, &session_bus);
                    break (
                        CommandTermination::Exited,
                        Some(status),
                        cleanup,
                        "linux.command.exited",
                    );
                }
                Ok(None) if cancellation.is_cancelled() => {
                    let cleanup = terminate_unit(
                        &self.manifest,
                        &unit,
                        &runtime_directory,
                        &session_bus,
                        &mut child,
                    );
                    break (
                        CommandTermination::Cancelled,
                        None,
                        cleanup,
                        "linux.command.cancelled",
                    );
                }
                Ok(None) if Instant::now() >= deadline => {
                    let cleanup = terminate_unit(
                        &self.manifest,
                        &unit,
                        &runtime_directory,
                        &session_bus,
                        &mut child,
                    );
                    break (
                        CommandTermination::TimedOut,
                        None,
                        cleanup,
                        "linux.command.timed_out",
                    );
                }
                Ok(None) => thread::sleep(POLL_INTERVAL),
                Err(_) => {
                    let cleanup = terminate_unit(
                        &self.manifest,
                        &unit,
                        &runtime_directory,
                        &session_bus,
                        &mut child,
                    );
                    break (
                        CommandTermination::LaunchFailed,
                        None,
                        cleanup,
                        "linux.command.wait.failed",
                    );
                }
            }
        };

        // The service manager, not this process, opens every `OpenFile=` path, and it
        // does so asynchronously while starting the unit. Releasing the held worktree
        // before the unit has finished would let the descriptor number be reused.
        drop(held_worktree);
        let Ok(stdout) = stdout_reader.join().unwrap_or(Err(())) else {
            return failed("linux.command.stdout.read");
        };
        let Ok(stderr) = stderr_reader.join().unwrap_or(Err(())) else {
            return failed("linux.command.stderr.read");
        };
        let termination = if stdout.exceeded || stderr.exceeded {
            CommandTermination::OutputLimit
        } else {
            termination
        };
        CommandPlatformResult {
            termination,
            exit_code: (termination == CommandTermination::Exited)
                .then(|| status.as_ref().and_then(ExitStatus::code))
                .flatten(),
            signal: status.as_ref().and_then(|status| status.signal()),
            stdout: stdout.retained,
            stdout_sha256: stdout.sha256,
            stdout_total_bytes: stdout.total,
            stderr: stderr.retained,
            stderr_sha256: stderr.sha256,
            stderr_total_bytes: stderr.total,
            elapsed_ms: u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX),
            resource_usage: resource_observation.finish(),
            descendants_terminated: cleanup_verified,
            platform_code: platform_code.to_owned(),
        }
    }
}

#[derive(Default)]
struct ResourceObservation {
    cpu_time_ns: u64,
    peak_memory_bytes: u64,
    peak_task_count: u32,
    observed: bool,
}

impl ResourceObservation {
    fn merge(&mut self, observation: Option<CommandResourceUsage>) {
        let Some(observation) = observation else {
            return;
        };
        self.observed = true;
        self.cpu_time_ns = self.cpu_time_ns.max(observation.cpu_time_ns);
        self.peak_memory_bytes = self.peak_memory_bytes.max(observation.peak_memory_bytes);
        self.peak_task_count = self.peak_task_count.max(observation.peak_task_count);
    }

    fn finish(self) -> Option<CommandResourceUsage> {
        self.observed.then_some(CommandResourceUsage {
            cpu_time_ns: self.cpu_time_ns,
            peak_memory_bytes: self.peak_memory_bytes,
            peak_task_count: self.peak_task_count,
        })
    }
}

fn observe_unit_resources(
    manifest: &LinuxCommandManifest,
    unit: &str,
    runtime_directory: &str,
    session_bus: &str,
) -> Option<CommandResourceUsage> {
    if revalidate(&manifest.systemctl).is_err() {
        return None;
    }
    let result = Command::new(&manifest.systemctl.launch_path)
        .env_clear()
        .env("XDG_RUNTIME_DIR", runtime_directory)
        .env("DBUS_SESSION_BUS_ADDRESS", session_bus)
        .args([
            "--user",
            "show",
            unit,
            "--no-pager",
            "--property=CPUUsageNSec",
            "--property=MemoryPeak",
            "--property=TasksCurrent",
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .ok()?;
    if !result.status.success() {
        return None;
    }
    let output = std::str::from_utf8(&result.stdout).ok()?;
    let properties = output
        .lines()
        .filter_map(|line| line.split_once('='))
        .collect::<BTreeMap<_, _>>();
    Some(CommandResourceUsage {
        cpu_time_ns: properties.get("CPUUsageNSec")?.parse().ok()?,
        peak_memory_bytes: properties.get("MemoryPeak")?.parse().ok()?,
        peak_task_count: properties.get("TasksCurrent")?.parse().ok()?,
    })
}

impl BoundedCommandExecutor for LinuxBoundedCommandExecutor {
    type WorkingDirectory = LinuxAuthorizedWorkspace;

    fn execute(
        &mut self,
        permit: CommandLaunchPermit<'_>,
        working_directory: &Self::WorkingDirectory,
        cancellation: &CancellationToken,
    ) -> CommandPlatformResult {
        self.run(permit.command(), working_directory, cancellation)
    }
}

struct BoundedRead {
    retained: Vec<u8>,
    sha256: String,
    total: u64,
    exceeded: bool,
}

fn read_bounded(mut input: impl Read, limit: u64) -> Result<BoundedRead, ()> {
    let capacity = usize::try_from(limit.min(HASH_BUFFER_BYTES as u64)).map_err(|_| ())?;
    let mut retained = Vec::with_capacity(capacity);
    let mut digest = Sha256::new();
    let mut total = 0_u64;
    let mut buffer = [0_u8; HASH_BUFFER_BYTES];
    loop {
        let count = input.read(&mut buffer).map_err(|_| ())?;
        if count == 0 {
            break;
        }
        digest.update(&buffer[..count]);
        total = total.saturating_add(count as u64);
        if (retained.len() as u64) < limit {
            let remaining = usize::try_from(limit - retained.len() as u64).map_err(|_| ())?;
            retained.extend_from_slice(&buffer[..count.min(remaining)]);
        }
    }
    Ok(BoundedRead {
        exceeded: total > limit,
        retained,
        sha256: hex(&digest.finalize()),
        total,
    })
}

fn terminate_unit(
    manifest: &LinuxCommandManifest,
    unit: &str,
    runtime_directory: &str,
    session_bus: &str,
    child: &mut Child,
) -> bool {
    let _ = systemctl(
        manifest,
        runtime_directory,
        session_bus,
        ["kill", "--signal=KILL", "--kill-whom=all", unit],
    );
    let _ = systemctl(
        manifest,
        runtime_directory,
        session_bus,
        ["stop", unit, "--no-block", "--no-ask-password"],
    );
    let deadline = Instant::now() + TERMINATION_GRACE;
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
    let inactive = unit_is_inactive(manifest, unit, runtime_directory, session_bus);
    let _ = cleanup_unit(manifest, unit, runtime_directory, session_bus);
    waited && inactive
}

fn cleanup_unit(
    manifest: &LinuxCommandManifest,
    unit: &str,
    runtime_directory: &str,
    session_bus: &str,
) -> bool {
    let inactive = unit_is_inactive(manifest, unit, runtime_directory, session_bus);
    let _ = systemctl(
        manifest,
        runtime_directory,
        session_bus,
        ["reset-failed", unit, "--no-ask-password"],
    );
    inactive
}

fn unit_is_inactive(
    manifest: &LinuxCommandManifest,
    unit: &str,
    runtime_directory: &str,
    session_bus: &str,
) -> bool {
    if revalidate(&manifest.systemctl).is_err() {
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

fn systemctl<const N: usize>(
    manifest: &LinuxCommandManifest,
    runtime_directory: &str,
    session_bus: &str,
    arguments: [&str; N],
) -> bool {
    if revalidate(&manifest.systemctl).is_err() {
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

/// Builds the `OpenFile=` properties in the exact order systemd numbers them,
/// so guest descriptors always match [`COMMAND_DESCRIPTOR`], [`SECCOMP_DESCRIPTOR`],
/// and [`WORKTREE_DESCRIPTOR`].
fn open_file_properties(
    parent_pid: u32,
    command_descriptor: RawFd,
    seccomp_descriptor: RawFd,
    worktree_descriptor: Option<RawFd>,
) -> Vec<String> {
    let mut properties = vec![
        open_file_property(parent_pid, command_descriptor, "command"),
        open_file_property(parent_pid, seccomp_descriptor, "seccomp"),
    ];
    properties.extend(
        worktree_descriptor
            .map(|descriptor| open_file_property(parent_pid, descriptor, "worktree")),
    );
    properties
}

fn open_file_property(parent_pid: u32, descriptor: RawFd, name: &str) -> String {
    format!("--property=OpenFile=/proc/{parent_pid}/fd/{descriptor}:{name}:read-only")
}

fn sealed_seccomp_descriptor(bytes: &[u8]) -> Result<OwnedFd, LinuxCommandRunnerError> {
    if bytes.is_empty() {
        return Err(error(LinuxCommandRunnerErrorKind::SeccompUnavailable));
    }
    let descriptor = memfd_create(
        "agentmage-command-seccomp",
        MemfdFlags::CLOEXEC | MemfdFlags::ALLOW_SEALING,
    )
    .map_err(|_| error(LinuxCommandRunnerErrorKind::SeccompUnavailable))?;
    let mut remaining = bytes;
    while !remaining.is_empty() {
        let count = write(&descriptor, remaining)
            .map_err(|_| error(LinuxCommandRunnerErrorKind::SeccompUnavailable))?;
        if count == 0 {
            return Err(error(LinuxCommandRunnerErrorKind::SeccompUnavailable));
        }
        remaining = &remaining[count..];
    }
    seek(&descriptor, SeekFrom::Start(0))
        .map_err(|_| error(LinuxCommandRunnerErrorKind::SeccompUnavailable))?;
    fcntl_add_seals(
        &descriptor,
        SealFlags::SEAL | SealFlags::SHRINK | SealFlags::GROW | SealFlags::WRITE,
    )
    .map_err(|_| error(LinuxCommandRunnerErrorKind::SeccompUnavailable))?;
    Ok(descriptor)
}

fn add_runtime_mounts(command: &mut Command) {
    for path in ["/usr/lib", "/usr/lib64", "/lib", "/lib64"] {
        if Path::new(path).exists() {
            command.args(["--ro-bind", path, path]);
        }
    }
    if Path::new("/etc/ld.so.cache").is_file() {
        command.args(["--ro-bind", "/etc/ld.so.cache", "/etc/ld.so.cache"]);
    }
}

const fn guest_working_directory(directory: CommandWorkingDirectory) -> &'static str {
    match directory {
        CommandWorkingDirectory::EmptyScratch | CommandWorkingDirectory::OwnedWorktree => "/work",
    }
}

fn verify_executable(path: &Path) -> Result<VerifiedCommandArtifact, LinuxCommandRunnerError> {
    if !path.is_absolute() {
        return Err(error(LinuxCommandRunnerErrorKind::InvalidManifest));
    }
    verify_root_owned_path(path)?;
    let descriptor = open(
        path,
        OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::CLOEXEC | OFlags::NONBLOCK,
        Mode::empty(),
    )
    .map_err(|_| error(LinuxCommandRunnerErrorKind::InvalidManifest))?;
    let stat =
        fstat(&descriptor).map_err(|_| error(LinuxCommandRunnerErrorKind::InvalidManifest))?;
    if FileType::from_raw_mode(stat.st_mode) != FileType::RegularFile
        || stat.st_uid != 0
        || stat.st_mode & 0o022 != 0
        || stat.st_mode & 0o111 == 0
    {
        return Err(error(LinuxCommandRunnerErrorKind::InvalidManifest));
    }
    let sha256 = hash_descriptor(&descriptor, stat.st_size)?;
    Ok(VerifiedCommandArtifact {
        descriptor,
        launch_path: path.to_path_buf(),
        sha256,
    })
}

fn verify_root_owned_path(path: &Path) -> Result<(), LinuxCommandRunnerError> {
    let mut current = PathBuf::from("/");
    for component in path.components().skip(1) {
        let Component::Normal(component) = component else {
            return Err(error(LinuxCommandRunnerErrorKind::InvalidManifest));
        };
        current.push(component);
        let metadata = fs::symlink_metadata(&current)
            .map_err(|_| error(LinuxCommandRunnerErrorKind::InvalidManifest))?;
        if metadata.uid() != 0 || metadata.mode() & 0o022 != 0 || metadata.file_type().is_symlink()
        {
            return Err(error(LinuxCommandRunnerErrorKind::InvalidManifest));
        }
    }
    Ok(())
}

fn revalidate(artifact: &VerifiedCommandArtifact) -> Result<(), LinuxCommandRunnerError> {
    let current = verify_executable(&artifact.launch_path)?;
    if current.sha256 != artifact.sha256 {
        return Err(error(LinuxCommandRunnerErrorKind::InvalidManifest));
    }
    Ok(())
}

fn hash_descriptor(descriptor: &OwnedFd, size: i64) -> Result<String, LinuxCommandRunnerError> {
    let size = u64::try_from(size)
        .ok()
        .filter(|size| *size <= MAX_ARTIFACT_BYTES)
        .ok_or_else(|| error(LinuxCommandRunnerErrorKind::InvalidManifest))?;
    let mut digest = Sha256::new();
    let mut buffer = [0_u8; HASH_BUFFER_BYTES];
    let mut offset = 0_u64;
    while offset < size {
        let requested = usize::try_from((size - offset).min(HASH_BUFFER_BYTES as u64))
            .map_err(|_| error(LinuxCommandRunnerErrorKind::InvalidManifest))?;
        let count = pread(descriptor, &mut buffer[..requested], offset)
            .map_err(|_| error(LinuxCommandRunnerErrorKind::InvalidManifest))?;
        if count == 0 {
            return Err(error(LinuxCommandRunnerErrorKind::InvalidManifest));
        }
        digest.update(&buffer[..count]);
        offset = offset
            .checked_add(count as u64)
            .ok_or_else(|| error(LinuxCommandRunnerErrorKind::InvalidManifest))?;
    }
    Ok(hex(&digest.finalize()))
}

fn random_unit_name() -> Result<String, LinuxCommandRunnerError> {
    let mut random = [0_u8; 12];
    getrandom(&mut random, GetRandomFlags::empty())
        .map_err(|_| error(LinuxCommandRunnerErrorKind::IsolationUnavailable))?;
    Ok(format!("agentmage-command-{}", hex(&random)))
}

fn failed(code: &str) -> CommandPlatformResult {
    CommandPlatformResult {
        termination: CommandTermination::LaunchFailed,
        exit_code: None,
        signal: None,
        stdout: Vec::new(),
        stdout_sha256: hex(&Sha256::digest([])),
        stdout_total_bytes: 0,
        stderr: Vec::new(),
        stderr_sha256: hex(&Sha256::digest([])),
        stderr_total_bytes: 0,
        elapsed_ms: 0,
        resource_usage: None,
        descendants_terminated: true,
        platform_code: code.to_owned(),
    }
}

fn error(kind: LinuxCommandRunnerErrorKind) -> LinuxCommandRunnerError {
    LinuxCommandRunnerError { kind }
}

fn hex(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        use fmt::Write as _;
        write!(&mut output, "{byte:02x}").expect("writing to a String cannot fail");
    }
    output
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::fs;
    use std::os::unix::fs::PermissionsExt as _;
    use std::path::PathBuf;
    use std::process::{Command, Stdio};
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::thread;
    use std::time::Duration;

    use agentmage_kernel_contracts::{
        AdapterInstanceId, BoundaryKind, CONTRACT_SCHEMA_VERSION, CancellationId,
        CancellationReason, CancellationSignal, CorrelationId, TaskId, WorkspaceAuthorizationId,
        WorkspaceId,
    };
    use agentmage_kernel_engine::command_runner::{
        CommandBounds, CommandRegistry, CommandRisk, CommandSpec, CommandTermination,
        CommandWorkingDirectory,
    };
    use agentmage_kernel_engine::propagation::CancellationToken;

    use rustix::fs::{Mode, OFlags, open};
    use rustix::io::{FdFlags, fcntl_getfd};
    use sha2::Digest as _;

    use super::{LinuxBoundedCommandExecutor, LinuxCommandManifest};
    use crate::LinuxAuthorizedWorkspace;

    static TEMP_ID: AtomicU64 = AtomicU64::new(1);

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new() -> Self {
            let id = TEMP_ID.fetch_add(1, Ordering::SeqCst);
            let path = std::env::temp_dir().join(format!(
                "agentmage-linux-command-{}-{id}",
                std::process::id()
            ));
            fs::create_dir(&path).expect("temporary directory creates");
            Self(path)
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.0).expect("temporary directory removes");
        }
    }

    fn held_worktree() -> (TestDirectory, LinuxAuthorizedWorkspace) {
        let temporary = TestDirectory::new();
        fs::create_dir(temporary.0.join("owned-worktree")).expect("worktree creates");
        fs::write(temporary.0.join("owned-worktree/marker.txt"), b"agentmage")
            .expect("marker writes");
        let adapter_id = AdapterInstanceId::from_raw("adapter-linux-command-0001");
        let workspace = crate::authorize_workspace_root(
            &temporary.0.join("owned-worktree"),
            WorkspaceId::from_raw("workspace-linux-command-0001"),
            WorkspaceAuthorizationId::from_raw("authorization-linux-command-0001"),
            adapter_id,
        )
        .expect("workspace authorizes");
        (temporary, workspace)
    }

    #[test]
    fn open_file_properties_number_guest_descriptors_in_declaration_order() {
        let scratch = super::open_file_properties(4_242, 7, 9, None);
        assert_eq!(
            scratch,
            vec![
                "--property=OpenFile=/proc/4242/fd/7:command:read-only".to_owned(),
                "--property=OpenFile=/proc/4242/fd/9:seccomp:read-only".to_owned(),
            ]
        );

        let owned = super::open_file_properties(4_242, 7, 9, Some(11));
        assert_eq!(owned.len(), 3);
        assert_eq!(owned[..2], scratch[..]);
        assert_eq!(
            owned[2],
            "--property=OpenFile=/proc/4242/fd/11:worktree:read-only"
        );

        for (offset, property) in owned.iter().enumerate() {
            let guest = super::LISTEN_FDS_START + u32::try_from(offset).expect("offset");
            let expected = match guest {
                super::COMMAND_DESCRIPTOR => "command",
                super::SECCOMP_DESCRIPTOR => "seccomp",
                super::WORKTREE_DESCRIPTOR => "worktree",
                other => panic!("unexpected guest descriptor {other}"),
            };
            assert!(
                property.ends_with(&format!(":{expected}:read-only")),
                "{property}"
            );
        }
    }

    #[test]
    fn manifest_binds_every_registered_executable_digest() {
        let executable = fs::canonicalize("/usr/bin/printf").expect("canonical printf");
        let executable = executable.to_str().expect("UTF-8 executable");
        let command = CommandSpec::seal(
            "fixture.printf",
            "1.0.0",
            executable,
            hash_file_for_test(executable),
            vec!["agentmage-ok".to_owned()],
            CommandWorkingDirectory::EmptyScratch,
            BTreeMap::from([
                ("LANG".to_owned(), "C".to_owned()),
                ("TZ".to_owned(), "UTC".to_owned()),
            ]),
            CommandRisk::Low,
            CommandBounds::new(1_000, 1_024, 1_024, 32 * 1024 * 1024, 4, 100).expect("limits"),
        )
        .expect("command");
        let registry = CommandRegistry::build(vec![command]).expect("registry");
        LinuxCommandManifest::verify(
            "/usr/bin/systemd-run",
            "/usr/bin/systemctl",
            "/usr/bin/bwrap",
            &registry,
        )
        .expect("verified manifest");
    }

    #[test]
    fn manifest_rejects_executable_digest_drift() {
        let executable = fs::canonicalize("/usr/bin/printf").expect("canonical printf");
        let executable = executable.to_str().expect("UTF-8 executable");
        let command = CommandSpec::seal(
            "fixture.printf",
            "1.0.0",
            executable,
            "1".repeat(64),
            vec!["agentmage-ok".to_owned()],
            CommandWorkingDirectory::EmptyScratch,
            BTreeMap::from([("LANG".to_owned(), "C".to_owned())]),
            CommandRisk::Low,
            CommandBounds::new(1_000, 1_024, 1_024, 32 * 1024 * 1024, 4, 100).expect("limits"),
        )
        .expect("command");
        let registry = CommandRegistry::build(vec![command]).expect("registry");
        assert!(
            LinuxCommandManifest::verify(
                "/usr/bin/systemd-run",
                "/usr/bin/systemctl",
                "/usr/bin/bwrap",
                &registry,
            )
            .is_err()
        );
    }

    #[test]
    #[ignore = "requires a supported Linux user systemd session and Bubblewrap"]
    fn live_exact_command_runs_offline_with_literal_output() {
        let executable = fs::canonicalize("/usr/bin/printf").expect("canonical printf");
        let executable = executable.to_str().expect("UTF-8 executable");
        let command = CommandSpec::seal(
            "fixture.printf",
            "1.0.0",
            executable,
            hash_file_for_test(executable),
            vec!["agentmage-live-ok".to_owned()],
            CommandWorkingDirectory::EmptyScratch,
            BTreeMap::from([
                ("LANG".to_owned(), "C".to_owned()),
                ("TZ".to_owned(), "UTC".to_owned()),
            ]),
            CommandRisk::Low,
            CommandBounds::new(5_000, 1_024, 4_096, 64 * 1024 * 1024, 8, 100).expect("limits"),
        )
        .expect("command");
        let registry = CommandRegistry::build(vec![command.clone()]).expect("registry");
        let manifest = LinuxCommandManifest::verify(
            "/usr/bin/systemd-run",
            "/usr/bin/systemctl",
            "/usr/bin/bwrap",
            &registry,
        )
        .expect("manifest");
        let executor = LinuxBoundedCommandExecutor::new(manifest).expect("executor");
        let cancellation = CancellationToken::root(
            BoundaryKind::Tool,
            TaskId::from_raw("task-linux-command-0001"),
            CorrelationId::from_raw("correlation-linux-command-0001"),
        );
        let (_temporary, held) = held_worktree();
        let result = executor.run(&command, &held, &cancellation);
        assert_eq!(result.termination, CommandTermination::Exited);
        assert_eq!(result.exit_code, Some(0), "{result:?}");
        assert_eq!(result.stdout, b"agentmage-live-ok");
        assert_eq!(result.stdout_total_bytes, 17);
        assert!(result.stderr_total_bytes <= command.bounds.stderr_bytes);
    }

    #[test]
    #[ignore = "requires a supported Linux user systemd session and Bubblewrap"]
    fn live_timeout_and_cancellation_terminate_the_process_unit() {
        let executable = "/usr/bin/sleep";
        let command = |attempt_timeout_ms| {
            CommandSpec::seal(
                "fixture.sleep",
                "1.0.0",
                executable,
                hash_file_for_test(executable),
                vec!["5".to_owned()],
                CommandWorkingDirectory::EmptyScratch,
                BTreeMap::from([
                    ("LANG".to_owned(), "C".to_owned()),
                    ("TZ".to_owned(), "UTC".to_owned()),
                ]),
                CommandRisk::Low,
                CommandBounds::new(attempt_timeout_ms, 1_024, 4_096, 64 * 1024 * 1024, 8, 100)
                    .expect("limits"),
            )
            .expect("command")
        };

        let timed = command(150);
        let timed_registry = CommandRegistry::build(vec![timed.clone()]).expect("registry");
        let timed_manifest = LinuxCommandManifest::verify(
            "/usr/bin/systemd-run",
            "/usr/bin/systemctl",
            "/usr/bin/bwrap",
            &timed_registry,
        )
        .expect("manifest");
        let timed_executor = LinuxBoundedCommandExecutor::new(timed_manifest).expect("executor");
        let timed_token = CancellationToken::root(
            BoundaryKind::Tool,
            TaskId::from_raw("task-linux-timeout-0001"),
            CorrelationId::from_raw("correlation-linux-timeout-0001"),
        );
        let (_timed_temporary, timed_held) = held_worktree();
        let timed_result = timed_executor.run(&timed, &timed_held, &timed_token);
        assert_eq!(timed_result.termination, CommandTermination::TimedOut);
        assert!(timed_result.descendants_terminated);
        let timed_usage = timed_result.resource_usage.expect("timed resource usage");
        assert!(timed_usage.peak_memory_bytes <= timed.bounds.memory_bytes);
        assert!(timed_usage.peak_task_count <= timed.bounds.task_count);

        let cancelled = command(5_000);
        let cancelled_registry = CommandRegistry::build(vec![cancelled.clone()]).expect("registry");
        let cancelled_manifest = LinuxCommandManifest::verify(
            "/usr/bin/systemd-run",
            "/usr/bin/systemctl",
            "/usr/bin/bwrap",
            &cancelled_registry,
        )
        .expect("manifest");
        let cancelled_executor =
            LinuxBoundedCommandExecutor::new(cancelled_manifest).expect("executor");
        let cancelled_token = CancellationToken::root(
            BoundaryKind::Tool,
            TaskId::from_raw("task-linux-cancel-0001"),
            CorrelationId::from_raw("correlation-linux-cancel-0001"),
        );
        let signal_token = cancelled_token.clone();
        let signaler = thread::spawn(move || {
            thread::sleep(Duration::from_millis(150));
            signal_token
                .cancel(CancellationSignal {
                    schema_version: CONTRACT_SCHEMA_VERSION,
                    cancellation_id: CancellationId::from_raw("cancel-linux-command-0001"),
                    correlation_id: CorrelationId::from_raw("correlation-linux-cancel-0001"),
                    task_id: TaskId::from_raw("task-linux-cancel-0001"),
                    reason: CancellationReason::UserRequested,
                    requested_by: BoundaryKind::Tool,
                })
                .expect("cancellation")
        });
        let (_cancelled_temporary, cancelled_held) = held_worktree();
        let cancelled_result =
            cancelled_executor.run(&cancelled, &cancelled_held, &cancelled_token);
        signaler.join().expect("signaler");
        assert_eq!(cancelled_result.termination, CommandTermination::Cancelled);
        assert!(cancelled_result.descendants_terminated);
        let cancelled_usage = cancelled_result
            .resource_usage
            .expect("cancelled resource usage");
        assert!(cancelled_usage.peak_memory_bytes <= cancelled.bounds.memory_bytes);
        assert!(cancelled_usage.peak_task_count <= cancelled.bounds.task_count);
    }

    #[test]
    #[ignore = "requires a supported Linux user systemd session, Bubblewrap, and OpenSSL"]
    fn live_hostile_descendant_process_tree_is_bounded_and_removed() {
        let executable = fs::canonicalize("/usr/bin/openssl").expect("canonical openssl");
        let executable = executable.to_str().expect("UTF-8 executable");
        let command = CommandSpec::seal(
            "fixture.openssl-descendants",
            "1.0.0",
            executable,
            hash_file_for_test(executable),
            vec![
                "speed".to_owned(),
                "-multi".to_owned(),
                "4".to_owned(),
                "sha256".to_owned(),
            ],
            CommandWorkingDirectory::EmptyScratch,
            BTreeMap::from([
                ("LANG".to_owned(), "C".to_owned()),
                ("TZ".to_owned(), "UTC".to_owned()),
            ]),
            CommandRisk::Moderate,
            CommandBounds::new(500, 1_024, 4_096, 64 * 1024 * 1024, 8, 100).expect("limits"),
        )
        .expect("command");
        let registry = CommandRegistry::build(vec![command.clone()]).expect("registry");
        let manifest = LinuxCommandManifest::verify(
            "/usr/bin/systemd-run",
            "/usr/bin/systemctl",
            "/usr/bin/bwrap",
            &registry,
        )
        .expect("manifest");
        let executor = LinuxBoundedCommandExecutor::new(manifest).expect("executor");
        let cancellation = CancellationToken::root(
            BoundaryKind::Tool,
            TaskId::from_raw("task-linux-descendants-0001"),
            CorrelationId::from_raw("correlation-linux-descendants-0001"),
        );
        let (_temporary, held) = held_worktree();
        let result = executor.run(&command, &held, &cancellation);
        assert_eq!(
            result.termination,
            CommandTermination::TimedOut,
            "{result:?}"
        );
        assert!(result.descendants_terminated, "{result:?}");
        let usage = result.resource_usage.expect("descendant resource usage");
        assert!(usage.cpu_time_ns > 0, "{usage:?}");
        assert!(usage.peak_memory_bytes <= command.bounds.memory_bytes);
        assert!((3..=command.bounds.task_count).contains(&usage.peak_task_count));
    }

    /// Seals a single-command registry and executor for one empty-scratch fixture.
    fn scratch_fixture(
        template_id: &str,
        executable: &str,
        arguments: Vec<String>,
        risk: CommandRisk,
    ) -> (CommandSpec, LinuxBoundedCommandExecutor) {
        command_fixture(
            template_id,
            executable,
            arguments,
            CommandWorkingDirectory::EmptyScratch,
            risk,
        )
    }

    /// Seals a single-command registry and executor for one owned-worktree fixture.
    fn owned_worktree_fixture(
        template_id: &str,
        executable: &str,
        arguments: Vec<String>,
        risk: CommandRisk,
    ) -> (CommandSpec, LinuxBoundedCommandExecutor) {
        command_fixture(
            template_id,
            executable,
            arguments,
            CommandWorkingDirectory::OwnedWorktree,
            risk,
        )
    }

    fn command_fixture(
        template_id: &str,
        executable: &str,
        arguments: Vec<String>,
        working_directory: CommandWorkingDirectory,
        risk: CommandRisk,
    ) -> (CommandSpec, LinuxBoundedCommandExecutor) {
        let command = CommandSpec::seal(
            template_id,
            "1.0.0",
            executable,
            hash_file_for_test(executable),
            arguments,
            working_directory,
            BTreeMap::from([
                ("LANG".to_owned(), "C".to_owned()),
                ("TZ".to_owned(), "UTC".to_owned()),
            ]),
            risk,
            CommandBounds::new(5_000, 4_096, 4_096, 64 * 1024 * 1024, 8, 100).expect("limits"),
        )
        .expect("command");
        let registry = CommandRegistry::build(vec![command.clone()]).expect("registry");
        let manifest = LinuxCommandManifest::verify(
            "/usr/bin/systemd-run",
            "/usr/bin/systemctl",
            "/usr/bin/bwrap",
            &registry,
        )
        .expect("manifest");
        let executor = LinuxBoundedCommandExecutor::new(manifest).expect("executor");
        (command, executor)
    }

    fn worktree_token(suffix: &str) -> CancellationToken {
        CancellationToken::root(
            BoundaryKind::Tool,
            TaskId::from_raw(format!("task-linux-worktree-{suffix}")),
            CorrelationId::from_raw(format!("correlation-linux-worktree-{suffix}")),
        )
    }

    #[test]
    #[ignore = "requires a supported Linux user systemd session and Bubblewrap"]
    fn live_guest_environment_holds_only_the_sealed_template_variables() {
        let executable = fs::canonicalize("/usr/bin/printenv").expect("canonical printenv");
        let executable = executable.to_str().expect("UTF-8 executable");
        let (command, executor) =
            scratch_fixture("fixture.printenv", executable, Vec::new(), CommandRisk::Low);
        // The ambient host environment carries these; none may cross the boundary.
        let ambient: Vec<String> = ["HOME", "PATH", "USER", "XDG_RUNTIME_DIR"]
            .into_iter()
            .filter_map(|name| std::env::var(name).ok())
            .collect();
        assert!(
            !ambient.is_empty(),
            "host environment is unexpectedly empty"
        );
        let (_temporary, held) = held_worktree();
        let result = executor.run(&command, &held, &worktree_token("environment-0001"));
        assert_eq!(result.termination, CommandTermination::Exited, "{result:?}");
        assert_eq!(result.exit_code, Some(0), "{result:?}");
        let observed = String::from_utf8(result.stdout.clone()).expect("UTF-8 environment");
        // Bubblewrap sets PWD from the fixed guest working directory; it names no host path.
        let mut names: Vec<&str> = observed.lines().collect();
        names.sort_unstable();
        assert_eq!(names, ["LANG=C", "PWD=/work", "TZ=UTC"], "{observed}");
        for value in &ambient {
            assert!(!observed.contains(value.as_str()), "{observed}");
        }
    }

    /// The guest mounts only `/app/command` and read-only runtime libraries, so an
    /// approved template cannot reach a second executable to exec. This closes
    /// pager, editor, hook, and helper-binary activation at the mount boundary and
    /// makes an exec-built grandchild tree unreachable rather than merely denied.
    #[test]
    #[ignore = "requires a supported Linux user systemd session and Bubblewrap"]
    fn live_guest_cannot_execute_any_second_program() {
        let executable = fs::canonicalize("/usr/bin/timeout").expect("canonical timeout");
        let executable = executable.to_str().expect("UTF-8 executable");
        let (command, executor) = scratch_fixture(
            "fixture.second-program-denied",
            executable,
            vec![
                "60".to_owned(),
                "/usr/bin/sleep".to_owned(),
                "60".to_owned(),
            ],
            CommandRisk::Moderate,
        );
        let (_temporary, held) = held_worktree();
        let result = executor.run(&command, &held, &worktree_token("second-program-0001"));
        assert_eq!(result.termination, CommandTermination::Exited, "{result:?}");
        assert_eq!(result.exit_code, Some(127), "{result:?}");
        let stderr = String::from_utf8_lossy(&result.stderr);
        assert!(stderr.contains("No such file or directory"), "{stderr}");
        assert!(result.descendants_terminated, "{result:?}");
        // A command this short can exit before the sampler observes the unit at all.
        if let Some(usage) = result.resource_usage {
            assert!(
                usage.peak_task_count <= command.bounds.task_count,
                "{usage:?}"
            );
        }
    }

    #[test]
    #[ignore = "requires a supported Linux user systemd session and Bubblewrap"]
    fn live_empty_scratch_retains_no_residue_between_attempts() {
        let write_executable = fs::canonicalize("/usr/bin/touch").expect("canonical touch");
        let write_executable = write_executable.to_str().expect("UTF-8 executable");
        let (write_command, write_executor) = scratch_fixture(
            "fixture.scratch-write",
            write_executable,
            vec!["residue.txt".to_owned()],
            CommandRisk::Moderate,
        );
        let (_temporary, held) = held_worktree();
        let written = write_executor.run(&write_command, &held, &worktree_token("scratch-write"));
        assert_eq!(
            written.termination,
            CommandTermination::Exited,
            "{written:?}"
        );
        assert_eq!(written.exit_code, Some(0), "{written:?}");

        let list_executable = fs::canonicalize("/usr/bin/ls").expect("canonical ls");
        let list_executable = list_executable.to_str().expect("UTF-8 executable");
        let (list_command, list_executor) = scratch_fixture(
            "fixture.scratch-list",
            list_executable,
            vec!["-A".to_owned()],
            CommandRisk::Low,
        );
        let listed = list_executor.run(&list_command, &held, &worktree_token("scratch-list"));
        assert_eq!(listed.termination, CommandTermination::Exited, "{listed:?}");
        assert_eq!(listed.exit_code, Some(0), "{listed:?}");
        assert!(listed.stdout.is_empty(), "{listed:?}");
    }

    #[test]
    #[ignore = "requires a supported Linux user systemd session and Bubblewrap"]
    fn live_owned_worktree_is_descriptor_bound_at_the_guest_working_directory() {
        let executable = fs::canonicalize("/usr/bin/cat").expect("canonical cat");
        let executable = executable.to_str().expect("UTF-8 executable");
        let (command, executor) = owned_worktree_fixture(
            "fixture.worktree-marker",
            executable,
            vec!["marker.txt".to_owned()],
            CommandRisk::Low,
        );
        let (_temporary, held) = held_worktree();
        let result = executor.run(&command, &held, &worktree_token("marker-0001"));
        assert_eq!(result.termination, CommandTermination::Exited, "{result:?}");
        assert_eq!(result.exit_code, Some(0), "{result:?}");
        assert_eq!(result.stdout, b"agentmage", "{result:?}");
    }

    #[test]
    #[ignore = "requires a supported Linux user systemd session and Bubblewrap"]
    fn live_owned_worktree_command_cannot_mutate_the_held_directory() {
        let (temporary, held) = held_worktree();

        let marker_executable = fs::canonicalize("/usr/bin/cat").expect("canonical cat");
        let marker_executable = marker_executable.to_str().expect("UTF-8 executable");
        let (marker_command, marker_executor) = owned_worktree_fixture(
            "fixture.worktree-marker",
            marker_executable,
            vec!["marker.txt".to_owned()],
            CommandRisk::Low,
        );
        let marker = marker_executor.run(&marker_command, &held, &worktree_token("denial-read"));
        assert_eq!(marker.termination, CommandTermination::Exited, "{marker:?}");
        assert_eq!(marker.exit_code, Some(0), "{marker:?}");
        assert_eq!(marker.stdout, b"agentmage", "{marker:?}");

        let write_executable = fs::canonicalize("/usr/bin/touch").expect("canonical touch");
        let write_executable = write_executable.to_str().expect("UTF-8 executable");
        let (write_command, write_executor) = owned_worktree_fixture(
            "fixture.worktree-write-denied",
            write_executable,
            vec!["forbidden.txt".to_owned()],
            CommandRisk::Moderate,
        );
        let result = write_executor.run(&write_command, &held, &worktree_token("denial-write"));
        assert_eq!(result.termination, CommandTermination::Exited, "{result:?}");
        assert_ne!(result.exit_code, Some(0), "{result:?}");
        let stderr = String::from_utf8_lossy(&result.stderr);
        assert!(stderr.contains("Read-only file system"), "{stderr}");
        assert!(!temporary.0.join("owned-worktree/forbidden.txt").exists());
    }

    #[test]
    #[ignore = "requires a supported Linux user systemd session and Bubblewrap"]
    fn live_owned_worktree_guest_sees_no_inherited_descriptors() {
        let executable = fs::canonicalize("/usr/bin/ls").expect("canonical ls");
        let executable = executable.to_str().expect("UTF-8 executable");
        let (command, executor) = owned_worktree_fixture(
            "fixture.worktree-fd-enumeration",
            executable,
            vec!["-l".to_owned(), "/proc/self/fd".to_owned()],
            CommandRisk::Low,
        );
        let (_temporary, held) = held_worktree();
        let result = executor.run(&command, &held, &worktree_token("descriptors-0001"));
        assert_eq!(result.termination, CommandTermination::Exited, "{result:?}");
        assert_eq!(result.exit_code, Some(0), "{result:?}");
        let listing = String::from_utf8(result.stdout.clone()).expect("UTF-8 guest listing");
        assert_exact_guest_descriptor_table(&listing);
    }

    #[test]
    #[ignore = "requires a supported Linux user systemd session and Bubblewrap"]
    fn live_changed_held_worktree_identity_fails_before_launch() {
        let executable = fs::canonicalize("/usr/bin/cat").expect("canonical cat");
        let executable = executable.to_str().expect("UTF-8 executable");
        let (command, executor) = owned_worktree_fixture(
            "fixture.worktree-marker",
            executable,
            vec!["marker.txt".to_owned()],
            CommandRisk::Low,
        );
        let (temporary, held) = held_worktree();
        let root = temporary.0.join("owned-worktree");
        fs::set_permissions(&root, fs::Permissions::from_mode(0o700))
            .expect("worktree mode changes");
        let result = executor.run(&command, &held, &worktree_token("identity-0001"));
        assert_eq!(
            result.termination,
            CommandTermination::LaunchFailed,
            "{result:?}"
        );
        assert_eq!(result.platform_code, "linux.command.worktree.changed");
        assert!(result.stdout.is_empty(), "{result:?}");
        assert!(result.resource_usage.is_none(), "{result:?}");
    }

    #[test]
    #[ignore = "requires a supported Linux user systemd session and Bubblewrap"]
    fn live_inherited_descriptors_do_not_reach_the_command_guest() {
        let temporary = TestDirectory::new();
        let secret_path = temporary.0.join("inherited-secret.txt");
        fs::write(&secret_path, b"agentmage-inherited-secret").expect("secret writes");
        let leaked_file =
            open(&secret_path, OFlags::RDWR, Mode::empty()).expect("secret descriptor opens");
        let leaked_directory = open(
            &temporary.0,
            OFlags::RDONLY | OFlags::DIRECTORY,
            Mode::empty(),
        )
        .expect("directory descriptor opens");
        for leaked in [&leaked_file, &leaked_directory] {
            assert!(
                !fcntl_getfd(leaked)
                    .expect("descriptor flags")
                    .contains(FdFlags::CLOEXEC)
            );
        }

        let control = Command::new("/usr/bin/ls")
            .env_clear()
            .args(["-l", "/proc/self/fd"])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .output()
            .expect("control child runs");
        assert!(control.status.success(), "{control:?}");
        let control_listing = String::from_utf8_lossy(&control.stdout);
        assert!(
            control_listing.contains("inherited-secret.txt"),
            "{control_listing}"
        );

        let executable = fs::canonicalize("/usr/bin/ls").expect("canonical ls");
        let executable = executable.to_str().expect("UTF-8 executable");
        let command = CommandSpec::seal(
            "fixture.fd-enumeration",
            "1.0.0",
            executable,
            hash_file_for_test(executable),
            vec!["-l".to_owned(), "/proc/self/fd".to_owned()],
            CommandWorkingDirectory::EmptyScratch,
            BTreeMap::from([
                ("LANG".to_owned(), "C".to_owned()),
                ("TZ".to_owned(), "UTC".to_owned()),
            ]),
            CommandRisk::Low,
            CommandBounds::new(5_000, 4_096, 4_096, 64 * 1024 * 1024, 8, 100).expect("limits"),
        )
        .expect("command");
        let registry = CommandRegistry::build(vec![command.clone()]).expect("registry");
        let manifest = LinuxCommandManifest::verify(
            "/usr/bin/systemd-run",
            "/usr/bin/systemctl",
            "/usr/bin/bwrap",
            &registry,
        )
        .expect("manifest");
        let executor = LinuxBoundedCommandExecutor::new(manifest).expect("executor");
        let cancellation = CancellationToken::root(
            BoundaryKind::Tool,
            TaskId::from_raw("task-linux-descriptors-0001"),
            CorrelationId::from_raw("correlation-linux-descriptors-0001"),
        );
        let (_worktree_temporary, held) = held_worktree();
        let result = executor.run(&command, &held, &cancellation);
        assert_eq!(result.termination, CommandTermination::Exited, "{result:?}");
        assert_eq!(result.exit_code, Some(0), "{result:?}");

        let listing = String::from_utf8(result.stdout.clone()).expect("UTF-8 guest listing");
        assert!(!listing.contains("inherited-secret"), "{listing}");
        let temporary_name = temporary
            .0
            .file_name()
            .and_then(|name| name.to_str())
            .expect("temporary name");
        assert!(!listing.contains(temporary_name), "{listing}");

        assert_exact_guest_descriptor_table(&listing);
    }

    /// Requires exactly stdin, stdout, stderr, and the enumerator's own descriptor.
    /// Bubblewrap consumes every `OpenFile=` descriptor while building the guest
    /// mounts, so none of them may remain visible to the executed command.
    fn assert_exact_guest_descriptor_table(listing: &str) {
        let mut lines = listing.lines();
        let header = lines.next().expect("listing header");
        assert!(header.starts_with("total "), "{listing}");
        let enumerated: Vec<(u32, &str)> = lines
            .map(|line| {
                let (left, target) = line
                    .split_once(" -> ")
                    .unwrap_or_else(|| panic!("unexpected listing line: {line}"));
                let descriptor = left
                    .rsplit(' ')
                    .next()
                    .expect("descriptor column")
                    .parse()
                    .unwrap_or_else(|_| panic!("unexpected descriptor column: {line}"));
                (descriptor, target)
            })
            .collect();
        assert_eq!(enumerated.len(), 4, "{listing}");
        let mut descriptors: Vec<u32> = enumerated
            .iter()
            .map(|(descriptor, _)| *descriptor)
            .collect();
        descriptors.sort_unstable();
        assert_eq!(descriptors[..3], [0, 1, 2], "{listing}");
        assert!(descriptors[3] > 2, "{listing}");
        for (descriptor, target) in &enumerated {
            match descriptor {
                0 => assert_eq!(*target, "/dev/null", "{listing}"),
                1 | 2 => {
                    let inode = target
                        .strip_prefix("pipe:[")
                        .and_then(|rest| rest.strip_suffix(']'))
                        .unwrap_or_else(|| panic!("unexpected stream target: {target}"));
                    assert!(
                        !inode.is_empty() && inode.bytes().all(|byte| byte.is_ascii_digit()),
                        "{listing}"
                    );
                }
                _ => {
                    let owner = target
                        .strip_prefix("/proc/")
                        .and_then(|rest| rest.strip_suffix("/fd"))
                        .unwrap_or_else(|| {
                            panic!("undeclared guest descriptor {descriptor} -> {target}")
                        });
                    assert!(
                        !owner.is_empty() && owner.bytes().all(|byte| byte.is_ascii_digit()),
                        "{listing}"
                    );
                }
            }
        }
    }

    fn hash_file_for_test(path: &str) -> String {
        super::hex(&sha2::Sha256::digest(
            fs::read(path).expect("fixture executable"),
        ))
    }
}
