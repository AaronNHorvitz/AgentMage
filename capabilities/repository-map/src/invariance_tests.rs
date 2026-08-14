#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::ffi::OsStr;
    use std::fs;
    use std::os::unix::fs::{MetadataExt, symlink};
    use std::path::{Path, PathBuf};
    use std::process::Command;
    use std::time::{SystemTime, UNIX_EPOCH};

    use agentmage_kernel_contracts::WorkspaceId;
    use sha2::{Digest, Sha256};

    use crate::{
        GitTrackedState, RepositoryFileInput, RepositoryMapInput, build_repository_map,
        verify_repository_map,
    };

    struct Fixture(PathBuf);

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn git(root: &Path, args: &[&str]) -> Vec<u8> {
        let output = Command::new("git")
            .args(args)
            .current_dir(root)
            .env_clear()
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_TERMINAL_PROMPT", "0")
            .env("LANG", "C")
            .output()
            .expect("pinned disposable Git fixture requires Git");
        assert!(output.status.success(), "fixture command failed: {args:?}");
        output.stdout
    }

    fn sha256(bytes: &[u8]) -> String {
        let mut output = String::with_capacity(64);
        for byte in Sha256::digest(bytes) {
            use std::fmt::Write;
            write!(&mut output, "{byte:02x}").expect("hex");
        }
        output
    }

    fn snapshot(root: &Path) -> BTreeMap<String, (u32, String, String)> {
        fn walk(root: &Path, current: &Path, values: &mut BTreeMap<String, (u32, String, String)>) {
            let mut entries = fs::read_dir(current)
                .expect("read fixture")
                .map(|entry| entry.expect("entry").path())
                .collect::<Vec<_>>();
            entries.sort();
            for path in entries {
                let metadata = fs::symlink_metadata(&path).expect("metadata");
                let relative = path
                    .strip_prefix(root)
                    .expect("relative")
                    .to_string_lossy()
                    .into_owned();
                if metadata.file_type().is_symlink() {
                    let target = fs::read_link(&path).expect("link");
                    values.insert(
                        relative,
                        (
                            metadata.mode(),
                            "symlink".to_owned(),
                            sha256(target.as_os_str().as_encoded_bytes()),
                        ),
                    );
                } else if metadata.is_dir() {
                    values.insert(
                        relative,
                        (metadata.mode(), "directory".to_owned(), String::new()),
                    );
                    walk(root, &path, values);
                } else {
                    values.insert(
                        relative,
                        (
                            metadata.mode(),
                            "file".to_owned(),
                            sha256(&fs::read(&path).expect("file")),
                        ),
                    );
                }
            }
        }
        let mut values = BTreeMap::new();
        walk(root, root, &mut values);
        values
    }

    fn input_file(
        root: &Path,
        relative: &[&str],
        git_state: GitTrackedState,
        policy_excluded: bool,
        generated: bool,
        vendored: bool,
    ) -> RepositoryFileInput {
        let path = relative
            .iter()
            .fold(root.to_path_buf(), |path, part| path.join(part));
        let metadata = fs::symlink_metadata(&path).expect("metadata");
        let content =
            (!policy_excluded && !generated && !vendored && git_state != GitTrackedState::Ignored)
                .then(|| fs::read(&path).expect("authorized fixture bytes"));
        let hash_material = content.as_deref().unwrap_or_else(|| {
            if metadata.file_type().is_symlink() {
                b"excluded-symlink"
            } else {
                b"excluded-content"
            }
        });
        RepositoryFileInput {
            path: relative.iter().map(|part| (*part).to_owned()).collect(),
            size_bytes: content
                .as_ref()
                .map_or(metadata.len(), |bytes| bytes.len() as u64),
            content_sha256: sha256(hash_material),
            content,
            git_state,
            policy_excluded,
            generated,
            vendored,
        }
    }

    #[test]
    fn disposable_git_mapping_is_byte_mode_ref_and_object_invariant() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "agentmage-repository-map-{}-{nonce}",
            std::process::id()
        ));
        for directory in ["src", "legacy", "generated", "vendor"] {
            fs::create_dir_all(root.join(directory)).expect("directory");
        }
        fs::write(root.join("src/lib.rs"), b"pub fn run() {}\n").expect("rust");
        fs::write(root.join("legacy/tool.rb"), b"puts 'NEEDLE'\n").expect("ruby");
        fs::write(root.join("binary.rs"), b"fn run() {}\0").expect("binary");
        fs::write(root.join("ignored.rs"), b"fn ignored() {}\n").expect("ignored");
        fs::write(root.join("generated/out.js"), b"export const value = 1;\n").expect("generated");
        fs::write(root.join("vendor/lib.py"), b"def vendored(): pass\n").expect("vendor");
        fs::write(root.join(".gitignore"), b"ignored.rs\n").expect("ignore rules");
        symlink(OsStr::new("."), root.join("recursive-link")).expect("recursive link");
        let fixture = Fixture(root);

        git(&fixture.0, &["init", "-q"]);
        git(&fixture.0, &["config", "user.name", "AgentMage Fixture"]);
        git(
            &fixture.0,
            &["config", "user.email", "fixture@example.invalid"],
        );
        git(
            &fixture.0,
            &[
                "add",
                ".gitignore",
                "src/lib.rs",
                "legacy/tool.rb",
                "binary.rs",
            ],
        );
        git(&fixture.0, &["commit", "-q", "-m", "fixture"]);
        git(&fixture.0, &["check-ignore", "-q", "ignored.rs"]);
        let commit = String::from_utf8(git(&fixture.0, &["rev-parse", "HEAD"]))
            .expect("commit")
            .trim()
            .to_owned();
        let branch = String::from_utf8(git(&fixture.0, &["rev-parse", "--abbrev-ref", "HEAD"]))
            .expect("branch")
            .trim()
            .to_owned();
        let before = snapshot(&fixture.0);

        let specifications = [
            (
                &[".gitignore"][..],
                GitTrackedState::TrackedClean,
                false,
                false,
                false,
            ),
            (
                &["binary.rs"][..],
                GitTrackedState::TrackedClean,
                false,
                false,
                false,
            ),
            (
                &["generated", "out.js"][..],
                GitTrackedState::Untracked,
                false,
                true,
                false,
            ),
            (
                &["ignored.rs"][..],
                GitTrackedState::Ignored,
                false,
                false,
                false,
            ),
            (
                &["legacy", "tool.rb"][..],
                GitTrackedState::TrackedClean,
                false,
                false,
                false,
            ),
            (
                &["recursive-link"][..],
                GitTrackedState::Untracked,
                true,
                false,
                false,
            ),
            (
                &["src", "lib.rs"][..],
                GitTrackedState::TrackedClean,
                false,
                false,
                false,
            ),
            (
                &["vendor", "lib.py"][..],
                GitTrackedState::Untracked,
                false,
                false,
                true,
            ),
        ];
        let files = specifications
            .iter()
            .map(|(path, state, excluded, generated, vendored)| {
                input_file(&fixture.0, path, *state, *excluded, *generated, *vendored)
            })
            .collect();
        let input = RepositoryMapInput {
            workspace_id: WorkspaceId::from_raw("workspace-live-invariance"),
            repository_sha256: sha256(b"disposable-repository"),
            worktree_sha256: sha256(b"exact-held-worktree"),
            branch: Some(branch),
            commit_id: commit,
            policy_sha256: sha256(b"fixture-policy"),
            freshness_sha256: sha256(b"fixture-freshness"),
            files,
        };
        let first = build_repository_map(input.clone()).expect("first map");
        let second = build_repository_map(input).expect("second map");
        assert!(verify_repository_map(&first));
        assert_eq!(first, second);
        assert_eq!(first.coverage.discovered_files, 8);
        assert_eq!(first.coverage.excluded_files, 4);
        assert_eq!(first.coverage.unsupported_files, 2);
        assert_eq!(snapshot(&fixture.0), before);
    }
}
