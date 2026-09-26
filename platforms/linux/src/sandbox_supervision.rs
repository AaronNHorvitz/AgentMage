//! Bounded lifecycle helpers of the existing Linux sandbox owner.
//! Private implementation; no caller-selected process or authority interface.

use super::{
    LinuxSandboxError, LinuxSandboxErrorKind, LinuxSandboxLimits, LinuxSandboxManifest,
    LinuxSandboxResult, SandboxCancellation, digest_bytes, error, revalidate_launch_artifact,
};
use agentmage_kernel_contracts::OperationOutcome;
use rustix::fd::OwnedFd;
use rustix::fs::{Mode, OFlags, fcntl_getfl, fcntl_setfl, fstat, fstatfs, open, openat};
use rustix::io::pread;
use rustix::process::getuid;
use sha2::{Digest, Sha256};
use std::io::{Read, Write};
use std::os::fd::AsFd;
use std::process::{Child, ChildStderr, ChildStdin, ChildStdout, Command, ExitStatus, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const CONTROL_TIME: Duration = Duration::from_millis(500);
const REAP_TIME: Duration = Duration::from_millis(500);
const CLEANUP_TIME: Duration = Duration::from_secs(3);
const POLL_TIME: Duration = Duration::from_millis(10);
const CONTROL_BYTES: usize = 8192;
const PIPE_CHUNK: usize = 8192;
const PIPE_POLLS: usize = 8;
const ATOMIC_FILTER_BYTES: usize = 4096;

// ONE retained attempt, never an accumulating orphan list. The slot outlives a
// runner/driver and keeps the exact /proc/PARENT/fd numbers used by OpenFile alive.
// A poisoned or unresolved owner refuses subsequent admission; no detached reaper.
static OWNED_ATTEMPT: Mutex<Option<Attempt>> = Mutex::new(None);

pub(super) fn admission_snapshot() -> Result<(), LinuxSandboxError> {
    admission_snapshot_of(&OWNED_ATTEMPT)
}

fn admission_snapshot_of<T>(slot: &Mutex<Option<T>>) -> Result<(), LinuxSandboxError> {
    match slot.try_lock() {
        Ok(current) if current.is_none() => Ok(()),
        Ok(_) | Err(std::sync::TryLockError::Poisoned(_)) => {
            Err(error(LinuxSandboxErrorKind::CleanupUncertain))
        }
        Err(std::sync::TryLockError::WouldBlock) => Err(failure()),
    }
}

fn failure() -> LinuxSandboxError {
    error(LinuxSandboxErrorKind::ExecutionFailed)
}

fn nonblocking(pipe: &impl AsFd) -> Result<(), LinuxSandboxError> {
    let flags = fcntl_getfl(pipe).map_err(|_| failure())?;
    fcntl_setfl(pipe, flags | OFlags::NONBLOCK).map_err(|_| failure())
}

struct Capture<R> {
    reader: R,
    eof: bool,
    exceeded: bool,
    limit: usize,
    retained: Vec<u8>,
    digest: Sha256,
    total: usize,
}

impl<R: Read + AsFd> Capture<R> {
    fn new(reader: R, limit: usize) -> Result<Self, LinuxSandboxError> {
        if limit == 0 || limit > super::MAX_OUTPUT_BYTES {
            return Err(failure());
        }
        nonblocking(&reader)?;
        Ok(Self {
            reader,
            eof: false,
            exceeded: false,
            limit,
            retained: Vec::new(),
            digest: Sha256::new(),
            total: 0,
        })
    }

    fn poll(&mut self) -> Result<(), LinuxSandboxError> {
        if self.exceeded {
            return Err(error(LinuxSandboxErrorKind::OutputLimitExceeded));
        }
        if self.eof {
            return Ok(());
        }
        let mut bytes = [0; PIPE_CHUNK];
        for _ in 0..PIPE_POLLS {
            match self.reader.read(&mut bytes) {
                Ok(0) => {
                    self.eof = true;
                    return Ok(());
                }
                Ok(count) => {
                    self.total = self.total.checked_add(count).ok_or_else(failure)?;
                    self.digest.update(&bytes[..count]);
                    let keep = count.min(self.limit.saturating_sub(self.retained.len()));
                    self.retained.extend_from_slice(&bytes[..keep]);
                    if self.total > self.limit {
                        self.exceeded = true;
                        return Err(error(LinuxSandboxErrorKind::OutputLimitExceeded));
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => return Ok(()),
                Err(e) if e.kind() == std::io::ErrorKind::Interrupted => {}
                Err(_) => return Err(failure()),
            }
        }
        Ok(())
    }
}

#[derive(PartialEq, Eq)]
enum FilterState {
    Withheld,
    Released,
    Failed,
}

struct WithheldFilter<W> {
    writer: Option<W>,
    state: FilterState,
}

impl<W: Write + AsFd> WithheldFilter<W> {
    fn initialize(&self) -> Result<(), LinuxSandboxError> {
        nonblocking(self.writer.as_ref().ok_or_else(failure)?)
    }

    fn release(&mut self, policy: &[u8]) -> Result<bool, LinuxSandboxError> {
        match self.state {
            FilterState::Released => return Ok(true),
            FilterState::Failed => return Err(failure()),
            FilterState::Withheld => {}
        }
        if policy.is_empty()
            || policy.len() > ATOMIC_FILTER_BYTES
            || !policy.len().is_multiple_of(8)
        {
            self.state = FilterState::Failed;
            return Err(error(LinuxSandboxErrorKind::SeccompUnavailable));
        }
        // One <= PIPE_BUF write, never a partial-policy loop. The owning attempt
        // invokes this ONLY after witnessing its exact transient unit and cgroup.
        match self.writer.as_mut().ok_or_else(failure)?.write(policy) {
            Ok(count) if count == policy.len() => {
                self.state = FilterState::Released;
                self.writer.take();
                Ok(true)
            }
            Err(e)
                if matches!(
                    e.kind(),
                    std::io::ErrorKind::WouldBlock | std::io::ErrorKind::Interrupted
                ) =>
            {
                Ok(false)
            }
            _ => {
                // Retain the writer on impossible short writes: EOF could install
                // a partial BPF program. No retries after ambiguous transmission.
                self.state = FilterState::Failed;
                Err(failure())
            }
        }
    }
}

struct Control {
    child: Child,
    stdout: Option<Capture<ChildStdout>>,
    stderr: Option<Capture<ChildStderr>>,
    status: Option<ExitStatus>,
}

impl Control {
    fn initialize(&mut self) -> Result<(), LinuxSandboxError> {
        self.stdout = Some(Capture::new(
            self.child.stdout.take().ok_or_else(failure)?,
            CONTROL_BYTES,
        )?);
        self.stderr = Some(Capture::new(
            self.child.stderr.take().ok_or_else(failure)?,
            CONTROL_BYTES,
        )?);
        Ok(())
    }

    fn reap(&mut self) -> Result<bool, LinuxSandboxError> {
        if self.status.is_none() {
            self.status = self.child.try_wait().map_err(|_| failure())?;
        }
        Ok(self.status.is_some())
    }

    fn abort_and_reap(&mut self, deadline: Instant) -> Result<bool, LinuxSandboxError> {
        if self.reap()? {
            return Ok(true);
        }
        // Only this opaque owned Child. A deadline never permits another spawn
        // around an unreaped child or a PID/name lookup for an unrelated process.
        let _ = self.child.kill();
        loop {
            if self.reap()? {
                return Ok(true);
            }
            let now = Instant::now();
            if now >= deadline {
                return Ok(false);
            }
            std::thread::sleep(POLL_TIME.min(deadline.saturating_duration_since(now)));
        }
    }

    fn poll(&mut self) -> Result<bool, LinuxSandboxError> {
        self.stdout.as_mut().ok_or_else(failure)?.poll()?;
        self.stderr.as_mut().ok_or_else(failure)?.poll()?;
        if self.reap()? {
            // Collect bytes written just before exit; bounded and nonblocking.
            self.stdout.as_mut().ok_or_else(failure)?.poll()?;
            self.stderr.as_mut().ok_or_else(failure)?.poll()?;
        }
        Ok(self.status.is_some()
            && self.stdout.as_ref().is_some_and(|s| s.eof)
            && self.stderr.as_ref().is_some_and(|s| s.eof))
    }
}

struct OwnedGroup {
    path: String,
    directory: OwnedFd,
    events: OwnedFd,
    kill: OwnedFd,
}

impl OwnedGroup {
    fn open(path: &str, unit: &str) -> Result<Self, LinuxSandboxError> {
        let uid = getuid().as_raw();
        let prefix = format!("/user.slice/user-{uid}.slice/user@{uid}.service/");
        if !path.starts_with(&prefix) || !cgroup(path, unit) {
            return Err(failure());
        }
        let flags = OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC;
        let mut directory = open("/sys/fs/cgroup", flags, Mode::empty()).map_err(|_| failure())?;
        // CGROUP2_SUPER_MAGIC, from Linux uapi/linux/magic.h. Never accept an
        // ordinary filesystem with attacker-controlled cgroup.events/kill files.
        if fstatfs(&directory).map_err(|_| failure())?.f_type != 0x6367_7270 {
            return Err(failure());
        }
        for component in path[1..].split('/') {
            directory =
                openat(&directory, component, flags, Mode::empty()).map_err(|_| failure())?;
        }
        let common = OFlags::NOFOLLOW | OFlags::CLOEXEC | OFlags::NONBLOCK;
        let events = openat(
            &directory,
            "cgroup.events",
            common | OFlags::RDONLY,
            Mode::empty(),
        )
        .map_err(|_| failure())?;
        let kill = openat(
            &directory,
            "cgroup.kill",
            common | OFlags::WRONLY,
            Mode::empty(),
        )
        .map_err(|_| failure())?;
        Ok(Self {
            path: path.to_owned(),
            directory,
            events,
            kill,
        })
    }

    fn empty(&self) -> Result<bool, LinuxSandboxError> {
        if fstat(&self.directory).map_err(|_| failure())?.st_nlink == 0 {
            return Ok(true);
        }
        let mut bytes = [0; 257];
        let count = pread(&self.events, &mut bytes, 0).map_err(|_| failure())?;
        parse_group_empty(&bytes[..count])
    }

    fn kill_owned(&self) -> Result<(), LinuxSandboxError> {
        if self.empty()? {
            return Ok(());
        }
        // The held cgroup descriptor, not a name lookup or a PID inventory.
        if rustix::io::write(&self.kill, b"1") != Ok(1) {
            return Err(failure());
        }
        Ok(())
    }
}

fn parse_group_empty(bytes: &[u8]) -> Result<bool, LinuxSandboxError> {
    if bytes.is_empty() || bytes.len() > 256 {
        return Err(failure());
    }
    let text = std::str::from_utf8(bytes).map_err(|_| failure())?;
    let mut populated = None;
    let mut frozen = None;
    if !text.ends_with('\n') {
        return Err(failure());
    }
    for line in text.lines() {
        match line.split_once(' ') {
            Some(("populated", "0" | "1")) if populated.is_none() => {
                populated = Some(line.ends_with('1'))
            }
            Some(("frozen", "0" | "1")) if frozen.is_none() => frozen = Some(line.ends_with('1')),
            _ => return Err(failure()),
        }
    }
    if frozen.is_none() {
        return Err(failure());
    }
    populated.map(|value| !value).ok_or_else(failure)
}

struct Observation {
    stage: UnitStage,
    invocation: String,
    cgroup: String,
}

fn same_running_generation(first: &Observation, second: &Observation) -> bool {
    first.stage == UnitStage::Running
        && second.stage == UnitStage::Running
        && !first.invocation.is_empty()
        && !first.cgroup.is_empty()
        && first.invocation == second.invocation
        && first.cgroup == second.cgroup
}

struct Attempt {
    manifest: Arc<LinuxSandboxManifest>,
    _projections: Vec<OwnedFd>,
    unit: String,
    child: Option<Child>,
    status: Option<ExitStatus>,
    stdout: Option<Capture<ChildStdout>>,
    stderr: Option<Capture<ChildStderr>>,
    filter: Option<WithheldFilter<ChildStdin>>,
    control: Option<Control>,
    invocation: Option<String>,
    group: Option<OwnedGroup>,
    clean: bool,
    #[cfg(test)]
    last_observation: Option<Vec<u8>>,
}

impl Attempt {
    fn initialize(&mut self, limit: usize) -> Result<(), LinuxSandboxError> {
        let child = self.child.as_mut().ok_or_else(failure)?;
        self.filter = Some(WithheldFilter {
            writer: Some(child.stdin.take().ok_or_else(failure)?),
            state: FilterState::Withheld,
        });
        self.filter.as_ref().ok_or_else(failure)?.initialize()?;
        self.stdout = Some(Capture::new(
            child.stdout.take().ok_or_else(failure)?,
            limit,
        )?);
        self.stderr = Some(Capture::new(
            child.stderr.take().ok_or_else(failure)?,
            limit,
        )?);
        Ok(())
    }

    fn reap(&mut self) -> Result<(), LinuxSandboxError> {
        if self.status.is_none() {
            self.status = self
                .child
                .as_mut()
                .ok_or_else(failure)?
                .try_wait()
                .map_err(|_| failure())?;
        }
        Ok(())
    }

    // None means the ORIGINAL operation deadline expired, not an invalid
    // response, control-process failure, or proof that descendants stopped.
    fn control(&mut self, deadline: Instant) -> Result<Option<Vec<u8>>, LinuxSandboxError> {
        if self.control.is_some() {
            return Err(failure());
        }
        if Instant::now() >= deadline {
            return Ok(None);
        }
        let artifact = &self.manifest.systemctl;
        revalidate_launch_artifact(artifact)?;
        let runtime = format!("/run/user/{}", getuid().as_raw());
        let mut command = Command::new(artifact.launch_path.as_ref().ok_or_else(failure)?);
        command
            .env_clear()
            .env("XDG_RUNTIME_DIR", &runtime)
            .env(
                "DBUS_SESSION_BUS_ADDRESS",
                format!("unix:path={runtime}/bus"),
            )
            .env("LANG", "C")
            .args(["--user", "--no-pager", "--no-ask-password"]);
        command.args(["--all", "show", &self.unit]);
        for property in UNIT_PROPERTIES {
            command.arg(format!("--property={property}"));
        }
        command
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let end = Instant::now()
            .checked_add(CONTROL_TIME)
            .ok_or_else(failure)?
            .min(deadline);
        // Store before any fallible pipe initialization; an unreapable child stays
        // in this ONE slot and prevents any additional control process.
        self.control = Some(Control {
            child: command.spawn().map_err(|_| failure())?,
            stdout: None,
            stderr: None,
            status: None,
        });
        let control = self.control.as_mut().ok_or_else(failure)?;
        let mut valid = control.initialize().is_ok();
        while valid && Instant::now() < end {
            match control.poll() {
                Ok(true) => {
                    let complete = self.control.take().ok_or_else(failure)?;
                    if !complete.status.is_some_and(|s| s.success()) {
                        return Err(failure());
                    }
                    return Ok(Some(complete.stdout.ok_or_else(failure)?.retained));
                }
                Ok(false) => {}
                Err(_) => valid = false,
            }
            std::thread::sleep(POLL_TIME);
        }
        // Preserve actual initialization/read/control errors even if their
        // cleanup crosses the deadline. Only an otherwise valid poll timeout
        // caused by the original deadline has a TimedOut disposition.
        let operation_deadline_reached = valid && Instant::now() >= deadline;
        let reap_end = Instant::now()
            .checked_add(REAP_TIME)
            .ok_or_else(failure)?
            .min(deadline);
        if control.abort_and_reap(reap_end)? {
            self.control.take();
        }
        if operation_deadline_reached {
            Ok(None)
        } else {
            Err(failure())
        }
    }

    fn observe(&mut self, deadline: Instant) -> Result<Option<Observation>, LinuxSandboxError> {
        let Some(bytes) = self.control(deadline)? else {
            return Ok(None);
        };
        #[cfg(test)]
        if self.last_observation.as_deref() != Some(bytes.as_slice()) {
            // Synthetic native fixtures retain only exact owned-unit properties;
            // no global inventory or raw worker payload. Not a product log path.
            eprintln!(
                "native-owned-observation unit={} properties={:?}",
                self.unit,
                String::from_utf8_lossy(&bytes)
            );
            self.last_observation = Some(bytes.clone());
        }
        let observation = parse_unit_observation(&self.unit, &bytes).map_err(|_| failure())?;
        if self.invocation.as_ref().is_some_and(|pinned| {
            !observation.invocation.is_empty() && pinned != observation.invocation
        }) {
            return Err(failure());
        }
        if self.group.as_ref().is_some_and(|pinned| {
            !observation.cgroup.is_empty() && pinned.path != observation.cgroup
        }) {
            return Err(failure());
        }
        Ok(Some(Observation {
            stage: observation.stage,
            invocation: observation.invocation.into(),
            cgroup: observation.cgroup.into(),
        }))
    }

    fn poll_output(&mut self) -> Result<(), LinuxSandboxError> {
        self.stdout.as_mut().ok_or_else(failure)?.poll()?;
        self.stderr.as_mut().ok_or_else(failure)?.poll()?;
        self.reap()
    }

    fn complete(&self, observation: &Observation) -> Result<bool, LinuxSandboxError> {
        Ok(self.invocation.is_some()
            && self.status.is_some()
            && matches!(observation.stage, UnitStage::Absent | UnitStage::Terminal)
            && self.group.as_ref().ok_or_else(failure)?.empty()?)
    }

    fn execute(
        &mut self,
        policy: &[u8],
        deadline: Instant,
        cancellation: &SandboxCancellation<'_>,
    ) -> Result<OperationOutcome, LinuxSandboxError> {
        loop {
            if cancellation.is_cancelled()? {
                return Ok(OperationOutcome::Cancelled);
            }
            if Instant::now() >= deadline {
                return Ok(OperationOutcome::TimedOut);
            }
            self.poll_output()?;
            let Some(observed) = self.observe(deadline)? else {
                return Ok(OperationOutcome::TimedOut);
            };
            if self.invocation.is_none() && observed.stage == UnitStage::Running {
                let group = OwnedGroup::open(&observed.cgroup, &self.unit)?;
                // The pathname may have been replaced after the first manager
                // observation. Do not adopt or signal that opened group until a
                // second exact running generation brackets the descriptor open.
                // Rejection leaves the filter withheld and ownership uncertain.
                let Some(confirmed) = self.observe(deadline)? else {
                    return Ok(OperationOutcome::TimedOut);
                };
                if !same_running_generation(&observed, &confirmed) {
                    return Err(failure());
                }
                self.invocation = Some(observed.invocation.clone());
                self.group = Some(group);
            }
            if self.invocation.is_some() {
                if cancellation.is_cancelled()? {
                    return Ok(OperationOutcome::Cancelled);
                }
                if Instant::now() >= deadline {
                    return Ok(OperationOutcome::TimedOut);
                }
                let filter = self.filter.as_mut().ok_or_else(failure)?;
                if filter.state == FilterState::Withheld && observed.stage == UnitStage::Running {
                    filter.release(policy)?;
                }
                if self.complete(&observed)? {
                    // Unit/cgroup cleanup does not make an inherited pipe EOF.
                    // Continue bounded drains until EOF or the ORIGINAL deadline.
                    if self.stdout.as_ref().is_some_and(|s| s.eof)
                        && self.stderr.as_ref().is_some_and(|s| s.eof)
                    {
                        self.clean = true;
                        return Ok(if self.status.is_some_and(|s| s.success()) {
                            OperationOutcome::Succeeded
                        } else {
                            OperationOutcome::Failed
                        });
                    }
                }
            }
            std::thread::sleep(POLL_TIME);
        }
    }

    fn cleanup(&mut self) -> Result<(), LinuxSandboxError> {
        let deadline = Instant::now()
            .checked_add(CLEANUP_TIME)
            .ok_or_else(failure)?;
        if let Some(control) = self.control.as_mut() {
            let reap_end = Instant::now()
                .checked_add(REAP_TIME)
                .ok_or_else(failure)?
                .min(deadline);
            if !control.abort_and_reap(reap_end)? {
                return Err(failure());
            }
            self.control.take();
        }
        // Without a witnessed invocation we cannot rule out a late queued Start.
        // Kill only our direct launcher, leave the empty filter and projections
        // owned, and quarantine the lane. This is never reported as clean.
        if self.invocation.is_none() || self.group.is_none() {
            if let Some(child) = self.child.as_mut() {
                let _ = child.kill();
            }
            while Instant::now() < deadline {
                self.reap()?;
                if self.status.is_some() {
                    break;
                }
                std::thread::sleep(POLL_TIME);
            }
            return Err(failure());
        }
        // Signal only the held owned cgroup, never systemctl stop/kill by name:
        // a name can denote a replacement after observation. The launch contract
        // sets Restart=no; removal/emptiness and exact terminal observation remain
        // mandatory. An unknown or replaced invocation is quarantined, not killed.
        let observed = self.observe(deadline)?.ok_or_else(failure)?;
        if !matches!(observed.stage, UnitStage::Absent | UnitStage::Terminal) {
            if self.invocation.as_deref() != Some(observed.invocation.as_str()) {
                return Err(failure());
            }
            let _ = self.group.as_ref().ok_or_else(failure)?.kill_owned();
        }
        while Instant::now() < deadline {
            self.reap()?;
            let observed = self.observe(deadline)?.ok_or_else(failure)?;
            if self.status.is_none()
                && matches!(observed.stage, UnitStage::Absent | UnitStage::Terminal)
                && self.group.as_ref().ok_or_else(failure)?.empty()?
            {
                // The exact worker is gone, but a stalled launcher is still ours.
                // Kill/reap that direct child without claiming its EOF was cleanup.
                let _ = self.child.as_mut().ok_or_else(failure)?.kill();
                self.reap()?;
            }
            if self.complete(&observed)? && self.control.is_none() {
                self.clean = true;
                return Ok(());
            }
            std::thread::sleep(POLL_TIME);
        }
        // Retain all ownership on unknown cleanup. Never block in Child::wait.
        Err(failure())
    }
}

pub(super) fn run(
    mut command: Command,
    manifest: Arc<LinuxSandboxManifest>,
    projections: Vec<OwnedFd>,
    unit: String,
    limits: LinuxSandboxLimits,
    policy: &[u8],
    cancellation: &SandboxCancellation<'_>,
) -> Result<LinuxSandboxResult, LinuxSandboxError> {
    if !unit_name(&unit)
        || projections.is_empty()
        || projections.len() > 8
        || policy.is_empty()
        || policy.len() > ATOMIC_FILTER_BYTES
        || !policy.len().is_multiple_of(8)
    {
        return Err(error(LinuxSandboxErrorKind::InvalidManifest));
    }
    let mut slot = OWNED_ATTEMPT.try_lock().map_err(|_| failure())?;
    if slot.is_some() {
        return Err(failure());
    }
    let deadline = Instant::now()
        .checked_add(Duration::from_secs(u64::from(limits.runtime_seconds)))
        .ok_or_else(failure)?;
    // Precancellation launches nothing and leaves the lane reusable.
    if cancellation.is_cancelled()? {
        return Ok(LinuxSandboxResult {
            outcome: OperationOutcome::Cancelled,
            stdout: Vec::new(),
            stdout_sha256: digest_bytes(&[]),
            stderr_sha256: digest_bytes(&[]),
            stderr_bytes: 0,
        });
    }
    *slot = Some(Attempt {
        manifest,
        _projections: projections,
        unit,
        child: None,
        status: None,
        stdout: None,
        stderr: None,
        filter: None,
        control: None,
        invocation: None,
        group: None,
        clean: false,
        #[cfg(test)]
        last_observation: None,
    });
    let attempt = slot.as_mut().ok_or_else(failure)?;
    #[cfg(test)]
    eprintln!("native-owned-unit={}", attempt.unit);
    match command.spawn() {
        Ok(child) => attempt.child = Some(child),
        Err(_) => {
            slot.take();
            return Err(error(LinuxSandboxErrorKind::IsolationUnavailable));
        }
    }
    let outcome = attempt
        .initialize(limits.output_bytes)
        .and_then(|()| attempt.execute(policy, deadline, cancellation));
    #[cfg(test)]
    if !matches!(outcome, Ok(OperationOutcome::Succeeded)) {
        eprintln!(
            "native-owned-result unit={} outcome={:?} launcher-status={:?} launcher-stderr-prefix={:?}",
            attempt.unit,
            outcome,
            attempt.status,
            attempt
                .stderr
                .as_ref()
                .map(|capture| String::from_utf8_lossy(
                    &capture.retained[..capture.retained.len().min(CONTROL_BYTES)]
                ))
        );
    }
    if !attempt.clean && attempt.cleanup().is_err() {
        #[cfg(test)]
        eprintln!(
            "native-owned-cleanup=uncertain unit={} invocation-observed={} launcher-reaped={} control-retained={}",
            attempt.unit,
            attempt.invocation.is_some(),
            attempt.status.is_some(),
            attempt.control.is_some()
        );
        return Err(error(LinuxSandboxErrorKind::CleanupUncertain));
    }
    #[cfg(test)]
    eprintln!("native-owned-cleanup=verified unit={}", attempt.unit);
    let finished = slot.take().ok_or_else(failure)?;
    let outcome = outcome?;
    let stdout = finished.stdout.ok_or_else(failure)?;
    let stderr = finished.stderr.ok_or_else(failure)?;
    // Forced cancellation/timeout intentionally has no successful output artifact.
    if !matches!(
        outcome,
        OperationOutcome::Cancelled | OperationOutcome::TimedOut
    ) && (!stdout.eof || !stderr.eof)
    {
        return Err(failure());
    }
    Ok(LinuxSandboxResult {
        outcome,
        stdout_sha256: digest_bytes(&stdout.retained),
        stdout: stdout.retained,
        stderr_sha256: stderr.digest.finalize().into(),
        stderr_bytes: stderr.total,
    })
}

use std::collections::BTreeMap;

const UNIT_OBSERVATION_BYTES: usize = 8192;
const UNIT_PROPERTIES: [&str; 11] = [
    "Id",
    "Transient",
    "LoadState",
    "ActiveState",
    "SubState",
    "InvocationID",
    "MainPID",
    "ControlPID",
    "ControlGroup",
    "Job",
    "Result",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct UnitObservationError;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum UnitStage {
    Absent,
    Pending,
    Running,
    Draining,
    Stopping,
    Terminal,
}

// No derived Debug: cgroup paths and process identities do not belong in generic
// user/model diagnostics. The owning native receipt uses only redacted identities.
struct UnitObservation<'a> {
    stage: UnitStage,
    invocation: &'a str,
    cgroup: &'a str,
}

fn decimal(value: &str, allow_zero: bool) -> Result<u32, UnitObservationError> {
    if value.is_empty()
        || value.len() > 10
        || !value.bytes().all(|byte| byte.is_ascii_digit())
        || (value.len() > 1 && value.starts_with('0'))
    {
        return Err(UnitObservationError);
    }
    let value: u32 = value.parse().map_err(|_| UnitObservationError)?;
    if value > i32::MAX as u32 || (value == 0 && !allow_zero) {
        return Err(UnitObservationError);
    }
    Ok(value)
}

fn unit_name(value: &str) -> bool {
    value
        .strip_prefix("agentmage-worker-")
        .and_then(|tail| tail.strip_suffix(".service"))
        .is_some_and(|nonce| {
            nonce.len() == 24
                && nonce
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        })
}

fn invocation(value: &str) -> bool {
    value.len() == 32
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        && value.bytes().any(|byte| byte != b'0')
}

fn cgroup(value: &str, expected_unit: &str) -> bool {
    value.len() <= 4096
        && value.starts_with('/')
        && value.rsplit('/').next() == Some(expected_unit)
        && value[1..].split('/').all(|part| {
            !part.is_empty()
                && !matches!(part, "." | "..")
                && part.bytes().all(|byte| {
                    byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.' | b'@')
                })
        })
}

fn parse_unit_observation<'a>(
    expected_unit: &str,
    bytes: &'a [u8],
) -> Result<UnitObservation<'a>, UnitObservationError> {
    if !unit_name(expected_unit) || bytes.is_empty() || bytes.len() > UNIT_OBSERVATION_BYTES {
        return Err(UnitObservationError);
    }
    let text = std::str::from_utf8(bytes).map_err(|_| UnitObservationError)?;
    if !text.ends_with('\n')
        || text
            .bytes()
            .any(|byte| !byte.is_ascii() || (byte.is_ascii_control() && byte != b'\n'))
    {
        return Err(UnitObservationError);
    }
    let mut fields = BTreeMap::new();
    for line in text.lines() {
        let (key, value) = line.split_once('=').ok_or(UnitObservationError)?;
        if !UNIT_PROPERTIES.contains(&key) || fields.insert(key, value).is_some() {
            return Err(UnitObservationError);
        }
    }
    if fields.len() != UNIT_PROPERTIES.len() || fields["Id"] != expected_unit {
        return Err(UnitObservationError);
    }
    let main_pid = decimal(fields["MainPID"], true)?;
    let control_pid = decimal(fields["ControlPID"], true)?;
    let pending_job = !fields["Job"].is_empty();
    if pending_job {
        decimal(fields["Job"], false)?;
    }
    let invocation_id = fields["InvocationID"];
    let group = fields["ControlGroup"];
    if (!invocation_id.is_empty() && !invocation(invocation_id))
        || (!group.is_empty() && !cgroup(group, expected_unit))
        || fields["Result"].is_empty()
        || fields["Result"].len() > 64
        || !fields["Result"]
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte == b'-')
    {
        return Err(UnitObservationError);
    }
    let no_process = main_pid == 0 && control_pid == 0 && group.is_empty();
    let stage = match (
        fields["LoadState"],
        fields["ActiveState"],
        fields["SubState"],
    ) {
        ("not-found", "inactive", "dead")
            if fields["Transient"] == "no"
                && !pending_job
                && no_process
                && invocation_id.is_empty() =>
        {
            UnitStage::Absent
        }
        ("loaded", active, substate) if fields["Transient"] == "yes" => {
            match (active, substate) {
                // Observed on the installed manager BEFORE the synthetic process:
                // inactive/dead plus a queued Job is NOT a cleaned-up worker.
                ("inactive", "dead") if pending_job && no_process => UnitStage::Pending,
                ("activating", "start-pre" | "start" | "start-post") => UnitStage::Pending,
                ("active", "running")
                    if !pending_job
                        && main_pid > 0
                        && control_pid == 0
                        && !group.is_empty()
                        && !invocation_id.is_empty()
                        && fields["Result"] == "success" =>
                {
                    UnitStage::Running
                }
                ("active", "running")
                    if !pending_job
                        && main_pid == 0
                        && control_pid == 0
                        && !group.is_empty()
                        && !invocation_id.is_empty() =>
                {
                    // ExitType=cgroup can outlive its main process. This is NOT
                    // a start-admission state and never releases a withheld filter.
                    UnitStage::Draining
                }
                (
                    "deactivating",
                    "stop" | "stop-sigterm" | "stop-sigkill" | "stop-post" | "final-sigterm"
                    | "final-sigkill",
                ) => UnitStage::Stopping,
                ("inactive", "dead") | ("failed", "failed")
                    if !pending_job && main_pid == 0 && control_pid == 0 =>
                {
                    // The manager may retain the path before cgroup pruning.
                    // The owner must still prove its held group empty/removed.
                    UnitStage::Terminal
                }
                _ => return Err(UnitObservationError),
            }
        }
        _ => return Err(UnitObservationError),
    };
    Ok(UnitObservation {
        stage,
        invocation: invocation_id,
        cgroup: group,
    })
}

