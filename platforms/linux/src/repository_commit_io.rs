// SPDX-License-Identifier: Apache-2.0
//! Closed non-signing I/O for the dormant signed local-commit adapter.

use super::*;
use agentmage_kernel_engine::local_commit::{LocalCommitPlan, verify_local_commit_plan};

/// This owner is internal plumbing, not an approval or reviewer capability.
/// The adapter retains the kernel permit, exact signature checks and final
/// preservation reconciliation. External signing is a separate boundary.
pub(crate) struct CommitIoSession {
    owner: Arc<LinuxRepositoryMutationScope>,
    plan: LocalCommitPlan,
    stage: Stage,
    deadline: Instant,
}

#[cfg(test)]
#[path = "repository_commit_io_tests.rs"]
mod tests;

enum Stage {
    Branch,
    Commit,
    Update(String),
    Terminal,
}

enum Step<'a> {
    Branch,
    Commit(&'a str),
    Update,
}

impl CommitIoSession {
    pub(crate) fn new(
        scope: &LinuxRepositoryScope,
        plan: &LocalCommitPlan,
    ) -> Result<Self, LinuxRepositoryError> {
        verify_local_commit_plan(plan).map_err(|_| denied())?;
        if plan.repository_sha256 != scope.repository_path_sha256 {
            return Err(denied());
        }
        Ok(Self {
            owner: Arc::new(LinuxRepositoryMutationScope::capture(scope)?),
            plan: plan.clone(),
            stage: Stage::Branch,
            deadline: Instant::now() + PROCESS_TIMEOUT,
        })
    }

    pub(crate) fn deadline(&self) -> Instant {
        self.deadline
    }

    pub(crate) fn branch_before(
        &mut self,
        git: &LinuxGitArtifact,
        cancellation: &CancellationToken,
    ) -> Result<(), LinuxRepositoryError> {
        self.step(git, Step::Branch, cancellation).map(|_| ())
    }

    pub(crate) fn read_commit(
        &mut self,
        git: &LinuxGitArtifact,
        object: &str,
        cancellation: &CancellationToken,
    ) -> Result<Vec<u8>, LinuxRepositoryError> {
        self.step(git, Step::Commit(object), cancellation)
    }

    // Call only after exact raw-content and pinned-signature verification. This
    // private process-only session does not restore authority from a plan/hash.
    pub(crate) fn update_task_branch(
        &mut self,
        git: &LinuxGitArtifact,
        cancellation: &CancellationToken,
    ) -> Result<(), LinuxRepositoryError> {
        self.step(git, Step::Update, cancellation).map(|_| ())
    }

    fn step(
        &mut self,
        git: &LinuxGitArtifact,
        step: Step<'_>,
        cancellation: &CancellationToken,
    ) -> Result<Vec<u8>, LinuxRepositoryError> {
        self.step_with(step, cancellation, |owner, args, env, deadline| {
            run_worker(git, owner, args, env, None, cancellation, deadline)
        })
    }

    fn step_with(
        &mut self,
        step: Step<'_>,
        cancellation: &CancellationToken,
        dispatch: impl FnOnce(
            Arc<LinuxRepositoryMutationScope>,
            &[String],
            &std::collections::BTreeMap<String, String>,
            Instant,
        ) -> Result<SupervisedResult, InvocationStatus>,
    ) -> Result<Vec<u8>, LinuxRepositoryError> {
        let result = self.perform(step, cancellation, dispatch);
        if result.is_err() {
            self.stage = Stage::Terminal;
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
            &std::collections::BTreeMap<String, String>,
            Instant,
        ) -> Result<SupervisedResult, InvocationStatus>,
    ) -> Result<Vec<u8>, LinuxRepositoryError> {
        if cancellation.is_cancelled() || Instant::now() >= self.deadline {
            return Err(denied());
        }
        self.owner.revalidate()?;
        let operation: Vec<String> = match (&self.stage, &step) {
            (Stage::Branch, Step::Branch) => vec![
                "rev-parse".into(),
                "--verify".into(),
                self.plan.task_branch.clone(),
            ],
            (Stage::Commit, Step::Commit(object))
                if object.len() == self.plan.parent_object.len()
                    && object
                        .bytes()
                        .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
                    && object.bytes().any(|b| b != b'0') =>
            {
                vec!["cat-file".into(), "commit".into(), (*object).into()]
            }
            (Stage::Update(object), Step::Update) => vec![
                "update-ref".into(),
                "--no-deref".into(),
                self.plan.task_branch.clone(),
                object.clone(),
                self.plan.parent_object.clone(),
            ],
            _ => return Err(denied()),
        };
        let mut args = crate::local_commit::hardened_git_prefix()
            .map(str::to_owned)
            .to_vec();
        args.push(format!(
            "--git-dir={}",
            self.owner.scope.git_directory.display()
        ));
        args.extend(operation);
        let environment = candidate::candidate_environment();
        let result = dispatch(self.owner.clone(), &args, &environment, self.deadline);
        self.owner.revalidate()?;
        let result = result.map_err(|_| denied())?;
        if cancellation.is_cancelled()
            || Instant::now() >= self.deadline
            || !result.output_complete
            || !matches!(result.outcome, Ok(OperationOutcome::Succeeded))
            || !result.status.is_some_and(|s| s.success())
            || result.stdout.total > MAX_OBSERVATION_BYTES
            || result.stderr.total > MAX_OBSERVATION_BYTES
            || result.stdout.total != result.stdout.retained.len()
            || result.stderr.total != result.stderr.retained.len()
        {
            return Err(denied());
        }
        let output = result.stdout.retained;
        self.stage = match step {
            Step::Branch if output == format!("{}\n", self.plan.parent_object).as_bytes() => {
                Stage::Commit
            }
            Step::Commit(object) if !output.is_empty() => Stage::Update(object.into()),
            Step::Update if output.is_empty() => Stage::Terminal,
            _ => return Err(denied()),
        };
        Ok(output)
    }
}
