//! Transport-neutral native Chat adapter for the shared runtime coordinator.

use std::collections::BTreeMap;
use std::fmt;

use agentmage_kernel_contracts::{
    BoundaryKind, CancellationId, CancellationReason, CancellationSignal, RuntimeApprovalChallenge,
    RuntimeApprovalResponse, RuntimeEvent, RuntimeEventCursor, RuntimeEventKind, RuntimeOutcome,
    RuntimeRunId, RuntimeRunRequest, RuntimeSessionMode,
};
use agentmage_kernel_engine::{
    runtime_coordinator::{
        verify_runtime_approval_challenge, verify_runtime_approval_response,
        verify_runtime_outcome, verify_runtime_run_request,
    },
    runtime_event::RuntimeEventSequence,
    runtime_loop::RuntimeCoordinatorStep,
};

use crate::coding_client::{CodingClientError, CodingCoordinatorPort};

const MAX_NATIVE_CHAT_RUNS: usize = 4;

/// Trusted inputs from one authenticated native Chat preparation request.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NativeChatPrepareInput {
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
pub struct NativeChatRuntimeStep {
    /// Exact active runtime run.
    pub run_id: RuntimeRunId,
    /// Digest of the exact admitted runtime request.
    pub request_sha256: String,
    /// Ordered verified events after the caller's supplied cursor.
    pub events: Vec<RuntimeEvent>,
    /// Exact protected challenge only while the coordinator is waiting.
    pub approval: Option<RuntimeApprovalChallenge>,
    /// Canonical outcome only after terminal completion.
    pub outcome: Option<RuntimeOutcome>,
}

/// Stable content-free refusal from the native Chat runtime boundary.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NativeChatRuntimeError {
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

impl NativeChatRuntimeError {
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

impl fmt::Display for NativeChatRuntimeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.code())
    }
}

impl std::error::Error for NativeChatRuntimeError {}

/// Host-owned runtime operations exposed to the authenticated shell router.
pub trait NativeChatRuntimePort {
    /// Frames one exact request from current trusted profile, workspace, policy, and task state.
    fn prepare(
        &mut self,
        input: NativeChatPrepareInput,
    ) -> Result<RuntimeRunRequest, NativeChatRuntimeError>;

    /// Starts one exact previously framed request and returns its first coordinator boundary.
    fn start(
        &mut self,
        request: RuntimeRunRequest,
    ) -> Result<NativeChatRuntimeStep, NativeChatRuntimeError>;

    /// Advances or replays one exact run from a verified event cursor.
    fn advance(
        &mut self,
        run_id: &RuntimeRunId,
        request_sha256: &str,
        after_event_cursor: Option<&RuntimeEventCursor>,
        response: Option<&RuntimeApprovalResponse>,
    ) -> Result<NativeChatRuntimeStep, NativeChatRuntimeError>;

    /// Cancels one exact run and returns its truthful terminal boundary.
    fn cancel(
        &mut self,
        run_id: &RuntimeRunId,
        request_sha256: &str,
        cancellation_id: CancellationId,
        after_event_cursor: Option<&RuntimeEventCursor>,
    ) -> Result<NativeChatRuntimeStep, NativeChatRuntimeError>;
}

/// Trusted composition boundary that frames requests and creates the shared coordinator.
pub trait NativeChatRuntimeFactory {
    /// Exact coordinator implementation used by this host composition.
    type Coordinator: CodingCoordinatorPort;

    /// Builds one exact request from current trusted state without starting model or tool work.
    fn prepare_runtime_request(
        &mut self,
        input: &NativeChatPrepareInput,
    ) -> Result<RuntimeRunRequest, NativeChatRuntimeError>;

    /// Composes the shared coordinator for one exact request previously returned by this factory.
    fn compose_runtime(
        &mut self,
        request: &RuntimeRunRequest,
    ) -> Result<Self::Coordinator, NativeChatRuntimeError>;
}

/// Bounded host registry for prepared and active native Chat runs.
pub struct NativeChatRuntimeService<F>
where
    F: NativeChatRuntimeFactory,
{
    factory: F,
    prepared: BTreeMap<String, RuntimeRunRequest>,
    active: BTreeMap<String, CoordinatorNativeChatSession<F::Coordinator>>,
}