// The supervisor still MUST pin the exact invocation, retain launch descriptors,
// bound/reap its control/launcher children and prove owned cgroup cleanup. In
// particular Absent before a witnessed start cannot rule out a queued late start.
// This helper intentionally has NO is_clean()/admit() method.

#[cfg(test)]
mod tests {
    use super::*;

    const UNIT: &str = "agentmage-worker-0123456789abcdef01234567.service";
    fn absent() -> String {
        format!(
            "Id={UNIT}\nLoadState=not-found\nActiveState=inactive\nSubState=dead\nJob=\nTransient=no\nInvocationID=\nMainPID=0\nControlPID=0\nResult=success\nControlGroup=\n"
        )
    }
    fn running() -> String {
        absent()
            .replace("LoadState=not-found", "LoadState=loaded")
            .replace("ActiveState=inactive", "ActiveState=active")
            .replace("SubState=dead", "SubState=running")
            .replace("Transient=no", "Transient=yes")
            .replace(
                "InvocationID=\n",
                "InvocationID=0123456789abcdef0123456789abcdef\n",
            )
            .replace("MainPID=0", "MainPID=42")
            .replace(
                "ControlGroup=\n",
                &format!(
                    "ControlGroup=/user.slice/user-1000.slice/user@1000.service/app.slice/{UNIT}\n"
                ),
            )
    }

