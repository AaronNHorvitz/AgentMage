// SPDX-License-Identifier: Apache-2.0
//! Offline public-identity inspection; no private key or agent is projected.

use super::*;
use crate::sandbox::supervision::{FilterPipe, Launch, LaunchOwner, SupervisedResult};
use rustix::fd::OwnedFd;
use rustix::fs::{AtFlags, FileType, Mode, OFlags, fstat, open, openat, statat};
use rustix::io::pread;
use std::os::fd::AsRawFd as _;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

const GUEST_GPG: &str = "/app/gpg";
const PUBLIC_KEY: &str = "/app/public.gpg";
const GUEST_HOME: &str = "/tmp/gnupg";
const LIMIT: usize = crate::sandbox::MAX_OUTPUT_BYTES;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum InspectionOperation {
    List,
    Export,
}

fn denied() -> LinuxLocalCommitError {
    error(LinuxLocalCommitErrorKind::ArtifactDenied)
}

// Capture exactly one public database through a held private directory. No
// directory, configuration, trust database, private key or socket is projected.
struct PublicDatabase {
    payload: OwnedFd,
    sha256: String,
}

impl PublicDatabase {
    fn capture(keyring: &Path) -> Result<Self, LinuxLocalCommitError> {
        if verify_private_keyring(keyring)? != keyring {
            return Err(denied());
        }
        let flags = OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::CLOEXEC | OFlags::NONBLOCK;
        let directory =
            open(keyring, flags | OFlags::DIRECTORY, Mode::empty()).map_err(|_| denied())?;
        let held = fstat(&directory).map_err(|_| denied())?;
        let revalidate = || {
            let current = fs::symlink_metadata(keyring).map_err(|_| denied())?;
            if !current.is_dir()
                || current.dev() != held.st_dev
                || current.ino() != held.st_ino
                || current.uid() != getuid().as_raw()
                || current.mode() & 0o077 != 0
            {
                return Err(denied());
            }
            Ok(())
        };
        revalidate()?;
        let mut selected = None;
        for name in ["pubring.kbx", "pubring.gpg"] {
            match openat(&directory, name, flags, Mode::empty()) {
                Ok(file) => {
                    selected = Some((name, file));
                    break;
                }
                Err(rustix::io::Errno::NOENT) => (),
                Err(_) => return Err(denied()),
            }
        }
        let (name, file) = selected.ok_or_else(denied)?;
        let before = fstat(&file).map_err(|_| denied())?;
        if FileType::from_raw_mode(before.st_mode) != FileType::RegularFile
            || before.st_uid != getuid().as_raw()
            || before.st_mode & 0o022 != 0
            || before.st_size <= 0
            || before.st_size > LIMIT as i64
        {
            return Err(denied());
        }
        let mut bytes = vec![0; before.st_size as usize];
        let mut offset = 0;
        while offset < bytes.len() {
            let count = pread(&file, &mut bytes[offset..], offset as u64).map_err(|_| denied())?;
            if count == 0 {
                return Err(denied());
            }
            offset += count;
        }
        let after = fstat(&file).map_err(|_| denied())?;
        let named = statat(&directory, name, AtFlags::SYMLINK_NOFOLLOW).map_err(|_| denied())?;
        if before.st_dev != named.st_dev
            || before.st_ino != named.st_ino
            || before.st_size != after.st_size
            || before.st_mode != after.st_mode
            || before.st_uid != after.st_uid
            || before.st_mtime != after.st_mtime
            || before.st_mtime_nsec != after.st_mtime_nsec
            || before.st_ctime != after.st_ctime
            || before.st_ctime_nsec != after.st_ctime_nsec
        {
            return Err(denied());
        }
        revalidate()?;
        Ok(Self {
            sha256: hash(&bytes),
            payload: crate::sandbox::sealed_payload("agentmage-inspection-public", &bytes)
                .map_err(|_| denied())?,
        })
    }
}

