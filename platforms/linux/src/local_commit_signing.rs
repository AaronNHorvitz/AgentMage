// SPDX-License-Identifier: Apache-2.0
//! Dormant socket-free private-key primitive. Not an OpenPGP commit signature.
//!
//! The product must not activate this primitive until packet construction,
//! independent verification, native admission and approval composition exist.
#![allow(
    dead_code,
    reason = "private signing stays dormant pending complete commit signature and approval composition"
)]

use super::*;
use crate::sandbox::supervision::{FilterPipe, Launch, LaunchOwner, SupervisedResult};
use rustix::fd::OwnedFd;
use rustix::fs::{AtFlags, FileType, Mode, OFlags, fstat, open, openat, statat};
use rustix::io::pread;
use std::os::fd::AsRawFd as _;
use std::sync::Arc;
use zeroize::Zeroizing;

const MAX_PRIVATE_BYTES: usize = 128 * 1024;
const MAX_SIGNATURE_BYTES: usize = 16 * 1024;
const GUEST_AGENT: &str = "/app/gpg-agent";
const GUEST_HOME: &str = "/tmp/gnupg";

fn denied() -> LinuxLocalCommitError {
    error(LinuxLocalCommitErrorKind::ArtifactDenied)
}

pub(super) fn valid_keygrip(value: &str) -> bool {
    value.len() == 40
        && value.bytes().all(|b| b.is_ascii_hexdigit())
        && value.bytes().any(|b| b != b'0')
}

// No Debug/serialization, private pathname or secret digest. A selected file is
// the entire secret projection, not a mounted directory or live agent connection.
struct PrivateKey {
    payload: OwnedFd,
}
impl PrivateKey {
    fn capture(
        keyring: &Path,
        keygrip: &str,
        mut check: impl FnMut() -> Result<(), LinuxLocalCommitError>,
    ) -> Result<Self, LinuxLocalCommitError> {
        check()?;
        if !valid_keygrip(keygrip) || verify_private_keyring(keyring)? != keyring {
            return Err(denied());
        }
        let flags = OFlags::RDONLY | OFlags::CLOEXEC | OFlags::NOFOLLOW | OFlags::NONBLOCK;
        let root = open(keyring, flags | OFlags::DIRECTORY, Mode::empty()).map_err(|_| denied())?;
        let directory = openat(
            &root,
            "private-keys-v1.d",
            flags | OFlags::DIRECTORY,
            Mode::empty(),
        )
        .map_err(|_| denied())?;
        let root_stat = fstat(&root).map_err(|_| denied())?;
        let dir_stat = fstat(&directory).map_err(|_| denied())?;
        let name = format!("{}.key", keygrip.to_ascii_uppercase());
        let file = openat(&directory, name.as_str(), flags, Mode::empty()).map_err(|_| denied())?;
        let before = fstat(&file).map_err(|_| denied())?;
        if [root_stat, dir_stat].iter().any(|s| {
            FileType::from_raw_mode(s.st_mode) != FileType::Directory
                || s.st_uid != getuid().as_raw()
                || s.st_mode & 0o077 != 0
        }) || FileType::from_raw_mode(before.st_mode) != FileType::RegularFile
            || before.st_uid != getuid().as_raw()
            || before.st_mode & 0o7177 != 0
            || before.st_nlink != 1
            || before.st_size <= 0
            || before.st_size > MAX_PRIVATE_BYTES as i64
        {
            return Err(denied());
        }
        let revalidate = || {
            let current = fs::symlink_metadata(keyring).map_err(|_| denied())?;
            let named_dir = statat(&root, "private-keys-v1.d", AtFlags::SYMLINK_NOFOLLOW)
                .map_err(|_| denied())?;
            let named_file = statat(&directory, name.as_str(), AtFlags::SYMLINK_NOFOLLOW)
                .map_err(|_| denied())?;
            if current.dev() != root_stat.st_dev
                || current.ino() != root_stat.st_ino
                || current.mode() != root_stat.st_mode
                || current.uid() != root_stat.st_uid
                || !same_file(&dir_stat, &named_dir)
                || !same_file(&before, &named_file)
                || !same_file(&before, &fstat(&file).map_err(|_| denied())?)
            {
                return Err(denied());
            }
            Ok(())
        };
        revalidate()?;
        let mut bytes = Zeroizing::new(vec![0_u8; before.st_size as usize]);
        let mut offset = 0;
        while offset < bytes.len() {
            check()?;
            let end = bytes.len().min(offset + 16 * 1024);
            let count =
                pread(&file, &mut bytes[offset..end], offset as u64).map_err(|_| denied())?;
            if count == 0 {
                return Err(denied());
            }
            offset += count;
        }
        check()?;
        revalidate()?;
        let payload = crate::sandbox::sealed_payload("agentmage-selected-private-key", &bytes)
            .map_err(|_| denied())?;
        check()?;
        revalidate()?;
        Ok(Self { payload })
    }
}

