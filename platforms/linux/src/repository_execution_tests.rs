// SPDX-License-Identifier: Apache-2.0
//! Synthetic worker results and inert launch plans; no native admission claim.

use super::*;
use crate::sandbox::supervision::CapturedStream;
use agentmage_kernel_contracts::{BoundaryKind, CorrelationId, TaskId};
use std::os::unix::fs::{PermissionsExt, symlink};
use std::os::unix::process::ExitStatusExt;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);
struct Fixture {
    base: PathBuf,
    scope: LinuxRepositoryScope,
}
impl Fixture {
    fn new() -> Self {
        let base = std::env::temp_dir().join(format!(
            "agentmage-git-effect-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let root = base.join("checkout");
        let metadata = root.join("selected metadata");
        let owned = base.join("owned");
        for p in [&root, &metadata, &owned] {
            fs::create_dir_all(p).unwrap();
            fs::set_permissions(p, fs::Permissions::from_mode(0o700)).unwrap();
        }
        fs::write(root.join("sentinel"), "user bytes").unwrap();
        Self {
            scope: LinuxRepositoryScope::verify(root, metadata, owned).unwrap(),
            base,
        }
    }
    fn invocation(&self) -> HardenedGitInvocation {
        let h = hash(b"synthetic observation");
        let manifest = RepositoryPreservationManifest::seal(RepositoryPreservationManifest {
            schema_version: 1,
            repository_sha256: h.clone(),
            common_directory_sha256: h.clone(),
            object_format: "sha1".into(),
            safe_ownership: true,
            head_object: Some("a".repeat(40)),
            current_branch: Some("refs/heads/main".into()),
            upstream: None,
            detached: false,
            index_sha256: h.clone(),
            path_dispositions_sha256: h.clone(),
            untracked_count: 0,
            ignored_count: 0,
            local_branches_sha256: h.clone(),
            remote_tracking_refs_sha256: h.clone(),
            tags_sha256: h.clone(),
            notes_sha256: h.clone(),
            stash_sha256: h.clone(),
            replacement_refs_sha256: h.clone(),
            reflogs_sha256: h.clone(),
            agentmage_refs_sha256: h.clone(),
            worktrees_sha256: h.clone(),
            submodules_sha256: h.clone(),
            lfs_sha256: h.clone(),
            object_database_sha256: h.clone(),
            configuration_sha256: h.clone(),
            hooks_sha256: h.clone(),
            content_drivers_sha256: h.clone(),
            remotes_sha256: h.clone(),
            operations_sha256: h,
            shallow: false,
            partial: false,
            hazardous_configuration: false,
            manifest_sha256: "0".repeat(64),
        })
        .unwrap();
        agentmage_kernel_engine::repository_safety::plan_worktree_create(
            "effect-test",
            "effect-test",
            &"a".repeat(40),
            self.scope.repository_path_sha256(),
            &hash(self.scope.worktree_path().as_os_str().as_bytes()),
            &manifest,
        )
        .unwrap()
        .invocations
        .remove(0)
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.base).unwrap();
    }
}
fn token() -> CancellationToken {
    CancellationToken::root(
        BoundaryKind::Shell,
        TaskId::from_raw("effect-test"),
        CorrelationId::from_raw("effect-test"),
    )
}
fn cancel(t: &CancellationToken) {
    use agentmage_kernel_contracts::*;
    t.cancel(CancellationSignal {
        schema_version: CONTRACT_SCHEMA_VERSION,
        cancellation_id: CancellationId::from_raw("cancel-effect"),
        task_id: TaskId::from_raw("effect-test"),
        correlation_id: CorrelationId::from_raw("effect-test"),
        reason: CancellationReason::UserRequested,
        requested_by: BoundaryKind::Shell,
    })
    .unwrap();
}
fn result(outcome: OperationOutcome, exit: Option<i32>) -> SupervisedResult {
    let stream = || CapturedStream {
        retained: vec![],
        total: 0,
        sha256: Sha256::digest([]).into(),
    };
    SupervisedResult {
        outcome: Ok(outcome),
        status: exit.map(|c| ExitStatus::from_raw(c << 8)),
        stdout: stream(),
        stderr: stream(),
        resources: None,
        output_complete: true,
    }
}
fn inert_artifact() -> LinuxGitArtifact {
    let path = Path::new("/usr/bin/git");
    LinuxGitArtifact {
        descriptor: open(path, OFlags::RDONLY | OFlags::CLOEXEC, Mode::empty()).unwrap(),
        launch_path: path.to_owned(),
        sha256: "0".repeat(64),
    }
}
fn inert_manifest() -> Arc<LinuxRepositoryInspectionManifest> {
    Arc::new(LinuxRepositoryInspectionManifest {
        systemd_run: inert_artifact(),
        systemctl: inert_artifact(),
        bubblewrap: inert_artifact(),
        git: inert_artifact(),
    })
}

#[test]
fn worktree_effect_worker_projects_only_exact_metadata_and_owned_roots_writable() {
    let f = Fixture::new();
    let invocation = f.invocation();
    let owner = LinuxRepositoryMutationScope::capture(&f.scope).unwrap();
    let filter = crate::sandbox::supervision::FilterPipe::new().unwrap();
    let command = worker_command(
        &inert_manifest(),
        &owner,
        &map_arguments(&invocation, &f.scope).unwrap(),
        &invocation,
        "agentmage-repository-git-0123456789abcdef01234567.service",
        &filter,
    );
    let args: Vec<_> = command.get_args().map(|a| a.to_str().unwrap()).collect();
    assert!(args.contains(&"--unshare-all") && args.contains(&"--disable-userns"));
    assert!(args.windows(2).any(|p| p == ["--seccomp", "4"]));
    assert!(
        args.windows(3)
            .any(|p| p == ["--ro-bind-fd", "3", GUEST_GIT_EXECUTABLE])
    );
    assert!(
        args.windows(3)
            .any(|p| p == ["--symlink", GUEST_GIT_EXECUTABLE, "/usr/bin/git"])
    );
    assert_eq!(
        args.windows(2)
            .filter(|p| *p == ["--ro-bind-fd", "3"])
            .count(),
        1
    );
    assert!(
        !args
            .windows(2)
            .any(|p| matches!(p[0], "--bind" | "--ro-bind") && p[1] == "/usr/bin")
    );
    assert!(
        args.windows(3)
            .any(|p| p == ["--ro-bind-fd", "5", f.scope.checkout_root.to_str().unwrap()])
    );
    assert!(
        args.windows(3)
            .any(|p| p == ["--bind-fd", "6", f.scope.git_directory.to_str().unwrap()])
    );
    assert!(
        args.windows(3)
            .any(|p| p == ["--bind-fd", "7", f.scope.owned_root.to_str().unwrap()])
    );
    assert_eq!(args.iter().filter(|a| **a == "--bind-fd").count(), 2);
    assert!(!args.contains(&"--bind"));
    assert!(args.contains(&format!("--git-dir={}", f.scope.git_directory.display()).as_str()));
    assert!(args.contains(&f.scope.worktree_path().to_str().unwrap()));
    assert!(args.contains(&"--no-replace-objects"));
    let properties: Vec<_> = args
        .iter()
        .filter(|a| a.starts_with("--property=OpenFile="))
        .collect();
    assert_eq!(properties.len(), 5);
    for (property, name) in properties
        .iter()
        .zip(["git", "seccomp", "checkout", "metadata", "owned"])
    {
        assert!(property.ends_with(&format!(":{name}:read-only")));
    }
    assert!(args.contains(&"--property=KillMode=control-group"));
    assert!(args.contains(&"--property=RuntimeMaxSec=31s"));
    assert_eq!(command.get_envs().count(), 2); // no ambient Git environment
    for path in [
        "/",
        "/usr",
        "/usr/local/repo",
        "/app/data",
        "/run/repo",
        "/etc",
        "/proc/self",
        "/tmp/../outside",
    ] {
        assert!(!admissible_projection_path(Path::new(path)), "{path}");
    }
    assert!(admissible_projection_path(&f.scope.checkout_root));
}

#[test]
fn worktree_effect_refuses_incomplete_failed_cancelled_and_oversized_workers() {
    let f = Fixture::new();
    let invocation = f.invocation();
    for (outcome, exit, expected) in [
        (
            OperationOutcome::Succeeded,
            Some(0),
            InvocationStatus::Succeeded,
        ),
        (
            OperationOutcome::Succeeded,
            Some(1),
            InvocationStatus::Failed,
        ),
        (OperationOutcome::Succeeded, None, InvocationStatus::Failed),
        (OperationOutcome::Failed, Some(0), InvocationStatus::Failed),
        (
            OperationOutcome::Cancelled,
            None,
            InvocationStatus::Cancelled,
        ),
        (OperationOutcome::TimedOut, None, InvocationStatus::TimedOut),
    ] {
        assert_eq!(
            execute_with(&f.scope, &invocation, &token(), |_, _| Ok(result(
                outcome, exit
            ))),
            expected
        );
    }
    for incomplete in [true, false] {
        assert_eq!(
            execute_with(&f.scope, &invocation, &token(), |_, _| {
                let mut r = result(OperationOutcome::Succeeded, Some(0));
                if incomplete {
                    r.output_complete = false;
                } else {
                    r.stderr.total = MAX_OBSERVATION_BYTES + 1;
                }
                Ok(r)
            }),
            if incomplete {
                InvocationStatus::CleanupFailed
            } else {
                InvocationStatus::OutputLimit
            }
        );
    }
    let t = token();
    assert_eq!(
        execute_with(&f.scope, &invocation, &t, |_, _| {
            cancel(&t);
            Ok(result(OperationOutcome::Succeeded, Some(0)))
        }),
        InvocationStatus::Cancelled
    );
    for failure in [
        InvocationStatus::Failed,
        InvocationStatus::CleanupFailed,
        InvocationStatus::OutputLimit,
    ] {
        assert_eq!(
            execute_with(&f.scope, &invocation, &token(), |_, _| Err(failure)),
            failure
        );
    }
    assert_eq!(
        fs::read(f.scope.checkout_root.join("sentinel")).unwrap(),
        b"user bytes"
    );
}

#[test]
fn worktree_effect_refuses_stale_roots_and_aliases_before_or_after_dispatch() {
    for index in 0..3 {
        for during in [false, true] {
            for alias in [false, true] {
                let f = Fixture::new();
                let path = [
                    &f.scope.checkout_root,
                    &f.scope.git_directory,
                    &f.scope.owned_root,
                ][index];
                let swap = || {
                    let saved = f.base.join("saved");
                    fs::rename(path, &saved).unwrap();
                    if alias {
                        symlink(&saved, path).unwrap();
                    } else {
                        fs::create_dir(path).unwrap();
                        fs::set_permissions(path, fs::Permissions::from_mode(0o700)).unwrap();
                    }
                };
                // Capture the held projection first; replacing metadata/owned roots
                // during execution must fail even when their path spelling is unchanged.
                if !during {
                    swap();
                }
                let expected = if during {
                    InvocationStatus::CleanupFailed
                } else {
                    InvocationStatus::Denied
                };
                if !during && index != 0 && !alias {
                    // The original scope pins only the checkout identity. All
                    // three directory identities are held at effect capture.
                    let owner = LinuxRepositoryMutationScope::capture(&f.scope).unwrap();
                    let replacement = f.base.join("replacement");
                    fs::rename(path, replacement).unwrap();
                    fs::create_dir(path).unwrap();
                    fs::set_permissions(path, fs::Permissions::from_mode(0o700)).unwrap();
                    assert!(owner.revalidate().is_err());
                    continue;
                }
                assert_eq!(
                    execute_with(&f.scope, &f.invocation(), &token(), |_, _| {
                        assert!(during, "stale scope dispatched");
                        swap();
                        Ok(result(OperationOutcome::Succeeded, Some(0)))
                    }),
                    expected,
                    "index={index} during={during} alias={alias}"
                );
            }
        }
    }
}

#[test]
fn worktree_effect_refuses_malformed_network_linked_and_precancelled_requests() {
    let f = Fixture::new();
    let good = f.invocation();
    let t = token();
    cancel(&t);
    assert_eq!(
        execute_with(&f.scope, &good, &t, |_, _| panic!(
            "precancelled effect dispatched"
        )),
        InvocationStatus::Cancelled
    );
    for network in [false, true] {
        let mut bad = good.clone();
        if network {
            bad.network = true;
        } else {
            bad.arguments.push("push".into());
        }
        assert_eq!(
            execute_with(&f.scope, &bad, &token(), |_, _| panic!(
                "malformed effect dispatched"
            )),
            InvocationStatus::Denied
        );
    }
    let mut bad_scope = f.scope.clone();
    bad_scope.git_directory = bad_scope.checkout_root.clone();
    assert_eq!(
        execute_with(&bad_scope, &good, &token(), |_, _| panic!(
            "broad metadata dispatched"
        )),
        InvocationStatus::Denied
    );
    let owner = LinuxRepositoryMutationScope::capture(&f.scope).unwrap();
    let linked_root = f.base.join("linked");
    let common = f.base.join("original/.git");
    let metadata = common.join("worktrees/linked");
    for p in [&linked_root, &metadata] {
        fs::create_dir_all(p).unwrap();
        fs::set_permissions(p, fs::Permissions::from_mode(0o700)).unwrap();
    }
    fs::write(
        linked_root.join(".git"),
        format!("gitdir: {}\n", metadata.display()),
    )
    .unwrap();
    fs::write(metadata.join("commondir"), "../..\n").unwrap();
    fs::write(
        metadata.join("gitdir"),
        format!("{}\n", linked_root.join(".git").display()),
    )
    .unwrap();
    fs::write(metadata.join("HEAD"), "ref: refs/heads/agentmage/task\n").unwrap();
    let linked = crate::LinuxLinkedRepository::verify(&linked_root, &metadata, &common).unwrap();
    let linked_scope =
        LinuxRepositoryScope::verify_linked_read_only(&linked, &f.scope.owned_root).unwrap();
    assert_eq!(
        execute_with(&linked_scope, &good, &token(), |_, _| panic!(
            "linked effect dispatched"
        )),
        InvocationStatus::Denied
    );
    assert_eq!(owner.projections().unwrap().len(), 3);
}

#[test]
fn worktree_effect_supervisor_requires_closed_owner_nonce_cardinality_and_filter() {
    let f = Fixture::new();
    let owner = Arc::new(LinuxRepositoryMutationScope::capture(&f.scope).unwrap());
    let cancelled = crate::sandbox::LinuxSandboxCancellation::default();
    cancelled.cancel();
    for (unit, count, with_filter, accepted) in [
        (
            "agentmage-repository-git-0123456789abcdef01234567.service",
            3,
            true,
            true,
        ),
        (
            "agentmage-repository-git-0123456789abcdef01234567.service",
            2,
            true,
            false,
        ),
        (
            "agentmage-repository-git-0123456789abcdef01234567.service",
            3,
            false,
            false,
        ),
        (
            "agentmage-git-inspection-0123456789abcdef01234567.service",
            3,
            true,
            false,
        ),
        ("agentmage-repository-git-*.service", 3, true, false),
    ] {
        let mut projections = owner.projections().unwrap();
        projections.truncate(count);
        let r = crate::sandbox::supervision::run_owned(
            Command::new("/nonexistent/must-never-launch"),
            Launch {
                owner: LaunchOwner::RepositoryGit(inert_manifest(), owner.clone()),
                projections,
                unit: unit.into(),
                deadline: Instant::now() + Duration::from_secs(1),
                stdout_limit: 1024,
                stderr_limit: 1024,
                filter_pipe: with_filter
                    .then(|| crate::sandbox::supervision::FilterPipe::new().unwrap()),
            },
            &[0; 8],
            &crate::sandbox::SandboxCancellation::Local(&cancelled),
        );
        if accepted {
            assert!(matches!(
                r.unwrap().outcome,
                Ok(OperationOutcome::Cancelled)
            ));
        } else {
            assert!(r.is_err());
        }
    }
}
