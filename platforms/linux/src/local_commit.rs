//! Linux temporary-index construction and externally pinned OpenPGP local commits.

use std::collections::BTreeMap;
use std::fmt;
use std::fs;
use std::io::{Read, Write};
use std::os::unix::ffi::OsStrExt as _;
use std::os::unix::fs::MetadataExt as _;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use agentmage_kernel_contracts::OperationOutcome;
use agentmage_kernel_engine::local_commit::{
    BoundedLocalCommitExecutor, CandidateTreeBlob, CandidateTreePlan, CandidateTreePlatformResult,
    CommitFileMode, CommitSignerKind, LocalCommitLaunchPermit, LocalCommitPlatformResult,
    PinnedCommitSigner, SignerInspectionSource, verify_candidate_tree_plan,
};
use agentmage_kernel_engine::propagation::CancellationToken;
use rustix::process::getuid;
use sha2::{Digest, Sha256};

use crate::repository_safety::{LinuxGitArtifact, LinuxRepositoryCollector, LinuxRepositoryScope};

const MAX_CAPTURE_BYTES: usize = 4 * 1024 * 1024;
const MAX_POSTIMAGE_BYTES: usize = 64 * 1024 * 1024;
#[allow(
    dead_code,
    reason = "candidate construction remains dormant until an exact product coordinator mediates it"
)]
const MAX_TOTAL_POSTIMAGE_BYTES: usize = 256 * 1024 * 1024;
const MAX_TRUSTED_EXECUTABLE_BYTES: u64 = 64 * 1024 * 1024;
const PROCESS_TIMEOUT: Duration = Duration::from_secs(30);
const POLL_INTERVAL: Duration = Duration::from_millis(10);

/// Stable Linux local-commit adapter failure class.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LinuxLocalCommitErrorKind {
    /// Git, GnuPG, or an external keyring is not trusted and pinned.
    ArtifactDenied,
    /// An owned path, temporary index, or postimage is invalid.
    ScopeDenied,
    /// Candidate-tree or local-commit input is stale or malformed.
    PlanDenied,
    /// A bounded local process failed or exceeded its resource boundary.
    ExecutionFailed,
    /// Independent commit or signature verification failed.
    VerificationFailed,
}

impl LinuxLocalCommitErrorKind {
    /// Returns one stable content-free code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::ArtifactDenied => "linux.local_commit.artifact.denied",
            Self::ScopeDenied => "linux.local_commit.scope.denied",
            Self::PlanDenied => "linux.local_commit.plan.denied",
            Self::ExecutionFailed => "linux.local_commit.execution.failed",
            Self::VerificationFailed => "linux.local_commit.verification.failed",
        }
    }
}

/// Redacted Linux local-commit error.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LinuxLocalCommitError {
    kind: LinuxLocalCommitErrorKind,
}

impl LinuxLocalCommitError {
    /// Returns the stable failure class.
    #[must_use]
    pub const fn kind(self) -> LinuxLocalCommitErrorKind {
        self.kind
    }
}

impl fmt::Display for LinuxLocalCommitError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.kind.code())
    }
}

impl std::error::Error for LinuxLocalCommitError {}

/// Exact approved postimage held only for one candidate-tree attempt.
#[allow(
    dead_code,
    reason = "candidate construction remains dormant until an exact product coordinator mediates it"
)]
pub struct LinuxApprovedPostimage {
    operation_id: String,
    bytes: Vec<u8>,
}

impl LinuxApprovedPostimage {
    /// Holds one exact postimage without logging or serializing its bytes.
    #[allow(
        dead_code,
        reason = "candidate construction remains dormant until an exact product coordinator mediates it"
    )]
    pub fn new(
        operation_id: impl Into<String>,
        bytes: Vec<u8>,
    ) -> Result<Self, LinuxLocalCommitError> {
        let operation_id = operation_id.into();
        if operation_id.is_empty()
            || operation_id.len() > 128
            || bytes.len() > MAX_POSTIMAGE_BYTES
            || !operation_id.bytes().all(|byte| {
                byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b':' | b'-')
            })
        {
            return Err(error(LinuxLocalCommitErrorKind::ScopeDenied));
        }
        Ok(Self {
            operation_id,
            bytes,
        })
    }
}

impl fmt::Debug for LinuxApprovedPostimage {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("LinuxApprovedPostimage")
            .field("operation_id", &self.operation_id)
            .field("byte_length", &self.bytes.len())
            .finish_non_exhaustive()
    }
}

/// Computes the exact future temporary-index path identity for a candidate build.
#[allow(
    dead_code,
    reason = "candidate construction remains dormant until an exact product coordinator mediates it"
)]
pub fn linux_candidate_index_path_sha256(
    scope: &LinuxRepositoryScope,
    candidate_build_id: &str,
) -> Result<String, LinuxLocalCommitError> {
    if candidate_build_id.is_empty()
        || candidate_build_id.len() > 128
        || candidate_build_id.contains('/')
    {
        return Err(error(LinuxLocalCommitErrorKind::ScopeDenied));
    }
    Ok(hash(
        scope
            .owned_root()
            .join(format!("candidate-index-{candidate_build_id}"))
            .as_os_str()
            .as_bytes(),
    ))
}

/// Linux candidate-tree builder using only an AgentMage-owned temporary index.
#[derive(Debug)]
#[allow(
    dead_code,
    reason = "candidate construction remains dormant until an exact product coordinator mediates it"
)]
pub struct LinuxCandidateTreeBuilder {
    collector: LinuxRepositoryCollector,
    scope: LinuxRepositoryScope,
}

