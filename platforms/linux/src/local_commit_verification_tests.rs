// SPDX-License-Identifier: Apache-2.0
//! Component contracts; synthetic supervisor results do not prove native admission.

use super::*;
use crate::local_commit::tests::synthetic_plan;
use crate::sandbox::supervision::CapturedStream;
use agentmage_kernel_contracts::*;
use std::os::unix::process::ExitStatusExt as _;

fn token() -> CancellationToken {
    CancellationToken::root(
        BoundaryKind::Shell,
        TaskId::from_raw("verify"),
        CorrelationId::from_raw("verify"),
    )
}
fn cancel(t: &CancellationToken) {
    t.cancel(CancellationSignal {
        schema_version: CONTRACT_SCHEMA_VERSION,
        cancellation_id: CancellationId::from_raw("verify-cancel"),
        task_id: TaskId::from_raw("verify"),
        correlation_id: CorrelationId::from_raw("verify"),
        reason: CancellationReason::UserRequested,
        requested_by: BoundaryKind::Shell,
    })
    .unwrap();
}
fn fixture() -> (LinuxOpenPgpSigner, LocalCommitPlan) {
    let mut plan = synthetic_plan(&hash(b"verification repository"));
    let public_key = b"synthetic public material".to_vec();
    plan.signer.public_key_sha256 = hash(&public_key);
    plan.signer = PinnedCommitSigner::seal(plan.signer).unwrap();
    reseal(&mut plan);
    (
        LinuxOpenPgpSigner {
            executable: PathBuf::from("/usr/bin/gpg"),
            executable_sha256: plan.signer.signer_executable_sha256.clone(),
            keyring: PathBuf::from("/never-projected-private-keyring"),
            public_key,
            pinned: plan.signer.clone(),
        },
        plan,
    )
}
fn reseal(plan: &mut LocalCommitPlan) {
    plan.plan_sha256 = "0".repeat(64);
    plan.plan_sha256 = hash(&serde_json::to_vec(plan).unwrap());
    agentmage_kernel_engine::local_commit::verify_local_commit_plan(plan).unwrap();
}
fn unsigned(plan: &LocalCommitPlan) -> Vec<u8> {
    format!(
        "tree {}\nparent {}\nauthor {} <{}> {} {}\ncommitter {} <{}> {} {}\n\n{}",
        plan.candidate_tree,
        plan.parent_object,
        plan.author.name,
        plan.author.email,
        plan.timestamp_epoch_seconds,
        git_timezone(plan.timezone_offset_minutes),
        plan.committer.name,
        plan.committer.email,
        plan.timestamp_epoch_seconds,
        git_timezone(plan.timezone_offset_minutes),
        plan.message
    )
    .into_bytes()
}
fn raw(plan: &LocalCommitPlan) -> Vec<u8> {
    let unsigned = unsigned(plan);
    let split = unsigned.windows(2).position(|w| w == b"\n\n").unwrap();
    let mut raw = unsigned[..split].to_vec();
    let header = if plan.parent_object.len() == 40 {
        "gpgsig"
    } else {
        "gpgsig-sha256"
    };
    raw.extend_from_slice(format!("\n{header} -----BEGIN PGP SIGNATURE-----\n \n fixture armor\n -----END PGP SIGNATURE-----").as_bytes());
    raw.extend_from_slice(&unsigned[split..]);
    raw
}
fn status(plan: &LocalCommitPlan) -> Vec<u8> {
    format!(
        "[GNUPG:] VALIDSIG {} 2026-11-10 1794268800 0 4 0 22 8 00 {}\n",
        plan.signer.key_fingerprint, plan.signer.key_fingerprint
    )
    .into_bytes()
}
fn result(status: &[u8]) -> SupervisedResult {
    let stream = |bytes: &[u8]| CapturedStream {
        retained: bytes.to_vec(),
        total: bytes.len(),
        sha256: Sha256::digest(bytes).into(),
    };
    SupervisedResult {
        outcome: Ok(OperationOutcome::Succeeded),
        status: Some(std::process::ExitStatus::from_raw(0)),
        stdout: stream(status),
        stderr: stream(b"gpg: diagnostic only\n"),
        resources: None,
        output_complete: true,
    }
}
fn read(fd: &OwnedFd) -> Vec<u8> {
    let mut bytes = vec![0; 4096];
    let count = rustix::io::pread(fd, &mut bytes, 0).unwrap();
    bytes.truncate(count);
    bytes
}

