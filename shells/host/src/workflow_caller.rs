//! Caller-neutral workflow attachment to the shared coding runtime.

use agentmage_kernel_contracts::{
    AgentStateKind, CONTRACT_SCHEMA_VERSION, EvidenceReference, ModelCancellationProbe,
    RuntimeApprovalChallenge, RuntimeApprovalResponse, RuntimeArtifactRef, RuntimeEvent,
    RuntimeOutcome, RuntimeRunRequest, ToolId, WorkPacket,
};
use agentmage_kernel_engine::{
    runtime_coordinator::verify_runtime_run_request,
    runtime_event::RuntimeEventSequence,
    workflow_authority::{WorkflowAuthorityIntersection, verify_workflow_authority_intersection},
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::coding_client::{CodingClientError, CodingCoordinatorPort};
use agentmage_kernel_engine::runtime_loop::RuntimeCoordinatorStep;

const ZERO_SHA256: &str = "0000000000000000000000000000000000000000000000000000000000000000";
const MAX_IDENTIFIER_BYTES: usize = 128;

/// Stable identities supplied by a future workflow caller.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkflowCallerIdentity {
    /// Stable identity of the immediate caller implementation.
    pub caller_id: String,
    /// Stable identity of the enclosing workflow.
    pub workflow_id: String,
    /// Stable identity of the exact workflow node.
    pub node_id: String,
    /// Stable identity of the parent invocation.
    pub parent_invocation_id: String,
}

/// Sealed caller-neutral submission to the existing coding runtime.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkflowRuntimeSubmission {
    /// Contract schema version.
    pub schema_version: u16,
    /// Exact caller and ancestry identities.
    pub caller: WorkflowCallerIdentity,
    /// Complete runtime request carrying budgets, stops, tools, snapshots, and policy.
    pub runtime_request: RuntimeRunRequest,
    /// Exact work packet repeated to prevent substitution by either boundary.
    pub work_packet: WorkPacket,
    /// Sorted unique tools requested by the caller, exactly equal to the visible catalog.
    pub requested_tool_ids: Vec<ToolId>,
    /// Seven-way narrowing-only authority result.
    pub authority: WorkflowAuthorityIntersection,
    /// Digest of this envelope with this field zeroed.
    pub submission_sha256: String,
}

/// Closed lifecycle states for the in-memory future-caller adapter.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowCallerState {
    /// The sealed submission was accepted.
    Submitted,
    /// The adapter acknowledged local ownership of the submission.
    Acknowledged,
    /// Canonical runtime events are being verified and returned.
    Streaming,
    /// Runtime execution is waiting for a separately supplied user decision.
    WaitingForUser,
    /// A protected user decision resumed the exact pending challenge.
    Resumed,
    /// Cancellation reached a verified terminal runtime outcome.
    Cancelled,
    /// The runtime reached another verified terminal outcome.
    Terminal,
}

/// One verified workflow-caller boundary result.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkflowCallerStep {
    /// Current closed lifecycle state.
    pub state: WorkflowCallerState,
    /// Newly verified canonical events in exact order.
    pub events: Vec<RuntimeEvent>,
    /// Protected challenge only while waiting for the user.
    pub approval: Option<RuntimeApprovalChallenge>,
    /// Verified canonical outcome only in a terminal state.
    pub outcome: Option<RuntimeOutcome>,
    /// Verified immutable artifact references available at this boundary.
    pub artifacts: Vec<RuntimeArtifactRef>,
    /// Grounded evidence retained by the terminal outcome.
    pub evidence: Vec<EvidenceReference>,
}

/// Stable fail-closed workflow caller error.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorkflowCallerError {
    /// Submission identity, bindings, authority, ordering, or digest are invalid.
    SubmissionDenied,
    /// The requested lifecycle transition is not legal from the current state.
    TransitionDenied,
    /// The shared coding runtime or canonical event verification failed.
    RuntimeDenied,
}

