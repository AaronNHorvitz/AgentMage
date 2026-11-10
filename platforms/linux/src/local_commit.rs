// SPDX-License-Identifier: Apache-2.0
//! Linux temporary-index construction and externally pinned OpenPGP local commits.

use std::collections::BTreeMap;
use std::fmt;
use std::fs;
use std::io::Read;
use std::os::unix::ffi::OsStrExt as _;
use std::os::unix::fs::MetadataExt as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use agentmage_kernel_contracts::OperationOutcome;
use agentmage_kernel_engine::local_commit::{
    BoundedLocalCommitExecutor, CandidateTreeBlob, CandidateTreePlan, CandidateTreePlatformResult,
    CommitSignerKind, LocalCommitLaunchPermit, LocalCommitPlatformResult, PinnedCommitSigner,
    SignerInspectionSource, verify_candidate_tree_plan,
};
use agentmage_kernel_engine::propagation::CancellationToken;
use rustix::process::getuid;
use sha2::{Digest, Sha256};

use crate::repository_safety::{LinuxGitArtifact, LinuxRepositoryCollector, LinuxRepositoryScope};

const MAX_POSTIMAGE_BYTES: usize = 64 * 1024 * 1024;
#[allow(
    dead_code,
    reason = "candidate construction remains dormant until an exact product coordinator mediates it"
)]
const MAX_TOTAL_POSTIMAGE_BYTES: usize = 256 * 1024 * 1024;
const MAX_TRUSTED_EXECUTABLE_BYTES: u64 = 64 * 1024 * 1024;

#[path = "local_commit_verification.rs"]
pub(crate) mod verification;

#[path = "local_commit_inspection.rs"]
pub(crate) mod inspection;

#[path = "local_commit_signing.rs"]
pub(crate) mod signing;

#[path = "local_commit_packet.rs"]
pub(crate) mod packet;

#[path = "local_commit_execution.rs"]
mod execution;

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
    /// A signer child may remain owned and cleanup cannot be confirmed.
    CleanupUncertain,
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
            Self::CleanupUncertain => "linux.local_commit.cleanup.uncertain",
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
    let index = crate::repository_safety::candidate_index_path(scope, candidate_build_id)
        .ok_or_else(|| error(LinuxLocalCommitErrorKind::ScopeDenied))?;
    Ok(hash(index.as_os_str().as_bytes()))
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
        if cancellation.is_cancelled() {
            return Err(error(LinuxLocalCommitErrorKind::ExecutionFailed));
        }
        verify_candidate_tree_plan(plan)
            .map_err(|_| error(LinuxLocalCommitErrorKind::PlanDenied))?;
        let before = self
            .collector
            .collect(&self.scope)
            .map_err(|_| error(LinuxLocalCommitErrorKind::PlanDenied))?;
        let temporary_index_sha256 =
            linux_candidate_index_path_sha256(&self.scope, &plan.candidate_build_id)?;
        if before.manifest_sha256 != plan.preservation_manifest_sha256
            || before.repository_sha256 != plan.repository_sha256
            || before.index_sha256 != plan.user_index_sha256
            || plan.temporary_index_path_sha256 != temporary_index_sha256
        {
            return Err(error(LinuxLocalCommitErrorKind::PlanDenied));
        }
        let count = postimages.len();
        let postimages = postimages
            .into_iter()
            .map(|postimage| (postimage.operation_id, postimage.bytes))
            .collect::<BTreeMap<_, _>>();
        if count != postimages.len()
            || postimages.len() != plan.files.len()
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

        let mut session = crate::repository_safety::CandidateSession::new(&self.scope, plan)
            .map_err(|_| error(LinuxLocalCommitErrorKind::ScopeDenied))?;
        let operation = (|| {
            session.read_tree(self.collector.git(), cancellation)?;
            let mut blobs = Vec::with_capacity(plan.files.len());
            for file in &plan.files {
                let bytes = &postimages[&file.operation_id];
                let blob = session.add_postimage(
                    self.collector.git(),
                    &file.operation_id,
                    bytes,
                    cancellation,
                )?;
                blobs.push(CandidateTreeBlob {
                    operation_id: file.operation_id.clone(),
                    postimage_sha256: file.postimage_sha256.clone(),
                    blob_object: blob,
                });
            }
            blobs.sort();
            let tree = session.write_tree(self.collector.git(), cancellation)?;
            Ok::<_, crate::repository_safety::LinuxRepositoryError>((tree, blobs))
        })()
        .map_err(|_| error(LinuxLocalCommitErrorKind::ExecutionFailed));
        let cleanup_verified = session.cleanup();
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

/// Pinned external OpenPGP public identity; private-key usability is not admitted.
pub struct LinuxOpenPgpSigner {
    executable: PathBuf,
    executable_sha256: String,
    keyring: PathBuf,
    public_key: Vec<u8>,
    // Derived only from the sealed public listing; never a caller-selected path.
    keygrip: Option<String>,
    pinned: PinnedCommitSigner,
}

impl LinuxOpenPgpSigner {
    /// Pins one approved external public identity without inspecting private keys.
    ///
    /// This does not establish that a usable private signing key is available.
    pub fn inspect(
        signer_id: &str,
        executable: impl AsRef<Path>,
        keyring: impl AsRef<Path>,
        key_fingerprint: &str,
        repository_scope: &LinuxRepositoryScope,
    ) -> Result<Self, LinuxLocalCommitError> {
        let cancellation = CancellationToken::root(
            agentmage_kernel_contracts::BoundaryKind::Kernel,
            agentmage_kernel_contracts::TaskId::from_raw("local-signer-inspection"),
            agentmage_kernel_contracts::CorrelationId::from_raw("local-signer-inspection"),
        );
        Self::inspect_cancellable(
            signer_id,
            executable,
            keyring,
            key_fingerprint,
            repository_scope,
            &cancellation,
        )
    }