impl<F> NativeChatRuntimeService<F>
where
    F: NativeChatRuntimeFactory,
{
    /// Creates an empty service with no prepared or active work.
    #[must_use]
    pub fn new(factory: F) -> Self {
        Self {
            factory,
            prepared: BTreeMap::new(),
            active: BTreeMap::new(),
        }
    }
}

impl<F> NativeChatRuntimePort for NativeChatRuntimeService<F>
where
    F: NativeChatRuntimeFactory,
{
    fn prepare(
        &mut self,
        input: NativeChatPrepareInput,
    ) -> Result<RuntimeRunRequest, NativeChatRuntimeError> {
        self.active.retain(|_, session| !session.is_terminal());
        if self.prepared.len() + self.active.len() >= MAX_NATIVE_CHAT_RUNS {
            return Err(NativeChatRuntimeError::CapacityExceeded);
        }
        let request = self.factory.prepare_runtime_request(&input)?;
        verify_runtime_run_request(&request).map_err(|_| NativeChatRuntimeError::RequestDenied)?;
        if request.mode != RuntimeSessionMode::EphemeralReadOnly
            || request.event_cursor.is_some()
            || request.model_profile.profile_id.as_str() != input.profile_id
            || request.workspace_id.as_str() != input.workspace_id
            || request.task.objective != input.prompt
            || self.prepared.contains_key(request.run_id.as_str())
            || self.active.contains_key(request.run_id.as_str())
        {
            return Err(NativeChatRuntimeError::RequestDenied);
        }
        self.prepared
            .insert(request.run_id.as_str().to_owned(), request.clone());
        Ok(request)
    }

    fn start(
        &mut self,
        request: RuntimeRunRequest,
    ) -> Result<NativeChatRuntimeStep, NativeChatRuntimeError> {
        verify_runtime_run_request(&request).map_err(|_| NativeChatRuntimeError::RequestDenied)?;
        let key = request.run_id.as_str().to_owned();
        let prepared = self
            .prepared
            .remove(&key)
            .ok_or(NativeChatRuntimeError::RunUnavailable)?;
        if prepared != request || self.active.contains_key(&key) {
            return Err(NativeChatRuntimeError::RequestDenied);
        }
        let coordinator = self.factory.compose_runtime(&request)?;
        let mut session = CoordinatorNativeChatSession::new(request, coordinator)?;
        let step = session.start()?;
        self.active.insert(key, session);
        Ok(step)
    }

    fn advance(
        &mut self,
        run_id: &RuntimeRunId,
        request_sha256: &str,
        after_event_cursor: Option<&RuntimeEventCursor>,
        response: Option<&RuntimeApprovalResponse>,
    ) -> Result<NativeChatRuntimeStep, NativeChatRuntimeError> {
        self.active
            .get_mut(run_id.as_str())
            .ok_or(NativeChatRuntimeError::RunUnavailable)?
            .advance(request_sha256, after_event_cursor, response)
    }

    fn cancel(
        &mut self,
        run_id: &RuntimeRunId,
        request_sha256: &str,
        cancellation_id: CancellationId,
        after_event_cursor: Option<&RuntimeEventCursor>,
    ) -> Result<NativeChatRuntimeStep, NativeChatRuntimeError> {
        self.active
            .get_mut(run_id.as_str())
            .ok_or(NativeChatRuntimeError::RunUnavailable)?
            .cancel(request_sha256, cancellation_id, after_event_cursor)
    }
}

/// One exact native Chat view over a reusable runtime coordinator.
pub struct CoordinatorNativeChatSession<R>
where
    R: CodingCoordinatorPort,
{
    request: RuntimeRunRequest,
    runtime: R,
    started: bool,
    pending_approval: Option<RuntimeApprovalChallenge>,
    outcome: Option<RuntimeOutcome>,
}