impl WorkflowCallerError {
    /// Returns one stable content-free diagnostic code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::SubmissionDenied => "workflow.caller.submission_denied",
            Self::TransitionDenied => "workflow.caller.transition_denied",
            Self::RuntimeDenied => "workflow.caller.runtime_denied",
        }
    }
}

impl std::fmt::Display for WorkflowCallerError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.code())
    }
}

impl std::error::Error for WorkflowCallerError {}

/// Seals one future-caller submission after validating every existing runtime binding.
pub fn seal_workflow_runtime_submission(
    mut submission: WorkflowRuntimeSubmission,
) -> Result<WorkflowRuntimeSubmission, WorkflowCallerError> {
    submission.schema_version = CONTRACT_SCHEMA_VERSION;
    submission.submission_sha256 = ZERO_SHA256.to_owned();
    validate_submission(&submission)?;
    submission.submission_sha256 = canonical_sha256(&submission)?;
    Ok(submission)
}

/// Verifies one retained future-caller submission from canonical bytes.
pub fn verify_workflow_runtime_submission(
    submission: &WorkflowRuntimeSubmission,
) -> Result<(), WorkflowCallerError> {
    validate_submission(submission)?;
    let mut preimage = submission.clone();
    preimage.submission_sha256 = ZERO_SHA256.to_owned();
    if canonical_sha256(&preimage)? != submission.submission_sha256 {
        return Err(WorkflowCallerError::SubmissionDenied);
    }
    Ok(())
}

/// Deterministic in-memory adapter over the existing shared runtime coordinator.
///
/// This type is deliberately not a workflow engine. It owns no scheduler, lease, retry policy,
/// trigger, credential, or approval source. A caller can only advance one sealed submission and
/// return canonical runtime boundaries.
pub struct InMemoryWorkflowCaller<R> {
    runtime: R,
    submission: WorkflowRuntimeSubmission,
    sequence: RuntimeEventSequence,
    presented: usize,
    state: WorkflowCallerState,
    history: Vec<WorkflowCallerState>,
    pending_approval: Option<RuntimeApprovalChallenge>,
}

impl<R: CodingCoordinatorPort> InMemoryWorkflowCaller<R> {
    /// Accepts one sealed submission and acknowledges local ownership without running it.
    pub fn submit(
        runtime: R,
        submission: WorkflowRuntimeSubmission,
    ) -> Result<Self, WorkflowCallerError> {
        verify_workflow_runtime_submission(&submission)?;
        Ok(Self {
            runtime,
            submission,
            sequence: RuntimeEventSequence::new(),
            presented: 0,
            state: WorkflowCallerState::Acknowledged,
            history: vec![
                WorkflowCallerState::Submitted,
                WorkflowCallerState::Acknowledged,
            ],
            pending_approval: None,
        })
    }

    /// Returns the exact sealed submission owned by this adapter.
    #[must_use]
    pub const fn submission(&self) -> &WorkflowRuntimeSubmission {
        &self.submission
    }

    /// Returns the complete deterministic lifecycle history.
    #[must_use]
    pub fn history(&self) -> &[WorkflowCallerState] {
        &self.history
    }

    /// Advances from acknowledgement or explicit resume to the next runtime boundary.
    pub fn advance(
        &mut self,
        cancellation: Option<&dyn ModelCancellationProbe>,
    ) -> Result<WorkflowCallerStep, WorkflowCallerError> {
        if !matches!(
            self.state,
            WorkflowCallerState::Acknowledged | WorkflowCallerState::Resumed
        ) {
            return Err(WorkflowCallerError::TransitionDenied);
        }
        self.transition(WorkflowCallerState::Streaming);
        self.advance_runtime(None, cancellation)
    }

