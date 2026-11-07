// SPDX-License-Identifier: Apache-2.0
//! Synthetic creation provenance, real held roots, grants, diffs and file writes.

use super::*;
use std::fs;
use std::os::unix::fs::{PermissionsExt, symlink};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);
const NOW: u64 = 1000;
const KEY: &[u8] = b"synthetic-original-apply-test-control-secret";
struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "agentmage-original-apply-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        for name in [
            "state",
            "original",
            "original/.git",
            "owned",
            "owned/worktree",
        ] {
            fs::create_dir(root.join(name)).unwrap();
            fs::set_permissions(root.join(name), fs::Permissions::from_mode(0o700)).unwrap();
        }
        for name in ["original", "owned/worktree"] {
            fs::write(root.join(name).join("one.py"), b"first\n").unwrap();
            fs::write(root.join(name).join("two.py"), b"second\n").unwrap();
        }
        fs::write(root.join("original/human"), b"untracked human work").unwrap();
        fs::write(
            root.join("original/.git/index"),
            b"synthetic index sentinel",
        )
        .unwrap();
        Self(root)
    }
    fn open(&self) -> LinuxCodingRepository {
        LinuxCodingRepository::open(&self.0.join("state"), KEY, NOW).unwrap()
    }
    fn owner(&self) -> LinuxCodingRepository {
        let mut owner = self.open();
        // This is not a Git creation run. Public construction only retains these
        // handles after the native executor returns successful preservation.
        let path = self.0.join("owned/worktree");
        owner.created = Some(CreatedWorktree {
            original_path: self.0.join("original"),
            original: held_root(&self.0.join("original")).unwrap(),
            path: path.clone(),
            held: held_root(&path).unwrap(),
            owned: held_root(&self.0.join("owned")).unwrap(),
            creation_receipt_sha256: hash(b"synthetic-successful-creation"),
            edits: BTreeMap::new(),
        });
        owner
    }
    fn edit(&self, owner: &mut LinuxCodingRepository, path: &str, content: &str) {
        let scope = owner.select_created_sources(&[path.into()]).unwrap();
        let before = hash(&fs::read(self.0.join("owned/worktree").join(path)).unwrap());
        let preview = owner
            .prepare_edit(
                scope,
                &[LinuxCodingReplacement {
                    path,
                    before_sha256: &before,
                    content,
                }],
                "Fixture repair",
                NOW,
            )
            .unwrap();
        let result = owner
            .decide_edit(preview["preview_sha256"].as_str().unwrap(), true, NOW)
            .unwrap();
        assert_eq!(result["outcome"], "committed");
    }
    fn prepare(&self, owner: &mut LinuxCodingRepository) -> Value {
        owner
            .prepare_current_apply(&["one.py".into()], "Apply fixture repair", NOW)
            .unwrap()
    }
    fn assert_preserved(&self) {
        assert_eq!(
            fs::read(self.0.join("original/human")).unwrap(),
            b"untracked human work"
        );
        assert_eq!(
            fs::read(self.0.join("original/.git/index")).unwrap(),
            b"synthetic index sentinel"
        );
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

// Test convenience only: existing tests use the owner's current identities.
// Production callers must supply the identities displayed before read consent.
trait CurrentApply {
    fn prepare_current_apply(&mut self, paths: &[String], intent: &str, now: u64) -> Result<Value>;
}
impl CurrentApply for LinuxCodingRepository {
    fn prepare_current_apply(&mut self, paths: &[String], intent: &str, now: u64) -> Result<Value> {
        let status = self.inspect_apply()?;
        self.prepare_apply(
            paths,
            intent,
            status["repository_root_sha256"].as_str().unwrap_or(""),
            status["worktree_root_sha256"].as_str().unwrap_or(""),
            now,
        )
    }
}

#[test]
fn original_apply_transfers_cumulative_exact_edits_once_and_preserves_other_work() {
    let start = Instant::now();
    let f = Fixture::new();
    let mut owner = f.owner();
    f.edit(&mut owner, "one.py", "revision one\n");
    f.edit(&mut owner, "one.py", "revision two\n");
    f.edit(&mut owner, "two.py", "second repaired\n");
    let preview = owner
        .prepare_current_apply(
            &["one.py".into(), "two.py".into()],
            "Apply both repairs",
            NOW,
        )
        .unwrap();
    assert_eq!(preview["operation"], "apply-to-original");
    let mut body = preview.clone();
    body.as_object_mut().unwrap().remove("preview_sha256");
    assert_eq!(
        preview["preview_sha256"],
        hash(&serde_json::to_vec(&body).unwrap())
    );
    assert_eq!(fs::read(f.0.join("original/one.py")).unwrap(), b"first\n");
    let digest = preview["preview_sha256"].as_str().unwrap();
    assert_eq!(
        owner
            .decide_apply(&hash(b"foreign"), true, NOW)
            .unwrap_err()
            .code(),
        "code.apply.stale"
    );
    assert_eq!(owner.inspect_apply().unwrap()["pending"], preview);
    let result = owner.decide_apply(digest, true, NOW).unwrap();
    assert_eq!(result["outcome"], "committed");
    assert_eq!(result["tested"], false);
    assert_eq!(owner.inspect_apply().unwrap()["latest"], result);
    assert_eq!(
        fs::read(f.0.join("original/one.py")).unwrap(),
        b"revision two\n"
    );
    assert_eq!(
        fs::read(f.0.join("original/two.py")).unwrap(),
        b"second repaired\n"
    );
    f.assert_preserved();
    assert!(owner.decide_apply(digest, true, NOW).is_err());
    assert!(
        owner
            .prepare_current_apply(&["one.py".into()], "No replay", NOW)
            .is_err()
    );
    // A subsequent successful edit starts a fresh original preimage chain.
    f.edit(&mut owner, "one.py", "revision three\n");
    let preview = f.prepare(&mut owner);
    assert_eq!(
        owner
            .decide_apply(preview["preview_sha256"].as_str().unwrap(), true, NOW)
            .unwrap()["outcome"],
        "committed"
    );
    assert_eq!(
        fs::read(f.0.join("original/one.py")).unwrap(),
        b"revision three\n"
    );
    f.assert_preserved();
    println!(
        "component-only real-file workflow: 4 approved worktree edits, 2 approved original transfers, 3 transferred file versions; elapsed_us={}",
        start.elapsed().as_micros()
    );
}

#[test]
fn original_apply_refuses_unowned_unedited_reserved_duplicate_and_drifted_sources() {
    let f = Fixture::new();
    let mut owner = f.open();
    assert!(
        owner
            .prepare_current_apply(&["one.py".into()], "Refuse unowned", NOW)
            .is_err()
    );
    drop(owner);
    let mut owner = f.owner();
    for paths in [
        vec![],
        vec!["one.py".into()],
        vec!["../human".into()],
        vec![".git/index".into()],
    ] {
        assert!(owner.prepare_current_apply(&paths, "Refuse", NOW).is_err());
    }
    f.edit(&mut owner, "one.py", "repaired\n");
    let preview = f.prepare(&mut owner);
    for paths in [
        vec!["one.py".into(), "one.py".into()],
        vec!["two.py".into()],
    ] {
        assert!(owner.prepare_current_apply(&paths, "Refuse", NOW).is_err());
        assert_eq!(owner.inspect_apply().unwrap()["pending"], preview);
    }
    fs::write(
        f.0.join("owned/worktree/one.py"),
        b"foreign unapproved change",
    )
    .unwrap();
    assert_eq!(
        owner
            .prepare_current_apply(&["one.py".into()], "Refuse", NOW)
            .unwrap_err()
            .code(),
        "code.apply.source-drift"
    );
    assert!(
        owner
            .decide_apply(preview["preview_sha256"].as_str().unwrap(), true, NOW)
            .is_err()
    );
    assert_eq!(fs::read(f.0.join("original/one.py")).unwrap(), b"first\n");
    f.assert_preserved();
}

#[test]
fn original_apply_exact_decisions_expiry_drift_and_link_refusals_preserve_human_bytes() {
    for case in [
        "deny",
        "expire",
        "clock",
        "original",
        "worktree",
        "original-root",
        "worktree-root",
        "management",
        "symlink",
        "hardlink",
        "receipt",
    ] {
        let f = Fixture::new();
        let mut owner = f.owner();
        f.edit(&mut owner, "one.py", "repaired\n");
        let preview = f.prepare(&mut owner);
        let original = f.0.join("original/one.py");
        let outside = f.0.join("outside");
        match case {
            "original" => fs::write(&original, b"human changed selected file").unwrap(),
            "worktree" => {
                fs::write(f.0.join("owned/worktree/one.py"), b"changed after review").unwrap()
            }
            "original-root" => {
                fs::rename(f.0.join("original"), f.0.join("saved-original")).unwrap();
                fs::create_dir(f.0.join("original")).unwrap();
                fs::write(&original, b"replacement").unwrap();
            }
            "worktree-root" => {
                fs::rename(f.0.join("owned/worktree"), f.0.join("saved-worktree")).unwrap();
                fs::create_dir(f.0.join("owned/worktree")).unwrap();
            }
            "management" => {
                fs::rename(f.0.join("owned"), f.0.join("saved-owned")).unwrap();
                fs::create_dir_all(f.0.join("owned/worktree")).unwrap();
            }
            "symlink" | "hardlink" => {
                fs::write(&outside, b"outside human work").unwrap();
                fs::remove_file(&original).unwrap();
                if case == "symlink" {
                    symlink(&outside, &original).unwrap();
                } else {
                    fs::hard_link(&outside, &original).unwrap();
                }
            }
            "receipt" => {
                owner.created.as_mut().unwrap().creation_receipt_sha256 = hash(b"foreign creation")
            }
            _ => (),
        }
        let digest = preview["preview_sha256"].as_str().unwrap();
        let now = if case == "expire" {
            NOW + LIFETIME_MS
        } else if case == "clock" {
            NOW - 1
        } else {
            NOW
        };
        let result = owner.decide_apply(digest, case != "deny", now);
        if case == "deny" {
            assert_eq!(result.unwrap()["outcome"], "denied");
        } else {
            assert!(result.is_err(), "{case}");
        }
        let expected: &[u8] = match case {
            "original" => b"human changed selected file",
            "original-root" => b"replacement",
            "symlink" | "hardlink" => b"outside human work",
            _ => b"first\n",
        };
        assert_eq!(fs::read(&original).unwrap(), expected, "{case}");
        assert!(owner.inspect_apply().unwrap()["pending"].is_null());
        assert!(owner.decide_apply(digest, true, NOW).is_err());
    }
}

#[test]
fn later_worktree_edit_invalidates_original_consent_and_restart_restores_no_ownership() {
    let f = Fixture::new();
    let mut owner = f.owner();
    f.edit(&mut owner, "one.py", "repaired\n");
    let preview = f.prepare(&mut owner);
    f.edit(&mut owner, "two.py", "other repair\n");
    assert!(
        owner
            .decide_apply(preview["preview_sha256"].as_str().unwrap(), true, NOW)
            .is_err()
    );
    let preview = f.prepare(&mut owner);
    drop(owner);
    let mut owner = f.open();
    assert!(owner.inspect_apply().unwrap()["pending"].is_null());
    assert!(owner.inspect_apply().unwrap()["latest"].is_null());
    assert!(owner.inspect_apply().unwrap()["repository_root_sha256"].is_null());
    assert!(
        owner
            .prepare_current_apply(&["one.py".into()], "Refuse adoption", NOW)
            .is_err()
    );
    assert!(
        owner
            .decide_apply(preview["preview_sha256"].as_str().unwrap(), true, NOW)
            .is_err()
    );
    assert_eq!(fs::read(f.0.join("original/one.py")).unwrap(), b"first\n");
}

#[test]
fn original_apply_rechecks_all_original_preimages_before_any_multi_file_write() {
    let f = Fixture::new();
    let mut owner = f.owner();
    f.edit(&mut owner, "one.py", "repaired one\n");
    f.edit(&mut owner, "two.py", "repaired two\n");
    let preview = owner
        .prepare_current_apply(&["one.py".into(), "two.py".into()], "Apply both", NOW)
        .unwrap();
    fs::write(f.0.join("original/two.py"), b"concurrent human change").unwrap();
    assert!(
        owner
            .decide_apply(preview["preview_sha256"].as_str().unwrap(), true, NOW)
            .is_err()
    );
    assert_eq!(fs::read(f.0.join("original/one.py")).unwrap(), b"first\n");
    assert_eq!(
        fs::read(f.0.join("original/two.py")).unwrap(),
        b"concurrent human change"
    );
    f.assert_preserved();
}

#[test]
fn original_drift_before_preparation_and_broken_edit_lineage_cannot_be_transferred() {
    let f = Fixture::new();
    let mut owner = f.owner();
    f.edit(&mut owner, "one.py", "repair\n");
    fs::write(f.0.join("original/one.py"), b"human edit\n").unwrap();
    assert!(
        owner
            .prepare_current_apply(&["one.py".into()], "Refuse changed original", NOW)
            .is_err()
    );
    assert!(owner.inspect_apply().unwrap()["pending"].is_null());
    fs::write(
        f.0.join("owned/worktree/one.py"),
        b"foreign worktree edit\n",
    )
    .unwrap();
    let scope = owner.select_created_sources(&["one.py".into()]).unwrap();
    assert_eq!(
        owner
            .prepare_edit(
                scope,
                &[LinuxCodingReplacement {
                    path: "one.py",
                    before_sha256: &hash(b"foreign worktree edit\n"),
                    content: "new repair\n"
                }],
                "Refuse broken lineage",
                NOW
            )
            .unwrap_err()
            .code(),
        "code.edit.lineage"
    );
    assert_eq!(
        fs::read(f.0.join("original/one.py")).unwrap(),
        b"human edit\n"
    );
}

#[test]
fn transferring_original_invalidates_an_older_pending_worktree_edit() {
    let f = Fixture::new();
    let mut owner = f.owner();
    f.edit(&mut owner, "one.py", "repair\n");
    let scope = owner.select_created_sources(&["one.py".into()]).unwrap();
    let edit = owner
        .prepare_edit(
            scope,
            &[LinuxCodingReplacement {
                path: "one.py",
                before_sha256: &hash(b"repair\n"),
                content: "later repair\n",
            }],
            "Pending edit",
            NOW,
        )
        .unwrap();
    let preview = f.prepare(&mut owner);
    assert_eq!(
        owner
            .decide_apply(preview["preview_sha256"].as_str().unwrap(), true, NOW)
            .unwrap()["outcome"],
        "committed"
    );
    assert!(
        owner
            .decide_edit(edit["preview_sha256"].as_str().unwrap(), true, NOW)
            .is_err()
    );
    assert_eq!(fs::read(f.0.join("original/one.py")).unwrap(), b"repair\n");
}

#[test]
fn refused_worktree_write_dispatch_removes_old_transfer_lineage() {
    let f = Fixture::new();
    let mut owner = f.owner();
    f.edit(&mut owner, "one.py", "first repair\n");
    let apply = f.prepare(&mut owner);
    let scope = owner.select_created_sources(&["one.py".into()]).unwrap();
    let preview = owner
        .prepare_edit(
            scope,
            &[LinuxCodingReplacement {
                path: "one.py",
                before_sha256: &hash(b"first repair\n"),
                content: "second repair\n",
            }],
            "Pending repair",
            NOW,
        )
        .unwrap();
    fs::write(
        f.0.join("owned/worktree/one.py"),
        b"human change after preview\n",
    )
    .unwrap();
    assert!(
        owner
            .decide_edit(preview["preview_sha256"].as_str().unwrap(), true, NOW)
            .is_err()
    );
    assert!(
        owner
            .decide_apply(apply["preview_sha256"].as_str().unwrap(), true, NOW)
            .is_err()
    );
    assert_eq!(owner.inspect_apply().unwrap()["available_paths"], json!([]));
    assert!(
        owner
            .prepare_current_apply(&["one.py".into()], "Refuse old lineage", NOW)
            .is_err()
    );
    assert_eq!(fs::read(f.0.join("original/one.py")).unwrap(), b"first\n");
}

#[test]
fn transfer_lineage_is_bounded_across_disjoint_edit_batches() {
    let f = Fixture::new();
    let mut owner = f.owner();
    let paths = (0..17).map(|n| format!("file{n}.py")).collect::<Vec<_>>();
    for path in &paths {
        fs::write(f.0.join("owned/worktree").join(path), b"before\n").unwrap();
    }
    let scope = owner.select_created_sources(&paths[..16]).unwrap();
    let before = hash(b"before\n");
    let replacements = paths[..16]
        .iter()
        .map(|path| LinuxCodingReplacement {
            path,
            before_sha256: &before,
            content: "after\n",
        })
        .collect::<Vec<_>>();
    let preview = owner
        .prepare_edit(scope, &replacements, "Sixteen edits", NOW)
        .unwrap();
    assert_eq!(
        owner
            .decide_edit(preview["preview_sha256"].as_str().unwrap(), true, NOW)
            .unwrap()["outcome"],
        "committed"
    );
    let scope = owner.select_created_sources(&paths[16..]).unwrap();
    assert_eq!(
        owner
            .prepare_edit(
                scope,
                &[LinuxCodingReplacement {
                    path: &paths[16],
                    before_sha256: &before,
                    content: "after\n"
                }],
                "Refuse seventeenth",
                NOW
            )
            .unwrap_err()
            .code(),
        "code.edit.lineage-limit"
    );
    assert_eq!(
        owner.inspect_apply().unwrap()["available_paths"]
            .as_array()
            .unwrap()
            .len(),
        16
    );
    assert_eq!(
        fs::read(f.0.join("owned/worktree").join(&paths[16])).unwrap(),
        b"before\n"
    );
}

#[test]
fn original_read_consent_binds_both_roots_before_any_source_or_root_inspection() {
    let f = Fixture::new();
    let mut owner = f.owner();
    f.edit(&mut owner, "one.py", "repair\n");
    let preview = f.prepare(&mut owner);
    let status = owner.inspect_apply().unwrap();
    let original = status["repository_root_sha256"].as_str().unwrap();
    let tree = status["worktree_root_sha256"].as_str().unwrap();
    let receipts = owner.inspect().unwrap()["receipts"].clone();
    // Make the held root unavailable. A stale consent must refuse even before
    // root inspection (and before the deliberately ineligible source selection).
    fs::rename(f.0.join("original"), f.0.join("parked")).unwrap();
    for (expected_original, expected_tree) in [
        ("", tree.to_owned()),
        (original, "".into()),
        ("foreign", tree.to_owned()),
        (original, "foreign".into()),
        (tree, original.to_owned()),
    ] {
        assert_eq!(
            owner
                .prepare_apply(
                    &["unselected.py".into()],
                    "Refuse stale consent",
                    expected_original,
                    &expected_tree,
                    NOW
                )
                .unwrap_err()
                .code(),
            "code.apply.read-scope"
        );
        assert_eq!(owner.inspect_apply().unwrap()["pending"], preview);
        assert!(owner.inspect_apply().unwrap()["latest"].is_null());
        assert_eq!(owner.inspect().unwrap()["receipts"], receipts);
        assert_eq!(fs::read(f.0.join("parked/one.py")).unwrap(), b"first\n");
        assert_eq!(
            fs::read(f.0.join("owned/worktree/one.py")).unwrap(),
            b"repair\n"
        );
    }
    fs::rename(f.0.join("parked"), f.0.join("original")).unwrap();
    assert_eq!(
        owner
            .prepare_apply(&["one.py".into()], "Correct consent", original, tree, NOW)
            .unwrap()["operation"],
        "apply-to-original"
    );
    f.assert_preserved();
}

#[test]
fn replacing_the_created_pair_cannot_retarget_an_older_read_consent() {
    let f = Fixture::new();
    let other = Fixture::new();
    let mut owner = f.owner();
    let mut other_owner = other.owner();
    f.edit(&mut owner, "one.py", "first repair\n");
    other.edit(&mut other_owner, "one.py", "other repair\n");
    let status = owner.inspect_apply().unwrap();
    // Synthetic success injection stays inside this component test. The public
    // path can replace these handles only after separate native creation consent.
    owner.created = other_owner.created.take();
    let original = status["repository_root_sha256"].as_str().unwrap();
    let tree = status["worktree_root_sha256"].as_str().unwrap();
    assert_eq!(
        owner
            .prepare_apply(&["one.py".into()], "Older consent", original, tree, NOW)
            .unwrap_err()
            .code(),
        "code.apply.read-scope"
    );
    assert!(owner.inspect_apply().unwrap()["pending"].is_null());
    let fresh = owner.inspect_apply().unwrap();
    let preview = owner
        .prepare_apply(
            &["one.py".into()],
            "Fresh consent",
            fresh["repository_root_sha256"].as_str().unwrap(),
            fresh["worktree_root_sha256"].as_str().unwrap(),
            NOW,
        )
        .unwrap();
    assert_eq!(
        preview["repository_root_sha256"],
        fresh["repository_root_sha256"]
    );
    assert_eq!(
        preview["worktree_root_sha256"],
        fresh["worktree_root_sha256"]
    );
    assert_eq!(fs::read(f.0.join("original/one.py")).unwrap(), b"first\n");
    assert_eq!(
        fs::read(other.0.join("original/one.py")).unwrap(),
        b"first\n"
    );
    f.assert_preserved();
    other.assert_preserved();
}

fn runtime_lease(owner: LinuxCodingRepository) -> crate::LinuxCodingWorktreeLease {
    let identity = owner.inspect_edit().unwrap()["worktree_root_sha256"]
        .as_str()
        .unwrap()
        .to_owned();
    owner.into_runtime_worktree(&identity).unwrap()
}
fn recover_lease(lease: crate::LinuxCodingWorktreeLease) -> LinuxCodingRepository {
    match lease.try_into_repository() {
        Ok(owner) => owner,
        Err(_) => panic!("runtime references remain"),
    }
}
fn runtime_edit(
    f: &Fixture,
    lease: &crate::LinuxCodingWorktreeLease,
    content: &str,
    publish: bool,
) {
    let scope =
        LinuxCodingReadScope::select(&f.0.join("owned/worktree"), &["one.py".into()]).unwrap();
    let before = hash(&fs::read(f.0.join("owned/worktree/one.py")).unwrap());
    let pending = scope
        .prepare_write(
            &[LinuxCodingReplacement {
                path: "one.py",
                before_sha256: &before,
                content,
            }],
            "Native runtime fixture edit",
            NOW,
        )
        .unwrap();
    let result = pending
        .apply_with_owned_driver(&scope, lease, NOW, publish, None)
        .unwrap();
    assert_eq!(
        result.outcome,
        agentmage_kernel_engine::write_transaction::WriteTransactionOutcome::Committed
    );
}

#[test]
fn runtime_handoff_preserves_native_edit_lineage_and_separate_original_approval() {
    let started = Instant::now();
    let f = Fixture::new();
    let mut owner = f.owner();
    f.edit(&mut owner, "one.py", "manual first revision\n");
    let old_apply = f.prepare(&mut owner);
    let lease = runtime_lease(owner);
    lease.check_workspace(&f.0.join("owned/worktree")).unwrap();
    assert!(lease.check_workspace(&f.0.join("original")).is_err());
    let live = lease.clone();
    let lease = match lease.try_into_repository() {
        Err(lease) => lease,
        Ok(_) => panic!("adopted live runtime"),
    };
    runtime_edit(&f, &live, "runtime second revision\n", true);
    runtime_edit(&f, &live, "runtime final revision\n", true);
    assert_eq!(fs::read(f.0.join("original/one.py")).unwrap(), b"first\n");
    drop(live);
    let mut owner = recover_lease(lease);
    assert!(
        owner
            .decide_apply(old_apply["preview_sha256"].as_str().unwrap(), true, NOW)
            .is_err()
    );
    assert!(owner.inspect_apply().unwrap()["pending"].is_null());
    let preview = f.prepare(&mut owner);
    assert!(
        preview["write_preview"]["complete_diffs"][0]
            .as_str()
            .unwrap()
            .contains("first")
    );
    let digest = preview["preview_sha256"].as_str().unwrap();
    assert_eq!(
        owner.decide_apply(digest, true, NOW).unwrap()["outcome"],
        "committed"
    );
    assert!(owner.decide_apply(digest, true, NOW).is_err());
    assert_eq!(
        fs::read(f.0.join("original/one.py")).unwrap(),
        b"runtime final revision\n"
    );
    assert!(
        owner.inspect_apply().unwrap()["available_paths"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    f.assert_preserved();
    eprintln!(
        "owned_runtime_native_writes_and_original_apply_us={}",
        started.elapsed().as_micros()
    );
}

#[test]
fn runtime_handoff_cannot_adopt_reopened_owner_wrong_root_or_substituted_worktree() {
    let f = Fixture::new();
    assert!(f.open().into_runtime_worktree(&"0".repeat(64)).is_err());
    assert!(f.owner().into_runtime_worktree(&"0".repeat(64)).is_err());
    let lease = runtime_lease(f.owner());
    let root = f.0.join("owned/worktree");
    fs::rename(&root, f.0.join("owned/old-worktree")).unwrap();
    fs::create_dir(&root).unwrap();
    fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
    assert!(lease.check_workspace(&root).is_err());
    let mut owner = recover_lease(lease);
    assert!(
        owner
            .prepare_current_apply(&["one.py".into()], "Refuse replacement", NOW)
            .is_err()
    );
    assert_eq!(fs::read(f.0.join("original/one.py")).unwrap(), b"first\n");
}

#[test]
fn runtime_native_write_without_terminal_publication_clears_transfer_lineage() {
    let f = Fixture::new();
    let mut owner = f.owner();
    f.edit(&mut owner, "one.py", "manual revision\n");
    let lease = runtime_lease(owner);
    runtime_edit(&f, &lease, "unpublished runtime revision\n", false);
    let mut owner = recover_lease(lease);
    assert_eq!(
        fs::read(f.0.join("owned/worktree/one.py")).unwrap(),
        b"unpublished runtime revision\n"
    );
    assert!(
        owner
            .prepare_current_apply(&["one.py".into()], "Refuse unknown publication", NOW)
            .is_err()
    );
    assert!(
        owner.inspect_apply().unwrap()["available_paths"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    f.assert_preserved();
}

#[test]
fn runtime_driver_refuses_foreign_root_and_concurrent_write_owner() {
    let f = Fixture::new();
    let lease = runtime_lease(f.owner());
    assert!(
        lease
            .write_driver(
                &held_root(&f.0.join("original")).unwrap(),
                crate::LinuxAtomicWriteDriverLimits::default()
            )
            .is_err()
    );
    let root = held_root(&f.0.join("owned/worktree")).unwrap();
    let driver = lease
        .write_driver(&root, crate::LinuxAtomicWriteDriverLimits::default())
        .unwrap();
    assert!(
        lease
            .write_driver(&root, crate::LinuxAtomicWriteDriverLimits::default())
            .is_err()
    );
    assert!(lease.check_workspace(&f.0.join("owned/worktree")).is_err());
    drop(driver);
    lease.check_workspace(&f.0.join("owned/worktree")).unwrap();
    let owner = recover_lease(lease);
    assert!(
        owner.inspect_apply().unwrap()["available_paths"]
            .as_array()
            .unwrap()
            .is_empty()
    );
}

#[test]
fn runtime_native_write_refuses_broken_lineage_and_preserves_foreign_bytes() {
    let f = Fixture::new();
    let mut owner = f.owner();
    f.edit(&mut owner, "one.py", "manual revision\n");
    let lease = runtime_lease(owner);
    fs::write(f.0.join("owned/worktree/one.py"), b"foreign edit\n").unwrap();
    let scope =
        LinuxCodingReadScope::select(&f.0.join("owned/worktree"), &["one.py".into()]).unwrap();
    let pending = scope
        .prepare_write(
            &[LinuxCodingReplacement {
                path: "one.py",
                before_sha256: &hash(b"foreign edit\n"),
                content: "must not write\n",
            }],
            "Refuse foreign lineage",
            NOW,
        )
        .unwrap();
    let result = pending
        .apply_with_owned_driver(&scope, &lease, NOW, true, None)
        .unwrap();
    assert_eq!(
        result.outcome,
        agentmage_kernel_engine::write_transaction::WriteTransactionOutcome::FailedNoChange
    );
    assert_eq!(
        fs::read(f.0.join("owned/worktree/one.py")).unwrap(),
        b"foreign edit\n"
    );
    let mut owner = recover_lease(lease);
    assert!(
        owner
            .prepare_current_apply(&["one.py".into()], "Refuse foreign transfer", NOW)
            .is_err()
    );
    f.assert_preserved();
}

#[test]
fn runtime_completion_refuses_other_transaction_grant_and_receipt_chain() {
    use agentmage_kernel_engine::write_transaction::WriteTransactionResult;
    let corruptions: [fn(&mut WriteTransactionResult); 3] = [
        |r| r.transaction_id = "foreign-transaction".into(),
        |r| r.grant_consumption.consumed_grant_sha256 = "0".repeat(64),
        |r| r.receipts.last_mut().unwrap().receipt_sha256 = "0".repeat(64),
    ];
    for corrupt in corruptions {
        let f = Fixture::new();
        let lease = runtime_lease(f.owner());
        let scope =
            LinuxCodingReadScope::select(&f.0.join("owned/worktree"), &["one.py".into()]).unwrap();
        let pending = scope
            .prepare_write(
                &[LinuxCodingReplacement {
                    path: "one.py",
                    before_sha256: &hash(b"first\n"),
                    content: "actual native postimage\n",
                }],
                "Refuse unrelated completion",
                NOW,
            )
            .unwrap();
        assert!(
            pending
                .apply_with_owned_driver(&scope, &lease, NOW, true, Some(corrupt))
                .is_err()
        );
        assert_eq!(
            fs::read(f.0.join("owned/worktree/one.py")).unwrap(),
            b"actual native postimage\n"
        );
        let mut owner = recover_lease(lease);
        assert!(
            owner
                .prepare_current_apply(&["one.py".into()], "Refuse altered completion", NOW)
                .is_err()
        );
        assert!(
            owner.inspect_apply().unwrap()["available_paths"]
                .as_array()
                .unwrap()
                .is_empty()
        );
        assert_eq!(fs::read(f.0.join("original/one.py")).unwrap(), b"first\n");
    }
}

#[test]
fn runtime_poisoned_owner_recovers_without_creation_authority_or_pending_approvals() {
    let f = Fixture::new();
    let mut owner = f.owner();
    f.edit(&mut owner, "one.py", "manual revision\n");
    let old_preview = f.prepare(&mut owner);
    let lease = runtime_lease(owner);
    let other = lease.clone();
    let root = f.0.join("owned/worktree");
    assert!(
        std::thread::spawn(move || {
            let held = held_root(&root).unwrap();
            let _driver = other
                .write_driver(&held, crate::LinuxAtomicWriteDriverLimits::default())
                .unwrap();
            panic!("synthetic native execution panic while ownership held");
        })
        .join()
        .is_err()
    );
    assert!(lease.check_workspace(&f.0.join("owned/worktree")).is_err());
    let mut owner = recover_lease(lease);
    assert!(owner.inspect_apply().unwrap()["repository_root_sha256"].is_null());
    assert!(owner.inspect_apply().unwrap()["pending"].is_null());
    assert!(
        owner
            .decide_apply(old_preview["preview_sha256"].as_str().unwrap(), true, NOW)
            .is_err()
    );
    assert!(owner.select_created_sources(&["one.py".into()]).is_err());
    assert_eq!(fs::read(f.0.join("original/one.py")).unwrap(), b"first\n");
    f.assert_preserved();
}