pub(super) fn inspect(
    executable: &Path,
    executable_sha256: &str,
    keyring: &Path,
    fingerprint: &str,
    cancellation: &CancellationToken,
    deadline: Instant,
) -> Result<(Vec<u8>, String), LinuxLocalCommitError> {
    if cancellation.is_cancelled() || Instant::now() >= deadline || !valid_fingerprint(fingerprint)
    {
        return Err(denied());
    }
    let database = PublicDatabase::capture(keyring)?;
    inspect_with(
        &database,
        fingerprint,
        cancellation,
        deadline,
        |payload, operation| {
            let manifest = Arc::new(verification::VerifierManifest::verify_executable(
                executable,
                executable_sha256,
            )?);
            crate::sandbox::supervision::admission_snapshot()
                .map_err(|e| capture_error(e.kind(), LinuxLocalCommitErrorKind::ArtifactDenied))?;
            let policy = crate::sandbox::compile_seccomp_policy().map_err(|_| denied())?;
            let filter = FilterPipe::new().map_err(|_| denied())?;
            let mut nonce = [0_u8; 12];
            rustix::rand::getrandom(&mut nonce, rustix::rand::GetRandomFlags::empty())
                .map_err(|_| denied())?;
            let unit = format!("agentmage-signer-inspection-{}.service", hex_digest(nonce));
            let command = worker_command(
                manifest.systemd_run.launch_path(),
                manifest.bubblewrap.launch_path(),
                manifest.gpg.held_descriptor().as_raw_fd(),
                &payload,
                (operation, fingerprint),
                &unit,
                &filter,
            );
            manifest.control_path()?;
            crate::sandbox::supervision::run_owned(
                command,
                Launch {
                    owner: LaunchOwner::SignerInspection(manifest),
                    projections: vec![payload],
                    unit,
                    deadline,
                    stdout_limit: LIMIT,
                    stderr_limit: LIMIT,
                    filter_pipe: Some(filter),
                },
                &policy,
                &crate::sandbox::SandboxCancellation::Observed(cancellation),
            )
            .map_err(|e| capture_error(e.kind(), LinuxLocalCommitErrorKind::ArtifactDenied))
        },
    )
}

fn inspect_with(
    database: &PublicDatabase,
    fingerprint: &str,
    cancellation: &CancellationToken,
    deadline: Instant,
    mut dispatch: impl FnMut(
        OwnedFd,
        InspectionOperation,
    ) -> Result<SupervisedResult, LinuxLocalCommitError>,
) -> Result<(Vec<u8>, String), LinuxLocalCommitError> {
    let mut run = |operation| {
        if cancellation.is_cancelled()
            || Instant::now() >= deadline
            || !valid_fingerprint(fingerprint)
        {
            return Err(denied());
        }
        let payload =
            rustix::io::fcntl_dupfd_cloexec(&database.payload, 3).map_err(|_| denied())?;
        let result = dispatch(payload, operation)?;
        validate_result(result, cancellation, deadline)
    };
    let listing = run(InspectionOperation::List)?;
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| denied())?
        .as_secs();
    if !signing_identity(&listing, fingerprint, now) {
        return Err(denied());
    }
    let public = run(InspectionOperation::Export)?;
    if public.is_empty() {
        return Err(denied());
    }
    let evidence = hash(
        &serde_json::to_vec(&(
            "offline-public-identity-v1",
            &database.sha256,
            hash(&listing),
            hash(&public),
            fingerprint,
        ))
        .map_err(|_| denied())?,
    );
    Ok((public, evidence))
}

fn validate_result(
    result: SupervisedResult,
    cancellation: &CancellationToken,
    deadline: Instant,
) -> Result<Vec<u8>, LinuxLocalCommitError> {
    if let Err(failure) = result.outcome {
        return Err(capture_error(
            failure.kind(),
            LinuxLocalCommitErrorKind::ArtifactDenied,
        ));
    }
    if cancellation.is_cancelled()
        || Instant::now() >= deadline
        || !matches!(result.outcome, Ok(OperationOutcome::Succeeded))
        || !result.status.is_some_and(|s| s.success())
        || !result.output_complete
        || [&result.stdout, &result.stderr].into_iter().any(|s| {
            s.total > LIMIT
                || s.total != s.retained.len()
                || s.sha256 != Sha256::digest(&s.retained).as_slice()
        })
    {
        return Err(denied());
    }
    Ok(result.stdout.retained)
}