fn same_file(before: &rustix::fs::Stat, after: &rustix::fs::Stat) -> bool {
    before.st_dev == after.st_dev
        && before.st_ino == after.st_ino
        && before.st_mode == after.st_mode
        && before.st_uid == after.st_uid
        && before.st_nlink == after.st_nlink
        && before.st_size == after.st_size
        && before.st_mtime == after.st_mtime
        && before.st_mtime_nsec == after.st_mtime_nsec
        && before.st_ctime == after.st_ctime
        && before.st_ctime_nsec == after.st_ctime_nsec
}

pub(crate) struct SigningManifest {
    systemd_run: LinuxGitArtifact,
    systemctl: LinuxGitArtifact,
    bubblewrap: LinuxGitArtifact,
    agent: LinuxGitArtifact,
}
impl SigningManifest {
    fn verify() -> Result<Self, LinuxLocalCommitError> {
        let artifact = |path| LinuxGitArtifact::verify(Path::new(path)).map_err(|_| denied());
        Ok(Self {
            systemd_run: artifact("/usr/bin/systemd-run")?,
            systemctl: artifact("/usr/bin/systemctl")?,
            bubblewrap: artifact("/usr/bin/bwrap")?,
            agent: artifact("/usr/bin/gpg-agent")?,
        })
    }
    pub(crate) fn control_path(&self) -> Result<&Path, LinuxLocalCommitError> {
        for a in [
            &self.systemd_run,
            &self.systemctl,
            &self.bubblewrap,
            &self.agent,
        ] {
            a.revalidate().map_err(|_| denied())?;
        }
        Ok(self.systemctl.launch_path())
    }
}

// Internal primitive only. The caller supplies the complete OpenPGP digest;
// the response conveys no verification, approval, object or publication authority.
pub(super) fn sign_hash(
    signer: &LinuxOpenPgpSigner,
    digest: &[u8; 32],
    cancellation: &CancellationToken,
    deadline: Instant,
) -> Result<Vec<u8>, LinuxLocalCommitError> {
    let deadline = deadline.min(Instant::now() + Duration::from_secs(30));
    inspection::check_control(cancellation, deadline)?;
    if signer.pinned.verify().is_err()
        || hash(&signer.public_key) != signer.pinned.public_key_sha256
    {
        return Err(denied());
    }
    let keygrip = signer
        .keygrip
        .as_deref()
        .filter(|g| valid_keygrip(g))
        .ok_or_else(denied)?;
    let key = PrivateKey::capture(&signer.keyring, keygrip, || {
        inspection::check_control(cancellation, deadline)
    })?;
    sign_with(key, keygrip, digest, cancellation, deadline, |payload| {
        let manifest = Arc::new(SigningManifest::verify()?);
        crate::sandbox::supervision::admission_snapshot()
            .map_err(|e| capture_error(e.kind(), LinuxLocalCommitErrorKind::ExecutionFailed))?;
        let policy = crate::sandbox::compile_seccomp_policy().map_err(|_| denied())?;
        let filter = FilterPipe::new().map_err(|_| denied())?;
        let mut nonce = [0_u8; 12];
        rustix::rand::getrandom(&mut nonce, rustix::rand::GetRandomFlags::empty())
            .map_err(|_| denied())?;
        let unit = format!("agentmage-private-signing-{}.service", hex_digest(nonce));
        let command = worker_command(
            manifest.systemd_run.launch_path(),
            manifest.bubblewrap.launch_path(),
            manifest.agent.held_descriptor().as_raw_fd(),
            &payload,
            keygrip,
            &unit,
            &filter,
        )?;
        manifest.control_path()?;
        crate::sandbox::supervision::run_owned(
            command,
            Launch {
                owner: LaunchOwner::PrivateSigning(manifest),
                projections: payload.into(),
                unit,
                deadline,
                stdout_limit: MAX_SIGNATURE_BYTES,
                stderr_limit: MAX_SIGNATURE_BYTES,
                filter_pipe: Some(filter),
            },
            &policy,
            &crate::sandbox::SandboxCancellation::Observed(cancellation),
        )
        .map_err(|e| capture_error(e.kind(), LinuxLocalCommitErrorKind::ExecutionFailed))
    })
}

