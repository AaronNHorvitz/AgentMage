// SPDX-License-Identifier: Apache-2.0
//! Live-owner checks before constructing an authority-free candidate-tree plan.

use super::*;
use crate::command_runner::worktree::WorkspaceObservation;
use crate::repository_base::BaseFilePreimage;
use agentmage_kernel_engine::local_commit::{
    CandidateTreePlan, CommitFileMode, plan_candidate_tree, verify_local_commit_review,
};
use agentmage_kernel_engine::propagation::EffectCancellationObservation;
use agentmage_kernel_engine::repository_safety::RepositoryPreservationManifest;
use agentmage_kernel_engine::review_packet::{
    ChangePurposeAssignment, LocalReviewPacket, ReviewableChangeSet, bind_validation_evidence,
};
use std::cell::RefCell;

#[derive(Clone, PartialEq, Eq)]
struct GitObservation {
    original: RepositoryPreservationManifest,
    task_head: String,
    task_branch: Option<String>,
    linked_identity: String,
    task_inventory_sha256: String,
    base_preimages: BTreeMap<String, Option<BaseFilePreimage>>,
}

/// Borrowed read scope for the original owner's complete commit review.
///
/// Obtain only through `LinuxCodingRepository::consented_commit_review` after
/// the trusted caller obtains consent for both displayed roots and offline Git
/// inspection. This process-only view binds that selection; it is not a grant,
/// an independent review verdict, or authority to build or commit anything.
pub struct LinuxCodingCommitReview<'a> {
    owner: &'a LinuxCodingRepository,
}

/// Original-owner inputs delivered only after candidate planning succeeds.
/// These borrows cannot be restored from a packet or serialized receipts.
pub struct LinuxCodingCandidateReviewEvidence<'a> {
    packet: &'a LocalReviewPacket,
    candidate: &'a CandidateTreePlan,
    creation: &'a LinuxCodingGitCreation,
    validation: LinuxCodingValidationEvidence<'a>,
    sources: &'a [LinuxCodingReviewSource],
    cancellation: &'a dyn EffectCancellationObservation,
}

/// Exact bounded source pair captured under the original two-root read scope.
/// Bytes are untrusted local data, not model disclosure or execution authority.
/// No serialization or content-bearing debug representation is provided.
pub struct LinuxCodingReviewSource {
    operation_id: String,
    path: String,
    preimage: Option<Vec<u8>>,
    postimage: Vec<u8>,
    mode: CommitFileMode,
}

impl LinuxCodingReviewSource {
    /// Original write operation in the complete packet.
    #[must_use]
    pub fn operation_id(&self) -> &str {
        &self.operation_id
    }

    /// Exact repository-relative path.
    #[must_use]
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Immutable-base bytes; absence differs from an existing empty file.
    #[must_use]
    pub fn preimage(&self) -> Option<&[u8]> {
        self.preimage.as_deref()
    }

    /// Current owned-worktree bytes, checked against the original write history.
    #[must_use]
    pub fn postimage(&self) -> &[u8] {
        &self.postimage
    }

    /// Candidate Git mode, including the separately approved mode for creation.
    #[must_use]
    pub const fn mode(&self) -> CommitFileMode {
        self.mode
    }
}

impl LinuxCodingCandidateReviewEvidence<'_> {
    /// Exact complete packet, including suppressed findings and artifact claims.
    #[must_use]
    pub fn packet(&self) -> &LocalReviewPacket {
        self.packet
    }

    /// Inert plan correlated with original writes and current observations.
    #[must_use]
    pub fn candidate(&self) -> &CandidateTreePlan {
        self.candidate
    }

    /// Historical native creation record from this same original owner.
    #[must_use]
    pub fn creation(&self) -> &LinuxCodingGitCreation {
        self.creation
    }

    /// Complete original test history, including failures and sealed inputs.
    #[must_use]
    pub fn validation(&self) -> &LinuxCodingValidationEvidence<'_> {
        &self.validation
    }

    /// Complete packet source pairs, including files outside the selected group.
    /// Captured locally; the checker needs separate authority for any disclosure.
    #[must_use]
    pub fn sources(&self) -> &[LinuxCodingReviewSource] {
        self.sources
    }

    /// Passive view of the caller's original cancellation scope. Check during
    /// bounded reviewer work and pass to existing confined worker poll loops.
    /// Observation failure must stop review as unavailable. This view cannot
    /// cancel the caller, create a token, or authorize a worker or disclosure.
    #[must_use]
    pub fn cancellation(&self) -> &dyn EffectCancellationObservation {
        self.cancellation
    }
}