impl<R> CoordinatorNativeChatSession<R>
where
    R: CodingCoordinatorPort,
{
    /// Binds one already composed coordinator to its exact admitted request.
    pub fn new(request: RuntimeRunRequest, runtime: R) -> Result<Self, NativeChatRuntimeError> {
        verify_runtime_run_request(&request).map_err(|_| NativeChatRuntimeError::RequestDenied)?;
        Ok(Self {
            request,
            runtime,
            started: false,
            pending_approval: None,
            outcome: None,
        })
    }

    /// Starts the coordinator exactly once.
    pub fn start(&mut self) -> Result<NativeChatRuntimeStep, NativeChatRuntimeError> {
        if self.started {
            return Err(NativeChatRuntimeError::RequestDenied);
        }
        self.started = true;
        self.run(None, None, None)
    }

    /// Advances from one protected decision or replays the current boundary without mutation.
    pub fn advance(
        &mut self,
        request_sha256: &str,
        after_event_cursor: Option<&RuntimeEventCursor>,
        response: Option<&RuntimeApprovalResponse>,
    ) -> Result<NativeChatRuntimeStep, NativeChatRuntimeError> {
        self.verify_binding(request_sha256)?;
        if response.is_none() || self.outcome.is_some() {
            if response.is_some() {
                return Err(NativeChatRuntimeError::ApprovalDenied);
            }
            return self.project(after_event_cursor);
        }
        let challenge = self
            .pending_approval
            .as_ref()
            .ok_or(NativeChatRuntimeError::ApprovalDenied)?;
        let structural_time = challenge.expires_at_epoch_ms.saturating_sub(1);
        verify_runtime_approval_response(challenge, response.expect("checked"), structural_time)
            .map_err(|_| NativeChatRuntimeError::ApprovalDenied)?;
        self.run(response, None, after_event_cursor)
    }

    /// Cancels the current coordinator boundary using one shell-originated cancellation tree.
    pub fn cancel(
        &mut self,
        request_sha256: &str,
        cancellation_id: CancellationId,
        after_event_cursor: Option<&RuntimeEventCursor>,
    ) -> Result<NativeChatRuntimeStep, NativeChatRuntimeError> {
        self.verify_binding(request_sha256)?;
        if self.outcome.is_some() {
            return self.project(after_event_cursor);
        }
        let correlation_id = self
            .runtime
            .runtime_events()
            .first()
            .map(|event| event.correlation_id.clone())
            .ok_or(NativeChatRuntimeError::RuntimeEvidenceDenied)?;
        let signal = CancellationSignal {
            schema_version: self.request.schema_version,
            cancellation_id,
            correlation_id,
            task_id: self.request.task.task_id.clone(),
            reason: CancellationReason::UserRequested,
            requested_by: BoundaryKind::Shell,
        };
        self.run(None, Some(&signal), after_event_cursor)
    }

    fn verify_binding(&self, request_sha256: &str) -> Result<(), NativeChatRuntimeError> {
        if !self.started || self.request.request_sha256 != request_sha256 {
            Err(NativeChatRuntimeError::RequestDenied)
        } else {
            Ok(())
        }
    }

    fn run(
        &mut self,
        response: Option<&RuntimeApprovalResponse>,
        cancellation: Option<&CancellationSignal>,
        after_event_cursor: Option<&RuntimeEventCursor>,
    ) -> Result<NativeChatRuntimeStep, NativeChatRuntimeError> {
        let step = self
            .runtime
            .advance(response, cancellation.map(|signal| signal as _))
            .map_err(map_client_error)?;
        match step {
            RuntimeCoordinatorStep::AwaitingApproval { challenge } => {
                verify_runtime_approval_challenge(&challenge)
                    .map_err(|_| NativeChatRuntimeError::RuntimeEvidenceDenied)?;
                if challenge.run_id != self.request.run_id
                    || challenge.task_id != self.request.task.task_id
                {
                    return Err(NativeChatRuntimeError::RuntimeEvidenceDenied);
                }
                self.pending_approval = Some(challenge);
            }
            RuntimeCoordinatorStep::Complete { outcome } => {
                verify_runtime_outcome(&outcome, &self.request)
                    .map_err(|_| NativeChatRuntimeError::RuntimeEvidenceDenied)?;
                self.pending_approval = None;
                self.outcome = Some(outcome);
            }
        }
        self.project(after_event_cursor)
    }

    fn project(
        &self,
        after_event_cursor: Option<&RuntimeEventCursor>,
    ) -> Result<NativeChatRuntimeStep, NativeChatRuntimeError> {
        let events = self.runtime.runtime_events();
        if events.is_empty() {
            return Err(NativeChatRuntimeError::RuntimeEvidenceDenied);
        }
        let mut sequence = RuntimeEventSequence::new();
        for event in events {
            if event.run_id != self.request.run_id
                || event.session_id != self.request.session_id
                || event.task_id != self.request.task.task_id
                || event.policy_id != self.request.policy_id
            {
                return Err(NativeChatRuntimeError::RuntimeEvidenceDenied);
            }
            sequence
                .push(event)
                .map_err(|_| NativeChatRuntimeError::RuntimeEvidenceDenied)?;
        }
        if self.outcome.is_some() != sequence.is_terminal()
            || self.pending_approval.is_some() == self.outcome.is_some()
        {
            return Err(NativeChatRuntimeError::RuntimeEvidenceDenied);
        }
        if let Some(challenge) = &self.pending_approval {
            let Some(RuntimeEvent {
                kind:
                    RuntimeEventKind::PermissionRequested {
                        approval_id,
                        preview_sha256,
                        expires_at_epoch_ms,
                        ..
                    },
                ..
            }) = events.last()
            else {
                return Err(NativeChatRuntimeError::RuntimeEvidenceDenied);
            };
            if approval_id != &challenge.approval_id
                || preview_sha256 != &challenge.preview_sha256
                || expires_at_epoch_ms != &challenge.expires_at_epoch_ms
            {
                return Err(NativeChatRuntimeError::RuntimeEvidenceDenied);
            }
        }
        let first = match after_event_cursor {
            None => 0,
            Some(cursor) => {
                let index = usize::try_from(cursor.sequence)
                    .map_err(|_| NativeChatRuntimeError::EventCursorDenied)?;
                let event = events
                    .get(index)
                    .ok_or(NativeChatRuntimeError::EventCursorDenied)?;
                if cursor.run_id != self.request.run_id
                    || event.event_id != cursor.event_id
                    || event.sequence != cursor.sequence
                    || event.event_sha256 != cursor.event_sha256
                {
                    return Err(NativeChatRuntimeError::EventCursorDenied);
                }
                index
                    .checked_add(1)
                    .ok_or(NativeChatRuntimeError::EventCursorDenied)?
            }
        };
        Ok(NativeChatRuntimeStep {
            run_id: self.request.run_id.clone(),
            request_sha256: self.request.request_sha256.clone(),
            events: events[first..].to_vec(),
            approval: self.pending_approval.clone(),
            outcome: self.outcome.clone(),
        })
    }

    fn is_terminal(&self) -> bool {
        self.outcome.is_some()
    }
}

