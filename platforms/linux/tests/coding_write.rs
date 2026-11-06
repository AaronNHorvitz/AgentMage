// SPDX-License-Identifier: Apache-2.0
//! Real filesystem mutations through exact kernel approval and native driver.

use std::fs;
use std::os::unix::fs::{MetadataExt, PermissionsExt, symlink};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use agentmage_platform_linux::{
    LinuxCodingPendingWrite, LinuxCodingReadScope, LinuxCodingReplacement, LinuxCodingWriteError,
};
use sha2::{Digest, Sha256};

static NEXT: AtomicU64 = AtomicU64::new(0);
const BEFORE: &str = "def add(a, b):\n    return a - b\n";
const AFTER: &str = "def add(a, b):\n    return a + b\n";

fn sha(text: &str) -> String {
    Sha256::digest(text.as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

struct Files(PathBuf);
impl Files {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "agentmage-code-write-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
        fs::write(root.join("calc.py"), BEFORE).unwrap();
        fs::set_permissions(root.join("calc.py"), fs::Permissions::from_mode(0o640)).unwrap();
        Self(root)
    }
    fn scope(&self) -> LinuxCodingReadScope {
        LinuxCodingReadScope::select(&self.0, &["calc.py".into()]).unwrap()
    }
    fn prepare(&self, scope: &LinuxCodingReadScope) -> LinuxCodingPendingWrite {
        scope
            .prepare_write(
                &[LinuxCodingReplacement {
                    path: "calc.py",
                    before_sha256: &sha(BEFORE),
                    content: AFTER,
                }],
                "Fix add",
                1_000,
            )
            .unwrap()
    }
    fn bytes(&self) -> String {
        fs::read_to_string(self.0.join("calc.py")).unwrap()
    }
}
impl Drop for Files {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

#[test]
fn prepare_is_inert_and_apply_retains_mode_and_exact_verified_receipts() {
    let files = Files::new();
    let scope = files.scope();
    let pending = files.prepare(&scope);
    assert_eq!(files.bytes(), BEFORE);
    let preview = pending.preview().clone();
    assert_eq!(preview.files, ["calc.py"]);
    assert!(preview.complete_diffs[0].contains("return a - b"));
    assert!(preview.complete_diffs[0].contains("return a + b"));
    let result = pending
        .apply(
            &scope,
            &preview.change_set_sha256,
            &preview.preview_sha256,
            2_000,
        )
        .unwrap();
    assert_eq!(result["outcome"], "committed");
    assert_eq!(result["tested"], false);
    assert_eq!(result["isolation_verified"], false);
    let states = result["receipts"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["status"].as_str().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(states, ["proposed", "approved", "applied", "verified"]);
    assert_eq!(files.bytes(), AFTER);
    assert_eq!(
        fs::metadata(files.0.join("calc.py")).unwrap().mode() & 0o777,
        0o640
    );
    assert_eq!(fs::read_dir(&files.0).unwrap().count(), 1);
}

#[test]
fn stale_contents_and_same_bytes_replacement_inode_cannot_apply() {
    for same_bytes in [false, true] {
        let files = Files::new();
        let scope = files.scope();
        let pending = files.prepare(&scope);
        let preview = pending.preview().clone();
        let person = if same_bytes {
            BEFORE
        } else {
            "person's edit\n"
        };
        fs::write(files.0.join("new"), person).unwrap();
        fs::rename(files.0.join("new"), files.0.join("calc.py")).unwrap();
        assert_eq!(
            pending
                .apply(
                    &scope,
                    &preview.change_set_sha256,
                    &preview.preview_sha256,
                    2_000
                )
                .err(),
            Some(LinuxCodingWriteError::Transaction)
        );
        assert_eq!(files.bytes(), person);
    }
}

#[test]
fn approval_digests_expiry_wrong_root_and_dropped_preview_never_write() {
    let files = Files::new();
    let scope = files.scope();
    drop(files.prepare(&scope));
    for (wrong_change, wrong_preview, time) in [
        (true, false, 2_000),
        (false, true, 2_000),
        (false, false, 121_000),
        (false, false, 999),
    ] {
        let pending = files.prepare(&scope);
        let preview = pending.preview().clone();
        assert_eq!(
            pending
                .apply(
                    &scope,
                    if wrong_change {
                        "wrong"
                    } else {
                        &preview.change_set_sha256
                    },
                    if wrong_preview {
                        "wrong"
                    } else {
                        &preview.preview_sha256
                    },
                    time
                )
                .err(),
            Some(LinuxCodingWriteError::Approval)
        );
        assert_eq!(files.bytes(), BEFORE);
    }
    let other = Files::new();
    let pending = files.prepare(&scope);
    let preview = pending.preview().clone();
    assert_eq!(
        pending
            .apply(
                &other.scope(),
                &preview.change_set_sha256,
                &preview.preview_sha256,
                2_000
            )
            .err(),
        Some(LinuxCodingWriteError::Approval)
    );
    assert_eq!(files.bytes(), BEFORE);
    assert_eq!(other.bytes(), BEFORE);
}

#[test]
fn symlink_and_hardlink_substitutions_do_not_touch_outside_objects() {
    for hard in [false, true] {
        let files = Files::new();
        let outside = Files::new();
        let scope = files.scope();
        let pending = files.prepare(&scope);
        let preview = pending.preview().clone();
        fs::remove_file(files.0.join("calc.py")).unwrap();
        if hard {
            fs::hard_link(outside.0.join("calc.py"), files.0.join("calc.py")).unwrap();
        } else {
            symlink(outside.0.join("calc.py"), files.0.join("calc.py")).unwrap();
        }
        assert_eq!(
            pending
                .apply(
                    &scope,
                    &preview.change_set_sha256,
                    &preview.preview_sha256,
                    2_000
                )
                .err(),
            Some(LinuxCodingWriteError::Transaction)
        );
        assert_eq!(outside.bytes(), BEFORE);
    }
}

#[test]
fn multi_file_preflight_is_whole_batch_and_malformed_replacements_are_inert() {
    let files = Files::new();
    fs::write(files.0.join("second.py"), BEFORE).unwrap();
    let scope =
        LinuxCodingReadScope::select(&files.0, &["calc.py".into(), "second.py".into()]).unwrap();
    let digest = sha(BEFORE);
    let edits = ["calc.py", "second.py"].map(|path| LinuxCodingReplacement {
        path,
        before_sha256: &digest,
        content: AFTER,
    });
    let pending = scope.prepare_write(&edits, "Fix both", 1_000).unwrap();
    let preview = pending.preview().clone();
    fs::write(files.0.join("second.py"), "person's second edit").unwrap();
    assert_eq!(
        pending
            .apply(
                &scope,
                &preview.change_set_sha256,
                &preview.preview_sha256,
                2_000
            )
            .err(),
        Some(LinuxCodingWriteError::Transaction)
    );
    assert_eq!(files.bytes(), BEFORE);
    assert_eq!(
        fs::read_to_string(files.0.join("second.py")).unwrap(),
        "person's second edit"
    );
    for (path, before, after) in [
        ("../calc.py", digest.as_str(), AFTER),
        ("calc.py", "bad", AFTER),
        ("calc.py", digest.as_str(), BEFORE),
        ("calc.py", digest.as_str(), "\0"),
    ] {
        assert!(
            scope
                .prepare_write(
                    &[LinuxCodingReplacement {
                        path,
                        before_sha256: before,
                        content: after
                    }],
                    "Fix",
                    1_000
                )
                .is_err()
        );
        assert_eq!(files.bytes(), BEFORE);
    }
}