/// Content-free outcome from an explicitly installed trusted reviewer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LinuxCodingCandidateReviewFailure {
    /// Independent review did not accept the exact complete candidate.
    Rejected,
    /// Review or trusted artifact evidence could not be obtained.
    Unavailable,
}

/// Explicit trusted opt-in at the original-owner review boundary.
///
/// The host must select an independent checker, outside model/tool input. It
/// must check the full packet, exact diff and acceptance criteria, original
/// validation history, completeness of findings and trusted provenance/scanning
/// of EVERY artifact claim. Missing evidence must refuse. A matching hash,
/// passing tests, empty findings or a caller's scanner flag cannot accept.
/// Source pairs are untrusted local input. Sending them to a model or another
/// service requires separate disclosure authority; read selection is insufficient.
/// There is no default accepting implementation or automatic reviewer selection.
/// This interface does not authenticate the installed implementation itself.
pub trait LinuxCodingCandidateReviewPort {
    /// Check original borrowed records without reentering their owner. Prior
    /// read consent must cover both complete roots and offline Git observations.
    /// This grants no extra reads, commands, disclosure or effect permission.
    /// Observe `evidence.cancellation()` during work; observation failure must
    /// stop as unavailable. The host remains responsible for worker authority,
    /// cleanup and deadlines. This synchronous call is not forcibly preempted.
    fn verify_candidate(
        &mut self,
        evidence: LinuxCodingCandidateReviewEvidence<'_>,
    ) -> std::result::Result<(), LinuxCodingCandidateReviewFailure>;
}

/// Process-only result of the installed checker and matching before/after plans.
///
/// No constructor, cloning, deserialization or restoration from packet hashes.
/// Borrows the original owner and packet, preventing mutation through those
/// references. External drift remains possible after return. This is evidence
/// of this check only, NOT execution/commit approval or an atomic state lease.
/// No candidate builder or commit executor consumes this type yet.
pub struct LinuxCodingCheckedCandidate<'a> {
    _owner: &'a LinuxCodingRepository,
    packet: &'a LocalReviewPacket,
    candidate: CandidateTreePlan,
}

impl LinuxCodingCheckedCandidate<'_> {
    /// Exact immutable packet that the installed checker received.
    #[must_use]
    pub fn packet(&self) -> &LocalReviewPacket {
        self.packet
    }

    /// Inert candidate plan from the matching post-check observation.
    #[must_use]
    pub fn candidate(&self) -> &CandidateTreePlan {
        &self.candidate
    }
}