impl LinuxCandidateTreeBuilder {
    /// Creates an inert candidate-tree builder.
    #[must_use]
    #[allow(
        dead_code,
        reason = "candidate construction remains dormant until an exact product coordinator mediates it"
    )]
    pub const fn new(collector: LinuxRepositoryCollector, scope: LinuxRepositoryScope) -> Self {
        Self { collector, scope }
    }

    /// Builds one exact tree without changing the user index or any ref.
    #[allow(
        dead_code,
        reason = "candidate construction remains dormant until an exact product coordinator mediates it"
    )]
    pub fn build(
        &self,
        plan: &CandidateTreePlan,
        postimages: Vec<LinuxApprovedPostimage>,
        cancellation: &CancellationToken,
    ) -> Result<CandidateTreePlatformResult, LinuxLocalCommitError> {
        verify_candidate_tree_plan(plan)
            .map_err(|_| error(LinuxLocalCommitErrorKind::PlanDenied))?;
        let before = self
            .collector
            .collect(&self.scope)
            .map_err(|_| error(LinuxLocalCommitErrorKind::PlanDenied))?;
        let temporary_index = self
            .scope
            .owned_root()
            .join(format!("candidate-index-{}", plan.candidate_build_id));
        if before.manifest_sha256 != plan.preservation_manifest_sha256
            || before.repository_sha256 != plan.repository_sha256
            || before.index_sha256 != plan.user_index_sha256
            || plan.temporary_index_path_sha256 != hash(temporary_index.as_os_str().as_bytes())
            || temporary_index.exists()
        {
            return Err(error(LinuxLocalCommitErrorKind::PlanDenied));
        }
        let postimages = postimages
            .into_iter()
            .map(|postimage| (postimage.operation_id, postimage.bytes))
            .collect::<BTreeMap<_, _>>();
        if postimages.len() != plan.files.len()
            || postimages
                .values()
                .try_fold(0_usize, |total, bytes| total.checked_add(bytes.len()))
                .is_none_or(|total| total > MAX_TOTAL_POSTIMAGE_BYTES)
        {
            return Err(error(LinuxLocalCommitErrorKind::PlanDenied));
        }
        for file in &plan.files {
            let bytes = postimages
                .get(&file.operation_id)
                .ok_or_else(|| error(LinuxLocalCommitErrorKind::PlanDenied))?;
            if hash(bytes) != file.postimage_sha256 {
                return Err(error(LinuxLocalCommitErrorKind::PlanDenied));
            }
        }

        let runner = GitRunner::new(
            self.collector.git(),
            &self.scope,
            Some(&temporary_index),
            None,
        );
        let operation = (|| {
            runner.run(&["read-tree", &plan.parent_object], &[], cancellation)?;
            let mut blobs = Vec::with_capacity(plan.files.len());
            for file in &plan.files {
                let bytes = postimages
                    .get(&file.operation_id)
                    .ok_or_else(|| error(LinuxLocalCommitErrorKind::PlanDenied))?;
                let blob = object_output(runner.run(
                    &["hash-object", "-w", "--stdin"],
                    bytes,
                    cancellation,
                )?)?;
                let mode = match file.mode {
                    CommitFileMode::Regular => "100644",
                    CommitFileMode::Executable => "100755",
                };
                let cache_info = format!("{mode},{blob},{}", file.path);
                runner.run(
                    &["update-index", "--add", "--cacheinfo", &cache_info],
                    &[],
                    cancellation,
                )?;
                blobs.push(CandidateTreeBlob {
                    operation_id: file.operation_id.clone(),
                    postimage_sha256: file.postimage_sha256.clone(),
                    blob_object: blob,
                });
            }
            blobs.sort();
            let tree = object_output(runner.run(&["write-tree"], &[], cancellation)?)?;
            Ok::<_, LinuxLocalCommitError>((tree, blobs))
        })();
        let cleanup_verified = !temporary_index.exists()
            || fs::remove_file(&temporary_index).is_ok() && !temporary_index.exists();
        let after = self
            .collector
            .collect(&self.scope)
            .map_err(|_| error(LinuxLocalCommitErrorKind::ExecutionFailed))?;
        let (candidate_tree, blobs) = operation?;
        if !cleanup_verified {
            return Err(error(LinuxLocalCommitErrorKind::ExecutionFailed));
        }
        Ok(CandidateTreePlatformResult {
            before,
            after: after.clone(),
            outcome: OperationOutcome::Succeeded,
            candidate_tree: Some(candidate_tree),
            blobs,
            user_index_after_sha256: after.index_sha256,
            owned_temporary_index: true,
            cleanup_verified,
            hooks_executed: false,
            filters_executed: false,
            repository_selected_program_used: false,
            network_used: false,
            platform_code: "linux.git.candidate.succeeded".to_owned(),
        })
    }
}

/// Verified external OpenPGP signer and keyring held outside a repository.
pub struct LinuxOpenPgpSigner {
    executable: PathBuf,
    executable_sha256: String,
    keyring: PathBuf,
    pinned: PinnedCommitSigner,
}