    /// Pins public identity under the caller's cancellation scope.
    ///
    /// The thirty-second budget covers capture and both offline workers. A
    /// cancelled attempt returns no signer; this never admits private-key use.
    pub fn inspect_cancellable(
        signer_id: &str,
        executable: impl AsRef<Path>,
        keyring: impl AsRef<Path>,
        key_fingerprint: &str,
        repository_scope: &LinuxRepositoryScope,
        cancellation: &CancellationToken,
    ) -> Result<Self, LinuxLocalCommitError> {
        let deadline = Instant::now() + Duration::from_secs(30);
        inspection::check_control(cancellation, deadline)?;
        let (executable, executable_sha256) =
            verify_root_executable_checked(executable.as_ref(), || {
                inspection::check_control(cancellation, deadline)
            })?;
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
        let (public_key, inspection, keygrip) = inspection::inspect(
            &executable,
            &executable_sha256,
            &keyring,
            &fingerprint,
            cancellation,
            deadline,
        )?;
        let pinned = PinnedCommitSigner::seal(PinnedCommitSigner {
            schema_version: 0,
            signer_id: signer_id.to_owned(),
            kind: CommitSignerKind::OpenPgp,
            source: SignerInspectionSource::ExternalKeyring,
            signer_path_sha256: hash(executable.as_os_str().as_bytes()),
            signer_executable_sha256: executable_sha256.clone(),
            key_fingerprint: fingerprint,
            public_key_sha256: hash(&public_key),
            inspection_evidence_sha256: inspection,
            repository_selected_program_used: false,
            unsigned_fallback: false,
            signer_sha256: String::new(),
        })
        .map_err(|_| error(LinuxLocalCommitErrorKind::ArtifactDenied))?;
        inspection::check_control(cancellation, deadline)?;
        Ok(Self {
            executable,
            executable_sha256,
            keyring,
            public_key,
            keygrip,
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
        let Ok(mut io) = crate::repository_safety::CommitIoSession::new(&self.scope, plan) else {
            return failed_commit_result(&before, "linux.git.commit.plan_denied");
        };
        // Only the kernel-permitted entry above can construct a native session.
        // The private execution seam admits no caller-supplied backend or bytes.
        execution::run(
            plan,
            before,
            &mut execution::NativeSession::new(self, &mut io, plan, cancellation),
        )
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

#[cfg(test)]
struct SignedGitRunner<'a> {
    git_path: &'a Path,
    scope: &'a LinuxRepositoryScope,
    signer: &'a LinuxOpenPgpSigner,
    deadline: Instant,
}

#[cfg(test)]
impl<'a> SignedGitRunner<'a> {
    #[cfg(test)]
    fn run(
        &self,
        arguments: &[&str],
        input: &[u8],
        cancellation: &CancellationToken,
    ) -> Result<crate::sandbox::supervision::LocalCommitCapture, LinuxLocalCommitError> {
        self.run_with_environment(arguments, input, cancellation, &BTreeMap::new())
    }

    fn run_with_environment(
        &self,
        arguments: &[&str],
        input: &[u8],
        cancellation: &CancellationToken,
        extra_environment: &BTreeMap<String, String>,
    ) -> Result<crate::sandbox::supervision::LocalCommitCapture, LinuxLocalCommitError> {
        if cancellation.is_cancelled()
            || Instant::now() >= self.deadline
            || input.len() > MAX_POSTIMAGE_BYTES
            || arguments.first() != Some(&"commit-tree")
        {
            return Err(error(LinuxLocalCommitErrorKind::ExecutionFailed));
        }
        let mut command = Command::new(self.git_path);
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
        command
            .env("GNUPGHOME", &self.signer.keyring)
            .arg("-c")
            .arg(format!("gpg.program={}", self.signer.executable.display()))
            .arg("-c")
            .arg(format!(
                "gpg.openpgp.program={}",
                self.signer.executable.display()
            ))
            .arg("-c")
            .arg("gpg.format=openpgp")
            .arg("-c")
            .arg("i18n.commitEncoding=UTF-8");
        // A sealed descriptor cannot block the parent writing a full pipe
        // before stdout/stderr readers and deadline observation have started.
        let payload = crate::sandbox::sealed_payload("agentmage-commit-message", input)
            .map_err(|_| error(LinuxLocalCommitErrorKind::ExecutionFailed))?;
        command
            .args(arguments)
            .stdin(Stdio::from(fs::File::from(payload)))
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let captured = crate::sandbox::supervision::capture_local_commit_process(
            command,
            Some(cancellation),
            self.deadline,
        )
        .map_err(|failure| {
            capture_error(failure.kind(), LinuxLocalCommitErrorKind::ExecutionFailed)
        })?;
        if captured.status.success() {
            Ok(captured)
        } else {
            Err(error(LinuxLocalCommitErrorKind::ExecutionFailed))
        }
    }
}

pub(crate) fn hardened_git_prefix() -> [&'static str; 36] {
    [
        "--no-pager",
        "--no-replace-objects",
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

#[cfg(test)]
fn object_output(output: Vec<u8>) -> Result<String, LinuxLocalCommitError> {
    let value = String::from_utf8(output)
        .map_err(|_| error(LinuxLocalCommitErrorKind::VerificationFailed))?
        .strip_suffix('\n')
        .ok_or_else(|| error(LinuxLocalCommitErrorKind::VerificationFailed))?
        .to_owned();
    if matches!(value.len(), 40 | 64)
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
        && value.bytes().any(|byte| byte != b'0')
    {
        Ok(value)
    } else {
        Err(error(LinuxLocalCommitErrorKind::VerificationFailed))
    }
}

#[cfg(test)]
fn signed_commit_arguments(
    plan: &agentmage_kernel_engine::local_commit::LocalCommitPlan,
) -> [String; 5] {
    // Pin the actual signing key, including when the selected fingerprint is
    // a primary with newer signing subkeys. Verification checks that exact key.
    [
        "commit-tree".into(),
        format!("-S{}!", plan.signer.key_fingerprint),
        plan.candidate_tree.clone(),
        "-p".into(),
        plan.parent_object.clone(),
    ]
}

fn verify_raw_commit(
    raw: &[u8],
    plan: &agentmage_kernel_engine::local_commit::LocalCommitPlan,
) -> bool {
    let Some(separator) = raw.windows(2).position(|window| window == b"\n\n") else {
        return false;
    };
    let Ok(headers) = std::str::from_utf8(&raw[..separator]) else {
        return false;
    };
    let message = &raw[separator + 2..];
    let timezone = git_timezone(plan.timezone_offset_minutes);
    let author = format!(
        "author {} <{}> {} {timezone}",
        plan.author.name, plan.author.email, plan.timestamp_epoch_seconds
    );
    let committer = format!(
        "committer {} <{}> {} {timezone}",
        plan.committer.name, plan.committer.email, plan.timestamp_epoch_seconds
    );
    let signature_header = match (plan.parent_object.len(), plan.candidate_tree.len()) {
        (40, 40) => "gpgsig -----BEGIN PGP SIGNATURE-----",
        (64, 64) => "gpgsig-sha256 -----BEGIN PGP SIGNATURE-----",
        _ => return false,
    };
    // Exact singleton headers: finding one approved parent among additional
    // parents, identities or trees cannot verify the planned commit.
    let mut lines = headers.split('\n');
    if lines.next() != Some(format!("tree {}", plan.candidate_tree).as_str())
        || lines.next() != Some(format!("parent {}", plan.parent_object).as_str())
        || lines.next() != Some(author.as_str())
        || lines.next() != Some(committer.as_str())
        || lines.next() != Some(signature_header)
        || message != plan.message.as_bytes()
    {
        return false;
    }
    let signature: Vec<_> = lines.collect();
    !signature.is_empty()
        && signature.iter().all(|line| line.starts_with(' '))
        && signature.last() == Some(&" -----END PGP SIGNATURE-----")
}

fn verify_signature_status(status: &[u8], fingerprint: &str) -> bool {
    let Ok(status) = std::str::from_utf8(status) else {
        return false;
    };
    if !valid_fingerprint(fingerprint)
        || status.len() > crate::sandbox::MAX_OUTPUT_BYTES
        || !status.ends_with('\n')
        || status.contains(['\0', '\r'])
    {
        return false;
    }
    let mut valid = 0;
    for line in status.lines() {
        let Some(record) = line.strip_prefix("[GNUPG:] ") else {
            return false;
        };
        let fields: Vec<_> = record.split_ascii_whitespace().collect();
        if fields.is_empty() {
            return false;
        }
        match fields.first().copied() {
            Some("VALIDSIG") => {
                if !(10..=11).contains(&fields.len())
                    || !fields[1].eq_ignore_ascii_case(fingerprint)
                {
                    return false;
                }
                valid += 1;
            }
            Some(
                "BADSIG" | "ERRSIG" | "REVKEYSIG" | "EXPKEYSIG" | "EXPSIG" | "NO_PUBKEY" | "NODATA"
                | "FAILURE" | "ERROR" | "KEYREVOKED" | "KEYEXPIRED" | "SIGEXPIRED",
            ) => return false,
            _ => (),
        }
    }
    valid == 1
}

#[cfg(test)]
fn format_git_date(epoch_seconds: i64, offset_minutes: i16) -> String {
    format!("{epoch_seconds} {}", git_timezone(offset_minutes))
}

fn git_timezone(offset_minutes: i16) -> String {
    let sign = if offset_minutes < 0 { '-' } else { '+' };
    let absolute = i32::from(offset_minutes).abs();
    format!("{sign}{:02}{:02}", absolute / 60, absolute % 60)
}

// Disposable cryptographic fixtures only. Product inspection has no direct fallback.
#[cfg(test)]
fn run_gpg(
    executable: &Path,
    keyring: &Path,
    arguments: &[&str],
    deadline: Instant,
) -> Result<Vec<u8>, LinuxLocalCommitError> {
    let mut command = Command::new(executable);
    command
        .env_clear()
        .env("GNUPGHOME", keyring)
        .env("HOME", "/nonexistent")
        .env("LANG", "C")
        .env("LC_ALL", "C")
        .args(["--batch", "--no-tty", "--no-auto-key-retrieve"])
        .args(arguments)
        .stdin(Stdio::null());
    let output = crate::sandbox::supervision::capture_local_commit_process(command, None, deadline)
        .map_err(|failure| {
            capture_error(failure.kind(), LinuxLocalCommitErrorKind::ArtifactDenied)
        })?;
    if !output.status.success() {
        return Err(error(LinuxLocalCommitErrorKind::ArtifactDenied));
    }
    Ok(output.into_stdout())
}

// Preserve the only cleanup claim made by direct capture. Other errors retain
// their caller's existing redacted class; no partial bytes or host paths escape.
fn capture_error(
    kind: crate::sandbox::LinuxSandboxErrorKind,
    fallback: LinuxLocalCommitErrorKind,
) -> LinuxLocalCommitError {
    error(
        if kind == crate::sandbox::LinuxSandboxErrorKind::CleanupUncertain {
            LinuxLocalCommitErrorKind::CleanupUncertain
        } else {
            fallback
        },
    )
}

fn failed_signer_result(
    before: &agentmage_kernel_engine::repository_safety::RepositoryPreservationManifest,
    failure: LinuxLocalCommitError,
    fallback: &str,
) -> LocalCommitPlatformResult {
    uncertain_commit_result(
        before,
        if failure.kind() == LinuxLocalCommitErrorKind::CleanupUncertain {
            "linux.git.commit.signer_cleanup_uncertain"
        } else {
            fallback
        },
    )
}

fn verify_root_executable(path: &Path) -> Result<(PathBuf, String), LinuxLocalCommitError> {
    verify_root_executable_checked(path, || Ok(()))
}

fn verify_root_executable_checked(
    path: &Path,
    mut check: impl FnMut() -> Result<(), LinuxLocalCommitError>,
) -> Result<(PathBuf, String), LinuxLocalCommitError> {
    check()?;
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
        check()?;
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
    check()?;
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
        cleanup_verified: false,
        platform_code: code.to_owned(),
    }
}

// After worker dispatch or signing starts, cleanup may be uncertain and objects
// or a ref may already have changed. The result
// schema requires manifest fields: these retain the last observation only,
// never a postflight observation. Failed outcome + unverified cleanup prevent
// reconciliation from treating those placeholders as preserved current state.
fn uncertain_commit_result(
    before: &agentmage_kernel_engine::repository_safety::RepositoryPreservationManifest,
    code: &str,
) -> LocalCommitPlatformResult {
    let mut result = failed_commit_result(before, code);
    result.outcome = OperationOutcome::Failed;
    result.cleanup_verified = false;
    result
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
pub(crate) mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt as _;
    use std::sync::atomic::{AtomicU64, Ordering};

    use agentmage_kernel_contracts::{BoundaryKind, CorrelationId, TaskId};
    use agentmage_kernel_engine::local_commit::{
        CommitFileMode, CommitIdentity, LocalCommitPlan, ManualCommitApprovalDecision,
        reconcile_candidate_tree, reconcile_local_commit, record_manual_commit_approval,
    };

    static TEMP_ID: AtomicU64 = AtomicU64::new(1);
    // Serialize fixtures that share the production owner's refusal of concurrent
    // admission; supervisor tests own local slots. Fixture assertion failures may
    // poison THIS test mutex, not the independent production capture owner.
    pub(super) static DIRECT_CAPTURE_TEST: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[test]
    fn local_commit_public_inspection_caller_cancellation_precedes_path_access() {
        struct UnreadPath;
        impl AsRef<Path> for UnreadPath {
            fn as_ref(&self) -> &Path {
                panic!("cancelled inspection must not consult paths");
            }
        }
        let fixture = Fixture::new();
        let scope = LinuxRepositoryScope::verify(
            &fixture.checkout,
            fixture.checkout.join(".git"),
            &fixture.owned,
        )
        .unwrap();
        let t = cancellation("public-inspect-cancel");
        t.cancel(agentmage_kernel_contracts::CancellationSignal {
            schema_version: agentmage_kernel_contracts::CONTRACT_SCHEMA_VERSION,
            cancellation_id: agentmage_kernel_contracts::CancellationId::from_raw("cancel-inspect"),
            task_id: agentmage_kernel_contracts::TaskId::from_raw("task-public-inspect-cancel"),
            correlation_id: agentmage_kernel_contracts::CorrelationId::from_raw(
                "correlation-public-inspect-cancel",
            ),
            reason: agentmage_kernel_contracts::CancellationReason::UserRequested,
            requested_by: agentmage_kernel_contracts::BoundaryKind::Kernel,
        })
        .unwrap();
        let failure = LinuxOpenPgpSigner::inspect_cancellable(
            "cancelled",
            UnreadPath,
            UnreadPath,
            &"a".repeat(40),
            &scope,
            &t,
        )
        .unwrap_err();
        assert_eq!(failure.kind(), LinuxLocalCommitErrorKind::ArtifactDenied);
    }

    // Inert input for parser and supervised-result tests. This is neither a
    // signer inspection nor a native execution/review/approval capability.
    pub(crate) fn synthetic_plan(repository_sha256: &str) -> LocalCommitPlan {
        let h = hash(b"synthetic local commit");
        let identity = CommitIdentity {
            name: "Fixture User".into(),
            email: "fixture@example.test".into(),
        };
        let signer = PinnedCommitSigner::seal(PinnedCommitSigner {
            schema_version: 0,
            signer_id: "signer-synthetic-0001".into(),
            kind: CommitSignerKind::OpenPgp,
            source: SignerInspectionSource::ExternalKeyring,
            signer_path_sha256: h.clone(),
            signer_executable_sha256: h.clone(),
            key_fingerprint: "a".repeat(40),
            public_key_sha256: h.clone(),
            inspection_evidence_sha256: h.clone(),
            repository_selected_program_used: false,
            unsigned_fallback: false,
            signer_sha256: String::new(),
        })
        .unwrap();
        let message = "Approved fixture change".to_owned();
        let mut plan = LocalCommitPlan {
            schema_version: 1,
            commit_transaction_id: "commit-synthetic-0001".into(),
            repository_sha256: repository_sha256.into(),
            review_packet_sha256: h.clone(),
            candidate_tree_receipt_sha256: h.clone(),
            commit_group_sha256: h.clone(),
            parent_object: "b".repeat(40),
            candidate_tree: "c".repeat(40),
            message_sha256: hash(message.as_bytes()),
            message,
            author: identity.clone(),
            committer: identity.clone(),
            identity_sha256: hash(&serde_json::to_vec(&(identity.clone(), identity)).unwrap()),
            timestamp_epoch_seconds: 1_787_000_000,
            timezone_offset_minutes: -300,
            signer,
            task_branch: "refs/heads/agentmage/tasks/fixture".into(),
            preservation_manifest_sha256: h.clone(),
            user_index_sha256: h,
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
        plan.plan_sha256 = hash(&serde_json::to_vec(&plan).unwrap());
        agentmage_kernel_engine::local_commit::verify_local_commit_plan(&plan).unwrap();
        plan
    }

    fn raw_commit(plan: &LocalCommitPlan) -> Vec<u8> {
        format!(
            "tree {}\nparent {}\nauthor {} <{}> {} {}\ncommitter {} <{}> {} {}\ngpgsig -----BEGIN PGP SIGNATURE-----\n \n synthetic armor\n -----END PGP SIGNATURE-----\n\n{}",
            plan.candidate_tree, plan.parent_object,
            plan.author.name, plan.author.email, plan.timestamp_epoch_seconds,
            git_timezone(plan.timezone_offset_minutes),
            plan.committer.name, plan.committer.email, plan.timestamp_epoch_seconds,
            git_timezone(plan.timezone_offset_minutes), plan.message,
        ).into_bytes()
    }

    #[test]
    fn local_commit_verifier_requires_exact_singleton_headers_and_message_bytes() {
        let mut plan = synthetic_plan(&hash(b"repository"));
        let raw = String::from_utf8(raw_commit(&plan)).unwrap();
        assert!(verify_raw_commit(raw.as_bytes(), &plan));
        for header in [
            format!("parent {}", "d".repeat(40)),
            format!("tree {}", "d".repeat(40)),
            format!(
                "author Other <other@example.test> {} -0500",
                plan.timestamp_epoch_seconds
            ),
            "encoding ISO-8859-1".into(),
        ] {
            let duplicate = raw.replacen("\ngpgsig ", &format!("\n{header}\ngpgsig "), 1);
            assert!(!verify_raw_commit(duplicate.as_bytes(), &plan), "{header}");
        }
        for changed in [
            raw.replace("synthetic armor", "synthetic armor\nparent forged"),
            raw.replace(" -----END PGP SIGNATURE-----", " missing end"),
            format!("{raw}\n"),
            raw[..raw.len() - 1].into(),
            raw.replace("Fixture User", "Other User"),
        ] {
            assert!(!verify_raw_commit(changed.as_bytes(), &plan));
        }
        let mut invalid_utf8 = raw.as_bytes().to_vec();
        invalid_utf8[0] = 255;
        assert!(!verify_raw_commit(&invalid_utf8, &plan));
        for message in ["Approved fixture change\n", "First line\n\nSecond line\n\n"] {
            plan.message = message.into();
            assert!(verify_raw_commit(&raw_commit(&plan), &plan));
        }
    }

    #[test]
    fn local_commit_verifier_binds_signature_header_to_both_object_formats() {
        for (size, header, other) in [
            (40, "gpgsig", "gpgsig-sha256"),
            (64, "gpgsig-sha256", "gpgsig"),
        ] {
            let mut plan = synthetic_plan(&hash(b"repository"));
            plan.parent_object = "b".repeat(size);
            plan.candidate_tree = "c".repeat(size);
            let raw = String::from_utf8(raw_commit(&plan)).unwrap();
            let raw = raw.replacen("\ngpgsig ", &format!("\n{header} "), 1);
            assert!(verify_raw_commit(raw.as_bytes(), &plan));
            for changed in [
                raw.replacen(&format!("\n{header} "), &format!("\n{other} "), 1),
                raw.replacen(
                    &format!("\n{header} "),
                    &format!("\n{other} -----BEGIN PGP SIGNATURE-----\n{header} "),
                    1,
                ),
                raw.replacen(
                    &format!("\n{header} "),
                    &format!("\n{header} -----BEGIN PGP SIGNATURE-----\n{header} "),
                    1,
                ),
            ] {
                assert!(!verify_raw_commit(changed.as_bytes(), &plan));
            }
            // Even exact headers cannot make mixed object formats admissible.
            plan.candidate_tree = "c".repeat(if size == 40 { 64 } else { 40 });
            let mixed = String::from_utf8(raw_commit(&plan)).unwrap().replacen(
                "\ngpgsig ",
                &format!("\n{header} "),
                1,
            );
            assert!(!verify_raw_commit(mixed.as_bytes(), &plan));
        }
    }

    #[test]
    fn local_commit_signature_verifier_requires_one_exact_status_fingerprint() {
        let fingerprint = "a".repeat(40);
        let status = format!(
            "[GNUPG:] NEWSIG\n[GNUPG:] VALIDSIG {fingerprint} 2026-11-10 1794297600 0 4 0 22 8 00 {fingerprint}\n"
        );
        assert!(verify_signature_status(status.as_bytes(), &fingerprint));
        assert!(verify_signature_status(
            status.to_ascii_uppercase().as_bytes(),
            &fingerprint
        ));
        for changed in [
            status.replace(
                &format!("VALIDSIG {fingerprint}"),
                &format!("VALIDSIG {fingerprint}a"),
            ),
            status.replace(
                &format!("VALIDSIG {fingerprint}"),
                &format!("VALIDSIG {}", "b".repeat(40)),
            ),
            status.replace("[GNUPG:] VALIDSIG", "warning: VALIDSIG"),
            status.replace("[GNUPG:] VALIDSIG", "[GNUPG:] GOODSIG"),
            format!("{status}{status}"),
            format!("[GNUPG:] VALIDSIG {fingerprint}\n"),
            format!("{status}\0"),
        ] {
            assert!(
                !verify_signature_status(changed.as_bytes(), &fingerprint),
                "{changed}"
            );
        }
        for failure in [
            "BADSIG",
            "ERRSIG",
            "REVKEYSIG",
            "EXPKEYSIG",
            "EXPSIG",
            "NO_PUBKEY",
            "NODATA",
            "FAILURE",
            "ERROR",
            "KEYREVOKED",
            "KEYEXPIRED",
            "SIGEXPIRED",
        ] {
            assert!(!verify_signature_status(
                format!("{status}[GNUPG:] {failure} detail\n").as_bytes(),
                &fingerprint
            ));
        }
        assert!(!verify_signature_status(&[255], &fingerprint));
    }

    #[test]
    fn local_commit_object_output_refuses_extra_text_whitespace_and_null_object() {
        for size in [40, 64] {
            let object = "a".repeat(size);
            assert_eq!(
                object_output(format!("{object}\n").into_bytes()).unwrap(),
                object
            );
            for invalid in [
                format!(" {object}\n"),
                format!("{object}\n\n"),
                object.clone(),
                format!("{object}\nwarning"),
                format!("{}\n", "0".repeat(size)),
                format!("{}\n", object.to_ascii_uppercase()),
            ] {
                assert!(object_output(invalid.into_bytes()).is_err());
            }
        }
    }

    #[test]
    fn local_commit_raw_verifier_matches_real_git_message_and_identity_encoding() {
        // Installed Git on an owned synthetic fixture, not product artifact
        // admission or cryptographic signing. Armor is deliberately synthetic.
        let fixture = Fixture::new();
        let mut plan = synthetic_plan(&hash(b"repository"));
        plan.parent_object =
            object_output(run_fixture(&fixture.checkout, &["rev-parse", "HEAD"])).unwrap();
        plan.candidate_tree = object_output(run_fixture(
            &fixture.checkout,
            &["rev-parse", "HEAD^{tree}"],
        ))
        .unwrap();
        for message in ["One line", "One line\n", "First\n\nSecond\n\n"] {
            plan.message = message.into();
            let payload =
                crate::sandbox::sealed_payload("fixture-message", message.as_bytes()).unwrap();
            let date = format_git_date(plan.timestamp_epoch_seconds, plan.timezone_offset_minutes);
            let output = Command::new("/usr/bin/git")
                .env_clear()
                .env("HOME", "/nonexistent")
                .env("GIT_CONFIG_NOSYSTEM", "1")
                .env("GIT_CONFIG_GLOBAL", "/dev/null")
                .env("GIT_AUTHOR_NAME", &plan.author.name)
                .env("GIT_AUTHOR_EMAIL", &plan.author.email)
                .env("GIT_COMMITTER_NAME", &plan.committer.name)
                .env("GIT_COMMITTER_EMAIL", &plan.committer.email)
                .env("GIT_AUTHOR_DATE", &date)
                .env("GIT_COMMITTER_DATE", &date)
                .args([
                    "-c",
                    "commit.gpgSign=false",
                    "-c",
                    "i18n.commitEncoding=UTF-8",
                    "commit-tree",
                    &plan.candidate_tree,
                    "-p",
                    &plan.parent_object,
                ])
                .current_dir(&fixture.checkout)
                .stdin(Stdio::from(fs::File::from(payload)))
                .output()
                .unwrap();
            assert!(output.status.success());
            let object = object_output(output.stdout).unwrap();
            let raw = run_fixture(&fixture.checkout, &["cat-file", "commit", &object]);
            let separator = raw.windows(2).position(|w| w == b"\n\n").unwrap();
            let mut with_armor = raw[..separator].to_vec();
            with_armor.extend_from_slice(b"\ngpgsig -----BEGIN PGP SIGNATURE-----\n \n synthetic armor\n -----END PGP SIGNATURE-----");
            with_armor.extend_from_slice(&raw[separator..]);
            assert!(verify_raw_commit(&with_armor, &plan), "{message:?}");
        }
    }

    #[test]
    fn local_commit_installed_git_signs_with_exact_primary_or_subkey_in_both_formats() {
        // Actual disposable Git/GnuPG signatures with a synthetic plan. This
        // exercises production signing capture, fixture inspection and exact
        // verification, not product artifact admission, confinement, approvals
        // or the native adapter workflow. Private fields below hold fixture
        // inputs only; the product constructor still requires a verified artifact.
        let _capture = DIRECT_CAPTURE_TEST
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        for format in ["sha1", "sha256"] {
            let fixture = Fixture::with_object_format(format);
            let (keyring, primary) = generate_fixture_key(&fixture);
            let added = fixture_gpg(&keyring)
                .args([
                    "--pinentry-mode",
                    "loopback",
                    "--passphrase",
                    "",
                    "--quick-add-key",
                    &primary,
                    "ed25519",
                    "sign",
                    "1d",
                ])
                .output()
                .expect("add fixture signing subkey");
            assert!(added.status.success(), "fixture subkey generation failed");
            // Direct fixture listing/export supports the signing matrix only.
            // Product inspection instead uses the offline public-only worker.
            let inspection_deadline = Instant::now() + Duration::from_secs(10);
            let listed = run_gpg(
                Path::new("/usr/bin/gpg"),
                &keyring,
                &[
                    "--with-colons",
                    "--with-subkey-fingerprint",
                    "--list-secret-keys",
                    &primary,
                ],
                inspection_deadline,
            )
            .unwrap();
            let exported = run_gpg(
                Path::new("/usr/bin/gpg"),
                &keyring,
                &["--export", &primary],
                inspection_deadline,
            )
            .unwrap();
            assert!(!exported.is_empty());
            assert_eq!(
                exported,
                fixture_gpg(&keyring)
                    .args(["--export", &primary])
                    .output()
                    .unwrap()
                    .stdout
            );
            let fingerprints: Vec<_> = String::from_utf8(listed)
                .unwrap()
                .lines()
                .filter_map(|line| {
                    let fields: Vec<_> = line.split(':').collect();
                    (fields.first() == Some(&"fpr"))
                        .then(|| fields.get(9).map(|s| s.to_ascii_lowercase()))
                        .flatten()
                })
                .collect();
            assert_eq!(fingerprints.len(), 2);
            assert_eq!(fingerprints[0], primary);
            assert_ne!(fingerprints[0], fingerprints[1]);
            let mut plan = synthetic_plan(&hash(b"repository"));
            plan.parent_object =
                object_output(run_fixture(&fixture.checkout, &["rev-parse", "HEAD"])).unwrap();
            plan.candidate_tree = object_output(run_fixture(
                &fixture.checkout,
                &["rev-parse", "HEAD^{tree}"],
            ))
            .unwrap();
            let scope = LinuxRepositoryScope::verify(
                &fixture.checkout,
                fixture.checkout.join(".git"),
                &fixture.owned,
            )
            .unwrap();
            let signer = LinuxOpenPgpSigner {
                executable: PathBuf::from("/usr/bin/gpg"),
                executable_sha256: hash(b"synthetic signer artifact"),
                keyring,
                keygrip: None,
                public_key: exported.clone(),
                pinned: plan.signer.clone(),
            };
            let runner = SignedGitRunner {
                git_path: Path::new("/usr/bin/git"),
                scope: &scope,
                signer: &signer,
                deadline: Instant::now() + Duration::from_secs(20),
            };
            let token = cancellation("production-signed-capture");
            let date = format_git_date(plan.timestamp_epoch_seconds, plan.timezone_offset_minutes);
            let environment = BTreeMap::from([
                ("GIT_AUTHOR_NAME".into(), plan.author.name.clone()),
                ("GIT_AUTHOR_EMAIL".into(), plan.author.email.clone()),
                ("GIT_COMMITTER_NAME".into(), plan.committer.name.clone()),
                ("GIT_COMMITTER_EMAIL".into(), plan.committer.email.clone()),
                ("GIT_AUTHOR_DATE".into(), date.clone()),
                ("GIT_COMMITTER_DATE".into(), date),
            ]);
            let verification_home = fixture.root.join("public-verification-home");
            fs::create_dir(&verification_home).unwrap();
            fs::set_permissions(&verification_home, fs::Permissions::from_mode(0o700)).unwrap();
            let commit_and_verify =
                |selected: &agentmage_kernel_engine::local_commit::LocalCommitPlan,
                 arguments: &[String],
                 message: &str| {
                    let output = runner
                        .run_with_environment(
                            &arguments.iter().map(String::as_str).collect::<Vec<_>>(),
                            message.as_bytes(),
                            &token,
                            &environment,
                        )
                        .expect("production runner signs fixture");
                    let object = object_output(output.stdout).unwrap();
                    let raw = run_fixture(&fixture.checkout, &["cat-file", "commit", &object]);
                    // Exercise the worker's public-only payload reconstruction and
                    // fixed GnuPG options directly. This is cryptographic component
                    // evidence; native supervisor admission remains a distinct test.
                    let verification = verification::fixture_verify(
                        &raw,
                        selected,
                        &exported,
                        &verification_home,
                        &token,
                    )
                    .expect("public-only component verification");
                    assert!(!verification.stdout.is_empty());
                    // Human log lines stay separate from status in the real
                    // installed-GnuPG component path as well as the guest argv.
                    assert!(verification.stderr.starts_with(b"gpg: "));
                    (raw, verification.stdout)
                };
            // Without exact selection, GnuPG chooses the newer signing subkey.
            let mut unpinned = signed_commit_arguments(&plan);
            unpinned[1] = format!("-S{primary}");
            let (_, status) = commit_and_verify(&plan, &unpinned, &plan.message);
            assert!(verify_signature_status(&status, &fingerprints[1]));
            assert!(!verify_signature_status(&status, &primary));
            for fingerprint in &fingerprints {
                for message in ["One line", "One line\n", "First\n\nSecond\n\n"] {
                    let mut selected = plan.clone();
                    selected.signer.key_fingerprint = fingerprint.clone();
                    selected.message = message.into();
                    let (raw, status) =
                        commit_and_verify(&selected, &signed_commit_arguments(&selected), message);
                    assert!(verify_raw_commit(&raw, &selected), "{format} {message:?}");
                    assert!(verify_signature_status(&status, fingerprint));
                    let other = fingerprints
                        .iter()
                        .find(|other| *other != fingerprint)
                        .unwrap();
                    assert!(!verify_signature_status(&status, other));
                    // A newly approved message with the old signature must
                    // still fail cryptographically after structural checks.
                    let mut changed = selected.clone();
                    changed.message.push_str("changed");
                    let mut tampered = raw.clone();
                    tampered.extend_from_slice(b"changed");
                    assert!(
                        verification::fixture_verify(
                            &tampered,
                            &changed,
                            &exported,
                            &verification_home,
                            &token,
                        )
                        .is_err()
                    );
                    assert!(
                        verification::fixture_verify(
                            &raw,
                            &selected,
                            b"not a public key",
                            &verification_home,
                            &token,
                        )
                        .is_err()
                    );
                }
            }
            assert_eq!(
                object_output(run_fixture(&fixture.checkout, &["rev-parse", "HEAD"])).unwrap(),
                plan.parent_object,
                "component signing must not move the fixture branch"
            );
            for private in ["S.gpg-agent", "S.gpg-agent.extra", "private-keys-v1.d"] {
                assert!(
                    !verification_home.join(private).exists(),
                    "public verification must not create an agent or private keys"
                );
            }
        }
    }

    #[test]
    fn local_commit_production_capture_callers_bound_output_exit_deadline_and_cancellation() {
        // Real child processes through production signing and the test-only GPG
        // fixture helper. No artifact admission or sandbox claim is made.
        let _capture = DIRECT_CAPTURE_TEST
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let fixture = Fixture::new();
        let scope = LinuxRepositoryScope::verify(
            &fixture.checkout,
            fixture.checkout.join(".git"),
            &fixture.owned,
        )
        .unwrap();
        let program = fixture.root.join("component-process");
        let signer = LinuxOpenPgpSigner {
            executable: PathBuf::from("/usr/bin/gpg"),
            executable_sha256: hash(b"synthetic signer artifact"),
            keyring: fixture.root.clone(),
            keygrip: None,
            public_key: Vec::new(),
            pinned: synthetic_plan(&hash(b"repository")).signer,
        };
        for (body, succeeds, timeout) in [
            ("printf 'object\n'; printf 'status\n' >&2", true, false),
            (
                "printf 'partial'; printf 'failure' >&2; exit 7",
                false,
                false,
            ),
            ("exec /usr/bin/head -c 4194305 /dev/zero", false, false),
            ("exec /usr/bin/head -c 4194305 /dev/zero >&2", false, false),
            ("exec /usr/bin/sleep 5", false, true),
        ] {
            fs::write(&program, format!("#!/bin/sh\n{body}\n")).unwrap();
            fs::set_permissions(&program, fs::Permissions::from_mode(0o700)).unwrap();
            for inspection in [true, false] {
                let token = cancellation("production-capture-bound");
                let started = Instant::now();
                let deadline = started
                    + if timeout {
                        Duration::from_millis(100)
                    } else {
                        Duration::from_secs(5)
                    };
                let runner = SignedGitRunner {
                    git_path: &program,
                    scope: &scope,
                    signer: &signer,
                    deadline,
                };
                let result = if inspection {
                    run_gpg(&program, &fixture.root, &["--export"], deadline)
                } else {
                    runner
                        .run(
                            &["commit-tree", &"b".repeat(40), "-p", &"c".repeat(40)],
                            &[],
                            &token,
                        )
                        .map(|captured| {
                            assert_eq!(captured.stderr, b"status\n");
                            captured.stdout
                        })
                };
                if succeeds {
                    assert_eq!(result.unwrap(), b"object\n");
                } else {
                    assert_eq!(
                        result.unwrap_err().kind(),
                        if inspection {
                            LinuxLocalCommitErrorKind::ArtifactDenied
                        } else {
                            LinuxLocalCommitErrorKind::ExecutionFailed
                        }
                    );
                }
                if timeout {
                    assert!(started.elapsed() < Duration::from_secs(1));
                }
            }
        }
        // Live cancellation of the actual production runner, not an already
        // cancelled preflight. The script execs, so the owned child is the sleeper.
        let ready = fixture.root.join("capture-ready");
        fs::write(
            &program,
            b"#!/bin/sh\nprintf ready > \"$AGENTMAGE_COMPONENT_READY\"\nexec /usr/bin/sleep 5\n",
        )
        .unwrap();
        let token = cancellation("production-capture-live-cancel");
        let request = token.clone();
        let observed_ready = ready.clone();
        let requester = std::thread::spawn(move || {
            use agentmage_kernel_contracts::*;
            let deadline = Instant::now() + Duration::from_secs(2);
            while !observed_ready.is_file() && Instant::now() < deadline {
                std::thread::sleep(Duration::from_millis(1));
            }
            let launched = observed_ready.is_file();
            request
                .cancel(CancellationSignal {
                    schema_version: CONTRACT_SCHEMA_VERSION,
                    cancellation_id: CancellationId::from_raw("cancel-production-capture"),
                    task_id: TaskId::from_raw("task-production-capture-live-cancel"),
                    correlation_id: CorrelationId::from_raw(
                        "correlation-production-capture-live-cancel",
                    ),
                    reason: CancellationReason::UserRequested,
                    requested_by: BoundaryKind::Kernel,
                })
                .unwrap();
            launched
        });
        let started = Instant::now();
        let runner = SignedGitRunner {
            git_path: &program,
            scope: &scope,
            signer: &signer,
            deadline: started + Duration::from_secs(4),
        };
        let result = runner.run_with_environment(
            &["commit-tree", &"b".repeat(40), "-p", &"c".repeat(40)],
            &[],
            &token,
            &BTreeMap::from([(
                "AGENTMAGE_COMPONENT_READY".into(),
                ready.to_str().unwrap().into(),
            )]),
        );
        assert!(
            requester.join().unwrap(),
            "child must launch before cancellation"
        );
        assert_eq!(
            result.err().unwrap().kind(),
            LinuxLocalCommitErrorKind::ExecutionFailed
        );
        assert!(started.elapsed() < Duration::from_secs(1));
        // A clean call through the original process owner can follow the failures.
        assert!(
            !run_gpg(
                Path::new("/usr/bin/gpg"),
                &fixture.root,
                &["--version"],
                Instant::now() + Duration::from_secs(5)
            )
            .unwrap()
            .is_empty()
        );
    }

    #[test]
    fn local_commit_cleanup_uncertainty_survives_caller_mapping_and_receipt_failure() {
        use crate::sandbox::LinuxSandboxErrorKind;
        use agentmage_kernel_engine::repository_safety::RepositoryPreservationManifest;
        let before: RepositoryPreservationManifest = serde_json::from_str(include_str!(
            "../../../schemas/runtime/examples/repository-preservation-manifest.valid.json"
        ))
        .unwrap();
        for fallback in [
            LinuxLocalCommitErrorKind::ArtifactDenied,
            LinuxLocalCommitErrorKind::ExecutionFailed,
        ] {
            let failure = capture_error(LinuxSandboxErrorKind::CleanupUncertain, fallback);
            assert_eq!(failure.kind(), LinuxLocalCommitErrorKind::CleanupUncertain);
            assert_eq!(failure.to_string(), "linux.local_commit.cleanup.uncertain");
            for code in [
                "linux.git.commit.creation_failed",
                "linux.git.commit.verification_failed",
            ] {
                let result = failed_signer_result(&before, failure, code);
                assert_eq!(
                    result.platform_code,
                    "linux.git.commit.signer_cleanup_uncertain"
                );
                assert_eq!(result.outcome, OperationOutcome::Failed);
                assert!(!result.cleanup_verified);
                assert!(!result.signature_verified);
                assert!(result.commit_object.is_none());
                assert_eq!(result.after, before);
                let ordinary = capture_error(LinuxSandboxErrorKind::OutputLimitExceeded, fallback);
                assert_eq!(ordinary.kind(), fallback);
                let ordinary = failed_signer_result(&before, ordinary, code);
                assert_eq!(ordinary.platform_code, code);
                assert_eq!(ordinary.outcome, OperationOutcome::Failed);
                assert!(!ordinary.cleanup_verified);
            }
        }
    }

    #[test]
    fn local_commit_uncertain_effect_cannot_reconcile_as_a_verified_preservation_receipt() {
        use agentmage_kernel_engine::repository_safety::RepositoryPreservationManifest;
        let before: RepositoryPreservationManifest = serde_json::from_str(include_str!(
            "../../../schemas/runtime/examples/repository-preservation-manifest.valid.json"
        ))
        .unwrap();
        let before = RepositoryPreservationManifest::seal(before).unwrap();
        let mut plan = synthetic_plan(&before.repository_sha256);
        plan.preservation_manifest_sha256 = before.manifest_sha256.clone();
        plan.user_index_sha256 = before.index_sha256.clone();
        plan.plan_sha256 = "0".repeat(64);
        plan.plan_sha256 = hash(&serde_json::to_vec(&plan).unwrap());
        let approval = record_manual_commit_approval(
            &plan,
            ManualCommitApprovalDecision {
                approval_id: "approval-uncertain-0001".into(),
                approval_channel_sha256: hash(b"synthetic approval channel"),
                approved_at_epoch_ms: 10_000,
                expires_at_epoch_ms: 20_000,
                human_confirmed: true,
            },
        )
        .unwrap();
        let mut result = uncertain_commit_result(&before, "linux.git.commit.postflight");
        assert_eq!(result.outcome, OperationOutcome::Failed);
        assert!(!result.cleanup_verified);
        // Supply otherwise passing synthetic observations so rejection below
        // is specifically the failed effect / unknown cleanup boundary.
        let object = "d".repeat(40);
        result.commit_object = Some(object.clone());
        result.observed_parent = Some(plan.parent_object.clone());
        result.observed_tree = Some(plan.candidate_tree.clone());
        result.observed_message_sha256 = Some(plan.message_sha256.clone());
        result.observed_identity_sha256 = Some(plan.identity_sha256.clone());
        result.observed_signer_fingerprint = Some(plan.signer.key_fingerprint.clone());
        result.signature_verified = true;
        result.task_branch_before = Some(plan.parent_object.clone());
        result.task_branch_after = Some(object);
        let reconcile = |r| {
            reconcile_local_commit(
                &plan,
                &approval,
                r,
                "authority-uncertain-0001",
                "attempt-uncertain-0001",
                15_000,
            )
        };
        assert!(reconcile(result.clone()).is_err());
        let mut success_only = result.clone();
        success_only.outcome = OperationOutcome::Succeeded;
        assert!(reconcile(success_only).is_err());
        result.cleanup_verified = true;
        assert!(reconcile(result.clone()).is_err());
        result.outcome = OperationOutcome::Succeeded;
        assert!(
            reconcile(result).is_ok(),
            "otherwise passing control must reconcile"
        );
    }

    struct Fixture {
        root: PathBuf,
        checkout: PathBuf,
        owned: PathBuf,
    }

    impl Fixture {
        fn new() -> Self {
            Self::with_object_format("sha1")
        }

        fn with_object_format(format: &str) -> Self {
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
            run_fixture(
                &checkout,
                &[
                    "init",
                    "--initial-branch=main",
                    &format!("--object-format={format}"),
                ],
            );
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

    fn fixture_gpg(keyring: &Path) -> Command {
        let mut command = Command::new("/usr/bin/gpg");
        command
            .env_clear()
            .env("GNUPGHOME", keyring)
            .env("HOME", "/nonexistent")
            .env("LANG", "C")
            .env("LC_ALL", "C")
            .args([
                "--batch",
                "--no-tty",
                "--no-options",
                "--no-auto-key-retrieve",
            ]);
        command
    }

    fn generate_fixture_key(fixture: &Fixture) -> (PathBuf, String) {
        let keyring = fixture.root.join("keyring");
        fs::create_dir(&keyring).expect("keyring");
        fs::set_permissions(&keyring, fs::Permissions::from_mode(0o700))
            .expect("keyring permissions");
        let generated = fixture_gpg(&keyring)
            .args([
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
        let listed = fixture_gpg(&keyring)
            .args(["--with-colons", "--fingerprint", "--list-secret-keys"])
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
        (keyring, fingerprint)
    }

    fn generate_signer(fixture: &Fixture, scope: &LinuxRepositoryScope) -> LinuxOpenPgpSigner {
        let (keyring, fingerprint) = generate_fixture_key(fixture);
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
        let _capture = DIRECT_CAPTURE_TEST
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
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