impl<'owner> LinuxCodingCommitReview<'owner> {
    /// Invoke an explicitly installed checker on original evidence, then repeat
    /// the complete live-owner plan checks before returning its inert result.
    /// Cancellation checkpoints cannot interrupt a blocking checker or an
    /// individual filesystem operation. A checker failure stops immediately;
    /// success cannot skip the post-check or authorize any subsequent effect.
    pub fn verify_candidate_tree<'a>(
        &self,
        packet: &'a LocalReviewPacket,
        group_id: &str,
        candidate_build_id: &str,
        reviewer: &mut impl LinuxCodingCandidateReviewPort,
        cancellation: &CancellationToken,
    ) -> Result<LinuxCodingCheckedCandidate<'a>>
    where
        'owner: 'a,
    {
        self.verify_candidate_tree_with(
            packet,
            group_id,
            candidate_build_id,
            reviewer,
            cancellation,
            || self.owner.observe_commit_git(packet, Some(cancellation)),
        )
    }

    // Observation substitution is private and available only to this module's
    // synthetic tests. Public callers cannot replace native Git collection.
    fn verify_candidate_tree_with<'a>(
        &self,
        packet: &'a LocalReviewPacket,
        group_id: &str,
        candidate_build_id: &str,
        reviewer: &mut impl LinuxCodingCandidateReviewPort,
        cancellation: &CancellationToken,
        mut observe: impl FnMut() -> Result<GitObservation>,
    ) -> Result<LinuxCodingCheckedCandidate<'a>>
    where
        'owner: 'a,
    {
        let check_cancelled = || {
            if cancellation.is_cancelled() {
                Err(err("code.commit.review-cancelled"))
            } else {
                Ok(())
            }
        };
        check_cancelled()?;
        // Candidate plans omit some inventory/link observations. Compare the
        // complete private observation across ALL pre/post samples as well;
        // equal plan hashes alone cannot hide drift across the checker call.
        let baseline = RefCell::new(None);
        let mut stable_observe = || {
            check_cancelled()?;
            let observed = observe()?;
            check_cancelled()?;
            let mut baseline = baseline.borrow_mut();
            if baseline.as_ref().is_some_and(|first| first != &observed) {
                return Err(err("code.commit.review-drift"));
            }
            baseline.get_or_insert_with(|| observed.clone());
            Ok(observed)
        };
        let before = self.owner.plan_reviewed_candidate_tree_with(
            packet,
            group_id,
            candidate_build_id,
            &mut stable_observe,
        )?;
        check_cancelled()?;
        let sources = self.owner.capture_candidate_review_sources(
            packet,
            baseline
                .borrow()
                .as_ref()
                .ok_or(err("code.commit.review-source"))?,
        )?;
        check_cancelled()?;
        let outcome = reviewer.verify_candidate(LinuxCodingCandidateReviewEvidence {
            packet,
            candidate: &before,
            creation: self.owner.created_git_identity()?,
            validation: self.owner.completion_validation_evidence()?,
            sources: &sources,
            cancellation,
        });
        outcome.map_err(|failure| {
            err(match failure {
                LinuxCodingCandidateReviewFailure::Rejected => "code.commit.review-rejected",
                LinuxCodingCandidateReviewFailure::Unavailable => "code.commit.review-unavailable",
            })
        })?;
        check_cancelled()?;
        let after = self.owner.plan_reviewed_candidate_tree_with(
            packet,
            group_id,
            candidate_build_id,
            &mut stable_observe,
        )?;
        check_cancelled()?;
        if before != after {
            return Err(err("code.commit.review-drift"));
        }
        Ok(LinuxCodingCheckedCandidate {
            _owner: self.owner,
            packet,
            candidate: after,
        })
    }

    /// Reads exact approved source history under the selected held roots.
    /// Purpose labels carry no approval or independent-review authority.
    pub fn capture_applied_write_review(
        &self,
        purposes: Vec<ChangePurposeAssignment>,
    ) -> Result<ReviewableChangeSet> {
        self.owner.capture_applied_write_review(purposes)
    }

    /// Rechecks original writes, validation, source and offline Git observations
    /// before returning an inert candidate plan. Caller findings and artifact
    /// metadata remain unauthenticated; the plan grants no execution authority.
    pub fn plan_candidate_tree(
        &self,
        packet: &LocalReviewPacket,
        group_id: &str,
        candidate_build_id: &str,
    ) -> Result<CandidateTreePlan> {
        self.owner
            .plan_reviewed_candidate_tree(packet, group_id, candidate_build_id)
    }
}

