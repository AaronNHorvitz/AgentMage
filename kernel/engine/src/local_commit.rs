//! Exact candidate-tree, signer, manual-approval, and local signed-commit contracts.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::fmt::Write as _;

use agentmage_kernel_contracts::{
    GrantOperation, HeldWorkspaceObject, OperationOutcome, StateChange,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::authority_transaction::{EffectAuthorization, EffectDriver, EffectLaunch, EffectResult};
use crate::propagation::CancellationToken;
use crate::repository_safety::{
    RepositoryOwnedDelta, RepositoryPreservationManifest, reconcile_preservation,
};
use crate::review_packet::{LocalReviewPacket, verify_review_packet};

const SCHEMA_VERSION: u16 = 1;
const MAX_COMMIT_FILES: usize = 128;
const MAX_COMMIT_MESSAGE_BYTES: usize = 16 * 1024;
const MAX_APPROVAL_TTL_MS: u64 = 10 * 60 * 1_000;
const ZERO_SHA256: &str = "0000000000000000000000000000000000000000000000000000000000000000";

/// Stable reason candidate-tree or local-commit processing failed closed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LocalCommitError {
    /// A field is malformed, unsupported, over bound, or internally inconsistent.
    InvalidInput,
    /// The review packet or logical group is stale, incomplete, or mismatched.
    ReviewMismatch,
    /// Candidate-tree construction changed unapproved repository or index state.
    CandidateTreeMismatch,
    /// The signer is unpinned, repository-selected, unsigned, or unsupported.
    SignerDenied,
    /// Exact manual approval is absent, stale, replayed, or mismatched.
    ApprovalDenied,
    /// Platform postconditions do not prove one exact signed local commit.
    ResultDenied,
    /// Canonical receipt serialization failed.
    ReceiptFailure,
}

impl LocalCommitError {
    /// Returns one content-free error code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidInput => "local-commit.input.invalid",
            Self::ReviewMismatch => "local-commit.review.mismatch",
            Self::CandidateTreeMismatch => "local-commit.candidate-tree.mismatch",
            Self::SignerDenied => "local-commit.signer.denied",
            Self::ApprovalDenied => "local-commit.approval.denied",
            Self::ResultDenied => "local-commit.result.denied",
            Self::ReceiptFailure => "local-commit.receipt.failure",
        }
    }
}

impl fmt::Display for LocalCommitError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.code())
    }
}

impl std::error::Error for LocalCommitError {}

/// Closed regular-file mode supported by the first local commit boundary.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CommitFileMode {
    /// Non-executable regular file (`100644`).
    Regular,
    /// Executable regular file (`100755`).
    Executable,
}

/// One exact reviewed file before Git blob construction.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CandidateTreeFilePlan {
    /// Exact shadow operation identity.
    pub operation_id: String,
    /// Exact workspace-relative path.
    pub path: String,
    /// Exact approved postimage bytes digest.
    pub postimage_sha256: String,
    /// Closed regular-file mode.
    pub mode: CommitFileMode,
}

/// Authority-free exact request for candidate-tree construction.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CandidateTreePlan {
    /// Closed schema version.
    pub schema_version: u16,
    /// Stable non-replayable candidate build identity.
    pub candidate_build_id: String,
    /// Exact repository identity.
    pub repository_sha256: String,
    /// Exact source review packet.
    pub review_packet_sha256: String,
    /// Exact source change set.
    pub change_set_sha256: String,
    /// Exact logical commit group.
    pub commit_group_sha256: String,
    /// Exact immutable parent object used to seed the temporary index.
    pub parent_object: String,
    /// AgentMage-owned temporary index path identity.
    pub temporary_index_path_sha256: String,
    /// Exact user index identity that must remain unchanged.
    pub user_index_sha256: String,
    /// Exact pre-effect preservation manifest.
    pub preservation_manifest_sha256: String,
    /// Exact approved path and postimage set, strictly path ordered.
    pub files: Vec<CandidateTreeFilePlan>,
    /// Candidate construction cannot run hooks.
    pub hooks_enabled: bool,
    /// Candidate construction cannot run filters or content drivers.
    pub filters_enabled: bool,
    /// Candidate construction cannot consume repository configuration.
    pub repository_configuration_enabled: bool,
    /// Candidate construction cannot access a remote.
    pub network_authority: bool,
    /// Canonical plan digest.
    pub plan_sha256: String,
}

/// Builds a candidate-tree plan from one exact verified review packet group.
pub fn plan_candidate_tree(
    candidate_build_id: &str,
    packet: &LocalReviewPacket,
    group_id: &str,
    temporary_index_path_sha256: &str,
    user_index_sha256: &str,
    modes: &[(String, CommitFileMode)],
) -> Result<CandidateTreePlan, LocalCommitError> {
    if !verify_review_packet(packet) {
        return Err(LocalCommitError::ReviewMismatch);
    }
    let group = packet
        .commit_plan
        .groups
        .iter()
        .find(|group| group.group_id == group_id)
        .ok_or(LocalCommitError::ReviewMismatch)?;
    let mode_by_operation = modes.iter().cloned().collect::<BTreeMap<_, _>>();
    if mode_by_operation.len() != modes.len()
        || mode_by_operation.len() != group.operation_ids.len()
    {
        return Err(LocalCommitError::ReviewMismatch);
    }
    let mut files = packet
        .change_set
        .files()
        .iter()
        .filter(|file| group.operation_ids.contains(&file.operation_id))
        .map(|file| {
            Ok(CandidateTreeFilePlan {
                operation_id: file.operation_id.clone(),
                path: file.path.clone(),
                postimage_sha256: file.postimage_sha256.clone(),
                mode: *mode_by_operation
                    .get(&file.operation_id)
                    .ok_or(LocalCommitError::ReviewMismatch)?,
            })
        })
        .collect::<Result<Vec<_>, LocalCommitError>>()?;
    files.sort_by(|left, right| left.path.cmp(&right.path));
    if files
        .iter()
        .map(|file| file.path.as_str())
        .collect::<Vec<_>>()
        != group.paths.iter().map(String::as_str).collect::<Vec<_>>()
    {
        return Err(LocalCommitError::ReviewMismatch);
    }
    seal_candidate_tree_plan(CandidateTreePlan {
        schema_version: SCHEMA_VERSION,
        candidate_build_id: candidate_build_id.to_owned(),
        repository_sha256: packet.repository_sha256.clone(),
        review_packet_sha256: packet.packet_sha256.clone(),
        change_set_sha256: packet.change_set.change_set_sha256().to_owned(),
        commit_group_sha256: group.group_sha256.clone(),
        parent_object: packet.base_object.clone(),
        temporary_index_path_sha256: temporary_index_path_sha256.to_owned(),
        user_index_sha256: user_index_sha256.to_owned(),
        preservation_manifest_sha256: packet.preservation_manifest_sha256.clone(),
        files,
        hooks_enabled: false,
        filters_enabled: false,
        repository_configuration_enabled: false,
        network_authority: false,
        plan_sha256: ZERO_SHA256.to_owned(),
    })
}

fn seal_candidate_tree_plan(
    mut plan: CandidateTreePlan,
) -> Result<CandidateTreePlan, LocalCommitError> {
    plan.plan_sha256 = ZERO_SHA256.to_owned();
    validate_candidate_tree_plan(&plan)?;
    plan.plan_sha256 = canonical_sha256(&plan)?;
    Ok(plan)
}

/// Revalidates a candidate-tree plan and its canonical digest.
pub fn verify_candidate_tree_plan(plan: &CandidateTreePlan) -> Result<(), LocalCommitError> {
    validate_candidate_tree_plan(plan)?;
    let mut canonical = plan.clone();
    canonical.plan_sha256 = ZERO_SHA256.to_owned();
    if canonical_sha256(&canonical)? != plan.plan_sha256 {
        return Err(LocalCommitError::CandidateTreeMismatch);
    }
    Ok(())
}

