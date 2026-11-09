// SPDX-License-Identifier: Apache-2.0
//! Synthetic platform outcomes through real native handles and durable authority.

use super::*;
use agentmage_kernel_contracts::{AuthorityTransactionState, OperationOutcome};
use agentmage_kernel_engine::repository_safety::{
    BoundedRepositoryExecutor, RepositoryLaunchPermit, RepositoryPlatformResult,
    RepositoryPreservationManifest,
};
use std::fs;
use std::os::unix::fs::{PermissionsExt, symlink};
use std::sync::{
    Arc,
    atomic::{AtomicU64, AtomicUsize, Ordering},
};

static NEXT: AtomicU64 = AtomicU64::new(0);
const KEY: &[u8] = b"synthetic-test-control-secret-not-a-real-credential";
const NOW: u64 = 1_000;
pub(super) struct Fixture {
    pub(super) root: PathBuf,
}
impl Fixture {
    pub(super) fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "agentmage-repository-owner-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        for name in ["state", "repo", "repo/.git", "owned"] {
            fs::create_dir(root.join(name)).unwrap();
            fs::set_permissions(root.join(name), fs::Permissions::from_mode(0o700)).unwrap();
        }
        fs::write(root.join("repo/sentinel"), b"preserve human work").unwrap();
        Self { root }
    }
    pub(super) fn open(&self) -> LinuxCodingRepository {
        LinuxCodingRepository::open(&self.root.join("state"), KEY, NOW).unwrap()
    }
    fn native(&self) -> LinuxAuthorizedWorkspace {
        held_root(&self.root.join("repo")).unwrap()
    }
    fn metadata(&self, repository: &LinuxAuthorizedWorkspace) -> LinuxHeldObject {
        LinuxPathAdapter::new(AdapterInstanceId::from_raw(TOOL), 0)
            .resolve(
                repository,
                &WorkspacePath::new(WorkspaceId::from_raw(TOOL), [".git"]).unwrap(),
                PathResolutionIntent::Metadata,
            )
            .unwrap()
    }
    fn plan(&self, id: &str) -> (PreparedRepositoryOperation, RepositoryPreservationManifest) {
        let before = manifest();
        let plan = plan_worktree_create(
            id,
            "component",
            &"1".repeat(40),
            &crate::linux_repository_path_sha256(self.root.join("repo")).unwrap(),
            &hash(self.root.join("owned/worktree").as_os_str().as_bytes()),
            &before,
        )
        .unwrap();
        (PreparedRepositoryOperation::new(plan).unwrap(), before)
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.root).unwrap();
    }
}

