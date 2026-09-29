//! Explicit development-only process and private-state effects.

use std::env;
use std::fs::{self, OpenOptions};
use std::io::{self, Read, Write};
use std::os::fd::AsFd;
use std::os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt};
use std::path::Path;
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::{
    Arc, LazyLock,
    atomic::{AtomicBool, AtomicU8, Ordering},
};
use std::time::{Duration, Instant};

use rustix::fs::{OFlags, fcntl_getfl, fcntl_setfl};

use crate::ipc::LinuxLaunchEnvelope;

#[path = "development_boundary/confirmation.rs"]
mod confirmation;
pub use confirmation::{
    LinuxDevelopmentConfirmation, LinuxDevelopmentConfirmationKind, LinuxDevelopmentInputLine,
    read_development_confirmation, read_development_line,
};

const MAX_DEVELOPMENT_HOST_BYTES: u64 = 512 * 1024 * 1024;
const STARTUP_TIME: Duration = Duration::from_secs(120);
const EXIT_TIME: Duration = Duration::from_secs(10);
const CLEANUP_TIME: Duration = Duration::from_secs(3);
const POLL_TIME: Duration = Duration::from_millis(10);
const SLOT_AVAILABLE: u8 = 0;
const SLOT_OWNED: u8 = 1;
const SLOT_UNCERTAIN: u8 = 2;
static HOST_SLOT: LazyLock<Arc<AtomicU8>> =
    LazyLock::new(|| Arc::new(AtomicU8::new(SLOT_AVAILABLE)));

/// Stable content-free failure from the explicit Linux development boundary.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LinuxDevelopmentBoundaryErrorKind {
    /// A caller supplied a value outside the closed development contract.
    InvalidInput,
    /// The bounded local confirmation input was malformed or unavailable.
    ConfirmationInputFailed,
    /// The exact sibling host executable was absent, linked, or unsafe.
    UnsafeExecutable,
    /// The sibling host could not be launched.
    LaunchFailed,
    /// An existing or uncertain direct host owner prevents another launch.
    OwnerUnavailable,
    /// The original startup deadline expired before a complete envelope arrived.
    TransferTimedOut,
    /// A local stop request cancelled startup before a complete envelope was accepted.
    StartupCancelled,
    /// The direct child did not exit within its graceful shutdown deadline.
    ExitTimedOut,
    /// Exact direct-child cleanup could not be established; replacement is refused.
    CleanupUncertain,
    /// The direct one-use launch envelope could not be read.
    TransferFailed,
    /// The child could not be terminated or reaped.
    ProcessControlFailed,
    /// A private development directory was absent or unsafe.
    UnsafeDirectory,
    /// A bounded owner-only development record could not be retained.
    RetentionFailed,
}

impl LinuxDevelopmentBoundaryErrorKind {
    /// Returns a stable code without paths, process identities or child output.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidInput => "linux.development.input.invalid",
            Self::ConfirmationInputFailed => "linux.development.confirmation-input.failed",
            Self::UnsafeExecutable => "linux.development.host-executable.unsafe",
            Self::LaunchFailed => "linux.development.host-launch.failed",
            Self::OwnerUnavailable => "linux.development.owner.unavailable",
            Self::TransferTimedOut => "linux.development.launch-envelope.timed-out",
            Self::StartupCancelled => "linux.development.startup.cancelled",
            Self::ExitTimedOut => "linux.development.host-exit.timed-out",
            Self::CleanupUncertain => "linux.development.cleanup.uncertain",
            Self::TransferFailed => "linux.development.launch-envelope.failed",
            Self::ProcessControlFailed => "linux.development.process-control.failed",
            Self::UnsafeDirectory => "linux.development.directory.unsafe",
            Self::RetentionFailed => "linux.development.retention.failed",
        }
    }
}

/// Content-free Linux development boundary error.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LinuxDevelopmentBoundaryError {
    kind: LinuxDevelopmentBoundaryErrorKind,
}

impl LinuxDevelopmentBoundaryError {
    /// Returns the stable failure class.
    #[must_use]
    pub const fn kind(&self) -> LinuxDevelopmentBoundaryErrorKind {
        self.kind
    }
}

// One process-local reservation, held until reaping rather than a kill request.
// The token is private and non-cloneable; tests can use isolated slot instances.
struct HostAdmission {
    slot: Arc<AtomicU8>,
    may_release: bool,
}

impl HostAdmission {
    fn reserve(slot: Arc<AtomicU8>) -> Result<Self, LinuxDevelopmentBoundaryError> {
        slot.compare_exchange(
            SLOT_AVAILABLE,
            SLOT_OWNED,
            Ordering::AcqRel,
            Ordering::Acquire,
        )
        .map_err(|_| development_error(LinuxDevelopmentBoundaryErrorKind::OwnerUnavailable))?;
        Ok(Self {
            slot,
            may_release: true,
        })
    }

    fn launched(&mut self) {
        self.may_release = false;
    }

