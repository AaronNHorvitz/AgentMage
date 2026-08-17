//! Narrow child-assignment adapter over the shared workflow caller runtime.

use agentmage_kernel_contracts::{
    EvidenceReference, ModelCancellationProbe, RuntimeApprovalResponse, RuntimeArtifactRef,
    RuntimeOutcome,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{
    coding_client::CodingCoordinatorPort,
    workflow_caller::{
        InMemoryWorkflowCaller, WorkflowCallerError, WorkflowCallerState, WorkflowCallerStep,
        WorkflowRuntimeSubmission, verify_workflow_runtime_submission,
    },
};

const ZERO_SHA256: &str = "0000000000000000000000000000000000000000000000000000000000000000";
const MAX_IDENTIFIER_BYTES: usize = 128;
const MAX_DEPENDENCIES: usize = 64;
const MAX_NESTING_DEPTH: u8 = 8;

/// One sealed child assignment that reuses the complete shared runtime submission.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChildRuntimeAssignment {
    /// Stable assignment identity.
    pub assignment_id: String,
    /// Stable parent invocation identity.
    pub parent_invocation_id: String,
    /// Stable child identity, equal to the runtime caller node identity.
    pub child_id: String,
    /// Sorted completed dependency assignment identities.
    pub dependency_ids: Vec<String>,
    /// One-based child nesting depth.
    pub nesting_depth: u8,
    /// Complete caller-neutral runtime submission.
    pub submission: WorkflowRuntimeSubmission,
    /// Always true: child results cannot certify completion for a parent.
    pub parent_review_required: bool,
    /// Digest of this assignment with this field zeroed.
    pub assignment_sha256: String,
}

/// Closed parent-review disposition for an untrusted child result.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChildReviewDisposition {
    /// No parent review has occurred.
    Pending,
    /// Parent review accepted the result as a proposal.
    Accepted,
    /// Parent review rejected the result.
    Rejected,
    /// Parent review requested a separately sealed revised assignment.
    Revise,
}

/// Terminal child output retained as an untrusted, authority-free proposal.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChildRuntimeProposal {
    /// Exact assignment identity.
    pub assignment_id: String,
    /// Exact child identity.
    pub child_id: String,
    /// Canonical runtime outcome, not a parent completion decision.
    pub outcome: RuntimeOutcome,
    /// Exact grounded evidence returned by the shared runtime.
    pub evidence: Vec<EvidenceReference>,
    /// Exact immutable artifact references returned by the shared runtime.
    pub artifacts: Vec<RuntimeArtifactRef>,
    /// Ordered digests of runtime events returned at the terminal boundary.
    pub terminal_event_sha256s: Vec<String>,
    /// Initially pending and changeable only by a future parent-review state machine.
    pub review: ChildReviewDisposition,
    /// Always false: child results contain no authority for another operation or child.
    pub authority_from_result: bool,
}

/// Stable child-assignment adapter refusal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChildAssignmentError {
    /// Assignment identity, ancestry, dependencies, depth, binding, or digest are invalid.
    AssignmentDenied,
    /// The shared workflow caller denied the submission or transition.
    RuntimeDenied,
}

impl ChildAssignmentError {
    /// Returns one stable content-free diagnostic code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::AssignmentDenied => "workflow.child.assignment_denied",
            Self::RuntimeDenied => "workflow.child.runtime_denied",
        }
    }
}

impl std::fmt::Display for ChildAssignmentError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.code())
    }
}

impl std::error::Error for ChildAssignmentError {}

/// Seals one child assignment without creating or running a child.
pub fn seal_child_runtime_assignment(
    mut assignment: ChildRuntimeAssignment,
) -> Result<ChildRuntimeAssignment, ChildAssignmentError> {
    assignment.assignment_sha256 = ZERO_SHA256.to_owned();
    validate_assignment(&assignment)?;
    assignment.assignment_sha256 = canonical_sha256(&assignment)?;
    Ok(assignment)
}

/// Verifies one retained child assignment from canonical bytes.
pub fn verify_child_runtime_assignment(
    assignment: &ChildRuntimeAssignment,
) -> Result<(), ChildAssignmentError> {
    validate_assignment(assignment)?;
    let mut preimage = assignment.clone();
    preimage.assignment_sha256 = ZERO_SHA256.to_owned();
    if canonical_sha256(&preimage)? != assignment.assignment_sha256 {
        return Err(ChildAssignmentError::AssignmentDenied);
    }
    Ok(())
}

/// One child adapter over the existing caller-neutral runtime.
pub struct ChildRuntimeCaller<R> {
    assignment: ChildRuntimeAssignment,
    caller: InMemoryWorkflowCaller<R>,
}

impl<R: CodingCoordinatorPort> ChildRuntimeCaller<R> {
    /// Submits one verified assignment through the shared runtime caller.
    pub fn submit(
        runtime: R,
        assignment: ChildRuntimeAssignment,
    ) -> Result<Self, ChildAssignmentError> {
        verify_child_runtime_assignment(&assignment)?;
        let caller = InMemoryWorkflowCaller::submit(runtime, assignment.submission.clone())
            .map_err(map_runtime_error)?;
        Ok(Self { assignment, caller })
    }

    /// Advances through the shared runtime without a child-specific execution path.
    pub fn advance(
        &mut self,
        cancellation: Option<&dyn ModelCancellationProbe>,
    ) -> Result<WorkflowCallerStep, ChildAssignmentError> {
        self.caller.advance(cancellation).map_err(map_runtime_error)
    }