const fn map_client_error(error: CodingClientError) -> NativeChatRuntimeError {
    match error {
        CodingClientError::Runtime => NativeChatRuntimeError::RuntimeFailed,
        CodingClientError::EventStream
        | CodingClientError::Approval
        | CodingClientError::Presentation => NativeChatRuntimeError::RuntimeEvidenceDenied,
    }
}

#[cfg(test)]
mod tests {
    use agentmage_kernel_contracts::{
        AgentStateKind, ApprovalId, CancellationSignal, ContextSensitivity, CorrelationId, GrantId,
        GrantOperation, ModelCancellationProbe, ModelRunId, RuntimeApprovalDisposition,
        RuntimeApprovalResponse, RuntimeArtifactRef, RuntimeEvent, RuntimeEventCursor,
        RuntimeEventId, RuntimeEventKind, RuntimeEventRetention, RuntimeEventRetentionKind,
        RuntimeOperationId, RuntimeOutcome, RuntimePermissionDisposition, RuntimeRunRequest,
        RuntimeTurnId, ToolCallId,
    };
    use agentmage_kernel_engine::{
        runtime_coordinator::{seal_runtime_approval_challenge, seal_runtime_outcome},
        runtime_event::{runtime_event_persistence, seal_runtime_event},
        runtime_loop::RuntimeCoordinatorStep,
    };

    use super::{
        CodingClientError, CodingCoordinatorPort, CoordinatorNativeChatSession,
        NativeChatPrepareInput, NativeChatRuntimeError, NativeChatRuntimeFactory,
        NativeChatRuntimePort, NativeChatRuntimeService,
    };

    const ZERO_SHA256: &str = "0000000000000000000000000000000000000000000000000000000000000000";

    struct ReplayCoordinator {
        events: Vec<RuntimeEvent>,
        outcome: RuntimeOutcome,
    }

