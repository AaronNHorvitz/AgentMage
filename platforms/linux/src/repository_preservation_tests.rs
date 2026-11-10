// SPDX-License-Identifier: Apache-2.0
//! Real bounded filesystem scans with synthetic Git output, not native admission.

use super::*;
use agentmage_kernel_contracts::{
    BoundaryKind, CONTRACT_SCHEMA_VERSION, CancellationId, CancellationReason, CancellationSignal,
    CorrelationId, TaskId,
};
use std::os::unix::fs::PermissionsExt;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);
struct Fixture {
    root: PathBuf,
    scope: LinuxRepositoryScope,
}
impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "agentmage-preservation-cancel-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        for path in ["repo", "repo/.git", "owned"] {
            fs::create_dir(root.join(path)).unwrap();
            fs::set_permissions(root.join(path), fs::Permissions::from_mode(0o700)).unwrap();
        }
        fs::write(root.join("repo/sentinel"), "source sentinel\n").unwrap();
        fs::write(root.join("repo/.git/index"), "index sentinel\n").unwrap();
        let scope = LinuxRepositoryScope::verify(
            root.join("repo"),
            root.join("repo/.git"),
            root.join("owned"),
        )
        .unwrap();
        Self { root, scope }
    }
    fn unchanged(&self) {
        assert_eq!(
            fs::read(self.root.join("repo/sentinel")).unwrap(),
            b"source sentinel\n"
        );
        assert_eq!(
            fs::read(self.root.join("repo/.git/index")).unwrap(),
            b"index sentinel\n"
        );
        assert_eq!(fs::read_dir(self.root.join("owned")).unwrap().count(), 0);
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.root).unwrap();
    }
}
fn token() -> CancellationToken {
    CancellationToken::root(
        BoundaryKind::Shell,
        TaskId::from_raw("preservation"),
        CorrelationId::from_raw("preservation"),
    )
}
fn cancel(token: &CancellationToken) {
    token
        .cancel(CancellationSignal {
            schema_version: CONTRACT_SCHEMA_VERSION,
            cancellation_id: CancellationId::from_raw("preservation-stop"),
            correlation_id: CorrelationId::from_raw("preservation"),
            task_id: TaskId::from_raw("preservation"),
            reason: CancellationReason::UserRequested,
            requested_by: BoundaryKind::Shell,
        })
        .unwrap();
}
fn reply(args: &[&str]) -> Option<Vec<u8>> {
    Some(match args {
        ["rev-parse", "--show-object-format"] => b"sha1\n".to_vec(),
        ["rev-parse", "--verify", "HEAD"] => format!("{}\n", "a".repeat(40)).into_bytes(),
        ["symbolic-ref", "--quiet", "HEAD"] => b"refs/heads/main\n".to_vec(),
        [
            "for-each-ref",
            "--format=%(upstream)",
            "--count=1",
            "refs/heads/main",
        ] => b"refs/remotes/origin/main\n".to_vec(),
        ["ls-files", ..] => vec![],
        _ => args.join(" ").into_bytes(),
    })
}

#[test]
fn preservation_cancellation_reaches_every_worker_and_stops_after_its_return() {
    let f = Fixture::new();
    let mut baseline = Vec::new();
    collect_with(&f.scope, None, |args, token| {
        assert!(token.is_none());
        baseline.push(args.iter().map(|s| s.to_string()).collect::<Vec<_>>());
        Ok(reply(args))
    })
    .unwrap();
    assert_eq!(baseline.len(), 17);
    for stop in 0..=baseline.len() {
        let token = token();
        if stop == 0 {
            cancel(&token);
        }
        let mut calls = Vec::new();
        let result = collect_with(&f.scope, Some(&token), |args, forwarded| {
            let forwarded = forwarded.expect("original token must reach every worker");
            assert!(!forwarded.is_cancelled());
            calls.push(args.iter().map(|s| s.to_string()).collect::<Vec<_>>());
            if calls.len() == stop {
                // Cancelling the worker's token must cancel the original, too.
                cancel(forwarded);
                assert!(token.is_cancelled());
            }
            Ok(reply(args))
        });
        assert_eq!(
            result.unwrap_err().kind(),
            LinuxRepositoryErrorKind::ObservationFailed,
            "stage {stop}"
        );
        assert_eq!(
            calls,
            baseline[..stop],
            "no later worker after stage {stop}"
        );
        f.unchanged();
    }
}