    /// Returns the underlying shared-runtime lifecycle without a child-specific state machine.
    #[must_use]
    pub fn history(&self) -> &[WorkflowCallerState] {
        self.caller.history()
    }

    /// Resumes only after the shared protected user-decision boundary validates the response.
    pub fn resume_after_user_decision(
        &mut self,
        response: &RuntimeApprovalResponse,
        cancellation: Option<&dyn ModelCancellationProbe>,
    ) -> Result<WorkflowCallerStep, ChildAssignmentError> {
        self.caller
            .resume_after_user_decision(response, cancellation)
            .map_err(map_runtime_error)
    }

    /// Converts a terminal shared-runtime step into an untrusted parent-review proposal.
    pub fn terminal_proposal(
        &self,
        step: WorkflowCallerStep,
    ) -> Result<ChildRuntimeProposal, ChildAssignmentError> {
        if !matches!(
            step.state,
            WorkflowCallerState::Terminal | WorkflowCallerState::Cancelled
        ) {
            return Err(ChildAssignmentError::RuntimeDenied);
        }
        let outcome = step.outcome.ok_or(ChildAssignmentError::RuntimeDenied)?;
        Ok(ChildRuntimeProposal {
            assignment_id: self.assignment.assignment_id.clone(),
            child_id: self.assignment.child_id.clone(),
            outcome,
            evidence: step.evidence,
            artifacts: step.artifacts,
            terminal_event_sha256s: step
                .events
                .iter()
                .map(|event| event.event_sha256.clone())
                .collect(),
            review: ChildReviewDisposition::Pending,
            authority_from_result: false,
        })
    }
}

fn validate_assignment(assignment: &ChildRuntimeAssignment) -> Result<(), ChildAssignmentError> {
    if !valid_identifier(&assignment.assignment_id)
        || !valid_identifier(&assignment.parent_invocation_id)
        || !valid_identifier(&assignment.child_id)
        || assignment.dependency_ids.len() > MAX_DEPENDENCIES
        || assignment
            .dependency_ids
            .iter()
            .any(|dependency| !valid_identifier(dependency))
        || !sorted_unique(&assignment.dependency_ids)
        || assignment
            .dependency_ids
            .iter()
            .any(|dependency| dependency == &assignment.assignment_id)
        || !(1..=MAX_NESTING_DEPTH).contains(&assignment.nesting_depth)
        || !assignment.parent_review_required
        || assignment.child_id != assignment.submission.caller.node_id
        || assignment.parent_invocation_id != assignment.submission.caller.parent_invocation_id
        || verify_workflow_runtime_submission(&assignment.submission).is_err()
    {
        return Err(ChildAssignmentError::AssignmentDenied);
    }
    Ok(())
}

fn valid_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_IDENTIFIER_BYTES
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b':' | b'-'))
}

fn sorted_unique<T: Ord>(values: &[T]) -> bool {
    values.windows(2).all(|pair| pair[0] < pair[1])
}

fn canonical_sha256(value: &impl Serialize) -> Result<String, ChildAssignmentError> {
    let bytes = serde_json::to_vec(value).map_err(|_| ChildAssignmentError::AssignmentDenied)?;
    Ok(Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect())
}

fn map_runtime_error(_error: WorkflowCallerError) -> ChildAssignmentError {
    ChildAssignmentError::RuntimeDenied
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workflow_caller::tests::submission;

    fn assignment() -> ChildRuntimeAssignment {
        let mut submission = submission();
        submission.caller.node_id = "fixture-child".to_owned();
        submission.caller.parent_invocation_id = "fixture-parent".to_owned();
        submission.submission_sha256 = ZERO_SHA256.to_owned();
        submission = crate::workflow_caller::seal_workflow_runtime_submission(submission)
            .expect("rebound submission");
        seal_child_runtime_assignment(ChildRuntimeAssignment {
            assignment_id: "fixture-assignment".to_owned(),
            parent_invocation_id: "fixture-parent".to_owned(),
            child_id: "fixture-child".to_owned(),
            dependency_ids: vec!["dependency-0001".to_owned()],
            nesting_depth: 1,
            submission,
            parent_review_required: true,
            assignment_sha256: ZERO_SHA256.to_owned(),
        })
        .expect("child assignment")
    }

    #[test]
    fn sprint_95_assignment_is_bound_to_the_shared_runtime_submission() {
        let assignment = assignment();
        verify_child_runtime_assignment(&assignment).expect("assignment verifies");
    }

    #[test]
    fn sprint_95_recursive_dependency_parent_drift_and_unreviewed_result_fail_closed() {
        let mut recursive = assignment();
        recursive.dependency_ids = vec![recursive.assignment_id.clone()];
        assert_eq!(
            seal_child_runtime_assignment(recursive),
            Err(ChildAssignmentError::AssignmentDenied)
        );

        let mut parent_drift = assignment();
        parent_drift.parent_invocation_id = "another-parent".to_owned();
        assert_eq!(
            seal_child_runtime_assignment(parent_drift),
            Err(ChildAssignmentError::AssignmentDenied)
        );

        let mut no_review = assignment();
        no_review.parent_review_required = false;
        assert_eq!(
            seal_child_runtime_assignment(no_review),
            Err(ChildAssignmentError::AssignmentDenied)
        );
    }
}