    impl CodingCoordinatorPort for ReplayCoordinator {
        fn advance(
            &mut self,
            _response: Option<&RuntimeApprovalResponse>,
            _cancellation: Option<&dyn ModelCancellationProbe>,
        ) -> Result<RuntimeCoordinatorStep, CodingClientError> {
            Ok(RuntimeCoordinatorStep::Complete {
                outcome: self.outcome.clone(),
            })
        }

        fn runtime_events(&self) -> &[RuntimeEvent] {
            &self.events
        }

        fn runtime_artifacts(&self) -> &[RuntimeArtifactRef] {
            &[]
        }
    }

    struct ReplayFactory {
        expected_input: NativeChatPrepareInput,
        request: RuntimeRunRequest,
        events: Vec<RuntimeEvent>,
        outcome: RuntimeOutcome,
    }

    impl NativeChatRuntimeFactory for ReplayFactory {
        type Coordinator = ReplayCoordinator;

        fn prepare_runtime_request(
            &mut self,
            input: &NativeChatPrepareInput,
        ) -> Result<RuntimeRunRequest, NativeChatRuntimeError> {
            if input != &self.expected_input {
                return Err(NativeChatRuntimeError::RequestDenied);
            }
            Ok(self.request.clone())
        }

        fn compose_runtime(
            &mut self,
            request: &RuntimeRunRequest,
        ) -> Result<Self::Coordinator, NativeChatRuntimeError> {
            if request != &self.request {
                return Err(NativeChatRuntimeError::RequestDenied);
            }
            Ok(ReplayCoordinator {
                events: self.events.clone(),
                outcome: self.outcome.clone(),
            })
        }
    }

    #[test]
    fn completed_shared_runtime_is_prepared_started_and_replayed_by_exact_cursor() {
        let (request, events, outcome, observed_result) =
            crate::runtime_read_tests::completed_native_read_fixture();
        assert!(observed_result);
        let input = NativeChatPrepareInput {
            profile_id: request.model_profile.profile_id.as_str().to_owned(),
            expected_entry_sha256: "a".repeat(64),
            workspace_id: request.workspace_id.as_str().to_owned(),
            workspace_root: "/tmp/agentmage-native-chat-fixture".to_owned(),
            prompt: request.task.objective.clone(),
        };
        let factory = ReplayFactory {
            expected_input: input.clone(),
            request: request.clone(),
            events: events.clone(),
            outcome: outcome.clone(),
        };
        let mut service = NativeChatRuntimeService::new(factory);

        let prepared = service.prepare(input).expect("exact request is prepared");
        assert_eq!(prepared, request);
        let step = service
            .start(prepared.clone())
            .expect("shared runtime is started");
        assert_eq!(step.events, events);
        assert_eq!(step.outcome.as_ref(), Some(&outcome));
        assert!(step.approval.is_none());

        let first_cursor = cursor(step.events.first().expect("run-start event"));
        let replay = service
            .advance(
                &prepared.run_id,
                &prepared.request_sha256,
                Some(&first_cursor),
                None,
            )
            .expect("verified cursor replays only unseen events");
        assert_eq!(replay.events, step.events[1..]);
        assert_eq!(replay.outcome, step.outcome);

        let mut substituted_cursor = first_cursor;
        substituted_cursor.event_sha256 = "f".repeat(64);
        assert_eq!(
            service.advance(
                &prepared.run_id,
                &prepared.request_sha256,
                Some(&substituted_cursor),
                None,
            ),
            Err(NativeChatRuntimeError::EventCursorDenied)
        );
        assert_eq!(
            service.advance(&prepared.run_id, &"f".repeat(64), None, None),
            Err(NativeChatRuntimeError::RequestDenied)
        );
        assert_eq!(
            service.start(prepared),
            Err(NativeChatRuntimeError::RunUnavailable)
        );
    }