impl LinuxOpenPgpSigner {
    /// Inspects one approved external secret-key identity without exposing key material.
    pub fn inspect(
        signer_id: &str,
        executable: impl AsRef<Path>,
        keyring: impl AsRef<Path>,
        key_fingerprint: &str,
        repository_scope: &LinuxRepositoryScope,
    ) -> Result<Self, LinuxLocalCommitError> {
        let (executable, executable_sha256) = verify_root_executable(executable.as_ref())?;
        let keyring = verify_private_keyring(keyring.as_ref())?;
        if keyring.starts_with(repository_scope.checkout_root())
            || keyring.starts_with(repository_scope.git_directory())
            || keyring.starts_with(repository_scope.owned_root())
        {
            return Err(error(LinuxLocalCommitErrorKind::ArtifactDenied));
        }
        let fingerprint = key_fingerprint.to_ascii_lowercase();
        if !valid_fingerprint(&fingerprint) {
            return Err(error(LinuxLocalCommitErrorKind::ArtifactDenied));
        }
        let listing = run_gpg(
            &executable,
            &keyring,
            &[
                "--with-colons",
                "--fingerprint",
                "--list-secret-keys",
                &fingerprint,
            ],
        )?;
        let listing_text = String::from_utf8_lossy(&listing).to_ascii_lowercase();
        if !listing_text
            .split(':')
            .any(|field| field.trim() == fingerprint)
        {
            return Err(error(LinuxLocalCommitErrorKind::ArtifactDenied));
        }
        let public_key = run_gpg(&executable, &keyring, &["--export", &fingerprint])?;
        if public_key.is_empty() {
            return Err(error(LinuxLocalCommitErrorKind::ArtifactDenied));
        }
        let mut inspection = listing;
        inspection.extend_from_slice(&public_key);
        let pinned = PinnedCommitSigner::seal(PinnedCommitSigner {
            schema_version: 0,
            signer_id: signer_id.to_owned(),
            kind: CommitSignerKind::OpenPgp,
            source: SignerInspectionSource::ExternalKeyring,
            signer_path_sha256: hash(executable.as_os_str().as_bytes()),
            signer_executable_sha256: executable_sha256.clone(),
            key_fingerprint: fingerprint,
            public_key_sha256: hash(&public_key),
            inspection_evidence_sha256: hash(&inspection),
            repository_selected_program_used: false,
            unsigned_fallback: false,
            signer_sha256: String::new(),
        })
        .map_err(|_| error(LinuxLocalCommitErrorKind::ArtifactDenied))?;
        Ok(Self {
            executable,
            executable_sha256,
            keyring,
            pinned,
        })
    }

    /// Returns the content-minimized signer report used by a commit plan.
    #[must_use]
    pub const fn pinned(&self) -> &PinnedCommitSigner {
        &self.pinned
    }

    fn revalidate(&self) -> Result<(), LinuxLocalCommitError> {
        let (_, digest) = verify_root_executable(&self.executable)?;
        let keyring = verify_private_keyring(&self.keyring)?;
        if digest != self.executable_sha256 || keyring != self.keyring {
            return Err(error(LinuxLocalCommitErrorKind::ArtifactDenied));
        }
        self.pinned
            .verify()
            .map_err(|_| error(LinuxLocalCommitErrorKind::ArtifactDenied))
    }
}

impl fmt::Debug for LinuxOpenPgpSigner {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("LinuxOpenPgpSigner")
            .field("executable_sha256", &self.executable_sha256)
            .field("signer_id", &self.pinned.signer_id)
            .finish_non_exhaustive()
    }
}

/// Linux executor for one exact signed local commit and task-branch compare-and-swap.
#[derive(Debug)]
pub struct LinuxLocalCommitExecutor {
    collector: LinuxRepositoryCollector,
    scope: LinuxRepositoryScope,
    signer: LinuxOpenPgpSigner,
}

impl LinuxLocalCommitExecutor {
    /// Creates an inert local commit executor around verified artifacts.
    #[must_use]
    pub const fn new(
        collector: LinuxRepositoryCollector,
        scope: LinuxRepositoryScope,
        signer: LinuxOpenPgpSigner,
    ) -> Self {
        Self {
            collector,
            scope,
            signer,
        }
    }

    fn execute_plan(
        &self,
        permit: LocalCommitLaunchPermit<'_>,
        cancellation: &CancellationToken,
    ) -> LocalCommitPlatformResult {
        self.run_plan(permit.plan(), permit.before(), cancellation)
    }

    fn run_plan(
        &self,
        plan: &agentmage_kernel_engine::local_commit::LocalCommitPlan,
        approved_before: &agentmage_kernel_engine::repository_safety::RepositoryPreservationManifest,
        cancellation: &CancellationToken,
    ) -> LocalCommitPlatformResult {
        let before = match self.collector.collect(&self.scope) {
            Ok(value) => value,
            Err(_) => return failed_commit_result(approved_before, "linux.git.commit.preflight"),
        };
        if before != *approved_before
            || self.signer.revalidate().is_err()
            || self.signer.pinned != plan.signer
        {
            return failed_commit_result(&before, "linux.git.commit.plan_denied");
        }
        let runner = GitRunner::new(self.collector.git(), &self.scope, None, Some(&self.signer));
        let branch_before = runner
            .run(
                &["rev-parse", "--verify", &plan.task_branch],
                &[],
                cancellation,
            )
            .and_then(object_output);
        if branch_before.as_deref() != Ok(plan.parent_object.as_str()) {
            return failed_commit_result(&before, "linux.git.commit.branch_stale");
        }
        let signed = format!("-S{}", plan.signer.key_fingerprint);
        let mut environment = BTreeMap::new();
        environment.insert("GIT_AUTHOR_NAME".to_owned(), plan.author.name.clone());
        environment.insert("GIT_AUTHOR_EMAIL".to_owned(), plan.author.email.clone());
        environment.insert("GIT_COMMITTER_NAME".to_owned(), plan.committer.name.clone());
        environment.insert(
            "GIT_COMMITTER_EMAIL".to_owned(),
            plan.committer.email.clone(),
        );
        let date = format_git_date(plan.timestamp_epoch_seconds, plan.timezone_offset_minutes);
        environment.insert("GIT_AUTHOR_DATE".to_owned(), date.clone());
        environment.insert("GIT_COMMITTER_DATE".to_owned(), date);
        let commit = runner
            .run_with_environment(
                &[
                    "commit-tree",
                    &signed,
                    &plan.candidate_tree,
                    "-p",
                    &plan.parent_object,
                ],
                plan.message.as_bytes(),
                cancellation,
                &environment,
            )
            .and_then(object_output);
        let Ok(commit_object) = commit else {
            return failed_commit_result(&before, "linux.git.commit.creation_failed");
        };
        let raw = runner.run(&["cat-file", "commit", &commit_object], &[], cancellation);
        let signature = runner.run(
            &["verify-commit", "--raw", &commit_object],
            &[],
            cancellation,
        );
        let (Ok(raw), Ok(signature)) = (raw, signature) else {
            return failed_commit_result(&before, "linux.git.commit.verification_failed");
        };
        if !verify_raw_commit(&raw, plan)
            || !String::from_utf8_lossy(&signature)
                .to_ascii_lowercase()
                .contains(&format!("validsig {}", plan.signer.key_fingerprint))
        {
            return failed_commit_result(&before, "linux.git.commit.verification_failed");
        }
        if runner
            .run(
                &[
                    "update-ref",
                    "--no-deref",
                    &plan.task_branch,
                    &commit_object,
                    &plan.parent_object,
                ],
                &[],
                cancellation,
            )
            .is_err()
        {
            return failed_commit_result(&before, "linux.git.commit.compare_and_swap_failed");
        }
        let after = match self.collector.collect(&self.scope) {
            Ok(value) => value,
            Err(_) => return failed_commit_result(&before, "linux.git.commit.postflight"),
        };
        LocalCommitPlatformResult {
            outcome: OperationOutcome::Succeeded,
            before,
            after: after.clone(),
            commit_object: Some(commit_object.clone()),
            observed_parent: Some(plan.parent_object.clone()),
            observed_tree: Some(plan.candidate_tree.clone()),
            observed_message_sha256: Some(plan.message_sha256.clone()),
            observed_identity_sha256: Some(plan.identity_sha256.clone()),
            observed_signer_fingerprint: Some(plan.signer.key_fingerprint.clone()),
            signature_verified: true,
            task_branch_before: Some(plan.parent_object.clone()),
            task_branch_after: Some(commit_object),
            user_index_after_sha256: after.index_sha256,
            remotes_after_sha256: after.remotes_sha256,
            hooks_executed: false,
            filters_executed: false,
            repository_selected_program_used: false,
            network_used: false,
            cleanup_verified: true,
            platform_code: "linux.git.commit.succeeded".to_owned(),
        }
    }
}