#[test]
fn commit_verification_extracts_exact_signed_content_in_both_formats() {
    let (_, mut plan) = fixture();
    for length in [40, 64] {
        plan.parent_object = "b".repeat(length);
        plan.candidate_tree = "c".repeat(length);
        for message in [
            "One line",
            "One line\n",
            "First\n\nSecond\n\n",
            "Unicode: café\n",
        ] {
            plan.message = message.into();
            plan.message_sha256 = hash(message.as_bytes());
            reseal(&mut plan);
            let bytes = raw(&plan);
            let (signature, content) = split_signed_commit(&bytes, &plan).unwrap();
            assert_eq!(
                signature,
                b"-----BEGIN PGP SIGNATURE-----\n\nfixture armor\n-----END PGP SIGNATURE-----\n"
            );
            assert_eq!(content, unsigned(&plan));
            let text = String::from_utf8(bytes).unwrap();
            for bad in [
                text.replace("fixture armor", "fixture armor\ngpgsig forged"),
                text.replace("-----END PGP SIGNATURE-----", "broken end"),
                text.replace("parent ", "parent 0"),
                format!("{text}changed"),
            ] {
                assert!(split_signed_commit(bad.as_bytes(), &plan).is_err());
            }
        }
    }
}

#[test]
fn commit_verification_projects_only_exact_sealed_public_material() {
    let (signer, plan) = fixture();
    let raw = raw(&plan);
    let deadline = Instant::now() + Duration::from_secs(5);
    verify_with(&signer, &raw, &plan, &token(), deadline, |payload| {
        let expected = [
            &signer.public_key,
            &split_signed_commit(&raw, &plan).unwrap().0,
            &unsigned(&plan),
        ];
        for (fd, expected) in payload.iter().zip(expected) {
            assert_eq!(&read(fd), expected);
            assert_eq!(
                rustix::io::pwrite(fd, b"X", 0).unwrap_err(),
                rustix::io::Errno::PERM
            );
        }
        Ok(result(&status(&plan)))
    })
    .unwrap();
}

#[test]
fn commit_verification_refuses_stale_public_material_plan_cancel_and_deadline_before_dispatch() {
    let (mut signer, plan) = fixture();
    let no_dispatch = |_| panic!("invalid verification must not dispatch");
    let future = Instant::now() + Duration::from_secs(5);
    signer.public_key.push(0);
    assert!(verify_with(&signer, &raw(&plan), &plan, &token(), future, no_dispatch).is_err());
    signer.public_key.pop();
    for invalid in [Vec::new(), vec![0; LIMIT + 1]] {
        assert!(verify_with(&signer, &invalid, &plan, &token(), future, no_dispatch).is_err());
    }
    let mut changed = plan.clone();
    changed.signer.key_fingerprint = "d".repeat(40);
    assert!(
        verify_with(
            &signer,
            &raw(&changed),
            &changed,
            &token(),
            future,
            no_dispatch
        )
        .is_err()
    );
    let t = token();
    cancel(&t);
    assert!(verify_with(&signer, &raw(&plan), &plan, &t, future, no_dispatch).is_err());
    assert!(
        verify_with(
            &signer,
            &raw(&plan),
            &plan,
            &token(),
            Instant::now(),
            no_dispatch
        )
        .is_err()
    );
    assert!(
        verify_with(&signer, &raw(&plan), &plan, &token(), future, |_| {
            Err(error(LinuxLocalCommitErrorKind::CleanupUncertain))
        })
        .is_err_and(|e| e.kind() == LinuxLocalCommitErrorKind::CleanupUncertain)
    );
}

