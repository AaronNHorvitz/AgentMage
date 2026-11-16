// SPDX-License-Identifier: MIT OR Apache-2.0
//! Actual local reads and replacement races; no native Git admission claimed.

use super::*;
use agentmage_kernel_contracts::{
    BoundaryKind, CONTRACT_SCHEMA_VERSION, CancellationId, CancellationReason, CancellationSignal,
    CorrelationId, TaskId,
};
use std::cell::Cell;
use std::os::unix::fs::{PermissionsExt, symlink};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);
struct Fixture {
    root: PathBuf,
    scope: LinuxRepositoryScope,
}
impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "agentmage-preservation-fs-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        for name in ["repo", "repo/.git", "owned", "outside"] {
            fs::create_dir(root.join(name)).unwrap();
            fs::set_permissions(root.join(name), fs::Permissions::from_mode(0o700)).unwrap();
        }
        fs::write(root.join("outside/secret"), b"outside sentinel").unwrap();
        let scope = LinuxRepositoryScope::verify(
            root.join("repo"),
            root.join("repo/.git"),
            root.join("owned"),
        )
        .unwrap();
        Self { root, scope }
    }
    fn files(&self) -> Files<'_> {
        Files::new(&self.scope, None).unwrap()
    }
    fn unchanged_outside(&self) {
        assert_eq!(
            fs::read(self.root.join("outside/secret")).unwrap(),
            b"outside sentinel"
        );
        assert_eq!(fs::read_dir(self.root.join("owned")).unwrap().count(), 0);
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
        TaskId::from_raw("preservation-fs"),
        CorrelationId::from_raw("preservation-fs"),
    )
}
fn cancel(token: &CancellationToken) {
    token
        .cancel(CancellationSignal {
            schema_version: CONTRACT_SCHEMA_VERSION,
            cancellation_id: CancellationId::from_raw("preservation-fs-stop"),
            correlation_id: CorrelationId::from_raw("preservation-fs"),
            task_id: TaskId::from_raw("preservation-fs"),
            reason: CancellationReason::UserRequested,
            requested_by: BoundaryKind::Shell,
        })
        .unwrap();
}
fn denied<T>(result: Result<T, LinuxRepositoryError>) {
    match result {
        Err(e) => assert_eq!(e.kind(), LinuxRepositoryErrorKind::ObservationFailed),
        Ok(_) => panic!("unsafe scan accepted"),
    }
}
fn metadata_record(path: &Path, relative: &str) -> Vec<u8> {
    let stat = fs::symlink_metadata(path).unwrap();
    let mut record = relative.as_bytes().to_vec();
    record.extend_from_slice(&stat.mode().to_be_bytes());
    record.extend_from_slice(&stat.len().to_be_bytes());
    record.extend_from_slice(&stat.mtime().to_be_bytes());
    record
}

#[test]
fn preservation_fs_keeps_exact_content_metadata_link_and_reflog_bindings() {
    let f = Fixture::new();
    let repo = f.root.join("repo");
    for dir in [
        "nested",
        ".git/hooks",
        ".git/logs",
        ".git/logs/refs",
        ".git/logs/refs/heads",
        ".git/logs/refs/heads/agentmage",
    ] {
        fs::create_dir(repo.join(dir)).unwrap();
    }
    fs::write(repo.join("nested/source"), b"original content").unwrap();
    fs::write(repo.join(".gitattributes"), b"*.txt text\n").unwrap();
    fs::write(repo.join(".git/index"), b"original index").unwrap();
    fs::write(repo.join(".git/hooks/example.sample"), b"hook sample").unwrap();
    fs::set_permissions(
        repo.join(".git/hooks/example.sample"),
        fs::Permissions::from_mode(0o700),
    )
    .unwrap();
    fs::write(repo.join(".git/logs/HEAD"), b"user reflog").unwrap();
    fs::write(
        repo.join(".git/logs/refs/heads/agentmage/task"),
        b"task reflog",
    )
    .unwrap();
    fs::write(repo.join(".git/MERGE_HEAD"), b"pending").unwrap();
    symlink(f.root.join("outside/secret"), repo.join("link")).unwrap();
    let files = f.files();
    let mut expected = Vec::new();
    let mut directory = metadata_record(&repo.join("nested"), "nested");
    directory.push(b'd');
    expected.push(directory);
    for (path, content) in [
        ("nested/source", b"original content".as_slice()),
        (".gitattributes", b"*.txt text\n".as_slice()),
    ] {
        let mut record = metadata_record(&repo.join(path), path);
        record.push(b'f');
        record.extend_from_slice(hash(content).as_bytes());
        expected.push(record);
    }
    let mut link = metadata_record(&repo.join("link"), "link");
    link.push(b'l');
    link.extend_from_slice(f.root.join("outside/secret").as_os_str().as_bytes());
    expected.push(link);
    expected.sort();
    assert_eq!(files.worktree().unwrap(), expected.concat());
    assert_eq!(
        files.hash_optional(true, "index").unwrap(),
        hash(b"original index")
    );
    assert_eq!(
        files.hash_optional(false, ".gitmodules").unwrap(),
        hash(b"absent")
    );
    assert_eq!(
        files.metadata("hooks").unwrap(),
        metadata_record(&repo.join(".git/hooks/example.sample"), "example.sample")
    );
    assert_eq!(
        files.reflogs("logs", true).unwrap(),
        [b"HEAD".as_slice(), hash(b"user reflog").as_bytes()].concat()
    );
    assert_eq!(
        files.reflogs("logs/refs/heads/agentmage", false).unwrap(),
        [b"task".as_slice(), hash(b"task reflog").as_bytes()].concat()
    );
    let stat = fs::metadata(repo.join(".git/MERGE_HEAD")).unwrap();
    assert_eq!(
        files.operations().unwrap(),
        [
            b"MERGE_HEAD".as_slice(),
            &stat.mode().to_be_bytes(),
            &stat.len().to_be_bytes()
        ]
        .concat()
    );
    assert_eq!(
        files.common_identity().unwrap(),
        path_identity(&f.scope.git_directory).unwrap()
    );
    assert!(!files.hooks().unwrap());
    assert!(!files.exists("objects/info/alternates").unwrap());
    fs::write(repo.join(".git/hooks/active"), b"hook").unwrap();
    fs::set_permissions(
        repo.join(".git/hooks/active"),
        fs::Permissions::from_mode(0o700),
    )
    .unwrap();
    assert!(files.hooks().unwrap());
    files.revalidate().unwrap();
    f.unchanged_outside();
}