    fn reaped(&mut self) {
        self.may_release = true;
    }

    fn quarantine(&mut self) {
        self.may_release = false;
        self.slot.store(SLOT_UNCERTAIN, Ordering::Release);
    }
}

impl Drop for HostAdmission {
    fn drop(&mut self) {
        if self.may_release {
            // An uncertain slot never becomes available through a late drop.
            let _ = self.slot.compare_exchange(
                SLOT_OWNED,
                SLOT_AVAILABLE,
                Ordering::AcqRel,
                Ordering::Acquire,
            );
        }
    }
}

// The existing envelope parser remains the only framing/credential decoder.
// Every read uses the same original deadline, including interrupted/partial reads.
struct DeadlineReader<'a, R> {
    input: R,
    deadline: Instant,
    timed_out: bool,
    cancellation: Option<&'a AtomicBool>,
    cancellation_observed: bool,
}

impl<'a, R: Read + AsFd> DeadlineReader<'a, R> {
    fn new(input: R, deadline: Instant, cancellation: Option<&'a AtomicBool>) -> io::Result<Self> {
        let flags = fcntl_getfl(&input)?;
        fcntl_setfl(&input, flags | OFlags::NONBLOCK)?;
        Ok(Self {
            input,
            deadline,
            timed_out: false,
            cancellation,
            cancellation_observed: false,
        })
    }
}

impl<R: Read> Read for DeadlineReader<'_, R> {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        if buffer.is_empty() {
            return Ok(0);
        }
        loop {
            if self
                .cancellation
                .is_some_and(|flag| flag.load(Ordering::Acquire))
            {
                self.cancellation_observed = true;
                // read_exact retries Interrupted, so use a terminal private-reader error.
                return Err(io::ErrorKind::ConnectionAborted.into());
            }
            if Instant::now() >= self.deadline {
                self.timed_out = true;
                return Err(io::ErrorKind::TimedOut.into());
            }
            match self.input.read(buffer) {
                Err(error)
                    if matches!(
                        error.kind(),
                        io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted
                    ) =>
                {
                    std::thread::sleep(
                        POLL_TIME.min(self.deadline.saturating_duration_since(Instant::now())),
                    );
                }
                result => return result,
            }
        }
    }
}

fn read_envelope_before(
    input: impl Read + AsFd,
    deadline: Instant,
) -> Result<LinuxLaunchEnvelope, LinuxDevelopmentBoundaryError> {
    read_envelope_before_cancellable(input, deadline, None)
}

fn read_envelope_before_cancellable(
    input: impl Read + AsFd,
    deadline: Instant,
    cancellation: Option<&AtomicBool>,
) -> Result<LinuxLaunchEnvelope, LinuxDevelopmentBoundaryError> {
    let mut reader = DeadlineReader::new(input, deadline, cancellation)
        .map_err(|_| development_error(LinuxDevelopmentBoundaryErrorKind::TransferFailed))?;
    LinuxLaunchEnvelope::read(&mut reader).map_err(|_| {
        development_error(if reader.cancellation_observed {
            LinuxDevelopmentBoundaryErrorKind::StartupCancelled
        } else if reader.timed_out {
            LinuxDevelopmentBoundaryErrorKind::TransferTimedOut
        } else {
            LinuxDevelopmentBoundaryErrorKind::TransferFailed
        })
    })
}

/// Platform-owned handle for one exact sibling development host.
pub struct LinuxDevelopmentHostProcess {
    child: Option<Child>,
    status: Option<ExitStatus>,
    startup_deadline: Instant,
    admission: HostAdmission,
    cleanup_uncertain: bool,
    exit_timed_out: bool,
}

