// SPDX-License-Identifier: MIT OR Apache-2.0
//! Dormant candidate construction through the closed offline Git supervisor.

use super::*;
use agentmage_kernel_engine::local_commit::{
    CandidateTreePlan, CommitFileMode, verify_candidate_tree_plan,
};
use rustix::fs::{AtFlags, mkdirat, openat, statat, unlinkat};
use std::collections::BTreeMap;

const MAX_POSTIMAGE: usize = 64 * 1024 * 1024;

// A plan remains authority-free. Only the dormant platform builder can reach
// this owner; there is no API, receipt restoration or caller-supplied executor.
pub(crate) struct CandidateSession {
    original: LinuxRepositoryMutationScope,
    worker: Arc<LinuxRepositoryMutationScope>,
    plan: CandidateTreePlan,
    directory: String,
    initialized: bool,
    next: usize,
    blob: Option<String>,
    terminal: bool,
    cleanup_allowed: bool,
    deadline: Instant,
}

pub(crate) fn index_path(scope: &LinuxRepositoryScope, id: &str) -> Option<PathBuf> {
    if id.is_empty()
        || id.len() > 128
        || !id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b':' | b'-'))
    {
        return None;
    }
    Some(
        scope
            .owned_root
            .join(format!("candidate-index-{id}"))
            .join("index"),
    )
}

#[derive(Clone, Copy)]
enum Step<'a> {
    ReadTree,
    Hash { operation: &'a str, bytes: &'a [u8] },
    Add,
    WriteTree,
}

// Setup owns no worker yet. Roll back only an empty, still-bound directory;
// unknown entries, replacements and failed identity checks retain the claim.
struct CandidateClaim<'a> {
    original: &'a LinuxRepositoryMutationScope,
    directory: &'a str,
    descriptor: OwnedFd,
    armed: bool,
}

impl CandidateClaim<'_> {
    fn matches(&self, descriptor: &OwnedFd) -> Result<(), LinuxRepositoryError> {
        let held = fstat(&self.descriptor).map_err(|_| denied())?;
        let captured = fstat(descriptor).map_err(|_| denied())?;
        if held.st_dev != captured.st_dev || held.st_ino != captured.st_ino {
            return Err(denied());
        }
        Ok(())
    }

    fn revalidate(&self) -> Result<(), LinuxRepositoryError> {
        self.original.revalidate()?;
        let held = fstat(&self.descriptor).map_err(|_| denied())?;
        let named = statat(
            &self.original.roots[2],
            self.directory,
            AtFlags::SYMLINK_NOFOLLOW,
        )
        .map_err(|_| denied())?;
        if held.st_dev != named.st_dev
            || held.st_ino != named.st_ino
            || held.st_uid != getuid().as_raw()
            || held.st_mode & 0o077 != 0
            || FileType::from_raw_mode(held.st_mode) != FileType::Directory
        {
            return Err(denied());
        }
        Ok(())
    }
}

impl Drop for CandidateClaim<'_> {
    fn drop(&mut self) {
        if self.armed && self.revalidate().is_ok() {
            // REMOVEDIR atomically refuses nonempty directories. No recursive
            // removal or removal of index files is authorized during setup.
            let _ = unlinkat(&self.original.roots[2], self.directory, AtFlags::REMOVEDIR);
        }
    }
}

impl CandidateSession {
    #[cfg(test)]
    pub(crate) fn new(
        scope: &LinuxRepositoryScope,
        plan: &CandidateTreePlan,
    ) -> Result<Self, LinuxRepositoryError> {
        Self::new_until(scope, plan, &test_token(), Instant::now() + PROCESS_TIMEOUT)
    }

    pub(crate) fn new_until(
        scope: &LinuxRepositoryScope,
        plan: &CandidateTreePlan,
        cancellation: &CancellationToken,
        deadline: Instant,
    ) -> Result<Self, LinuxRepositoryError> {
        Self::new_with_until(scope, plan, cancellation, deadline, |scratch| {
            let worker_scope =
                LinuxRepositoryScope::verify(&scope.checkout_root, &scope.git_directory, scratch)?;
            LinuxRepositoryMutationScope::capture(&worker_scope)
        })
    }