fn sign_with(
    key: PrivateKey,
    keygrip: &str,
    digest: &[u8; 32],
    cancellation: &CancellationToken,
    deadline: Instant,
    dispatch: impl FnOnce([OwnedFd; 2]) -> Result<SupervisedResult, LinuxLocalCommitError>,
) -> Result<Vec<u8>, LinuxLocalCommitError> {
    inspection::check_control(cancellation, deadline)?;
    if !valid_keygrip(keygrip) {
        return Err(denied());
    }
    let commands = format!(
        "RESET\nSIGKEY {}\nSETHASH 8 {}\nPKSIGN\nBYE\n",
        keygrip.to_ascii_uppercase(),
        hex_digest(digest)
    );
    let input = crate::sandbox::sealed_payload("agentmage-signing-commands", commands.as_bytes())
        .map_err(|_| denied())?;
    let output = dispatch([key.payload, input])?;
    validate_result(output, cancellation, deadline)
}

fn validate_result(
    result: SupervisedResult,
    cancellation: &CancellationToken,
    deadline: Instant,
) -> Result<Vec<u8>, LinuxLocalCommitError> {
    if let Err(e) = result.outcome {
        return Err(capture_error(
            e.kind(),
            LinuxLocalCommitErrorKind::ExecutionFailed,
        ));
    }
    inspection::check_control(cancellation, deadline)?;
    if !matches!(result.outcome, Ok(OperationOutcome::Succeeded))
        || !result.output_complete
        || !result.status.is_some_and(|s| s.success())
        || [&result.stdout, &result.stderr].into_iter().any(|s| {
            s.total > MAX_SIGNATURE_BYTES
                || s.total != s.retained.len()
                || s.sha256 != Sha256::digest(&s.retained).as_slice()
        })
    {
        return Err(error(LinuxLocalCommitErrorKind::ExecutionFailed));
    }
    parse_response(&result.stdout.retained)
        .ok_or_else(|| error(LinuxLocalCommitErrorKind::ExecutionFailed))
}

// Bounded, phase-ordered protocol. Errors and interactive inquiries never yield
// a result; diagnostics cannot replace stdout. Signature bytes stay untrusted.
fn parse_response(bytes: &[u8]) -> Option<Vec<u8>> {
    if bytes.len() > MAX_SIGNATURE_BYTES || !bytes.ends_with(b"\n") {
        return None;
    }
    let mut phase = 0;
    let mut data = Vec::new();
    for line in bytes[..bytes.len() - 1].split(|b| *b == b'\n') {
        if line == b"OK" || line.starts_with(b"OK ") {
            if phase == 4 && data.is_empty() {
                return None;
            }
            phase += 1;
            if phase > 6 {
                return None;
            }
        } else if let Some(encoded) = line.strip_prefix(b"D ") {
            if phase != 4 {
                return None;
            }
            let mut i = 0;
            while i < encoded.len() {
                if encoded[i] == b'%' {
                    let hex = std::str::from_utf8(encoded.get(i + 1..i + 3)?).ok()?;
                    data.push(u8::from_str_radix(hex, 16).ok()?);
                    i += 3;
                } else {
                    if encoded[i] == b'\r' {
                        return None;
                    }
                    data.push(encoded[i]);
                    i += 1;
                }
            }
        } else {
            return None;
        }
    }
    (phase == 6 && data.starts_with(b"(7:sig-val(") && data.ends_with(b"))")).then_some(data)
}

fn agent_arguments(home: &str) -> Vec<&str> {
    vec![
        "--no-options",
        "--homedir",
        home,
        "--server",
        "--batch",
        "--disable-scdaemon",
        "--pinentry-program",
        "/nonexistent",
        "--no-allow-external-cache",
        "--no-allow-mark-trusted",
        "--default-cache-ttl",
        "0",
        "--max-cache-ttl",
        "0",
    ]
}