    #[test]
    fn protected_approval_and_cancellation_preserve_exact_runtime_identity() {
        let (request, _, _, _) = crate::runtime_read_tests::completed_native_read_fixture();

        let mut approval_session = CoordinatorNativeChatSession::new(
            request.clone(),
            ScriptedApprovalCoordinator::new(request.clone()),
        )
        .expect("approval session");
        let awaiting = approval_session.start().expect("approval boundary");
        let challenge = awaiting.approval.clone().expect("protected challenge");
        let permission_cursor = cursor(awaiting.events.last().expect("permission event"));
        assert!(awaiting.outcome.is_none());

        let mut substituted = response(&challenge, RuntimeApprovalDisposition::Deny);
        substituted.challenge_sha256 = "f".repeat(64);
        assert_eq!(
            approval_session.advance(
                &request.request_sha256,
                Some(&permission_cursor),
                Some(&substituted),
            ),
            Err(NativeChatRuntimeError::ApprovalDenied)
        );

        let denied = approval_session
            .advance(
                &request.request_sha256,
                Some(&permission_cursor),
                Some(&response(&challenge, RuntimeApprovalDisposition::Deny)),
            )
            .expect("exact denial reaches a terminal boundary");
        assert_eq!(
            denied.outcome.as_ref().map(|outcome| outcome.state),
            Some(AgentStateKind::Declined)
        );
        assert!(matches!(
            denied.events.first().map(|event| &event.kind),
            Some(RuntimeEventKind::PermissionDecided {
                disposition: RuntimePermissionDisposition::Deny,
                ..
            })
        ));

        let mut cancellation_session = CoordinatorNativeChatSession::new(
            request.clone(),
            ScriptedApprovalCoordinator::new(request.clone()),
        )
        .expect("cancellation session");
        let awaiting = cancellation_session.start().expect("approval boundary");
        let permission_cursor = cursor(awaiting.events.last().expect("permission event"));
        let cancellation_id =
            agentmage_kernel_contracts::CancellationId::from_raw("native-chat-cancellation-0001");
        let cancelled = cancellation_session
            .cancel(
                &request.request_sha256,
                cancellation_id.clone(),
                Some(&permission_cursor),
            )
            .expect("exact cancellation reaches a terminal boundary");
        assert_eq!(
            cancelled.outcome.as_ref().map(|outcome| outcome.state),
            Some(AgentStateKind::Cancelled)
        );
        assert!(cancelled.events.iter().any(|event| matches!(
            &event.kind,
            RuntimeEventKind::CancellationObserved { cancellation_id: observed }
                if observed == &cancellation_id
        )));
    }

    fn cursor(event: &RuntimeEvent) -> RuntimeEventCursor {
        RuntimeEventCursor {
            run_id: event.run_id.clone(),
            event_id: event.event_id.clone(),
            sequence: event.sequence,
            event_sha256: event.event_sha256.clone(),
        }
    }

    fn response(
        challenge: &agentmage_kernel_contracts::RuntimeApprovalChallenge,
        disposition: RuntimeApprovalDisposition,
    ) -> RuntimeApprovalResponse {
        RuntimeApprovalResponse {
            schema_version: challenge.schema_version,
            run_id: challenge.run_id.clone(),
            approval_id: challenge.approval_id.clone(),
            disposition,
            challenge_sha256: challenge.challenge_sha256.clone(),
            grant_id: (disposition == RuntimeApprovalDisposition::Allow)
                .then(|| challenge.proposed_grant_id.clone()),
        }
    }

    struct ScriptedApprovalCoordinator {
        request: RuntimeRunRequest,
        stream: FixtureStream,
        events: Vec<RuntimeEvent>,
        challenge: agentmage_kernel_contracts::RuntimeApprovalChallenge,
        started: bool,
        outcome: Option<RuntimeOutcome>,
    }

    impl ScriptedApprovalCoordinator {
        fn new(request: RuntimeRunRequest) -> Self {
            let approval_id = ApprovalId::from_raw("native-chat-approval-0001");
            let turn_id = RuntimeTurnId::from_raw("native-chat-turn-0001");
            let operation_id = RuntimeOperationId::from_raw("native-chat-operation-0001");
            let tool_call_id = ToolCallId::from_raw("native-chat-tool-call-0001");
            let challenge = seal_runtime_approval_challenge(
                agentmage_kernel_contracts::RuntimeApprovalChallenge {
                    schema_version: request.schema_version,
                    run_id: request.run_id.clone(),
                    task_id: request.task.task_id.clone(),
                    turn_id,
                    operation_id,
                    tool_call_id,
                    approval_id,
                    proposed_grant_id: GrantId::from_raw("native-chat-grant-0001"),
                    operation: GrantOperation::WorkspaceRead,
                    preview_sha256: "5".repeat(64),
                    expires_at_epoch_ms: 50_000,
                    challenge_sha256: ZERO_SHA256.to_owned(),
                },
            )
            .expect("challenge seals");
            Self {
                stream: FixtureStream::new(&request),
                request,
                events: Vec::new(),
                challenge,
                started: false,
                outcome: None,
            }
        }