impl BoundedLocalCommitExecutor for LinuxLocalCommitExecutor {
    fn execute(
        &mut self,
        permit: LocalCommitLaunchPermit<'_>,
        cancellation: &CancellationToken,
    ) -> LocalCommitPlatformResult {
        self.execute_plan(permit, cancellation)
    }
}

struct GitRunner<'a> {
    git: &'a LinuxGitArtifact,
    scope: &'a LinuxRepositoryScope,
    temporary_index: Option<&'a Path>,
    signer: Option<&'a LinuxOpenPgpSigner>,
}

impl<'a> GitRunner<'a> {
    const fn new(
        git: &'a LinuxGitArtifact,
        scope: &'a LinuxRepositoryScope,
        temporary_index: Option<&'a Path>,
        signer: Option<&'a LinuxOpenPgpSigner>,
    ) -> Self {
        Self {
            git,
            scope,
            temporary_index,
            signer,
        }
    }

    fn run(
        &self,
        arguments: &[&str],
        input: &[u8],
        cancellation: &CancellationToken,
    ) -> Result<Vec<u8>, LinuxLocalCommitError> {
        self.run_with_environment(arguments, input, cancellation, &BTreeMap::new())
    }

    fn run_with_environment(
        &self,
        arguments: &[&str],
        input: &[u8],
        cancellation: &CancellationToken,
        extra_environment: &BTreeMap<String, String>,
    ) -> Result<Vec<u8>, LinuxLocalCommitError> {
        if cancellation.is_cancelled() || input.len() > MAX_POSTIMAGE_BYTES {
            return Err(error(LinuxLocalCommitErrorKind::ExecutionFailed));
        }
        let mut command = Command::new(self.git.launch_path());
        command
            .env_clear()
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_TERMINAL_PROMPT", "0")
            .env("GIT_OPTIONAL_LOCKS", "0")
            .env("GIT_LFS_SKIP_SMUDGE", "1")
            .env("GCM_INTERACTIVE", "Never")
            .env("HOME", "/nonexistent")
            .env("XDG_CONFIG_HOME", "/nonexistent")
            .env("LANG", "C")
            .env("LC_ALL", "C")
            .envs(extra_environment)
            .args(hardened_git_prefix())
            .arg(format!(
                "--git-dir={}",
                self.scope.git_directory().display()
            ))
            .arg(format!(
                "--work-tree={}",
                self.scope.checkout_root().display()
            ));
        if let Some(index) = self.temporary_index {
            command.env("GIT_INDEX_FILE", index);
        }
        if let Some(signer) = self.signer {
            command
                .env("GNUPGHOME", &signer.keyring)
                .arg("-c")
                .arg(format!("gpg.program={}", signer.executable.display()))
                .arg("-c")
                .arg("gpg.format=openpgp");
        }
        command
            .args(arguments)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let mut child = command
            .spawn()
            .map_err(|_| error(LinuxLocalCommitErrorKind::ExecutionFailed))?;
        let Some(mut stdin) = child.stdin.take() else {
            return Err(error(LinuxLocalCommitErrorKind::ExecutionFailed));
        };
        stdin
            .write_all(input)
            .map_err(|_| error(LinuxLocalCommitErrorKind::ExecutionFailed))?;
        drop(stdin);
        let captured = wait_capture(&mut child, cancellation)?;
        if captured.status.success() {
            let mut combined = captured.stdout;
            combined.extend_from_slice(&captured.stderr);
            Ok(combined)
        } else {
            Err(error(LinuxLocalCommitErrorKind::ExecutionFailed))
        }
    }
}

