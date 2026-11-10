// SPDX-License-Identifier: Apache-2.0
//! One complete candidate-build budget; native admission stays mandatory.

use super::*;
use agentmage_kernel_engine::repository_safety::RepositoryPreservationManifest;

const HASH_CHUNK: usize = 64 * 1024;

// Private sequencing seam. Public callers cannot install a collector, clock or
// worker implementation. Production always uses the concrete native owner.
trait Session {
    fn check(&self) -> Result<(), LinuxLocalCommitError>;
    fn collect(&mut self, deadline: Instant) -> Result<RepositoryPreservationManifest, ()>;
    fn start(&mut self, plan: &CandidateTreePlan, deadline: Instant) -> Result<(), ()>;
    fn read_tree(&mut self) -> Result<(), ()>;
    fn add(&mut self, operation: &str, bytes: &[u8]) -> Result<String, ()>;
    fn write_tree(&mut self) -> Result<String, ()>;
    fn cleanup(&mut self) -> bool;
}

struct NativeSession<'a> {
    builder: &'a LinuxCandidateTreeBuilder,
    cancellation: &'a CancellationToken,
    deadline: Instant,
    worker: Option<crate::repository_safety::CandidateSession>,
}

fn check_at(
    cancellation: &CancellationToken,
    deadline: Instant,
    now: Instant,
) -> Result<(), LinuxLocalCommitError> {
    if cancellation.is_cancelled() || now >= deadline {
        Err(error(LinuxLocalCommitErrorKind::ExecutionFailed))
    } else {
        Ok(())
    }
}

impl Session for NativeSession<'_> {
    fn check(&self) -> Result<(), LinuxLocalCommitError> {
        check_at(self.cancellation, self.deadline, Instant::now())
    }

    fn collect(&mut self, deadline: Instant) -> Result<RepositoryPreservationManifest, ()> {
        self.builder
            .collector
            .collect_until(&self.builder.scope, Some(self.cancellation), deadline)
            .map_err(|_| ())
    }

    fn start(&mut self, plan: &CandidateTreePlan, deadline: Instant) -> Result<(), ()> {
        let path = linux_candidate_index_path_sha256(&self.builder.scope, &plan.candidate_build_id)
            .map_err(|_| ())?;
        if plan.temporary_index_path_sha256 != path {
            return Err(());
        }
        self.worker = Some(
            crate::repository_safety::CandidateSession::new_until(
                &self.builder.scope,
                plan,
                self.cancellation,
                deadline,
            )
            .map_err(|_| ())?,
        );
        Ok(())
    }

    fn read_tree(&mut self) -> Result<(), ()> {
        self.worker
            .as_mut()
            .ok_or(())?
            .read_tree(self.builder.collector.git(), self.cancellation)
            .map_err(|_| ())
    }

    fn add(&mut self, operation: &str, bytes: &[u8]) -> Result<String, ()> {
        self.worker
            .as_mut()
            .ok_or(())?
            .add_postimage(
                self.builder.collector.git(),
                operation,
                bytes,
                self.cancellation,
            )
            .map_err(|_| ())
    }

    fn write_tree(&mut self) -> Result<String, ()> {
        self.worker
            .as_mut()
            .ok_or(())?
            .write_tree(self.builder.collector.git(), self.cancellation)
            .map_err(|_| ())
    }

    fn cleanup(&mut self) -> bool {
        self.worker.as_mut().is_some_and(|worker| worker.cleanup())
    }
}

pub(super) fn run(
    builder: &LinuxCandidateTreeBuilder,
    plan: &CandidateTreePlan,
    postimages: Vec<LinuxApprovedPostimage>,
    cancellation: &CancellationToken,
    deadline: Instant,
) -> Result<CandidateTreePlatformResult, LinuxLocalCommitError> {
    run_with(
        plan,
        postimages,
        deadline,
        &mut NativeSession {
            builder,
            cancellation,
            deadline,
            worker: None,
        },
    )
}

fn checked_hash(bytes: &[u8], session: &impl Session) -> Result<String, LinuxLocalCommitError> {
    session.check()?;
    let mut digest = Sha256::new();
    for chunk in bytes.chunks(HASH_CHUNK) {
        session.check()?;
        digest.update(chunk);
    }
    session.check()?;
    Ok(hex_digest(digest.finalize()))
}

fn run_with(
    plan: &CandidateTreePlan,
    postimages: Vec<LinuxApprovedPostimage>,
    deadline: Instant,
    session: &mut impl Session,
) -> Result<CandidateTreePlatformResult, LinuxLocalCommitError> {
    session.check()?;
    verify_candidate_tree_plan(plan).map_err(|_| error(LinuxLocalCommitErrorKind::PlanDenied))?;
    session.check()?;
    let before = session
        .collect(deadline)
        .map_err(|_| error(LinuxLocalCommitErrorKind::PlanDenied))?;
    session.check()?;
    if before.manifest_sha256 != plan.preservation_manifest_sha256
        || before.repository_sha256 != plan.repository_sha256
        || before.index_sha256 != plan.user_index_sha256
    {
        return Err(error(LinuxLocalCommitErrorKind::PlanDenied));
    }
    let count = postimages.len();
    // The sealed plan already bounds file count. Refuse an excessive caller
    // vector before building a map or walking its bytes under this budget.
    if count != plan.files.len() {
        return Err(error(LinuxLocalCommitErrorKind::PlanDenied));
    }
    let postimages = postimages
        .into_iter()
        .map(|p| (p.operation_id, p.bytes))
        .collect::<BTreeMap<_, _>>();
    if count != postimages.len()
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
        if checked_hash(bytes, session)? != file.postimage_sha256 {
            return Err(error(LinuxLocalCommitErrorKind::PlanDenied));
        }
    }
    session.check()?;
    session
        .start(plan, deadline)
        .map_err(|_| error(LinuxLocalCommitErrorKind::ScopeDenied))?;
    let operation: Result<_, LinuxLocalCommitError> = (|| {
        session.check()?;
        session
            .read_tree()
            .map_err(|_| error(LinuxLocalCommitErrorKind::ExecutionFailed))?;
        session.check()?;
        let mut blobs = Vec::with_capacity(plan.files.len());
        for file in &plan.files {
            let blob = session
                .add(&file.operation_id, &postimages[&file.operation_id])
                .map_err(|_| error(LinuxLocalCommitErrorKind::ExecutionFailed))?;
            session.check()?;
            blobs.push(CandidateTreeBlob {
                operation_id: file.operation_id.clone(),
                postimage_sha256: file.postimage_sha256.clone(),
                blob_object: blob,
            });
        }
        blobs.sort();
        let tree = session
            .write_tree()
            .map_err(|_| error(LinuxLocalCommitErrorKind::ExecutionFailed))?;
        session.check()?;
        Ok((tree, blobs))
    })();
    // Cleanup never inherits a refusal to run: known scratch must still be
    // reconciled after expiry/cancellation. Unknown/replaced scratch is retained.
    let cleanup_verified = session.cleanup();
    session.check()?;
    let after = session
        .collect(deadline)
        .map_err(|_| error(LinuxLocalCommitErrorKind::ExecutionFailed))?;
    session.check()?;
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

#[cfg(test)]
#[path = "local_commit_candidate_build_tests.rs"]
mod tests;
