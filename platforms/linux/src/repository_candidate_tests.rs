// SPDX-License-Identifier: Apache-2.0
//! Synthetic candidate worker tests; no positive native admission claim.
use super::*;
use crate::sandbox::supervision::CapturedStream;
use agentmage_kernel_contracts::{BoundaryKind, CorrelationId, TaskId};
use agentmage_kernel_engine::local_commit::CandidateTreeFilePlan;
use std::os::unix::fs::{PermissionsExt, symlink};
use std::os::unix::process::ExitStatusExt;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);
struct Fixture {
    base: PathBuf,
    scope: LinuxRepositoryScope,
    plan: CandidateTreePlan,
}
impl Fixture {
    fn new(format: usize) -> Self {
        let base = std::env::temp_dir().join(format!(
            "candidate-confined-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let root = base.join("checkout");
        let git = root.join("selected metadata");
        let owned = base.join("owned");
        for p in [&root, &git, &owned] {
            fs::create_dir_all(p).unwrap();
            fs::set_permissions(p, fs::Permissions::from_mode(0o700)).unwrap();
        }
        fs::write(root.join("source"), b"unchanged source").unwrap();
        fs::write(git.join("index"), b"unchanged index").unwrap();
        let scope = LinuxRepositoryScope::verify(root, git, owned).unwrap();
        let h = hash(b"synthetic evidence");
        let mut plan = CandidateTreePlan {
            schema_version: 1,
            candidate_build_id: "candidate-fixture".into(),
            repository_sha256: scope.repository_path_sha256.clone(),
            review_packet_sha256: h.clone(),
            change_set_sha256: h.clone(),
            commit_group_sha256: h.clone(),
            parent_object: "a".repeat(format),
            temporary_index_path_sha256: hash(
                index_path(&scope, "candidate-fixture")
                    .unwrap()
                    .as_os_str()
                    .as_bytes(),
            ),
            user_index_sha256: h.clone(),
            preservation_manifest_sha256: h,
            files: vec![
                CandidateTreeFilePlan {
                    operation_id: "one".into(),
                    path: "src/a, space.rs".into(),
                    postimage_sha256: hash(b"first\n"),
                    mode: CommitFileMode::Regular,
                },
                CandidateTreeFilePlan {
                    operation_id: "two".into(),
                    path: "src/b.rs".into(),
                    postimage_sha256: hash(b"second\n"),
                    mode: CommitFileMode::Executable,
                },
            ],
            hooks_enabled: false,
            filters_enabled: false,
            repository_program_selection_enabled: false,
            network_authority: false,
            plan_sha256: "0".repeat(64),
        };
        reseal(&mut plan);
        Self { base, scope, plan }
    }
    fn session(&self) -> CandidateSession {
        CandidateSession::new(&self.scope, &self.plan).unwrap()
    }
    fn unchanged(&self) {
        assert_eq!(
            fs::read(self.scope.checkout_root.join("source")).unwrap(),
            b"unchanged source"
        );
        assert_eq!(
            fs::read(self.scope.git_directory.join("index")).unwrap(),
            b"unchanged index"
        );
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.base).unwrap();
    }
}
fn reseal(plan: &mut CandidateTreePlan) {
    plan.plan_sha256 = "0".repeat(64);
    plan.plan_sha256 = hash(&serde_json::to_vec(plan).unwrap());
}
fn token() -> CancellationToken {
    CancellationToken::root(
        BoundaryKind::Shell,
        TaskId::from_raw("candidate-test"),
        CorrelationId::from_raw("candidate-test"),
    )
}
fn result(bytes: &[u8]) -> SupervisedResult {
    let capture = |data: &[u8]| CapturedStream {
        retained: data.to_vec(),
        total: data.len(),
        sha256: Sha256::digest(data).into(),
    };
    SupervisedResult {
        outcome: Ok(OperationOutcome::Succeeded),
        status: Some(ExitStatus::from_raw(0)),
        stdout: capture(bytes),
        stderr: capture(&[]),
        resources: None,
        output_complete: true,
    }
}
fn initialize(s: &mut CandidateSession) {
    s.step_with(Step::ReadTree, &token(), |_, _, _, _, _| Ok(result(&[])))
        .unwrap();
}
fn no_dispatch(
    _: Arc<LinuxRepositoryMutationScope>,
    _: &[String],
    _: &BTreeMap<String, String>,
    _: Option<OwnedFd>,
    _: Instant,
) -> Result<SupervisedResult, InvocationStatus> {
    panic!("dispatch must refuse");
}

#[test]
fn candidate_setup_rolls_back_empty_claim_after_capture_failure_and_allows_fresh_attempt() {
    let f = Fixture::new(40);
    for failure in ["before-verify", "after-verify"] {
        assert!(
            CandidateSession::new_with(&f.scope, &f.plan, |scratch| {
                assert!(scratch.is_dir());
                assert_eq!(fs::metadata(scratch).unwrap().mode() & 0o777, 0o700);
                if failure == "after-verify" {
                    LinuxRepositoryScope::verify(
                        &f.scope.checkout_root,
                        &f.scope.git_directory,
                        scratch,
                    )?;
                }
                Err(denied())
            })
            .is_err()
        );
        assert!(fs::read_dir(&f.scope.owned_root).unwrap().next().is_none());
        let mut session = f.session();
        assert!(session.cleanup());
        f.unchanged();
    }
}

#[test]
fn candidate_setup_refuses_replacement_between_claim_and_worker_capture_without_removing_it() {
    for replacement in ["directory", "symlink", "parent"] {
        let f = Fixture::new(40);
        let scratch = index_path(&f.scope, &f.plan.candidate_build_id)
            .unwrap()
            .parent()
            .unwrap()
            .to_owned();
        let target = if replacement == "parent" {
            f.scope.owned_root.clone()
        } else {
            scratch.clone()
        };
        let retained = target.with_extension("retained");
        assert!(
            CandidateSession::new_with(&f.scope, &f.plan, |path| {
                fs::rename(&target, &retained).unwrap();
                if replacement == "symlink" {
                    symlink(&retained, &target).unwrap();
                } else {
                    fs::create_dir(&target).unwrap();
                    fs::set_permissions(&target, fs::Permissions::from_mode(0o700)).unwrap();
                    if replacement == "parent" {
                        fs::create_dir(path).unwrap();
                        fs::set_permissions(path, fs::Permissions::from_mode(0o700)).unwrap();
                    }
                }
                // Leave the replacement EMPTY: rollback must refuse by identity,
                // not merely because a sentinel makes rmdir fail.
                let worker_scope = LinuxRepositoryScope::verify(
                    &f.scope.checkout_root,
                    &f.scope.git_directory,
                    path,
                )?;
                LinuxRepositoryMutationScope::capture(&worker_scope)
            })
            .is_err(),
            "{replacement}"
        );
        assert!(retained.is_dir());
        assert!(target.symlink_metadata().is_ok());
        assert!(scratch.symlink_metadata().is_ok());
        assert!(CandidateSession::new(&f.scope, &f.plan).is_err());
        f.unchanged();
    }
}

#[test]
fn candidate_setup_retains_nonempty_or_changed_mode_claims_after_failure() {
    for entry in ["file", "directory", "symlink", "mode", "mode-after-capture"] {
        let f = Fixture::new(40);
        let mut scratch = PathBuf::new();
        assert!(
            CandidateSession::new_with(&f.scope, &f.plan, |path| {
                scratch = path.to_owned();
                match entry {
                    "file" => fs::write(path.join("unknown"), b"retain").unwrap(),
                    "directory" => fs::create_dir(path.join("unknown")).unwrap(),
                    "symlink" => {
                        symlink(f.scope.git_directory.join("index"), path.join("unknown")).unwrap()
                    }
                    _ => fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap(),
                }
                if entry == "mode-after-capture" {
                    let worker_scope = LinuxRepositoryScope::verify(
                        &f.scope.checkout_root,
                        &f.scope.git_directory,
                        path,
                    )?;
                    return LinuxRepositoryMutationScope::capture(&worker_scope);
                }
                Err(denied())
            })
            .is_err()
        );
        assert!(scratch.is_dir());
        if !entry.starts_with("mode") {
            assert!(scratch.join("unknown").symlink_metadata().is_ok());
        }
        assert!(CandidateSession::new(&f.scope, &f.plan).is_err());
        f.unchanged();
    }
}

#[test]
fn candidate_setup_rejects_foreign_captured_descriptor_and_rolls_back_only_its_empty_claim() {
    let f = Fixture::new(40);
    let other = Fixture::new(40);
    fs::write(other.scope.owned_root.join("sentinel"), b"preserve").unwrap();
    assert!(
        CandidateSession::new_with(&f.scope, &f.plan, |_| {
            LinuxRepositoryMutationScope::capture(&other.scope)
        })
        .is_err()
    );
    assert!(fs::read_dir(&f.scope.owned_root).unwrap().next().is_none());
    assert_eq!(
        fs::read(other.scope.owned_root.join("sentinel")).unwrap(),
        b"preserve"
    );
    f.unchanged();
    other.unchanged();
}

#[test]
fn candidate_worker_builds_exact_ordered_plan_with_sealed_postimages_and_private_index() {
    for format in [40, 64] {
        let f = Fixture::new(format);
        let mut s = f.session();
        let scratch = s.worker.scope.owned_root.clone();
        assert_eq!(fs::metadata(&scratch).unwrap().mode() & 0o777, 0o700);
        s.step_with(
            Step::ReadTree,
            &token(),
            |owner, args, env, input, deadline| {
                assert!(args.ends_with(&["read-tree".into(), f.plan.parent_object.clone()]));
                assert!(input.is_none());
                assert_eq!(owner.scope.owned_root, scratch);
                assert_eq!(
                    env["GIT_INDEX_FILE"],
                    scratch.join("index").to_str().unwrap()
                );
                assert!(deadline <= Instant::now() + PROCESS_TIMEOUT);
                assert_eq!(env.len(), 11);
                assert_eq!(env["HOME"], "/nonexistent");
                assert!(args.contains(&"protocol.allow=never".into()));
                Ok(result(&[]))
            },
        )
        .unwrap();
        for (operation, bytes, mode, path) in [
            ("one", b"first\n".as_slice(), "100644", "src/a, space.rs"),
            ("two", b"second\n".as_slice(), "100755", "src/b.rs"),
        ] {
            let object = crate::linux_git_blob_object_id(
                if format == 40 { "sha1" } else { "sha256" },
                bytes,
            )
            .unwrap();
            let observed = s
                .step_with(
                    Step::Hash { operation, bytes },
                    &token(),
                    |_, args, _, input, _| {
                        assert!(args.ends_with(&[
                            "hash-object".into(),
                            "-w".into(),
                            "--no-filters".into(),
                            "--stdin".into()
                        ]));
                        let input = input.unwrap();
                        let mut copy = vec![0; bytes.len()];
                        assert_eq!(pread(&input, &mut copy, 0).unwrap(), bytes.len());
                        assert_eq!(copy, bytes);
                        use rustix::fs::{SealFlags, fcntl_get_seals};
                        assert_eq!(
                            fcntl_get_seals(&input).unwrap(),
                            SealFlags::SEAL
                                | SealFlags::SHRINK
                                | SealFlags::GROW
                                | SealFlags::WRITE
                        );
                        assert!(rustix::io::pwrite(&input, b"x", 0).is_err());
                        assert_eq!(
                            rustix::fs::seek(&input, rustix::fs::SeekFrom::Current(0)).unwrap(),
                            0
                        );
                        Ok(result(format!("{object}\n").as_bytes()))
                    },
                )
                .unwrap();
            assert_eq!(observed, object.as_bytes());
            s.step_with(Step::Add, &token(), |_, args, _, input, _| {
                assert!(args.ends_with(&[
                    "update-index".into(),
                    "--add".into(),
                    "--cacheinfo".into(),
                    format!("{mode},{object},{path}")
                ]));
                assert!(input.is_none());
                Ok(result(&[]))
            })
            .unwrap();
        }
        assert_eq!(
            s.step_with(Step::WriteTree, &token(), |_, args, _, input, _| {
                assert_eq!(args.last().unwrap(), "write-tree");
                assert!(input.is_none());
                Ok(result(format!("{}\n", "b".repeat(format)).as_bytes()))
            })
            .unwrap(),
            "b".repeat(format).as_bytes()
        );
        assert!(s.step_with(Step::ReadTree, &token(), no_dispatch).is_err());
        fs::write(scratch.join("index"), b"synthetic index").unwrap();
        fs::write(scratch.join("index.lock"), b"synthetic lock").unwrap();
        assert!(s.cleanup());
        assert!(!scratch.exists());
        f.unchanged();
    }
}

#[test]
fn candidate_worker_refuses_unbound_plans_and_any_occupied_scratch_identity() {
    let f = Fixture::new(40);
    for field in ["digest", "network", "index", "repository"] {
        let mut p = f.plan.clone();
        match field {
            "digest" => p.plan_sha256 = "f".repeat(64),
            "network" => p.network_authority = true,
            "index" => p.temporary_index_path_sha256 = "f".repeat(64),
            _ => p.repository_sha256 = "f".repeat(64),
        };
        if field != "digest" {
            reseal(&mut p);
        }
        assert!(CandidateSession::new(&f.scope, &p).is_err());
        assert!(fs::read_dir(&f.scope.owned_root).unwrap().next().is_none());
    }
    for id in ["", "../escape", "slash/name", "newline\n", "a\0b"] {
        assert!(index_path(&f.scope, id).is_none());
    }
    let scratch = index_path(&f.scope, &f.plan.candidate_build_id)
        .unwrap()
        .parent()
        .unwrap()
        .to_owned();
    symlink(f.base.join("missing"), &scratch).unwrap();
    assert!(CandidateSession::new(&f.scope, &f.plan).is_err());
    assert!(scratch.is_symlink());
    fs::remove_file(&scratch).unwrap();
    fs::create_dir(&scratch).unwrap();
    fs::write(scratch.join("sentinel"), b"existing").unwrap();
    assert!(CandidateSession::new(&f.scope, &f.plan).is_err());
    assert_eq!(fs::read(scratch.join("sentinel")).unwrap(), b"existing");
    f.unchanged();
}

#[test]
fn candidate_worker_refuses_wrong_order_postimage_and_reuse_without_dispatch() {
    for bad in [
        "early-write",
        "early-add",
        "early-hash",
        "wrong-operation",
        "wrong-bytes",
        "repeat-read",
        "missing-files",
    ] {
        let f = Fixture::new(40);
        let mut s = f.session();
        if matches!(
            bad,
            "wrong-operation" | "wrong-bytes" | "repeat-read" | "missing-files"
        ) {
            initialize(&mut s);
        }
        let step = match bad {
            "early-write" | "missing-files" => Step::WriteTree,
            "early-add" => Step::Add,
            "early-hash" => Step::Hash {
                operation: "one",
                bytes: b"first\n",
            },
            "wrong-operation" => Step::Hash {
                operation: "two",
                bytes: b"second\n",
            },
            "wrong-bytes" => Step::Hash {
                operation: "one",
                bytes: b"wrong",
            },
            _ => Step::ReadTree,
        };
        assert!(s.step_with(step, &token(), no_dispatch).is_err(), "{bad}");
        assert!(s.step_with(Step::ReadTree, &token(), no_dispatch).is_err());
        assert!(s.cleanup());
        f.unchanged();
    }
}

#[test]
fn candidate_worker_refuses_failed_incomplete_cancelled_and_malformed_results() {
    for bad in [
        "incomplete",
        "failed",
        "no-status",
        "overflow",
        "truncated",
        "unexpected-output",
        "timeout",
        "dispatch-error",
    ] {
        let f = Fixture::new(40);
        let mut s = f.session();
        assert!(
            s.step_with(Step::ReadTree, &token(), |_, _, _, _, _| {
                let mut r = result(&[]);
                match bad {
                    "incomplete" => r.output_complete = false,
                    "failed" => r.status = Some(ExitStatus::from_raw(1 << 8)),
                    "no-status" => r.status = None,
                    "overflow" => r.stderr.total = MAX_OBSERVATION_BYTES + 1,
                    "truncated" => r.stdout.total = 1,
                    "unexpected-output" => r.stdout = result(b"forged").stdout,
                    "timeout" => r.outcome = Ok(OperationOutcome::TimedOut),
                    _ => return Err(InvocationStatus::CleanupFailed),
                };
                Ok(r)
            })
            .is_err(),
            "{bad}"
        );
        assert!(s.step_with(Step::ReadTree, &token(), no_dispatch).is_err());
        if matches!(bad, "incomplete" | "dispatch-error") {
            assert!(!s.cleanup());
            assert!(s.worker.scope.owned_root.exists());
        } else {
            assert!(s.cleanup());
        }
        f.unchanged();
    }
    let f = Fixture::new(40);
    let mut s = f.session();
    initialize(&mut s);
    assert!(
        s.step_with(
            Step::Hash {
                operation: "one",
                bytes: b"first\n"
            },
            &token(),
            |_, _, _, _, _| Ok(result(format!("{}\n", "f".repeat(40)).as_bytes()))
        )
        .is_err()
    );
    assert!(s.cleanup());
    f.unchanged();
}

#[test]
fn candidate_worker_refuses_replaced_roots_and_preserves_unknown_cleanup_entries() {
    for bad in [
        "replace-scratch",
        "replace-parent",
        "unknown-file",
        "symlink-index",
    ] {
        let f = Fixture::new(40);
        let mut s = f.session();
        let scratch = s.worker.scope.owned_root.clone();
        match bad {
            "replace-scratch" | "replace-parent" => {
                assert!(
                    s.step_with(Step::ReadTree, &token(), |_, _, _, _, _| {
                        let target = if bad == "replace-parent" {
                            &f.scope.owned_root
                        } else {
                            &scratch
                        };
                        fs::rename(target, target.with_extension("retained")).unwrap();
                        fs::create_dir(target).unwrap();
                        fs::set_permissions(target, fs::Permissions::from_mode(0o700)).unwrap();
                        fs::write(target.join("sentinel"), b"replacement").unwrap();
                        Ok(result(&[]))
                    })
                    .is_err()
                );
            }
            "unknown-file" => fs::write(scratch.join("unknown"), b"preserve").unwrap(),
            _ => symlink(f.scope.git_directory.join("index"), scratch.join("index")).unwrap(),
        }
        assert!(!s.cleanup());
        if bad == "replace-parent" {
            assert!(
                f.scope
                    .owned_root
                    .with_extension("retained")
                    .join(scratch.file_name().unwrap())
                    .exists()
            );
            assert_eq!(
                fs::read(f.scope.owned_root.join("sentinel")).unwrap(),
                b"replacement"
            );
        } else {
            assert!(scratch.exists());
        }
        f.unchanged();
    }
}

#[test]
fn candidate_worker_obeys_one_total_deadline_and_precancellation_without_launch() {
    let f = Fixture::new(40);
    let mut s = f.session();
    s.deadline = Instant::now() - Duration::from_secs(1);
    assert!(s.step_with(Step::ReadTree, &token(), no_dispatch).is_err());
    assert!(s.cleanup());
    let mut s = f.session();
    let t = token();
    use agentmage_kernel_contracts::*;
    t.cancel(CancellationSignal {
        schema_version: CONTRACT_SCHEMA_VERSION,
        cancellation_id: CancellationId::from_raw("cancel-candidate"),
        task_id: TaskId::from_raw("candidate-test"),
        correlation_id: CorrelationId::from_raw("candidate-test"),
        reason: CancellationReason::UserRequested,
        requested_by: BoundaryKind::Shell,
    })
    .unwrap();
    let git = LinuxGitArtifact {
        descriptor: open(
            "/usr/bin/git",
            OFlags::RDONLY | OFlags::CLOEXEC,
            Mode::empty(),
        )
        .unwrap(),
        launch_path: "/usr/bin/git".into(),
        sha256: "0".repeat(64),
    };
    assert!(s.read_tree(&git, &t).is_err());
    assert!(s.cleanup());
    f.unchanged();
}

#[test]
fn candidate_worker_projects_only_its_private_scratch_and_keeps_filter_separate_from_input() {
    let f = Fixture::new(40);
    let mut s = f.session();
    let scratch = s.worker.scope.owned_root.clone();
    s.step_with(Step::ReadTree, &token(), |owner, args, env, _, _| {
        let artifact = || LinuxGitArtifact {
            descriptor: open(
                "/usr/bin/git",
                OFlags::RDONLY | OFlags::CLOEXEC,
                Mode::empty(),
            )
            .unwrap(),
            launch_path: "/usr/bin/git".into(),
            sha256: "0".repeat(64),
        };
        let manifest = LinuxRepositoryInspectionManifest {
            systemd_run: artifact(),
            systemctl: artifact(),
            bubblewrap: artifact(),
            git: artifact(),
        };
        let filter = crate::sandbox::supervision::FilterPipe::new().unwrap();
        let command = worker_command(
            &manifest,
            &owner,
            args,
            env,
            "agentmage-repository-git-0123456789abcdef01234567.service",
            &filter,
        );
        let args = command
            .get_args()
            .map(|a| a.to_str().unwrap())
            .collect::<Vec<_>>();
        assert!(
            args.windows(3)
                .any(|a| a == ["--bind-fd", "7", scratch.to_str().unwrap()])
        );
        assert!(
            !args
                .windows(3)
                .any(|a| a == ["--bind-fd", "7", f.scope.owned_root.to_str().unwrap()])
        );
        assert!(
            args.windows(3)
                .any(|a| a == ["--ro-bind-fd", "5", f.scope.checkout_root.to_str().unwrap()])
        );
        assert!(args.windows(2).any(|a| a == ["--seccomp", "4"]));
        assert!(args.windows(3).any(|a| a
            == [
                "--setenv",
                "GIT_INDEX_FILE",
                scratch.join("index").to_str().unwrap()
            ]));
        assert_eq!(owner.projections().unwrap().len(), 3);
        assert_eq!(command.get_envs().count(), 2);
        assert!(args.contains(&"--no-replace-objects"));
        assert!(args.contains(&"--unshare-all"));
        Ok(result(&[]))
    })
    .unwrap();
    assert!(s.cleanup());
    f.unchanged();
}

#[test]
fn candidate_worker_refuses_index_aliases_before_dispatch_and_cancellation_after_dispatch() {
    for bad in ["symlink", "hardlink", "lock", "unknown"] {
        let f = Fixture::new(40);
        let mut s = f.session();
        let scratch = s.worker.scope.owned_root.clone();
        match bad {
            "symlink" => {
                symlink(f.scope.git_directory.join("index"), scratch.join("index")).unwrap()
            }
            "hardlink" => {
                fs::hard_link(f.scope.git_directory.join("index"), scratch.join("index")).unwrap()
            }
            "lock" => fs::write(scratch.join("index.lock"), b"stale lock").unwrap(),
            _ => fs::write(scratch.join("unowned"), b"retain").unwrap(),
        }
        assert!(s.step_with(Step::ReadTree, &token(), no_dispatch).is_err());
        if matches!(bad, "symlink" | "unknown") {
            assert!(!s.cleanup());
        } else {
            assert!(s.cleanup());
        }
        f.unchanged();
    }
    let f = Fixture::new(40);
    let mut s = f.session();
    let t = token();
    assert!(
        s.step_with(Step::ReadTree, &t, |_, _, _, _, _| {
            use agentmage_kernel_contracts::*;
            t.cancel(CancellationSignal {
                schema_version: CONTRACT_SCHEMA_VERSION,
                cancellation_id: CancellationId::from_raw("during-candidate"),
                task_id: TaskId::from_raw("candidate-test"),
                correlation_id: CorrelationId::from_raw("candidate-test"),
                reason: CancellationReason::UserRequested,
                requested_by: BoundaryKind::Shell,
            })
            .unwrap();
            Ok(result(&[]))
        })
        .is_err()
    );
    assert!(s.cleanup());
    f.unchanged();
}

#[test]
fn candidate_setup_inherits_outer_deadline_and_expired_or_cancelled_entry_creates_nothing() {
    let f = Fixture::new(40);
    let deadline = Instant::now() + Duration::from_secs(300);
    let token = token();
    let mut session = CandidateSession::new_until(&f.scope, &f.plan, &token, deadline).unwrap();
    assert!(session.deadline < deadline); // Existing thirty-second cap still applies.
    assert!(session.cleanup());
    let expired = Instant::now();
    assert!(CandidateSession::new_until(&f.scope, &f.plan, &token, expired).is_err());
    assert!(fs::read_dir(&f.scope.owned_root).unwrap().next().is_none());
    let deadline = Instant::now() + Duration::from_secs(20);
    let mut session = CandidateSession::new_until(&f.scope, &f.plan, &token, deadline).unwrap();
    assert_eq!(session.deadline, deadline);
    // Native worker forwarding checks this identical deadline before and after
    // every step; no short real-time completion window is needed in this test.
    session
        .step_with(Step::ReadTree, &token, |_, _, _, _, observed| {
            assert_eq!(observed, deadline);
            Ok(result(&[]))
        })
        .unwrap();
    assert!(session.cleanup());
    token
        .cancel(agentmage_kernel_contracts::CancellationSignal {
            schema_version: agentmage_kernel_contracts::CONTRACT_SCHEMA_VERSION,
            cancellation_id: agentmage_kernel_contracts::CancellationId::from_raw("setup-cancel"),
            task_id: TaskId::from_raw("candidate-test"),
            correlation_id: CorrelationId::from_raw("candidate-test"),
            requested_by: BoundaryKind::Shell,
            reason: agentmage_kernel_contracts::CancellationReason::UserRequested,
        })
        .unwrap();
    assert!(CandidateSession::new_until(&f.scope, &f.plan, &token, deadline).is_err());
    assert!(fs::read_dir(&f.scope.owned_root).unwrap().next().is_none());
    f.unchanged();
}

#[test]
fn candidate_setup_cancellation_during_capture_rolls_back_only_its_empty_claim() {
    for occupied in [false, true] {
        let f = Fixture::new(40);
        let token = token();
        let deadline = Instant::now() + Duration::from_secs(300);
        assert!(
            CandidateSession::new_with_until(&f.scope, &f.plan, &token, deadline, |scratch| {
                let scope = LinuxRepositoryScope::verify(
                    &f.scope.checkout_root,
                    &f.scope.git_directory,
                    scratch,
                )?;
                let worker = LinuxRepositoryMutationScope::capture(&scope)?;
                if occupied {
                    fs::write(scratch.join("unattributed"), b"preserve").unwrap();
                }
                token
                    .cancel(agentmage_kernel_contracts::CancellationSignal {
                        schema_version: agentmage_kernel_contracts::CONTRACT_SCHEMA_VERSION,
                        cancellation_id: agentmage_kernel_contracts::CancellationId::from_raw(
                            "capture-cancel",
                        ),
                        task_id: TaskId::from_raw("candidate-test"),
                        correlation_id: CorrelationId::from_raw("candidate-test"),
                        requested_by: BoundaryKind::Shell,
                        reason: agentmage_kernel_contracts::CancellationReason::UserRequested,
                    })
                    .unwrap();
                Ok(worker)
            })
            .is_err()
        );
        let scratch = index_path(&f.scope, &f.plan.candidate_build_id)
            .unwrap()
            .parent()
            .unwrap()
            .to_owned();
        assert_eq!(scratch.exists(), occupied);
        if occupied {
            assert_eq!(fs::read(scratch.join("unattributed")).unwrap(), b"preserve");
        }
        f.unchanged();
    }
}