fn validate_candidate_tree_plan(plan: &CandidateTreePlan) -> Result<(), LocalCommitError> {
    let operation_count = plan
        .files
        .iter()
        .map(|file| file.operation_id.as_str())
        .collect::<BTreeSet<_>>()
        .len();
    if plan.schema_version != SCHEMA_VERSION
        || !valid_identifier(&plan.candidate_build_id)
        || !is_sha256(&plan.repository_sha256)
        || !is_sha256(&plan.review_packet_sha256)
        || !is_sha256(&plan.change_set_sha256)
        || !is_sha256(&plan.commit_group_sha256)
        || !valid_object_id(&plan.parent_object)
        || !is_sha256(&plan.temporary_index_path_sha256)
        || !is_sha256(&plan.user_index_sha256)
        || !is_sha256(&plan.preservation_manifest_sha256)
        || plan.files.is_empty()
        || plan.files.len() > MAX_COMMIT_FILES
        || operation_count != plan.files.len()
        || !plan
            .files
            .windows(2)
            .all(|pair| pair[0].path < pair[1].path)
        || plan.files.iter().any(|file| {
            !valid_identifier(&file.operation_id)
                || !valid_path(&file.path)
                || !is_sha256(&file.postimage_sha256)
        })
        || plan.hooks_enabled
        || plan.filters_enabled
        || plan.repository_configuration_enabled
        || plan.network_authority
        || !is_sha256(&plan.plan_sha256)
    {
        return Err(LocalCommitError::InvalidInput);
    }
    Ok(())
}

/// Blob identity established from one exact approved postimage.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CandidateTreeBlob {
    /// Exact operation identity.
    pub operation_id: String,
    /// Exact approved postimage identity.
    pub postimage_sha256: String,
    /// Git blob object produced from those exact bytes.
    pub blob_object: String,
}

/// Trusted platform observations after temporary-index candidate construction.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CandidateTreePlatformResult {
    /// Exact pre-effect manifest.
    pub before: RepositoryPreservationManifest,
    /// Exact post-effect manifest.
    pub after: RepositoryPreservationManifest,
    /// Terminal platform outcome.
    pub outcome: OperationOutcome,
    /// Exact candidate tree object when successful.
    pub candidate_tree: Option<String>,
    /// Exact approved postimage-to-blob mappings.
    pub blobs: Vec<CandidateTreeBlob>,
    /// User-index identity observed after execution.
    pub user_index_after_sha256: String,
    /// Temporary index was created only beneath the AgentMage-owned root.
    pub owned_temporary_index: bool,
    /// Temporary index and attributed processes were removed.
    pub cleanup_verified: bool,
    /// Whether any hook ran.
    pub hooks_executed: bool,
    /// Whether any content transform ran.
    pub filters_executed: bool,
    /// Whether repository-selected configuration influenced the operation.
    pub repository_configuration_used: bool,
    /// Whether any network connection was attempted.
    pub network_used: bool,
    /// Stable content-free platform code.
    pub platform_code: String,
}

/// Integrity-protected candidate tree receipt used by an exact manual commit approval.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CandidateTreeReceipt {
    /// Closed schema version.
    pub schema_version: u16,
    /// Exact candidate build identity.
    pub candidate_build_id: String,
    /// Exact candidate plan identity.
    pub candidate_plan_sha256: String,
    /// Exact repository identity.
    pub repository_sha256: String,
    /// Exact immutable parent.
    pub parent_object: String,
    /// Exact candidate tree.
    pub candidate_tree: String,
    /// Exact approved postimage-to-blob mappings.
    pub blobs: Vec<CandidateTreeBlob>,
    /// Exact pre-effect manifest identity.
    pub before_manifest_sha256: String,
    /// Exact post-effect manifest identity.
    pub after_manifest_sha256: String,
    /// Exact unchanged user index.
    pub user_index_sha256: String,
    /// Candidate tree is authority-free and cannot update a ref.
    pub ref_update_authority: bool,
    /// Stable content-free platform code.
    pub platform_code: String,
    /// Canonical receipt digest.
    pub receipt_sha256: String,
}

/// Reconciles one candidate-tree build while permitting only new exact objects.
pub fn reconcile_candidate_tree(
    plan: &CandidateTreePlan,
    mut result: CandidateTreePlatformResult,
) -> Result<CandidateTreeReceipt, LocalCommitError> {
    verify_candidate_tree_plan(plan)?;
    result
        .before
        .verify()
        .map_err(|_| LocalCommitError::CandidateTreeMismatch)?;
    result
        .after
        .verify()
        .map_err(|_| LocalCommitError::CandidateTreeMismatch)?;
    result.blobs.sort();
    let mut expected_operations = plan
        .files
        .iter()
        .map(|file| (file.operation_id.as_str(), file.postimage_sha256.as_str()))
        .collect::<Vec<_>>();
    expected_operations.sort();
    let observed_operations = result
        .blobs
        .iter()
        .map(|blob| (blob.operation_id.as_str(), blob.postimage_sha256.as_str()))
        .collect::<Vec<_>>();
    let candidate_tree = result
        .candidate_tree
        .filter(|object| valid_object_id(object))
        .ok_or(LocalCommitError::CandidateTreeMismatch)?;
    if result.outcome != OperationOutcome::Succeeded
        || result.before.manifest_sha256 != plan.preservation_manifest_sha256
        || result.before.repository_sha256 != plan.repository_sha256
        || result.before.index_sha256 != plan.user_index_sha256
        || result.user_index_after_sha256 != plan.user_index_sha256
        || expected_operations != observed_operations
        || result
            .blobs
            .iter()
            .any(|blob| !valid_object_id(&blob.blob_object))
        || !result.owned_temporary_index
        || !result.cleanup_verified
        || result.hooks_executed
        || result.filters_executed
        || result.repository_configuration_used
        || result.network_used
        || !valid_platform_code(&result.platform_code)
    {
        return Err(LocalCommitError::CandidateTreeMismatch);
    }
    reconcile_preservation(
        &result.before,
        &result.after,
        RepositoryOwnedDelta::CandidateTree,
    )
    .map_err(|_| LocalCommitError::CandidateTreeMismatch)?;
    let mut receipt = CandidateTreeReceipt {
        schema_version: SCHEMA_VERSION,
        candidate_build_id: plan.candidate_build_id.clone(),
        candidate_plan_sha256: plan.plan_sha256.clone(),
        repository_sha256: plan.repository_sha256.clone(),
        parent_object: plan.parent_object.clone(),
        candidate_tree,
        blobs: result.blobs,
        before_manifest_sha256: result.before.manifest_sha256,
        after_manifest_sha256: result.after.manifest_sha256,
        user_index_sha256: plan.user_index_sha256.clone(),
        ref_update_authority: false,
        platform_code: result.platform_code,
        receipt_sha256: ZERO_SHA256.to_owned(),
    };
    receipt.receipt_sha256 = canonical_sha256(&receipt)?;
    Ok(receipt)
}

/// Closed trusted signer family admitted for local commits.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CommitSignerKind {
    /// Hardware-backed identity exposed by a trusted platform broker.
    HardwareBacked,
    /// OpenPGP identity held in an approved external keyring.
    OpenPgp,
}

/// Closed signer-inspection source that cannot be repository configuration.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SignerInspectionSource {
    /// Trusted operating-system signing broker or key store.
    PlatformBroker,
    /// User-approved keyring outside the repository and worktree.
    ExternalKeyring,
}

/// Pinned signer identity and executable boundary, without private-key material.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PinnedCommitSigner {
    /// Closed schema version.
    pub schema_version: u16,
    /// Stable signer inspection identity.
    pub signer_id: String,
    /// Closed signer family.
    pub kind: CommitSignerKind,
    /// Trusted inspection source outside repository configuration.
    pub source: SignerInspectionSource,
    /// Digest of the approved signer launch path.
    pub signer_path_sha256: String,
    /// Exact signer executable or broker artifact digest.
    pub signer_executable_sha256: String,
    /// Exact lowercase OpenPGP or platform key fingerprint.
    pub key_fingerprint: String,
    /// Exact public verification material digest.
    pub public_key_sha256: String,
    /// Exact external inspection evidence digest.
    pub inspection_evidence_sha256: String,
    /// Repository configuration was not consulted.
    pub repository_configuration_used: bool,
    /// Unsigned fallback is never available.
    pub unsigned_fallback: bool,
    /// Canonical signer report digest.
    pub signer_sha256: String,
}

