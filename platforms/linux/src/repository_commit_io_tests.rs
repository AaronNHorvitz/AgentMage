// SPDX-License-Identifier: Apache-2.0
//! Real held roots and synthetic supervisor results, not native signer evidence.

use super::*;
use crate::local_commit::tests::synthetic_plan;
use crate::sandbox::supervision::CapturedStream;
use agentmage_kernel_contracts::{BoundaryKind, CorrelationId, TaskId};
use std::os::unix::fs::{PermissionsExt, symlink};
use std::os::unix::process::ExitStatusExt;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);
struct Fixture {
    root: PathBuf,
    scope: LinuxRepositoryScope,
    plan: LocalCommitPlan,
}
impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "agentmage-commit-io-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed),
        ));
        let checkout = root.join("checkout");
        let metadata = checkout.join("selected metadata");
        let owned = root.join("owned");
        for path in [&checkout, &metadata, &owned] {
            fs::create_dir_all(path).unwrap();
            fs::set_permissions(path, fs::Permissions::from_mode(0o700)).unwrap();
        }
        let scope = LinuxRepositoryScope::verify(checkout, metadata, owned).unwrap();
        let plan = synthetic_plan(&scope.repository_path_sha256);
        Self { root, scope, plan }
    }
    fn session(&self) -> CommitIoSession {
        CommitIoSession::new(&self.scope, &self.plan).unwrap()
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
        TaskId::from_raw("commit-io"),
        CorrelationId::from_raw("commit-io"),
    )
}
fn cancel(t: &CancellationToken) {
    use agentmage_kernel_contracts::*;
    t.cancel(CancellationSignal {
        schema_version: CONTRACT_SCHEMA_VERSION,
        cancellation_id: CancellationId::from_raw("cancel-commit-io"),
        task_id: TaskId::from_raw("commit-io"),
        correlation_id: CorrelationId::from_raw("commit-io"),
        reason: CancellationReason::UserRequested,
        requested_by: BoundaryKind::Shell,
    })
    .unwrap();
}
fn success(bytes: &[u8]) -> SupervisedResult {
    let stream = |bytes: &[u8]| CapturedStream {
        retained: bytes.to_vec(),
        total: bytes.len(),
        sha256: Sha256::digest(bytes).into(),
    };
    SupervisedResult {
        outcome: Ok(OperationOutcome::Succeeded),
        status: Some(ExitStatus::from_raw(0)),
        stdout: stream(bytes),
        stderr: stream(&[]),
        resources: None,
        output_complete: true,
    }
}
fn no_dispatch(
    _: Arc<LinuxRepositoryMutationScope>,
    _: &[String],
    _: &std::collections::BTreeMap<String, String>,
    _: Instant,
) -> Result<SupervisedResult, InvocationStatus> {
    panic!("must refuse before dispatch")
}
fn branch(session: &mut CommitIoSession) {
    let parent = session.plan.parent_object.clone();
    session
        .step_with(Step::Branch, &token(), |_, _, _, _| {
            Ok(success(format!("{parent}\n").as_bytes()))
        })
        .unwrap();
}

#[test]
fn commit_io_worker_binds_exact_order_ref_objects_roots_environment_and_total_deadline() {
    let f = Fixture::new();
    let mut s = f.session();
    let deadline = s.deadline();
    let object = "d".repeat(40);
    let operations = [
        (
            Step::Branch,
            vec!["rev-parse", "--verify", f.plan.task_branch.as_str()],
            format!("{}\n", f.plan.parent_object).into_bytes(),
        ),
        (
            Step::Commit(&object),
            vec!["cat-file", "commit", object.as_str()],
            b"raw signed commit".to_vec(),
        ),
        (
            Step::Update,
            vec![
                "update-ref",
                "--no-deref",
                f.plan.task_branch.as_str(),
                object.as_str(),
                f.plan.parent_object.as_str(),
            ],
            vec![],
        ),
    ];
    for (step, expected, output) in operations {
        let got = s
            .step_with(step, &token(), |owner, args, env, observed_deadline| {
                assert_eq!(observed_deadline, deadline);
                owner.revalidate().unwrap();
                assert_eq!(owner.scope.checkout_root, f.scope.checkout_root);
                assert_eq!(owner.scope.git_directory, f.scope.git_directory);
                assert_eq!(owner.scope.owned_root, f.scope.owned_root);
                assert_eq!(args[args.len() - expected.len()..], expected);
                assert!(args.contains(&"--no-replace-objects".into()));
                assert!(args.contains(&"protocol.allow=never".into()));
                assert!(args.contains(&format!("--git-dir={}", f.scope.git_directory.display())));
                assert_eq!(env, &candidate::candidate_environment());
                assert!(!env.contains_key("GNUPGHOME") && !env.contains_key("GIT_INDEX_FILE"));
                Ok(success(&output))
            })
            .unwrap();
        assert_eq!(got, output);
    }
    assert!(s.step_with(Step::Update, &token(), no_dispatch).is_err());
}