    #[test]
    fn actual_manager_shapes_keep_pending_distinct_from_absent_or_terminal() {
        let before = absent();
        assert_eq!(
            parse_unit_observation(UNIT, before.as_bytes())
                .unwrap()
                .stage,
            UnitStage::Absent
        );
        let queued = before
            .replace("LoadState=not-found", "LoadState=loaded")
            .replace("Transient=no", "Transient=yes")
            .replace("Job=\n", "Job=123\n");
        assert_eq!(
            parse_unit_observation(UNIT, queued.as_bytes())
                .unwrap()
                .stage,
            UnitStage::Pending
        );
        let running = running();
        let observed = parse_unit_observation(UNIT, running.as_bytes()).unwrap();
        assert_eq!(observed.stage, UnitStage::Running);
        assert_eq!(observed.invocation, "0123456789abcdef0123456789abcdef");
        assert!(observed.cgroup.ends_with(UNIT));
        let terminal = queued.replace("Job=123", "Job=");
        assert_eq!(
            parse_unit_observation(UNIT, terminal.as_bytes())
                .unwrap()
                .stage,
            UnitStage::Terminal
        );
    }

    #[test]
    fn absent_with_job_process_or_cgroup_is_never_a_valid_observation() {
        for (from, to) in [
            ("Job=\n", "Job=1\n"),
            ("MainPID=0", "MainPID=1"),
            ("ControlPID=0", "ControlPID=1"),
            (
                "InvocationID=\n",
                "InvocationID=0123456789abcdef0123456789abcdef\n",
            ),
            ("Transient=no", "Transient=yes"),
        ] {
            assert!(parse_unit_observation(UNIT, absent().replace(from, to).as_bytes()).is_err());
        }
        let bad = absent().replace(
            "ControlGroup=\n",
            &format!("ControlGroup=/app.slice/{UNIT}\n"),
        );
        assert!(parse_unit_observation(UNIT, bad.as_bytes()).is_err());
    }