impl LinuxCodingRepository {
    fn capture_candidate_review_sources(
        &self,
        packet: &LocalReviewPacket,
        observed: &GitObservation,
    ) -> Result<Vec<LinuxCodingReviewSource>> {
        let created = self.verify_apply_roots()?;
        let files = packet.change_set.files();
        let paths = files.iter().map(|f| f.path.clone()).collect::<Vec<_>>();
        let scope = self.select_created_sources(&paths)?;
        let current = scope.read(&paths).map_err(|e| err(e.code()))?;
        if current.len() != files.len() || observed.base_preimages.len() != files.len() {
            return Err(err("code.commit.review-source"));
        }
        let mut sources = Vec::with_capacity(files.len());
        for (file, source) in files.iter().zip(current) {
            if file.path != source.path || hash(source.content.as_bytes()) != file.postimage_sha256
            {
                return Err(err("code.commit.review-source"));
            }
            let edit = created
                .edits
                .get(&file.path)
                .ok_or(err("code.commit.review-source"))?;
            let (preimage, mode) = match observed.base_preimages.get(&file.path) {
                Some(Some(base))
                    if hash(&base.bytes) == file.preimage_sha256
                        && base.sha256 == file.preimage_sha256 =>
                {
                    (Some(base.bytes.clone()), base.mode)
                }
                Some(None) if file.preimage_sha256 == "0".repeat(64) => {
                    let mode = edit.created_mode.ok_or(err("code.commit.review-source"))?;
                    (
                        None,
                        if mode & 0o100 == 0 {
                            CommitFileMode::Regular
                        } else {
                            CommitFileMode::Executable
                        },
                    )
                }
                _ => return Err(err("code.commit.review-source")),
            };
            sources.push(LinuxCodingReviewSource {
                operation_id: file.operation_id.clone(),
                path: file.path.clone(),
                preimage,
                postimage: source.content.into_bytes(),
                mode,
            });
        }
        self.verify_apply_roots()?;
        Ok(sources)
    }

