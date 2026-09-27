//! Native public-research adapter of the existing sandbox owner.
//! Only the pinned trusted worker mediates DNS/peer/TLS/destinations. The OS profile
//! isolates files/processes/syscalls; it does NOT claim IP/domain filtering.

use std::fmt;
use std::os::fd::AsRawFd;
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::time::{Duration, Instant};

use agentmage_kernel_contracts::{OperationOutcome, StateChange};
use agentmage_kernel_engine::authority_transaction::{
    EffectAuthorization, EffectLaunch, EffectResult,
};
use agentmage_kernel_engine::propagation::{
    EffectCancellationObservation, ScopedEffectCancellation,
};
use agentmage_kernel_engine::research_dispatch::{ResearchDispatch, ResearchEffectDriver};
use agentmage_kernel_engine::research_effect_binding::PublicGetEffectBinding;
use agentmage_kernel_engine::research_fetch::PublicGetWorkerPacket;
use agentmage_kernel_engine::research_response::PublicGetResponse;
use agentmage_kernel_engine::research_result_binding::{
    PublicGetNativeIdentity, PublicGetParentInterval, PublicGetResultBinding,
};
use agentmage_kernel_engine::runtime_loop::RuntimeClock;
use rustix::fd::OwnedFd;
use rustix::fs::{FileType, Mode, OFlags, fstat, open, openat};
use rustix::process::getuid;
use serde_json::json;

use super::{
    DevelopmentWorkerKind, LinuxSandboxError, LinuxSandboxErrorKind, LinuxSandboxLimits,
    LinuxWorkerRuntimeFile, SandboxCancellation, VerifiedArtifact, descriptor_bytes,
    development_worker_artifact_kind, digest_bytes, error, hash_descriptor, hex_digest,
    research_resolver, research_seccomp, revalidate_launch_artifact, sealed_payload, supervision,
    verify_artifact,
};

const WORKER: &str = "agentmage-public-research-worker";
const WORKER_GUEST: &str = "/app/agentmage-public-research-worker";
const PROFILE: &str = "agentmage.linux.public-research.trusted-worker.v1";

/// Exact descriptor-held worker/runtime/resolver manifest, not effect permission.
/// The connected profile is separate from the ordinary offline read worker.
pub struct LinuxPublicResearchManifest {
    systemd_run: VerifiedArtifact,
    systemctl: VerifiedArtifact,
    path_executor: VerifiedArtifact,
    bubblewrap: VerifiedArtifact,
    worker: VerifiedArtifact,
    runtime_files: Vec<VerifiedArtifact>,
    resolv_conf: OwnedFd,
    nsswitch_conf: OwnedFd,
    sha256: String,
}

impl fmt::Debug for LinuxPublicResearchManifest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LinuxPublicResearchManifest")
            .field("sha256", &self.sha256)
            .finish_non_exhaustive()
    }
}

impl LinuxPublicResearchManifest {
    /// Pins a root-owned exact research executable and root-owned runtime closure.
    /// Matching bytes is necessary, not sufficient for native capability admission.
    pub fn verify(
        worker: &Path,
        expected_worker_sha256: [u8; 32],
        runtime_files: &[LinuxWorkerRuntimeFile],
    ) -> Result<Self, LinuxSandboxError> {
        if worker.file_name().and_then(|name| name.to_str()) != Some(WORKER) {
            return Err(invalid());
        }
        let worker = verify_artifact(worker, Some(Path::new(WORKER_GUEST)), false)?;
        Self::with_worker(worker, expected_worker_sha256, runtime_files)
    }

    /// Pins only the exact host sibling under explicit disposable development activation.
    /// This does not relax production root ownership or make a profile qualified.
    pub fn verify_development_worker(
        _platform: &crate::LinuxDevelopmentPlatformAdapter,
        expected_worker_sha256: [u8; 32],
    ) -> Result<Self, LinuxSandboxError> {
        let current = std::env::current_exe()
            .and_then(std::fs::canonicalize)
            .map_err(|_| invalid())?;
        if current.file_name().and_then(|name| name.to_str()) != Some("agentmage-host") {
            return Err(invalid());
        }
        let worker = development_worker_artifact_kind(
            &current.with_file_name(WORKER),
            DevelopmentWorkerKind::PublicResearch,
        )?;
        let runtime_files = ["libgcc_s.so.1", "libc.so.6", "ld-linux-x86-64.so.2"]
            .into_iter()
            .map(|name| {
                let host = std::fs::canonicalize(Path::new("/usr/lib64").join(name))
                    .map_err(|_| invalid())?;
                Ok(LinuxWorkerRuntimeFile::new(
                    host,
                    Path::new("/lib64").join(name),
                ))
            })
            .collect::<Result<Vec<_>, LinuxSandboxError>>()?;
        Self::with_worker(worker, expected_worker_sha256, &runtime_files)
    }