#[test]
fn commit_io_worker_refuses_unsealed_foreign_network_and_unsafe_mutation_scopes() {
    let f = Fixture::new();
    for change in [0, 1, 2] {
        let mut plan = f.plan.clone();
        match change {
            0 => plan.network_authority = true,
            1 => plan.repository_sha256 = hash(b"foreign repository"),
            _ => plan.task_branch = "refs/heads/main".into(),
        }
        plan.plan_sha256 = "0".repeat(64);
        plan.plan_sha256 = hash(&serde_json::to_vec(&plan).unwrap());
        assert!(CommitIoSession::new(&f.scope, &plan).is_err());
    }
    let mut plan = f.plan.clone();
    plan.candidate_tree = "e".repeat(40);
    assert!(CommitIoSession::new(&f.scope, &plan).is_err());
    // An aliased management root cannot become a mutation owner.
    let mut scope = f.scope.clone();
    scope.owned_root = scope.checkout_root.clone();
    assert!(CommitIoSession::new(&scope, &f.plan).is_err());
}

#[test]
fn commit_io_worker_supports_sha256_and_refuses_empty_commit_or_nonempty_update_output() {
    let f = Fixture::new();
    let mut plan = f.plan.clone();
    plan.parent_object = "b".repeat(64);
    plan.candidate_tree = "c".repeat(64);
    plan.plan_sha256 = "0".repeat(64);
    plan.plan_sha256 = hash(&serde_json::to_vec(&plan).unwrap());
    let object = "d".repeat(64);
    let mut s = CommitIoSession::new(&f.scope, &plan).unwrap();
    branch(&mut s);
    assert!(
        s.step_with(Step::Commit(&object), &token(), |_, _, _, _| Ok(success(
            &[]
        )))
        .is_err()
    );
    assert!(s.step_with(Step::Update, &token(), no_dispatch).is_err());

    let mut s = CommitIoSession::new(&f.scope, &plan).unwrap();
    branch(&mut s);
    s.step_with(Step::Commit(&object), &token(), |_, args, _, _| {
        assert_eq!(args.last(), Some(&object));
        Ok(success(b"raw commit"))
    })
    .unwrap();
    assert!(
        s.step_with(Step::Update, &token(), |_, args, _, _| {
            assert_eq!(
                &args[args.len() - 2..],
                &[object, plan.parent_object.clone()]
            );
            Ok(success(b"unexpected output"))
        })
        .is_err()
    );
    assert!(s.step_with(Step::Update, &token(), no_dispatch).is_err());
}

