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
struct Fixture {
    root: PathBuf,
}
impl Fixture {
    fn new() -> Self {
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
    fn open(&self) -> LinuxCodingRepository {
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

fn manifest() -> RepositoryPreservationManifest {
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