impl LinuxDevelopmentHostProcess {
    /// Launches the exact sibling host with the closed development operation.
    pub fn launch(
        state_root: &Path,
        disposable_root: &Path,
        workspace_root: &Path,
        scenario: &str,
        model: &str,
        resume: bool,
    ) -> Result<Self, LinuxDevelopmentBoundaryError> {
        if !development_scenario_valid(scenario)
            || !matches!(model, "scripted" | "muse" | "gpt-oss")
            || [state_root, disposable_root, workspace_root]
                .iter()
                .any(|path| !path.is_absolute())
        {
            return Err(development_error(
                LinuxDevelopmentBoundaryErrorKind::InvalidInput,
            ));
        }
        let mut admission = HostAdmission::reserve(Arc::clone(&HOST_SLOT))?;
        let startup_deadline = Instant::now()
            .checked_add(STARTUP_TIME)
            .ok_or_else(|| development_error(LinuxDevelopmentBoundaryErrorKind::LaunchFailed))?;
        let current = env::current_exe()
            .and_then(fs::canonicalize)
            .map_err(|_| development_error(LinuxDevelopmentBoundaryErrorKind::UnsafeExecutable))?;
        let sibling = current.with_file_name("agentmage-host");
        let host = fs::canonicalize(&sibling)
            .map_err(|_| development_error(LinuxDevelopmentBoundaryErrorKind::UnsafeExecutable))?;
        if host != sibling {
            return Err(development_error(
                LinuxDevelopmentBoundaryErrorKind::UnsafeExecutable,
            ));
        }
        let metadata = fs::symlink_metadata(&host)
            .map_err(|_| development_error(LinuxDevelopmentBoundaryErrorKind::UnsafeExecutable))?;
        if !metadata.file_type().is_file()
            || metadata.uid() != rustix::process::getuid().as_raw()
            || metadata.mode() & 0o111 == 0
            || metadata.len() == 0
            || metadata.len() > MAX_DEVELOPMENT_HOST_BYTES
        {
            return Err(development_error(
                LinuxDevelopmentBoundaryErrorKind::UnsafeExecutable,
            ));
        }
        let child = Command::new(host)
            .arg("--coding-development-host")
            .arg(state_root)
            .arg(disposable_root)
            .arg(workspace_root)
            .arg(scenario)
            .arg(model)
            .arg(if resume { "resume" } else { "new" })
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .map_err(|_| development_error(LinuxDevelopmentBoundaryErrorKind::LaunchFailed))?;
        admission.launched();
        Ok(Self {
            child: Some(child),
            status: None,
            startup_deadline,
            admission,
            cleanup_uncertain: false,
            exit_timed_out: false,
        })
    }

    /// Reads the one-use launch envelope within the original startup deadline.
    pub fn read_launch_envelope(
        &mut self,
    ) -> Result<LinuxLaunchEnvelope, LinuxDevelopmentBoundaryError> {
        let output = self.take_launch_output()?;
        read_envelope_before(output, self.startup_deadline)
    }

    /// Reads the one-use envelope while observing a local stop flag without resetting it.
    pub fn read_launch_envelope_cancellable(
        &mut self,
        cancellation: &AtomicBool,
    ) -> Result<LinuxLaunchEnvelope, LinuxDevelopmentBoundaryError> {
        let output = self.take_launch_output()?;
        read_envelope_before_cancellable(output, self.startup_deadline, Some(cancellation))
    }

    fn take_launch_output(
        &mut self,
    ) -> Result<std::process::ChildStdout, LinuxDevelopmentBoundaryError> {
        if self.cleanup_uncertain {
            return Err(development_error(
                LinuxDevelopmentBoundaryErrorKind::CleanupUncertain,
            ));
        }
        self.child
            .as_mut()
            .and_then(|child| child.stdout.take())
            .ok_or_else(|| development_error(LinuxDevelopmentBoundaryErrorKind::TransferFailed))
    }

    fn uncertain(&mut self) -> LinuxDevelopmentBoundaryError {
        self.cleanup_uncertain = true;
        self.admission.quarantine();
        development_error(LinuxDevelopmentBoundaryErrorKind::CleanupUncertain)
    }

    fn observe_exit(&mut self) -> Result<Option<ExitStatus>, LinuxDevelopmentBoundaryError> {
        if self.cleanup_uncertain {
            return Err(development_error(
                LinuxDevelopmentBoundaryErrorKind::CleanupUncertain,
            ));
        }
        if let Some(status) = self.status {
            return Ok(Some(status));
        }
        let observed = self
            .child
            .as_mut()
            .ok_or_else(|| {
                development_error(LinuxDevelopmentBoundaryErrorKind::ProcessControlFailed)
            })?
            .try_wait();
        match observed {
            Ok(Some(status)) => {
                self.status = Some(status);
                self.admission.reaped();
                Ok(Some(status))
            }
            Ok(None) => Ok(None),
            // An unexpected wait failure cannot justify signalling a possibly
            // no-longer-owned process identity or admitting a replacement.
            Err(_) => Err(self.uncertain()),
        }
    }

    /// Requests termination of the exact owned child; this alone is not cleanup.
    pub fn terminate(&mut self) -> Result<(), LinuxDevelopmentBoundaryError> {
        if self.observe_exit()?.is_some() {
            return Ok(());
        }
        let child = self.child.as_mut().ok_or_else(|| {
            development_error(LinuxDevelopmentBoundaryErrorKind::ProcessControlFailed)
        })?;
        if child.kill().is_ok() || self.observe_exit()?.is_some() {
            Ok(())
        } else {
            Err(development_error(
                LinuxDevelopmentBoundaryErrorKind::ProcessControlFailed,
            ))
        }
    }

    fn wait_before(
        &mut self,
        deadline: Instant,
    ) -> Result<Option<ExitStatus>, LinuxDevelopmentBoundaryError> {
        if self.cleanup_uncertain {
            return Err(development_error(
                LinuxDevelopmentBoundaryErrorKind::CleanupUncertain,
            ));
        }
        if self.status.is_some() {
            return Ok(self.status);
        }
        while Instant::now() < deadline {
            if let Some(status) = self.observe_exit()? {
                return Ok(Some(status));
            }
            std::thread::sleep(POLL_TIME.min(deadline.saturating_duration_since(Instant::now())));
        }
        Ok(None)
    }