impl PinnedCommitSigner {
    /// Seals one externally inspected signer without repository-selected programs.
    pub fn seal(mut signer: Self) -> Result<Self, LocalCommitError> {
        signer.schema_version = SCHEMA_VERSION;
        signer.signer_sha256 = ZERO_SHA256.to_owned();
        validate_signer(&signer)?;
        signer.signer_sha256 = canonical_sha256(&signer)?;
        Ok(signer)
    }

    /// Revalidates the pinned signer report and its canonical identity.
    pub fn verify(&self) -> Result<(), LocalCommitError> {
        validate_signer(self)?;
        let mut canonical = self.clone();
        canonical.signer_sha256 = ZERO_SHA256.to_owned();
        if canonical_sha256(&canonical)? != self.signer_sha256 {
            return Err(LocalCommitError::SignerDenied);
        }
        Ok(())
    }
}

fn validate_signer(signer: &PinnedCommitSigner) -> Result<(), LocalCommitError> {
    if signer.schema_version != SCHEMA_VERSION
        || !valid_identifier(&signer.signer_id)
        || !is_sha256(&signer.signer_path_sha256)
        || !is_sha256(&signer.signer_executable_sha256)
        || !valid_fingerprint(&signer.key_fingerprint)
        || !is_sha256(&signer.public_key_sha256)
        || !is_sha256(&signer.inspection_evidence_sha256)
        || signer.repository_configuration_used
        || signer.unsigned_fallback
        || !is_sha256(&signer.signer_sha256)
    {
        return Err(LocalCommitError::SignerDenied);
    }
    Ok(())
}

/// Exact Git author or committer identity.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CommitIdentity {
    /// Exact display name approved for this commit.
    pub name: String,
    /// Exact email address approved for this commit.
    pub email: String,
}

/// Exact, authority-free signed local commit plan.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LocalCommitPlan {
    /// Closed schema version.
    pub schema_version: u16,
    /// Stable non-replayable local commit identity.
    pub commit_transaction_id: String,
    /// Exact repository identity.
    pub repository_sha256: String,
    /// Exact source review packet.
    pub review_packet_sha256: String,
    /// Exact candidate tree receipt.
    pub candidate_tree_receipt_sha256: String,
    /// Exact logical commit group.
    pub commit_group_sha256: String,
    /// Exact immutable parent and expected task-branch value.
    pub parent_object: String,
    /// Exact candidate tree.
    pub candidate_tree: String,
    /// Exact approved commit message.
    pub message: String,
    /// Exact commit message identity.
    pub message_sha256: String,
    /// Exact author identity.
    pub author: CommitIdentity,
    /// Exact committer identity.
    pub committer: CommitIdentity,
    /// Exact author/committer identity digest.
    pub identity_sha256: String,
    /// Exact Unix timestamp in seconds.
    pub timestamp_epoch_seconds: i64,
    /// Exact timezone offset in minutes.
    pub timezone_offset_minutes: i16,
    /// Exact externally inspected signer.
    pub signer: PinnedCommitSigner,
    /// Exact AgentMage-owned task branch.
    pub task_branch: String,
    /// Exact pre-effect repository preservation manifest.
    pub preservation_manifest_sha256: String,
    /// Exact unchanged user-index identity.
    pub user_index_sha256: String,
    /// Automatic commit is prohibited.
    pub automatic_commit: bool,
    /// Push is prohibited.
    pub push_authority: bool,
    /// Merge is prohibited.
    pub merge_authority: bool,
    /// Release is prohibited.
    pub release_authority: bool,
    /// Amend and history rewrite are prohibited.
    pub history_rewrite_authority: bool,
    /// Reset is prohibited.
    pub reset_authority: bool,
    /// Discard is prohibited.
    pub discard_authority: bool,
    /// Force operation is prohibited.
    pub force_authority: bool,
    /// Network use is prohibited.
    pub network_authority: bool,
    /// Canonical plan digest.
    pub plan_sha256: String,
}

/// Builds one commit plan after candidate-tree creation and before manual approval.
#[allow(clippy::too_many_arguments)]
pub fn plan_local_commit(
    commit_transaction_id: &str,
    packet: &LocalReviewPacket,
    group_id: &str,
    candidate_plan: &CandidateTreePlan,
    candidate: &CandidateTreeReceipt,
    author: CommitIdentity,
    committer: CommitIdentity,
    timestamp_epoch_seconds: i64,
    timezone_offset_minutes: i16,
    signer: PinnedCommitSigner,
    task_branch: &str,
) -> Result<LocalCommitPlan, LocalCommitError> {
    if !verify_review_packet(packet)
        || verify_candidate_tree_receipt(candidate_plan, candidate).is_err()
    {
        return Err(LocalCommitError::ReviewMismatch);
    }
    signer.verify()?;
    let group = packet
        .commit_plan
        .groups
        .iter()
        .find(|group| group.group_id == group_id)
        .ok_or(LocalCommitError::ReviewMismatch)?;
    let blob_operations = candidate
        .blobs
        .iter()
        .map(|blob| blob.operation_id.as_str())
        .collect::<Vec<_>>();
    if candidate_plan.review_packet_sha256 != packet.packet_sha256
        || candidate_plan.change_set_sha256 != packet.change_set.change_set_sha256()
        || candidate_plan.commit_group_sha256 != group.group_sha256
        || candidate.repository_sha256 != packet.repository_sha256
        || candidate.parent_object != packet.base_object
        || blob_operations
            != group
                .operation_ids
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>()
    {
        return Err(LocalCommitError::ReviewMismatch);
    }
    let identity_sha256 = canonical_sha256(&(author.clone(), committer.clone()))?;
    let mut plan = LocalCommitPlan {
        schema_version: SCHEMA_VERSION,
        commit_transaction_id: commit_transaction_id.to_owned(),
        repository_sha256: packet.repository_sha256.clone(),
        review_packet_sha256: packet.packet_sha256.clone(),
        candidate_tree_receipt_sha256: candidate.receipt_sha256.clone(),
        commit_group_sha256: group.group_sha256.clone(),
        parent_object: packet.base_object.clone(),
        candidate_tree: candidate.candidate_tree.clone(),
        message: group.proposed_message.clone(),
        message_sha256: sha256_hex(group.proposed_message.as_bytes()),
        author,
        committer,
        identity_sha256,
        timestamp_epoch_seconds,
        timezone_offset_minutes,
        signer,
        task_branch: task_branch.to_owned(),
        preservation_manifest_sha256: candidate.after_manifest_sha256.clone(),
        user_index_sha256: candidate.user_index_sha256.clone(),
        automatic_commit: false,
        push_authority: false,
        merge_authority: false,
        release_authority: false,
        history_rewrite_authority: false,
        reset_authority: false,
        discard_authority: false,
        force_authority: false,
        network_authority: false,
        plan_sha256: ZERO_SHA256.to_owned(),
    };
    plan.plan_sha256 = canonical_sha256(&plan)?;
    verify_local_commit_plan(&plan)?;
    Ok(plan)
}