#[test]
fn commit_verification_requires_complete_success_and_exact_status_after_cleanup() {
    let (_, plan) = fixture();
    let status = status(&plan);
    let deadline = Instant::now() + Duration::from_secs(5);
    for case in 0..18 {
        let mut r = result(&status);
        match case {
            0 => r.outcome = Ok(OperationOutcome::Failed),
            1 => r.outcome = Ok(OperationOutcome::Cancelled),
            2 => r.outcome = Ok(OperationOutcome::TimedOut),
            3 => r.status = None,
            4 => r.status = Some(std::process::ExitStatus::from_raw(7 << 8)),
            5 => r.output_complete = false,
            6 => r.stdout.total = 1,
            7 => r.stdout.retained.push(0),
            8 => r.stderr.total = LIMIT + 1,
            9 => r.stderr.total -= 1,
            10 => r.stderr.sha256 = [0; 32],
            11 => r = result(&[status.clone(), status.clone()].concat()),
            12 => r = result(&[status.clone(), b"[GNUPG:] BADSIG x\n".to_vec()].concat()),
            13 => {
                r = result(
                    &String::from_utf8(status.clone())
                        .unwrap()
                        .replace(&plan.signer.key_fingerprint, &"d".repeat(40))
                        .into_bytes(),
                )
            }
            14 => r.stdout.total = LIMIT + 1,
            15 => r.stdout.total -= 1,
            16 => r.stdout.sha256 = [0; 32],
            17 => r.stderr.retained.push(0),
            _ => unreachable!(),
        }
        assert!(
            validate_result(r, &plan.signer.key_fingerprint, &token(), deadline).is_err(),
            "case {case}"
        );
    }
    let t = token();
    cancel(&t);
    assert!(validate_result(result(&status), &plan.signer.key_fingerprint, &t, deadline).is_err());
    assert!(
        validate_result(
            result(&status),
            &plan.signer.key_fingerprint,
            &token(),
            Instant::now()
        )
        .is_err()
    );
    validate_result(
        result(&status),
        &plan.signer.key_fingerprint,
        &token(),
        deadline,
    )
    .unwrap();
}

#[test]
fn commit_verification_never_uses_diagnostics_as_signature_authority() {
    let (_, plan) = fixture();
    let good = status(&plan);
    let fingerprint = &plan.signer.key_fingerprint;
    let deadline = Instant::now() + Duration::from_secs(5);
    let check = |r| validate_result(r, fingerprint, &token(), deadline);
    // A complete successful process and valid-looking diagnostics cannot stand
    // in for machine status, including a swapped pair of authentic streams.
    for stdout in [
        Vec::new(),
        b"gpg: verified\n".to_vec(),
        b"[GNUPG:] GOODSIG x\n".to_vec(),
    ] {
        let mut r = result(&stdout);
        r.stderr = result(&good).stdout;
        assert!(check(r).is_err());
    }
    // Conversely neither failure-looking status, human logs nor malformed bytes
    // on stderr can override an authentic valid status stream. Stderr remains
    // length/hash checked and bounded, but it is not parsed for a verdict.
    for diagnostics in [
        b"".as_slice(),
        b"[GNUPG:] BADSIG x\n".as_slice(),
        good.as_slice(),
        b"human log without trailing newline",
        &[0, 255, b'\r', b'\n'],
    ] {
        let mut r = result(&good);
        r.stderr = result(diagnostics).stdout;
        check(r).unwrap();
    }
}

#[test]
fn commit_verification_status_stream_refuses_mixed_partial_and_malformed_records() {
    let (_, plan) = fixture();
    let good = status(&plan);
    let deadline = Instant::now() + Duration::from_secs(5);
    for bytes in [
        [b"gpg: human diagnostic\n".as_slice(), &good].concat(),
        [&good, b"human suffix\n".as_slice()].concat(),
        [&good, b"\n".as_slice()].concat(),
        [&good, b"[GNUPG:] \n".as_slice()].concat(),
        [&good, b"[GNUPG:] KEY_CONSIDERED partial".as_slice()].concat(),
        good[..good.len() - 1].to_vec(),
        [&good, b"\0\n".as_slice()].concat(),
        [good.as_slice(), &[255, b'\n']].concat(),
        good.iter()
            .flat_map(|b| {
                if *b == b'\n' {
                    vec![b'\r', b'\n']
                } else {
                    vec![*b]
                }
            })
            .collect::<Vec<u8>>(),
    ] {
        assert!(
            validate_result(
                result(&bytes),
                &plan.signer.key_fingerprint,
                &token(),
                deadline
            )
            .is_err()
        );
    }
    // Additional complete status records are expected in real GnuPG output.
    let complete = [
        b"[GNUPG:] NEWSIG\n".as_slice(),
        &good,
        b"[GNUPG:] TRUST_UNDEFINED 0 pgp\n".as_slice(),
    ]
    .concat();
    validate_result(
        result(&complete),
        &plan.signer.key_fingerprint,
        &token(),
        deadline,
    )
    .unwrap();
}