struct CapturedProcess {
    status: ExitStatus,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

fn wait_capture(
    child: &mut Child,
    cancellation: &CancellationToken,
) -> Result<CapturedProcess, LinuxLocalCommitError> {
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| error(LinuxLocalCommitErrorKind::ExecutionFailed))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| error(LinuxLocalCommitErrorKind::ExecutionFailed))?;
    let stdout_reader = thread::spawn(move || read_bounded(stdout));
    let stderr_reader = thread::spawn(move || read_bounded(stderr));
    let deadline = Instant::now() + PROCESS_TIMEOUT;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break Some(status),
            Ok(None) if cancellation.is_cancelled() || Instant::now() >= deadline => {
                let _ = child.kill();
                let _ = child.wait();
                break None;
            }
            Ok(None) => thread::sleep(POLL_INTERVAL),
            Err(_) => {
                let _ = child.kill();
                let _ = child.wait();
                break None;
            }
        }
    };
    let stdout = stdout_reader
        .join()
        .ok()
        .and_then(Result::ok)
        .ok_or_else(|| error(LinuxLocalCommitErrorKind::ExecutionFailed))?;
    let stderr = stderr_reader
        .join()
        .ok()
        .and_then(Result::ok)
        .ok_or_else(|| error(LinuxLocalCommitErrorKind::ExecutionFailed))?;
    let status = status.ok_or_else(|| error(LinuxLocalCommitErrorKind::ExecutionFailed))?;
    Ok(CapturedProcess {
        status,
        stdout,
        stderr,
    })
}

fn read_bounded(mut input: impl Read) -> Result<Vec<u8>, LinuxLocalCommitError> {
    let mut output = Vec::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let count = input
            .read(&mut buffer)
            .map_err(|_| error(LinuxLocalCommitErrorKind::ExecutionFailed))?;
        if count == 0 {
            return Ok(output);
        }
        if output.len().saturating_add(count) > MAX_CAPTURE_BYTES {
            return Err(error(LinuxLocalCommitErrorKind::ExecutionFailed));
        }
        output.extend_from_slice(&buffer[..count]);
    }
}

fn hardened_git_prefix() -> [&'static str; 35] {
    [
        "--no-pager",
        "-c",
        "core.hooksPath=/dev/null",
        "-c",
        "core.fsmonitor=false",
        "-c",
        "core.askPass=",
        "-c",
        "credential.helper=",
        "-c",
        "credential.interactive=never",
        "-c",
        "diff.external=",
        "-c",
        "diff.trustExitCode=false",
        "-c",
        "core.pager=cat",
        "-c",
        "maintenance.auto=false",
        "-c",
        "gc.auto=0",
        "-c",
        "submodule.recurse=false",
        "-c",
        "filter.lfs.clean=",
        "-c",
        "filter.lfs.smudge=",
        "-c",
        "filter.lfs.process=",
        "-c",
        "filter.lfs.required=false",
        "-c",
        "core.attributesFile=/dev/null",
        "-c",
        "protocol.allow=never",
    ]
}

fn object_output(output: Vec<u8>) -> Result<String, LinuxLocalCommitError> {
    let value = String::from_utf8(output)
        .map_err(|_| error(LinuxLocalCommitErrorKind::VerificationFailed))?
        .trim()
        .to_owned();
    if matches!(value.len(), 40 | 64)
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
    {
        Ok(value)
    } else {
        Err(error(LinuxLocalCommitErrorKind::VerificationFailed))
    }
}

fn verify_raw_commit(
    raw: &[u8],
    plan: &agentmage_kernel_engine::local_commit::LocalCommitPlan,
) -> bool {
    let Some(separator) = raw.windows(2).position(|window| window == b"\n\n") else {
        return false;
    };
    let headers = String::from_utf8_lossy(&raw[..separator]);
    let mut message = &raw[separator + 2..];
    if message.ends_with(b"\n") {
        message = &message[..message.len() - 1];
    }
    let timezone = git_timezone(plan.timezone_offset_minutes);
    let author = format!(
        "author {} <{}> {} {timezone}",
        plan.author.name, plan.author.email, plan.timestamp_epoch_seconds
    );
    let committer = format!(
        "committer {} <{}> {} {timezone}",
        plan.committer.name, plan.committer.email, plan.timestamp_epoch_seconds
    );
    headers
        .lines()
        .any(|line| line == format!("tree {}", plan.candidate_tree))
        && headers
            .lines()
            .any(|line| line == format!("parent {}", plan.parent_object))
        && headers.lines().any(|line| line == author)
        && headers.lines().any(|line| line == committer)
        && message == plan.message.as_bytes()
        && headers.lines().any(|line| line.starts_with("gpgsig "))
}

fn format_git_date(epoch_seconds: i64, offset_minutes: i16) -> String {
    format!("{epoch_seconds} {}", git_timezone(offset_minutes))
}

fn git_timezone(offset_minutes: i16) -> String {
    let sign = if offset_minutes < 0 { '-' } else { '+' };
    let absolute = i32::from(offset_minutes).abs();
    format!("{sign}{:02}{:02}", absolute / 60, absolute % 60)
}

fn run_gpg(
    executable: &Path,
    keyring: &Path,
    arguments: &[&str],
) -> Result<Vec<u8>, LinuxLocalCommitError> {
    let output = Command::new(executable)
        .env_clear()
        .env("GNUPGHOME", keyring)
        .env("HOME", "/nonexistent")
        .env("LANG", "C")
        .env("LC_ALL", "C")
        .args(["--batch", "--no-tty", "--no-auto-key-retrieve"])
        .args(arguments)
        .stdin(Stdio::null())
        .output()
        .map_err(|_| error(LinuxLocalCommitErrorKind::ArtifactDenied))?;
    if !output.status.success()
        || output.stdout.len() > MAX_CAPTURE_BYTES
        || output.stderr.len() > MAX_CAPTURE_BYTES
    {
        return Err(error(LinuxLocalCommitErrorKind::ArtifactDenied));
    }
    Ok(output.stdout)
}