/// Revalidates every exact commit field and prohibition.
pub fn verify_local_commit_plan(plan: &LocalCommitPlan) -> Result<(), LocalCommitError> {
    plan.signer.verify()?;
    if plan.schema_version != SCHEMA_VERSION
        || !valid_identifier(&plan.commit_transaction_id)
        || !is_sha256(&plan.repository_sha256)
        || !is_sha256(&plan.review_packet_sha256)
        || !is_sha256(&plan.candidate_tree_receipt_sha256)
        || !is_sha256(&plan.commit_group_sha256)
        || !valid_object_id(&plan.parent_object)
        || !valid_object_id(&plan.candidate_tree)
        || plan.message.is_empty()
        || plan.message.len() > MAX_COMMIT_MESSAGE_BYTES
        || plan.message.as_bytes().contains(&0)
        || sha256_hex(plan.message.as_bytes()) != plan.message_sha256
        || !valid_identity(&plan.author)
        || !valid_identity(&plan.committer)
        || canonical_sha256(&(plan.author.clone(), plan.committer.clone()))? != plan.identity_sha256
        || !(-720..=840).contains(&plan.timezone_offset_minutes)
        || !valid_task_branch(&plan.task_branch)
        || !is_sha256(&plan.preservation_manifest_sha256)
        || !is_sha256(&plan.user_index_sha256)
        || plan.automatic_commit
        || plan.push_authority
        || plan.merge_authority
        || plan.release_authority
        || plan.history_rewrite_authority
        || plan.reset_authority
        || plan.discard_authority
        || plan.force_authority
        || plan.network_authority
        || !is_sha256(&plan.plan_sha256)
    {
        return Err(LocalCommitError::InvalidInput);
    }
    let mut canonical = plan.clone();
    canonical.plan_sha256 = ZERO_SHA256.to_owned();
    if canonical_sha256(&canonical)? != plan.plan_sha256 {
        return Err(LocalCommitError::InvalidInput);
    }
    Ok(())
}

/// Trusted manual approval observation from an interactive local surface.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ManualCommitApprovalDecision {
    /// Stable one-shot approval identity.
    pub approval_id: String,
    /// Exact trusted local approval-channel artifact identity.
    pub approval_channel_sha256: String,
    /// Kernel-observed approval timestamp.
    pub approved_at_epoch_ms: u64,
    /// Kernel-enforced approval expiry.
    pub expires_at_epoch_ms: u64,
    /// The user explicitly confirmed the complete exact preview.
    pub human_confirmed: bool,
}

/// Exact one-shot manual approval bound to all commit-significant fields.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ManualCommitApprovalReceipt {
    /// Closed schema version.
    pub schema_version: u16,
    /// Stable one-shot approval identity.
    pub approval_id: String,
    /// Exact local commit plan.
    pub commit_plan_sha256: String,
    /// Exact candidate tree.
    pub candidate_tree: String,
    /// Exact parent.
    pub parent_object: String,
    /// Exact message identity.
    pub message_sha256: String,
    /// Exact author and committer identity.
    pub identity_sha256: String,
    /// Exact externally inspected signer.
    pub signer_sha256: String,
    /// Exact task branch.
    pub task_branch: String,
    /// Exact pre-effect preservation manifest.
    pub preservation_manifest_sha256: String,
    /// Exact trusted local approval-channel artifact.
    pub approval_channel_sha256: String,
    /// Kernel-observed approval timestamp.
    pub approved_at_epoch_ms: u64,
    /// Kernel-enforced approval expiry.
    pub expires_at_epoch_ms: u64,
    /// Approval was explicitly human-confirmed.
    pub human_confirmed: bool,
    /// Model output can never constitute approval.
    pub model_confirmed: bool,
    /// Approval is not execution authority by itself.
    pub execution_authority: bool,
    /// Canonical receipt identity.
    pub receipt_sha256: String,
}

/// Records an exact current manual approval without launching Git.
pub fn record_manual_commit_approval(
    plan: &LocalCommitPlan,
    decision: ManualCommitApprovalDecision,
) -> Result<ManualCommitApprovalReceipt, LocalCommitError> {
    verify_local_commit_plan(plan)?;
    if !valid_identifier(&decision.approval_id)
        || !is_sha256(&decision.approval_channel_sha256)
        || !decision.human_confirmed
        || decision.expires_at_epoch_ms <= decision.approved_at_epoch_ms
        || decision.expires_at_epoch_ms - decision.approved_at_epoch_ms > MAX_APPROVAL_TTL_MS
    {
        return Err(LocalCommitError::ApprovalDenied);
    }
    let mut receipt = ManualCommitApprovalReceipt {
        schema_version: SCHEMA_VERSION,
        approval_id: decision.approval_id,
        commit_plan_sha256: plan.plan_sha256.clone(),
        candidate_tree: plan.candidate_tree.clone(),
        parent_object: plan.parent_object.clone(),
        message_sha256: plan.message_sha256.clone(),
        identity_sha256: plan.identity_sha256.clone(),
        signer_sha256: plan.signer.signer_sha256.clone(),
        task_branch: plan.task_branch.clone(),
        preservation_manifest_sha256: plan.preservation_manifest_sha256.clone(),
        approval_channel_sha256: decision.approval_channel_sha256,
        approved_at_epoch_ms: decision.approved_at_epoch_ms,
        expires_at_epoch_ms: decision.expires_at_epoch_ms,
        human_confirmed: true,
        model_confirmed: false,
        execution_authority: false,
        receipt_sha256: ZERO_SHA256.to_owned(),
    };
    receipt.receipt_sha256 = canonical_sha256(&receipt)?;
    Ok(receipt)
}

/// Verifies exact approval bindings and currentness at effect launch.
pub fn verify_manual_commit_approval(
    plan: &LocalCommitPlan,
    approval: &ManualCommitApprovalReceipt,
    now_epoch_ms: u64,
) -> Result<(), LocalCommitError> {
    verify_local_commit_plan(plan)?;
    let mut canonical = approval.clone();
    canonical.receipt_sha256 = ZERO_SHA256.to_owned();
    if approval.schema_version != SCHEMA_VERSION
        || !valid_identifier(&approval.approval_id)
        || approval.commit_plan_sha256 != plan.plan_sha256
        || approval.candidate_tree != plan.candidate_tree
        || approval.parent_object != plan.parent_object
        || approval.message_sha256 != plan.message_sha256
        || approval.identity_sha256 != plan.identity_sha256
        || approval.signer_sha256 != plan.signer.signer_sha256
        || approval.task_branch != plan.task_branch
        || approval.preservation_manifest_sha256 != plan.preservation_manifest_sha256
        || !is_sha256(&approval.approval_channel_sha256)
        || !approval.human_confirmed
        || approval.model_confirmed
        || approval.execution_authority
        || now_epoch_ms < approval.approved_at_epoch_ms
        || now_epoch_ms >= approval.expires_at_epoch_ms
        || approval.expires_at_epoch_ms - approval.approved_at_epoch_ms > MAX_APPROVAL_TTL_MS
        || canonical_sha256(&canonical)? != approval.receipt_sha256
    {
        return Err(LocalCommitError::ApprovalDenied);
    }
    Ok(())
}

/// Trusted terminal observations from one exact signed local commit attempt.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LocalCommitPlatformResult {
    /// Terminal outcome.
    pub outcome: OperationOutcome,
    /// Manifest immediately before launch.
    pub before: RepositoryPreservationManifest,
    /// Manifest after completion or recovery.
    pub after: RepositoryPreservationManifest,
    /// Exact created commit object on success.
    pub commit_object: Option<String>,
    /// Parent observed by independent commit inspection.
    pub observed_parent: Option<String>,
    /// Tree observed by independent commit inspection.
    pub observed_tree: Option<String>,
    /// Message digest observed by independent commit inspection.
    pub observed_message_sha256: Option<String>,
    /// Identity digest observed by independent commit inspection.
    pub observed_identity_sha256: Option<String>,
    /// Signer fingerprint observed during signature verification.
    pub observed_signer_fingerprint: Option<String>,
    /// Independent signature verification succeeded.
    pub signature_verified: bool,
    /// Exact task branch value before compare-and-swap.
    pub task_branch_before: Option<String>,
    /// Exact task branch value after compare-and-swap.
    pub task_branch_after: Option<String>,
    /// Exact user index after the operation.
    pub user_index_after_sha256: String,
    /// Exact remotes snapshot after the operation.
    pub remotes_after_sha256: String,
    /// Whether a repository hook ran.
    pub hooks_executed: bool,
    /// Whether a content transform ran.
    pub filters_executed: bool,
    /// Whether repository-selected configuration was used.
    pub repository_configuration_used: bool,
    /// Whether any network connection was attempted.
    pub network_used: bool,
    /// Every attributed process and temporary artifact was reconciled.
    pub cleanup_verified: bool,
    /// Stable content-free platform code.
    pub platform_code: String,
}

