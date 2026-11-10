// SPDX-License-Identifier: Apache-2.0
//! Disposable component cryptography; no native sandbox or approval admission.

use super::*;
use agentmage_kernel_contracts::*;
use std::os::unix::fs::DirBuilderExt as _;
use std::sync::atomic::{AtomicU64, Ordering};

fn plan() -> LocalCommitPlan {
    super::super::tests::synthetic_plan(&hash(b"packet fixture"))
}
fn reseal(plan: &mut LocalCommitPlan) {
    plan.message_sha256 = hash(plan.message.as_bytes());
    plan.plan_sha256 = "0".repeat(64);
    plan.plan_sha256 = hash(&serde_json::to_vec(plan).unwrap());
}
fn signer(plan: &LocalCommitPlan) -> LinuxOpenPgpSigner {
    LinuxOpenPgpSigner {
        executable: "/usr/bin/gpg".into(),
        executable_sha256: hash(b"synthetic artifact"),
        keyring: "/nonexistent".into(),
        public_key: b"public fixture".to_vec(),
        keygrip: None,
        pinned: plan.signer.clone(),
    }
}
fn token() -> CancellationToken {
    CancellationToken::root(
        BoundaryKind::Kernel,
        TaskId::from_raw("packet"),
        CorrelationId::from_raw("packet"),
    )
}
fn cancel(token: &CancellationToken) {
    token
        .cancel(CancellationSignal {
            schema_version: CONTRACT_SCHEMA_VERSION,
            cancellation_id: CancellationId::from_raw("cancel-packet"),
            task_id: TaskId::from_raw("packet"),
            correlation_id: CorrelationId::from_raw("packet"),
            reason: CancellationReason::UserRequested,
            requested_by: BoundaryKind::Kernel,
        })
        .unwrap();
}
fn expression(scalar: &[u8]) -> Vec<u8> {
    let mut out = format!("(7:sig-val(3:rsa(1:s{}:", scalar.len()).into_bytes();
    out.extend_from_slice(scalar);
    out.extend_from_slice(b")))");
    out
}
fn bound_plan() -> LocalCommitPlan {
    let mut p = plan();
    p.signer.public_key_sha256 = hash(b"public fixture");
    p.signer.signer_sha256.clear();
    p.signer = PinnedCommitSigner::seal(p.signer).unwrap();
    reseal(&mut p);
    p
}

#[test]
fn commit_packet_preserves_exact_messages_and_headers_in_both_object_formats() {
    for width in [40, 64] {
        for message in [
            "no final newline",
            "trailing\n\n",
            " leading\nlines\r\n\nΩ\n",
        ] {
            let mut p = plan();
            p.parent_object = "b".repeat(width);
            p.candidate_tree = "c".repeat(width);
            p.message = message.into();
            reseal(&mut p);
            let prepared = Prepared::new(&p).unwrap();
            let raw = prepared.assemble(&expression(&[1, 2, 3])).unwrap();
            assert!(verify_raw_commit(&raw, &p));
            let (signature, content) = verification::split_signed_commit(&raw, &p).unwrap();
            assert_eq!(content, prepared.content);
            assert!(content.ends_with(message.as_bytes()));
            assert!(signature.ends_with(b"-----END PGP SIGNATURE-----\n"));
            assert_eq!(
                raw.windows(7).filter(|w| *w == b"gpgsig ").count(),
                usize::from(width == 40)
            );
            assert_eq!(
                raw.windows(14).filter(|w| *w == b"gpgsig-sha256 ").count(),
                usize::from(width == 64)
            );
            let mut changed = p.clone();
            changed.message.push('!');
            reseal(&mut changed);
            assert!(verification::split_signed_commit(&raw, &changed).is_err());
        }
    }
}

#[test]
fn commit_packet_refuses_stale_mixed_format_and_unrepresentable_plans() {
    let mut p = plan();
    p.message.push('!');
    assert!(Prepared::new(&p).is_err());
    for timestamp in [-1, i64::from(u32::MAX) + 1] {
        let mut p = plan();
        p.timestamp_epoch_seconds = timestamp;
        reseal(&mut p);
        assert!(Prepared::new(&p).is_err());
    }
    let mut p = plan();
    p.candidate_tree = "c".repeat(64);
    reseal(&mut p);
    assert!(Prepared::new(&p).is_err());
}