fn verify_root_executable(path: &Path) -> Result<(PathBuf, String), LinuxLocalCommitError> {
    if !path.is_absolute() || path.symlink_metadata().is_err() {
        return Err(error(LinuxLocalCommitErrorKind::ArtifactDenied));
    }
    let canonical =
        fs::canonicalize(path).map_err(|_| error(LinuxLocalCommitErrorKind::ArtifactDenied))?;
    if canonical != path {
        return Err(error(LinuxLocalCommitErrorKind::ArtifactDenied));
    }
    let metadata =
        fs::metadata(&canonical).map_err(|_| error(LinuxLocalCommitErrorKind::ArtifactDenied))?;
    if !metadata.is_file()
        || metadata.uid() != 0
        || metadata.mode() & 0o022 != 0
        || metadata.mode() & 0o111 == 0
        || metadata.len() == 0
        || metadata.len() > MAX_TRUSTED_EXECUTABLE_BYTES
    {
        return Err(error(LinuxLocalCommitErrorKind::ArtifactDenied));
    }
    let mut executable =
        fs::File::open(&canonical).map_err(|_| error(LinuxLocalCommitErrorKind::ArtifactDenied))?;
    let mut digest = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    let mut total = 0_u64;
    loop {
        let count = executable
            .read(&mut buffer)
            .map_err(|_| error(LinuxLocalCommitErrorKind::ArtifactDenied))?;
        if count == 0 {
            break;
        }
        total = total
            .checked_add(count as u64)
            .filter(|value| *value <= MAX_TRUSTED_EXECUTABLE_BYTES)
            .ok_or_else(|| error(LinuxLocalCommitErrorKind::ArtifactDenied))?;
        digest.update(&buffer[..count]);
    }
    if total != metadata.len() {
        return Err(error(LinuxLocalCommitErrorKind::ArtifactDenied));
    }
    Ok((canonical, hex_digest(digest.finalize())))
}

fn verify_private_keyring(path: &Path) -> Result<PathBuf, LinuxLocalCommitError> {
    let canonical =
        fs::canonicalize(path).map_err(|_| error(LinuxLocalCommitErrorKind::ArtifactDenied))?;
    let metadata =
        fs::metadata(&canonical).map_err(|_| error(LinuxLocalCommitErrorKind::ArtifactDenied))?;
    if !canonical.is_absolute()
        || !metadata.is_dir()
        || metadata.uid() != getuid().as_raw()
        || metadata.mode() & 0o077 != 0
    {
        return Err(error(LinuxLocalCommitErrorKind::ArtifactDenied));
    }
    Ok(canonical)
}

fn valid_fingerprint(value: &str) -> bool {
    matches!(value.len(), 40 | 64)
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}

fn failed_commit_result(
    before: &agentmage_kernel_engine::repository_safety::RepositoryPreservationManifest,
    code: &str,
) -> LocalCommitPlatformResult {
    LocalCommitPlatformResult {
        outcome: OperationOutcome::Denied,
        before: before.clone(),
        after: before.clone(),
        commit_object: None,
        observed_parent: None,
        observed_tree: None,
        observed_message_sha256: None,
        observed_identity_sha256: None,
        observed_signer_fingerprint: None,
        signature_verified: false,
        task_branch_before: None,
        task_branch_after: None,
        user_index_after_sha256: before.index_sha256.clone(),
        remotes_after_sha256: before.remotes_sha256.clone(),
        hooks_executed: false,
        filters_executed: false,
        repository_selected_program_used: false,
        network_used: false,
        cleanup_verified: true,
        platform_code: code.to_owned(),
    }
}

fn hash(bytes: &[u8]) -> String {
    hex_digest(Sha256::digest(bytes))
}

fn hex_digest(bytes: impl AsRef<[u8]>) -> String {
    let bytes = bytes.as_ref();
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        use std::fmt::Write as _;
        write!(&mut output, "{byte:02x}").expect("writing to String cannot fail");
    }
    output
}