// Only an exact fingerprint attached to its public key/subkey record, with
// that key's own signing capability, is eligible. This is not a private-key probe.
fn signing_identity(listing: &[u8], fingerprint: &str, now: u64) -> bool {
    let Ok(text) = std::str::from_utf8(listing) else {
        return false;
    };
    if !valid_fingerprint(fingerprint) || !text.ends_with('\n') || text.contains(['\0', '\r']) {
        return false;
    }
    let mut pending = None;
    let mut matches = 0;
    let mut primary_usable = false;
    for line in text.lines() {
        let fields: Vec<_> = line.split(':').collect();
        match fields.first().copied() {
            Some(kind @ ("pub" | "sub")) => {
                if pending.is_some() || fields.len() < 12 {
                    return false;
                }
                let expiry = if fields[6].is_empty() {
                    0
                } else {
                    let Ok(expiry) = fields[6].parse::<u64>() else {
                        return false;
                    };
                    expiry
                };
                let usable = matches!(fields[1], "-" | "o" | "q" | "n" | "m" | "f" | "u")
                    && (expiry == 0 || expiry > now)
                    && !fields[11].contains('D');
                if kind == "pub" {
                    primary_usable = usable;
                }
                pending = Some(usable && primary_usable && fields[11].contains('s'));
            }
            Some("fpr") => {
                let Some(usable) = pending.take() else {
                    return false;
                };
                if fields.len() < 10 || !valid_fingerprint(&fields[9].to_ascii_lowercase()) {
                    return false;
                }
                if fields[9].eq_ignore_ascii_case(fingerprint) {
                    if !usable {
                        return false;
                    }
                    matches += 1;
                }
            }
            Some("tru" | "uid" | "uat" | "grp" | "rvk") => {
                if pending.is_some() {
                    return false;
                }
            }
            _ => return false,
        }
    }
    matches == 1 && pending.is_none()
}

fn gpg_arguments<'a>(
    home: &'a str,
    public: &'a str,
    operation: InspectionOperation,
    fingerprint: &'a str,
) -> Vec<&'a str> {
    let mut args = vec![
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
        "--lock-never",
    ];
    match operation {
        InspectionOperation::List => args.extend([
            "--with-colons",
            "--fixed-list-mode",
            "--with-fingerprint",
            "--with-subkey-fingerprint",
            "--list-keys",
        ]),
        InspectionOperation::Export => args.push("--export"),
    }
    args.push(fingerprint);
    args
}

fn worker_command(
    systemd_run: &Path,
    bubblewrap: &Path,
    gpg_descriptor: std::os::fd::RawFd,
    payload: &OwnedFd,
    selection: (InspectionOperation, &str),
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
    // Exact allocation: 3:GnuPG, 4:filter, 5:sealed public key database.
    for (fd, name) in [
        (gpg_descriptor, "gpg"),
        (filter.read_descriptor(), "seccomp"),
        (payload.as_raw_fd(), "public"),
    ] {
        c.arg(format!(
            "--property=OpenFile=/proc/{}/fd/{fd}:{name}:read-only",
            std::process::id()
        ));
    }
    c.arg(bubblewrap);
    add_guest_arguments(&mut c, selection.0, selection.1);
    c.stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    c
}

fn add_guest_arguments(c: &mut Command, operation: InspectionOperation, fingerprint: &str) {
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
    for (fd, guest) in [("3", GUEST_GPG), ("5", PUBLIC_KEY)] {
        c.args(["--ro-bind-fd", fd, guest]);
    }
    c.args(["--chdir", "/app", "--seccomp", "4", "--", GUEST_GPG]);
    c.args(gpg_arguments(
        GUEST_HOME,
        PUBLIC_KEY,
        operation,
        fingerprint,
    ));
}

#[cfg(test)]
#[path = "local_commit_inspection_tests.rs"]
mod tests;