    fn with_worker(
        worker: VerifiedArtifact,
        expected: [u8; 32],
        runtime_files: &[LinuxWorkerRuntimeFile],
    ) -> Result<Self, LinuxSandboxError> {
        if expected == [0; 32]
            || worker.sha256 != expected
            || runtime_files.len() > super::MAX_RUNTIME_FILES
            || worker.guest_path.as_deref() != Some(Path::new(WORKER_GUEST))
        {
            return Err(invalid());
        }
        let launcher = |path| {
            let path = std::fs::canonicalize(path).map_err(|_| invalid())?;
            verify_artifact(&path, None, true)
        };
        let systemd_run = launcher("/usr/bin/systemd-run")?;
        let systemctl = launcher(super::SYSTEMCTL)?;
        let path_executor = launcher(super::PATH_EXECUTOR)?;
        let bubblewrap = launcher("/usr/bin/bwrap")?;
        let mut verified_runtime: Vec<VerifiedArtifact> = Vec::new();
        for file in runtime_files {
            if !super::valid_runtime_guest_path(&file.guest_path)
                || verified_runtime
                    .iter()
                    .any(|entry| entry.guest_path.as_ref() == Some(&file.guest_path))
            {
                return Err(invalid());
            }
            verified_runtime.push(verify_artifact(
                &file.host_path,
                Some(&file.guest_path),
                false,
            )?);
        }
        // Resolve only the fixed system resolver configuration, never a caller's
        // workspace, environment or alternate NSS path. Hold/recheck its full bytes.
        let resolver = system_resolver_artifact()?;
        let source = descriptor_bytes(&resolver.descriptor, 16 * 1024)?;
        if digest_bytes(&source) != resolver.sha256 {
            return Err(invalid());
        }
        let projected =
            research_resolver::ResolverProjection::prepare(&source).map_err(|_| invalid())?;
        let resolv_conf = sealed_payload("research-resolver", projected.resolv_conf())?;
        let nsswitch_conf = sealed_payload("research-nss", projected.nsswitch_conf())?;
        let launchers = [&systemd_run, &systemctl, &path_executor, &bubblewrap]
            .map(|file| hex_digest(&file.sha256));
        let material = json!({
            "schema_version": 1, "kind": PROFILE,
            "worker": hex_digest(&worker.sha256),
            "launchers": launchers,
            "runtime": verified_runtime.iter().map(|file| json!({"guest": file.guest_path, "sha256": hex_digest(&file.sha256)})).collect::<Vec<_>>(),
            "resolver_source_sha256": hex_digest(&resolver.sha256),
            "resolver_projection_sha256": hex_digest(&digest_bytes(projected.resolv_conf())),
            "nss_projection_sha256": hex_digest(&digest_bytes(projected.nsswitch_conf())),
        });
        let sha256 = hex_digest(&digest_bytes(
            &serde_json::to_vec(&material).map_err(|_| invalid())?,
        ));
        // Native test evidence only: identities, never resolver/request contents.
        #[cfg(test)]
        eprintln!("research-native-manifest={material} sha256={sha256}");
        Ok(Self {
            systemd_run,
            systemctl,
            path_executor,
            bubblewrap,
            worker,
            runtime_files: verified_runtime,
            resolv_conf,
            nsswitch_conf,
            sha256,
        })
    }

    pub(crate) fn control_path(&self) -> Result<&Path, LinuxSandboxError> {
        revalidate_launch_artifact(&self.systemctl)?;
        self.systemctl.launch_path.as_deref().ok_or_else(invalid)
    }