    fn cleanup_before(&mut self, deadline: Instant) -> Result<(), LinuxDevelopmentBoundaryError> {
        // A refused signal may race a natural exit. Only reaping establishes
        // cleanup; a wait failure above has already quarantined this owner.
        let _ = self.terminate();
        match self.wait_before(deadline) {
            Ok(Some(_)) => Ok(()),
            Ok(None) | Err(_) => Err(self.uncertain()),
        }
    }

    /// Terminates and reaps this direct child within the fixed cleanup budget.
    pub fn terminate_and_reap(&mut self) -> Result<(), LinuxDevelopmentBoundaryError> {
        let deadline = Instant::now()
            .checked_add(CLEANUP_TIME)
            .ok_or_else(|| self.uncertain())?;
        self.cleanup_before(deadline)
    }

    /// Waits for bounded graceful exit; late/forced exit is never success.
    pub fn wait_success(&mut self) -> Result<bool, LinuxDevelopmentBoundaryError> {
        self.wait_success_with_limits(EXIT_TIME, CLEANUP_TIME)
    }

    fn wait_success_with_limits(
        &mut self,
        graceful_time: Duration,
        cleanup_time: Duration,
    ) -> Result<bool, LinuxDevelopmentBoundaryError> {
        if self.cleanup_uncertain {
            return Err(development_error(
                LinuxDevelopmentBoundaryErrorKind::CleanupUncertain,
            ));
        }
        if self.exit_timed_out {
            return Err(development_error(
                LinuxDevelopmentBoundaryErrorKind::ExitTimedOut,
            ));
        }
        let deadline = Instant::now()
            .checked_add(graceful_time)
            .ok_or_else(|| self.uncertain())?;
        if let Some(status) = self.wait_before(deadline)? {
            return Ok(status.success());
        }
        self.exit_timed_out = true;
        let cleanup_deadline = Instant::now()
            .checked_add(cleanup_time)
            .ok_or_else(|| self.uncertain())?;
        self.cleanup_before(cleanup_deadline)?;
        Err(development_error(
            LinuxDevelopmentBoundaryErrorKind::ExitTimedOut,
        ))
    }
}

impl Drop for LinuxDevelopmentHostProcess {
    fn drop(&mut self) {
        if self.status.is_some() && !self.cleanup_uncertain {
            return;
        }
        if !self.cleanup_uncertain && self.terminate_and_reap().is_ok() {
            return;
        }
        self.uncertain();
        if let Some(child) = self.child.take() {
            // At most one: the process-local slot remains quarantined. Keep its
            // exact handle/descriptors until process exit rather than announce
            // cleanup, signal a replacement PID or detach an unbounded reaper.
            std::mem::forget(child);
        }
        eprintln!(
            "{}",
            LinuxDevelopmentBoundaryErrorKind::CleanupUncertain.code()
        );
    }
}

/// Creates or verifies one exact owner-only development directory.
pub fn ensure_private_development_directory(
    path: &Path,
) -> Result<(), LinuxDevelopmentBoundaryError> {
    if !path.is_absolute() {
        return Err(development_error(
            LinuxDevelopmentBoundaryErrorKind::UnsafeDirectory,
        ));
    }
    let parent = path
        .parent()
        .ok_or_else(|| development_error(LinuxDevelopmentBoundaryErrorKind::UnsafeDirectory))?;
    verify_private_directory(parent)?;
    match fs::create_dir(path) {
        Ok(()) => fs::set_permissions(path, fs::Permissions::from_mode(0o700))
            .map_err(|_| development_error(LinuxDevelopmentBoundaryErrorKind::UnsafeDirectory))?,
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
        Err(_) => {
            return Err(development_error(
                LinuxDevelopmentBoundaryErrorKind::UnsafeDirectory,
            ));
        }
    }
    verify_private_directory(path)
}

/// Retains one rejected development-model response and its bounded metadata.
///
/// Both files are new owner-only ordinary files. A metadata failure removes only
/// the raw file created by this call; existing or substituted paths are never
/// overwritten or recursively removed.
pub fn retain_rejected_development_output(
    root: &Path,
    identity: &str,
    response: &[u8],
    metadata: &[u8],
) -> Result<(), LinuxDevelopmentBoundaryError> {
    ensure_private_development_directory(root)?;
    if identity.len() != 24 || !identity.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(development_error(
            LinuxDevelopmentBoundaryErrorKind::InvalidInput,
        ));
    }
    let raw_path = root.join(format!("{identity}.response.bin"));
    let metadata_path = root.join(format!("{identity}.metadata.json"));
    write_private_new(&raw_path, response)?;
    if let Err(error) = write_private_new(&metadata_path, metadata) {
        let _ = fs::remove_file(&raw_path);
        return Err(error);
    }
    Ok(())
}