#[test]
fn preservation_fs_replacement_before_open_refuses_links_files_and_fifo_without_blocking() {
    let f = Fixture::new();
    let files = f.files();
    let path = f.root.join("repo/source");
    for replacement in ["symlink", "regular", "fifo"] {
        fs::write(&path, b"approved bytes").unwrap();
        let held = entry(&files.checkout, "source").unwrap().unwrap();
        let stat = fstat(&held).unwrap();
        denied(read_regular(
            &files.checkout,
            "source",
            &stat,
            || Ok(()),
            || {
                fs::remove_file(&path).unwrap();
                match replacement {
                    "symlink" => symlink(f.root.join("outside/secret"), &path).unwrap(),
                    "regular" => fs::write(&path, b"replacement bytes").unwrap(),
                    _ => rustix::fs::mkfifoat(&files.checkout, "source", Mode::RUSR | Mode::WUSR)
                        .unwrap(),
                }
                Ok(())
            },
        ));
        fs::remove_file(&path).unwrap();
        f.unchanged_outside();
    }
}

#[test]
fn preservation_fs_growing_shrinking_and_same_size_rewrites_are_not_certified() {
    let f = Fixture::new();
    let files = f.files();
    let path = f.root.join("repo/source");
    for changed in [
        vec![b'y'; HASH_BUFFER_BYTES * 3],
        vec![b'y'; HASH_BUFFER_BYTES / 2],
        vec![b'y'; HASH_BUFFER_BYTES * 2],
    ] {
        fs::write(&path, vec![b'x'; HASH_BUFFER_BYTES * 2]).unwrap();
        let held = entry(&files.checkout, "source").unwrap().unwrap();
        let stat = fstat(&held).unwrap();
        let checks = Cell::new(0);
        denied(read_regular(
            &files.checkout,
            "source",
            &stat,
            || {
                checks.set(checks.get() + 1);
                if checks.get() == 3 {
                    fs::write(&path, &changed).unwrap();
                }
                Ok(())
            },
            || Ok(()),
        ));
        assert!(checks.get() >= 3);
    }
    // Oversized files refuse before opening/allocating/reading their content.
    fs::File::create(&path)
        .unwrap()
        .set_len(MAX_HASHED_FILE_BYTES + 1)
        .unwrap();
    let held = entry(&files.checkout, "source").unwrap().unwrap();
    let opened = Cell::new(false);
    denied(read_regular(
        &files.checkout,
        "source",
        &fstat(&held).unwrap(),
        || Ok(()),
        || {
            opened.set(true);
            Ok(())
        },
    ));
    assert!(!opened.get());
    f.unchanged_outside();
}