    #[test]
    fn main_exit_and_terminal_group_path_do_not_establish_start_or_cleanup() {
        let draining = running().replace("MainPID=42", "MainPID=0");
        let observed = parse_unit_observation(UNIT, draining.as_bytes()).unwrap();
        assert_eq!(observed.stage, UnitStage::Draining);
        assert_ne!(observed.stage, UnitStage::Running);
        assert_ne!(observed.stage, UnitStage::Terminal);
        assert!(!observed.cgroup.is_empty());
        let terminal = draining
            .replace("ActiveState=active", "ActiveState=inactive")
            .replace("SubState=running", "SubState=dead");
        let observed = parse_unit_observation(UNIT, terminal.as_bytes()).unwrap();
        assert_eq!(observed.stage, UnitStage::Terminal);
        assert!(!observed.cgroup.is_empty());
        for (from, to) in [
            ("MainPID=0", "MainPID=1"),
            ("ControlPID=0", "ControlPID=1"),
            ("Job=\n", "Job=1\n"),
        ] {
            assert!(parse_unit_observation(UNIT, terminal.replace(from, to).as_bytes()).is_err());
        }
    }

    #[test]
    fn exact_complete_closed_identity_is_required() {
        let source = running();
        for bytes in [
            source
                .replace(UNIT, "agentmage-worker-ffffffffffffffffffffffff.service")
                .into_bytes(),
            source.replace("Result=success\n", "").into_bytes(),
            format!("{source}Result=success\n").into_bytes(),
            format!("{source}Unknown=yes\n").into_bytes(),
            source.trim_end().as_bytes().to_vec(),
            source.replace("\n", "\r\n").into_bytes(),
            source
                .replace("Result=success", "Result=private\0canary")
                .into_bytes(),
            vec![b'x'; UNIT_OBSERVATION_BYTES + 1],
            vec![0xff],
        ] {
            assert!(parse_unit_observation(UNIT, &bytes).is_err());
        }
        for unit in [
            "",
            "other.service",
            "agentmage-worker-*.service",
            "../test.service",
        ] {
            assert!(parse_unit_observation(unit, source.as_bytes()).is_err());
        }
    }