#[test]
fn commit_verification_command_has_no_repository_private_keyring_or_host_socket_projection() {
    let payload = ["public", "signature", "content"]
        .map(|n| crate::sandbox::sealed_payload(n, n.as_bytes()).unwrap());
    let gpg = crate::sandbox::sealed_payload("synthetic-gpg", b"not executable").unwrap();
    let filter = FilterPipe::new().unwrap();
    let c = worker_command(
        Path::new("/synthetic/systemd-run"),
        Path::new("/synthetic/bwrap"),
        gpg.as_raw_fd(),
        &payload,
        "agentmage-commit-verification-0123456789abcdef01234567.service",
        &filter,
    );
    assert_eq!(c.get_program(), "/synthetic/systemd-run");
    let args: Vec<_> = c.get_args().map(|a| a.to_str().unwrap()).collect();
    let properties: Vec<_> = args
        .iter()
        .filter(|a| a.starts_with("--property=OpenFile="))
        .collect();
    assert_eq!(properties.len(), 5);
    for (fd, name) in [
        (gpg.as_raw_fd(), "gpg"),
        (filter.read_descriptor(), "seccomp"),
        (payload[0].as_raw_fd(), "public"),
        (payload[1].as_raw_fd(), "signature"),
        (payload[2].as_raw_fd(), "content"),
    ] {
        assert!(
            properties.contains(
                &&format!(
                    "--property=OpenFile=/proc/{}/fd/{fd}:{name}:read-only",
                    std::process::id()
                )
                .as_str()
            )
        );
    }
    for mandatory in [
        "--property=KillMode=control-group",
        "--property=SendSIGKILL=yes",
        "--property=Restart=no",
        "--property=MemoryMax=268435456",
        "--property=TasksMax=16",
        "--property=CPUQuota=100%",
        "--unshare-all",
        "--disable-userns",
        "--clearenv",
        "--seccomp",
        "--no-options",
        "--no-autostart",
        "--no-auto-key-retrieve",
        "--status-fd=1",
    ] {
        assert!(args.contains(&mandatory), "{mandatory}");
    }
    let guest = args.iter().position(|a| *a == "/synthetic/bwrap").unwrap();
    for forbidden in [
        "--bind",
        "--bind-fd",
        "--share-net",
        "GNUPGHOME",
        "DBUS_SESSION_BUS_ADDRESS",
        "--git-dir",
        "--work-tree",
        "--status-fd=2",
    ] {
        assert!(!args[guest..].contains(&forbidden), "{forbidden}");
    }
    assert_eq!(
        &args[args.len() - gpg_arguments(GUEST_HOME, PUBLIC_KEY, SIGNATURE, CONTENT).len()..],
        gpg_arguments(GUEST_HOME, PUBLIC_KEY, SIGNATURE, CONTENT)
    );
    let env: Vec<_> = c.get_envs().map(|(key, _)| key.to_str().unwrap()).collect();
    assert_eq!(env, ["DBUS_SESSION_BUS_ADDRESS", "XDG_RUNTIME_DIR"]);
    let mounts: Vec<_> = args
        .windows(3)
        .filter(|a| a[0] == "--ro-bind-fd")
        .map(|a| (a[1], a[2]))
        .collect();
    assert_eq!(
        mounts,
        [
            ("3", GUEST_GPG),
            ("5", PUBLIC_KEY),
            ("6", SIGNATURE),
            ("7", CONTENT)
        ]
    );
}
