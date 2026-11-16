// SPDX-License-Identifier: MIT OR Apache-2.0
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
fn signer_inspection_capture_observes_control_before_reads_and_after_sealing() {
    let f = Fixture::new();
    let bytes = vec![b'p'; 2 * 64 * 1024 + 1];
    fs::write(f.0.join("pubring.kbx"), &bytes).unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    // Initial check, three bounded reads, pre-seal and post-seal checks.
    for stop in 1..=6 {
        let t = token();
        let mut checks = 0;
        let captured = PublicDatabase::capture_checked(&f.0, || {
            checks += 1;
            if checks == stop {
                cancel(&t);
            }
            check_control(&t, deadline)
        });
        assert_eq!(
            captured.err().unwrap().kind(),
            LinuxLocalCommitErrorKind::ArtifactDenied
        );
        assert_eq!(checks, stop);
    }
    let mut checks = 0;
    let captured = PublicDatabase::capture_checked(&f.0, || {
        checks += 1;
        check_control(&token(), deadline)
    })
    .unwrap();
    assert_eq!(checks, 6);
    assert_eq!(read(&captured.payload), bytes);
    assert_eq!(captured.sha256, hash(&bytes));
    assert!(PublicDatabase::capture_until(&f.0, &token(), Instant::now()).is_err());
    let cancelled = token();
    cancel(&cancelled);
    assert!(PublicDatabase::capture_until(&f.0, &cancelled, deadline).is_err());
}

