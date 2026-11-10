// SPDX-License-Identifier: Apache-2.0
//! Live-owner checks before constructing an authority-free candidate-tree plan.

use super::*;
use crate::command_runner::worktree::WorkspaceObservation;
use agentmage_kernel_engine::local_commit::{
    CandidateTreePlan, CommitFileMode, plan_candidate_tree,
};
use agentmage_kernel_engine::repository_safety::RepositoryPreservationManifest;
use agentmage_kernel_engine::review_packet::{
    ChangePurposeAssignment, FindingSeverity, LocalReviewPacket, ReviewableChangeSet,
    bind_validation_evidence, verify_review_packet,
};

#[derive(PartialEq, Eq)]
struct GitObservation {
    original: RepositoryPreservationManifest,
    task_head: String,
    task_branch: Option<String>,
    linked_identity: String,
    task_inventory_sha256: String,
}

impl LinuxCodingRepository {
    /// Projects the original native write's approval and checked application
    /// receipt into an authority-free review change set. Call only after source
    /// read consent. Purposes are caller review labels, never approval evidence.
    ///
    /// The retained single native transaction must cover the complete current
    /// existing-file lineage from its first preimage to its final postimage.
    /// Multiple transactions with incomplete cumulative coverage and new files
    /// refuse. No receipt/path/JSON can restore this process-only proof.
    pub fn capture_applied_write_review(
        &self,
        purposes: Vec<ChangePurposeAssignment>,
    ) -> Result<ReviewableChangeSet> {
        let created = self.verify_apply_roots()?;
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
                    edit.created_mode.is_some()
                        || edit.before_sha256 != f.preimage_sha256
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
    /// method checks current content/lineage, original held validation evidence
    /// and fresh Git observations. It does not authenticate caller-supplied
    /// review findings or evidence artifacts. Write-approval and application
    /// identities must match the retained original native transaction exactly.
    /// The returned plan grants no candidate build, commit or publication. Any
    /// future executor still needs complete provenance and separate authority.
    /// Source and Git checks are bounded observations, not an atomic lease.
    pub fn plan_reviewed_candidate_tree(
        &self,
        packet: &LocalReviewPacket,
        group_id: &str,
        candidate_build_id: &str,
    ) -> Result<CandidateTreePlan> {
        self.plan_reviewed_candidate_tree_with(packet, group_id, candidate_build_id, || {
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
            let task_scope =
                LinuxRepositoryScope::verify_linked_read_only(&linked, &self.state_root)
                    .map_err(|e| err(e.kind().code()))?;
            let collector = LinuxRepositoryCollector::new(
                LinuxGitArtifact::verify("/usr/bin/git").map_err(|e| err(e.kind().code()))?,
            );
            let original = collector
                .collect(&scope)
                .map_err(|e| err(e.kind().code()))?;
            let task = collector
                .collect_inventory(&task_scope)
                .map_err(|e| err(e.kind().code()))?;
            linked.revalidate().map_err(|e| err(e.kind().code()))?;
            if task.repository_sha256 != created.held.root_path_sha256() {
                return Err(err("code.commit.git-state"));
            }
            Ok(GitObservation {
                original,
                task_head: task.commit_id,
                task_branch: task.branch,
                linked_identity: linked.identity_sha256().into(),
                task_inventory_sha256: task.git_evidence_sha256,
            })
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
        let identity = self.created_git_identity()?;
        if !verify_review_packet(packet)
            || packet.review_results.findings.iter().any(|finding| {
                matches!(
                    finding.severity,
                    FindingSeverity::Medium | FindingSeverity::High | FindingSeverity::Critical
                )
            })
        {
            return Err(err("code.commit.review-packet"));
        }
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
                // New-file review uses a separate kernel transaction family;
                // this first existing-file bridge must not invent its proof.
                || edit.created_mode.is_some()
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
