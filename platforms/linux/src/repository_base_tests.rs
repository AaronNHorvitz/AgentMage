// SPDX-License-Identifier: MIT OR Apache-2.0
//! Private parser tests and a real Git protocol fixture, not native admission.

use super::*;

fn tree(mode: &str, kind: &str, object: &str, path: &str) -> Vec<u8> {
    format!("{mode} {kind} {object}\t{path}\0").into_bytes()
}

fn preimage(bytes: &[u8], mode: CommitFileMode) -> Option<BaseFilePreimage> {
    Some(BaseFilePreimage {
        sha256: digest(bytes),
        mode,
        bytes: bytes.to_vec(),
    })
}

fn digest(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

#[test]
fn base_preimages_bind_exact_blobs_and_distinguish_absence_from_empty() {
    for format in ["sha1", "sha256"] {
        let width = if format == "sha1" { 40 } else { 64 };
        let base = "a".repeat(width);
        let bytes = b"base source\n";
        let blob = crate::linux_git_blob_object_id(format, bytes).unwrap();
        let empty = crate::linux_git_blob_object_id(format, b"").unwrap();
        let mut raw = tree("100644", "blob", &blob, ":(glob)*.py");
        raw.extend(tree("100755", "blob", &empty, "empty"));
        let mut calls = Vec::new();
        let result = collect_with(
            &base,
            &[
                ":(glob)*.py".into(),
                "empty".into(),
                "missing/new.py".into(),
            ],
            |args, limit| {
                calls.push((
                    args.iter().map(|s| s.to_string()).collect::<Vec<_>>(),
                    limit,
                ));
                Ok(match args {
                    ["--no-replace-objects", "cat-file", "-t", id] if *id == base => {
                        b"commit\n".to_vec()
                    }
                    [
                        "--no-replace-objects",
                        "ls-tree",
                        "-r",
                        "-t",
                        "-z",
                        "--full-tree",
                        id,
                    ] if *id == base => raw.clone(),
                    ["--no-replace-objects", "cat-file", "blob", id] if *id == blob => {
                        bytes.to_vec()
                    }
                    ["--no-replace-objects", "cat-file", "blob", id] if *id == empty => vec![],
                    _ => panic!("unexpected read"),
                })
            },
        )
        .unwrap();
        assert_eq!(
            result[":(glob)*.py"],
            preimage(bytes, CommitFileMode::Regular)
        );
        assert_eq!(result["empty"], preimage(b"", CommitFileMode::Executable));
        assert_eq!(result["missing/new.py"], None);
        assert_eq!(calls.len(), 4);
        assert_eq!(calls[0].1, 32);
        assert_eq!(calls[1].1, MAX_TREE_BYTES);
        assert!(calls[2..].iter().all(|(_, limit)| *limit == MAX_BLOB_BYTES));
        // Paths are map keys, never Git revisions, arguments or pathspecs.
        assert!(
            calls
                .iter()
                .all(|(args, _)| !args.contains(&":(glob)*.py".into()))
        );
    }
}

#[test]
fn base_preimage_debug_omits_retained_source_bytes() {
    let value = preimage(b"private source sentinel", CommitFileMode::Regular).unwrap();
    assert_eq!(value.bytes, b"private source sentinel");
    let debug = format!("{value:?}");
    assert!(debug.contains(&value.sha256));
    assert!(!debug.contains("private source sentinel"));
    assert!(!debug.contains("bytes"));
    assert!(!debug.contains("112, 114, 105"));
}

#[test]
fn base_preimages_preserve_distinct_modes_for_identical_blob_bytes() {
    for format in ["sha1", "sha256"] {
        let base = "a".repeat(if format == "sha1" { 40 } else { 64 });
        let bytes = b"same source\n";
        let object = crate::linux_git_blob_object_id(format, bytes).unwrap();
        let mut raw = tree("100644", "blob", &object, "regular.py");
        raw.extend(tree("100755", "blob", &object, "executable.py"));
        let output = collect_with(
            &base,
            &["regular.py".into(), "executable.py".into()],
            |args, _| {
                Ok(match args {
                    ["--no-replace-objects", "cat-file", "-t", _] => b"commit\n".to_vec(),
                    ["--no-replace-objects", "ls-tree", ..] => raw.clone(),
                    ["--no-replace-objects", "cat-file", "blob", id] if *id == object => {
                        bytes.to_vec()
                    }
                    _ => panic!("unexpected read"),
                })
            },
        )
        .unwrap();
        assert_eq!(
            output["regular.py"],
            preimage(bytes, CommitFileMode::Regular)
        );
        assert_eq!(
            output["executable.py"],
            preimage(bytes, CommitFileMode::Executable)
        );
    }
}

#[test]
fn base_preimages_reject_noncommit_and_invalid_inputs_before_any_path_read() {
    for (base, paths) in [
        ("HEAD".into(), vec!["source".into()]),
        ("A".repeat(40), vec!["source".into()]),
        ("0".repeat(40), vec!["source".into()]),
        ("a".repeat(40), vec![]),
        ("a".repeat(40), vec!["../outside".into()]),
        ("a".repeat(40), vec!["a//b".into()]),
        ("a".repeat(40), vec!["/outside".into()]),
        ("a".repeat(40), vec!["a\nb".into()]),
        ("a".repeat(40), vec!["source".into(), "source".into()]),
        (
            "a".repeat(40),
            (0..17).map(|n| format!("file-{n}")).collect(),
        ),
    ] {
        assert!(collect_with(&base, &paths, |_, _| panic!("invalid input launched Git")).is_err());
    }
    for output in [b"blob\n".as_slice(), b"tree\n", b"commit", b"commit\nextra"] {
        let mut calls = 0;
        assert!(
            collect_with(&"a".repeat(40), &["source".into()], |_, _| {
                calls += 1;
                Ok(output.to_vec())
            })
            .is_err()
        );
        assert_eq!(calls, 1);
    }
}

#[test]
fn base_preimages_refuse_symlinks_gitlinks_directories_and_their_descendants() {
    for (mode, kind) in [("120000", "blob"), ("160000", "commit"), ("040000", "tree")] {
        for selected in ["node", "node/new.py"] {
            // A real base directory can contain a new file. Other node kinds cannot.
            let expected_ok = mode == "040000" && selected == "node/new.py";
            let result = collect_with(&"a".repeat(40), &[selected.into()], |args, _| {
                Ok(if args[1] == "cat-file" {
                    b"commit\n".to_vec()
                } else {
                    tree(mode, kind, &"b".repeat(40), "node")
                })
            });
            assert_eq!(result.is_ok(), expected_ok, "{mode} {selected}");
        }
    }
}

#[test]
fn base_preimages_refuse_corrupt_oversize_and_failed_blob_reads() {
    let object = crate::linux_git_blob_object_id("sha1", b"expected").unwrap();
    for output in [
        Ok(b"different".to_vec()),
        Ok(vec![b'x'; MAX_BLOB_BYTES + 1]),
        Err(failed()),
    ] {
        assert!(
            collect_with(&"a".repeat(40), &["source".into()], |args, _| {
                match args {
                    ["--no-replace-objects", "cat-file", "-t", _] => Ok(b"commit\n".to_vec()),
                    ["--no-replace-objects", "ls-tree", ..] => {
                        Ok(tree("100644", "blob", &object, "source"))
                    }
                    ["--no-replace-objects", "cat-file", "blob", id] if *id == object => {
                        output.clone()
                    }
                    _ => panic!("unexpected command"),
                }
            })
            .is_err()
        );
    }
}

#[test]
fn base_tree_parser_refuses_ambiguous_truncated_unbounded_or_malformed_records() {
    let object = "b".repeat(40);
    let valid = tree("100644", "blob", &object, "file");
    let mut duplicate = valid.clone();
    duplicate.extend_from_slice(&valid);
    for raw in [
        duplicate,
        valid[..valid.len() - 1].to_vec(),
        tree("100644", "tree", &object, "file"),
        tree("100644", "blob", &"b".repeat(64), "file"),
        tree("100644", "blob", &"B".repeat(40), "file"),
        tree("100644", "blob", &"0".repeat(40), "file"),
        tree("100644", "blob", &object, "../file"),
        tree("100644", "blob", &object, "missing-parent/file"),
        b"\0".to_vec(),
        [valid.clone(), vec![0]].concat(),
        [valid.clone(), vec![0xff, 0]].concat(),
        vec![0; MAX_TREE_BYTES + 1],
    ] {
        assert!(parse_tree(&raw, 40).is_err());
    }
    let mut too_many = Vec::new();
    for n in 0..=MAX_TREE_ENTRIES {
        too_many.extend(tree("100644", "blob", &object, &format!("file-{n}")));
    }
    assert!(parse_tree(&too_many, 40).is_err());
    assert!(parse_tree(&valid, 40).is_ok());
    assert!(parse_tree(b"", 40).unwrap().is_empty());
}

#[test]
fn base_preimages_real_git_protocol_fixture_keeps_base_independent_of_worktree_and_index() {
    use std::fs;
    use std::process::Command;
    use std::sync::atomic::{AtomicUsize, Ordering};

    // Direct Git is solely a test data producer. This fixture neither constructs
    // LinuxGitArtifact nor admits the confined native product workflow.
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let root = std::env::temp_dir().join(format!(
        "base-protocol-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir(&root).unwrap();
    struct Cleanup(std::path::PathBuf);
    impl Drop for Cleanup {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.0).unwrap();
        }
    }
    let _cleanup = Cleanup(root.clone());
    let git = |args: &[&str]| {
        Command::new("/usr/bin/git")
            .env_clear()
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_AUTHOR_NAME", "Test")
            .env("GIT_AUTHOR_EMAIL", "test@example.invalid")
            .env("GIT_COMMITTER_NAME", "Test")
            .env("GIT_COMMITTER_EMAIL", "test@example.invalid")
            .args([
                "-c",
                "core.hooksPath=/dev/null",
                "-c",
                "maintenance.auto=false",
                "-c",
                "gc.auto=0",
            ])
            .args(args)
            .current_dir(&root)
            .output()
            .unwrap()
    };
    for args in [
        vec!["init", "--quiet"],
        vec!["config", "commit.gpgsign", "false"],
    ] {
        assert!(git(&args).status.success());
    }
    fs::write(root.join("source.py"), b"base\n").unwrap();
    fs::write(root.join("empty.py"), b"").unwrap();
    assert!(
        git(&["add", "--", "source.py", "empty.py"])
            .status
            .success()
    );
    // Force a committed executable bit independent of host permissions and
    // core.filemode. Later index metadata must not replace this base mode.
    assert!(
        git(&["update-index", "--chmod=+x", "--", "empty.py"])
            .status
            .success()
    );
    assert!(git(&["commit", "--quiet", "-m", "base"]).status.success());
    let base = String::from_utf8(git(&["rev-parse", "HEAD"]).stdout)
        .unwrap()
        .trim()
        .to_owned();
    fs::write(root.join("source.py"), b"dirty staged content\n").unwrap();
    fs::write(root.join("untracked.py"), b"untracked initial content\n").unwrap();
    assert!(git(&["add", "--", "source.py"]).status.success());
    assert!(
        git(&["update-index", "--chmod=-x", "--", "empty.py"])
            .status
            .success()
    );
    // A replacement ref must not change the meaning of the immutable base.
    let alternate_tree = String::from_utf8(git(&["write-tree"]).stdout)
        .unwrap()
        .trim()
        .to_owned();
    let alternate = git(&["commit-tree", &alternate_tree, "-m", "alternate"]);
    assert!(alternate.status.success());
    let alternate = String::from_utf8(alternate.stdout)
        .unwrap()
        .trim()
        .to_owned();
    assert!(git(&["replace", &base, &alternate]).status.success());
    assert_eq!(
        git(&["cat-file", "blob", &format!("{base}:source.py")]).stdout,
        b"dirty staged content\n"
    );
    let index = fs::read(root.join(".git/index")).unwrap();
    let head = fs::read(root.join(".git/HEAD")).unwrap();
    let result = collect_with(
        &base,
        &["source.py".into(), "empty.py".into(), "untracked.py".into()],
        |args, _| {
            let out = git(args);
            if out.status.success() {
                Ok(out.stdout)
            } else {
                Err(failed())
            }
        },
    )
    .unwrap();
    assert_eq!(
        result["source.py"],
        preimage(b"base\n", CommitFileMode::Regular)
    );
    assert_eq!(
        result["empty.py"],
        preimage(b"", CommitFileMode::Executable)
    );
    assert_eq!(result["untracked.py"], None);
    assert_eq!(fs::read(root.join(".git/index")).unwrap(), index);
    assert_eq!(fs::read(root.join(".git/HEAD")).unwrap(), head);
    assert_eq!(
        fs::read(root.join("source.py")).unwrap(),
        b"dirty staged content\n"
    );
    assert_eq!(
        fs::read(root.join("untracked.py")).unwrap(),
        b"untracked initial content\n"
    );
}

#[test]
fn base_cancellation_reaches_type_tree_and_each_blob_and_never_returns_partial_sources() {
    use agentmage_kernel_contracts::{
        BoundaryKind, CONTRACT_SCHEMA_VERSION, CancellationId, CancellationReason,
        CancellationSignal, CorrelationId, TaskId,
    };
    let base = "a".repeat(40);
    let blob = crate::linux_git_blob_object_id("sha1", b"base").unwrap();
    let raw = [
        tree("100644", "blob", &blob, "first"),
        tree("100644", "blob", &blob, "second"),
    ]
    .concat();
    for stop in 0..=4 {
        let token = CancellationToken::root(
            BoundaryKind::Shell,
            TaskId::from_raw("base"),
            CorrelationId::from_raw("base"),
        );
        let cancel = |token: &CancellationToken| {
            token
                .cancel(CancellationSignal {
                    schema_version: CONTRACT_SCHEMA_VERSION,
                    cancellation_id: CancellationId::from_raw("base-stop"),
                    correlation_id: CorrelationId::from_raw("base"),
                    task_id: TaskId::from_raw("base"),
                    reason: CancellationReason::UserRequested,
                    requested_by: BoundaryKind::Shell,
                })
                .unwrap()
        };
        if stop == 0 {
            cancel(&token);
        }
        let mut calls = 0;
        let output = collect_cancellable_with(
            &base,
            &["first".into(), "second".into()],
            Some(&token),
            |args, _, forwarded| {
                let forwarded = forwarded.expect("caller token must reach every base worker");
                calls += 1;
                if calls == stop {
                    cancel(forwarded);
                    assert!(token.is_cancelled());
                }
                Ok(match args {
                    ["--no-replace-objects", "cat-file", "-t", _] => b"commit\n".to_vec(),
                    ["--no-replace-objects", "ls-tree", ..] => raw.clone(),
                    ["--no-replace-objects", "cat-file", "blob", _] => b"base".to_vec(),
                    _ => panic!("unexpected base read"),
                })
            },
        );
        assert_eq!(calls, stop);
        assert_eq!(
            output.unwrap_err().kind(),
            LinuxRepositoryErrorKind::ObservationFailed
        );
    }
}

#[test]
fn base_deadline_reaches_every_worker_and_rejects_late_success_without_partial_sources() {
    use std::time::Duration;
    let base = "a".repeat(40);
    let blob = crate::linux_git_blob_object_id("sha1", b"base").unwrap();
    let raw = [
        tree("100644", "blob", &blob, "first"),
        tree("100644", "blob", &blob, "second"),
    ]
    .concat();
    // None is the normal successful case; the other cases expire before
    // dispatch, after type, after tree or after either blob observation.
    for stop in [None, Some(0), Some(1), Some(2), Some(3), Some(4)] {
        for with_token in [false, true] {
            let token = agentmage_kernel_engine::propagation::CancellationToken::root(
                agentmage_kernel_contracts::BoundaryKind::Shell,
                agentmage_kernel_contracts::TaskId::from_raw("base-deadline"),
                agentmage_kernel_contracts::CorrelationId::from_raw("base-deadline"),
            );
            let cancellation = with_token.then_some(&token);
            let deadline = Instant::now()
                + if stop == Some(0) {
                    Duration::ZERO
                } else {
                    Duration::from_millis(100)
                };
            let mut calls = 0;
            let result = collect_until_with(
                &base,
                &["first".into(), "second".into()],
                cancellation,
                Some(deadline),
                |args, _, forwarded, until| {
                    calls += 1;
                    assert_eq!(
                        forwarded.map(|t| t as *const _),
                        cancellation.map(|t| t as *const _)
                    );
                    assert_eq!(until, Some(deadline));
                    if stop == Some(calls) {
                        while let Some(remaining) = deadline.checked_duration_since(Instant::now())
                        {
                            std::thread::sleep(remaining);
                        }
                    }
                    Ok(match args {
                        ["--no-replace-objects", "cat-file", "-t", _] => b"commit\n".to_vec(),
                        ["--no-replace-objects", "ls-tree", ..] => raw.clone(),
                        ["--no-replace-objects", "cat-file", "blob", _] => b"base".to_vec(),
                        _ => panic!("unexpected base read"),
                    })
                },
            );
            assert_eq!(calls, stop.unwrap_or(4));
            if stop.is_some() {
                assert_eq!(
                    result.unwrap_err().kind(),
                    LinuxRepositoryErrorKind::ObservationFailed
                );
            } else {
                let result = result.unwrap();
                assert_eq!(result.len(), 2);
                assert_eq!(result["first"], preimage(b"base", CommitFileMode::Regular));
                assert_eq!(result["second"], result["first"]);
            }
            assert!(!token.is_cancelled());
        }
    }
}