pub(super) fn manifest() -> RepositoryPreservationManifest {
    RepositoryPreservationManifest::seal(RepositoryPreservationManifest {
        schema_version: 0,
        repository_sha256: hash(b"repo"),
        common_directory_sha256: hash(b"common"),
        object_format: "sha1".into(),
        safe_ownership: true,
        head_object: Some("1".repeat(40)),
        current_branch: Some("refs/heads/main".into()),
        upstream: None,
        detached: false,
        index_sha256: hash(b"index"),
        path_dispositions_sha256: hash(b"paths"),
        untracked_count: 1,
        ignored_count: 0,
        local_branches_sha256: hash(b"local"),
        remote_tracking_refs_sha256: hash(b"remote"),
        tags_sha256: hash(b"tags"),
        notes_sha256: hash(b"notes"),
        stash_sha256: hash(b"stash"),
        replacement_refs_sha256: hash(b"replacements"),
        reflogs_sha256: hash(b"reflogs"),
        agentmage_refs_sha256: hash(b"agentmage"),
        worktrees_sha256: hash(b"worktrees"),
        submodules_sha256: hash(b"submodules"),
        lfs_sha256: hash(b"lfs"),
        object_database_sha256: hash(b"objects"),
        configuration_sha256: hash(b"config"),
        hooks_sha256: hash(b"hooks"),
        content_drivers_sha256: hash(b"drivers"),
        remotes_sha256: hash(b"remotes"),
        operations_sha256: hash(b"operations"),
        shallow: false,
        partial: false,
        hazardous_configuration: false,
        manifest_sha256: String::new(),
    })
    .unwrap()
}
struct Synthetic {
    calls: Arc<AtomicUsize>,
    corrupt: bool,
    crash: bool,
}
impl BoundedRepositoryExecutor for Synthetic {
    fn execute(
        &mut self,
        permit: RepositoryLaunchPermit<'_>,
        _: &CancellationToken,
    ) -> RepositoryPlatformResult {
        self.calls.fetch_add(1, Ordering::Relaxed);
        assert_eq!(
            permit.plan().operation,
            agentmage_kernel_engine::repository_safety::RepositoryOperation::WorktreeCreate
        );
        if self.crash {
            panic!("synthetic abrupt worker loss after durable consume");
        }
        let mut after = permit.before().clone();
        after.agentmage_refs_sha256 = hash(b"new task branch");
        after.worktrees_sha256 = hash(b"new worktree registration");
        if self.corrupt {
            after.index_sha256 = hash(b"unapproved checkout index change");
        }
        after = RepositoryPreservationManifest::seal(after).unwrap();
        RepositoryPlatformResult {
            outcome: OperationOutcome::Succeeded,
            before: permit.before().clone(),
            after,
            cleanup_verified: true,
            platform_code: "test.synthetic.worktree".into(),
        }
    }
}

#[test]
fn exact_worktree_authority_is_durable_and_not_replayed_on_reopen() {
    let f = Fixture::new();
    let mut owner = f.open();
    let repository = f.native();
    let (plan, before) = f.plan("synthetic-success");
    let calls = Arc::new(AtomicUsize::new(0));
    let (receipt, specialized) = dispatch(
        owner.authority.authority_mut(),
        &repository,
        f.metadata(&repository),
        plan,
        before,
        hash(b"approved preview"),
        NOW + LIFETIME_MS,
        NOW,
        Synthetic {
            calls: calls.clone(),
            corrupt: false,
            crash: false,
        },
    )
    .unwrap();
    assert_eq!(receipt.outcome, OperationOutcome::Succeeded);
    assert_eq!(specialized.unwrap().outcome, OperationOutcome::Succeeded);
    assert_eq!(calls.load(Ordering::Relaxed), 1);
    let retained = owner.inspect().unwrap()["receipts"].clone();
    let (again, before) = f.plan("synthetic-success");
    assert!(
        dispatch(
            owner.authority.authority_mut(),
            &repository,
            f.metadata(&repository),
            again,
            before,
            hash(b"approved preview"),
            NOW + LIFETIME_MS,
            NOW,
            Synthetic {
                calls: calls.clone(),
                corrupt: false,
                crash: false
            }
        )
        .is_err()
    );
    assert_eq!(calls.load(Ordering::Relaxed), 1);
    drop(owner);
    let reopened = f.open();
    assert_eq!(reopened.inspect().unwrap()["receipts"], retained);
    assert!(reopened.inspect().unwrap()["pending"].is_null());
    assert_eq!(calls.load(Ordering::Relaxed), 1);
    assert_eq!(
        fs::read(f.root.join("repo/sentinel")).unwrap(),
        b"preserve human work"
    );
    assert!(
        !f.root.join("owned/worktree").exists(),
        "synthetic result is never native creation"
    );
}