    #[cfg(test)]
    fn new_with(
        scope: &LinuxRepositoryScope,
        plan: &CandidateTreePlan,
        capture: impl FnOnce(&Path) -> Result<LinuxRepositoryMutationScope, LinuxRepositoryError>,
    ) -> Result<Self, LinuxRepositoryError> {
        Self::new_with_until(
            scope,
            plan,
            &test_token(),
            Instant::now() + PROCESS_TIMEOUT,
            capture,
        )
    }

    // Private setup seam; product callers retain their original outer budget.
    fn new_with_until(
        scope: &LinuxRepositoryScope,
        plan: &CandidateTreePlan,
        cancellation: &CancellationToken,
        deadline: Instant,
        capture: impl FnOnce(&Path) -> Result<LinuxRepositoryMutationScope, LinuxRepositoryError>,
    ) -> Result<Self, LinuxRepositoryError> {
        let deadline = deadline.min(Instant::now() + PROCESS_TIMEOUT);
        let check = || {
            if cancellation.is_cancelled() || Instant::now() >= deadline {
                Err(denied())
            } else {
                Ok(())
            }
        };
        check()?;
        verify_candidate_tree_plan(plan).map_err(|_| denied())?;
        let index = index_path(scope, &plan.candidate_build_id).ok_or_else(denied)?;
        if hash(index.as_os_str().as_bytes()) != plan.temporary_index_path_sha256
            || plan.repository_sha256 != scope.repository_path_sha256
        {
            return Err(denied());
        }
        let original = LinuxRepositoryMutationScope::capture(scope)?;
        let directory = format!("candidate-index-{}", plan.candidate_build_id);
        check()?;
        // Atomic exclusive directory claim, relative to the HELD parent. Existing
        // directories, symlinks (including dangling ones) and reused IDs refuse.
        mkdirat(
            &original.roots[2],
            directory.as_str(),
            Mode::from_raw_mode(0o700),
        )
        .map_err(|_| denied())?;
        // Open through the held parent before any path-based worker capture.
        // If opening fails, no child identity was obtained: retain rather than
        // remove an entry whose identity cannot be checked.
        let mut claim = CandidateClaim {
            original: &original,
            directory: &directory,
            descriptor: openat(
                &original.roots[2],
                directory.as_str(),
                OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
                Mode::empty(),
            )
            .map_err(|_| denied())?,
            armed: true,
        };
        claim.revalidate()?;
        let scratch = index.parent().ok_or_else(denied)?;
        let worker = Arc::new(capture(scratch)?);
        check()?;
        claim.matches(&worker.roots[2])?;
        claim.revalidate()?;
        worker.revalidate()?;
        check()?;
        claim.armed = false;
        drop(claim);
        Ok(Self {
            original,
            worker,
            plan: plan.clone(),
            directory,
            initialized: false,
            next: 0,
            blob: None,
            terminal: false,
            cleanup_allowed: true,
            deadline,
        })
    }

    pub(crate) fn read_tree(
        &mut self,
        git: &LinuxGitArtifact,
        cancellation: &CancellationToken,
    ) -> Result<(), LinuxRepositoryError> {
        self.step(git, Step::ReadTree, cancellation).map(|_| ())
    }

    pub(crate) fn add_postimage(
        &mut self,
        git: &LinuxGitArtifact,
        operation: &str,
        bytes: &[u8],
        cancellation: &CancellationToken,
    ) -> Result<String, LinuxRepositoryError> {
        let blob = self.step(git, Step::Hash { operation, bytes }, cancellation)?;
        self.step(git, Step::Add, cancellation)?;
        String::from_utf8(blob).map_err(|_| denied())
    }

    pub(crate) fn write_tree(
        &mut self,
        git: &LinuxGitArtifact,
        cancellation: &CancellationToken,
    ) -> Result<String, LinuxRepositoryError> {
        String::from_utf8(self.step(git, Step::WriteTree, cancellation)?).map_err(|_| denied())
    }

    fn step(
        &mut self,
        git: &LinuxGitArtifact,
        step: Step<'_>,
        cancellation: &CancellationToken,
    ) -> Result<Vec<u8>, LinuxRepositoryError> {
        self.step_with(step, cancellation, |owner, args, env, input, deadline| {
            run_worker(git, owner, args, env, input, cancellation, deadline)
        })
    }