/// Hash-bound terminal receipt proving one exact signed local commit only.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LocalCommitReceipt {
    /// Closed schema version.
    pub schema_version: u16,
    /// Exact local commit transaction.
    pub commit_transaction_id: String,
    /// Exact authority transaction.
    pub authority_transaction_id: String,
    /// Exact non-replayable operation attempt.
    pub operation_attempt_id: String,
    /// Exact commit plan.
    pub commit_plan_sha256: String,
    /// Exact manual approval.
    pub approval_receipt_sha256: String,
    /// Exact parent.
    pub parent_object: String,
    /// Exact tree.
    pub candidate_tree: String,
    /// Exact created signed commit.
    pub commit_object: String,
    /// Exact task branch updated by compare-and-swap.
    pub task_branch: String,
    /// Exact externally inspected signer.
    pub signer_sha256: String,
    /// Signature verification succeeded.
    pub signature_verified: bool,
    /// User index remained unchanged.
    pub user_index_unchanged: bool,
    /// Remotes remained unchanged.
    pub remotes_unchanged: bool,
    /// No hooks or content drivers ran.
    pub repository_execution_surfaces_disabled: bool,
    /// No remote effect occurred.
    pub network_used: bool,
    /// Terminal outcome.
    pub outcome: OperationOutcome,
    /// Exact pre-effect manifest.
    pub before_manifest_sha256: String,
    /// Exact post-effect manifest.
    pub after_manifest_sha256: String,
    /// Stable content-free platform code.
    pub platform_code: String,
    /// Canonical receipt digest.
    pub receipt_sha256: String,
}

/// Reconciles exact commit, signature, branch, index, remote, and repository postconditions.
pub fn reconcile_local_commit(
    plan: &LocalCommitPlan,
    approval: &ManualCommitApprovalReceipt,
    result: LocalCommitPlatformResult,
    authority_transaction_id: &str,
    operation_attempt_id: &str,
    now_epoch_ms: u64,
) -> Result<LocalCommitReceipt, LocalCommitError> {
    verify_manual_commit_approval(plan, approval, now_epoch_ms)?;
    result
        .before
        .verify()
        .map_err(|_| LocalCommitError::ResultDenied)?;
    result
        .after
        .verify()
        .map_err(|_| LocalCommitError::ResultDenied)?;
    let commit_object = result
        .commit_object
        .filter(|object| valid_object_id(object))
        .ok_or(LocalCommitError::ResultDenied)?;
    if !valid_identifier(authority_transaction_id)
        || !valid_identifier(operation_attempt_id)
        || result.outcome != OperationOutcome::Succeeded
        || result.before.manifest_sha256 != plan.preservation_manifest_sha256
        || result.before.repository_sha256 != plan.repository_sha256
        || result.before.index_sha256 != plan.user_index_sha256
        || result.observed_parent.as_deref() != Some(plan.parent_object.as_str())
        || result.observed_tree.as_deref() != Some(plan.candidate_tree.as_str())
        || result.observed_message_sha256.as_deref() != Some(plan.message_sha256.as_str())
        || result.observed_identity_sha256.as_deref() != Some(plan.identity_sha256.as_str())
        || result.observed_signer_fingerprint.as_deref()
            != Some(plan.signer.key_fingerprint.as_str())
        || !result.signature_verified
        || result.task_branch_before.as_deref() != Some(plan.parent_object.as_str())
        || result.task_branch_after.as_deref() != Some(commit_object.as_str())
        || result.user_index_after_sha256 != plan.user_index_sha256
        || result.remotes_after_sha256 != result.before.remotes_sha256
        || result.hooks_executed
        || result.filters_executed
        || result.repository_configuration_used
        || result.network_used
        || !result.cleanup_verified
        || !valid_platform_code(&result.platform_code)
    {
        return Err(LocalCommitError::ResultDenied);
    }
    reconcile_preservation(
        &result.before,
        &result.after,
        RepositoryOwnedDelta::LocalCommit,
    )
    .map_err(|_| LocalCommitError::ResultDenied)?;
    let mut receipt = LocalCommitReceipt {
        schema_version: SCHEMA_VERSION,
        commit_transaction_id: plan.commit_transaction_id.clone(),
        authority_transaction_id: authority_transaction_id.to_owned(),
        operation_attempt_id: operation_attempt_id.to_owned(),
        commit_plan_sha256: plan.plan_sha256.clone(),
        approval_receipt_sha256: approval.receipt_sha256.clone(),
        parent_object: plan.parent_object.clone(),
        candidate_tree: plan.candidate_tree.clone(),
        commit_object,
        task_branch: plan.task_branch.clone(),
        signer_sha256: plan.signer.signer_sha256.clone(),
        signature_verified: true,
        user_index_unchanged: true,
        remotes_unchanged: true,
        repository_execution_surfaces_disabled: true,
        network_used: false,
        outcome: result.outcome,
        before_manifest_sha256: result.before.manifest_sha256,
        after_manifest_sha256: result.after.manifest_sha256,
        platform_code: result.platform_code,
        receipt_sha256: ZERO_SHA256.to_owned(),
    };
    receipt.receipt_sha256 = canonical_sha256(&receipt)?;
    Ok(receipt)
}

#[derive(Clone, Debug, Serialize)]
#[serde(deny_unknown_fields)]
struct CommitExecutionRequest<'a> {
    plan: &'a LocalCommitPlan,
    approval_receipt_sha256: &'a str,
}

/// Authority-free exact bytes used for one GitCommit grant and call binding.
#[derive(Clone, Debug)]
pub struct PreparedLocalCommit {
    plan: LocalCommitPlan,
    approval: ManualCommitApprovalReceipt,
    request_bytes: Vec<u8>,
    request_sha256: String,
}

impl PreparedLocalCommit {
    /// Validates and serializes one exact current manual commit request.
    pub fn new(
        plan: LocalCommitPlan,
        approval: ManualCommitApprovalReceipt,
        now_epoch_ms: u64,
    ) -> Result<Self, LocalCommitError> {
        verify_manual_commit_approval(&plan, &approval, now_epoch_ms)?;
        let request_bytes = serde_json::to_vec(&CommitExecutionRequest {
            plan: &plan,
            approval_receipt_sha256: &approval.receipt_sha256,
        })
        .map_err(|_| LocalCommitError::ReceiptFailure)?;
        let request_sha256 = sha256_hex(&request_bytes);
        Ok(Self {
            plan,
            approval,
            request_bytes,
            request_sha256,
        })
    }

    /// Returns exact canonical request bytes for grant and tool-call binding.
    #[must_use]
    pub fn request_bytes(&self) -> &[u8] {
        &self.request_bytes
    }

    /// Returns the digest of the exact request bytes.
    #[must_use]
    pub fn request_sha256(&self) -> &str {
        &self.request_sha256
    }

    /// Returns the exact approval-ready plan.
    #[must_use]
    pub const fn plan(&self) -> &LocalCommitPlan {
        &self.plan
    }
}

/// Nonforgeable launch permit created only after exact GitCommit authority validation.
pub struct LocalCommitLaunchPermit<'plan> {
    plan: &'plan LocalCommitPlan,
    before: &'plan RepositoryPreservationManifest,
}

impl LocalCommitLaunchPermit<'_> {
    /// Returns the exact approved commit plan.
    #[must_use]
    pub const fn plan(&self) -> &LocalCommitPlan {
        self.plan
    }

    /// Returns the exact current pre-effect preservation manifest.
    #[must_use]
    pub const fn before(&self) -> &RepositoryPreservationManifest {
        self.before
    }
}

