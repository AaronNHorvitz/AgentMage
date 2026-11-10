// SPDX-License-Identifier: Apache-2.0
//! Component evidence; no native artifact, sandbox or private-key admission.

use super::*;
use crate::sandbox::supervision::CapturedStream;
use agentmage_kernel_contracts::*;
use std::os::unix::fs::{DirBuilderExt as _, PermissionsExt as _, symlink};
use std::os::unix::process::ExitStatusExt as _;
use std::sync::atomic::{AtomicU64, Ordering};

static NONCE: AtomicU64 = AtomicU64::new(1);
struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "si-{}-{}",
            std::process::id(),
            NONCE.fetch_add(1, Ordering::Relaxed)
        ));
        fs::DirBuilder::new().mode(0o700).create(&path).unwrap();
        Self(path)
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = Command::new("/usr/bin/gpgconf")
            .env_clear()
            .env("PATH", "/usr/bin")
            .arg("--homedir")
            .arg(&self.0)
            .args(["--kill", "gpg-agent"])
            .output();
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn token() -> CancellationToken {
    CancellationToken::root(
        BoundaryKind::Kernel,
        TaskId::from_raw("inspect"),
        CorrelationId::from_raw("inspect"),
    )
}
fn cancel(t: &CancellationToken) {
    t.cancel(CancellationSignal {
        schema_version: CONTRACT_SCHEMA_VERSION,
        cancellation_id: CancellationId::from_raw("inspect-cancel"),
        task_id: TaskId::from_raw("inspect"),
        correlation_id: CorrelationId::from_raw("inspect"),
        reason: CancellationReason::UserRequested,
        requested_by: BoundaryKind::Kernel,
    })
    .unwrap();
}
fn result(bytes: &[u8]) -> SupervisedResult {
    let stream = |b: &[u8]| CapturedStream {
        total: b.len(),
        retained: b.to_vec(),
        sha256: Sha256::digest(b).into(),
    };
    SupervisedResult {
        outcome: Ok(OperationOutcome::Succeeded),
        status: Some(std::process::ExitStatus::from_raw(0)),
        stdout: stream(bytes),
        stderr: stream(b"diagnostic only\n"),
        resources: None,
        output_complete: true,
    }
}
fn listing(
    fingerprint: &str,
    kind: &str,
    validity: &str,
    expiry: &str,
    capability: &str,
) -> Vec<u8> {
    format!("{kind}:{validity}:255:22:0000000000000000:1:{expiry}:::::{capability}:\nfpr:::::::::{fingerprint}:\n").into_bytes()
}
fn read(fd: &OwnedFd) -> Vec<u8> {
    let mut bytes = vec![0; fstat(fd).unwrap().st_size as usize];
    assert_eq!(pread(fd, &mut bytes, 0).unwrap(), bytes.len());
    bytes
}

#[test]
fn signer_inspection_captures_only_a_bounded_sealed_public_database() {
    for name in ["pubring.kbx", "pubring.gpg"] {
        let f = Fixture::new();
        fs::write(f.0.join(name), b"public snapshot").unwrap();
        fs::write(f.0.join("gpg.conf"), b"untrusted options").unwrap();
        fs::create_dir(f.0.join("private-keys-v1.d")).unwrap();
        fs::write(f.0.join("private-keys-v1.d/secret.key"), b"never read").unwrap();
        let _socket = std::os::unix::net::UnixListener::bind(f.0.join("S.gpg-agent")).unwrap();
        let db = PublicDatabase::capture(&f.0).unwrap();
        fs::write(f.0.join(name), b"changed after capture").unwrap();
        assert_eq!(read(&db.payload), b"public snapshot");
        assert_eq!(db.sha256, hash(b"public snapshot"));
        use rustix::fs::{SealFlags, fcntl_get_seals};
        assert_eq!(
            fcntl_get_seals(&db.payload).unwrap(),
            SealFlags::SEAL | SealFlags::SHRINK | SealFlags::GROW | SealFlags::WRITE
        );
        assert!(rustix::io::pwrite(&db.payload, b"change", 0).is_err());
    }
}

