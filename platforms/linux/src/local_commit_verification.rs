// SPDX-License-Identifier: Apache-2.0
//! Offline, public-material-only verification of one exact local commit.

use super::*;
use crate::sandbox::supervision::{FilterPipe, Launch, LaunchOwner, SupervisedResult};
use agentmage_kernel_engine::local_commit::LocalCommitPlan;
use rustix::fd::OwnedFd;
use std::os::fd::AsRawFd as _;
use std::sync::Arc;

const GUEST_GPG: &str = "/app/gpg";
const PUBLIC_KEY: &str = "/app/public.gpg";
const SIGNATURE: &str = "/app/signature";
const CONTENT: &str = "/app/content";
const GUEST_HOME: &str = "/tmp/gnupg";
const LIMIT: usize = crate::sandbox::MAX_OUTPUT_BYTES;

// The existing installed-executable verifier checks held bytes and every
// root-owned ancestor. No caller-selected launcher or direct fallback.
pub(crate) struct VerifierManifest {
    systemd_run: LinuxGitArtifact,
    systemctl: LinuxGitArtifact,
    bubblewrap: LinuxGitArtifact,
    gpg: LinuxGitArtifact,
}

impl VerifierManifest {
    fn verify(signer: &LinuxOpenPgpSigner) -> Result<Self, LinuxLocalCommitError> {
        let artifact = |path: &Path| {
            LinuxGitArtifact::verify(path)
                .map_err(|_| error(LinuxLocalCommitErrorKind::ArtifactDenied))
        };
        let manifest = Self {
            systemd_run: artifact(Path::new("/usr/bin/systemd-run"))?,
            systemctl: artifact(Path::new("/usr/bin/systemctl"))?,
            bubblewrap: artifact(Path::new("/usr/bin/bwrap"))?,
            gpg: artifact(&signer.executable)?,
        };
        if manifest.gpg.sha256() != signer.executable_sha256 {
            return Err(error(LinuxLocalCommitErrorKind::ArtifactDenied));
        }
        Ok(manifest)
    }

    pub(crate) fn control_path(&self) -> Result<&Path, LinuxLocalCommitError> {
        for artifact in [
            &self.systemd_run,
            &self.systemctl,
            &self.bubblewrap,
            &self.gpg,
        ] {
            artifact
                .revalidate()
                .map_err(|_| error(LinuxLocalCommitErrorKind::ArtifactDenied))?;
        }
        Ok(self.systemctl.launch_path())
    }
}

pub(super) fn verify(
    signer: &LinuxOpenPgpSigner,
    raw: &[u8],
    plan: &LocalCommitPlan,
    cancellation: &CancellationToken,
    deadline: Instant,
) -> Result<(), LinuxLocalCommitError> {
    verify_with(signer, raw, plan, cancellation, deadline, |payload| {
        let manifest = Arc::new(VerifierManifest::verify(signer)?);
        crate::sandbox::supervision::admission_snapshot()
            .map_err(|e| capture_error(e.kind(), LinuxLocalCommitErrorKind::ExecutionFailed))?;
        let policy = crate::sandbox::compile_seccomp_policy()
            .map_err(|_| error(LinuxLocalCommitErrorKind::ArtifactDenied))?;
        let filter =
            FilterPipe::new().map_err(|_| error(LinuxLocalCommitErrorKind::ExecutionFailed))?;
        let mut nonce = [0_u8; 12];
        rustix::rand::getrandom(&mut nonce, rustix::rand::GetRandomFlags::empty())
            .map_err(|_| error(LinuxLocalCommitErrorKind::ExecutionFailed))?;
        let unit = format!(
            "agentmage-commit-verification-{}.service",
            hex_digest(nonce)
        );
        let command = worker_command(
            manifest.systemd_run.launch_path(),
            manifest.bubblewrap.launch_path(),
            manifest.gpg.held_descriptor().as_raw_fd(),
            &payload,
            &unit,
            &filter,
        );
        manifest
            .control_path()
            .map_err(|_| error(LinuxLocalCommitErrorKind::ArtifactDenied))?;
        crate::sandbox::supervision::run_owned(
            command,
            Launch {
                owner: LaunchOwner::CommitVerification(manifest),
                projections: payload.into(),
                unit,
                deadline,
                stdout_limit: LIMIT,
                stderr_limit: LIMIT,
                filter_pipe: Some(filter),
            },
            &policy,
            &crate::sandbox::SandboxCancellation::Observed(cancellation),
        )
        .map_err(|e| capture_error(e.kind(), LinuxLocalCommitErrorKind::ExecutionFailed))
    })
}