impl fmt::Debug for LocalCommitLaunchPermit<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("LocalCommitLaunchPermit")
            .field("transaction_id", &self.plan.commit_transaction_id)
            .field("task_branch", &self.plan.task_branch)
            .finish_non_exhaustive()
    }
}

/// Trusted platform executor whose launch requires one kernel-created permit.
pub trait BoundedLocalCommitExecutor {
    /// Executes one exact local signed commit plan without network access.
    fn execute(
        &mut self,
        permit: LocalCommitLaunchPermit<'_>,
        cancellation: &CancellationToken,
    ) -> LocalCommitPlatformResult;
}

/// Inert local commit driver crossing the effect boundary only with consumed authority.
pub struct LocalCommitEffectDriver<E, H> {
    executor: E,
    held_repository_scope: H,
    prepared: PreparedLocalCommit,
    before: RepositoryPreservationManifest,
    cancellation: CancellationToken,
    now_epoch_ms: u64,
    receipt: Option<LocalCommitReceipt>,
    error: Option<LocalCommitError>,
}

impl<E, H> LocalCommitEffectDriver<E, H> {
    /// Creates an inert driver after exact approval and manifest validation.
    pub fn new(
        executor: E,
        held_repository_scope: H,
        prepared: PreparedLocalCommit,
        before: RepositoryPreservationManifest,
        cancellation: CancellationToken,
        now_epoch_ms: u64,
    ) -> Result<Self, LocalCommitError> {
        before
            .verify()
            .map_err(|_| LocalCommitError::ResultDenied)?;
        verify_manual_commit_approval(&prepared.plan, &prepared.approval, now_epoch_ms)?;
        if before.manifest_sha256 != prepared.plan.preservation_manifest_sha256
            || before.repository_sha256 != prepared.plan.repository_sha256
            || before.index_sha256 != prepared.plan.user_index_sha256
        {
            return Err(LocalCommitError::ResultDenied);
        }
        Ok(Self {
            executor,
            held_repository_scope,
            prepared,
            before,
            cancellation,
            now_epoch_ms,
            receipt: None,
            error: None,
        })
    }

    /// Takes the terminal local commit receipt after mediated execution.
    pub fn take_receipt(&mut self) -> Option<LocalCommitReceipt> {
        self.receipt.take()
    }

    /// Takes a content-free local commit boundary error.
    pub fn take_error(&mut self) -> Option<LocalCommitError> {
        self.error.take()
    }

    /// Returns the executor after the attempt closes.
    #[must_use]
    pub fn into_executor(self) -> E {
        self.executor
    }
}

impl<E, H> fmt::Debug for LocalCommitEffectDriver<E, H> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("LocalCommitEffectDriver")
            .field("transaction_id", &self.prepared.plan.commit_transaction_id)
            .field("has_receipt", &self.receipt.is_some())
            .field("has_error", &self.error.is_some())
            .finish_non_exhaustive()
    }
}

impl<E, H> EffectDriver for LocalCommitEffectDriver<E, H>
where
    E: BoundedLocalCommitExecutor,
    H: HeldWorkspaceObject,
{
    fn execute(&mut self, authorization: EffectAuthorization<'_>) -> EffectLaunch {
        let call = authorization.call();
        if authorization.operation().operation() != GrantOperation::GitCommit
            || !authorization.authorizes_held_object(&self.held_repository_scope)
            || call.arguments.sha256 != self.prepared.request_sha256
            || call.arguments.bytes != self.prepared.request_bytes
            || sha256_hex(&call.arguments.bytes) != call.arguments.sha256
            || verify_manual_commit_approval(
                &self.prepared.plan,
                &self.prepared.approval,
                self.now_epoch_ms,
            )
            .is_err()
        {
            self.error = Some(LocalCommitError::ApprovalDenied);
            return EffectLaunch::failed();
        }
        if self.cancellation.is_cancelled() {
            self.error = Some(LocalCommitError::ResultDenied);
            return EffectLaunch::failed();
        }
        let result = self.executor.execute(
            LocalCommitLaunchPermit {
                plan: &self.prepared.plan,
                before: &self.before,
            },
            &self.cancellation,
        );
        match reconcile_local_commit(
            &self.prepared.plan,
            &self.prepared.approval,
            result,
            authorization.transaction_id().as_str(),
            authorization.attempt_id().as_str(),
            self.now_epoch_ms,
        ) {
            Ok(receipt) => {
                let effect = EffectResult::from_redacted_material(
                    receipt.outcome,
                    receipt.receipt_sha256.as_bytes(),
                    StateChange::Changed,
                );
                self.receipt = Some(receipt);
                EffectLaunch::completed(effect)
            }
            Err(error) => {
                self.error = Some(error);
                EffectLaunch::failed()
            }
        }
    }
}

/// Verifies a candidate receipt against the exact authority-free construction plan.
pub fn verify_candidate_tree_receipt(
    plan: &CandidateTreePlan,
    receipt: &CandidateTreeReceipt,
) -> Result<(), LocalCommitError> {
    verify_candidate_tree_plan(plan)?;
    let mut canonical = receipt.clone();
    canonical.receipt_sha256 = ZERO_SHA256.to_owned();
    if receipt.schema_version != SCHEMA_VERSION
        || !valid_identifier(&receipt.candidate_build_id)
        || receipt.candidate_build_id != plan.candidate_build_id
        || !is_sha256(&receipt.candidate_plan_sha256)
        || receipt.candidate_plan_sha256 != plan.plan_sha256
        || !is_sha256(&receipt.repository_sha256)
        || receipt.repository_sha256 != plan.repository_sha256
        || !valid_object_id(&receipt.parent_object)
        || receipt.parent_object != plan.parent_object
        || !valid_object_id(&receipt.candidate_tree)
        || receipt.blobs.is_empty()
        || receipt.blobs.len() > MAX_COMMIT_FILES
        || !receipt.blobs.windows(2).all(|pair| pair[0] < pair[1])
        || receipt.blobs.iter().any(|blob| {
            !valid_identifier(&blob.operation_id)
                || !is_sha256(&blob.postimage_sha256)
                || !valid_object_id(&blob.blob_object)
        })
        || !is_sha256(&receipt.before_manifest_sha256)
        || !is_sha256(&receipt.after_manifest_sha256)
        || !is_sha256(&receipt.user_index_sha256)
        || receipt.user_index_sha256 != plan.user_index_sha256
        || receipt.ref_update_authority
        || !valid_platform_code(&receipt.platform_code)
        || canonical_sha256(&canonical)? != receipt.receipt_sha256
    {
        return Err(LocalCommitError::CandidateTreeMismatch);
    }
    Ok(())
}

fn valid_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b':' | b'-'))
}

fn valid_path(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 4_096
        && !value.starts_with('/')
        && !value.contains('\\')
        && !value.chars().any(char::is_control)
        && value
            .split('/')
            .all(|component| !component.is_empty() && !matches!(component, "." | ".."))
}

fn valid_object_id(value: &str) -> bool {
    matches!(value.len(), 40 | 64)
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}

fn valid_fingerprint(value: &str) -> bool {
    matches!(value.len(), 40 | 64)
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}

fn valid_identity(identity: &CommitIdentity) -> bool {
    !identity.name.trim().is_empty()
        && identity.name.len() <= 128
        && !identity.name.chars().any(char::is_control)
        && identity.email.len() <= 320
        && identity
            .email
            .split_once('@')
            .is_some_and(|(local, domain)| {
                !local.is_empty()
                    && !domain.is_empty()
                    && !local.chars().any(char::is_control)
                    && domain
                        .bytes()
                        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-'))
            })
}

fn valid_task_branch(value: &str) -> bool {
    value.starts_with("refs/heads/agentmage/tasks/")
        && value.len() <= 512
        && !value.contains("..")
        && !value.ends_with('/')
        && !value.ends_with('.')
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'/' | b'.' | b'_' | b'-'))
}