#[test]
fn commit_packet_unsigned_content_matches_installed_git_without_message_normalization() {
    for format in ["sha1", "sha256"] {
        let root = Fixture::new();
        let mut p = plan();
        let git = |args: &[&str], input: &[u8], p: &LocalCommitPlan| {
            let fd = crate::sandbox::sealed_payload("unsigned-fixture", input).unwrap();
            let output = Command::new("/usr/bin/git")
                .env_clear()
                .env("PATH", "/usr/bin")
                .env("GIT_CONFIG_NOSYSTEM", "1")
                .env("GIT_CONFIG_GLOBAL", "/dev/null")
                .env("GIT_AUTHOR_NAME", &p.author.name)
                .env("GIT_AUTHOR_EMAIL", &p.author.email)
                .env("GIT_COMMITTER_NAME", &p.committer.name)
                .env("GIT_COMMITTER_EMAIL", &p.committer.email)
                .env(
                    "GIT_AUTHOR_DATE",
                    format!(
                        "@{} {}",
                        p.timestamp_epoch_seconds,
                        git_timezone(p.timezone_offset_minutes)
                    ),
                )
                .env(
                    "GIT_COMMITTER_DATE",
                    format!(
                        "@{} {}",
                        p.timestamp_epoch_seconds,
                        git_timezone(p.timezone_offset_minutes)
                    ),
                )
                .arg("-C")
                .arg(&root.0)
                .args(["-c", "core.hooksPath=/dev/null", "-c", "gc.auto=0"])
                .args(args)
                .stdin(Stdio::from(fs::File::from(fd)))
                .output()
                .unwrap();
            assert!(output.status.success(), "disposable Git operation failed");
            output.stdout
        };
        git(
            &["init", "--quiet", &format!("--object-format={format}")],
            b"",
            &p,
        );
        let tree = String::from_utf8(git(&["mktree"], b"", &p))
            .unwrap()
            .trim()
            .to_owned();
        let parent =
            String::from_utf8(git(&["commit-tree", "--no-gpg-sign", &tree], b"parent", &p))
                .unwrap()
                .trim()
                .to_owned();
        p.parent_object = parent;
        p.candidate_tree = tree;
        for message in ["no newline", "trailing\n\n", " leading\r\nΩ"] {
            p.message = message.into();
            reseal(&mut p);
            let oid = String::from_utf8(git(
                &[
                    "commit-tree",
                    "--no-gpg-sign",
                    &p.candidate_tree,
                    "-p",
                    &p.parent_object,
                ],
                message.as_bytes(),
                &p,
            ))
            .unwrap()
            .trim()
            .to_owned();
            let actual = git(&["cat-file", "commit", &oid], b"", &p);
            assert_eq!(Prepared::new(&p).unwrap().content, actual);
        }
    }
}

#[test]
fn commit_packet_rsa_parser_refuses_ambiguity_lengths_and_other_algorithms() {
    assert_eq!(
        rsa_scalar(&expression(&[128, 1])),
        Some([128, 1].as_slice())
    );
    assert_eq!(
        rsa_scalar(&expression(&[127, 255])),
        Some([127, 255].as_slice())
    );
    for bytes in [&[][..], &[0], &[0, 1], &[0, 0, 128], &[0, 128], &[0, 255]] {
        assert!(rsa_scalar(&expression(bytes)).is_none());
    }
    let good = expression(&[1, 0, 255]);
    for end in 0..good.len() {
        assert!(rsa_scalar(&good[..end]).is_none());
    }
    for bad in [
        b"(7:sig-val(3:rsa(1:s01:a)))".as_slice(),
        b"(7:sig-val(3:rsa(1:s+1:a)))",
        b"(7:sig-val(3:rsa(1:s9999:a)))",
        b"(7:sig-val(5:eddsa(1:r1:a)(1:s1:b)))",
        b"(7:sig-val(3:rsa(1:s1:a)(1:s1:b)))",
        b"(7:sig-val(3:rsa(1:s1:a)))extra",
    ] {
        assert!(rsa_scalar(bad).is_none());
    }
    assert!(rsa_scalar(&expression(&vec![1; MAX_RSA_BYTES + 1])).is_none());
}

