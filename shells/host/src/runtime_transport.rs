//! Caller-neutral transport contract for authenticated shared-runtime clients.

use std::fmt;

use agentmage_kernel_contracts::{
    CancellationId, RuntimeApprovalChallenge, RuntimeApprovalResponse, RuntimeArtifactRef,
    RuntimeEvent, RuntimeEventCursor, RuntimeOutcome, RuntimeRunId, RuntimeRunRequest, SessionId,
};

/// Trusted inputs from one authenticated runtime preparation request.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimePrepareInput {
    /// Exact pre-existing Engineering session for approved-Plan execution, when applicable.
    pub engineering_session_id: Option<SessionId>,
    /// Exact selected profile identity.
    pub profile_id: String,
    /// Digest of the exact picker entry displayed to the user.
    pub expected_entry_sha256: String,
    /// Exact selected local workspace identity.
    pub workspace_id: String,
    /// Selected absolute local workspace root, consumed only by the trusted host factory.
    pub workspace_root: String,
    /// Bounded user objective retained as inert task input.
    pub prompt: String,
}

/// One verified coordinator boundary returned to a transport-only client.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeTransportStep {
    /// Exact active runtime run.
    pub run_id: RuntimeRunId,
    /// Digest of the exact admitted runtime request.
    pub request_sha256: String,
    /// Ordered verified events after the caller's supplied cursor.
    pub events: Vec<RuntimeEvent>,
    /// Complete verified artifact-reference set currently owned by the coordinator.
    pub artifacts: Vec<RuntimeArtifactRef>,
    /// Exact protected challenge only while the coordinator is waiting.
    pub approval: Option<RuntimeApprovalChallenge>,
    /// Canonical outcome only after terminal completion.
    pub outcome: Option<RuntimeOutcome>,
}

/// Stable content-free refusal from a shared runtime transport boundary.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuntimeTransportError {
    /// The preparation or runtime request was invalid, stale, or substituted.
    RequestDenied,
    /// The selected run is absent or no longer active.
    RunUnavailable,
    /// The supplied event cursor does not identify the exact current stream.
    EventCursorDenied,
    /// The approval response did not match the one pending challenge.
    ApprovalDenied,
    /// The reusable runtime failed closed.
    RuntimeFailed,
    /// The coordinator exposed a malformed or inconsistent event/outcome boundary.
    RuntimeEvidenceDenied,
    /// The bounded active/prepared run ceiling was reached.
    CapacityExceeded,
}

impl RuntimeTransportError {
    /// Returns one stable content-free diagnostic code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::RequestDenied => "host.runtime.request_denied",
            Self::RunUnavailable => "host.runtime.run_unavailable",
            Self::EventCursorDenied => "host.runtime.event_cursor_denied",
            Self::ApprovalDenied => "host.runtime.approval_denied",
            Self::RuntimeFailed => "host.runtime.failed",
            Self::RuntimeEvidenceDenied => "host.runtime.evidence_denied",
            Self::CapacityExceeded => "host.runtime.capacity_exceeded",
        }
    }
}

impl fmt::Display for RuntimeTransportError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.code())
    }
}

impl std::error::Error for RuntimeTransportError {}

/// Host-owned runtime operations exposed to one authenticated local client.
pub trait RuntimeTransportPort {
    /// Frames one exact request from current trusted profile, workspace, policy, and task state.
    fn prepare(
        &mut self,
        input: RuntimePrepareInput,
    ) -> Result<RuntimeRunRequest, RuntimeTransportError>;

    /// Starts one exact previously framed request and returns its first coordinator boundary.
    fn start(
        &mut self,
        request: RuntimeRunRequest,
    ) -> Result<RuntimeTransportStep, RuntimeTransportError>;

    /// Advances or replays one exact run from a verified event cursor.
    fn advance(
        &mut self,
        run_id: &RuntimeRunId,
        request_sha256: &str,
        after_event_cursor: Option<&RuntimeEventCursor>,
        response: Option<&RuntimeApprovalResponse>,
    ) -> Result<RuntimeTransportStep, RuntimeTransportError>;

    /// Cancels one exact run and returns its truthful terminal boundary.
    fn cancel(
        &mut self,
        run_id: &RuntimeRunId,
        request_sha256: &str,
        cancellation_id: CancellationId,
        after_event_cursor: Option<&RuntimeEventCursor>,
    ) -> Result<RuntimeTransportStep, RuntimeTransportError>;

    /// Discards one unstarted request or removes one already terminal run.
    fn release(
        &mut self,
        run_id: &RuntimeRunId,
        request_sha256: &str,
    ) -> Result<(), RuntimeTransportError>;
}
