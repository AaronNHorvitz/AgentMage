//! Thin interactive and headless clients for the shared coding runtime.

use std::fmt;

use agentmage_kernel_contracts::{
    CONTRACT_SCHEMA_VERSION, ModelCancellationProbe, RuntimeApprovalChallenge,
    RuntimeApprovalDisposition, RuntimeApprovalResponse, RuntimeArtifactRef, RuntimeEvent,
    RuntimeOutcome,
};
use agentmage_kernel_engine::{
    runtime_event::RuntimeEventSequence,
    runtime_loop::{
        ReusableRuntimeCoordinator, RuntimeClock, RuntimeContextPort, RuntimeCoordinatorStep,
        RuntimeModelPort, RuntimeToolBoundary, RuntimeVerifierPort,
    },
};

/// Stable content-free failure from a coding client presentation boundary.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CodingClientError {
    /// The shared runtime rejected or could not complete the operation.
    Runtime,
    /// The canonical event chain was malformed, reordered, or incomplete.
    EventStream,
    /// The protected approval channel could not produce one exact decision.
    Approval,
    /// The bounded client presentation sink could not accept an event.
    Presentation,
}

impl CodingClientError {
    /// Returns a stable content-free diagnostic code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::Runtime => "coding.client.runtime_failed",
            Self::EventStream => "coding.client.event_stream_invalid",
            Self::Approval => "coding.client.approval_unavailable",
            Self::Presentation => "coding.client.presentation_failed",
        }
    }
}

impl fmt::Display for CodingClientError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.code())
    }
}

impl std::error::Error for CodingClientError {}

/// Minimal shared-runtime surface consumed by every first-party coding client.
pub trait CodingCoordinatorPort {
    /// Advances the one canonical operation to approval or terminal completion.
    fn advance(
        &mut self,
        response: Option<&RuntimeApprovalResponse>,
        cancellation: Option<&dyn ModelCancellationProbe>,
    ) -> Result<RuntimeCoordinatorStep, CodingClientError>;

    /// Returns the complete canonical event history currently held by the runtime.
    fn runtime_events(&self) -> &[RuntimeEvent];

    /// Returns every verified artifact reference published by the runtime.
    fn runtime_artifacts(&self) -> &[RuntimeArtifactRef];
}

impl<M, X, T, V, C> CodingCoordinatorPort for ReusableRuntimeCoordinator<M, X, T, V, C>
where
    M: RuntimeModelPort,
    X: RuntimeContextPort,
    T: RuntimeToolBoundary,
    V: RuntimeVerifierPort,
    C: RuntimeClock,
{
    fn advance(
        &mut self,
        response: Option<&RuntimeApprovalResponse>,
        cancellation: Option<&dyn ModelCancellationProbe>,
    ) -> Result<RuntimeCoordinatorStep, CodingClientError> {
        self.run_until_boundary(response, cancellation)
            .map_err(|_| CodingClientError::Runtime)
    }

    fn runtime_events(&self) -> &[RuntimeEvent] {
        self.events()
    }

    fn runtime_artifacts(&self) -> &[RuntimeArtifactRef] {
        self.artifact_references()
    }
}

/// Protected user-decision boundary supplied by an interactive client.
pub trait CodingApprovalPort {
    /// Returns one explicit decision for the exact displayed challenge.
    fn decide(
        &mut self,
        challenge: &RuntimeApprovalChallenge,
    ) -> Result<RuntimeApprovalDisposition, CodingClientError>;
}

/// Bounded presentation boundary for already verified canonical runtime events.
pub trait CodingEventSink {
    /// Presents one event without changing runtime authority or execution state.
    fn present(&mut self, event: &RuntimeEvent) -> Result<(), CodingClientError>;
}

/// Approval policy for noninteractive callers: every unexpected prompt is denied.
#[derive(Clone, Copy, Debug, Default)]
pub struct DenyHeadlessApproval;

impl CodingApprovalPort for DenyHeadlessApproval {
    fn decide(
        &mut self,
        _challenge: &RuntimeApprovalChallenge,
    ) -> Result<RuntimeApprovalDisposition, CodingClientError> {
        Ok(RuntimeApprovalDisposition::Deny)
    }
}

/// Verified interface-neutral result returned after one coding client reaches a terminal state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CodingClientResult {
    /// Canonical runtime outcome shared by every client.
    pub outcome: RuntimeOutcome,
    /// Exact verified artifact references available to the presentation layer.
    pub artifacts: Vec<RuntimeArtifactRef>,
    /// Number of canonical events independently verified and presented.
    pub presented_events: u64,
}

/// Drives one coding client entirely through the shared runtime coordinator.
///
/// This function cannot dispatch tools, mint grants, read artifacts, or alter policy. It only
/// advances the coordinator, presents its verified event chain, and returns exact approval choices
/// to the coordinator's protected boundary.
pub fn drive_coding_client<R, A, S>(
    runtime: &mut R,
    approvals: &mut A,
    sink: &mut S,
    cancellation: Option<&dyn ModelCancellationProbe>,
) -> Result<CodingClientResult, CodingClientError>
where
    R: CodingCoordinatorPort,
    A: CodingApprovalPort,
    S: CodingEventSink,
{
    let mut response = None;
    let mut presented = 0_usize;
    let mut sequence = RuntimeEventSequence::new();
    loop {
        let step = runtime.advance(response.as_ref(), cancellation)?;
        present_new_events(runtime, sink, &mut sequence, &mut presented)?;
        match step {
            RuntimeCoordinatorStep::AwaitingApproval { challenge } => {
                let disposition = approvals.decide(&challenge)?;
                response = Some(approval_response(&challenge, disposition));
            }
            RuntimeCoordinatorStep::Complete { outcome } => {
                if !sequence.is_terminal() {
                    return Err(CodingClientError::EventStream);
                }
                return Ok(CodingClientResult {
                    outcome,
                    artifacts: runtime.runtime_artifacts().to_vec(),
                    presented_events: sequence.event_count(),
                });
            }
        }
    }
}

fn present_new_events<R, S>(
    runtime: &R,
    sink: &mut S,
    sequence: &mut RuntimeEventSequence,
    presented: &mut usize,
) -> Result<(), CodingClientError>
where
    R: CodingCoordinatorPort,
    S: CodingEventSink,
{
    let events = runtime.runtime_events();
    if *presented > events.len() {
        return Err(CodingClientError::EventStream);
    }
    for event in &events[*presented..] {
        sequence
            .push(event)
            .map_err(|_| CodingClientError::EventStream)?;
        sink.present(event)?;
        *presented += 1;
    }
    Ok(())
}

fn approval_response(
    challenge: &RuntimeApprovalChallenge,
    disposition: RuntimeApprovalDisposition,
) -> RuntimeApprovalResponse {
    RuntimeApprovalResponse {
        schema_version: CONTRACT_SCHEMA_VERSION,
        run_id: challenge.run_id.clone(),
        approval_id: challenge.approval_id.clone(),
        disposition,
        challenge_sha256: challenge.challenge_sha256.clone(),
        grant_id: (disposition == RuntimeApprovalDisposition::Allow)
            .then(|| challenge.proposed_grant_id.clone()),
    }
}