#[test]
fn signer_inspection_refuses_future_or_inconsistent_primary_and_subkey_dates() {
    let fp = "a".repeat(40);
    let make = |kind: &str, created: &str, expiry: &str| {
        String::from_utf8(listing(&fp, kind, "u", expiry, "s"))
            .unwrap()
            .replace(
                ":0000000000000000:1:",
                &format!(":0000000000000000:{created}:"),
            )
            .into_bytes()
    };
    for (created, expiry) in [
        ("101", ""),
        ("bad", ""),
        ("", ""),
        ("+1", ""),
        ("-1", ""),
        ("1", "+101"),
        ("18446744073709551616", ""),
        ("101", "101"),
        ("102", "101"),
    ] {
        assert!(!signing_identity(&make("pub", created, expiry), &fp, 100));
        let primary = listing(&"b".repeat(40), "pub", "u", "", "cS");
        assert!(!signing_identity(
            &[primary, make("sub", created, expiry)].concat(),
            &fp,
            100
        ));
        let subkey = listing(&"b".repeat(40), "sub", "u", "", "s");
        assert!(!signing_identity(
            &[make("pub", created, expiry), subkey].concat(),
            &"b".repeat(40),
            100
        ));
    }
    // Creation exactly now and non-expiring epoch-zero keys remain current.
    assert!(signing_identity(&make("pub", "100", "101"), &fp, 100));
    assert!(signing_identity(&make("pub", "0", ""), &fp, 100));
    assert!(signing_identity(&make("pub", "0", "0"), &fp, 100));
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
fn signer_inspection_refuses_daemon_markers_even_with_a_valid_legacy_database() {
    for marker in ["common.conf", "public-keys.d"] {
        for kind in ["file", "directory", "symlink", "socket"] {
            let f = Fixture::new();
            fs::write(f.0.join("pubring.kbx"), b"stale public database").unwrap();
            fs::write(f.0.join("pubring.gpg"), b"stale fallback").unwrap();
            let path = f.0.join(marker);
            let mut socket = None;
            match kind {
                "file" => fs::write(&path, b"use-keyboxd\n").unwrap(),
                "directory" => fs::create_dir(&path).unwrap(),
                "symlink" => symlink("absent-target", &path).unwrap(),
                _ => socket = Some(std::os::unix::net::UnixListener::bind(&path).unwrap()),
            }
            assert!(PublicDatabase::capture(&f.0).is_err(), "{marker}/{kind}");
            drop(socket);
        }
        // Recheck both after the file read and after sealing: a migration
        // starting during capture must not return a stale public snapshot.
        for create_at in [3, 4] {
            let f = Fixture::new();
            fs::write(f.0.join("pubring.kbx"), b"legacy").unwrap();
            let mut checks = 0;
            assert!(
                PublicDatabase::capture_checked(&f.0, || {
                    checks += 1;
                    if checks == create_at {
                        fs::write(f.0.join(marker), b"use-keyboxd\n").unwrap();
                    }
                    Ok(())
                })
                .is_err(),
                "{marker}/{create_at}"
            );
            assert_eq!(checks, create_at);
        }
    }
    let f = Fixture::new();
    fs::write(f.0.join("pubring.kbx"), b"legacy").unwrap();
    // Refusal of all common configuration is intentional, even if empty.
    fs::write(f.0.join("common.conf"), b"").unwrap();
    assert!(PublicDatabase::capture(&f.0).is_err());
}

#[test]
fn signer_inspection_distinguishes_designated_revokers_from_signing_fingerprints() {
    let fp = "a".repeat(40);
    let revoker = "b".repeat(40);
    let rvk = format!("rvk:::22::::::{revoker}:80:\n");
    let listed = String::from_utf8(listing(&fp, "pub", "u", "", "s"))
        .unwrap()
        .replace("\nfpr:", &format!("\n{rvk}fpr:"));
    assert!(signing_identity(listed.as_bytes(), &fp, 100));
    assert!(!signing_identity(listed.as_bytes(), &revoker, 100));
    assert!(!signing_identity(
        listed
            .replace(&format!("fpr:::::::::{fp}:\n"), "")
            .as_bytes(),
        &fp,
        100
    ));
    let sub = String::from_utf8(listing(&revoker, "sub", "u", "", "s"))
        .unwrap()
        .replace("\nfpr:", &format!("\n{rvk}fpr:"));
    assert!(!signing_identity(
        &[listed.as_bytes(), sub.as_bytes()].concat(),
        &revoker,
        100
    ));
}

#[test]
fn signer_inspection_preserves_sandbox_cleanup_errors_at_both_worker_boundaries() {
    use crate::sandbox::{LinuxSandboxError, LinuxSandboxErrorKind};
    for kind in [
        LinuxSandboxErrorKind::CleanupUncertain,
        LinuxSandboxErrorKind::InvalidManifest,
    ] {
        let expected = if kind == LinuxSandboxErrorKind::CleanupUncertain {
            LinuxLocalCommitErrorKind::CleanupUncertain
        } else {
            LinuxLocalCommitErrorKind::ArtifactDenied
        };
        assert_eq!(
            map_dispatch(Err(LinuxSandboxError::for_test(kind)))
                .err()
                .unwrap()
                .kind(),
            expected
        );
        let mut captured = result(b"public output");
        captured.outcome = Err(LinuxSandboxError::for_test(kind));
        assert_eq!(
            validate_result(captured, &token(), Instant::now() + Duration::from_secs(5))
                .unwrap_err()
                .kind(),
            expected
        );
    }
    assert_eq!(
        map_dispatch(Ok(result(b"public output")))
            .unwrap()
            .stdout
            .retained,
        b"public output"
    );
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
    let (public, evidence, keygrip) =
        inspect_with(&db, &fingerprint, &token(), deadline, |fd, op| {
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
    assert_eq!(keygrip, None);
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
    for cancel_at in [InspectionOperation::List, InspectionOperation::Export] {
        let cancel_between = token();
        let mut count = 0;
        assert!(
            inspect_with(&db, &fingerprint, &cancel_between, deadline, |_, op| {
                count += 1;
                if op == cancel_at {
                    cancel(&cancel_between);
                }
                Ok(result(if op == InspectionOperation::List {
                    &listing
                } else {
                    b"public"
                }))
            })
            .is_err()
        );
        assert_eq!(
            count,
            if cancel_at == InspectionOperation::List {
                1
            } else {
                2
            }
        );
    }
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
    // The disposable edit below uses the same single-child capture fixture as
    // the signing/pipe controls. Keep their existing shared exclusion contract.
    let _capture = super::super::tests::DIRECT_CAPTURE_TEST
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
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
    let revoker_created = Command::new("/usr/bin/gpg")
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
            "Revoker <revoker@example.invalid>",
            "ed25519",
            "cert",
            "1d",
        ])
        .output()
        .unwrap();
    assert!(revoker_created.status.success());
    let revoker_listed = Command::new("/usr/bin/gpg")
        .arg("--homedir")
        .arg(&f.0)
        .args([
            "--no-options",
            "--batch",
            "--with-colons",
            "--list-keys",
            "revoker@example.invalid",
        ])
        .output()
        .unwrap();
    assert!(revoker_listed.status.success());
    let revoker = String::from_utf8(revoker_listed.stdout)
        .unwrap()
        .lines()
        .find(|l| l.starts_with("fpr:"))
        .unwrap()
        .split(':')
        .nth(9)
        .unwrap()
        .to_owned();
    let script = crate::sandbox::sealed_payload(
        "revoker-script",
        format!("addrevoker\n{revoker}\ny\nsave\n").as_bytes(),
    )
    .unwrap();
    let mut edit = Command::new("/usr/bin/gpg");
    edit.env_clear()
        .env("PATH", "/usr/bin")
        .env("LANG", "C")
        .arg("--homedir")
        .arg(&f.0)
        .args([
            "--no-options",
            "--batch",
            "--no-tty",
            "--expert",
            "--command-fd",
            "0",
            "--edit-key",
            &fingerprint,
        ])
        .stdin(Stdio::from(fs::File::from(script)));
    let edited = crate::sandbox::supervision::capture_local_commit_process(
        edit,
        None,
        Instant::now() + Duration::from_secs(10),
    )
    .unwrap();
    assert!(edited.status.success());
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
            &fingerprint,
        ])
        .output()
        .unwrap();
    assert!(both.status.success());
    assert!(
        both.stdout
            .split(|b| *b == b'\n')
            .any(|l| l.starts_with(b"rvk:"))
    );
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    assert!(signing_identity(&both.stdout, &fingerprint, now));
    assert!(!signing_identity(
        &both.stdout,
        &revoker.to_ascii_lowercase(),
        now
    ));
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
            let (export, _, keygrip) = inspect_with(
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
            assert!(keygrip.as_ref().is_some_and(|g| signing::valid_keygrip(g)));
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

#[test]
fn signer_inspection_keygrip_is_attached_to_exact_fingerprint_and_not_a_revoker() {
    let primary = "a".repeat(40);
    let subkey = "b".repeat(40);
    let first = "C".repeat(40);
    let second = "D".repeat(40);
    let listing = format!(
        "pub:-:255:22:0000000000000000:1::::::s:\nrvk:::::::::{}:\nfpr:::::::::{primary}:\ngrp:::::::::{first}:\nsub:-:255:22:0000000000000000:1::::::s:\nfpr:::::::::{subkey}:\ngrp:::::::::{second}:\n",
        "e".repeat(40)
    );
    assert!(signing_identity(listing.as_bytes(), &primary, 2));
    assert_eq!(
        selected_keygrip(listing.as_bytes(), &primary).unwrap(),
        first
    );
    assert_eq!(
        selected_keygrip(listing.as_bytes(), &subkey).unwrap(),
        second
    );
    assert_eq!(selected_keygrip(listing.as_bytes(), &"e".repeat(40)), None);
    for bad in [
        String::new(),
        "../not-a-file".into(),
        "A\nPKSIGN".into(),
        "0".repeat(40),
        "A".repeat(39),
        "A".repeat(41),
        "G".repeat(40),
    ] {
        assert_eq!(
            selected_keygrip(listing.replace(&first, &bad).as_bytes(), &primary),
            None
        );
    }
    let duplicate = listing.replace(
        &format!("grp:::::::::{first}:\n"),
        &format!("grp:::::::::{first}:\ngrp:::::::::{first}:\n"),
    );
    assert_eq!(selected_keygrip(duplicate.as_bytes(), &primary), None);
    let missing = listing.replace(&format!("grp:::::::::{first}:\n"), "");
    assert_eq!(selected_keygrip(missing.as_bytes(), &primary), None);
}