    /// Resumes only the exact pending challenge with a separately protected user response.
    pub fn resume_after_user_decision(
        &mut self,
        response: &RuntimeApprovalResponse,
        cancellation: Option<&dyn ModelCancellationProbe>,
    ) -> Result<WorkflowCallerStep, WorkflowCallerError> {
        let Some(challenge) = self.pending_approval.as_ref() else {
            return Err(WorkflowCallerError::TransitionDenied);
        };
        if self.state != WorkflowCallerState::WaitingForUser
            || response.run_id != challenge.run_id
            || response.approval_id != challenge.approval_id
            || response.challenge_sha256 != challenge.challenge_sha256
        {
            return Err(WorkflowCallerError::TransitionDenied);
        }
        self.pending_approval = None;
        self.transition(WorkflowCallerState::Resumed);
        self.transition(WorkflowCallerState::Streaming);
        self.advance_runtime(Some(response), cancellation)
    }

    fn advance_runtime(
        &mut self,
        response: Option<&RuntimeApprovalResponse>,
        cancellation: Option<&dyn ModelCancellationProbe>,
    ) -> Result<WorkflowCallerStep, WorkflowCallerError> {
        let boundary = self
            .runtime
            .advance(response, cancellation)
            .map_err(map_runtime_error)?;
        let events = self.take_new_events()?;
        match boundary {
            RuntimeCoordinatorStep::AwaitingApproval { challenge } => {
                self.pending_approval = Some(challenge.clone());
                self.transition(WorkflowCallerState::WaitingForUser);
                Ok(WorkflowCallerStep {
                    state: self.state,
                    events,
                    approval: Some(challenge),
                    outcome: None,
                    artifacts: self.runtime.runtime_artifacts().to_vec(),
                    evidence: Vec::new(),
                })
            }
            RuntimeCoordinatorStep::Complete { outcome } => {
                if !self.sequence.is_terminal() {
                    return Err(WorkflowCallerError::RuntimeDenied);
                }
                let state = if outcome.state == AgentStateKind::Cancelled {
                    WorkflowCallerState::Cancelled
                } else {
                    WorkflowCallerState::Terminal
                };
                self.transition(state);
                Ok(WorkflowCallerStep {
                    state,
                    events,
                    approval: None,
                    artifacts: self.runtime.runtime_artifacts().to_vec(),
                    evidence: outcome.evidence.clone(),
                    outcome: Some(outcome),
                })
            }
        }
    }

    fn take_new_events(&mut self) -> Result<Vec<RuntimeEvent>, WorkflowCallerError> {
        let events = self.runtime.runtime_events();
        if self.presented > events.len() {
            return Err(WorkflowCallerError::RuntimeDenied);
        }
        let new_events = events[self.presented..].to_vec();
        for event in &new_events {
            self.sequence
                .push(event)
                .map_err(|_| WorkflowCallerError::RuntimeDenied)?;
        }
        self.presented = events.len();
        Ok(new_events)
    }

    fn transition(&mut self, state: WorkflowCallerState) {
        self.state = state;
        self.history.push(state);
    }
}

fn validate_submission(submission: &WorkflowRuntimeSubmission) -> Result<(), WorkflowCallerError> {
    if submission.schema_version != CONTRACT_SCHEMA_VERSION
        || !valid_caller(&submission.caller)
        || submission.work_packet != submission.runtime_request.work_packet
        || verify_runtime_run_request(&submission.runtime_request).is_err()
        || verify_workflow_authority_intersection(&submission.authority).is_err()
        || submission.authority.unattended_approval
        || !submission
            .authority
            .operations
            .contains(&agentmage_kernel_contracts::GrantOperation::ModelInference)
        || !submission
            .authority
            .target_scope_sha256s
            .contains(&submission.runtime_request.workspace_snapshot_sha256)
        || !submission
            .authority
            .target_scope_sha256s
            .contains(&submission.runtime_request.repository_snapshot_sha256)
        || !sorted_unique(&submission.requested_tool_ids)
    {
        return Err(WorkflowCallerError::SubmissionDenied);
    }
    let visible_tool_ids = submission
        .runtime_request
        .visible_tools
        .iter()
        .map(|tool| tool.tool_id.clone())
        .collect::<Vec<_>>();
    if submission.requested_tool_ids != visible_tool_ids
        || submission
            .requested_tool_ids
            .iter()
            .any(|tool_id| !submission.authority.tool_ids.contains(tool_id))
    {
        return Err(WorkflowCallerError::SubmissionDenied);
    }
    Ok(())
}