#[test]
fn signer_inspection_refuses_missing_empty_special_symlink_oversize_and_writable_databases() {
    for case in [
        "missing",
        "empty",
        "directory",
        "symlink",
        "socket",
        "oversize",
        "writable",
        "home-mode",
    ] {
        let f = Fixture::new();
        let path = f.0.join("pubring.kbx");
        let mut socket = None;
        match case {
            "missing" => (),
            "empty" => fs::write(&path, b"").unwrap(),
            "directory" => fs::create_dir(&path).unwrap(),
            "symlink" => {
                fs::write(f.0.join("pubring.gpg"), b"valid fallback").unwrap();
                symlink(f.0.join("pubring.gpg"), &path).unwrap();
            }
            "socket" => {
                socket = Some(std::os::unix::net::UnixListener::bind(&path).unwrap());
            }
            "oversize" => fs::File::create(&path)
                .unwrap()
                .set_len(LIMIT as u64 + 1)
                .unwrap(),
            _ => {
                fs::write(&path, b"public").unwrap();
                let target = if case == "writable" { &path } else { &f.0 };
                fs::set_permissions(target, fs::Permissions::from_mode(0o777)).unwrap();
            }
        }
        assert!(PublicDatabase::capture(&f.0).is_err(), "{case}");
        drop(socket);
    }
}

#[test]
fn signer_inspection_requires_exact_attached_current_signing_fingerprint() {
    let fingerprint = "a".repeat(40);
    let valid = listing(&fingerprint, "pub", "-", "", "scSC");
    let v5 = "a".repeat(64);
    assert!(signing_identity(
        &listing(&v5, "pub", "-", "", "s"),
        &v5,
        100
    ));
    assert!(signing_identity(&valid, &fingerprint, 100));
    let primary = listing(&"b".repeat(40), "pub", "u", "", "cSC");
    let subkey = listing(&fingerprint, "sub", "u", "101", "s");
    assert!(signing_identity(
        &[primary.clone(), subkey.clone()].concat(),
        &fingerprint,
        100
    ));
    for bad in [
        valid[..valid.len() - 1].to_vec(),
        b"diagnostic fingerprint\n".to_vec(),
        format!("uid:::::::::{fingerprint}:\n").into_bytes(),
        listing(&fingerprint, "pub", "r", "", "s"),
        listing(&fingerprint, "pub", "e", "", "s"),
        listing(&fingerprint, "pub", "u", "100", "s"),
        listing(&fingerprint, "pub", "u", "bad", "s"),
        listing(&fingerprint, "pub", "u", "", "cS"),
        listing(&fingerprint, "pub", "u", "", "sD"),
        [valid.clone(), valid.clone()].concat(),
        subkey.clone(),
        [
            b"pub:-:255:22:0000000000000000:1::::::s:\n".as_slice(),
            &subkey,
        ]
        .concat(),
        [listing(&"b".repeat(40), "pub", "r", "", "cSC"), subkey].concat(),
        valid.iter().copied().chain([0]).collect(),
        valid
            .iter()
            .copied()
            .map(|b| if b == b'\n' { b'\r' } else { b })
            .collect(),
    ] {
        assert!(!signing_identity(&bad, &fingerprint, 100), "{bad:?}");
    }
    assert!(!signing_identity(&valid, &"b".repeat(40), 100));
    assert!(!signing_identity(&valid, &fingerprint[..20], 100));
}