// Deterministic result seam. Product dispatch always uses run_owned above.
// Only immutable public key, signature and content descriptors are projected.
fn verify_with(
    signer: &LinuxOpenPgpSigner,
    raw: &[u8],
    plan: &LocalCommitPlan,
    cancellation: &CancellationToken,
    deadline: Instant,
    dispatch: impl FnOnce([OwnedFd; 3]) -> Result<SupervisedResult, LinuxLocalCommitError>,
) -> Result<(), LinuxLocalCommitError> {
    if cancellation.is_cancelled()
        || Instant::now() >= deadline
        || agentmage_kernel_engine::local_commit::verify_local_commit_plan(plan).is_err()
        || plan.signer != signer.pinned
        || signer.pinned.verify().is_err()
        || signer.public_key.is_empty()
        || signer.public_key.len() > LIMIT
        || hash(&signer.public_key) != signer.pinned.public_key_sha256
        || raw.len() > LIMIT
    {
        return Err(error(LinuxLocalCommitErrorKind::PlanDenied));
    }
    let (signature, content) = split_signed_commit(raw, plan)?;
    let seal = |name, bytes: &[u8]| {
        crate::sandbox::sealed_payload(name, bytes)
            .map_err(|_| error(LinuxLocalCommitErrorKind::ExecutionFailed))
    };
    let result = dispatch([
        seal("agentmage-verification-public", &signer.public_key)?,
        seal("agentmage-verification-signature", &signature)?,
        seal("agentmage-verification-content", &content)?,
    ])?;
    validate_result(result, &plan.signer.key_fingerprint, cancellation, deadline)
}

fn validate_result(
    result: SupervisedResult,
    fingerprint: &str,
    cancellation: &CancellationToken,
    deadline: Instant,
) -> Result<(), LinuxLocalCommitError> {
    if let Err(failure) = result.outcome {
        return Err(capture_error(
            failure.kind(),
            LinuxLocalCommitErrorKind::VerificationFailed,
        ));
    }
    if cancellation.is_cancelled()
        || Instant::now() >= deadline
        || !matches!(result.outcome, Ok(OperationOutcome::Succeeded))
        || !result.output_complete
        || !result.status.is_some_and(|s| s.success())
        || result.stdout.total > LIMIT
        || result.stdout.total != result.stdout.retained.len()
        || result.stderr.total > LIMIT
        || result.stderr.total != result.stderr.retained.len()
        || result.stdout.sha256 != Sha256::digest(&result.stdout.retained).as_slice()
        || result.stderr.sha256 != Sha256::digest(&result.stderr.retained).as_slice()
        || !verify_signature_status(&result.stdout.retained, fingerprint)
    {
        return Err(error(LinuxLocalCommitErrorKind::VerificationFailed));
    }
    Ok(())
}

pub(super) fn split_signed_commit(
    raw: &[u8],
    plan: &LocalCommitPlan,
) -> Result<(Vec<u8>, Vec<u8>), LinuxLocalCommitError> {
    if !verify_raw_commit(raw, plan) {
        return Err(error(LinuxLocalCommitErrorKind::VerificationFailed));
    }
    let separator = raw
        .windows(2)
        .position(|w| w == b"\n\n")
        .ok_or_else(|| error(LinuxLocalCommitErrorKind::VerificationFailed))?;
    let header = if plan.parent_object.len() == 40 {
        b"gpgsig ".as_slice()
    } else {
        b"gpgsig-sha256 ".as_slice()
    };
    let mut signature = Vec::new();
    let mut content = Vec::with_capacity(raw.len());
    let mut in_signature = false;
    for line in raw[..=separator].split_inclusive(|b| *b == b'\n') {
        if let Some(first) = line.strip_prefix(header) {
            in_signature = true;
            signature.extend_from_slice(first);
        } else if in_signature && line.starts_with(b" ") {
            signature.extend_from_slice(&line[1..]);
        } else {
            in_signature = false;
            content.extend_from_slice(line);
        }
    }
    content.extend_from_slice(&raw[separator + 1..]);
    if !signature.starts_with(b"-----BEGIN PGP SIGNATURE-----\n")
        || !signature.ends_with(b"-----END PGP SIGNATURE-----\n")
    {
        return Err(error(LinuxLocalCommitErrorKind::VerificationFailed));
    }
    Ok((signature, content))
}

// Common fixed options for both the worker and disposable cryptographic tests.
fn gpg_arguments<'a>(
    home: &'a str,
    public: &'a str,
    signature: &'a str,
    content: &'a str,
) -> Vec<&'a str> {
    vec![
        "--no-options",
        "--batch",
        "--no-tty",
        "--no-autostart",
        "--no-auto-key-retrieve",
        "--auto-key-locate",
        "clear",
        "--no-auto-check-trustdb",
        "--trust-model",
        "always",
        "--homedir",
        home,
        "--no-default-keyring",
        "--keyring",
        public,
        // Verification emits no content on stdout. Reserve it exclusively for
        // machine status; bounded stderr is diagnostic only and has no authority.
        "--status-fd=1",
        "--verify",
        signature,
        content,
    ]
}