    fn revalidate(&self) -> Result<(), LinuxSandboxError> {
        for launcher in [
            &self.systemd_run,
            &self.systemctl,
            &self.path_executor,
            &self.bubblewrap,
        ] {
            revalidate_launch_artifact(launcher)?;
        }
        for file in std::iter::once(&self.worker).chain(&self.runtime_files) {
            let stat = fstat(&file.descriptor).map_err(|_| invalid())?;
            if hash_descriptor(&file.descriptor, stat.st_size)? != file.sha256 {
                return Err(invalid());
            }
        }
        Ok(())
    }
}

/// Resolver data is not executable trust. The fixed system resolver may be managed
/// by systemd-resolved rather than root. Only its two exact published files under
/// the root-delegated runtime directory are admitted; no UID range, NSS lookup,
/// caller-supplied path, runtime library or executable inherits this exception.
fn system_resolver_artifact() -> Result<VerifiedArtifact, LinuxSandboxError> {
    use std::os::unix::fs::MetadataExt;
    super::verify_root_owned_path(Path::new("/etc"))?;
    let link = std::fs::symlink_metadata("/etc/resolv.conf").map_err(|_| invalid())?;
    if link.uid() != 0 || (!link.file_type().is_symlink() && link.mode() & 0o022 != 0) {
        return Err(invalid());
    }
    let path = std::fs::canonicalize("/etc/resolv.conf").map_err(|_| invalid())?;
    if !matches!(
        path.to_str(),
        Some("/run/systemd/resolve/stub-resolv.conf" | "/run/systemd/resolve/resolv.conf")
    ) {
        return verify_artifact(&path, None, false);
    }
    // A trusted root-owned parent delegates this one directory to the service.
    // Hold both directories and open relative to them with NOFOLLOW. Do not trust
    // a same-user runtime directory or chase links inside the service directory.
    super::verify_root_owned_path(Path::new("/run/systemd"))?;
    let flags = OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::CLOEXEC | OFlags::NONBLOCK;
    let parent =
        open("/run/systemd", flags | OFlags::DIRECTORY, Mode::empty()).map_err(|_| invalid())?;
    let parent_stat = fstat(&parent).map_err(|_| invalid())?;
    if parent_stat.st_uid != 0 || parent_stat.st_mode & 0o022 != 0 {
        return Err(invalid());
    }
    let directory = openat(&parent, "resolve", flags | OFlags::DIRECTORY, Mode::empty())
        .map_err(|_| invalid())?;
    let directory_stat = fstat(&directory).map_err(|_| invalid())?;
    let descriptor = openat(
        &directory,
        path.file_name().ok_or_else(invalid)?,
        flags,
        Mode::empty(),
    )
    .map_err(|_| invalid())?;
    let before = fstat(&descriptor).map_err(|_| invalid())?;
    if !valid_delegated_resolver(
        directory_stat.st_uid,
        directory_stat.st_mode,
        before.st_uid,
        before.st_mode,
        before.st_size,
        getuid().as_raw(),
    ) {
        return Err(invalid());
    }
    let bytes = descriptor_bytes(&descriptor, 16 * 1024)?;
    let sha256 = digest_bytes(&bytes);
    let after = fstat(&descriptor).map_err(|_| invalid())?;
    if before.st_size != after.st_size
        || before.st_uid != after.st_uid
        || before.st_mode != after.st_mode
        || before.st_mtime != after.st_mtime
        || before.st_mtime_nsec != after.st_mtime_nsec
        || before.st_ctime != after.st_ctime
        || before.st_ctime_nsec != after.st_ctime_nsec
        || hash_descriptor(&descriptor, after.st_size)? != sha256
    {
        return Err(invalid());
    }
    Ok(VerifiedArtifact {
        descriptor,
        guest_path: None,
        launch_path: None,
        sha256,
        sealed_snapshot: false,
    })
}

fn valid_delegated_resolver(
    directory_uid: u32,
    directory_mode: u32,
    file_uid: u32,
    file_mode: u32,
    size: i64,
    caller_uid: u32,
) -> bool {
    directory_uid != caller_uid
        && directory_uid != 65534
        && directory_uid == file_uid
        && FileType::from_raw_mode(directory_mode) == FileType::Directory
        && FileType::from_raw_mode(file_mode) == FileType::RegularFile
        && directory_mode & 0o022 == 0
        && file_mode & 0o022 == 0
        && (1..=16 * 1024).contains(&size)
}

