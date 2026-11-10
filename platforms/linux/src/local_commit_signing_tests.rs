// SPDX-License-Identifier: Apache-2.0
//! Component tests; direct disposable cryptography is not native admission.

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
            "sk-{}-{}",
            std::process::id(),
            NONCE.fetch_add(1, Ordering::Relaxed)
        ));
        fs::DirBuilder::new().mode(0o700).create(&path).unwrap();
        Self(path)
    }
    fn key(&self, bytes: &[u8]) -> (String, PathBuf) {
        let grip = "A".repeat(40);
        fs::DirBuilder::new()
            .mode(0o700)
            .create(self.0.join("private-keys-v1.d"))
            .unwrap();
        let path = self.0.join(format!("private-keys-v1.d/{grip}.key"));
        fs::write(&path, bytes).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
        (grip, path)
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn token() -> CancellationToken {
    CancellationToken::root(
        BoundaryKind::Kernel,
        TaskId::from_raw("signing"),
        CorrelationId::from_raw("signing"),
    )
}
fn cancel(t: &CancellationToken) {
    t.cancel(CancellationSignal {
        schema_version: CONTRACT_SCHEMA_VERSION,
        cancellation_id: CancellationId::from_raw("cancel-signing"),
        task_id: TaskId::from_raw("signing"),
        correlation_id: CorrelationId::from_raw("signing"),
        reason: CancellationReason::UserRequested,
        requested_by: BoundaryKind::Kernel,
    })
    .unwrap();
}
fn result(bytes: &[u8]) -> SupervisedResult {
    let stream = |bytes: &[u8]| CapturedStream {
        retained: bytes.to_vec(),
        total: bytes.len(),
        sha256: Sha256::digest(bytes).into(),
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
fn response() -> Vec<u8> {
    b"OK hello\nOK\nOK\nOK\nD (7:sig-val(5:eddsa(1:r1:a)(1:s1:b)))\nOK\nOK closing\n".to_vec()
}
fn read(fd: &OwnedFd) -> Vec<u8> {
    let mut bytes = vec![0; fstat(fd).unwrap().st_size as usize];
    assert_eq!(pread(fd, &mut bytes, 0).unwrap(), bytes.len());
    bytes
}

#[test]
fn private_signing_capture_seals_only_selected_bytes_and_refuses_unsafe_sources() {
    let f = Fixture::new();
    let (grip, path) = f.key(b"disposable selected bytes");
    fs::write(
        f.0.join("private-keys-v1.d/unrelated.key"),
        b"must not project",
    )
    .unwrap();
    fs::write(f.0.join("gpg-agent.conf"), b"must not read").unwrap();
    let key = PrivateKey::capture(&f.0, &grip, || Ok(())).unwrap();
    assert_eq!(read(&key.payload), b"disposable selected bytes");
    assert!(rustix::io::pwrite(&key.payload, b"x", 0).is_err());
    fs::write(&path, b"changed source").unwrap();
    assert_eq!(read(&key.payload), b"disposable selected bytes");
    for kind in [
        "empty",
        "oversize",
        "mode",
        "symlink",
        "hardlink",
        "directory",
        "fifo",
        "directory-link",
        "directory-mode",
        "root-mode",
        "special-mode",
    ] {
        let f = Fixture::new();
        let (grip, path) = f.key(b"disposable bytes");
        match kind {
            "empty" => fs::write(&path, b"").unwrap(),
            "oversize" => fs::write(&path, vec![0; MAX_PRIVATE_BYTES + 1]).unwrap(),
            "special-mode" => {
                fs::set_permissions(&path, fs::Permissions::from_mode(0o4600)).unwrap()
            }
            "mode" => fs::set_permissions(&path, fs::Permissions::from_mode(0o640)).unwrap(),
            "symlink" => {
                fs::remove_file(&path).unwrap();
                symlink("unrelated.key", &path).unwrap();
            }
            "hardlink" => fs::hard_link(&path, f.0.join("another-link")).unwrap(),
            "directory" => {
                fs::remove_file(&path).unwrap();
                fs::create_dir(&path).unwrap();
            }
            "fifo" => {
                fs::remove_file(&path).unwrap();
                rustix::fs::mknodat(
                    rustix::fs::CWD,
                    &path,
                    FileType::Fifo,
                    Mode::RUSR | Mode::WUSR,
                    0,
                )
                .unwrap();
            }
            "directory-link" => {
                fs::rename(f.0.join("private-keys-v1.d"), f.0.join("moved")).unwrap();
                symlink("moved", f.0.join("private-keys-v1.d")).unwrap();
            }
            "directory-mode" => fs::set_permissions(
                f.0.join("private-keys-v1.d"),
                fs::Permissions::from_mode(0o750),
            )
            .unwrap(),
            "root-mode" => fs::set_permissions(&f.0, fs::Permissions::from_mode(0o750)).unwrap(),
            _ => unreachable!(),
        }
        assert!(
            PrivateKey::capture(&f.0, &grip, || Ok(())).is_err(),
            "{kind}"
        );
    }
    for bad in [
        "",
        "../bad",
        "A\nPKSIGN",
        &"0".repeat(40),
        &"a".repeat(39),
        &"a".repeat(41),
        &"z".repeat(40),
    ] {
        assert!(PrivateKey::capture(&f.0, bad, || Ok(())).is_err());
    }
}

#[test]
fn private_signing_capture_observes_control_and_detects_replacement_before_and_after_seal() {
    for fail_at in 1..=6 {
        let f = Fixture::new();
        let (grip, _) = f.key(&vec![42; 32 * 1024 + 1]);
        let mut calls = 0;
        assert!(
            PrivateKey::capture(&f.0, &grip, || {
                calls += 1;
                if calls == fail_at {
                    Err(denied())
                } else {
                    Ok(())
                }
            })
            .is_err()
        );
        assert_eq!(calls, fail_at);
    }
    for checkpoint in [2, 3, 4] {
        for kind in ["file", "directory", "root", "mode"] {
            let f = Fixture::new();
            let (grip, path) = f.key(b"private fixture");
            let mut calls = 0;
            let got = PrivateKey::capture(&f.0, &grip, || {
                calls += 1;
                if calls == checkpoint {
                    match kind {
                        "file" => {
                            fs::rename(&path, f.0.join("old-key")).unwrap();
                            fs::write(&path, b"replacement").unwrap();
                        }
                        "directory" => {
                            fs::rename(f.0.join("private-keys-v1.d"), f.0.join("old-dir")).unwrap();
                            fs::DirBuilder::new()
                                .mode(0o700)
                                .create(f.0.join("private-keys-v1.d"))
                                .unwrap();
                        }
                        "root" => {
                            fs::rename(&f.0, f.0.with_extension("moved")).unwrap();
                            fs::DirBuilder::new().mode(0o700).create(&f.0).unwrap();
                        }
                        "mode" => {
                            fs::set_permissions(&path, fs::Permissions::from_mode(0o640)).unwrap()
                        }
                        _ => unreachable!(),
                    }
                }
                Ok(())
            });
            assert!(got.is_err(), "{checkpoint}/{kind}");
            if kind == "root" {
                fs::remove_dir_all(f.0.with_extension("moved")).unwrap();
            }
        }
    }
}

#[test]
fn private_signing_protocol_requires_exact_order_complete_data_and_no_inquiries() {
    assert!(parse_response(&response()).is_some());
    let percent = b"OK\nOK\nOK\nOK\nD (7:sig-val(1:x3:a%00%25))\nOK\nOK\n";
    assert_eq!(parse_response(percent).unwrap(), b"(7:sig-val(1:x3:a\0%))");
    for bad in [
        b"".as_slice(),
        b"OK\n",
        b"ERR failed\n",
        b"INQUIRE PASSPHRASE\n",
    ] {
        assert!(parse_response(bad).is_none());
    }
    let text = String::from_utf8(response()).unwrap();
    for changed in [
        text.trim_end().to_owned(),
        text.replace("D ", "ERR "),
        text.replace("D ", "S "),
        text.replace("D ", "INQUIRE "),
        text.replace("D ", "D %GG"),
        text.replace("(1:r1:a)", "(1:r1:%+1)"),
        text.replace("D ", "D %"),
        text.replace("D ", "D \r"),
        text.replace("OK hello\n", ""),
        text.replace("OK closing\n", ""),
        format!("D (7:sig-val())\n{text}"),
        format!("{text}OK\n"),
        format!("{text}D (7:sig-val())\n"),
        text.replace("(7:sig-val", "(8:not-sign"),
        text.replace("OK closing", "ERR closing"),
        text.replace("D (7:sig-val(5:eddsa(1:r1:a)(1:s1:b)))\n", ""),
    ] {
        assert!(
            parse_response(changed.as_bytes()).is_none(),
            "malformed protocol accepted"
        );
    }
    assert!(parse_response(&vec![b'\n'; MAX_SIGNATURE_BYTES + 1]).is_none());
}

#[test]
fn private_signing_percent_escapes_require_two_hex_digits_and_preserve_every_byte() {
    let wrap = |encoded: &[u8]| {
        [
            b"OK\nOK\nOK\nOK\nD (7:sig-val(1:x1:".as_slice(),
            encoded,
            b"))\nOK\nOK\n",
        ]
        .concat()
    };
    // Arbitrary signature octets, including NUL, LF, CR and percent, must
    // survive decoding. Case is permitted only within the two hex digits.
    for byte in 0..=255_u8 {
        let expected = [b"(7:sig-val(1:x1:".as_slice(), &[byte], b"))"].concat();
        for encoded in [format!("%{byte:02x}"), format!("%{byte:02X}")] {
            assert_eq!(parse_response(&wrap(encoded.as_bytes())).unwrap(), expected);
        }
    }
    // In particular the integer parser must not accept its optional leading
    // plus sign as part of an escape. Reject all other non-hex pairs too.
    for first in 0..=255_u8 {
        for second in 0..=255_u8 {
            if first.is_ascii_hexdigit() && second.is_ascii_hexdigit() {
                continue;
            }
            assert!(
                parse_response(&wrap(&[b'%', first, second])).is_none(),
                "non-hex escape accepted: {first:02x} {second:02x}"
            );
        }
    }
    for encoded in [b"%".as_slice(), b"%0", b"%\nD 00", b"%0\nD 0"] {
        assert!(parse_response(&wrap(encoded)).is_none());
    }
    // Splitting valid data records preserves bytes; splitting an escape does
    // not allow the next record to complete it.
    assert_eq!(
        parse_response(b"OK\nOK\nOK\nOK\nD (7:sig-val(\nD 1:x1:%25))\nOK\nOK\n").unwrap(),
        b"(7:sig-val(1:x1:%))"
    );
}

#[test]
fn private_signing_dispatch_requires_consistent_success_and_preserves_cleanup_uncertainty() {
    let deadline = Instant::now() + Duration::from_secs(5);
    for mutation in 0..12 {
        let mut r = result(&response());
        let t = token();
        match mutation {
            0 => r.output_complete = false,
            1 => r.status = None,
            2 => r.status = Some(std::process::ExitStatus::from_raw(256)),
            3 => r.stdout.total += 1,
            4 => r.stdout.sha256 = [0; 32],
            5 => r.stderr.total += 1,
            6 => r.stderr.sha256 = [0; 32],
            7 => r.stdout.total = MAX_SIGNATURE_BYTES + 1,
            8 => r.outcome = Ok(OperationOutcome::Cancelled),
            9 => r.outcome = Ok(OperationOutcome::TimedOut),
            10 => cancel(&t),
            11 => r.stdout.retained.clear(),
            _ => unreachable!(),
        }
        assert!(validate_result(r, &t, deadline).is_err());
    }
    assert!(validate_result(result(&response()), &token(), Instant::now()).is_err());
    let failure = crate::sandbox::LinuxSandboxError::for_test(
        crate::sandbox::LinuxSandboxErrorKind::CleanupUncertain,
    );
    let mut r = result(&response());
    r.outcome = Err(failure);
    assert_eq!(
        validate_result(r, &token(), deadline).unwrap_err().kind(),
        LinuxLocalCommitErrorKind::CleanupUncertain
    );
    let f = Fixture::new();
    let (grip, _) = f.key(b"disposable fixture");
    let key = PrivateKey::capture(&f.0, &grip, || Ok(())).unwrap();
    assert_eq!(
        sign_with(key, &grip, &[1; 32], &token(), deadline, |_| Err(error(
            LinuxLocalCommitErrorKind::CleanupUncertain
        )))
        .unwrap_err()
        .kind(),
        LinuxLocalCommitErrorKind::CleanupUncertain
    );
}

#[test]
fn private_signing_command_projects_no_host_keyring_socket_or_configuration_and_seals_input() {
    let f = Fixture::new();
    let (grip, _) = f.key(b"selected fixture bytes");
    let key = PrivateKey::capture(&f.0, &grip, || Ok(())).unwrap();
    let signature = sign_with(
        key,
        &grip.to_ascii_lowercase(),
        &[0xab; 32],
        &token(),
        Instant::now() + Duration::from_secs(5),
        |payload| {
            assert_eq!(read(&payload[0]), b"selected fixture bytes");
            assert_eq!(
                read(&payload[1]),
                format!(
                    "RESET\nSIGKEY {grip}\nSETHASH 8 {}\nPKSIGN\nBYE\n",
                    "ab".repeat(32)
                )
                .as_bytes()
            );
            for p in &payload {
                assert!(rustix::io::pwrite(p, b"x", 0).is_err());
            }
            let filter = FilterPipe::new().unwrap();
            let c = worker_command(
                Path::new("/trusted/systemd-run"),
                Path::new("/trusted/bwrap"),
                123,
                &payload,
                &grip,
                "agentmage-private-signing-0123456789abcdef01234567.service",
                &filter,
            )
            .unwrap();
            let args: Vec<_> = c.get_args().map(|s| s.to_str().unwrap()).collect();
            assert_eq!(
                args.iter()
                    .filter(|a| a.starts_with("--property=OpenFile="))
                    .count(),
                3
            );
            assert_eq!(args.iter().filter(|a| **a == "--ro-bind-fd").count(), 2);
            for required in [
                "--unshare-all",
                "--clearenv",
                "--seccomp",
                "--server",
                "--no-options",
                "--disable-scdaemon",
                "--no-allow-external-cache",
                "--property=KillMode=control-group",
            ] {
                assert!(args.contains(&required));
            }
            for forbidden in [
                "/run",
                "/run/user",
                "gpg-agent.conf",
                "S.gpg-agent",
                "/usr/bin",
                "--daemon",
                "--enable-ssh-support",
            ] {
                assert!(!args.contains(&forbidden));
            }
            assert!(!format!("{c:?}").contains(f.0.to_str().unwrap()));
            Ok(result(&response()))
        },
    )
    .unwrap();
    assert!(signature.starts_with(b"(7:sig-val("));
    for stopped in [false, true] {
        let key = PrivateKey::capture(&f.0, &grip, || Ok(())).unwrap();
        let t = token();
        if stopped {
            cancel(&t);
        }
        assert!(
            sign_with(
                key,
                &grip,
                &[1; 32],
                &t,
                if stopped {
                    Instant::now() + Duration::from_secs(5)
                } else {
                    Instant::now()
                },
                |_| panic!("no dispatch after cancellation/deadline")
            )
            .is_err()
        );
    }
}

#[test]
fn private_signing_installed_foreground_agent_reads_selected_sealed_key_without_sockets() {
    // Real disposable primitive through the fixed production protocol/arguments.
    // This is NOT the native systemd/Bubblewrap worker or an OpenPGP signature.
    let _capture = super::super::tests::DIRECT_CAPTURE_TEST
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let source = Fixture::new();
    struct SourceAgentGuard(PathBuf);
    impl Drop for SourceAgentGuard {
        fn drop(&mut self) {
            let _ = Command::new("/usr/bin/gpgconf")
                .env_clear()
                .env("PATH", "/usr/bin")
                .arg("--homedir")
                .arg(&self.0)
                .args(["--kill", "gpg-agent"])
                .output();
        }
    }
    // Drops before the fixture home even if a generation/capture assertion fails.
    let _source_agent = SourceAgentGuard(source.0.clone());
    let worker = Fixture::new();
    let gpg = |args: &[&str]| {
        let out = Command::new("/usr/bin/gpg")
            .env_clear()
            .env("PATH", "/usr/bin")
            .env("LANG", "C")
            .env("LC_ALL", "C")
            .args(["--no-options", "--batch", "--no-tty", "--homedir"])
            .arg(&source.0)
            .args(args)
            .output()
            .unwrap();
        assert!(out.status.success(), "fixture GnuPG failed");
        out.stdout
    };
    gpg(&[
        "--pinentry-mode",
        "loopback",
        "--passphrase",
        "",
        "--quick-generate-key",
        "Disposable Signing Fixture <fixture@example.test>",
        "ed25519",
        "sign",
        "1d",
    ]);
    let listed = gpg(&["--with-colons", "--with-keygrip", "--list-keys"]);
    let text = std::str::from_utf8(&listed).unwrap();
    let fingerprint = text
        .lines()
        .find(|l| l.starts_with("fpr:"))
        .unwrap()
        .split(':')
        .nth(9)
        .unwrap()
        .to_ascii_lowercase();
    let grip = super::super::inspection::selected_keygrip(&listed, &fingerprint).unwrap();
    let stop = Command::new("/usr/bin/gpgconf")
        .env_clear()
        .env("PATH", "/usr/bin")
        .arg("--homedir")
        .arg(&source.0)
        .args(["--kill", "gpg-agent"])
        .output()
        .unwrap();
    assert!(stop.status.success());
    let key = PrivateKey::capture(&source.0, &grip, || Ok(())).unwrap();
    fs::DirBuilder::new()
        .mode(0o700)
        .create(worker.0.join("private-keys-v1.d"))
        .unwrap();
    let path = worker.0.join(format!("private-keys-v1.d/{grip}.key"));
    // Same sealed payload, named only in this private disposable fixture. The
    // product uses a read-only descriptor mount, never writes a private key.
    fs::write(&path, read(&key.payload)).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
    let t = token();
    let deadline = Instant::now() + Duration::from_secs(5);
    let data = sign_with(key, &grip, &[0xab; 32], &t, deadline, |payload| {
        let input = rustix::io::fcntl_dupfd_cloexec(&payload[1], 3).unwrap();
        let mut command = Command::new("/usr/bin/gpg-agent");
        command
            .env_clear()
            .env("LANG", "C")
            .env("LC_ALL", "C")
            .args(agent_arguments(worker.0.to_str().unwrap()))
            .stdin(Stdio::from(fs::File::from(input)));
        let captured =
            crate::sandbox::supervision::capture_local_commit_process(command, Some(&t), deadline)
                .map_err(|e| capture_error(e.kind(), LinuxLocalCommitErrorKind::ExecutionFailed))?;
        let mut r = result(&captured.stdout);
        r.status = Some(captured.status);
        r.stderr = CapturedStream {
            total: captured.stderr.len(),
            sha256: Sha256::digest(&captured.stderr).into(),
            retained: captured.stderr,
        };
        Ok(r)
    })
    .unwrap();
    assert!(data.starts_with(b"(7:sig-val(5:eddsa"));
    for home in [&source.0, &worker.0] {
        assert!(!home.join("S.gpg-agent").exists());
    }
    assert!(!worker.0.join("gpg-agent.conf").exists());
}

#[test]
fn private_signing_refuses_invalid_selection_before_capture_and_control_changes_after_dispatch() {
    let fixture = Fixture::new();
    let (valid_grip, _) = fixture.key(b"selected private fixture bytes");
    assert_eq!(
        read(
            &PrivateKey::capture(&fixture.0, &valid_grip, || Ok(()))
                .unwrap()
                .payload
        ),
        b"selected private fixture bytes"
    );
    let public_key = b"synthetic public export".to_vec();
    let mut pinned = super::super::tests::synthetic_plan(&hash(b"fixture repository")).signer;
    pinned.public_key_sha256 = hash(&public_key);
    let mut signer = LinuxOpenPgpSigner {
        executable: PathBuf::from("/never-launched"),
        executable_sha256: pinned.signer_executable_sha256.clone(),
        keyring: fixture.0.clone(),
        public_key,
        keygrip: None,
        pinned: PinnedCommitSigner::seal(pinned).unwrap(),
    };
    let deadline = Instant::now() + Duration::from_secs(5);
    assert_eq!(verify_private_keyring(&signer.keyring).unwrap(), fixture.0);
    assert!(signer.pinned.verify().is_ok());
    assert_eq!(hash(&signer.public_key), signer.pinned.public_key_sha256);
    assert_eq!(
        sign_hash(&signer, &[1; 32], &token(), deadline)
            .unwrap_err()
            .kind(),
        LinuxLocalCommitErrorKind::ArtifactDenied
    );
    signer.keygrip = Some("A\nPKSIGN".into());
    assert_eq!(
        sign_hash(&signer, &[1; 32], &token(), deadline)
            .unwrap_err()
            .kind(),
        LinuxLocalCommitErrorKind::ArtifactDenied
    );
    let t = token();
    cancel(&t);
    assert!(sign_hash(&signer, &[1; 32], &t, deadline).is_err());
    assert!(sign_hash(&signer, &[1; 32], &token(), Instant::now()).is_err());
    let f = Fixture::new();
    let (grip, _) = f.key(b"fixture bytes");
    let key = PrivateKey::capture(&f.0, &grip, || Ok(())).unwrap();
    let t = token();
    assert!(
        sign_with(key, &grip, &[1; 32], &t, deadline, |_| {
            cancel(&t);
            Ok(result(&response()))
        })
        .is_err()
    );
}