    // Private deterministic seam. Product code always calls the closed worker.
    fn step_with(
        &mut self,
        step: Step<'_>,
        cancellation: &CancellationToken,
        dispatch: impl FnOnce(
            Arc<LinuxRepositoryMutationScope>,
            &[String],
            &BTreeMap<String, String>,
            Option<OwnedFd>,
            Instant,
        ) -> Result<SupervisedResult, InvocationStatus>,
    ) -> Result<Vec<u8>, LinuxRepositoryError> {
        let result = self.perform(step, cancellation, dispatch);
        if result.is_err() {
            self.terminal = true;
        }
        result
    }

    fn perform(
        &mut self,
        step: Step<'_>,
        cancellation: &CancellationToken,
        dispatch: impl FnOnce(
            Arc<LinuxRepositoryMutationScope>,
            &[String],
            &BTreeMap<String, String>,
            Option<OwnedFd>,
            Instant,
        ) -> Result<SupervisedResult, InvocationStatus>,
    ) -> Result<Vec<u8>, LinuxRepositoryError> {
        if self.terminal || cancellation.is_cancelled() || Instant::now() >= self.deadline {
            return Err(denied());
        }
        self.original.revalidate()?;
        self.worker.revalidate()?;
        self.validate_scratch()?;
        let mut args = crate::local_commit::hardened_git_prefix()
            .map(str::to_owned)
            .to_vec();
        for setting in [
            "core.splitIndex=false",
            "core.sparseCheckout=false",
            "index.sparse=false",
            "core.sharedRepository=0",
            "index.threads=1",
            "core.preloadIndex=false",
        ] {
            args.extend(["-c".into(), setting.into()]);
        }
        args.push(format!(
            "--git-dir={}",
            self.worker.scope.git_directory.display()
        ));
        let mut input = None;
        let operation: Vec<String> = match step {
            Step::ReadTree if !self.initialized && self.next == 0 => {
                vec!["read-tree".into(), self.plan.parent_object.clone()]
            }
            Step::Hash { operation, bytes } if self.initialized && self.blob.is_none() => {
                let file = self.plan.files.get(self.next).ok_or_else(denied)?;
                if file.operation_id != operation
                    || bytes.len() > MAX_POSTIMAGE
                    || hash(bytes) != file.postimage_sha256
                {
                    return Err(denied());
                }
                input = Some(
                    crate::sandbox::sealed_payload("agentmage-candidate-postimage", bytes)
                        .map_err(|_| denied())?,
                );
                vec![
                    "hash-object".into(),
                    "-w".into(),
                    "--no-filters".into(),
                    "--stdin".into(),
                ]
            }
            Step::Add if self.initialized && self.blob.is_some() => {
                let file = self.plan.files.get(self.next).ok_or_else(denied)?;
                let mode = match file.mode {
                    CommitFileMode::Regular => "100644",
                    CommitFileMode::Executable => "100755",
                };
                vec![
                    "update-index".into(),
                    "--add".into(),
                    "--cacheinfo".into(),
                    format!(
                        "{mode},{},{}",
                        self.blob.as_ref().ok_or_else(denied)?,
                        file.path
                    ),
                ]
            }
            Step::WriteTree
                if self.initialized
                    && self.next == self.plan.files.len()
                    && self.blob.is_none() =>
            {
                vec!["write-tree".into()]
            }
            _ => return Err(denied()),
        };
        args.extend(operation);
        let index = self.worker.scope.owned_root.join("index");
        let mut env = candidate_environment();
        env.insert(
            "GIT_INDEX_FILE".into(),
            index.to_str().ok_or_else(denied)?.into(),
        );
        self.cleanup_allowed = false;
        let result = dispatch(self.worker.clone(), &args, &env, input, self.deadline);
        self.original.revalidate()?;
        self.worker.revalidate()?;
        let result = result.map_err(|_| denied())?;
        self.cleanup_allowed = result.output_complete;
        // No truncated/incomplete capture, unknown cleanup, cancellation or
        // successful-looking stdout from a failed worker may advance the state.
        if cancellation.is_cancelled()
            || Instant::now() >= self.deadline
            || !result.output_complete
            || !matches!(result.outcome, Ok(OperationOutcome::Succeeded))
            || !result.status.is_some_and(|s| s.success())
            || result.stdout.total > MAX_OBSERVATION_BYTES
            || result.stderr.total > MAX_OBSERVATION_BYTES
            || result.stdout.total != result.stdout.retained.len()
        {
            return Err(denied());
        }
        self.validate_scratch()?;
        let output = result.stdout.retained;
        match step {
            Step::ReadTree => {
                if !output.is_empty() {
                    return Err(denied());
                }
                self.initialized = true;
                Ok(output)
            }
            Step::Hash { bytes, .. } => {
                let format = if self.plan.parent_object.len() == 40 {
                    "sha1"
                } else {
                    "sha256"
                };
                let expected =
                    crate::linux_git_blob_object_id(format, bytes).map_err(|_| denied())?;
                if output != format!("{expected}\n").as_bytes() {
                    return Err(denied());
                }
                self.blob = Some(expected.clone());
                Ok(expected.into_bytes())
            }
            Step::Add => {
                if !output.is_empty() {
                    return Err(denied());
                }
                self.next += 1;
                self.blob = None;
                Ok(output)
            }
            Step::WriteTree => {
                let object = std::str::from_utf8(&output).map_err(|_| denied())?;
                let object = object.strip_suffix('\n').ok_or_else(denied)?;
                if object.len() != self.plan.parent_object.len()
                    || !object
                        .bytes()
                        .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
                    || object.bytes().all(|b| b == b'0')
                {
                    return Err(denied());
                }
                self.terminal = true;
                Ok(object.as_bytes().to_vec())
            }
        }
    }