/// Inert native runner. Raw execution is private and requires the dual-proof driver.
pub struct LinuxPublicResearchRunner {
    manifest: Arc<LinuxPublicResearchManifest>,
    limits: LinuxSandboxLimits,
    policy: Vec<u8>,
    identity: PublicGetNativeIdentity,
}

impl fmt::Debug for LinuxPublicResearchRunner {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LinuxPublicResearchRunner")
            .field("identity", &self.identity)
            .finish_non_exhaustive()
    }
}

impl LinuxPublicResearchRunner {
    /// Prepares exact confinement material without consuming permission or launching.
    pub fn new(
        manifest: LinuxPublicResearchManifest,
        limits: LinuxSandboxLimits,
    ) -> Result<Self, LinuxSandboxError> {
        // Development memfds are copied by --ro-bind-data inside this same memory
        // budget. Refuse an already impossible image before creating a unit. This
        // is a lower bound, not a guarantee that the remaining workload fits.
        let worker_size = fstat(&manifest.worker.descriptor)
            .map_err(|_| invalid())?
            .st_size;
        if manifest.worker.sealed_snapshot && !copied_worker_fits(worker_size, limits.memory_bytes)
        {
            return Err(error(LinuxSandboxErrorKind::InvalidLimit));
        }
        let policy = research_seccomp::compile()?;
        let confinement = json!({"schema_version": 1, "profile": PROFILE,
            "syscall_policy": research_seccomp::POLICY_ID, "syscall_sha256": hex_digest(&digest_bytes(&policy)),
            "network": "parent-namespace;pinned-trusted-worker-DNS-peer-TLS-mediation;no-OS-IP-filter-claim",
            "memory_bytes": limits.memory_bytes, "tasks": limits.task_count,
            "cpu_percent": limits.cpu_percent, "runtime_seconds": limits.runtime_seconds,
            "output_bytes": limits.output_bytes, "swap_bytes": 0 });
        let identity = PublicGetNativeIdentity {
            worker_sha256: hex_digest(&manifest.worker.sha256),
            manifest_sha256: manifest.sha256.clone(),
            confinement_sha256: hex_digest(&digest_bytes(
                &serde_json::to_vec(&confinement).map_err(|_| invalid())?,
            )),
        };
        #[cfg(test)]
        eprintln!("research-native-confinement={confinement} identity={identity:?}");
        Ok(Self {
            manifest: Arc::new(manifest),
            limits,
            policy,
            identity,
        })
    }