#[test]
fn commit_packet_verification_is_mandatory_and_control_stops_each_stage() {
    let p = bound_plan();
    let s = signer(&p);
    let t = token();
    let deadline = Instant::now() + Duration::from_secs(5);
    assert!(
        sign_commit_with(
            &s,
            &p,
            &t,
            deadline,
            |_| Err(refused()),
            |_| panic!("no verify")
        )
        .is_err()
    );
    assert!(
        sign_commit_with(
            &s,
            &p,
            &t,
            deadline,
            |_| Ok(b"invalid".to_vec()),
            |_| panic!("no verify")
        )
        .is_err()
    );
    assert!(
        sign_commit_with(
            &s,
            &p,
            &t,
            deadline,
            |_| Ok(expression(&[1])),
            |_| Err(error(LinuxLocalCommitErrorKind::VerificationFailed))
        )
        .is_err()
    );
    let uncertain = sign_commit_with(
        &s,
        &p,
        &t,
        deadline,
        |_| Ok(expression(&[1])),
        |_| Err(error(LinuxLocalCommitErrorKind::CleanupUncertain)),
    )
    .unwrap_err();
    assert_eq!(
        uncertain.kind(),
        LinuxLocalCommitErrorKind::CleanupUncertain
    );
    for stage in 0..3 {
        let t = token();
        if stage == 0 {
            cancel(&t);
        }
        assert!(
            sign_commit_with(
                &s,
                &p,
                &t,
                deadline,
                |_| {
                    assert_ne!(stage, 0);
                    if stage == 1 {
                        cancel(&t);
                    }
                    Ok(expression(&[1]))
                },
                |_| {
                    assert_eq!(stage, 2);
                    cancel(&t);
                    Ok(())
                },
            )
            .is_err()
        );
    }
    assert!(
        sign_commit_with(
            &s,
            &p,
            &t,
            Instant::now(),
            |_| panic!("expired"),
            |_| panic!("expired")
        )
        .is_err()
    );
    let mut changed = signer(&p);
    changed.public_key.push(1);
    assert!(
        sign_commit_with(
            &changed,
            &p,
            &t,
            deadline,
            |_| panic!("public mismatch"),
            |_| panic!("public mismatch")
        )
        .is_err()
    );
    let mut changed = p.clone();
    changed.signer.key_fingerprint = "b".repeat(40);
    changed.signer.signer_sha256.clear();
    changed.signer = PinnedCommitSigner::seal(changed.signer).unwrap();
    reseal(&mut changed);
    assert!(
        sign_commit_with(
            &s,
            &changed,
            &t,
            deadline,
            |_| panic!("signer mismatch"),
            |_| panic!("signer mismatch")
        )
        .is_err()
    );
}