fn worker_command(
    systemd_run: &Path,
    bubblewrap: &Path,
    gpg_descriptor: std::os::fd::RawFd,
    payload: &[OwnedFd; 3],
    unit: &str,
    filter: &FilterPipe,
) -> Command {
    let runtime = format!("/run/user/{}", getuid().as_raw());
    let mut c = Command::new(systemd_run);
    c.env_clear()
        .env("XDG_RUNTIME_DIR", &runtime)
        .env(
            "DBUS_SESSION_BUS_ADDRESS",
            format!("unix:path={runtime}/bus"),
        )
        .args([
            "--user",
            "--wait",
            "--quiet",
            "--pipe",
            "--collect",
            "--expand-environment=no",
        ])
        .arg(format!("--unit={unit}"))
        .args([
            "--property=NoNewPrivileges=yes",
            "--property=RestrictSUIDSGID=yes",
            "--property=LockPersonality=yes",
            "--property=RestrictAddressFamilies=AF_UNIX AF_NETLINK",
            "--property=MemorySwapMax=0",
            "--property=MemoryMax=268435456",
            "--property=TasksMax=16",
            "--property=CPUQuota=100%",
            "--property=RuntimeMaxSec=31s",
            "--property=Restart=no",
            "--property=JobRunningTimeoutSec=30s",
            "--property=TimeoutStartSec=30s",
            "--property=KillMode=control-group",
            "--property=SendSIGKILL=yes",
            "--property=TimeoutStopSec=2s",
        ]);
    // Exact allocation: 3:GnuPG, 4:filter, 5:public key, 6:signature, 7:content.
    for (fd, name) in [
        (gpg_descriptor, "gpg"),
        (filter.read_descriptor(), "seccomp"),
        (payload[0].as_raw_fd(), "public"),
        (payload[1].as_raw_fd(), "signature"),
        (payload[2].as_raw_fd(), "content"),
    ] {
        c.arg(format!(
            "--property=OpenFile=/proc/{}/fd/{fd}:{name}:read-only",
            std::process::id()
        ));
    }
    c.arg(bubblewrap);
    add_guest_arguments(&mut c);
    c.stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    c
}

fn add_guest_arguments(c: &mut Command) {
    c.args([
        "--unshare-all",
        "--unshare-user",
        "--disable-userns",
        "--new-session",
        "--die-with-parent",
        "--clearenv",
        "--setenv",
        "HOME",
        "/nonexistent",
        "--setenv",
        "LANG",
        "C",
        "--setenv",
        "LC_ALL",
        "C",
        "--cap-drop",
        "ALL",
        "--proc",
        "/proc",
        "--dev",
        "/dev",
        "--tmpfs",
        "/tmp",
        "--perms",
        "0700",
        "--dir",
        GUEST_HOME,
        "--dir",
        "/app",
        "--dir",
        "/usr",
        "--dir",
        "/etc",
    ]);
    for path in ["/usr/lib", "/usr/lib64", "/usr/libexec", "/lib", "/lib64"] {
        if Path::new(path).exists() {
            c.args(["--ro-bind", path, path]);
        }
    }
    if Path::new("/etc/ld.so.cache").is_file() {
        c.args(["--ro-bind", "/etc/ld.so.cache", "/etc/ld.so.cache"]);
    }
    for (fd, guest) in [
        ("3", GUEST_GPG),
        ("5", PUBLIC_KEY),
        ("6", SIGNATURE),
        ("7", CONTENT),
    ] {
        c.args(["--ro-bind-fd", fd, guest]);
    }
    c.args(["--chdir", "/app", "--seccomp", "4", "--", GUEST_GPG]);
    c.args(gpg_arguments(GUEST_HOME, PUBLIC_KEY, SIGNATURE, CONTENT));
}

#[cfg(test)]
pub(super) fn fixture_verify(
    raw: &[u8],
    plan: &LocalCommitPlan,
    public: &[u8],
    home: &Path,
    cancellation: &CancellationToken,
) -> Result<crate::sandbox::supervision::LocalCommitCapture, LinuxLocalCommitError> {
    let (signature, content) = split_signed_commit(raw, plan)?;
    let mut paths = Vec::new();
    let mut held = Vec::new();
    for (name, bytes) in [
        ("public", public),
        ("signature", &signature),
        ("content", &content),
    ] {
        let fd = crate::sandbox::sealed_payload(name, bytes).unwrap();
        paths.push(format!(
            "/proc/{}/fd/{}",
            std::process::id(),
            fd.as_raw_fd()
        ));
        held.push(fd);
    }
    let mut command = Command::new("/usr/bin/gpg");
    command
        .env_clear()
        .env("LANG", "C")
        .env("LC_ALL", "C")
        .args(gpg_arguments(
            home.to_str().unwrap(),
            &paths[0],
            &paths[1],
            &paths[2],
        ))
        .stdin(Stdio::null());
    let output = crate::sandbox::supervision::capture_local_commit_process(
        command,
        Some(cancellation),
        Instant::now() + Duration::from_secs(5),
    )
    .map_err(|e| capture_error(e.kind(), LinuxLocalCommitErrorKind::VerificationFailed))?;
    if !output.status.success() {
        return Err(error(LinuxLocalCommitErrorKind::VerificationFailed));
    }
    Ok(output)
}

#[cfg(test)]
#[path = "local_commit_verification_tests.rs"]
mod tests;