#[test]
fn preservation_live_token_keeps_complete_manifest_and_optional_ref_semantics() {
    let f = Fixture::new();
    let token = token();
    for missing in [false, true] {
        let make = |forwarded: Option<&CancellationToken>| {
            collect_with(&f.scope, forwarded, |args, _| {
                if missing
                    && matches!(
                        args,
                        ["rev-parse", "--verify", "HEAD"] | ["symbolic-ref", "--quiet", "HEAD"]
                    )
                {
                    Ok(None)
                } else {
                    Ok(reply(args))
                }
            })
            .unwrap()
        };
        let legacy = make(None);
        let cancellable = make(Some(&token));
        assert_eq!(legacy, cancellable);
        legacy.verify().unwrap();
        assert_eq!(legacy.index_sha256, hash(b"index sentinel\n"));
        assert_eq!(legacy.detached, missing);
        assert_eq!(legacy.head_object.is_none(), missing);
        assert_eq!(legacy.upstream.is_none(), missing);
        assert!(!legacy.path_dispositions_sha256.is_empty());
    }
    // Keep content in the preservation binding: neither source nor metadata
    // changes can preserve the digest under either collection entry mode.
    let before = collect_with(&f.scope, Some(&token), |args, _| Ok(reply(args))).unwrap();
    fs::write(f.root.join("repo/sentinel"), "changed source\n").unwrap();
    let changed = collect_with(&f.scope, Some(&token), |args, _| Ok(reply(args))).unwrap();
    assert_ne!(
        before.path_dispositions_sha256,
        changed.path_dispositions_sha256
    );
    assert_ne!(before.manifest_sha256, changed.manifest_sha256);
    fs::write(f.root.join("repo/.git/index"), "changed index\n").unwrap();
    let indexed = collect_with(&f.scope, Some(&token), |args, _| Ok(reply(args))).unwrap();
    assert_ne!(changed.index_sha256, indexed.index_sha256);
    assert_ne!(changed.manifest_sha256, indexed.manifest_sha256);
}

#[test]
fn preservation_cancelled_public_entry_refuses_before_untrusted_artifact_or_filesystem_scan() {
    let f = Fixture::new();
    // Deliberately untrusted synthetic artifact. It cannot pass production
    // revalidation; precancellation must refuse before even that admission.
    let git = LinuxGitArtifact {
        descriptor: open(
            f.root.join("repo/sentinel"),
            OFlags::RDONLY | OFlags::CLOEXEC,
            Mode::empty(),
        )
        .unwrap(),
        launch_path: f.root.join("repo/sentinel"),
        sha256: hash(b"source sentinel\n"),
    };
    let collector = LinuxRepositoryCollector::new(git);
    let token = token();
    cancel(&token);
    fs::remove_file(f.root.join("repo/sentinel")).unwrap();
    assert_eq!(
        collector
            .collect_cancellable(&f.scope, &token)
            .unwrap_err()
            .kind(),
        LinuxRepositoryErrorKind::ObservationFailed
    );
    assert_eq!(
        collector.collect(&f.scope).unwrap_err().kind(),
        LinuxRepositoryErrorKind::InvalidGitArtifact
    );
}

#[test]
fn preservation_root_replacement_during_git_reads_cannot_release_a_manifest() {
    for target in ["repo", "repo/.git"] {
        for stop in [1, 17] {
            let f = Fixture::new();
            let mut calls = 0;
            let result = collect_with(&f.scope, None, |args, _| {
                calls += 1;
                if calls == stop {
                    fs::rename(f.root.join(target), f.root.join("displaced")).unwrap();
                    fs::create_dir(f.root.join(target)).unwrap();
                    fs::set_permissions(f.root.join(target), fs::Permissions::from_mode(0o700))
                        .unwrap();
                    if target == "repo" {
                        fs::create_dir(f.root.join("repo/.git")).unwrap();
                    }
                }
                Ok(reply(args))
            });
            assert_eq!(
                result.unwrap_err().kind(),
                LinuxRepositoryErrorKind::ObservationFailed,
                "{target} at worker {stop}"
            );
            assert_eq!(
                calls, 17,
                "synthetic workers; final held-root check refuses"
            );
            let index = if target == "repo" {
                "displaced/.git/index"
            } else {
                "displaced/index"
            };
            assert_eq!(fs::read(f.root.join(index)).unwrap(), b"index sentinel\n");
            assert_eq!(fs::read_dir(f.root.join("owned")).unwrap().count(), 0);
        }
    }
}