    fn validate_scratch(&self) -> Result<(), LinuxRepositoryError> {
        for entry in fs::read_dir(&self.worker.scope.owned_root).map_err(|_| denied())? {
            let entry = entry.map_err(|_| denied())?;
            let metadata = entry.path().symlink_metadata().map_err(|_| denied())?;
            if entry.file_name() != "index"
                || !metadata.is_file()
                || metadata.uid() != getuid().as_raw()
                || metadata.mode() & 0o022 != 0
                || metadata.nlink() != 1
                || metadata.len() > MAX_HASHED_FILE_BYTES
            {
                return Err(denied());
            }
        }
        Ok(())
    }

    // Called even after an operation error. Never recursively delete or follow
    // a replaced scratch path. Unknown files are retained for explicit recovery.
    pub(crate) fn cleanup(&mut self) -> bool {
        self.terminal = true;
        self.cleanup_allowed && self.cleanup_inner().is_ok()
    }
    fn cleanup_inner(&self) -> Result<(), LinuxRepositoryError> {
        self.original.revalidate()?;
        self.worker.revalidate()?;
        let root = &self.worker.scope.owned_root;
        for entry in fs::read_dir(root).map_err(|_| denied())? {
            let entry = entry.map_err(|_| denied())?;
            if !matches!(entry.file_name().to_str(), Some("index" | "index.lock"))
                || !entry.file_type().map_err(|_| denied())?.is_file()
            {
                return Err(denied());
            }
        }
        for name in ["index", "index.lock"] {
            match unlinkat(&self.worker.roots[2], name, AtFlags::empty()) {
                Ok(()) | Err(rustix::io::Errno::NOENT) => (),
                Err(_) => return Err(denied()),
            }
        }
        self.worker.revalidate()?;
        unlinkat(
            &self.original.roots[2],
            self.directory.as_str(),
            AtFlags::REMOVEDIR,
        )
        .map_err(|_| denied())?;
        self.original.revalidate()?;
        if root.symlink_metadata().is_ok() {
            return Err(denied());
        }
        Ok(())
    }
}

pub(super) fn candidate_environment() -> BTreeMap<String, String> {
    [
        ("GIT_CONFIG_NOSYSTEM", "1"),
        ("GIT_CONFIG_GLOBAL", "/dev/null"),
        ("GIT_TERMINAL_PROMPT", "0"),
        ("GIT_OPTIONAL_LOCKS", "0"),
        ("GIT_LFS_SKIP_SMUDGE", "1"),
        ("GCM_INTERACTIVE", "Never"),
        ("HOME", "/nonexistent"),
        ("XDG_CONFIG_HOME", "/nonexistent"),
        ("LANG", "C"),
        ("LC_ALL", "C"),
    ]
    .into_iter()
    .map(|(k, v)| (k.into(), v.into()))
    .collect()
}

#[cfg(test)]
#[path = "repository_candidate_tests.rs"]
mod tests;

#[cfg(test)]
fn test_token() -> CancellationToken {
    CancellationToken::root(
        agentmage_kernel_contracts::BoundaryKind::Kernel,
        agentmage_kernel_contracts::TaskId::from_raw("candidate-setup-test"),
        agentmage_kernel_contracts::CorrelationId::from_raw("candidate-setup-test"),
    )
}