#[test]
fn preservation_fs_cancellation_stops_chunk_reads_and_all_local_scan_entries() {
    let f = Fixture::new();
    fs::write(
        f.root.join("repo/source"),
        vec![b'x'; HASH_BUFFER_BYTES * 2],
    )
    .unwrap();
    let token = token();
    let files = Files::new(&f.scope, Some(&token)).unwrap();
    let held = entry(&files.checkout, "source").unwrap().unwrap();
    let checks = Cell::new(0);
    denied(read_regular(
        &files.checkout,
        "source",
        &fstat(&held).unwrap(),
        || {
            checks.set(checks.get() + 1);
            if checks.get() == 3 {
                cancel(&token);
            }
            files.check()
        },
        || Ok(()),
    ));
    assert_eq!(checks.get(), 3, "no next chunk after caller cancellation");
    denied(files.worktree());
    denied(files.metadata("missing"));
    denied(files.reflogs("missing", true));
    denied(files.read_optional(false, "missing"));
    denied(files.hash_optional(true, "index"));
    denied(files.operations());
    denied(files.hooks());
    denied(files.exists("shallow"));
    denied(files.common_identity());
    denied(files.revalidate());
    denied(Files::new(&f.scope, Some(&token)));
    f.unchanged_outside();
}

#[test]
fn preservation_fs_shared_deadline_stops_chunks_and_every_scan_entry_without_cancelling() {
    let f = Fixture::new();
    fs::write(
        f.root.join("repo/source"),
        vec![b'x'; HASH_BUFFER_BYTES * 2],
    )
    .unwrap();
    let token = token();
    let deadline = Instant::now() + Duration::from_secs(1);
    let files = Files::new_until(&f.scope, Some(&token), deadline).unwrap();
    let held = entry(&files.checkout, "source").unwrap().unwrap();
    let checks = Cell::new(0);
    denied(read_regular(
        &files.checkout,
        "source",
        &fstat(&held).unwrap(),
        || {
            checks.set(checks.get() + 1);
            if checks.get() == 3 {
                // Initial check, first chunk precheck, first chunk postcheck.
                // Stop after reading a real chunk, before the next one.
                std::thread::sleep(deadline.saturating_duration_since(Instant::now()));
            }
            files.check()
        },
        || Ok(()),
    ));
    assert_eq!(checks.get(), 3);
    denied(files.worktree());
    denied(files.metadata("missing"));
    denied(files.reflogs("missing", true));
    denied(files.read_optional(false, "missing"));
    denied(files.hash_optional(true, "index"));
    denied(files.operations());
    denied(files.hooks());
    denied(files.exists("shallow"));
    denied(files.common_identity());
    denied(files.revalidate());
    denied(Files::new_until(&f.scope, Some(&token), deadline));
    assert!(!token.is_cancelled());
    assert_eq!(
        fs::metadata(f.root.join("repo/source")).unwrap().len(),
        (HASH_BUFFER_BYTES * 2) as u64
    );
    f.unchanged_outside();
}

#[test]
fn preservation_fs_symlinked_metadata_ancestors_and_special_files_refuse() {
    let f = Fixture::new();
    let files = f.files();
    let git = f.root.join("repo/.git");
    for name in ["hooks", "logs", "objects"] {
        symlink(f.root.join("outside"), git.join(name)).unwrap();
        match name {
            "hooks" => {
                denied(files.metadata(name));
                denied(files.hooks());
            }
            "logs" => denied(files.reflogs(name, true)),
            _ => denied(files.exists("objects/info/alternates")),
        }
        fs::remove_file(git.join(name)).unwrap();
    }
    symlink(f.root.join("outside/secret"), git.join("index")).unwrap();
    denied(files.read_optional(true, "index"));
    rustix::fs::mkfifoat(&files.checkout, ".gitattributes", Mode::RUSR | Mode::WUSR).unwrap();
    denied(files.read_optional(false, ".gitattributes"));
    f.unchanged_outside();
}

#[test]
fn preservation_fs_displaced_checkout_git_and_symlink_ancestors_refuse() {
    for name in ["repo", "repo/.git"] {
        let f = Fixture::new();
        let files = f.files();
        fs::rename(f.root.join(name), f.root.join("displaced")).unwrap();
        fs::create_dir(f.root.join(name)).unwrap();
        denied(files.revalidate());
        if name == "repo" {
            denied(Files::new(&f.scope, None));
        }
        f.unchanged_outside();
    }
    let f = Fixture::new();
    let files = f.files();
    fs::rename(f.root.join("repo"), f.root.join("displaced")).unwrap();
    symlink(f.root.join("displaced"), f.root.join("repo")).unwrap();
    denied(files.revalidate());
    denied(Files::new(&f.scope, None));
}

#[test]
fn preservation_fs_directory_only_reflog_trees_obey_existing_entry_limit() {
    let f = Fixture::new();
    let logs = f.root.join("repo/.git/logs");
    fs::create_dir(&logs).unwrap();
    for n in 0..=MAX_METADATA_ENTRIES {
        fs::create_dir(logs.join(n.to_string())).unwrap();
    }
    denied(f.files().reflogs("logs", false));
    fs::remove_dir(logs.join(MAX_METADATA_ENTRIES.to_string())).unwrap();
    assert_eq!(
        f.files().reflogs("logs", false).unwrap(),
        Vec::<u8>::new(),
        "exactly the entry limit must succeed without descriptor exhaustion"
    );
    f.unchanged_outside();
}