#[test]
fn commit_io_worker_refuses_wrong_order_malformed_object_and_stale_branch_without_retry() {
    let f = Fixture::new();
    for step in [Step::Update, Step::Commit(&"d".repeat(40))] {
        let mut s = f.session();
        assert!(s.step_with(step, &token(), no_dispatch).is_err());
        assert!(s.step_with(Step::Branch, &token(), no_dispatch).is_err());
    }
    for object in [
        "0".repeat(40),
        "d".repeat(64),
        "D".repeat(40),
        "--help".into(),
        format!("{}\n", "d".repeat(40)),
    ] {
        let mut s = f.session();
        branch(&mut s);
        assert!(
            s.step_with(Step::Commit(&object), &token(), no_dispatch)
                .is_err()
        );
        assert!(
            s.step_with(Step::Commit(&"d".repeat(40)), &token(), no_dispatch)
                .is_err()
        );
    }
    for bytes in [
        format!("{}\n", "e".repeat(40)),
        format!("{}\n\n", f.plan.parent_object),
        f.plan.parent_object.clone(),
    ] {
        let mut s = f.session();
        assert!(
            s.step_with(Step::Branch, &token(), |_, _, _, _| Ok(success(
                bytes.as_bytes()
            )))
            .is_err()
        );
        assert!(s.step_with(Step::Branch, &token(), no_dispatch).is_err());
    }
}

#[test]
fn commit_io_worker_refuses_incomplete_failed_oversized_and_cancelled_results() {
    let f = Fixture::new();
    for case in 0..10 {
        let mut s = f.session();
        let t = token();
        let mut r = success(format!("{}\n", f.plan.parent_object).as_bytes());
        match case {
            0 => r.output_complete = false,
            1 => r.status = Some(ExitStatus::from_raw(1 << 8)),
            2 => r.outcome = Ok(OperationOutcome::Failed),
            3 => r.status = None,
            4 => r.stdout.total = MAX_OBSERVATION_BYTES + 1,
            5 => r.stderr.total = MAX_OBSERVATION_BYTES + 1,
            6 => r.stdout.total += 1,
            7 => r.stderr.total += 1,
            8 => r.outcome = Ok(OperationOutcome::TimedOut),
            _ => (),
        }
        assert!(
            s.step_with(Step::Branch, &t, |_, _, _, _| {
                if case == 9 {
                    cancel(&t);
                }
                Ok(r)
            })
            .is_err(),
            "case {case}"
        );
        assert!(s.step_with(Step::Branch, &token(), no_dispatch).is_err());
    }
    let mut s = f.session();
    assert!(
        s.step_with(Step::Branch, &token(), |_, _, _, _| Err(
            InvocationStatus::CleanupFailed
        ))
        .is_err()
    );
    assert!(s.step_with(Step::Branch, &token(), no_dispatch).is_err());
}

#[test]
fn commit_io_worker_refuses_changed_roots_aliases_and_expired_or_precancelled_sessions() {
    for during in [false, true] {
        let f = Fixture::new();
        let mut s = f.session();
        let swap = || {
            fs::rename(&f.scope.owned_root, f.root.join("retained")).unwrap();
            fs::create_dir(&f.scope.owned_root).unwrap();
            fs::set_permissions(&f.scope.owned_root, fs::Permissions::from_mode(0o700)).unwrap();
            fs::write(f.scope.owned_root.join("sentinel"), "retained replacement").unwrap();
        };
        if !during {
            swap();
        }
        assert!(
            s.step_with(Step::Branch, &token(), |_, _, _, _| {
                assert!(during);
                swap();
                Ok(success(format!("{}\n", f.plan.parent_object).as_bytes()))
            })
            .is_err()
        );
        assert!(s.step_with(Step::Branch, &token(), no_dispatch).is_err());
        assert_eq!(
            fs::read(f.scope.owned_root.join("sentinel")).unwrap(),
            b"retained replacement"
        );
    }
    let f = Fixture::new();
    let mut s = f.session();
    s.deadline = Instant::now();
    assert!(s.step_with(Step::Branch, &token(), no_dispatch).is_err());
    let mut s = f.session();
    let t = token();
    cancel(&t);
    assert!(s.step_with(Step::Branch, &t, no_dispatch).is_err());
    let mut s = f.session();
    fs::rename(
        &f.scope.git_directory,
        f.scope.checkout_root.join("retained metadata"),
    )
    .unwrap();
    symlink(
        f.scope.checkout_root.join("retained metadata"),
        &f.scope.git_directory,
    )
    .unwrap();
    assert!(s.step_with(Step::Branch, &token(), no_dispatch).is_err());
}
