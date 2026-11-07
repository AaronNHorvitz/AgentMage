// SPDX-License-Identifier: Apache-2.0
//! Real filesystem controls with synthetic Git layout; no executable trust bypass.

use super::*;
use std::fs;
use std::os::unix::fs::{PermissionsExt, symlink};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

struct Fixture {
    base: PathBuf,
    root: PathBuf,
    git: PathBuf,
    common: PathBuf,
}
impl Fixture {
    fn new() -> Self {
        let base = std::env::temp_dir().join(format!(
            "agentmage-linked-metadata-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let root = base.join("owned/worktree");
        let common = base.join("original/.git");
        let git = common.join("worktrees/worktree");
        for path in [&root, &git] {
            fs::create_dir_all(path).unwrap();
        }
        for path in [
            &base,
            &base.join("owned"),
            &root,
            &base.join("original"),
            &common,
            &common.join("worktrees"),
            &git,
        ] {
            fs::set_permissions(path, fs::Permissions::from_mode(0o700)).unwrap();
        }
        fs::write(root.join(".git"), format!("gitdir: {}\n", git.display())).unwrap();
        fs::write(git.join("commondir"), "../..\n").unwrap();
        fs::write(
            git.join("gitdir"),
            format!("{}\n", root.join(".git").display()),
        )
        .unwrap();
        fs::write(
            git.join("HEAD"),
            "1234567890123456789012345678901234567890\n",
        )
        .unwrap();
        Self {
            base,
            root,
            git,
            common,
        }
    }
    fn verify(&self) -> LinuxLinkedRepository {
        LinuxLinkedRepository::verify(&self.root, &self.git, &self.common).unwrap()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.base).unwrap();
    }
}

#[test]
fn exact_linked_layout_binds_roots_and_pointer_objects_but_not_head_contents() {
    let f = Fixture::new();
    let binding = f.verify();
    assert_eq!(binding.worktree_root(), f.root);
    assert_eq!(binding.git_directory(), f.git);
    assert_eq!(binding.common_directory(), f.common);
    assert_eq!(binding.identity_sha256().len(), 64);
    binding.revalidate().unwrap();
    assert!(binding.matches_root(&binding.root.fd));
    assert!(!binding.matches_root(&binding.git.fd));
    fs::write(f.git.join("HEAD"), "ref: refs/heads/agentmage/task\n").unwrap();
    binding.revalidate().unwrap();
    let old = f.root.join("old-pointer");
    fs::rename(f.root.join(".git"), &old).unwrap();
    fs::write(f.root.join(".git"), fs::read(&old).unwrap()).unwrap();
    assert!(
        binding.revalidate().is_err(),
        "identical replacement cannot inherit identity"
    );
}

#[test]
fn pointers_refuse_aliases_relative_targets_wrong_backlinks_and_oversize() {
    let f = Fixture::new();
    let pointer = fs::read(f.root.join(".git")).unwrap();
    let alias = f.base.join("alias");
    symlink(&f.common, &alias).unwrap();
    for candidate in [
        "gitdir: ../../original/.git/worktrees/worktree\n".to_owned(),
        format!("gitdir: {}/worktrees/worktree\n", alias.display()),
        format!("gitdir: {}\nextra\n", f.git.display()),
        format!("gitdir: {}/../worktree\n", f.git.display()),
        "x".repeat(4097),
    ] {
        fs::write(f.root.join(".git"), candidate).unwrap();
        assert!(LinuxLinkedRepository::verify(&f.root, &f.git, &f.common).is_err());
    }
    fs::write(f.root.join(".git"), &pointer).unwrap();
    for candidate in ["../../outside\n", "../../\n", "../..\nextra\n"] {
        fs::write(f.git.join("commondir"), candidate).unwrap();
        assert!(LinuxLinkedRepository::verify(&f.root, &f.git, &f.common).is_err());
    }
    fs::write(f.git.join("commondir"), "../..\n").unwrap();
    fs::write(
        f.git.join("gitdir"),
        format!("{}\n", f.base.join("other/.git").display()),
    )
    .unwrap();
    assert!(LinuxLinkedRepository::verify(&f.root, &f.git, &f.common).is_err());
}

#[test]
fn metadata_refuses_symlinks_hardlinks_unsafe_modes_and_replacements() {
    for name in [".git", "commondir", "gitdir", "HEAD"] {
        let f = Fixture::new();
        let binding = f.verify();
        let path = if name == ".git" {
            f.root.join(name)
        } else {
            f.git.join(name)
        };
        let saved = f.base.join("saved");
        fs::rename(&path, &saved).unwrap();
        symlink(&saved, &path).unwrap();
        assert!(binding.revalidate().is_err(), "symlink {name}");
        fs::remove_file(&path).unwrap();
        fs::hard_link(&saved, &path).unwrap();
        assert!(binding.revalidate().is_err(), "hardlink {name}");
        fs::remove_file(&path).unwrap();
        fs::rename(&saved, &path).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o666)).unwrap();
        assert!(binding.revalidate().is_err(), "writable {name}");
    }
    for dir in ["root", "git", "common"] {
        let f = Fixture::new();
        let binding = f.verify();
        let path = match dir {
            "root" => &f.root,
            "git" => &f.git,
            _ => &f.common,
        };
        fs::set_permissions(path, fs::Permissions::from_mode(0o777)).unwrap();
        assert!(binding.revalidate().is_err(), "writable {dir}");
        fs::set_permissions(path, fs::Permissions::from_mode(0o700)).unwrap();
        binding.revalidate().unwrap();
        let saved = path.with_extension("saved");
        fs::rename(path, &saved).unwrap();
        fs::create_dir(path).unwrap();
        fs::set_permissions(path, fs::Permissions::from_mode(0o700)).unwrap();
        assert!(binding.revalidate().is_err(), "replacement {dir}");
    }
}

#[test]
fn linked_inventory_scope_is_distinct_from_mutating_repository_scope() {
    let f = Fixture::new();
    let binding = f.verify();
    let state = f.base.join("state");
    fs::create_dir(&state).unwrap();
    fs::set_permissions(&state, fs::Permissions::from_mode(0o700)).unwrap();
    assert!(crate::LinuxRepositoryScope::verify(&f.root, &f.git, &state).is_err());
    let scope = crate::LinuxRepositoryScope::verify_linked_read_only(&binding, &state).unwrap();
    assert_eq!(scope.git_directory(), f.git);
    for overlap in [&f.root, &f.common, &f.git, &f.base] {
        assert!(crate::LinuxRepositoryScope::verify_linked_read_only(&binding, overlap).is_err());
    }
    fs::write(f.git.join("commondir"), "../outside\n").unwrap();
    assert!(crate::LinuxRepositoryScope::verify_linked_read_only(&binding, &state).is_err());
}