    #[test]
    fn malformed_process_generation_and_group_are_refused() {
        for bad in [
            "",
            "-1",
            "+1",
            "1.0",
            "01",
            "2147483648",
            "4294967296",
            " 1",
        ] {
            assert!(
                parse_unit_observation(
                    UNIT,
                    running()
                        .replace("MainPID=42", &format!("MainPID={bad}"))
                        .as_bytes()
                )
                .is_err()
            );
        }
        for bad in [
            "",
            "0".repeat(32).as_str(),
            "ABCDEF0123456789abcdef0123456789ab",
            "short",
        ] {
            assert!(
                parse_unit_observation(
                    UNIT,
                    running()
                        .replace(
                            "InvocationID=0123456789abcdef0123456789abcdef",
                            &format!("InvocationID={bad}")
                        )
                        .as_bytes()
                )
                .is_err()
            );
        }
        for bad in [
            format!("/../{UNIT}"),
            format!("/app.slice//{UNIT}"),
            format!("app.slice/{UNIT}"),
            "/app.slice/other.service".into(),
        ] {
            let source = running();
            let original = parse_unit_observation(UNIT, source.as_bytes())
                .unwrap()
                .cgroup;
            assert!(
                parse_unit_observation(UNIT, source.replace(original, &bad).as_bytes()).is_err()
            );
        }
    }