#[test]
fn signer_inspection_orders_both_workers_over_the_same_snapshot_and_stops_on_failure() {
    let f = Fixture::new();
    fs::write(f.0.join("pubring.kbx"), b"snapshot").unwrap();
    let db = PublicDatabase::capture(&f.0).unwrap();
    let fingerprint = "a".repeat(40);
    let listing = listing(&fingerprint, "pub", "-", "", "s");
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut operations = Vec::new();
    let (public, evidence) = inspect_with(&db, &fingerprint, &token(), deadline, |fd, op| {
        assert_eq!(read(&fd), b"snapshot");
        assert!(rustix::io::pwrite(&fd, b"x", 0).is_err());
        operations.push(op);
        Ok(result(if op == InspectionOperation::List {
            &listing
        } else {
            b"export"
        }))
    })
    .unwrap();
    assert_eq!(
        operations,
        [InspectionOperation::List, InspectionOperation::Export]
    );
    assert_eq!(public, b"export");
    assert_eq!(
        evidence,
        hash(
            &serde_json::to_vec(&(
                "offline-public-identity-v1",
                &db.sha256,
                hash(&listing),
                hash(b"export"),
                &fingerprint
            ))
            .unwrap()
        )
    );
    for failure in [0, 1, 2] {
        let mut count = 0;
        let result = inspect_with(&db, &fingerprint, &token(), deadline, |_, op| {
            count += 1;
            if failure == 0 {
                Ok(result(b"malformed listing\n"))
            } else if op == InspectionOperation::List {
                Ok(result(&listing))
            } else if failure == 1 {
                Ok(result(b""))
            } else {
                Err(error(LinuxLocalCommitErrorKind::CleanupUncertain))
            }
        });
        assert_eq!(
            result.err().unwrap().kind(),
            if failure == 2 {
                LinuxLocalCommitErrorKind::CleanupUncertain
            } else {
                LinuxLocalCommitErrorKind::ArtifactDenied
            }
        );
        assert_eq!(count, if failure == 0 { 1 } else { 2 });
    }
    let cancelled = token();
    cancel(&cancelled);
    for (fingerprint, token, deadline) in [
        ("bad", token(), deadline),
        (&fingerprint, cancelled, deadline),
        (
            &fingerprint,
            token(),
            Instant::now() - Duration::from_millis(1),
        ),
    ] {
        assert!(
            inspect_with(&db, fingerprint, &token, deadline, |_, _| panic!(
                "preflight must not dispatch"
            ))
            .is_err()
        );
    }
    let cancel_between = token();
    let mut count = 0;
    assert!(
        inspect_with(&db, &fingerprint, &cancel_between, deadline, |_, _| {
            count += 1;
            cancel(&cancel_between);
            Ok(result(&listing))
        })
        .is_err()
    );
    assert_eq!(count, 1);
}

#[test]
fn signer_inspection_requires_complete_consistent_success_after_cleanup() {
    let deadline = Instant::now() + Duration::from_secs(5);
    for mutation in 0..12 {
        let mut r = result(b"public output");
        match mutation {
            0 => r.outcome = Ok(OperationOutcome::Failed),
            1 => r.status = None,
            2 => r.status = Some(std::process::ExitStatus::from_raw(7 << 8)),
            3 => r.output_complete = false,
            4 => r.stdout.total += 1,
            5 => r.stdout.sha256[0] ^= 1,
            6 => r.stderr.total += 1,
            7 => r.stderr.sha256[0] ^= 1,
            8 => r.stdout.total = LIMIT + 1,
            9 => r.stderr.total = LIMIT + 1,
            10 => r.outcome = Ok(OperationOutcome::Cancelled),
            _ => r.outcome = Ok(OperationOutcome::TimedOut),
        }
        assert_eq!(
            validate_result(r, &token(), deadline).unwrap_err().kind(),
            LinuxLocalCommitErrorKind::ArtifactDenied
        );
    }
    assert_eq!(
        validate_result(result(b"public output"), &token(), deadline).unwrap(),
        b"public output"
    );
}

