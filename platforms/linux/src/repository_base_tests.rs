// SPDX-License-Identifier: Apache-2.0
//! Private parser tests and a real Git protocol fixture, not native admission.

use super::*;

fn tree(mode: &str, kind: &str, object: &str, path: &str) -> Vec<u8> {
    format!("{mode} {kind} {object}\t{path}\0").into_bytes()
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
        assert_eq!(result[":(glob)*.py"], Some(digest(bytes)));
        assert_eq!(result["empty"], Some(digest(b"")));
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
    assert!(git(&["commit", "--quiet", "-m", "base"]).status.success());
    let base = String::from_utf8(git(&["rev-parse", "HEAD"]).stdout)
        .unwrap()
        .trim()
        .to_owned();
    fs::write(root.join("source.py"), b"dirty staged content\n").unwrap();
    fs::write(root.join("untracked.py"), b"untracked initial content\n").unwrap();
    assert!(git(&["add", "--", "source.py"]).status.success());
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
    assert_eq!(result["source.py"], Some(digest(b"base\n")));
    assert_eq!(result["empty.py"], Some(digest(b"")));
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