#[test]
fn preservation_fs_traversal_budgets_refuse_negative_sizes_and_counter_overflow() {
    let mut budget = TraversalBudget::default();
    denied(budget.content(-1));
    let mut budget = TraversalBudget {
        content_bytes: u64::MAX,
        ..TraversalBudget::default()
    };
    denied(budget.content(1));
    let mut budget = TraversalBudget {
        entries: usize::MAX,
        ..TraversalBudget::default()
    };
    denied(budget.entry(Path::new("file")));
    let mut budget = TraversalBudget {
        path_bytes: usize::MAX,
        ..TraversalBudget::default()
    };
    denied(budget.entry(Path::new("file")));
}

#[test]
fn preservation_fs_path_budget_bounds_real_walks_at_the_exact_limit() {
    let f = Fixture::new();
    let tree = f.root.join("repo/.git/logs");
    fs::create_dir(&tree).unwrap();
    fs::create_dir(tree.join("nested")).unwrap();
    fs::write(tree.join("nested/file"), b"bounded").unwrap();
    let files = f.files();
    // Each stored relative name is charged once, including directory names.
    let names = b"nested".len() + b"nested/file".len();
    for kind in [Tree::Worktree, Tree::Metadata, Tree::Reflogs] {
        let expected = files
            .tree(&files.git, Path::new("logs"), kind, false)
            .unwrap();
        let scan = |remaining| {
            files.tree_with_budget(
                &files.git,
                Path::new("logs"),
                kind,
                false,
                TraversalBudget {
                    path_bytes: MAX_TRAVERSAL_PATH_BYTES - remaining,
                    ..TraversalBudget::default()
                },
            )
        };
        assert_eq!(scan(names).unwrap(), expected);
        denied(scan(names - 1));
    }
    f.unchanged_outside();
}

#[test]
fn preservation_fs_path_budget_charges_exclusions_before_opening_them() {
    let f = Fixture::new();
    let files = f.files();
    // The only worktree child is excluded .git, still subject to both budgets.
    let scan = |path_bytes, entries| {
        files.tree_with_budget(
            &files.checkout,
            Path::new(""),
            Tree::Worktree,
            false,
            TraversalBudget {
                entries,
                path_bytes,
                ..TraversalBudget::default()
            },
        )
    };
    assert_eq!(
        scan(MAX_TRAVERSAL_PATH_BYTES - 4, MAX_METADATA_ENTRIES - 1).unwrap(),
        Vec::<u8>::new()
    );
    denied(scan(MAX_TRAVERSAL_PATH_BYTES - 3, 0));
    denied(scan(0, MAX_METADATA_ENTRIES));
    // The same accounting applies to excluded agent-owned reflog entries.
    fs::create_dir(f.root.join("repo/.git/logs")).unwrap();
    symlink(
        f.root.join("outside/secret"),
        f.root.join("repo/.git/logs/agentmage"),
    )
    .unwrap();
    denied(files.tree_with_budget(
        &files.git,
        Path::new("logs"),
        Tree::Reflogs,
        true,
        TraversalBudget {
            path_bytes: MAX_TRAVERSAL_PATH_BYTES,
            ..TraversalBudget::default()
        },
    ));
    assert_eq!(files.reflogs("logs", true).unwrap(), Vec::<u8>::new());
    f.unchanged_outside();
}

#[test]
fn preservation_fs_reflogs_obey_aggregate_content_limit_with_unchanged_hashes() {
    for agent in [false, true] {
        let f = Fixture::new();
        let relative = if agent {
            "logs/refs/heads/agentmage"
        } else {
            "logs"
        };
        let logs = f.root.join("repo/.git").join(relative);
        fs::create_dir_all(&logs).unwrap();
        let half = MAX_HASHED_FILE_BYTES / 2;
        for name in ["first", "second"] {
            fs::File::create(logs.join(name))
                .unwrap()
                .set_len(half)
                .unwrap();
        }
        let digest = hash(&vec![0; half as usize]);
        let expected = [
            b"first".as_slice(),
            digest.as_bytes(),
            b"second".as_slice(),
            digest.as_bytes(),
        ]
        .concat();
        let files = f.files();
        assert_eq!(files.reflogs(relative, !agent).unwrap(), expected);
        fs::OpenOptions::new()
            .write(true)
            .open(logs.join("second"))
            .unwrap()
            .set_len(half + 1)
            .unwrap();
        denied(files.reflogs(relative, !agent));
        f.unchanged_outside();
    }
}