    fn run(
        &self,
        packet: &PublicGetWorkerPacket,
        deadline: Instant,
        cancellation: &SandboxCancellation<'_>,
    ) -> Result<supervision::SupervisedResult, LinuxSandboxError> {
        self.manifest.revalidate()?;
        let maximum = PublicGetResponse::maximum_frame_bytes(packet);
        if maximum > self.limits.output_bytes as u64 || maximum > super::MAX_OUTPUT_BYTES as u64 {
            return Err(error(LinuxSandboxErrorKind::InvalidLimit));
        }
        let projections = vec![
            sealed_payload("research-request", packet.bytes())?,
            self.manifest
                .resolv_conf
                .try_clone()
                .map_err(|_| invalid())?,
            self.manifest
                .nsswitch_conf
                .try_clone()
                .map_err(|_| invalid())?,
        ];
        let filter_pipe = supervision::FilterPipe::new()?;
        let unit =
            super::random_unit_name()?.replacen("agentmage-worker-", "agentmage-research-", 1);
        let parent = std::process::id();
        let descriptor = |fd| format!("/proc/{parent}/fd/{fd}");
        let runtime = format!("/run/user/{}", getuid().as_raw());
        let mut command = Command::new(
            self.manifest
                .systemd_run
                .launch_path
                .as_deref()
                .ok_or_else(invalid)?,
        );
        command
            .env_clear()
            .env("XDG_RUNTIME_DIR", &runtime)
            .env(
                "DBUS_SESSION_BUS_ADDRESS",
                format!("unix:path={runtime}/bus"),
            )
            .args([
                "--user",
                "--wait",
                "--collect",
                "--quiet",
                "--pipe",
                "--expand-environment=no",
            ])
            .arg(format!("--unit={unit}"))
            .args([
                "--property=Type=exec",
                "--property=ExitType=cgroup",
                "--property=Restart=no",
                "--property=KillMode=control-group",
                "--property=TimeoutStopSec=1s",
                "--property=SendSIGKILL=yes",
                "--property=NoNewPrivileges=yes",
                "--property=RestrictSUIDSGID=yes",
                "--property=LockPersonality=yes",
                "--property=RestrictAddressFamilies=AF_UNIX AF_NETLINK AF_INET AF_INET6",
                "--property=MemorySwapMax=0",
            ])
            .args(self.limits.start_timeout_properties())
            .arg(format!("--property=MemoryMax={}", self.limits.memory_bytes))
            .arg(format!("--property=TasksMax={}", self.limits.task_count))
            .arg(format!("--property=CPUQuota={}%", self.limits.cpu_percent))
            .arg(format!(
                "--property=RuntimeMaxSec={}s",
                self.limits.runtime_seconds.saturating_add(1)
            ));
        // Fixed descriptors: request3/resolver4/NSS5/filter6/worker7/runtime8+.
        for (index, projection) in projections.iter().enumerate() {
            command.arg(format!(
                "--property=OpenFile={}:projection-{index}:read-only",
                descriptor(projection.as_raw_fd())
            ));
        }
        command
            .arg(format!(
                "--property=OpenFile={}:filter:read-only",
                descriptor(filter_pipe.read_descriptor())
            ))
            .arg(format!(
                "--property=OpenFile={}:worker:read-only",
                descriptor(self.manifest.worker.descriptor.as_raw_fd())
            ));
        for (index, file) in self.manifest.runtime_files.iter().enumerate() {
            command.arg(format!(
                "--property=OpenFile={}:runtime-{index}:read-only",
                descriptor(file.descriptor.as_raw_fd())
            ));
        }
        command
            .arg(
                self.manifest
                    .path_executor
                    .launch_path
                    .as_deref()
                    .ok_or_else(invalid)?,
            )
            .arg("--")
            .arg(
                self.manifest
                    .bubblewrap
                    .launch_path
                    .as_deref()
                    .ok_or_else(invalid)?,
            )
            .args([
                "--unshare-all",
                "--share-net",
                "--unshare-user",
                "--disable-userns",
                "--new-session",
                "--die-with-parent",
                "--clearenv",
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
                "--dir",
                "/etc",
                "--ro-bind-data",
                "3",
                "/input/request",
                "--ro-bind-data",
                "4",
                "/etc/resolv.conf",
                "--ro-bind-data",
                "5",
                "/etc/nsswitch.conf",
            ]);
        if self.manifest.worker.sealed_snapshot {
            command.args(["--perms", "0500", "--ro-bind-data"]);
        } else {
            command.arg("--ro-bind-fd");
        }
        command.args(["7", WORKER_GUEST]);
        for (index, file) in self.manifest.runtime_files.iter().enumerate() {
            command
                .arg("--ro-bind-fd")
                .arg((8 + index).to_string())
                .arg(file.guest_path.as_deref().ok_or_else(invalid)?);
        }
        command
            .args(["--chdir", "/input", "--seccomp", "6", "--", WORKER_GUEST])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        supervision::run_owned(
            command,
            supervision::Launch {
                owner: supervision::LaunchOwner::Research(self.manifest.clone()),
                projections,
                unit,
                deadline,
                stdout_limit: maximum as usize,
                stderr_limit: self.limits.output_bytes,
                filter_pipe: Some(filter_pipe),
            },
            &self.policy,
            cancellation,
        )
    }
}

fn copied_worker_fits(image_bytes: i64, memory_bytes: u64) -> bool {
    u64::try_from(image_bytes).is_ok_and(|size| size > 0 && size < memory_bytes)
}

fn native_output_succeeded(result: &supervision::SupervisedResult) -> bool {
    matches!(result.outcome, Ok(OperationOutcome::Succeeded))
        && result.output_complete
        && result
            .status
            .as_ref()
            .is_some_and(|status| status.success())
        && result.stderr.total == 0
        && result.stderr.retained.is_empty()
        && result.stdout.total == result.stdout.retained.len()
        && result.stdout.sha256 == digest_bytes(&result.stdout.retained)
}