fn valid_caller(caller: &WorkflowCallerIdentity) -> bool {
    [
        caller.caller_id.as_str(),
        caller.workflow_id.as_str(),
        caller.node_id.as_str(),
        caller.parent_invocation_id.as_str(),
    ]
    .into_iter()
    .all(valid_identifier)
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

fn canonical_sha256(value: &impl Serialize) -> Result<String, WorkflowCallerError> {
    let bytes = serde_json::to_vec(value).map_err(|_| WorkflowCallerError::SubmissionDenied)?;
    Ok(Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect())
}

fn map_runtime_error(_error: CodingClientError) -> WorkflowCallerError {
    WorkflowCallerError::RuntimeDenied
}

#[cfg(test)]
mod tests {
    use agentmage_kernel_contracts::GrantOperation;
    use agentmage_kernel_engine::workflow_authority::{
        WorkflowAuthorityLayer, WorkflowAuthorityLayerKind, intersect_workflow_authority,
    };

    use super::*;
    use crate::coding_run::tests::fixture_profile_and_request;

    fn submission() -> WorkflowRuntimeSubmission {
        let (_, request) = fixture_profile_and_request();
        let mut tool_ids = request
            .visible_tools
            .iter()
            .map(|tool| tool.tool_id.clone())
            .collect::<Vec<_>>();
        tool_ids.sort();
        tool_ids.dedup();
        let mut targets = vec![
            request.workspace_snapshot_sha256.clone(),
            request.repository_snapshot_sha256.clone(),
        ];
        targets.sort();
        targets.dedup();
        let layers = WorkflowAuthorityLayerKind::ALL
            .into_iter()
            .enumerate()
            .map(|(index, kind)| WorkflowAuthorityLayer {
                kind,
                operations: vec![
                    GrantOperation::WorkspaceRead,
                    GrantOperation::ModelInference,
                ],
                tool_ids: tool_ids.clone(),
                target_scope_sha256s: targets.clone(),
                source_sha256: format!("{}", index + 1).repeat(64),
            })
            .collect();
        seal_workflow_runtime_submission(WorkflowRuntimeSubmission {
            schema_version: CONTRACT_SCHEMA_VERSION,
            caller: WorkflowCallerIdentity {
                caller_id: "fixture-caller".to_owned(),
                workflow_id: "fixture-workflow".to_owned(),
                node_id: "fixture-node".to_owned(),
                parent_invocation_id: "fixture-parent".to_owned(),
            },
            work_packet: request.work_packet.clone(),
            requested_tool_ids: tool_ids,
            runtime_request: request,
            authority: intersect_workflow_authority(layers).expect("authority intersection"),
            submission_sha256: ZERO_SHA256.to_owned(),
        })
        .expect("workflow submission")
    }

    #[test]
    fn caller_submission_binds_packet_tools_targets_and_authority() {
        let submission = submission();
        verify_workflow_runtime_submission(&submission).expect("submission verifies");
    }

    #[test]
    fn caller_submission_rejects_substitution_and_broadening() {
        let mut packet_drift = submission();
        packet_drift.work_packet.revision += 1;
        assert_eq!(
            verify_workflow_runtime_submission(&packet_drift),
            Err(WorkflowCallerError::SubmissionDenied)
        );

        let mut extra_tool = submission();
        extra_tool
            .requested_tool_ids
            .push(ToolId::from_raw("fixture.unseen"));
        assert_eq!(
            seal_workflow_runtime_submission(extra_tool),
            Err(WorkflowCallerError::SubmissionDenied)
        );

        let mut unattended = submission();
        unattended.authority.unattended_approval = true;
        assert_eq!(
            seal_workflow_runtime_submission(unattended),
            Err(WorkflowCallerError::SubmissionDenied)
        );
    }
}
