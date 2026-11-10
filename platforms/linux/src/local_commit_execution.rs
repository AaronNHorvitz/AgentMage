// SPDX-License-Identifier: Apache-2.0
//! Private commit sequencing seam; synthetic backends exist only in tests.

use super::*;
use agentmage_kernel_engine::local_commit::LocalCommitPlan;
use agentmage_kernel_engine::repository_safety::RepositoryPreservationManifest;

// Neither the trait nor its runner is reachable outside the local-commit module.
// Public callers cannot supply a backend, manufacture a permit or skip preflight.
trait Session {
    fn branch_before(&mut self) -> Result<(), ()>;
    fn sign(&mut self) -> Result<packet::VerifiedCommit, LinuxLocalCommitError>;
    fn write(&mut self, signed: &packet::VerifiedCommit) -> Result<String, ()>;
    fn read(&mut self, object: &str) -> Result<Vec<u8>, ()>;
    fn verify(&mut self, raw: &[u8]) -> Result<(), LinuxLocalCommitError>;
    fn update(&mut self) -> Result<(), ()>;
    fn postflight(&mut self) -> Result<RepositoryPreservationManifest, ()>;
}

pub(super) struct NativeSession<'a> {
    executor: &'a LinuxLocalCommitExecutor,
    io: &'a mut crate::repository_safety::CommitIoSession,
    plan: &'a LocalCommitPlan,
    cancellation: &'a CancellationToken,
}

impl<'a> NativeSession<'a> {
    pub(super) fn new(
        executor: &'a LinuxLocalCommitExecutor,
        io: &'a mut crate::repository_safety::CommitIoSession,
        plan: &'a LocalCommitPlan,
        cancellation: &'a CancellationToken,
    ) -> Self {
        Self {
            executor,
            io,
            plan,
            cancellation,
        }
    }
}

impl Session for NativeSession<'_> {
    fn branch_before(&mut self) -> Result<(), ()> {
        self.io
            .branch_before(self.executor.collector.git(), self.cancellation)
            .map_err(|_| ())
    }
    fn sign(&mut self) -> Result<packet::VerifiedCommit, LinuxLocalCommitError> {
        packet::sign_commit(
            &self.executor.signer,
            self.plan,
            self.cancellation,
            self.io.deadline(),
        )
    }
    fn write(&mut self, signed: &packet::VerifiedCommit) -> Result<String, ()> {
        self.io
            .write_commit(self.executor.collector.git(), signed, self.cancellation)
            .map_err(|_| ())
    }
    fn read(&mut self, object: &str) -> Result<Vec<u8>, ()> {
        self.io
            .read_commit(self.executor.collector.git(), object, self.cancellation)
            .map_err(|_| ())
    }
    fn verify(&mut self, raw: &[u8]) -> Result<(), LinuxLocalCommitError> {
        verification::verify(
            &self.executor.signer,
            raw,
            self.plan,
            self.cancellation,
            self.io.deadline(),
        )
    }
    fn update(&mut self) -> Result<(), ()> {
        self.io
            .update_task_branch(self.executor.collector.git(), self.cancellation)
            .map_err(|_| ())
    }
    fn postflight(&mut self) -> Result<RepositoryPreservationManifest, ()> {
        self.executor
            .collector
            .collect(&self.executor.scope)
            .map_err(|_| ())
    }
}

// Production accepts only the concrete native owner. Tests invoke the same
// private generic body, without creating native admission or approval evidence.
pub(super) fn run(
    plan: &LocalCommitPlan,
    before: RepositoryPreservationManifest,
    session: &mut NativeSession<'_>,
) -> LocalCommitPlatformResult {
    run_with(plan, before, session)
}

fn run_with(
    plan: &LocalCommitPlan,
    before: RepositoryPreservationManifest,
    session: &mut impl Session,
) -> LocalCommitPlatformResult {
    if session.branch_before().is_err() {
        return uncertain_commit_result(&before, "linux.git.commit.branch_stale");
    }
    let signed = match session.sign() {
        Ok(commit) => commit,
        Err(failure) => {
            return failed_signer_result(&before, failure, "linux.git.commit.creation_failed");
        }
    };
    let commit_object = match session.write(&signed) {
        Ok(object) => object,
        Err(()) => return uncertain_commit_result(&before, "linux.git.commit.storage_failed"),
    };
    let Ok(raw) = session.read(&commit_object) else {
        return uncertain_commit_result(&before, "linux.git.commit.verification_failed");
    };
    if !verify_raw_commit(&raw, plan) {
        return uncertain_commit_result(&before, "linux.git.commit.verification_failed");
    }
    if let Err(failure) = session.verify(&raw) {
        return failed_signer_result(&before, failure, "linux.git.commit.verification_failed");
    }
    if session.update().is_err() {
        return uncertain_commit_result(&before, "linux.git.commit.compare_and_swap_failed");
    }
    let after = match session.postflight() {
        Ok(value) => value,
        Err(()) => return uncertain_commit_result(&before, "linux.git.commit.postflight"),
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

#[cfg(test)]
#[path = "local_commit_execution_tests.rs"]
mod tests;
