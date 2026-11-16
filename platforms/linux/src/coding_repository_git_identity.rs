// SPDX-License-Identifier: MIT OR Apache-2.0
//! Historical native Git creation identity retained by the original live owner.

use super::*;
use agentmage_kernel_contracts::{OperationOutcome, Receipt};
use agentmage_kernel_engine::repository_safety::{
    RepositoryOperation, RepositoryOperationPlan, RepositoryOperationReceipt,
};

/// Read-only historical creation provenance, obtainable only from the live owner.
/// No deserialization, path adoption, grant, current-ref observation or commit
/// approval. A future Git effect must independently recollect and verify state.
/// Synthetic test-created worktrees deliberately have no such provenance.
pub struct LinuxCodingGitCreation {
    branch_ref: String,
    base_object: String,
    plan_sha256: String,
    repository_receipt_sha256: String,
    authority_receipt_sha256: String,
}

impl LinuxCodingGitCreation {
    // Private: only the original verified dispatch can supply these observations.
    // Equality checks correlate the two receipts; they do not authenticate input
    // from an external caller. Neither receipts nor serialized inspection can
    // construct this type through the product API.
    pub(super) fn from_dispatch(
        plan: &RepositoryOperationPlan,
        authority: &Receipt,
        repository: Option<&RepositoryOperationReceipt>,
    ) -> Result<Self> {
        let repository = repository.ok_or(err("code.repository.git-provenance"))?;
        plan.verify()
            .map_err(|_| err("code.repository.git-provenance"))?;
        if plan.operation != RepositoryOperation::WorktreeCreate
            || authority.outcome != OperationOutcome::Succeeded
            || authority.operation != OperationBinding::new(GrantOperation::GitWorktreeCreate)
            || repository.outcome != OperationOutcome::Succeeded
            || repository.operation != RepositoryOperation::WorktreeCreate
            || !repository.cleanup_verified
            || authority.authority_transaction_id.as_str() != plan.transaction_id
            || repository.repository_transaction_id != plan.transaction_id
            || repository.plan_sha256 != plan.plan_sha256
            || repository.before_manifest_sha256 != plan.preservation_manifest_sha256
            || repository.authority_transaction_id != authority.authority_transaction_id.as_str()
            || repository.operation_attempt_id != authority.operation_attempt_id.as_str()
        {
            return Err(err("code.repository.git-provenance"));
        }
        Ok(Self {
            branch_ref: plan
                .branch_ref
                .clone()
                .ok_or(err("code.repository.git-provenance"))?,
            base_object: plan
                .expected_new_object
                .clone()
                .ok_or(err("code.repository.git-provenance"))?,
            plan_sha256: plan.plan_sha256.clone(),
            repository_receipt_sha256: repository.receipt_sha256.clone(),
            authority_receipt_sha256: authority.receipt_sha256.clone(),
        })
    }

    /// Exact task branch created by the verified operation, not its current ref.
    #[must_use]
    pub fn branch_ref(&self) -> &str {
        &self.branch_ref
    }

    /// Immutable object used at creation; this is not a current HEAD assertion.
    #[must_use]
    pub fn base_object(&self) -> &str {
        &self.base_object
    }

    /// Original canonical creation-plan digest.
    #[must_use]
    pub fn plan_sha256(&self) -> &str {
        &self.plan_sha256
    }

    /// Specialized native repository receipt correlated with the authority result.
    #[must_use]
    pub fn repository_receipt_sha256(&self) -> &str {
        &self.repository_receipt_sha256
    }

    /// Original canonical authority receipt, not an approval to run another effect.
    #[must_use]
    pub fn authority_receipt_sha256(&self) -> &str {
        &self.authority_receipt_sha256
    }

    pub(super) fn inspection(&self) -> Value {
        json!({"scope":"historical-worktree-creation", "branch_ref":self.branch_ref,
            "base_object":self.base_object,"plan_sha256":self.plan_sha256,
            "repository_receipt_sha256":self.repository_receipt_sha256,
            "authority_receipt_sha256":self.authority_receipt_sha256,
            "current_git_state_verified":false,"commit_authority":false,"publication_authority":false})
    }
}

impl LinuxCodingRepository {
    /// Borrows historical Git provenance after revalidating both held roots.
    /// Live runtime references must first return the same repository owner.
    /// Restart, a serialized record or a synthetic creation cannot supply it.
    /// This performs filesystem metadata checks, not Git execution or ref reads.
    pub fn created_git_identity(&self) -> Result<&LinuxCodingGitCreation> {
        let created = self.verify_apply_roots()?;
        let identity = created
            .git_creation
            .as_ref()
            .ok_or(err("code.repository.git-provenance"))?;
        if identity.authority_receipt_sha256 != created.creation_receipt_sha256 {
            return Err(err("code.repository.git-provenance"));
        }
        Ok(identity)
    }
}