#[test]
fn signer_inspection_command_projects_no_live_keyring_private_keys_configuration_or_sockets() {
    let payload = crate::sandbox::sealed_payload("public", b"public").unwrap();
    let gpg = crate::sandbox::sealed_payload("synthetic-gpg", b"not executable").unwrap();
    let fingerprint = "a".repeat(40);
    for op in [InspectionOperation::List, InspectionOperation::Export] {
        let filter = FilterPipe::new().unwrap();
        let c = worker_command(
            Path::new("/synthetic/systemd-run"),
            Path::new("/synthetic/bwrap"),
            gpg.as_raw_fd(),
            &payload,
            (op, &fingerprint),
            "agentmage-signer-inspection-0123456789abcdef01234567.service",
            &filter,
        );
        let args: Vec<_> = c.get_args().map(|a| a.to_str().unwrap()).collect();
        assert_eq!(
            args.iter()
                .filter(|a| a.starts_with("--property=OpenFile="))
                .count(),
            3
        );
        assert_eq!(args.iter().filter(|a| **a == "--ro-bind-fd").count(), 2);
        for mandatory in [
            "--property=KillMode=control-group",
            "--property=SendSIGKILL=yes",
            "--property=Restart=no",
            "--property=MemoryMax=268435456",
            "--property=TasksMax=16",
            "--property=CPUQuota=100%",
            "--property=RuntimeMaxSec=31s",
            "--unshare-all",
            "--disable-userns",
            "--clearenv",
            "--seccomp",
            "--no-options",
            "--no-autostart",
            "--no-auto-key-retrieve",
            "--lock-never",
        ] {
            assert!(args.contains(&mandatory), "{mandatory}");
        }
        let guest = args.iter().position(|a| *a == "/synthetic/bwrap").unwrap();
        for forbidden in [
            "--bind",
            "--bind-fd",
            "--share-net",
            "GNUPGHOME",
            "--list-secret-keys",
            "--status-fd=1",
            "S.gpg-agent",
            "private-keys-v1.d",
            "gpg.conf",
        ] {
            assert!(
                !args[guest..].iter().any(|a| a.contains(forbidden)),
                "{forbidden}"
            );
        }
        assert!(args.ends_with(&[
            if op == InspectionOperation::List {
                "--list-keys"
            } else {
                "--export"
            },
            &fingerprint
        ]));
        let binds: Vec<_> = args[guest..]
            .windows(3)
            .filter(|w| w[0] == "--ro-bind")
            .collect();
        assert!(binds.iter().all(|w| matches!(
            w[1],
            "/usr/lib" | "/usr/lib64" | "/usr/libexec" | "/lib" | "/lib64" | "/etc/ld.so.cache"
        ) && w[1] == w[2]));
        assert!(
            c.get_envs()
                .all(|(k, _)| k == "XDG_RUNTIME_DIR" || k == "DBUS_SESSION_BUS_ADDRESS")
        );
    }
}

