//! Bounded pre-workspace probes for mandatory Linux security controls.

use std::fs::File;
use std::io::{Read, Write};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use agentmage_kernel_contracts::PlatformCapabilityStatus;
use rustix::process::getuid;
use rustix::rand::{GetRandomFlags, getrandom};
use sha2::{Digest, Sha256};

use crate::sandbox::compile_seccomp_policy;
use crate::{LinuxSecretService, LinuxSecretServiceManifest, strict_descriptor_paths_available};

const BUBBLEWRAP_PATH: &str = "/usr/bin/bwrap";
const SECRET_TOOL_PATH: &str = "/usr/bin/secret-tool";
const SYSTEMD_RUN_PATH: &str = "/usr/bin/systemd-run";
const PROBE_TIMEOUT: Duration = Duration::from_secs(8);
const MAX_MOUNTINFO_BYTES: u64 = 1024 * 1024;

/// Mandatory Linux controls whose absence must prevent platform activation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LinuxSecurityControl {
    Bubblewrap = 0,
    UserNamespaces = 1,
    Seccomp = 2,
    Cgroups = 3,
    SecretService = 4,
    DescriptorSafePaths = 5,
    NetworkIsolation = 6,
}

pub(crate) const REQUIRED_LINUX_SECURITY_CONTROLS: [LinuxSecurityControl; 7] = [
    LinuxSecurityControl::Bubblewrap,
    LinuxSecurityControl::UserNamespaces,
    LinuxSecurityControl::Seccomp,
    LinuxSecurityControl::Cgroups,
    LinuxSecurityControl::SecretService,
    LinuxSecurityControl::DescriptorSafePaths,
    LinuxSecurityControl::NetworkIsolation,
];