fn verify_private_directory(path: &Path) -> Result<(), LinuxDevelopmentBoundaryError> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|_| development_error(LinuxDevelopmentBoundaryErrorKind::UnsafeDirectory))?;
    if !metadata.file_type().is_dir()
        || metadata.uid() != rustix::process::getuid().as_raw()
        || metadata.mode() & 0o777 != 0o700
    {
        return Err(development_error(
            LinuxDevelopmentBoundaryErrorKind::UnsafeDirectory,
        ));
    }
    Ok(())
}

fn write_private_new(path: &Path, bytes: &[u8]) -> Result<(), LinuxDevelopmentBoundaryError> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)
        .map_err(|_| development_error(LinuxDevelopmentBoundaryErrorKind::RetentionFailed))?;
    if file
        .write_all(bytes)
        .and_then(|()| file.sync_all())
        .is_err()
    {
        drop(file);
        let _ = fs::remove_file(path);
        return Err(development_error(
            LinuxDevelopmentBoundaryErrorKind::RetentionFailed,
        ));
    }
    let metadata = file
        .metadata()
        .map_err(|_| development_error(LinuxDevelopmentBoundaryErrorKind::RetentionFailed))?;
    if !metadata.file_type().is_file()
        || metadata.uid() != rustix::process::getuid().as_raw()
        || metadata.mode() & 0o777 != 0o600
        || metadata.nlink() != 1
        || metadata.len() != bytes.len() as u64
    {
        drop(file);
        let _ = fs::remove_file(path);
        return Err(development_error(
            LinuxDevelopmentBoundaryErrorKind::RetentionFailed,
        ));
    }
    Ok(())
}

fn development_scenario_valid(scenario: &str) -> bool {
    matches!(
        scenario,
        "no-op"
            | "failed-test-repair"
            | "native-command-failure"
            | "slow-cancel"
            | "restart-repair"
            | "protocol-correction"
            | "arguments-correction"
            | "read-arguments-correction"
            | "read-arguments-denied"
            | "repeated-protocol-rejection"
            | "restart-protocol-correction"
            | "new-file"
            | "multi-file"
            | "rollback"
            | "false-completion"
            | "overflow"
            | "disk-pressure"
            | "output-pressure"
    )
}