/// Redacted failure details; cleanup never reverses an attempted external disclosure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LinuxPublicResearchFailure {
    outcome: OperationOutcome,
    reason: &'static str,
    external_disclosure_uncertain: bool,
}

impl LinuxPublicResearchFailure {
    fn effect(&self) -> EffectResult {
        let material = format!(
            "research-native-failure-v1:{:?}:{}:{}",
            self.outcome, self.reason, self.external_disclosure_uncertain
        );
        EffectResult::from_redacted_material(
            if self.external_disclosure_uncertain {
                OperationOutcome::Uncertain
            } else {
                self.outcome
            },
            material.as_bytes(),
            if self.external_disclosure_uncertain {
                StateChange::Uncertain
            } else {
                StateChange::NotChanged
            },
        )
    }

    /// Observed reason, distinct from the conservative authority outcome.
    #[must_use]
    pub const fn outcome(&self) -> OperationOutcome {
        self.outcome
    }
    /// Content-free stable failure category.
    #[must_use]
    pub const fn reason(&self) -> &'static str {
        self.reason
    }
    /// Whether this attempt may have disclosed approved information externally.
    #[must_use]
    pub const fn external_disclosure_uncertain(&self) -> bool {
        self.external_disclosure_uncertain
    }
}

/// Native effect with separate consumed-grant and durable-freshness proofs.
/// No ordinary `EffectDriver` implementation or public raw launch is provided.
pub struct LinuxPublicResearchEffectDriver<'source> {
    runner: LinuxPublicResearchRunner,
    root: &'source crate::LinuxAuthorizedWorkspace,
    binding: PublicGetEffectBinding,
    clock: &'source mut dyn RuntimeClock,
    cancellation: &'source dyn EffectCancellationObservation,
    attempted: bool,
    result: Option<PublicGetResultBinding>,
    failure: Option<LinuxPublicResearchFailure>,
}

impl<'source> LinuxPublicResearchEffectDriver<'source> {
    /// Constructs an inert one-attempt adapter; no authority is issued or retained.
    pub fn new(
        runner: LinuxPublicResearchRunner,
        root: &'source crate::LinuxAuthorizedWorkspace,
        binding: PublicGetEffectBinding,
        clock: &'source mut dyn RuntimeClock,
        cancellation: &'source dyn EffectCancellationObservation,
    ) -> Self {
        Self {
            runner,
            root,
            binding,
            clock,
            cancellation,
            attempted: false,
            result: None,
            failure: None,
        }
    }

    /// Takes full consistent bytes after the canonical owner closes its transaction.
    /// The consumer must still match that actual receipt, producer and runtime events.
    pub fn take_result(&mut self) -> Option<PublicGetResultBinding> {
        self.result.take()
    }
    /// Takes content-free diagnostics, never verified source evidence.
    pub fn take_failure(&mut self) -> Option<LinuxPublicResearchFailure> {
        self.failure.take()
    }

    fn refused(
        &mut self,
        outcome: OperationOutcome,
        reason: &'static str,
        uncertain: bool,
    ) -> EffectLaunch {
        let failure = LinuxPublicResearchFailure {
            outcome,
            reason,
            external_disclosure_uncertain: uncertain,
        };
        let effect = failure.effect();
        self.failure = Some(failure);
        EffectLaunch::completed(effect)
    }
}