#[test]
fn unexpected_human_checkout_change_cannot_be_reported_as_success() {
    let f = Fixture::new();
    let mut owner = f.open();
    let repository = f.native();
    let (plan, before) = f.plan("synthetic-drift");
    let (receipt, specialized) = dispatch(
        owner.authority.authority_mut(),
        &repository,
        f.metadata(&repository),
        plan,
        before,
        hash(b"approved"),
        NOW + LIFETIME_MS,
        NOW,
        Synthetic {
            calls: Arc::new(AtomicUsize::new(0)),
            corrupt: true,
            crash: false,
        },
    )
    .unwrap();
    assert_ne!(receipt.outcome, OperationOutcome::Succeeded);
    assert!(specialized.is_none());
    drop(owner);
    assert_ne!(
        f.open().inspect().unwrap()["receipts"][0]["outcome"],
        "succeeded"
    );
}

#[test]
fn interrupted_dispatch_recovers_as_uncertain_without_another_launch() {
    let f = Fixture::new();
    let mut owner = f.open();
    let repository = f.native();
    let (plan, before) = f.plan("synthetic-interrupted");
    let calls = Arc::new(AtomicUsize::new(0));
    assert!(
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _ = dispatch(
                owner.authority.authority_mut(),
                &repository,
                f.metadata(&repository),
                plan,
                before,
                hash(b"approved"),
                NOW + LIFETIME_MS,
                NOW,
                Synthetic {
                    calls: calls.clone(),
                    corrupt: false,
                    crash: true,
                },
            );
        }))
        .is_err()
    );
    drop(owner);
    let mut reopened = f.open();
    let transaction = reopened
        .authority
        .authority()
        .current_transaction(&AuthorityTransactionId::from_raw("synthetic-interrupted"))
        .unwrap();
    assert_eq!(transaction.state, AuthorityTransactionState::Terminal);
    assert_eq!(transaction.outcome, Some(OperationOutcome::Uncertain));
    assert_eq!(calls.load(Ordering::Relaxed), 1);
    assert_eq!(
        reopened
            .decide(&hash(b"approved"), true, NOW)
            .err()
            .unwrap()
            .code(),
        "code.repository.choice-required"
    );
}