fn error(kind: LinuxLocalCommitErrorKind) -> LinuxLocalCommitError {
    LinuxLocalCommitError { kind }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt as _;
    use std::sync::atomic::{AtomicU64, Ordering};

    use agentmage_kernel_contracts::{BoundaryKind, CorrelationId, TaskId};
    use agentmage_kernel_engine::local_commit::{
        CommitIdentity, LocalCommitPlan, ManualCommitApprovalDecision, reconcile_candidate_tree,
        reconcile_local_commit, record_manual_commit_approval,
    };

    static TEMP_ID: AtomicU64 = AtomicU64::new(1);

    struct Fixture {
        root: PathBuf,
        checkout: PathBuf,
        owned: PathBuf,
    }

    impl Fixture {
        fn new() -> Self {
            let nonce = TEMP_ID.fetch_add(1, Ordering::Relaxed);
            let root = std::env::temp_dir().join(format!(
                "agentmage-local-commit-{}-{nonce}",
                std::process::id()
            ));
            let checkout = root.join("checkout");
            let owned = root.join("owned");
            fs::create_dir_all(checkout.join("src")).expect("checkout");
            fs::create_dir_all(&owned).expect("owned");
            fs::set_permissions(&checkout, fs::Permissions::from_mode(0o700))
                .expect("private checkout");
            fs::set_permissions(&owned, fs::Permissions::from_mode(0o700))
                .expect("private owned root");
            fs::write(checkout.join("src/lib.rs"), b"old\n").expect("fixture source");
            run_fixture(&checkout, &["init", "--initial-branch=main"]);
            run_fixture(&checkout, &["add", "--", "src/lib.rs"]);
            run_fixture(
                &checkout,
                &[
                    "-c",
                    "user.name=Fixture User",
                    "-c",
                    "user.email=fixture@example.test",
                    "commit",
                    "--no-gpg-sign",
                    "-m",
                    "initial",
                ],
            );
            fs::set_permissions(checkout.join(".git"), fs::Permissions::from_mode(0o700))
                .expect("private Git directory");
            Self {
                root,
                checkout,
                owned,
            }
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let keyring = self.root.join("keyring");
            if keyring.exists() {
                let _ = Command::new("/usr/bin/gpgconf")
                    .args([
                        "--homedir",
                        keyring.to_str().unwrap_or_default(),
                        "--kill",
                        "all",
                    ])
                    .output();
            }
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    fn run_fixture(repository: &Path, arguments: &[&str]) -> Vec<u8> {
        let output = Command::new("/usr/bin/git")
            .env_clear()
            .env("HOME", "/nonexistent")
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .args(arguments)
            .current_dir(repository)
            .output()
            .expect("fixture Git runs");
        assert!(output.status.success(), "fixture Git failed");
        output.stdout
    }

    fn cancellation(label: &str) -> CancellationToken {
        CancellationToken::root(
            BoundaryKind::Kernel,
            TaskId::from_raw(format!("task-{label}")),
            CorrelationId::from_raw(format!("correlation-{label}")),
        )
    }

    fn generate_signer(fixture: &Fixture, scope: &LinuxRepositoryScope) -> LinuxOpenPgpSigner {
        let keyring = fixture.root.join("keyring");
        fs::create_dir(&keyring).expect("keyring");
        fs::set_permissions(&keyring, fs::Permissions::from_mode(0o700))
            .expect("keyring permissions");
        let generated = Command::new("/usr/bin/gpg")
            .env_clear()
            .env("GNUPGHOME", &keyring)
            .env("HOME", "/nonexistent")
            .env("LANG", "C")
            .env("LC_ALL", "C")
            .args([
                "--batch",
                "--no-tty",
                "--pinentry-mode",
                "loopback",
                "--passphrase",
                "",
                "--quick-generate-key",
                "Fixture User <fixture@example.test>",
                "ed25519",
                "sign",
                "1d",
            ])
            .output()
            .expect("generate fixture key");
        assert!(generated.status.success(), "fixture key generation failed");
        let listed = Command::new("/usr/bin/gpg")
            .env_clear()
            .env("GNUPGHOME", &keyring)
            .env("HOME", "/nonexistent")
            .args([
                "--batch",
                "--with-colons",
                "--fingerprint",
                "--list-secret-keys",
            ])
            .output()
            .expect("list fixture key");
        assert!(listed.status.success(), "fixture key listing failed");
        let fingerprint = String::from_utf8(listed.stdout)
            .expect("listing UTF-8")
            .lines()
            .find_map(|line| {
                let fields = line.split(':').collect::<Vec<_>>();
                (fields.first() == Some(&"fpr"))
                    .then(|| fields.get(9).copied())
                    .flatten()
            })
            .expect("fixture fingerprint")
            .to_ascii_lowercase();
        LinuxOpenPgpSigner::inspect(
            "signer-fixture-0001",
            "/usr/bin/gpg",
            keyring,
            &fingerprint,
            scope,
        )
        .expect("inspect fixture signer")
    }

    #[test]
    fn native_candidate_tree_uses_only_owned_index_and_preserves_user_state() {
        let fixture = Fixture::new();
        let scope = LinuxRepositoryScope::verify(
            &fixture.checkout,
            fixture.checkout.join(".git"),
            &fixture.owned,
        )
        .expect("scope");
        let git = LinuxGitArtifact::verify("/usr/bin/git").expect("git artifact");
        let collector = LinuxRepositoryCollector::new(git);
        let before = collector.collect(&scope).expect("before manifest");
        let parent = String::from_utf8(run_fixture(&fixture.checkout, &["rev-parse", "HEAD"]))
            .expect("parent")
            .trim()
            .to_owned();
        let postimage = b"new\n".to_vec();
        let mut plan = CandidateTreePlan {
            schema_version: 1,
            candidate_build_id: "candidate-native-0001".to_owned(),
            repository_sha256: before.repository_sha256.clone(),
            review_packet_sha256: hash(b"review"),
            change_set_sha256: hash(b"change"),
            commit_group_sha256: hash(b"group"),
            parent_object: parent,
            temporary_index_path_sha256: linux_candidate_index_path_sha256(
                &scope,
                "candidate-native-0001",
            )
            .expect("index identity"),
            user_index_sha256: before.index_sha256.clone(),
            preservation_manifest_sha256: before.manifest_sha256.clone(),
            files: vec![
                agentmage_kernel_engine::local_commit::CandidateTreeFilePlan {
                    operation_id: "operation-01".to_owned(),
                    path: "src/lib.rs".to_owned(),
                    postimage_sha256: hash(&postimage),
                    mode: CommitFileMode::Regular,
                },
            ],
            hooks_enabled: false,
            filters_enabled: false,
            repository_program_selection_enabled: false,
            network_authority: false,
            plan_sha256: "0".repeat(64),
        };
        plan.plan_sha256 = hash(&serde_json::to_vec(&plan).expect("plan serialization"));
        let builder = LinuxCandidateTreeBuilder::new(collector, scope);
        let result = builder
            .build(
                &plan,
                vec![LinuxApprovedPostimage::new("operation-01", postimage).expect("postimage")],
                &cancellation("candidate"),
            )
            .expect("candidate tree");
        assert_eq!(result.before.index_sha256, result.after.index_sha256);
        assert_eq!(
            result.before.agentmage_refs_sha256,
            result.after.agentmage_refs_sha256
        );
        assert_ne!(
            result.before.object_database_sha256,
            result.after.object_database_sha256
        );
        let receipt = reconcile_candidate_tree(&plan, result).expect("candidate receipt");
        assert!(!receipt.ref_update_authority);
        assert!(
            !fixture
                .owned
                .join("candidate-index-candidate-native-0001")
                .exists()
        );
    }

    #[test]
    fn native_signed_commit_verifies_exact_content_and_only_updates_task_branch() {
        let fixture = Fixture::new();
        let parent = String::from_utf8(run_fixture(&fixture.checkout, &["rev-parse", "HEAD"]))
            .expect("parent")
            .trim()
            .to_owned();
        run_fixture(
            &fixture.checkout,
            &["update-ref", "refs/heads/agentmage/tasks/fixture", &parent],
        );
        let scope = LinuxRepositoryScope::verify(
            &fixture.checkout,
            fixture.checkout.join(".git"),
            &fixture.owned,
        )
        .expect("scope");
        let signer = generate_signer(&fixture, &scope);
        let git = LinuxGitArtifact::verify("/usr/bin/git").expect("git artifact");
        let collector = LinuxRepositoryCollector::new(git);
        let candidate_before = collector.collect(&scope).expect("candidate before");
        let postimage = b"signed new value\n".to_vec();
        let mut candidate_plan = CandidateTreePlan {
            schema_version: 1,
            candidate_build_id: "candidate-signed-0001".to_owned(),
            repository_sha256: candidate_before.repository_sha256.clone(),
            review_packet_sha256: hash(b"review-signed"),
            change_set_sha256: hash(b"change-signed"),
            commit_group_sha256: hash(b"group-signed"),
            parent_object: parent.clone(),
            temporary_index_path_sha256: linux_candidate_index_path_sha256(
                &scope,
                "candidate-signed-0001",
            )
            .expect("index identity"),
            user_index_sha256: candidate_before.index_sha256.clone(),
            preservation_manifest_sha256: candidate_before.manifest_sha256.clone(),
            files: vec![
                agentmage_kernel_engine::local_commit::CandidateTreeFilePlan {
                    operation_id: "operation-01".to_owned(),
                    path: "src/lib.rs".to_owned(),
                    postimage_sha256: hash(&postimage),
                    mode: CommitFileMode::Regular,
                },
            ],
            hooks_enabled: false,
            filters_enabled: false,
            repository_program_selection_enabled: false,
            network_authority: false,
            plan_sha256: "0".repeat(64),
        };
        candidate_plan.plan_sha256 =
            hash(&serde_json::to_vec(&candidate_plan).expect("candidate serialization"));
        let builder = LinuxCandidateTreeBuilder::new(collector, scope.clone());
        let candidate_result = builder
            .build(
                &candidate_plan,
                vec![LinuxApprovedPostimage::new("operation-01", postimage).expect("postimage")],
                &cancellation("signed-candidate"),
            )
            .expect("candidate tree");
        let commit_before = candidate_result.after.clone();
        let candidate_receipt =
            reconcile_candidate_tree(&candidate_plan, candidate_result).expect("candidate receipt");
        let author = CommitIdentity {
            name: "Fixture User".to_owned(),
            email: "fixture@example.test".to_owned(),
        };
        let message = "feat: apply approved behavior change\n\nFiles: 1".to_owned();
        let mut commit_plan = LocalCommitPlan {
            schema_version: 1,
            commit_transaction_id: "commit-native-0001".to_owned(),
            repository_sha256: commit_before.repository_sha256.clone(),
            review_packet_sha256: candidate_plan.review_packet_sha256.clone(),
            candidate_tree_receipt_sha256: candidate_receipt.receipt_sha256.clone(),
            commit_group_sha256: candidate_plan.commit_group_sha256.clone(),
            parent_object: parent.clone(),
            candidate_tree: candidate_receipt.candidate_tree.clone(),
            message_sha256: hash(message.as_bytes()),
            message,
            author: author.clone(),
            committer: author.clone(),
            identity_sha256: hash(
                &serde_json::to_vec(&(author.clone(), author)).expect("identity serialization"),
            ),
            timestamp_epoch_seconds: 1_787_000_000,
            timezone_offset_minutes: -300,
            signer: signer.pinned().clone(),
            task_branch: "refs/heads/agentmage/tasks/fixture".to_owned(),
            preservation_manifest_sha256: commit_before.manifest_sha256.clone(),
            user_index_sha256: commit_before.index_sha256.clone(),
            automatic_commit: false,
            push_authority: false,
            merge_authority: false,
            release_authority: false,
            history_rewrite_authority: false,
            reset_authority: false,
            discard_authority: false,
            force_authority: false,
            network_authority: false,
            plan_sha256: "0".repeat(64),
        };
        commit_plan.plan_sha256 =
            hash(&serde_json::to_vec(&commit_plan).expect("commit serialization"));
        let approval = record_manual_commit_approval(
            &commit_plan,
            ManualCommitApprovalDecision {
                approval_id: "approval-native-0001".to_owned(),
                approval_channel_sha256: hash(b"approval-channel"),
                approved_at_epoch_ms: 10_000,
                expires_at_epoch_ms: 20_000,
                human_confirmed: true,
            },
        )
        .expect("manual approval");
        let git = LinuxGitArtifact::verify("/usr/bin/git").expect("git artifact");
        let collector = LinuxRepositoryCollector::new(git);
        let executor = LinuxLocalCommitExecutor::new(collector, scope, signer);
        let platform =
            executor.run_plan(&commit_plan, &commit_before, &cancellation("signed-commit"));
        let receipt = reconcile_local_commit(
            &commit_plan,
            &approval,
            platform,
            "authority-native-0001",
            "attempt-native-0001",
            15_000,
        )
        .expect("signed commit receipt");
        assert!(receipt.signature_verified);
        assert!(receipt.user_index_unchanged);
        assert!(receipt.remotes_unchanged);
        assert!(!receipt.network_used);
        assert_eq!(
            String::from_utf8(run_fixture(
                &fixture.checkout,
                &["rev-parse", "refs/heads/agentmage/tasks/fixture"]
            ))
            .expect("branch")
            .trim(),
            receipt.commit_object
        );
        assert_eq!(
            String::from_utf8(run_fixture(&fixture.checkout, &["rev-parse", "main"]))
                .expect("main")
                .trim(),
            parent
        );
    }
}
