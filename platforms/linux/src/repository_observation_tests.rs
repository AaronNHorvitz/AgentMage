// SPDX-License-Identifier: Apache-2.0
//! Synthetic worker observations; real scope and held-root race checks.

use super::*;
use std::os::unix::fs::{PermissionsExt, symlink};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

struct Fixture {
    base: PathBuf,
    scope: LinuxRepositoryScope,
}
impl Fixture {
    fn new() -> Self {
        let base = std::env::temp_dir().join(format!(
            "agentmage-observation-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&base).unwrap();
        let root = base.join("source");
        let git = root.join("chosen metadata");
        let owned = base.join("state");
        for p in [&root, &git, &owned] {
            fs::create_dir(p).unwrap();
            fs::set_permissions(p, fs::Permissions::from_mode(0o700)).unwrap();
        }
        fs::write(root.join(".git"), "unselected metadata\n").unwrap();
        fs::write(git.join("index"), "index sentinel\n").unwrap();
        fs::write(root.join("untracked"), "user sentinel\n").unwrap();
        let scope = LinuxRepositoryScope::verify(root, git, owned).unwrap();
        Self { base, scope }
    }
    fn unchanged(&self) {
        assert_eq!(
            fs::read(self.scope.checkout_root.join("untracked")).unwrap(),
            b"user sentinel\n"
        );
        assert_eq!(
            fs::read(self.scope.git_directory.join("index")).unwrap(),
            b"index sentinel\n"
        );
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.base).unwrap();
    }
}

fn result(code: i32) -> RepositoryInspectionPlatformResult {
    RepositoryInspectionPlatformResult {
        termination: RepositoryInspectionTermination::Exited,
        exit_code: Some(code),
        stdout: b"worker output\n".to_vec(),
        stdout_sha256: hash(b"worker output\n"),
        stdout_bytes: 14,
        stderr_sha256: hash(b""),
        stderr_bytes: 0,
        elapsed_ms: 1,
        descendants_terminated: true,
        platform_code: "linux.git-inspection.exited".into(),
    }
}

#[test]
fn ordinary_observations_bind_selected_metadata_and_bounded_worker_request() {
    let f = Fixture::new();
    for limit in [
        1024,
        MAX_OBSERVATION_BYTES,
        crate::repository_inventory::MAX_INVENTORY_BYTES,
    ] {
        let output = observe_with(
            &f.scope,
            &["ls-files", "--stage", "-z"],
            limit,
            None,
            |r, token| {
                assert!(!token.is_cancelled());
                assert_eq!(r.stdout_limit, limit.min(MAX_OBSERVATION_BYTES));
                assert_eq!(
                    r.workspace.root_path_sha256(),
                    f.scope.repository_path_sha256()
                );
                assert_eq!(
                    &r.arguments[r.arguments.len() - 5..],
                    [
                        "--git-dir=/work/chosen metadata",
                        "--work-tree=/work",
                        "ls-files",
                        "--stage",
                        "-z",
                    ]
                );
                assert!(
                    !r.arguments
                        .iter()
                        .any(|a| a.contains(f.base.to_str().unwrap()))
                );
                assert_eq!(r.environment["GIT_CONFIG_GLOBAL"], "/dev/null");
                assert_eq!(r.environment["GIT_CONFIG_NOSYSTEM"], "1");
                assert_eq!(r.environment["GIT_OPTIONAL_LOCKS"], "0");
                assert_eq!(r.environment["HOME"], "/nonexistent");
                assert_eq!(
                    r.arguments
                        .iter()
                        .filter(|a| a.as_str() == "protocol.allow=never")
                        .count(),
                    1
                );
                assert!(r.arguments.iter().any(|a| a == "core.fsmonitor=false"));
                Ok(result(0))
            },
        )
        .unwrap();
        assert_eq!(output.unwrap(), b"worker output\n");
        f.unchanged();
    }
}

#[test]
fn ordinary_observations_refuse_failed_incomplete_oversize_and_cancelled_workers() {
    let f = Fixture::new();
    for case in 0..11 {
        let observed = observe_with(&f.scope, &["rev-parse", "HEAD"], 1024, None, |_, token| {
            let mut r = result(0);
            match case {
                0 => return Err(failed()),
                1 => r.termination = RepositoryInspectionTermination::TimedOut,
                2 => r.termination = RepositoryInspectionTermination::Cancelled,
                3 => r.termination = RepositoryInspectionTermination::OutputLimit,
                4 => r.descendants_terminated = false,
                5 => {
                    r.stdout = vec![b'x'; 1025];
                    r.stdout_bytes = 1025;
                    r.stdout_sha256 = hash(&r.stdout);
                }
                6 => r.exit_code = Some(2),
                7 => r.exit_code = None,
                8 => r.stdout_sha256 = hash(b"substituted"),
                9 => r.stdout_bytes += 1,
                10 => {
                    use agentmage_kernel_contracts::{
                        CONTRACT_SCHEMA_VERSION, CancellationId, CancellationReason,
                        CancellationSignal,
                    };
                    token
                        .cancel(CancellationSignal {
                            schema_version: CONTRACT_SCHEMA_VERSION,
                            cancellation_id: CancellationId::from_raw("cancel-observation"),
                            correlation_id: CorrelationId::from_raw("repository-observation"),
                            task_id: TaskId::from_raw("repository-observation"),
                            reason: CancellationReason::UserRequested,
                            requested_by: BoundaryKind::Shell,
                        })
                        .unwrap();
                }
                _ => unreachable!(),
            }
            Ok(r)
        });
        assert!(
            observed.is_err(),
            "worker case {case} must refuse, with no ambient fallback"
        );
        f.unchanged();
    }
    for code in [1, 128] {
        assert!(
            observe_with(
                &f.scope,
                &["symbolic-ref", "--quiet", "HEAD"],
                1024,
                None,
                |_, _| Ok(result(code))
            )
            .unwrap()
            .is_none()
        );
    }
}

#[test]
fn ordinary_observations_refuse_invalid_limits_and_precancellation_before_dispatch() {
    let f = Fixture::new();
    for limit in [0, crate::repository_inventory::MAX_INVENTORY_BYTES + 1] {
        assert!(
            observe_with(&f.scope, &["ls-files"], limit, None, |_, _| panic!(
                "invalid bounds dispatched"
            ))
            .is_err()
        );
    }
    let token = CancellationToken::root(
        BoundaryKind::Shell,
        TaskId::from_raw("repository-observation"),
        CorrelationId::from_raw("repository-observation"),
    );
    token
        .cancel(agentmage_kernel_contracts::CancellationSignal {
            schema_version: agentmage_kernel_contracts::CONTRACT_SCHEMA_VERSION,
            cancellation_id: agentmage_kernel_contracts::CancellationId::from_raw("cancel-before"),
            correlation_id: CorrelationId::from_raw("repository-observation"),
            task_id: TaskId::from_raw("repository-observation"),
            reason: agentmage_kernel_contracts::CancellationReason::UserRequested,
            requested_by: BoundaryKind::Shell,
        })
        .unwrap();
    assert!(
        observe_with(&f.scope, &["ls-files"], 1024, Some(&token), |_, _| panic!(
            "pre-cancelled read dispatched"
        ))
        .is_err()
    );
    f.unchanged();
}

#[test]
fn ordinary_observations_refuse_replaced_roots_and_external_metadata_aliases() {
    for during in [false, true] {
        let f = Fixture::new();
        let swap = || {
            fs::rename(&f.scope.checkout_root, f.base.join("saved")).unwrap();
            fs::create_dir(&f.scope.checkout_root).unwrap();
            fs::set_permissions(&f.scope.checkout_root, fs::Permissions::from_mode(0o700)).unwrap();
            fs::create_dir(&f.scope.git_directory).unwrap();
        };
        if !during {
            swap();
        }
        assert!(
            observe_with(&f.scope, &["ls-files"], 1024, None, |_, _| {
                assert!(during, "replaced root dispatched");
                swap();
                Ok(result(0))
            })
            .is_err()
        );
    }
    for during in [false, true] {
        let f = Fixture::new();
        let alias = || {
            let saved = f.base.join("metadata-outside");
            fs::rename(&f.scope.git_directory, &saved).unwrap();
            symlink(&saved, &f.scope.git_directory).unwrap();
        };
        if !during {
            alias();
        }
        assert!(
            observe_with(&f.scope, &["ls-files"], 1024, None, |_, _| {
                assert!(during, "external metadata alias dispatched");
                alias();
                Ok(result(0))
            })
            .is_err()
        );
    }
}

#[test]
fn linked_observations_keep_existing_worker_metadata_selection_and_rechecks() {
    let f = Fixture::new();
    let common = &f.scope.git_directory;
    let git = common.join("worktrees/task");
    let root = f.base.join("task");
    fs::create_dir_all(&git).unwrap();
    fs::create_dir(&root).unwrap();
    fs::write(root.join(".git"), format!("gitdir: {}\n", git.display())).unwrap();
    fs::write(git.join("commondir"), "../..\n").unwrap();
    fs::write(
        git.join("gitdir"),
        format!("{}\n", root.join(".git").display()),
    )
    .unwrap();
    fs::write(git.join("HEAD"), "ref: refs/heads/agentmage/task\n").unwrap();
    let linked = crate::LinuxLinkedRepository::verify(&root, &git, common).unwrap();
    let scope =
        LinuxRepositoryScope::verify_linked_read_only(&linked, &f.scope.owned_root).unwrap();
    assert!(
        observe_with(&scope, &["ls-files", "-z"], 1024, None, |r, _| {
            // Linked worker supplies its own descriptor-bound guest metadata args.
            assert!(
                !r.arguments
                    .iter()
                    .any(|a| a.starts_with("--git-dir=") || a.starts_with("--work-tree="))
            );
            assert_eq!(&r.arguments[r.arguments.len() - 2..], ["ls-files", "-z"]);
            Ok(result(0))
        })
        .unwrap()
        .is_some()
    );
    assert!(
        observe_with(&scope, &["ls-files"], 1024, None, |_, _| {
            fs::write(git.join("commondir"), "../../elsewhere\n").unwrap();
            Ok(result(0))
        })
        .is_err()
    );
    f.unchanged();
}

#[test]
fn supplied_cancellation_reaches_the_worker_and_rejects_concurrent_success() {
    use agentmage_kernel_contracts::{
        CONTRACT_SCHEMA_VERSION, CancellationId, CancellationReason, CancellationSignal,
    };
    let f = Fixture::new();
    let token = CancellationToken::root(
        BoundaryKind::Shell,
        TaskId::from_raw("external-stop"),
        CorrelationId::from_raw("external-stop"),
    );
    let external = token.clone();
    let output = observe_with(
        &f.scope,
        &["rev-parse", "HEAD"],
        1024,
        Some(&token),
        |_, worker_token| {
            assert!(!worker_token.is_cancelled());
            // A different caller thread cancels the original token while its
            // synthetic worker is running. No timer or polling timing assumption.
            std::thread::spawn(move || {
                external
                    .cancel(CancellationSignal {
                        schema_version: CONTRACT_SCHEMA_VERSION,
                        cancellation_id: CancellationId::from_raw("external-stop"),
                        correlation_id: CorrelationId::from_raw("external-stop"),
                        task_id: TaskId::from_raw("external-stop"),
                        reason: CancellationReason::UserRequested,
                        requested_by: BoundaryKind::Shell,
                    })
                    .unwrap()
            })
            .join()
            .unwrap();
            assert!(
                worker_token.is_cancelled(),
                "worker must observe the original scope"
            );
            Ok(result(0))
        },
    );
    assert!(token.is_cancelled());
    assert_eq!(
        output.unwrap_err().kind(),
        LinuxRepositoryErrorKind::ObservationFailed
    );
    f.unchanged();
}