        fn start_events(&mut self) {
            let turn = self.challenge.turn_id.clone();
            let operation = self.challenge.operation_id.clone();
            let tool_call = self.challenge.tool_call_id.clone();
            self.emit(
                RuntimeEventKind::RunStarted {
                    request_sha256: self.request.request_sha256.clone(),
                },
                None,
                None,
            );
            self.emit(RuntimeEventKind::TurnStarted, Some(&turn), None);
            self.emit(
                RuntimeEventKind::ModelRequested {
                    model_run_id: ModelRunId::from_raw("native-chat-model-run-0001"),
                    request_sha256: "2".repeat(64),
                },
                Some(&turn),
                None,
            );
            self.emit(
                RuntimeEventKind::ModelCompleted {
                    model_run_id: ModelRunId::from_raw("native-chat-model-run-0001"),
                    result_sha256: "3".repeat(64),
                },
                Some(&turn),
                None,
            );
            self.emit(
                RuntimeEventKind::ToolRequested {
                    tool_call_id: tool_call,
                    arguments_sha256: "4".repeat(64),
                },
                Some(&turn),
                Some(&operation),
            );
            self.emit(
                RuntimeEventKind::PermissionRequested {
                    approval_id: self.challenge.approval_id.clone(),
                    operation: self.challenge.operation,
                    preview_sha256: self.challenge.preview_sha256.clone(),
                    expires_at_epoch_ms: self.challenge.expires_at_epoch_ms,
                },
                Some(&turn),
                Some(&operation),
            );
        }

        fn finish_denied(&mut self, response: &RuntimeApprovalResponse) -> RuntimeOutcome {
            let turn = self.challenge.turn_id.clone();
            let operation = self.challenge.operation_id.clone();
            self.emit(
                RuntimeEventKind::PermissionDecided {
                    approval_id: response.approval_id.clone(),
                    disposition: RuntimePermissionDisposition::Deny,
                    grant_id: None,
                    decision_sha256: response.challenge_sha256.clone(),
                },
                Some(&turn),
                Some(&operation),
            );
            self.emit(
                RuntimeEventKind::TurnCompleted {
                    outcome_sha256: "6".repeat(64),
                },
                Some(&turn),
                None,
            );
            self.finish(AgentStateKind::Declined)
        }

        fn finish_cancelled(&mut self, signal: &CancellationSignal) -> RuntimeOutcome {
            self.emit(
                RuntimeEventKind::CancellationRequested {
                    cancellation_id: signal.cancellation_id.clone(),
                },
                None,
                None,
            );
            self.emit(
                RuntimeEventKind::CancellationObserved {
                    cancellation_id: signal.cancellation_id.clone(),
                },
                None,
                None,
            );
            self.finish(AgentStateKind::Cancelled)
        }

        fn finish(&mut self, state: AgentStateKind) -> RuntimeOutcome {
            let prior = self.events.last().expect("prior event");
            let outcome = seal_runtime_outcome(
                RuntimeOutcome {
                    schema_version: self.request.schema_version,
                    run_id: self.request.run_id.clone(),
                    session_id: self.request.session_id.clone(),
                    task_id: self.request.task.task_id.clone(),
                    request_sha256: self.request.request_sha256.clone(),
                    state,
                    turn_count: 1,
                    model_call_count: 1,
                    tool_call_count: 0,
                    prior_event_id: prior.event_id.clone(),
                    prior_event_sha256: prior.event_sha256.clone(),
                    evidence: Vec::new(),
                    receipt_ids: Vec::new(),
                    unresolved_codes: Vec::new(),
                    output: None,
                    outcome_sha256: ZERO_SHA256.to_owned(),
                },
                &self.request,
            )
            .expect("outcome seals");
            self.emit(
                RuntimeEventKind::RunTerminal {
                    state,
                    outcome_sha256: outcome.outcome_sha256.clone(),
                },
                None,
                None,
            );
            self.outcome = Some(outcome.clone());
            outcome
        }