#[test]
fn signer_inspection_installed_gpg_reads_sealed_public_identity_without_an_agent() {
    // Real GnuPG operation with fixed worker argv, outside native sandbox/artifact
    // admission. Setup alone generates disposable secrets; inspection never gets them.
    let f = Fixture::new();
    let mut setup = Command::new("/usr/bin/gpg");
    setup
        .env_clear()
        .env("PATH", "/usr/bin")
        .env("LANG", "C")
        .arg("--homedir")
        .arg(&f.0)
        .args([
            "--no-options",
            "--batch",
            "--no-tty",
            "--pinentry-mode",
            "loopback",
            "--passphrase",
            "",
            "--quick-generate-key",
            "Fixture <fixture@example.invalid>",
            "ed25519",
            "sign",
            "1d",
        ]);
    assert!(setup.output().unwrap().status.success());
    let initial = Command::new("/usr/bin/gpg")
        .arg("--homedir")
        .arg(&f.0)
        .args(["--no-options", "--batch", "--with-colons", "--list-keys"])
        .output()
        .unwrap();
    assert!(initial.status.success());
    let fingerprint = String::from_utf8(initial.stdout)
        .unwrap()
        .lines()
        .find(|l| l.starts_with("fpr:"))
        .unwrap()
        .split(':')
        .nth(9)
        .unwrap()
        .to_ascii_lowercase();
    let added = Command::new("/usr/bin/gpg")
        .env_clear()
        .env("PATH", "/usr/bin")
        .env("LANG", "C")
        .arg("--homedir")
        .arg(&f.0)
        .args([
            "--no-options",
            "--batch",
            "--no-tty",
            "--pinentry-mode",
            "loopback",
            "--passphrase",
            "",
            "--quick-add-key",
            &fingerprint,
            "ed25519",
            "sign",
            "1d",
        ])
        .output()
        .unwrap();
    assert!(added.status.success());
    let both = Command::new("/usr/bin/gpg")
        .env_clear()
        .env("LANG", "C")
        .arg("--homedir")
        .arg(&f.0)
        .args([
            "--no-options",
            "--batch",
            "--with-colons",
            "--with-subkey-fingerprint",
            "--list-keys",
        ])
        .output()
        .unwrap();
    assert!(both.status.success());
    let fingerprints: Vec<_> = String::from_utf8(both.stdout)
        .unwrap()
        .lines()
        .filter(|l| l.starts_with("fpr:"))
        .map(|l| l.split(':').nth(9).unwrap().to_ascii_lowercase())
        .collect();
    assert_eq!(fingerprints.len(), 2);
    assert_eq!(fingerprints[0], fingerprint);
    assert!(
        Command::new("/usr/bin/gpgconf")
            .env_clear()
            .env("PATH", "/usr/bin")
            .arg("--homedir")
            .arg(&f.0)
            .args(["--kill", "gpg-agent"])
            .output()
            .unwrap()
            .status
            .success()
    );
    let secret_probe = Command::new("/usr/bin/gpg")
        .env_clear()
        .env("LANG", "C")
        .arg("--homedir")
        .arg(&f.0)
        .args([
            "--no-options",
            "--batch",
            "--no-autostart",
            "--with-colons",
            "--list-secret-keys",
        ])
        .output()
        .unwrap();
    assert!(secret_probe.status.success());
    assert!(
        !secret_probe
            .stdout
            .split(|b| *b == b'\n')
            .any(|l| l.starts_with(b"sec:") || l.starts_with(b"ssb:"))
    );
    let public_ring = Fixture::new();
    let mut expected = None;
    for database in 0..2 {
        let db =
            PublicDatabase::capture(if database == 0 { &f.0 } else { &public_ring.0 }).unwrap();
        for fingerprint in &fingerprints {
            let home =
                f.0.join(format!("public-worker-home-{database}-{fingerprint}"));
            fs::DirBuilder::new().mode(0o700).create(&home).unwrap();
            let (export, _) = inspect_with(
                &db,
                fingerprint,
                &token(),
                Instant::now() + Duration::from_secs(5),
                |fd, op| {
                    let source = format!("/proc/{}/fd/{}", std::process::id(), fd.as_raw_fd());
                    let output = Command::new("/usr/bin/gpg")
                        .env_clear()
                        .env("LANG", "C")
                        .env("LC_ALL", "C")
                        .args(gpg_arguments(
                            home.to_str().unwrap(),
                            &source,
                            op,
                            fingerprint,
                        ))
                        .output()
                        .unwrap();
                    assert!(output.status.success(), "fixed inspection options failed");
                    let mut captured = result(&output.stdout);
                    captured.stderr = CapturedStream {
                        total: output.stderr.len(),
                        sha256: Sha256::digest(&output.stderr).into(),
                        retained: output.stderr,
                    };
                    Ok(captured)
                },
            )
            .unwrap();
            assert!(!export.is_empty());
            if let Some(expected) = &expected {
                assert_eq!(&export, expected);
            } else {
                expected = Some(export.clone());
            }
            for private in [
                "S.gpg-agent",
                "S.gpg-agent.extra",
                "private-keys-v1.d",
                "gpg.conf",
            ] {
                assert!(
                    !home.join(private).exists(),
                    "inspection must not create {private}"
                );
            }
        }
        if database == 0 {
            fs::write(
                public_ring.0.join("pubring.gpg"),
                expected.as_ref().unwrap(),
            )
            .unwrap();
        }
    }
    assert!(!f.0.join("S.gpg-agent").exists());
    assert!(!public_ring.0.join("S.gpg-agent").exists());
}