    /// Binds trusted-caller consent to the exact original and worktree roots
    /// displayed by `inspect_apply`, before source reads or Git inspection.
    /// Consent must cover complete source/metadata reads and offline Git reads;
    /// path digests correlate that selection and do not themselves prove consent.
    ///
    /// Mismatches refuse before held-root inspection. Matching roots still need
    /// revalidation, including on every use of the returned view. The borrow
    /// prevents replacing the owner or performing effects through it while the
    /// view is live. External changes remain possible and are checked by the
    /// existing source and Git observations, not an atomic filesystem lease.
    pub fn consented_commit_review(
        &self,
        expected_original_root_sha256: &str,
        expected_worktree_root_sha256: &str,
    ) -> Result<LinuxCodingCommitReview<'_>> {
        let created = self
            .created
            .as_ref()
            .ok_or(err("code.command.worktree-required"))?;
        if expected_original_root_sha256 != created.original.root_path_sha256()
            || expected_worktree_root_sha256 != created.held.root_path_sha256()
        {
            return Err(err("code.commit.read-scope"));
        }
        self.verify_apply_roots()?;
        Ok(LinuxCodingCommitReview { owner: self })
    }

    /// Projects the original native write's approval and checked application
    /// receipt into an authority-free review change set. Call only after source
    /// read consent. Purposes are caller review labels, never approval evidence.
    ///
    /// Retained original native transactions must cover the complete current
    /// edit or absent-before creation lineage through final postimage. Histories
    /// retain each approval separately and use path-ordered `history-N` purpose
    /// identities. Incomplete chains refuse. No receipt/path/JSON
    /// can restore this process-only proof.
    pub(super) fn capture_applied_write_review(
        &self,
        purposes: Vec<ChangePurposeAssignment>,
    ) -> Result<ReviewableChangeSet> {
        let created = self.verify_apply_roots()?;
        if self.creation_review_incomplete {
            return Err(err("code.commit.creation-provenance"));
        }
        let proof = self
            .applied_write
            .as_ref()
            .ok_or(err("code.commit.write-provenance"))?;
        if proof.root_sha256() != created.held.root_path_sha256() {
            return Err(err("code.commit.write-provenance"));
        }
        let projected = proof
            .project(purposes)
            .map_err(|_| err("code.commit.write-provenance"))?;
        let files = projected.files();
        if files.is_empty()
            || files.len() != created.edits.len()
            || files.iter().any(|f| {
                created.edits.get(&f.path).is_none_or(|edit| {
                    edit.before_sha256 != f.preimage_sha256
                        || edit.after_sha256 != f.postimage_sha256
                })
            })
        {
            return Err(err("code.commit.write-provenance"));
        }
        let paths = files.iter().map(|f| f.path.clone()).collect::<Vec<_>>();
        let scope = self.select_created_sources(&paths)?;
        let sources = scope.read(&paths).map_err(|e| err(e.code()))?;
        if files.iter().zip(sources).any(|(file, source)| {
            file.path != source.path || file.postimage_sha256 != hash(source.content.as_bytes())
        }) {
            return Err(err("code.commit.write-provenance"));
        }
        // Creation mode was part of the original approval. A later metadata
        // change cannot silently replace it, even when the bytes still match.
        for file in files {
            if let Some(mode) = created.edits[&file.path].created_mode {
                let path =
                    WorkspacePath::new(created.held.workspace_id.clone(), file.path.split('/'))
                        .map_err(|_| err("code.commit.mode"))?;
                let target =
                    LinuxPathAdapter::new(created.held.adapter_instance_id.clone(), 128 * 1024)
                        .resolve(&created.held, &path, PathResolutionIntent::Metadata)
                        .map_err(|_| err("code.commit.mode"))?;
                if file.preimage_sha256 != "0".repeat(64)
                    || target.object_snapshot.mode & 0o7777 != mode
                {
                    return Err(err("code.commit.write-provenance"));
                }
            } else if file.preimage_sha256 == "0".repeat(64) {
                return Err(err("code.commit.write-provenance"));
            }
        }
        self.verify_apply_roots()?;
        Ok(projected)
    }

    /// Checks a complete typed review packet against this original live owner
    /// and returns the existing kernel's inert candidate-tree plan.
    ///
    /// Call only after explicit consent to inspect the complete original and
    /// created worktree, including Git metadata, and to run offline Git reads.
    /// Reads use the existing trusted collector and confined linked inventory.
    /// There is no HTTP/terminal entry point for this internal composition yet.
    ///
    /// A packet's hashes do not establish ownership or independent review. This
    /// method checks current content/lineage, immutable base preimages, original
    /// held validation evidence and fresh Git observations. It does not authenticate caller-supplied
    /// review findings or evidence artifacts. Write-approval and application
    /// identities must match the retained original native transactions exactly.
    /// The returned plan grants no candidate build, commit or publication. Any
    /// future executor still needs complete provenance and separate authority.
    /// Source and Git checks are bounded observations, not an atomic lease.
    fn plan_reviewed_candidate_tree(
        &self,
        packet: &LocalReviewPacket,
        group_id: &str,
        candidate_build_id: &str,
    ) -> Result<CandidateTreePlan> {
        self.plan_reviewed_candidate_tree_with(packet, group_id, candidate_build_id, || {
            self.observe_commit_git(packet, None)
        })
    }

    fn observe_commit_git(
        &self,
        packet: &LocalReviewPacket,
        cancellation: Option<&CancellationToken>,
    ) -> Result<GitObservation> {
        if cancellation.is_some_and(CancellationToken::is_cancelled) {
            return Err(err("code.commit.review-cancelled"));
        }
        let created = self.verify_apply_roots()?;
        let scope = LinuxRepositoryScope::verify(
            &created.original_path,
            created.original_path.join(".git"),
            created.path.parent().ok_or(err("code.command.scope"))?,
        )
        .map_err(|e| err(e.kind().code()))?;
        let linked = crate::LinuxLinkedRepository::verify_owned_worktree(
            &created.path,
            scope.git_directory(),
        )
        .map_err(|e| err(e.kind().code()))?;
        let task_scope = LinuxRepositoryScope::verify_linked_read_only(&linked, &self.state_root)
            .map_err(|e| err(e.kind().code()))?;
        let collector = LinuxRepositoryCollector::new(
            LinuxGitArtifact::verify("/usr/bin/git").map_err(|e| err(e.kind().code()))?,
        );
        let original = match cancellation {
            Some(token) => collector.collect_cancellable(&scope, token),
            None => collector.collect(&scope),
        }
        .map_err(|e| err(e.kind().code()))?;
        let task = match cancellation {
            Some(token) => collector.collect_inventory_cancellable(&task_scope, token),
            None => collector.collect_inventory(&task_scope),
        }
        .map_err(|e| err(e.kind().code()))?;
        let paths = packet
            .change_set
            .files()
            .iter()
            .map(|f| f.path.clone())
            .collect::<Vec<_>>();
        let base_preimages = collector
            .collect_base_preimages(
                &task_scope,
                self.created_git_identity()?.base_object(),
                &paths,
                cancellation,
            )
            .map_err(|_| err("code.commit.base-preimage"))?;
        linked.revalidate().map_err(|e| err(e.kind().code()))?;
        if cancellation.is_some_and(CancellationToken::is_cancelled) {
            return Err(err("code.commit.review-cancelled"));
        }
        if task.repository_sha256 != created.held.root_path_sha256() {
            return Err(err("code.commit.git-state"));
        }
        Ok(GitObservation {
            original,
            task_head: task.commit_id,
            task_branch: task.branch,
            linked_identity: linked.identity_sha256().into(),
            task_inventory_sha256: task.git_evidence_sha256,
            base_preimages,
        })
    }

    // Only this module's tests can substitute Git observations. Product callers
    // supply neither an executor, a path, a receipt nor a manifest to this path.
    fn plan_reviewed_candidate_tree_with(
        &self,
        packet: &LocalReviewPacket,
        group_id: &str,
        candidate_build_id: &str,
        mut observe: impl FnMut() -> Result<GitObservation>,
    ) -> Result<CandidateTreePlan> {
        verify_local_commit_review(packet).map_err(|_| err("code.commit.review-packet"))?;
        let identity = self.created_git_identity()?;
        let purposes = packet
            .change_set
            .files()
            .iter()
            .map(|file| ChangePurposeAssignment {
                operation_id: file.operation_id.clone(),
                purpose: file.purpose,
            })
            .collect();
        if self.capture_applied_write_review(purposes)? != packet.change_set {
            return Err(err("code.commit.write-provenance"));
        }
        // Reject pending effects and stale/nonsealed validation before any Git
        // child or repository-wide preservation read.
        self.check_completion_validation()?;
        let created = self.verify_apply_roots()?;
        let source = WorkspaceObservation::capture(&created.held)
            .ok_or(err("code.commit.source-binding"))?;
        let evidence = self.completion_validation_evidence()?;
        let final_attempt = evidence
            .attempts()
            .last()
            .ok_or(err("code.commit.validation"))?;
        let validation = bind_validation_evidence(
            evidence.template(),
            &final_attempt.prepared,
            &final_attempt.command,
            &final_attempt.validation,
        )
        .map_err(|_| err("code.commit.validation"))?;
        if packet.repository_sha256 != created.original.root_path_sha256()
            || packet.base_object != identity.base_object()
            || packet.candidate_snapshot_sha256 != source.digest
            || packet.validations != [validation]
        {
            return Err(err("code.commit.review-binding"));
        }
        let files = packet.change_set.files();
        if files.len() != created.edits.len() || files.is_empty() {
            return Err(err("code.commit.edit-lineage"));
        }
        let paths = files
            .iter()
            .map(|file| file.path.clone())
            .collect::<Vec<_>>();
        let scope = self.select_created_sources(&paths)?;
        let sources = scope.read(&paths).map_err(|e| err(e.code()))?;
        let mut modes = Vec::with_capacity(files.len());
        for (file, source) in files.iter().zip(&sources) {
            let edit = created
                .edits
                .get(&file.path)
                .ok_or(err("code.commit.edit-lineage"))?;
            if file.path != source.path
                || file.preimage_sha256 != edit.before_sha256
                || file.postimage_sha256 != edit.after_sha256
                || hash(source.content.as_bytes()) != edit.after_sha256
                || edit.created_mode.is_some_and(|mode| {
                    file.preimage_sha256 != "0".repeat(64) || mode & 0o7000 != 0
                })
            {
                return Err(err("code.commit.edit-lineage"));
            }
            let path = WorkspacePath::new(created.held.workspace_id.clone(), file.path.split('/'))
                .map_err(|_| err("code.commit.mode"))?;
            let target =
                LinuxPathAdapter::new(created.held.adapter_instance_id.clone(), 128 * 1024)
                    .resolve(&created.held, &path, PathResolutionIntent::Metadata)
                    .map_err(|_| err("code.commit.mode"))?;
            let permissions = target.object_snapshot.mode & 0o7777;
            if permissions & 0o7000 != 0 {
                return Err(err("code.commit.mode"));
            }
            if edit.created_mode.is_some_and(|mode| mode != permissions) {
                return Err(err("code.commit.mode"));
            }
            // Git records the owner's executable bit, not private host read or
            // write permissions. Owner-only 0600/0700 sources remain usable.
            let mode = if permissions & 0o100 == 0 {
                CommitFileMode::Regular
            } else {
                CommitFileMode::Executable
            };
            modes.push((file.operation_id.clone(), mode));
        }
        let before = observe()?;
        before
            .original
            .verify()
            .map_err(|_| err("code.commit.git-state"))?;
        if before.original.repository_sha256 != packet.repository_sha256
            || before.original.manifest_sha256 != packet.preservation_manifest_sha256
            || !before.original.safe_ownership
            || before.original.shallow
            || before.original.partial
            || before.original.hazardous_configuration
            || before.task_branch.as_deref() != Some(identity.branch_ref())
            || before.task_head != identity.base_object()
        {
            return Err(err("code.commit.git-state"));
        }
        // The first approved filesystem preimage may include dirty or untracked
        // starting content. It is not evidence that the same bytes were in the
        // immutable creation base. Never silently commit that extra content.
        // Existing-file edit approvals do not authorize chmod: require the
        // candidate's Git mode to match that base too. New files instead retain
        // their separately approved exact creation mode, checked above.
        if before.base_preimages.len() != files.len()
            || files
                .iter()
                .any(|file| match before.base_preimages.get(&file.path) {
                    Some(None) => file.preimage_sha256 != "0".repeat(64),
                    Some(Some(base)) => {
                        file.preimage_sha256 == "0".repeat(64)
                            || file.preimage_sha256 != base.sha256
                            || modes
                                .iter()
                                .find(|(id, _)| id == &file.operation_id)
                                .is_none_or(|(_, mode)| *mode != base.mode)
                    }
                    None => true,
                })
        {
            return Err(err("code.commit.base-preimage"));
        }
        // A group may select a subset of the full packet. Do not supply modes
        // for operations outside that exact group to the kernel planner.
        let group = packet
            .commit_plan
            .groups
            .iter()
            .find(|g| g.group_id == group_id)
            .ok_or(err("code.commit.group"))?;
        modes.retain(|(id, _)| group.operation_ids.contains(id));
        let repository_scope = LinuxRepositoryScope::verify(
            &created.original_path,
            created.original_path.join(".git"),
            created.path.parent().ok_or(err("code.command.scope"))?,
        )
        .map_err(|e| err(e.kind().code()))?;
        let temporary_index = crate::local_commit::linux_candidate_index_path_sha256(
            &repository_scope,
            candidate_build_id,
        )
        .map_err(|_| err("code.commit.candidate-index"))?;
        let plan = plan_candidate_tree(
            candidate_build_id,
            packet,
            group_id,
            &temporary_index,
            &before.original.index_sha256,
            &modes,
        )
        .map_err(|e| err(e.code()))?;
        // Do not release even an inert plan after a changed observation. An
        // error never means there were no external effects during these reads.
        if observe()? != before {
            return Err(err("code.commit.git-drift"));
        }
        self.created_git_identity()?;
        self.check_completion_validation()?;
        if WorkspaceObservation::capture(&self.verify_apply_roots()?.held)
            .is_none_or(|now| now != source)
        {
            return Err(err("code.commit.source-binding"));
        }
        Ok(plan)
    }
}

#[cfg(test)]
#[path = "coding_repository_commit_review_tests.rs"]
mod tests;