fn worker_command(
    systemd_run: &Path,
    bubblewrap: &Path,
    agent: std::os::fd::RawFd,
    payload: &[OwnedFd; 2],
    keygrip: &str,
    unit: &str,
    filter: &FilterPipe,
) -> Result<Command, LinuxLocalCommitError> {
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
    for (fd, name) in [
        (agent, "agent"),
        (filter.read_descriptor(), "seccomp"),
        (payload[0].as_raw_fd(), "selected-key"),
    ] {
        c.arg(format!(
            "--property=OpenFile=/proc/{}/fd/{fd}:{name}:read-only",
            std::process::id()
        ));
    }
    c.arg(bubblewrap);
    add_guest_arguments(&mut c, keygrip);
    let input = rustix::io::fcntl_dupfd_cloexec(&payload[1], 3).map_err(|_| denied())?;
    c.stdin(Stdio::from(fs::File::from(input)))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    Ok(c)
}

fn add_guest_arguments(c: &mut Command, keygrip: &str) {
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
        "--perms",
        "0700",
        "--dir",
        "/tmp/gnupg/private-keys-v1.d",
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
    let private = format!(
        "{GUEST_HOME}/private-keys-v1.d/{}.key",
        keygrip.to_ascii_uppercase()
    );
    for (fd, guest) in [("3", GUEST_AGENT), ("5", private.as_str())] {
        c.args(["--ro-bind-fd", fd, guest]);
    }
    // Fixed commands reach stdin through systemd-run --pipe, independently of
    // the withheld filter. No shell, socket or live keyring is involved.
    c.args(["--chdir", GUEST_HOME, "--seccomp", "4", "--", GUEST_AGENT]);
    c.args(agent_arguments(GUEST_HOME));
}

#[cfg(test)]
pub(super) fn fixture_sign(
    source: &Path,
    home: &Path,
    keygrip: &str,
    digest: &[u8; 32],
    cancellation: &CancellationToken,
) -> Result<Vec<u8>, LinuxLocalCommitError> {
    // Disposable fixture only: emulate the descriptor mount in a private home.
    // Production never writes a private-key file and always uses run_owned.
    use std::os::unix::fs::{DirBuilderExt as _, PermissionsExt as _};
    let key = PrivateKey::capture(source, keygrip, || Ok(()))?;
    fs::DirBuilder::new()
        .mode(0o700)
        .create(home.join("private-keys-v1.d"))
        .map_err(|_| denied())?;
    let mut bytes = Zeroizing::new(vec![
        0;
        fstat(&key.payload).map_err(|_| denied())?.st_size
            as usize
    ]);
    if pread(&key.payload, &mut bytes[..], 0).map_err(|_| denied())? != bytes.len() {
        return Err(denied());
    }
    let path = home.join(format!(
        "private-keys-v1.d/{}.key",
        keygrip.to_ascii_uppercase()
    ));
    fs::write(&path, &*bytes).map_err(|_| denied())?;
    fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).map_err(|_| denied())?;
    let deadline = Instant::now() + Duration::from_secs(5);
    sign_with(key, keygrip, digest, cancellation, deadline, |payload| {
        let input = rustix::io::fcntl_dupfd_cloexec(&payload[1], 3).map_err(|_| denied())?;
        let mut command = Command::new("/usr/bin/gpg-agent");
        command
            .env_clear()
            .env("LANG", "C")
            .env("LC_ALL", "C")
            .args(agent_arguments(home.to_str().ok_or_else(denied)?))
            .stdin(Stdio::from(fs::File::from(input)));
        let output = crate::sandbox::supervision::capture_local_commit_process(
            command,
            Some(cancellation),
            deadline,
        )
        .map_err(|e| capture_error(e.kind(), LinuxLocalCommitErrorKind::ExecutionFailed))?;
        let stream = |bytes: Vec<u8>| crate::sandbox::supervision::CapturedStream {
            total: bytes.len(),
            sha256: Sha256::digest(&bytes).into(),
            retained: bytes,
        };
        Ok(SupervisedResult {
            outcome: Ok(OperationOutcome::Succeeded),
            status: Some(output.status),
            stdout: stream(output.stdout),
            stderr: stream(output.stderr),
            resources: None,
            output_complete: true,
        })
    })
}

#[cfg(test)]
#[path = "local_commit_signing_tests.rs"]
mod tests;