impl ResearchEffectDriver for LinuxPublicResearchEffectDriver<'_> {
    fn execute_research(
        &mut self,
        authorization: EffectAuthorization<'_>,
        dispatch: ResearchDispatch<'_>,
    ) -> EffectLaunch {
        if self.attempted {
            return EffectLaunch::failed();
        }
        self.attempted = true;
        let monotonic_started = Instant::now();
        let Ok(started) = self.clock.now_epoch_ms() else {
            return self.refused(OperationOutcome::Failed, "clock", false);
        };
        let packet = self.binding.packet();
        if self
            .binding
            .validate(&authorization, self.root, started)
            .is_err()
            || !dispatch.matches_packet_at(packet, started)
            || self.root.revalidate().is_err()
        {
            return self.refused(OperationOutcome::Denied, "binding", false);
        }
        if PublicGetResponse::maximum_frame_bytes(packet) > self.runner.limits.output_bytes as u64 {
            return self.refused(OperationOutcome::Denied, "output-bound", false);
        }
        let duration = Duration::from_millis(packet.deadline_epoch_ms() - started).min(
            Duration::from_secs(u64::from(self.runner.limits.runtime_seconds)),
        );
        let Some(deadline) = monotonic_started.checked_add(duration) else {
            return self.refused(OperationOutcome::Failed, "deadline", false);
        };
        let cancellation = SandboxCancellation::Bound(ScopedEffectCancellation::new(
            self.cancellation,
            authorization.task_id(),
            &authorization.call().correlation_id,
        ));
        match cancellation.is_cancelled() {
            Ok(false) => (),
            Ok(true) => {
                return self.refused(OperationOutcome::Cancelled, "prelaunch-cancelled", false);
            }
            Err(_) => return self.refused(OperationOutcome::Failed, "prelaunch-control", false),
        }
        // Beyond this boundary a network attempt is possible. Even known cleanup
        // does not establish that approved DNS/TLS/HTTP disclosure was undone.
        let result = match self.runner.run(packet, deadline, &cancellation) {
            Ok(result) => result,
            Err(error) => return self.refused(OperationOutcome::Failed, error.kind().code(), true),
        };
        let outcome = match result.outcome {
            Ok(outcome) => outcome,
            Err(error) => return self.refused(OperationOutcome::Failed, error.kind().code(), true),
        };
        if !native_output_succeeded(&result) {
            return self.refused(
                if outcome == OperationOutcome::Succeeded {
                    OperationOutcome::Failed
                } else {
                    outcome
                },
                "native-nonsuccess",
                true,
            );
        }
        let Ok(completed) = self.clock.now_epoch_ms() else {
            return self.refused(OperationOutcome::Failed, "completion-clock", true);
        };
        if self.root.revalidate().is_err() {
            return self.refused(OperationOutcome::Failed, "workspace-drift", true);
        }
        let binding = match PublicGetResultBinding::seal(
            packet,
            dispatch.reservation_sha256(),
            self.runner.identity.clone(),
            PublicGetParentInterval {
                started_epoch_ms: started,
                cleanup_verified_epoch_ms: completed,
            },
            &result.stdout.retained,
        ) {
            Ok(binding) => binding,
            Err(_) => return self.refused(OperationOutcome::Failed, "response-binding", true),
        };
        let effect = EffectResult::from_redacted_material(
            OperationOutcome::Succeeded,
            binding.redacted_material(),
            // Known external disclosure is irreversible even though GET neither
            // edits this workspace nor promises remote content mutation. The
            // ordinary offline runtime remains unregistered for this operation.
            StateChange::Changed,
        );
        self.result = Some(binding);
        EffectLaunch::completed(effect)
    }
}

fn invalid() -> LinuxSandboxError {
    error(LinuxSandboxErrorKind::InvalidManifest)
}

#[cfg(test)]
mod tests {
    include!("research_sandbox_tests.rs");
    include!("research_dispatch_tests.rs");

    #[test]
    fn native_success_requires_exact_exit_complete_full_output_and_no_stderr() {
        use std::os::unix::process::ExitStatusExt;
        for mutation in 0..9 {
            let stdout = b"synthetic frame checked separately by response binding";
            let mut result = supervision::SupervisedResult {
                outcome: Ok(OperationOutcome::Succeeded),
                status: Some(std::process::ExitStatus::from_raw(0)),
                stdout: supervision::CapturedStream {
                    retained: stdout.to_vec(),
                    sha256: digest_bytes(stdout),
                    total: stdout.len(),
                },
                stderr: supervision::CapturedStream {
                    retained: vec![],
                    sha256: digest_bytes(&[]),
                    total: 0,
                },
                resources: None,
                output_complete: true,
            };
            match mutation {
                0 => (),
                1 => result.outcome = Ok(OperationOutcome::Failed),
                2 => result.status = None,
                3 => result.status = Some(std::process::ExitStatus::from_raw(5 << 8)),
                4 => result.output_complete = false,
                5 => result.stderr.total = 1,
                6 => result.stderr.retained = vec![b'!'],
                7 => result.stdout.total += 1,
                8 => result.stdout.sha256 = [0; 32],
                _ => unreachable!(),
            }
            assert_eq!(
                native_output_succeeded(&result),
                mutation == 0,
                "mutation {mutation}"
            );
        }
    }
}