static NONCE: AtomicU64 = AtomicU64::new(1);
struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "pk-{}-{}",
            std::process::id(),
            NONCE.fetch_add(1, Ordering::Relaxed)
        ));
        fs::DirBuilder::new().mode(0o700).create(&path).unwrap();
        Self(path)
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
struct AgentGuard(PathBuf);
impl AgentGuard {
    fn stop(&self) {
        assert!(
            Command::new("/usr/bin/gpgconf")
                .env_clear()
                .env("PATH", "/usr/bin")
                .arg("--homedir")
                .arg(&self.0)
                .args(["--kill", "gpg-agent"])
                .output()
                .unwrap()
                .status
                .success()
        );
    }
}
impl Drop for AgentGuard {
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

#[test]
fn commit_packet_disposable_rsa_primary_and_subkey_verify_exact_content_and_refuse_tampering() {
    let _capture = super::super::tests::DIRECT_CAPTURE_TEST
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let source = Fixture::new();
    let agent = AgentGuard(source.0.clone());
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
        assert!(out.status.success(), "disposable key operation failed");
        out.stdout
    };
    gpg(&[
        "--pinentry-mode",
        "loopback",
        "--passphrase",
        "",
        "--quick-generate-key",
        "Disposable Packet Fixture <fixture@example.test>",
        "rsa2048",
        "sign",
        "1d",
    ]);
    let listed = gpg(&["--with-colons", "--with-keygrip", "--list-keys"]);
    let primary = std::str::from_utf8(&listed)
        .unwrap()
        .lines()
        .find(|l| l.starts_with("fpr:"))
        .unwrap()
        .split(':')
        .nth(9)
        .unwrap()
        .to_ascii_lowercase();
    gpg(&[
        "--pinentry-mode",
        "loopback",
        "--passphrase",
        "",
        "--quick-add-key",
        &primary,
        "rsa2048",
        "sign",
        "1d",
    ]);
    let listed = gpg(&["--with-colons", "--with-keygrip", "--list-keys"]);
    let fingerprints: Vec<_> = std::str::from_utf8(&listed)
        .unwrap()
        .lines()
        .filter(|l| l.starts_with("fpr:"))
        .map(|l| l.split(':').nth(9).unwrap().to_ascii_lowercase())
        .collect();
    assert_eq!(fingerprints.len(), 2);
    let public = gpg(&["--export", &primary]);
    agent.stop();
    for fingerprint in &fingerprints {
        let grip = inspection::selected_keygrip(&listed, fingerprint).unwrap();
        for width in [40, 64] {
            let mut p = bound_plan();
            p.parent_object = "b".repeat(width);
            p.candidate_tree = "c".repeat(width);
            p.timestamp_epoch_seconds = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64;
            p.message = "Exact bytes\n\nΩ\r\nno final newline".into();
            p.signer.key_fingerprint = fingerprint.clone();
            p.signer.public_key_sha256 = hash(&public);
            p.signer.signer_sha256.clear();
            p.signer = PinnedCommitSigner::seal(p.signer).unwrap();
            reseal(&mut p);
            let mut s = signer(&p);
            s.public_key = public.clone();
            let worker = Fixture::new();
            let verifier = Fixture::new();
            let t = token();
            let mut captured_expression = Vec::new();
            let raw = sign_commit_with(
                &s,
                &p,
                &t,
                Instant::now() + Duration::from_secs(10),
                |digest| {
                    captured_expression =
                        signing::fixture_sign(&source.0, &worker.0, &grip, digest, &t)?;
                    Ok(captured_expression.clone())
                },
                |raw| {
                    let output = verification::fixture_verify(raw, &p, &public, &verifier.0, &t)?;
                    if verify_signature_status(&output.stdout, fingerprint) {
                        Ok(())
                    } else {
                        Err(error(LinuxLocalCommitErrorKind::VerificationFailed))
                    }
                },
            )
            .unwrap();
            assert!(verify_raw_commit(&raw, &p));
            assert_eq!(
                verification::split_signed_commit(&raw, &p).unwrap().1,
                Prepared::new(&p).unwrap().content
            );
            // Independently verify changed signed content with the old signature.
            let mut changed = p.clone();
            changed.message.push('!');
            reseal(&mut changed);
            let mutated = Prepared::new(&changed)
                .unwrap()
                .assemble(&captured_expression)
                .unwrap();
            assert!(
                verification::fixture_verify(&mutated, &changed, &public, &verifier.0, &t).is_err()
            );
            // Corrupt the RSA value but retain a structurally valid packet.
            let mut damaged = captured_expression.clone();
            let last = damaged.len() - 4;
            damaged[last] ^= 1;
            let damaged = Prepared::new(&p).unwrap().assemble(&damaged).unwrap();
            assert!(verification::fixture_verify(&damaged, &p, &public, &verifier.0, &t).is_err());
            let other = fingerprints
                .iter()
                .find(|other| *other != fingerprint)
                .unwrap();
            let output = verification::fixture_verify(&raw, &p, &public, &verifier.0, &t).unwrap();
            assert!(!verify_signature_status(&output.stdout, other));
            for home in [&worker.0, &verifier.0, &source.0] {
                assert!(!home.join("S.gpg-agent").exists());
            }
        }
    }
}
