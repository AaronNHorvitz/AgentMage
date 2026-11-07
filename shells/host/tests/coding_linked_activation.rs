// SPDX-License-Identifier: Apache-2.0
//! Marker command contracts over synthetic filesystem layouts; no model/worker.
#![cfg(target_os = "linux")]

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "agentmage-marker-command-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
        Self(path)
    }
    fn run(&self) -> std::process::Output {
        Command::new(env!("CARGO_BIN_EXE_agentmage-host"))
            .arg("--coding-development-marker")
            .arg(self.0.join("worktree"))
            .output()
            .unwrap()
    }
    fn run_linked(&self, git: &std::path::Path, common: &std::path::Path) -> std::process::Output {
        Command::new(env!("CARGO_BIN_EXE_agentmage-host"))
            .arg("--coding-development-linked-marker")
            .arg(self.0.join("worktree"))
            .arg(git)
            .arg(common)
            .output()
            .unwrap()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

#[test]
fn marker_command_only_displays_exact_ordinary_or_linked_setup_bytes() {
    let f = Fixture::new();
    let root = f.0.join("worktree");
    fs::create_dir(&root).unwrap();
    fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
    fs::create_dir(root.join(".git")).unwrap();
    fs::write(root.join(".git/HEAD"), "ref: refs/heads/main\n").unwrap();
    let expected =
        agentmage_host::coding_development_activation::coding_development_marker(&root).unwrap();
    let output = f.run();
    assert!(output.status.success());
    assert_eq!(output.stdout, expected);
    assert!(output.stderr.is_empty());
    assert!(!root.join(".agentmage-development-workspace").exists());
    fs::remove_dir_all(root.join(".git")).unwrap();
    let git = f.0.join("original/.git/worktrees/task");
    fs::create_dir_all(&git).unwrap();
    fs::write(root.join(".git"), format!("gitdir: {}\n", git.display())).unwrap();
    fs::write(git.join("commondir"), "../..\n").unwrap();
    fs::write(
        git.join("gitdir"),
        format!("{}\n", root.join(".git").display()),
    )
    .unwrap();
    fs::write(git.join("HEAD"), "ref: refs/heads/agentmage/task\n").unwrap();
    let output = f.run();
    assert!(
        !output.status.success(),
        "ordinary marker command must not infer external scope"
    );
    assert!(output.stdout.is_empty());
    let common = f.0.join("original/.git");
    let expected = agentmage_host::coding_development_activation::coding_development_linked_marker(
        &root, &git, &common,
    )
    .unwrap();
    let output = f.run_linked(&git, &common);
    assert!(output.status.success());
    assert_eq!(output.stdout, expected);
    assert!(output.stderr.is_empty());
    assert!(!root.join(".agentmage-development-workspace").exists());
    fs::write(git.join("commondir"), "../../outside\n").unwrap();
    let output = f.run_linked(&git, &common);
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("coding.development.git-worktree-denied")
    );
}