        fn emit(
            &mut self,
            kind: RuntimeEventKind,
            turn_id: Option<&RuntimeTurnId>,
            operation_id: Option<&RuntimeOperationId>,
        ) {
            self.events
                .push(self.stream.event(kind, turn_id, operation_id));
        }
    }

    impl CodingCoordinatorPort for ScriptedApprovalCoordinator {
        fn advance(
            &mut self,
            response: Option<&RuntimeApprovalResponse>,
            cancellation: Option<&dyn ModelCancellationProbe>,
        ) -> Result<RuntimeCoordinatorStep, CodingClientError> {
            if !self.started {
                if response.is_some() || cancellation.is_some() {
                    return Err(CodingClientError::Runtime);
                }
                self.started = true;
                self.start_events();
                return Ok(RuntimeCoordinatorStep::AwaitingApproval {
                    challenge: self.challenge.clone(),
                });
            }
            if let Some(outcome) = &self.outcome {
                return Ok(RuntimeCoordinatorStep::Complete {
                    outcome: outcome.clone(),
                });
            }
            if let Some(probe) = cancellation {
                let signal = probe
                    .observe()
                    .map_err(|_| CodingClientError::Runtime)?
                    .ok_or(CodingClientError::Runtime)?;
                if signal.task_id != self.request.task.task_id
                    || signal.correlation_id != self.stream.correlation_id
                {
                    return Err(CodingClientError::Runtime);
                }
                return Ok(RuntimeCoordinatorStep::Complete {
                    outcome: self.finish_cancelled(&signal),
                });
            }
            let response = response.ok_or(CodingClientError::Approval)?;
            if response.disposition != RuntimeApprovalDisposition::Deny {
                return Err(CodingClientError::Approval);
            }
            Ok(RuntimeCoordinatorStep::Complete {
                outcome: self.finish_denied(response),
            })
        }

        fn runtime_events(&self) -> &[RuntimeEvent] {
            &self.events
        }

        fn runtime_artifacts(&self) -> &[RuntimeArtifactRef] {
            &[]
        }
    }

    struct FixtureStream {
        request: RuntimeRunRequest,
        correlation_id: CorrelationId,
        sequence: u64,
        previous_sha256: String,
        causation_event_id: Option<RuntimeEventId>,
    }

    impl FixtureStream {
        fn new(request: &RuntimeRunRequest) -> Self {
            Self {
                request: request.clone(),
                correlation_id: CorrelationId::from_raw("native-chat-correlation-0001"),
                sequence: 0,
                previous_sha256: ZERO_SHA256.to_owned(),
                causation_event_id: None,
            }
        }

        fn event(
            &mut self,
            kind: RuntimeEventKind,
            turn_id: Option<&RuntimeTurnId>,
            operation_id: Option<&RuntimeOperationId>,
        ) -> RuntimeEvent {
            let event_id =
                RuntimeEventId::from_raw(format!("native-chat-event-{:04}", self.sequence));
            let event = seal_runtime_event(RuntimeEvent {
                schema_version: self.request.schema_version,
                event_id: event_id.clone(),
                run_id: self.request.run_id.clone(),
                session_id: self.request.session_id.clone(),
                task_id: self.request.task.task_id.clone(),
                turn_id: turn_id.cloned(),
                operation_id: operation_id.cloned(),
                correlation_id: self.correlation_id.clone(),
                causation_event_id: self.causation_event_id.clone(),
                sequence: self.sequence,
                occurred_at_epoch_ms: 1_000 + self.sequence,
                sensitivity: ContextSensitivity::Internal,
                retention: RuntimeEventRetention {
                    kind: RuntimeEventRetentionKind::Ephemeral,
                    expires_at_epoch_ms: None,
                },
                persistence: runtime_event_persistence(&kind),
                policy_id: self.request.policy_id.clone(),
                payload_reference: None,
                kind,
                previous_event_sha256: self.previous_sha256.clone(),
                event_sha256: ZERO_SHA256.to_owned(),
            })
            .expect("fixture event seals");
            self.sequence += 1;
            self.previous_sha256.clone_from(&event.event_sha256);
            self.causation_event_id = Some(event_id);
            event
        }
    }
}