#[test]
fn state_and_repository_scopes_refuse_unsafe_inputs_without_commands() {
    let f = Fixture::new();
    let mut owner = f.open();
    for (root, owned, task, base) in [
        (
            PathBuf::from("relative"),
            f.root.join("owned"),
            "task",
            "1".repeat(40),
        ),
        (
            f.root.join("repo"),
            f.root.join("repo/owned"),
            "task",
            "1".repeat(40),
        ),
        (
            f.root.join("repo"),
            f.root.join("owned"),
            "../escape",
            "1".repeat(40),
        ),
        (
            f.root.join("repo"),
            f.root.join("owned"),
            "task",
            "HEAD".into(),
        ),
    ] {
        assert_eq!(
            owner
                .prepare(&root, &owned, task, &base, NOW)
                .err()
                .unwrap()
                .code(),
            "code.repository.input"
        );
    }
    assert_eq!(
        owner
            .prepare(
                &f.root.join("state"),
                &f.root.join("owned"),
                "task",
                &"1".repeat(40),
                NOW
            )
            .err()
            .unwrap()
            .code(),
        "code.repository.scope"
    );
    symlink(f.root.join("repo"), f.root.join("link")).unwrap();
    assert_eq!(
        owner
            .prepare(
                &f.root.join("link"),
                &f.root.join("owned"),
                "task",
                &"1".repeat(40),
                NOW
            )
            .err()
            .unwrap()
            .code(),
        "code.repository.scope"
    );
    fs::write(f.root.join("owned/sentinel"), b"preserve").unwrap();
    assert!(
        owner
            .prepare(
                &f.root.join("repo"),
                &f.root.join("owned"),
                "task",
                &"1".repeat(40),
                NOW
            )
            .is_err()
    );
    assert_eq!(
        fs::read(f.root.join("owned/sentinel")).unwrap(),
        b"preserve"
    );
    assert!(
        owner.inspect().unwrap()["receipts"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    drop(owner);
    assert!(
        LinuxCodingRepository::open(
            &f.root.join("state"),
            b"different-synthetic-control-secret-with-enough-bytes",
            NOW
        )
        .is_err()
    );
    assert!(
        f.open().inspect().unwrap()["receipts"]
            .as_array()
            .unwrap()
            .is_empty()
    );
}

#[test]
fn native_worktree_path_runs_or_retains_the_exact_trust_refusal() {
    let f = Fixture::new();
    let mut owner = f.open();
    let repo = f.root.join("repo");
    let owned = f.root.join("owned");
    if let Err(error) = LinuxGitArtifact::verify("/usr/bin/git") {
        let refusal = owner
            .prepare(&repo, &owned, "native", &"1".repeat(40), NOW)
            .err()
            .unwrap();
        assert_eq!(refusal.code(), error.kind().code());
        assert_eq!(refusal.code(), "linux.repository.git_artifact.invalid");
        assert!(owner.inspect().unwrap()["pending"].is_null());
        assert!(
            owner.inspect().unwrap()["receipts"]
                .as_array()
                .unwrap()
                .is_empty()
        );
        assert!(!owned.join("worktree").exists());
        assert!(owner.created_git_identity().is_err());
        assert!(owner.inspect_apply().unwrap()["git_creation"].is_null());
        println!("native creation not measured: exact system Git trust refusal retained");
        return;
    }
    // Fixture setup only, never a product executor or trust workaround.
    let git = |args: &[&str]| {
        let output = std::process::Command::new("/usr/bin/git")
            .env_clear()
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .args([
                "-c",
                "core.hooksPath=/dev/null",
                "-c",
                "maintenance.auto=false",
                "-c",
                "gc.auto=0",
            ])
            .args(args)
            .current_dir(&repo)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8(output.stdout).unwrap().trim().to_owned()
    };
    git(&["init", "--template=", "--initial-branch=main"]);
    git(&["add", "sentinel"]);
    git(&[
        "-c",
        "user.name=Synthetic Fixture",
        "-c",
        "user.email=fixture@example.invalid",
        "commit",
        "-m",
        "base",
    ]);
    let base = git(&["rev-parse", "HEAD"]);
    fs::write(repo.join("sentinel"), b"unstaged human edit").unwrap();
    fs::write(repo.join("untracked"), b"untracked human edit").unwrap();
    let index = fs::read(repo.join(".git/index")).unwrap();
    let preview = owner.prepare(&repo, &owned, "native", &base, NOW).unwrap();
    assert!(owner.decide(&hash(b"foreign preview"), true, NOW).is_err());
    assert_eq!(owner.inspect().unwrap()["pending"], preview);
    assert_eq!(
        owner
            .decide(preview["preview_sha256"].as_str().unwrap(), false, NOW)
            .unwrap()["outcome"],
        "denied"
    );
    assert!(!owned.join("worktree").exists());
    let preview = owner.prepare(&repo, &owned, "native", &base, NOW).unwrap();
    assert_eq!(
        owner
            .decide(
                preview["preview_sha256"].as_str().unwrap(),
                true,
                NOW + LIFETIME_MS
            )
            .err()
            .unwrap()
            .code(),
        "code.repository.expired"
    );
    assert!(!owned.join("worktree").exists());
    let preview = owner.prepare(&repo, &owned, "native", &base, NOW).unwrap();
    let result = owner
        .decide(preview["preview_sha256"].as_str().unwrap(), true, NOW)
        .unwrap();
    assert_eq!(result["outcome"], "succeeded");
    let identity = owner.created_git_identity().unwrap();
    assert_eq!(identity.branch_ref(), "refs/heads/agentmage/tasks/native");
    assert_eq!(identity.base_object(), base);
    assert_eq!(identity.plan_sha256(), preview["plan"]["plan_sha256"]);
    assert_eq!(
        identity.authority_receipt_sha256(),
        result["receipt"]["receipt_sha256"]
    );
    assert_eq!(
        identity.repository_receipt_sha256(),
        result["repository_receipt"]["receipt_sha256"]
    );
    assert_eq!(
        owner.inspect_apply().unwrap()["git_creation"],
        identity.inspection()
    );
    assert_eq!(
        fs::read(owned.join("worktree/sentinel")).unwrap(),
        b"preserve human work"
    );
    assert_eq!(
        fs::read(repo.join("sentinel")).unwrap(),
        b"unstaged human edit"
    );
    assert_eq!(
        fs::read(repo.join("untracked")).unwrap(),
        b"untracked human edit"
    );
    assert_eq!(fs::read(repo.join(".git/index")).unwrap(), index);
    assert_eq!(git(&["rev-parse", "HEAD"]), base);
    assert_eq!(git(&["symbolic-ref", "HEAD"]), "refs/heads/main");
    drop(owner);
    let mut reopened = f.open();
    assert_eq!(
        reopened.inspect().unwrap()["receipts"][0]["outcome"],
        "succeeded"
    );
    assert!(
        reopened
            .decide(preview["preview_sha256"].as_str().unwrap(), true, NOW)
            .is_err()
    );
}

// Synthetic executor observations exercise correlation and owner lifetime only.
// They never claim a native branch, Git execution, signature or commit approval.
fn synthetic_creation_observations(
    f: &Fixture,
) -> (
    LinuxCodingRepository,
    agentmage_kernel_engine::repository_safety::RepositoryOperationPlan,
    agentmage_kernel_contracts::Receipt,
    agentmage_kernel_engine::repository_safety::RepositoryOperationReceipt,
) {
    let mut owner = f.open();
    let repository = f.native();
    let (prepared, before) = f.plan("synthetic-git-identity");
    let plan = prepared.plan().clone();
    let (authority, specialized) = dispatch(
        owner.authority.authority_mut(),
        &repository,
        f.metadata(&repository),
        prepared,
        before,
        hash(b"synthetic exact decision"),
        NOW + LIFETIME_MS,
        NOW,
        Synthetic {
            calls: Arc::new(AtomicUsize::new(0)),
            corrupt: false,
            crash: false,
        },
    )
    .unwrap();
    (owner, plan, authority, specialized.unwrap())
}

pub(super) fn owner_with_synthetic_git_observations(f: &Fixture) -> LinuxCodingRepository {
    let (mut owner, plan, authority, repository) = synthetic_creation_observations(f);
    fs::create_dir(f.root.join("owned/worktree")).unwrap();
    owner.created = Some(CreatedWorktree {
        original_path: f.root.join("repo"),
        original: f.native(),
        path: f.root.join("owned/worktree"),
        held: held_root(&f.root.join("owned/worktree")).unwrap(),
        owned: held_root(&f.root.join("owned")).unwrap(),
        creation_receipt_sha256: authority.receipt_sha256.clone(),
        git_creation: Some(
            LinuxCodingGitCreation::from_dispatch(&plan, &authority, Some(&repository)).unwrap(),
        ),
        edits: BTreeMap::new(),
    });
    owner
}

#[test]
fn git_creation_identity_correlates_original_plan_and_both_dispatch_receipts() {
    let f = Fixture::new();
    let (_owner, plan, authority, repository) = synthetic_creation_observations(&f);
    let identity =
        LinuxCodingGitCreation::from_dispatch(&plan, &authority, Some(&repository)).unwrap();
    assert_eq!(
        identity.branch_ref(),
        "refs/heads/agentmage/tasks/component"
    );
    assert_eq!(identity.base_object(), "1".repeat(40));
    assert_eq!(identity.plan_sha256(), plan.plan_sha256);
    assert_eq!(
        identity.authority_receipt_sha256(),
        authority.receipt_sha256
    );
    assert_eq!(
        identity.repository_receipt_sha256(),
        repository.receipt_sha256
    );
    assert_eq!(identity.inspection()["current_git_state_verified"], false);
    assert_eq!(identity.inspection()["commit_authority"], false);
    assert_eq!(identity.inspection()["publication_authority"], false);
    assert!(!f.root.join("owned/worktree").exists());
    assert_eq!(
        fs::read(f.root.join("repo/sentinel")).unwrap(),
        b"preserve human work"
    );
}

#[test]
fn git_creation_identity_refuses_missing_failed_or_uncorrelated_dispatch_results() {
    use agentmage_kernel_engine::repository_safety::RepositoryOperation;
    let f = Fixture::new();
    let (_owner, plan, authority, repository) = synthetic_creation_observations(&f);
    assert!(LinuxCodingGitCreation::from_dispatch(&plan, &authority, None).is_err());
    for case in 0..14 {
        let mut plan = plan.clone();
        let mut authority = authority.clone();
        let mut repository = repository.clone();
        match case {
            0 => authority.outcome = OperationOutcome::Failed,
            1 => authority.operation = OperationBinding::new(GrantOperation::GitBranchFastForward),
            2 => repository.outcome = OperationOutcome::Failed,
            3 => repository.operation = RepositoryOperation::Fetch,
            4 => repository.cleanup_verified = false,
            5 => repository.repository_transaction_id = "foreign-transaction".into(),
            6 => repository.plan_sha256 = hash(b"foreign plan"),
            7 => repository.before_manifest_sha256 = hash(b"foreign manifest"),
            8 => repository.authority_transaction_id = "foreign-authority".into(),
            9 => repository.operation_attempt_id = "foreign-attempt".into(),
            10 => plan.branch_ref = Some("refs/heads/main".into()),
            11 => plan.expected_new_object = Some("2".repeat(40)),
            12 => plan.operation = RepositoryOperation::Fetch,
            13 => {
                authority.authority_transaction_id =
                    AuthorityTransactionId::from_raw("foreign-pair");
                repository.authority_transaction_id = "foreign-pair".into();
            }
            _ => unreachable!(),
        }
        assert_eq!(
            LinuxCodingGitCreation::from_dispatch(&plan, &authority, Some(&repository))
                .err()
                .unwrap()
                .code(),
            "code.repository.git-provenance",
            "case {case}"
        );
    }
}

#[test]
fn git_creation_identity_survives_same_owner_handoff_but_not_record_reopen() {
    let f = Fixture::new();
    let owner = owner_with_synthetic_git_observations(&f);
    let expected = owner.created_git_identity().unwrap().inspection();
    assert_eq!(owner.inspect_apply().unwrap()["git_creation"], expected);
    let root = owner
        .created
        .as_ref()
        .unwrap()
        .held
        .root_path_sha256()
        .to_owned();
    let lease = match owner.into_runtime_worktree(&root) {
        Ok(lease) => lease,
        Err(_) => panic!("synthetic held fixture must transfer"),
    };
    let other = lease.clone();
    let lease = match lease.try_into_repository() {
        Err(lease) => lease,
        Ok(_) => panic!("live reference must prevent owner return"),
    };
    drop(other);
    let returned = match lease.try_into_repository() {
        Ok(owner) => owner,
        Err(_) => panic!("last live reference must return same owner"),
    };
    assert_eq!(
        returned.created_git_identity().unwrap().inspection(),
        expected
    );
    assert_eq!(returned.inspect_apply().unwrap()["git_creation"], expected);
    let receipts = returned.inspect().unwrap()["receipts"].clone();
    drop(returned);
    let reopened = f.open();
    assert_eq!(reopened.inspect().unwrap()["receipts"], receipts);
    assert!(reopened.inspect_apply().unwrap()["git_creation"].is_null());
    assert!(reopened.created_git_identity().is_err());
    assert!(f.root.join("owned/worktree").is_dir());
    assert_eq!(
        fs::read(f.root.join("repo/sentinel")).unwrap(),
        b"preserve human work"
    );
}

#[test]
fn git_creation_identity_refuses_synthetic_creation_and_changed_held_roots() {
    for case in 0..5 {
        let f = Fixture::new();
        let mut owner = owner_with_synthetic_git_observations(&f);
        match case {
            0 => owner.created.as_mut().unwrap().git_creation = None,
            1 => owner.created.as_mut().unwrap().creation_receipt_sha256 = hash(b"substitution"),
            2..=4 => {
                let path = match case {
                    2 => "repo",
                    3 => "owned/worktree",
                    _ => "owned",
                };
                fs::rename(f.root.join(path), f.root.join("retained-before")).unwrap();
                fs::create_dir_all(f.root.join(path)).unwrap();
            }
            _ => unreachable!(),
        }
        assert!(owner.created_git_identity().is_err(), "case {case}");
        assert!(!f.root.join("owned/worktree/.git").exists());
    }
}

#[test]
fn git_creation_identity_is_bound_into_separately_approved_original_diff() {
    let f = Fixture::new();
    let mut owner = owner_with_synthetic_git_observations(&f);
    let expected = owner.created_git_identity().unwrap().inspection();
    let original = f.root.join("repo/sentinel");
    let worktree = f.root.join("owned/worktree/sentinel");
    let initial = fs::read(&original).unwrap();
    fs::write(&worktree, &initial).unwrap();
    fs::write(f.root.join("repo/.git/index"), b"human index sentinel").unwrap();
    fs::write(f.root.join("repo/untracked"), b"human untracked sentinel").unwrap();
    let paths = vec!["sentinel".into()];
    let scope = owner.select_created_sources(&paths).unwrap();
    let edit = owner
        .prepare_edit(
            scope,
            &[crate::LinuxCodingReplacement {
                path: "sentinel",
                before_sha256: &hash(&initial),
                content: "approved repair\n",
            }],
            "Synthetic creation; real native edit",
            NOW,
        )
        .unwrap();
    assert_eq!(
        owner
            .decide_edit(edit["preview_sha256"].as_str().unwrap(), true, NOW)
            .unwrap()["outcome"],
        "committed"
    );
    let status = owner.inspect_apply().unwrap();
    let preview = owner
        .prepare_apply(
            &paths,
            "Separately approve original diff",
            status["repository_root_sha256"].as_str().unwrap(),
            status["worktree_root_sha256"].as_str().unwrap(),
            NOW,
        )
        .unwrap();
    assert_eq!(preview["git_creation"], expected);
    let mut bound = preview.clone();
    bound.as_object_mut().unwrap().remove("preview_sha256");
    assert_eq!(
        preview["preview_sha256"],
        hash(&serde_json::to_vec(&bound).unwrap())
    );
    bound["git_creation"]["base_object"] = json!("2".repeat(40));
    let wrong = hash(&serde_json::to_vec(&bound).unwrap());
    assert!(owner.decide_apply(&wrong, true, NOW).is_err());
    assert_eq!(fs::read(&original).unwrap(), initial);
    let result = owner
        .decide_apply(preview["preview_sha256"].as_str().unwrap(), true, NOW)
        .unwrap();
    assert_eq!(result["outcome"], "committed");
    assert_eq!(fs::read(&original).unwrap(), b"approved repair\n");
    assert_eq!(fs::read(&worktree).unwrap(), b"approved repair\n");
    assert_eq!(
        fs::read(f.root.join("repo/.git/index")).unwrap(),
        b"human index sentinel"
    );
    assert_eq!(
        fs::read(f.root.join("repo/untracked")).unwrap(),
        b"human untracked sentinel"
    );
    assert_eq!(owner.created_git_identity().unwrap().inspection(), expected);
    assert_eq!(owner.inspect_apply().unwrap()["git_creation"], expected);
    assert!(!f.root.join("owned/worktree/.git").exists());
}