const fn development_error(
    kind: LinuxDevelopmentBoundaryErrorKind,
) -> LinuxDevelopmentBoundaryError {
    LinuxDevelopmentBoundaryError { kind }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::os::unix::fs::{MetadataExt, PermissionsExt};
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    use super::{
        LinuxDevelopmentBoundaryErrorKind, LinuxDevelopmentHostProcess,
        ensure_private_development_directory, retain_rejected_development_output,
    };

    static FIXTURE_ID: AtomicU64 = AtomicU64::new(1);

    #[test]
    fn private_development_retention_is_new_owner_only_and_bounded() {
        let fixture = private_fixture();
        let retained = fixture.join("rejections");
        ensure_private_development_directory(&retained).expect("private retained root");
        retain_rejected_development_output(
            &retained,
            "0123456789abcdef01234567",
            b"raw-response",
            br#"{"disposition":"rejected"}"#,
        )
        .expect("retained pair");
        let raw = retained.join("0123456789abcdef01234567.response.bin");
        let metadata = retained.join("0123456789abcdef01234567.metadata.json");
        assert_eq!(fs::read(&raw).expect("raw"), b"raw-response");
        assert_eq!(
            fs::symlink_metadata(&raw).expect("raw metadata").mode() & 0o777,
            0o600
        );
        assert_eq!(
            fs::symlink_metadata(&metadata)
                .expect("record metadata")
                .mode()
                & 0o777,
            0o600
        );
        assert!(
            retain_rejected_development_output(
                &retained,
                "0123456789abcdef01234567",
                b"substitute",
                b"{}",
            )
            .is_err()
        );
        assert_eq!(fs::read(raw).expect("original remains"), b"raw-response");
        fs::remove_dir_all(fixture).expect("fixture cleanup");
    }

    #[test]
    fn correction_scenarios_are_explicit_closed_development_operations() {
        for scenario in [
            "native-command-failure",
            "protocol-correction",
            "arguments-correction",
            "read-arguments-correction",
            "read-arguments-denied",
            "repeated-protocol-rejection",
            "restart-protocol-correction",
        ] {
            assert!(super::development_scenario_valid(scenario));
        }
        for scenario in ["arbitrary", "protocol-correction;sh", "", " correction"] {
            assert!(!super::development_scenario_valid(scenario));
        }
    }

    #[test]
    fn development_host_rejects_open_ended_operation_before_launch() {
        let fixture = private_fixture();
        let error = LinuxDevelopmentHostProcess::launch(
            &fixture,
            &fixture,
            &fixture,
            "arbitrary",
            "scripted",
            false,
        )
        .err()
        .expect("closed scenario");
        assert_eq!(
            error.kind(),
            LinuxDevelopmentBoundaryErrorKind::InvalidInput
        );
        fs::remove_dir_all(fixture).expect("fixture cleanup");
    }

    fn envelope_fixture() -> Vec<u8> {
        // Existing v1 parser contract, with synthetic values and no connection.
        let endpoint = b"/synthetic";
        let mut frame = b"AGMB".to_vec();
        frame.extend_from_slice(&1_u16.to_be_bytes());
        frame.extend_from_slice(&(endpoint.len() as u16).to_be_bytes());
        frame.extend_from_slice(endpoint);
        frame.extend_from_slice(&[1; 32]);
        frame.extend_from_slice(&[2; 32]);
        frame.extend_from_slice(&1000_u32.to_be_bytes());
        frame.extend_from_slice(&42_i32.to_be_bytes());
        frame.extend_from_slice(&1_u64.to_be_bytes());
        frame.extend_from_slice(&[3; 32]);
        frame
    }

    #[test]
    fn empty_and_partial_open_pipes_expire_at_the_original_startup_deadline() {
        use std::io::Write;
        use std::time::{Duration, Instant};
        let frame = envelope_fixture();
        for prefix in [0, 4, 8, frame.len() - 1] {
            let (input, mut output) = std::io::pipe().unwrap();
            output.write_all(&frame[..prefix]).unwrap();
            let started = Instant::now();
            let error = super::read_envelope_before(input, started + Duration::from_millis(25))
                .expect_err("an open but incomplete pipe must expire");
            assert_eq!(
                error.kind(),
                LinuxDevelopmentBoundaryErrorKind::TransferTimedOut
            );
            assert!(started.elapsed() >= Duration::from_millis(25));
            assert!(started.elapsed() < Duration::from_secs(2));
            drop(output);
        }
    }

    #[test]
    fn complete_pipe_uses_existing_parser_without_waiting_for_eof() {
        use std::io::Write;
        use std::time::{Duration, Instant};
        let (input, mut output) = std::io::pipe().unwrap();
        output.write_all(&envelope_fixture()).unwrap();
        let envelope = super::read_envelope_before(input, Instant::now() + Duration::from_secs(1))
            .expect("the existing v1 frame completes while the writer remains open");
        assert!(!format!("{envelope:?}").contains("/synthetic"));
        drop(output);
    }

    #[test]
    fn eof_malformed_version_and_oversized_frames_remain_transfer_refusals() {
        use std::io::Write;
        use std::time::{Duration, Instant};
        let frame = envelope_fixture();
        let mut version = frame.clone();
        version[5] = 2;
        let mut length = frame.clone();
        length[7] = 108;
        let mut endpoint = frame.clone();
        endpoint[8] = b'x';
        let mut secret = frame.clone();
        secret[50..82].fill(0);
        for bytes in [
            Vec::new(),
            frame[..7].to_vec(),
            version,
            length,
            endpoint,
            secret,
        ] {
            let (input, mut output) = std::io::pipe().unwrap();
            output.write_all(&bytes).unwrap();
            drop(output);
            let error = super::read_envelope_before(input, Instant::now() + Duration::from_secs(1))
                .expect_err("no alternate decoder or accepted credentials");
            assert_eq!(
                error.kind(),
                LinuxDevelopmentBoundaryErrorKind::TransferFailed
            );
        }
    }

    #[test]
    fn drip_fed_progress_cannot_renew_the_startup_deadline() {
        use std::io::Write;
        use std::time::{Duration, Instant};
        let (input, mut output) = std::io::pipe().unwrap();
        let writer = std::thread::spawn(move || {
            for byte in envelope_fixture() {
                if output.write_all(&[byte]).is_err() {
                    break;
                }
                std::thread::sleep(Duration::from_millis(10));
            }
        });
        let started = Instant::now();
        let result = super::read_envelope_before(input, started + Duration::from_millis(35));
        writer.join().unwrap();
        assert_eq!(
            result.expect_err("fragments do not renew time").kind(),
            LinuxDevelopmentBoundaryErrorKind::TransferTimedOut
        );
        assert!(started.elapsed() < Duration::from_secs(1));
    }

    #[test]
    fn expired_deadline_never_accepts_an_already_buffered_envelope() {
        use std::io::Write;
        let (input, mut output) = std::io::pipe().unwrap();
        output.write_all(&envelope_fixture()).unwrap();
        let error = super::read_envelope_before(input, std::time::Instant::now())
            .expect_err("a late frame is not startup success");
        assert_eq!(
            error.kind(),
            LinuxDevelopmentBoundaryErrorKind::TransferTimedOut
        );
    }

    #[test]
    fn startup_cancellation_refuses_even_a_complete_buffered_frame_without_consuming_flag() {
        use std::io::Write;
        use std::sync::atomic::{AtomicBool, Ordering};
        use std::time::{Duration, Instant};
        let (input, mut output) = std::io::pipe().unwrap();
        output.write_all(&envelope_fixture()).unwrap();
        let requested = AtomicBool::new(true);
        let error = super::read_envelope_before_cancellable(
            input,
            Instant::now() + Duration::from_secs(2),
            Some(&requested),
        )
        .expect_err("stop takes precedence over buffered startup credentials");
        assert_eq!(
            error.kind(),
            LinuxDevelopmentBoundaryErrorKind::StartupCancelled
        );
        assert!(requested.load(Ordering::Acquire));
    }

    #[test]
    fn startup_cancellation_interrupts_a_partial_open_pipe_before_its_deadline() {
        use std::io::Write;
        use std::sync::{
            Arc,
            atomic::{AtomicBool, Ordering},
        };
        use std::time::{Duration, Instant};
        let (input, mut output) = std::io::pipe().unwrap();
        output.write_all(b"AGMB").unwrap();
        let requested = Arc::new(AtomicBool::new(false));
        let setter = Arc::clone(&requested);
        let worker = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(20));
            setter.store(true, Ordering::Release);
        });
        let started = Instant::now();
        let result = super::read_envelope_before_cancellable(
            input,
            started + Duration::from_secs(3),
            Some(&requested),
        );
        worker.join().unwrap();
        assert_eq!(
            result
                .expect_err("stop interrupts an incomplete frame")
                .kind(),
            LinuxDevelopmentBoundaryErrorKind::StartupCancelled
        );
        assert!(started.elapsed() < Duration::from_secs(1));
        assert!(requested.load(Ordering::Acquire));
    }

    #[test]
    fn admission_race_allows_one_owner_and_pre_spawn_drop_releases_it() {
        use std::sync::{Arc, Barrier, atomic::AtomicU8};
        let slot = Arc::new(AtomicU8::new(super::SLOT_AVAILABLE));
        let before = Arc::new(Barrier::new(3));
        let after = Arc::new(Barrier::new(3));
        let workers: Vec<_> = (0..2)
            .map(|_| {
                let (slot, before, after) =
                    (Arc::clone(&slot), Arc::clone(&before), Arc::clone(&after));
                std::thread::spawn(move || {
                    before.wait();
                    let owner = super::HostAdmission::reserve(slot);
                    after.wait();
                    owner.is_ok()
                })
            })
            .collect();
        before.wait();
        after.wait();
        let admitted = workers
            .into_iter()
            .map(|worker| usize::from(worker.join().unwrap()))
            .sum::<usize>();
        assert_eq!(admitted, 1);
        drop(super::HostAdmission::reserve(slot).expect("pre-spawn token released"));
    }

    fn test_host(
        mode: &str,
    ) -> (
        LinuxDevelopmentHostProcess,
        PathBuf,
        std::sync::Arc<std::sync::atomic::AtomicU8>,
    ) {
        use std::process::{Command, Stdio};
        use std::sync::{Arc, atomic::AtomicU8};
        use std::time::{Duration, Instant};
        let fixture = private_fixture();
        let ready = fixture.join("child-ready");
        let slot = Arc::new(AtomicU8::new(super::SLOT_AVAILABLE));
        let mut admission = super::HostAdmission::reserve(Arc::clone(&slot)).unwrap();
        let child = Command::new(std::env::current_exe().unwrap())
            .args([
                "--ignored",
                "--exact",
                "development_boundary::tests::owned_lifecycle_child",
                "--test-threads=1",
            ])
            .env("AGENTMAGE_DEVELOPMENT_CHILD_TEST_MODE", mode)
            .env("AGENTMAGE_DEVELOPMENT_CHILD_TEST_READY", &ready)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        admission.launched();
        let host = LinuxDevelopmentHostProcess {
            child: Some(child),
            status: None,
            startup_deadline: Instant::now() + Duration::from_secs(1),
            admission,
            cleanup_uncertain: false,
            exit_timed_out: false,
        };
        let deadline = Instant::now() + Duration::from_secs(3);
        while !ready.exists() && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(5));
        }
        assert_eq!(fs::read(&ready).unwrap(), b"entered");
        (host, fixture, slot)
    }

    #[test]
    #[ignore = "test-owned subprocess helper; parent lifecycle tests select it explicitly"]
    fn owned_lifecycle_child() {
        let mode = std::env::var("AGENTMAGE_DEVELOPMENT_CHILD_TEST_MODE").unwrap();
        let ready = std::env::var_os("AGENTMAGE_DEVELOPMENT_CHILD_TEST_READY").unwrap();
        let ready = PathBuf::from(ready);
        let preparing = ready.with_extension("preparing");
        fs::write(&preparing, b"entered").unwrap();
        fs::rename(preparing, ready).unwrap();
        match mode.as_str() {
            "hold" => std::thread::sleep(std::time::Duration::from_secs(30)),
            "success" => std::process::exit(0),
            "failure" => std::process::exit(23),
            _ => panic!("unregistered synthetic child mode"),
        }
    }

    #[test]
    fn direct_child_success_and_failure_are_reaped_before_admission_release() {
        for (mode, success) in [("success", true), ("failure", false)] {
            let (mut host, fixture, slot) = test_host(mode);
            assert_eq!(host.wait_success().unwrap(), success);
            assert_eq!(host.wait_success().unwrap(), success);
            assert!(super::HostAdmission::reserve(std::sync::Arc::clone(&slot)).is_err());
            host.terminate_and_reap().unwrap();
            drop(host);
            let fresh = super::HostAdmission::reserve(slot).expect("reaped owner released");
            drop(fresh);
            fs::remove_dir_all(fixture).unwrap();
        }
    }

    #[test]
    fn graceful_timeout_kills_and_reaps_only_the_owned_child_and_stays_failed() {
        use std::time::{Duration, Instant};
        let (mut host, fixture, slot) = test_host("hold");
        let (mut unrelated, other_fixture, _) = test_host("hold");
        let started = Instant::now();
        let error = host
            .wait_success_with_limits(Duration::from_millis(25), Duration::from_secs(1))
            .expect_err("a forced exit cannot become success");
        assert_eq!(
            error.kind(),
            LinuxDevelopmentBoundaryErrorKind::ExitTimedOut
        );
        assert!(host.status.is_some());
        assert_eq!(
            host.wait_success().unwrap_err().kind(),
            LinuxDevelopmentBoundaryErrorKind::ExitTimedOut
        );
        assert!(started.elapsed() < Duration::from_secs(2));
        assert!(unrelated.observe_exit().unwrap().is_none());
        unrelated.terminate_and_reap().unwrap();
        drop(unrelated);
        drop(host);
        drop(super::HostAdmission::reserve(slot).unwrap());
        fs::remove_dir_all(fixture).unwrap();
        fs::remove_dir_all(other_fixture).unwrap();
    }

    #[test]
    fn late_zero_exit_does_not_turn_a_repeated_timeout_wait_into_success() {
        use std::time::Duration;
        let (mut host, fixture, _) = test_host("success");
        std::thread::sleep(Duration::from_millis(30));
        assert_eq!(
            host.wait_success_with_limits(Duration::ZERO, Duration::from_secs(1))
                .unwrap_err()
                .kind(),
            LinuxDevelopmentBoundaryErrorKind::ExitTimedOut
        );
        assert!(host.status.unwrap().success());
        assert_eq!(
            host.wait_success().unwrap_err().kind(),
            LinuxDevelopmentBoundaryErrorKind::ExitTimedOut
        );
        drop(host);
        fs::remove_dir_all(fixture).unwrap();
    }

    #[test]
    fn destructor_reaps_the_owned_child_and_releases_its_slot() {
        use rustix::process::{Pid, WaitOptions, waitpid};
        let (host, fixture, slot) = test_host("hold");
        let pid = Pid::from_raw(i32::try_from(host.child.as_ref().unwrap().id()).unwrap()).unwrap();
        let started = std::time::Instant::now();
        drop(host);
        assert!(started.elapsed() < std::time::Duration::from_secs(4));
        assert!(matches!(
            waitpid(Some(pid), WaitOptions::NOHANG),
            Err(rustix::io::Errno::CHILD)
        ));
        drop(super::HostAdmission::reserve(slot).unwrap());
        fs::remove_dir_all(fixture).unwrap();
    }

    #[test]
    fn synthetic_lost_handle_quarantines_without_signalling_an_unrelated_child() {
        use std::sync::{Arc, atomic::AtomicU8};
        let slot = Arc::new(AtomicU8::new(super::SLOT_AVAILABLE));
        let mut admission = super::HostAdmission::reserve(Arc::clone(&slot)).unwrap();
        admission.launched();
        let mut lost = LinuxDevelopmentHostProcess {
            child: None,
            status: None,
            startup_deadline: std::time::Instant::now(),
            admission,
            cleanup_uncertain: false,
            exit_timed_out: false,
        };
        let (mut unrelated, fixture, _) = test_host("hold");
        assert_eq!(
            lost.terminate_and_reap().unwrap_err().kind(),
            LinuxDevelopmentBoundaryErrorKind::CleanupUncertain
        );
        assert_eq!(
            lost.wait_success().unwrap_err().kind(),
            LinuxDevelopmentBoundaryErrorKind::CleanupUncertain
        );
        assert!(unrelated.observe_exit().unwrap().is_none());
        drop(lost);
        assert_eq!(
            super::HostAdmission::reserve(slot).err().unwrap().kind(),
            LinuxDevelopmentBoundaryErrorKind::OwnerUnavailable
        );
        unrelated.terminate_and_reap().unwrap();
        drop(unrelated);
        fs::remove_dir_all(fixture).unwrap();
    }

    fn private_fixture() -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "agentmage-development-boundary-{}-{}",
            std::process::id(),
            FIXTURE_ID.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).expect("fixture root");
        fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).expect("fixture mode");
        path
    }
}