fn valid_platform_code(value: &str) -> bool {
    valid_identifier(value)
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}

fn canonical_sha256(value: &impl Serialize) -> Result<String, LocalCommitError> {
    serde_json::to_vec(value)
        .map(|bytes| sha256_hex(&bytes))
        .map_err(|_| LocalCommitError::ReceiptFailure)
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(64);
    for byte in Sha256::digest(bytes) {
        write!(&mut output, "{byte:02x}").expect("writing to String cannot fail");
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hash(character: char) -> String {
        character.to_string().repeat(64)
    }

    fn object(character: char) -> String {
        character.to_string().repeat(40)
    }

    fn manifest(object_database: char, agent_refs: char) -> RepositoryPreservationManifest {
        RepositoryPreservationManifest::seal(RepositoryPreservationManifest {
            schema_version: 0,
            repository_sha256: hash('1'),
            common_directory_sha256: hash('2'),
            object_format: "sha1".to_owned(),
            safe_ownership: true,
            head_object: Some(object('1')),
            current_branch: Some("refs/heads/main".to_owned()),
            upstream: Some("refs/remotes/origin/main".to_owned()),
            detached: false,
            index_sha256: hash('3'),
            path_dispositions_sha256: hash('4'),
            untracked_count: 1,
            ignored_count: 2,
            local_branches_sha256: hash('5'),
            remote_tracking_refs_sha256: hash('6'),
            tags_sha256: hash('7'),
            notes_sha256: hash('8'),
            stash_sha256: hash('9'),
            replacement_refs_sha256: hash('a'),
            reflogs_sha256: hash('b'),
            agentmage_refs_sha256: hash(agent_refs),
            worktrees_sha256: hash('d'),
            submodules_sha256: hash('e'),
            lfs_sha256: hash('f'),
            object_database_sha256: hash(object_database),
            configuration_sha256: hash('2'),
            hooks_sha256: hash('3'),
            content_drivers_sha256: hash('4'),
            remotes_sha256: hash('5'),
            operations_sha256: hash('6'),
            shallow: false,
            partial: false,
            hazardous_configuration: false,
            manifest_sha256: String::new(),
        })
        .expect("manifest")
    }

    fn candidate_plan(before: &RepositoryPreservationManifest) -> CandidateTreePlan {
        seal_candidate_tree_plan(CandidateTreePlan {
            schema_version: SCHEMA_VERSION,
            candidate_build_id: "candidate-build-0001".to_owned(),
            repository_sha256: before.repository_sha256.clone(),
            review_packet_sha256: hash('7'),
            change_set_sha256: hash('8'),
            commit_group_sha256: hash('9'),
            parent_object: object('a'),
            temporary_index_path_sha256: hash('b'),
            user_index_sha256: before.index_sha256.clone(),
            preservation_manifest_sha256: before.manifest_sha256.clone(),
            files: vec![CandidateTreeFilePlan {
                operation_id: "operation-01".to_owned(),
                path: "src/lib.rs".to_owned(),
                postimage_sha256: hash('c'),
                mode: CommitFileMode::Regular,
            }],
            hooks_enabled: false,
            filters_enabled: false,
            repository_configuration_enabled: false,
            network_authority: false,
            plan_sha256: ZERO_SHA256.to_owned(),
        })
        .expect("candidate plan")
    }

    fn candidate_receipt(
        before: &RepositoryPreservationManifest,
        after: &RepositoryPreservationManifest,
    ) -> CandidateTreeReceipt {
        let plan = candidate_plan(before);
        reconcile_candidate_tree(
            &plan,
            CandidateTreePlatformResult {
                before: before.clone(),
                after: after.clone(),
                outcome: OperationOutcome::Succeeded,
                candidate_tree: Some(object('b')),
                blobs: vec![CandidateTreeBlob {
                    operation_id: "operation-01".to_owned(),
                    postimage_sha256: hash('c'),
                    blob_object: object('c'),
                }],
                user_index_after_sha256: before.index_sha256.clone(),
                owned_temporary_index: true,
                cleanup_verified: true,
                hooks_executed: false,
                filters_executed: false,
                repository_configuration_used: false,
                network_used: false,
                platform_code: "linux.git.candidate.succeeded".to_owned(),
            },
        )
        .expect("candidate receipt")
    }

    fn signer() -> PinnedCommitSigner {
        PinnedCommitSigner::seal(PinnedCommitSigner {
            schema_version: 0,
            signer_id: "signer-openpgp-0001".to_owned(),
            kind: CommitSignerKind::OpenPgp,
            source: SignerInspectionSource::ExternalKeyring,
            signer_path_sha256: hash('7'),
            signer_executable_sha256: hash('8'),
            key_fingerprint: "1234567890abcdef1234567890abcdef12345678".to_owned(),
            public_key_sha256: hash('9'),
            inspection_evidence_sha256: hash('a'),
            repository_configuration_used: false,
            unsigned_fallback: false,
            signer_sha256: String::new(),
        })
        .expect("signer")
    }

    fn identity() -> CommitIdentity {
        CommitIdentity {
            name: "Fixture User".to_owned(),
            email: "fixture@example.test".to_owned(),
        }
    }

    fn local_plan(
        before: &RepositoryPreservationManifest,
        candidate: &CandidateTreeReceipt,
    ) -> LocalCommitPlan {
        let author = identity();
        let committer = identity();
        let message = "feat: apply approved behavior change\n\nFiles: 1".to_owned();
        let mut plan = LocalCommitPlan {
            schema_version: SCHEMA_VERSION,
            commit_transaction_id: "commit-transaction-0001".to_owned(),
            repository_sha256: before.repository_sha256.clone(),
            review_packet_sha256: hash('7'),
            candidate_tree_receipt_sha256: candidate.receipt_sha256.clone(),
            commit_group_sha256: hash('9'),
            parent_object: candidate.parent_object.clone(),
            candidate_tree: candidate.candidate_tree.clone(),
            message_sha256: sha256_hex(message.as_bytes()),
            message,
            identity_sha256: canonical_sha256(&(author.clone(), committer.clone()))
                .expect("identity"),
            author,
            committer,
            timestamp_epoch_seconds: 1_787_000_000,
            timezone_offset_minutes: -300,
            signer: signer(),
            task_branch: "refs/heads/agentmage/tasks/fixture".to_owned(),
            preservation_manifest_sha256: before.manifest_sha256.clone(),
            user_index_sha256: before.index_sha256.clone(),
            automatic_commit: false,
            push_authority: false,
            merge_authority: false,
            release_authority: false,
            history_rewrite_authority: false,
            reset_authority: false,
            discard_authority: false,
            force_authority: false,
            network_authority: false,
            plan_sha256: ZERO_SHA256.to_owned(),
        };
        plan.plan_sha256 = canonical_sha256(&plan).expect("plan");
        plan
    }

    fn approval(plan: &LocalCommitPlan) -> ManualCommitApprovalReceipt {
        record_manual_commit_approval(
            plan,
            ManualCommitApprovalDecision {
                approval_id: "approval-commit-0001".to_owned(),
                approval_channel_sha256: hash('d'),
                approved_at_epoch_ms: 10_000,
                expires_at_epoch_ms: 20_000,
                human_confirmed: true,
            },
        )
        .expect("approval")
    }

    #[test]
    fn candidate_tree_requires_owned_index_exact_bytes_and_no_execution_surfaces() {
        let before = manifest('7', 'c');
        let after = manifest('8', 'c');
        let plan = candidate_plan(&before);
        assert!(verify_candidate_tree_plan(&plan).is_ok());
        let receipt = candidate_receipt(&before, &after);
        assert!(verify_candidate_tree_receipt(&plan, &receipt).is_ok());
        assert!(!receipt.ref_update_authority);

        for mutation in ["index", "hook", "filter", "config", "network", "ref"] {
            let mut result = CandidateTreePlatformResult {
                before: before.clone(),
                after: after.clone(),
                outcome: OperationOutcome::Succeeded,
                candidate_tree: Some(object('b')),
                blobs: vec![CandidateTreeBlob {
                    operation_id: "operation-01".to_owned(),
                    postimage_sha256: hash('c'),
                    blob_object: object('c'),
                }],
                user_index_after_sha256: before.index_sha256.clone(),
                owned_temporary_index: true,
                cleanup_verified: true,
                hooks_executed: false,
                filters_executed: false,
                repository_configuration_used: false,
                network_used: false,
                platform_code: "linux.git.candidate.succeeded".to_owned(),
            };
            match mutation {
                "index" => result.user_index_after_sha256 = hash('e'),
                "hook" => result.hooks_executed = true,
                "filter" => result.filters_executed = true,
                "config" => result.repository_configuration_used = true,
                "network" => result.network_used = true,
                "ref" => result.after = manifest('8', 'e'),
                _ => unreachable!(),
            }
            assert_eq!(
                reconcile_candidate_tree(&plan, result),
                Err(LocalCommitError::CandidateTreeMismatch)
            );
        }
    }

    #[test]
    fn approval_binds_tree_parent_message_identity_signer_branch_and_expiry() {
        let candidate_before = manifest('7', 'c');
        let commit_before = manifest('8', 'c');
        let candidate = candidate_receipt(&candidate_before, &commit_before);
        let plan = local_plan(&commit_before, &candidate);
        assert!(verify_local_commit_plan(&plan).is_ok());
        let approval = approval(&plan);
        assert!(verify_manual_commit_approval(&plan, &approval, 15_000).is_ok());
        let prepared = PreparedLocalCommit::new(plan.clone(), approval.clone(), 15_000)
            .expect("prepared commit");
        assert_eq!(
            sha256_hex(prepared.request_bytes()),
            prepared.request_sha256()
        );
        assert_eq!(prepared.plan(), &plan);
        assert_eq!(
            PreparedLocalCommit::new(plan.clone(), approval.clone(), 20_000)
                .expect_err("expired preparation")
                .code(),
            LocalCommitError::ApprovalDenied.code()
        );
        assert_eq!(
            verify_manual_commit_approval(&plan, &approval, 20_000),
            Err(LocalCommitError::ApprovalDenied)
        );

        for mutation in ["tree", "parent", "message", "identity", "signer", "branch"] {
            let mut changed = plan.clone();
            match mutation {
                "tree" => changed.candidate_tree = object('d'),
                "parent" => changed.parent_object = object('e'),
                "message" => changed.message = "changed".to_owned(),
                "identity" => changed.author.email = "changed@example.test".to_owned(),
                "signer" => changed.signer.key_fingerprint = "a".repeat(40),
                "branch" => changed.task_branch = "refs/heads/main".to_owned(),
                _ => unreachable!(),
            }
            changed.plan_sha256 = ZERO_SHA256.to_owned();
            changed.plan_sha256 = canonical_sha256(&changed).expect("changed plan");
            assert!(verify_manual_commit_approval(&changed, &approval, 15_000).is_err());
        }
    }

    #[test]
    fn signed_commit_reconciliation_rejects_index_remote_signer_and_surface_drift() {
        let candidate_before = manifest('7', 'c');
        let commit_before = manifest('8', 'c');
        let commit_after = manifest('9', 'd');
        let candidate = candidate_receipt(&candidate_before, &commit_before);
        let plan = local_plan(&commit_before, &candidate);
        let approval = approval(&plan);
        let commit_object = object('f');
        let result = || LocalCommitPlatformResult {
            outcome: OperationOutcome::Succeeded,
            before: commit_before.clone(),
            after: commit_after.clone(),
            commit_object: Some(commit_object.clone()),
            observed_parent: Some(plan.parent_object.clone()),
            observed_tree: Some(plan.candidate_tree.clone()),
            observed_message_sha256: Some(plan.message_sha256.clone()),
            observed_identity_sha256: Some(plan.identity_sha256.clone()),
            observed_signer_fingerprint: Some(plan.signer.key_fingerprint.clone()),
            signature_verified: true,
            task_branch_before: Some(plan.parent_object.clone()),
            task_branch_after: Some(commit_object.clone()),
            user_index_after_sha256: plan.user_index_sha256.clone(),
            remotes_after_sha256: commit_before.remotes_sha256.clone(),
            hooks_executed: false,
            filters_executed: false,
            repository_configuration_used: false,
            network_used: false,
            cleanup_verified: true,
            platform_code: "linux.git.commit.succeeded".to_owned(),
        };
        let receipt = reconcile_local_commit(
            &plan,
            &approval,
            result(),
            "authority-transaction-0001",
            "operation-attempt-0001",
            15_000,
        )
        .expect("commit receipt");
        assert!(receipt.signature_verified);
        assert!(receipt.user_index_unchanged);
        assert!(receipt.remotes_unchanged);
        assert!(!receipt.network_used);

        for mutation in [
            "index",
            "remote",
            "signer",
            "signature",
            "hook",
            "filter",
            "config",
            "branch",
        ] {
            let mut changed = result();
            match mutation {
                "index" => changed.user_index_after_sha256 = hash('e'),
                "remote" => changed.remotes_after_sha256 = hash('e'),
                "signer" => changed.observed_signer_fingerprint = Some("a".repeat(40)),
                "signature" => changed.signature_verified = false,
                "hook" => changed.hooks_executed = true,
                "filter" => changed.filters_executed = true,
                "config" => changed.repository_configuration_used = true,
                "branch" => changed.task_branch_before = Some(object('e')),
                _ => unreachable!(),
            }
            assert_eq!(
                reconcile_local_commit(
                    &plan,
                    &approval,
                    changed,
                    "authority-transaction-0001",
                    "operation-attempt-0001",
                    15_000,
                ),
                Err(LocalCommitError::ResultDenied)
            );
        }
    }

    #[test]
    fn signer_and_plan_never_admit_repository_selection_or_publication_authority() {
        let mut untrusted = signer();
        untrusted.repository_configuration_used = true;
        untrusted.signer_sha256 = ZERO_SHA256.to_owned();
        untrusted.signer_sha256 = canonical_sha256(&untrusted).expect("signer digest");
        assert_eq!(untrusted.verify(), Err(LocalCommitError::SignerDenied));

        let candidate_before = manifest('7', 'c');
        let commit_before = manifest('8', 'c');
        let candidate = candidate_receipt(&candidate_before, &commit_before);
        let mut plan = local_plan(&commit_before, &candidate);
        for field in [
            "automatic",
            "push",
            "merge",
            "release",
            "history",
            "reset",
            "discard",
            "force",
            "network",
        ] {
            let mut changed = plan.clone();
            match field {
                "automatic" => changed.automatic_commit = true,
                "push" => changed.push_authority = true,
                "merge" => changed.merge_authority = true,
                "release" => changed.release_authority = true,
                "history" => changed.history_rewrite_authority = true,
                "reset" => changed.reset_authority = true,
                "discard" => changed.discard_authority = true,
                "force" => changed.force_authority = true,
                "network" => changed.network_authority = true,
                _ => unreachable!(),
            }
            changed.plan_sha256 = ZERO_SHA256.to_owned();
            changed.plan_sha256 = canonical_sha256(&changed).expect("changed plan");
            assert_eq!(
                verify_local_commit_plan(&changed),
                Err(LocalCommitError::InvalidInput)
            );
        }
        plan.signer.unsigned_fallback = true;
        plan.signer.signer_sha256 = ZERO_SHA256.to_owned();
        plan.signer.signer_sha256 = canonical_sha256(&plan.signer).expect("signer digest");
        plan.plan_sha256 = ZERO_SHA256.to_owned();
        plan.plan_sha256 = canonical_sha256(&plan).expect("plan digest");
        assert_eq!(
            verify_local_commit_plan(&plan),
            Err(LocalCommitError::SignerDenied)
        );
    }
}