    use std::collections::VecDeque;
    use std::io::ErrorKind;
    use std::os::fd::BorrowedFd;
    use std::os::unix::net::UnixStream;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct Endless {
        descriptor: UnixStream,
        calls: Arc<AtomicUsize>,
        interrupted: bool,
    }
    impl AsFd for Endless {
        fn as_fd(&self) -> BorrowedFd<'_> {
            self.descriptor.as_fd()
        }
    }
    impl Read for Endless {
        fn read(&mut self, bytes: &mut [u8]) -> std::io::Result<usize> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            if self.interrupted {
                return Err(ErrorKind::Interrupted.into());
            }
            bytes.fill(b'x');
            Ok(bytes.len())
        }
    }

    struct FilterWriter {
        descriptor: UnixStream,
        answers: VecDeque<std::io::Result<usize>>,
        writes: Arc<Mutex<Vec<Vec<u8>>>>,
        dropped: Arc<AtomicUsize>,
    }
    impl AsFd for FilterWriter {
        fn as_fd(&self) -> BorrowedFd<'_> {
            self.descriptor.as_fd()
        }
    }
    impl Write for FilterWriter {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.writes.lock().unwrap().push(bytes.to_vec());
            self.answers.pop_front().unwrap_or(Ok(bytes.len()))
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    impl Drop for FilterWriter {
        fn drop(&mut self) {
            self.dropped.fetch_add(1, Ordering::SeqCst);
        }
    }

    #[test]
    fn output_empty_exact_limit_and_no_eof_are_distinct() {
        let (reader, mut writer) = UnixStream::pair().unwrap();
        let mut capture = Capture::new(reader, 4).unwrap();
        capture.poll().unwrap();
        assert!(!capture.eof);
        writer.write_all(b"1234").unwrap();
        capture.poll().unwrap();
        assert_eq!(capture.retained, b"1234");
        assert_eq!(capture.total, 4);
        assert!(!capture.eof);
        drop(writer);
        capture.poll().unwrap();
        assert!(capture.eof);
        assert_eq!(
            <[u8; 32]>::from(capture.digest.finalize()),
            digest_bytes(b"1234")
        );
        let (reader, writer) = UnixStream::pair().unwrap();
        let mut empty = Capture::new(reader, 1).unwrap();
        drop(writer);
        empty.poll().unwrap();
        assert!(empty.eof && empty.retained.is_empty() && empty.total == 0);
    }

    #[test]
    fn output_over_limit_refuses_instead_of_returning_truncated_success() {
        let (reader, mut writer) = UnixStream::pair().unwrap();
        writer.write_all(b"12345").unwrap();
        let mut capture = Capture::new(reader, 4).unwrap();
        assert_eq!(
            capture.poll().unwrap_err().kind(),
            LinuxSandboxErrorKind::OutputLimitExceeded
        );
        assert_eq!(capture.retained, b"1234");
        assert_eq!(capture.total, 5);
        drop(writer);
        assert_eq!(
            capture.poll().unwrap_err().kind(),
            LinuxSandboxErrorKind::OutputLimitExceeded
        );
    }

    #[test]
    fn cleanup_requires_complete_closed_cgroup_population_observation() {
        assert!(parse_group_empty(b"populated 0\nfrozen 0\n").unwrap());
        assert!(parse_group_empty(b"frozen 1\npopulated 0\n").unwrap());
        assert!(!parse_group_empty(b"populated 1\nfrozen 0\n").unwrap());
        for bytes in [
            b"".as_slice(),
            b"populated 0\n",
            b"frozen 0\n",
            b"populated 0\nfrozen 0",
            b"populated 0\nfrozen 0\npopulated 1\n",
            b"populated 0\nfrozen 0\nunknown 0\n",
            b"populated 00\nfrozen 0\n",
            b"populated 0\nfrozen 0\0\n",
            &[0xff],
        ] {
            assert!(parse_group_empty(bytes).is_err());
        }
        assert!(parse_group_empty(&[b'x'; 257]).is_err());
    }

    #[test]
    fn continuous_output_or_interruptions_cannot_starve_other_supervisor_checks() {
        for interrupted in [false, true] {
            let (descriptor, _peer) = UnixStream::pair().unwrap();
            let calls = Arc::new(AtomicUsize::new(0));
            let mut capture = Capture::new(
                Endless {
                    descriptor,
                    calls: calls.clone(),
                    interrupted,
                },
                super::super::MAX_OUTPUT_BYTES,
            )
            .unwrap();
            capture.poll().unwrap();
            assert_eq!(calls.load(Ordering::SeqCst), PIPE_POLLS);
            assert_eq!(
                capture.total,
                if interrupted {
                    0
                } else {
                    PIPE_POLLS * PIPE_CHUNK
                }
            );
            assert!(!capture.eof);
        }
    }

    #[test]
    fn complete_filter_is_withheld_then_atomically_retried_without_prefix_writes() {
        let (descriptor, _peer) = UnixStream::pair().unwrap();
        let writes = Arc::new(Mutex::new(Vec::new()));
        let dropped = Arc::new(AtomicUsize::new(0));
        let mut filter = WithheldFilter {
            writer: Some(FilterWriter {
                descriptor,
                answers: VecDeque::from([
                    Err(ErrorKind::WouldBlock.into()),
                    Err(ErrorKind::Interrupted.into()),
                    Ok(16),
                ]),
                writes: writes.clone(),
                dropped: dropped.clone(),
            }),
            state: FilterState::Withheld,
        };
        filter.initialize().unwrap();
        assert!(writes.lock().unwrap().is_empty());
        let policy = [7; 16];
        assert!(!filter.release(&policy).unwrap());
        assert!(!filter.release(&policy).unwrap());
        assert_eq!(dropped.load(Ordering::SeqCst), 0);
        assert!(filter.release(&policy).unwrap());
        assert!(filter.release(&policy).unwrap());
        assert_eq!(*writes.lock().unwrap(), vec![policy.to_vec(); 3]);
        assert_eq!(dropped.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn malformed_or_short_filter_never_retries_or_closes_partial_policy() {
        for (policy, answer) in [
            (vec![], 0),
            (vec![1; 7], 7),
            (vec![1; ATOMIC_FILTER_BYTES + 8], 8),
            (vec![1; 16], 8),
        ] {
            let (descriptor, _peer) = UnixStream::pair().unwrap();
            let writes = Arc::new(Mutex::new(Vec::new()));
            let dropped = Arc::new(AtomicUsize::new(0));
            let mut filter = WithheldFilter {
                writer: Some(FilterWriter {
                    descriptor,
                    answers: VecDeque::from([Ok(answer)]),
                    writes: writes.clone(),
                    dropped: dropped.clone(),
                }),
                state: FilterState::Withheld,
            };
            filter.initialize().unwrap();
            assert!(filter.release(&policy).is_err());
            let first_count = writes.lock().unwrap().len();
            assert!(filter.release(&[1; 16]).is_err());
            assert_eq!(first_count, writes.lock().unwrap().len());
            assert_eq!(dropped.load(Ordering::SeqCst), 0);
            assert!(filter.writer.is_some());
            drop(filter); // only the enclosing owner can decide cleanup permits Drop
            assert_eq!(dropped.load(Ordering::SeqCst), 1);
        }
    }

    #[test]
    fn owned_control_exit_pressure_and_expired_deadline_are_bounded_and_reaped() {
        fn fixture(program: &str, arguments: &[&str]) -> Control {
            let child = Command::new(program)
                .env_clear()
                .args(arguments)
                .stdin(Stdio::null())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .unwrap();
            let mut control = Control {
                child,
                stdout: None,
                stderr: None,
                status: None,
            };
            control.initialize().unwrap();
            control
        }
        let mut short = fixture("/usr/bin/true", &[]);
        let deadline = Instant::now() + Duration::from_secs(1);
        while !short.poll().unwrap() && Instant::now() < deadline {
            std::thread::sleep(POLL_TIME);
        }
        let completed = short.status.is_some_and(|status| status.success());
        assert!(short.abort_and_reap(Instant::now() + REAP_TIME).unwrap());
        assert!(completed);
        assert!(short.stdout.as_ref().unwrap().eof && short.stderr.as_ref().unwrap().eof);
        assert!(short.abort_and_reap(Instant::now()).unwrap()); // cached status, no new signal

        let mut pressure = fixture("/usr/bin/yes", &[]);
        let deadline = Instant::now() + Duration::from_secs(1);
        let mut exceeded = false;
        while Instant::now() < deadline {
            match pressure.poll() {
                Err(error) => {
                    exceeded = error.kind() == LinuxSandboxErrorKind::OutputLimitExceeded;
                    break;
                }
                Ok(_) => std::thread::sleep(POLL_TIME),
            }
        }
        assert!(pressure.abort_and_reap(Instant::now() + REAP_TIME).unwrap());
        assert!(exceeded);
        assert!(pressure.stdout.as_ref().unwrap().retained.len() <= CONTROL_BYTES);

        let mut hung = fixture("/usr/bin/sleep", &["10"]);
        // An expired execution deadline still triggers a bounded try_wait/kill;
        // the same retained child is subsequently reaped during teardown.
        let _ = hung.abort_and_reap(Instant::now()).unwrap();
        assert!(hung.abort_and_reap(Instant::now() + REAP_TIME).unwrap());
        assert!(hung.status.is_some());
    }

    #[test]
    fn generated_unit_names_match_the_closed_supervisor_identity_contract() {
        let mut observed = std::collections::BTreeSet::new();
        for _ in 0..16 {
            let name = super::super::random_unit_name().unwrap();
            assert!(unit_name(&name));
            assert!(observed.insert(name));
        }
    }

    #[test]
    fn group_pin_requires_same_running_generation_on_both_sides_of_open() {
        fn observation() -> Observation {
            Observation {
                stage: UnitStage::Running,
                invocation: "0123456789abcdef0123456789abcdef".into(),
                cgroup: format!("/user.slice/user-1000.slice/user@1000.service/app.slice/{UNIT}"),
            }
        }
        let first = observation();
        assert!(same_running_generation(&first, &observation()));
        for stage in [
            UnitStage::Absent,
            UnitStage::Pending,
            UnitStage::Draining,
            UnitStage::Stopping,
            UnitStage::Terminal,
        ] {
            let mut changed = observation();
            changed.stage = stage;
            assert!(!same_running_generation(&first, &changed));
            assert!(!same_running_generation(&changed, &first));
        }
        for invocation in ["", "fedcba9876543210fedcba9876543210"] {
            let mut changed = observation();
            changed.invocation = invocation.into();
            assert!(!same_running_generation(&first, &changed));
            assert!(!same_running_generation(&changed, &first));
        }
        let mut changed = observation();
        changed.cgroup =
            format!("/user.slice/user-1000.slice/user@1000.service/other.slice/{UNIT}");
        assert!(!same_running_generation(&first, &changed));
        changed.cgroup.clear();
        assert!(!same_running_generation(&first, &changed));
        // Malformed synthetic observations cannot establish identity by equality.
        assert!(!same_running_generation(&changed, &changed));
        changed = observation();
        changed.invocation.clear();
        assert!(!same_running_generation(&changed, &changed));
    }

    #[test]
    fn admission_snapshot_is_nonblocking_read_only_and_refuses_retained_or_poisoned_state() {
        // Isolated state only: never poison or manufacture a production attempt.
        let slot = Mutex::new(None::<()>);
        assert!(admission_snapshot_of(&slot).is_ok());
        let held = slot.lock().unwrap();
        assert_eq!(
            admission_snapshot_of(&slot).unwrap_err().kind(),
            LinuxSandboxErrorKind::ExecutionFailed
        );
        drop(held);
        *slot.lock().unwrap() = Some(());
        assert_eq!(
            admission_snapshot_of(&slot).unwrap_err().kind(),
            LinuxSandboxErrorKind::CleanupUncertain
        );
        assert!(slot.lock().unwrap().is_some()); // inspection never releases ownership
        let poisoned = Mutex::new(None::<()>);
        assert!(
            std::panic::catch_unwind(|| {
                let _held = poisoned.lock().unwrap();
                panic!("isolated test owner poison");
            })
            .is_err()
        );
        assert_eq!(
            admission_snapshot_of(&poisoned).unwrap_err().kind(),
            LinuxSandboxErrorKind::CleanupUncertain
        );
        assert!(poisoned.is_poisoned());
    }

    #[test]
    fn fixed_offline_filter_fits_atomic_pipe_limit_without_changing_policy() {
        let policy = super::super::compile_seccomp_policy().unwrap();
        assert!(!policy.is_empty());
        assert_eq!(policy.len() % 8, 0);
        assert!(policy.len() <= ATOMIC_FILTER_BYTES);
    }
}