impl LinuxSecurityControl {
    pub(crate) const fn id(self) -> &'static str {
        match self {
            Self::Bubblewrap => "bubblewrap",
            Self::UserNamespaces => "user-namespaces",
            Self::Seccomp => "seccomp",
            Self::Cgroups => "cgroups-v2",
            Self::SecretService => "secret-service",
            Self::DescriptorSafePaths => "descriptor-safe-paths",
            Self::NetworkIsolation => "network-isolation",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct LinuxSecurityControlObservation {
    status: PlatformCapabilityStatus,
    mechanism_sha256: [u8; 32],
}

impl LinuxSecurityControlObservation {
    pub(crate) const fn status(self) -> PlatformCapabilityStatus {
        self.status
    }

    pub(crate) const fn mechanism_sha256(self) -> [u8; 32] {
        self.mechanism_sha256
    }
}

/// Exact seven-control result consumed by Linux aggregate discovery.
#[derive(Clone, Debug)]
pub(crate) struct LinuxSecurityControls {
    observations: [LinuxSecurityControlObservation; REQUIRED_LINUX_SECURITY_CONTROLS.len()],
}

impl LinuxSecurityControls {
    /// Runs fixed, no-input probes without workspace or product authority.
    pub(crate) fn discover(
        bubblewrap_trusted: bool,
        systemd_run_trusted: bool,
        secret_tool_trusted: bool,
    ) -> Self {
        let seccomp_policy = compile_seccomp_policy().ok();
        let bubblewrap_status = available_if(bubblewrap_trusted);
        let user_namespace_status = available_if(
            bubblewrap_trusted && run_bubblewrap_probe(BubblewrapProbe::UserNamespace, None),
        );
        let seccomp_status = available_if(
            bubblewrap_trusted
                && seccomp_policy.as_deref().is_some_and(|policy| {
                    run_bubblewrap_probe(BubblewrapProbe::Seccomp, Some(policy))
                }),
        );
        let network_status = available_if(
            bubblewrap_trusted && run_bubblewrap_probe(BubblewrapProbe::Network, None),
        );
        let cgroup_status =
            available_if(systemd_run_trusted && cgroup_v2_available() && run_cgroup_probe());
        let secret_service_status = available_if(secret_tool_trusted && run_secret_service_probe());
        let descriptor_status = available_if(strict_descriptor_paths_available());
        let seccomp_identity = seccomp_policy
            .as_deref()
            .map(Sha256::digest)
            .map(Into::into)
            .unwrap_or([0; 32]);

        Self {
            observations: [
                control_observation(LinuxSecurityControl::Bubblewrap, bubblewrap_status, None),
                control_observation(
                    LinuxSecurityControl::UserNamespaces,
                    user_namespace_status,
                    None,
                ),
                control_observation(
                    LinuxSecurityControl::Seccomp,
                    seccomp_status,
                    Some(&seccomp_identity),
                ),
                control_observation(LinuxSecurityControl::Cgroups, cgroup_status, None),
                control_observation(
                    LinuxSecurityControl::SecretService,
                    secret_service_status,
                    None,
                ),
                control_observation(
                    LinuxSecurityControl::DescriptorSafePaths,
                    descriptor_status,
                    None,
                ),
                control_observation(LinuxSecurityControl::NetworkIsolation, network_status, None),
            ],
        }
    }

    pub(crate) const fn observation(
        &self,
        control: LinuxSecurityControl,
    ) -> LinuxSecurityControlObservation {
        self.observations[control as usize]
    }

    #[cfg(test)]
    pub(crate) fn all_verified() -> Self {
        let seccomp_identity: [u8; 32] = compile_seccomp_policy()
            .map(|policy| Sha256::digest(policy).into())
            .unwrap_or([0; 32]);
        Self {
            observations: REQUIRED_LINUX_SECURITY_CONTROLS.map(|control| {
                control_observation(
                    control,
                    PlatformCapabilityStatus::Verified,
                    (control == LinuxSecurityControl::Seccomp).then_some(&seccomp_identity),
                )
            }),
        }
    }

    #[cfg(test)]
    pub(crate) fn set_status(
        &mut self,
        control: LinuxSecurityControl,
        status: PlatformCapabilityStatus,
    ) {
        self.observations[control as usize].status = status;
    }
}

#[derive(Clone, Copy)]
enum BubblewrapProbe {
    UserNamespace,
    Seccomp,
    Network,
}

fn run_bubblewrap_probe(probe: BubblewrapProbe, seccomp_policy: Option<&[u8]>) -> bool {
    let mut command = Command::new(BUBBLEWRAP_PATH);
    command.env_clear().args([
        "--unshare-user",
        "--new-session",
        "--die-with-parent",
        "--clearenv",
    ]);
    if matches!(probe, BubblewrapProbe::Network) {
        command.arg("--unshare-net");
    }
    command.args(["--ro-bind", "/", "/"]);
    if matches!(probe, BubblewrapProbe::Seccomp) {
        command.args(["--seccomp", "0"]);
    }
    command
        .args(["--", BUBBLEWRAP_PATH, "--version"])
        .stdin(if seccomp_policy.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    spawn_and_wait(&mut command, seccomp_policy)
}

fn run_cgroup_probe() -> bool {
    let mut random = [0_u8; 12];
    if getrandom(&mut random, GetRandomFlags::empty()).is_err() {
        return false;
    }
    let unit = format!("agentmage-control-probe-{}", hex_digest(&random));
    let runtime_directory = format!("/run/user/{}", getuid().as_raw());
    let session_bus = format!("unix:path={runtime_directory}/bus");
    let mut command = Command::new(SYSTEMD_RUN_PATH);
    command
        .env_clear()
        .env("XDG_RUNTIME_DIR", runtime_directory)
        .env("DBUS_SESSION_BUS_ADDRESS", session_bus)
        .args([
            "--user",
            "--wait",
            "--collect",
            "--quiet",
            "--pipe",
            &format!("--unit={unit}"),
            "--property=NoNewPrivileges=yes",
            "--property=MemoryMax=33554432",
            "--property=MemorySwapMax=0",
            "--property=TasksMax=4",
            "--property=CPUQuota=100%",
            "--property=RuntimeMaxSec=5s",
            SYSTEMD_RUN_PATH,
            "--version",
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    spawn_and_wait(&mut command, None)
}

fn cgroup_v2_available() -> bool {
    let Ok(file) = File::open("/proc/self/mountinfo") else {
        return false;
    };
    let mut mountinfo = String::new();
    if file
        .take(MAX_MOUNTINFO_BYTES + 1)
        .read_to_string(&mut mountinfo)
        .is_err()
        || mountinfo.len() as u64 > MAX_MOUNTINFO_BYTES
    {
        return false;
    }
    mountinfo.lines().any(|line| line.contains(" - cgroup2 "))
}

fn run_secret_service_probe() -> bool {
    LinuxSecretServiceManifest::verify(SECRET_TOOL_PATH)
        .map(LinuxSecretService::new)
        .and_then(|service| service.probe())
        .is_ok()
}

fn spawn_and_wait(command: &mut Command, input: Option<&[u8]>) -> bool {
    let Ok(mut child) = command.spawn() else {
        return false;
    };
    if let Some(bytes) = input {
        let write_succeeded = child
            .stdin
            .take()
            .is_some_and(|mut stdin| stdin.write_all(bytes).is_ok());
        if !write_succeeded {
            terminate(&mut child);
            return false;
        }
    }
    drop(child.stdin.take());
    let deadline = Instant::now() + PROBE_TIMEOUT;
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return status.success(),
            Ok(None) if Instant::now() < deadline => thread::sleep(Duration::from_millis(10)),
            Ok(None) | Err(_) => {
                terminate(&mut child);
                return false;
            }
        }
    }
}

fn terminate(child: &mut Child) {
    let _ = child.kill();
    let _ = child.wait();
}

fn control_observation(
    control: LinuxSecurityControl,
    status: PlatformCapabilityStatus,
    extra_identity: Option<&[u8; 32]>,
) -> LinuxSecurityControlObservation {
    let mut digest = Sha256::new();
    digest.update(b"agentmage.linux-security-control.v1\0");
    digest.update(control.id().as_bytes());
    if let Some(extra_identity) = extra_identity {
        digest.update(extra_identity);
    }
    LinuxSecurityControlObservation {
        status,
        mechanism_sha256: digest.finalize().into(),
    }
}

const fn available_if(available: bool) -> PlatformCapabilityStatus {
    if available {
        PlatformCapabilityStatus::Verified
    } else {
        PlatformCapabilityStatus::Unavailable
    }
}

fn hex_digest(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        use std::fmt::Write as _;
        let _ = write!(output, "{byte:02x}");
    }
    output
}

#[cfg(test)]
mod tests {
    use agentmage_kernel_contracts::PlatformCapabilityStatus;

    use super::{LinuxSecurityControls, REQUIRED_LINUX_SECURITY_CONTROLS};

    #[test]
    fn control_identities_are_nonzero_distinct_and_status_independent() {
        let baseline = LinuxSecurityControls::all_verified();
        let mut identities = Vec::new();
        for control in REQUIRED_LINUX_SECURITY_CONTROLS {
            let observation = baseline.observation(control);
            assert_eq!(observation.status(), PlatformCapabilityStatus::Verified);
            assert_ne!(observation.mechanism_sha256(), [0; 32]);
            identities.push(observation.mechanism_sha256());

            let mut unavailable = baseline.clone();
            unavailable.set_status(control, PlatformCapabilityStatus::Unavailable);
            assert_eq!(
                unavailable.observation(control).mechanism_sha256(),
                observation.mechanism_sha256()
            );
        }
        identities.sort_unstable();
        identities.dedup();
        assert_eq!(identities.len(), REQUIRED_LINUX_SECURITY_CONTROLS.len());
    }

    #[test]
    #[ignore = "requires supported Fedora controls and an unlocked Secret Service session"]
    fn live_required_control_preflight_verifies_without_workspace_authority() {
        let controls = LinuxSecurityControls::discover(true, true, true);
        for control in REQUIRED_LINUX_SECURITY_CONTROLS {
            assert_eq!(
                controls.observation(control).status(),
                PlatformCapabilityStatus::Verified,
                "{}",
                control.id()
            );
        }
    }
}
