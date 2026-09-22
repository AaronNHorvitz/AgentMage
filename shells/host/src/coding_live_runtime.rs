//! Live host scheduling for a reusable coding coordinator.
//!
//! Model and tool work runs on one owned worker. The authenticated IPC owner remains available to
//! drain bounded canonical events, validate cursors, present approvals, and set a cancellation
//! signal. This module adds no effect, model-selection, approval, or storage authority.

use std::collections::BTreeMap;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
    mpsc::{Receiver, SyncSender, TryRecvError, sync_channel},
};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use agentmage_kernel_contracts::{
    BoundaryKind, CancellationId, CancellationReason, CancellationSignal, ModelCancellationProbe,
    ModelRuntimeFailure, RuntimeApprovalChallenge, RuntimeApprovalResponse, RuntimeArtifactRef,
    RuntimeEvent, RuntimeEventCursor, RuntimeEventKind, RuntimeOutcome, RuntimeRunId,
    RuntimeRunRequest, RuntimeSessionMode,
};
use agentmage_kernel_engine::{
    runtime_artifact::{RuntimeArtifactPage, RuntimeArtifactState},
    runtime_coordinator::{
        verify_runtime_approval_challenge, verify_runtime_approval_response,
        verify_runtime_outcome, verify_runtime_run_request,
    },
    runtime_event::{RuntimeEventSequence, RuntimeEventSubscription},
    runtime_hardening::{
        MAX_RUNTIME_CLIENT_QUEUE_BYTES, MAX_RUNTIME_CLIENT_QUEUE_EVENTS,
        MAX_RUNTIME_EVENT_ENVELOPE_BYTES,
    },
    runtime_loop::RuntimeCoordinatorStep,
};

use crate::coding_client::{CodingClientError, LiveCodingCoordinatorPort};
use crate::native_chat_runtime::NativeChatRuntimeFactory;
use crate::runtime_transport::{
    RuntimePrepareInput, RuntimeTransportError, RuntimeTransportPort, RuntimeTransportStep,
};

const MAX_LIVE_RUNS: usize = 4;
const MAX_LIVE_CURSOR_AGE_EVENTS: usize = 32;
const CONTROL_WAIT: Duration = Duration::from_millis(25);
const START_WAIT: Duration = Duration::from_millis(250);
const WAIT_SLICE: Duration = Duration::from_millis(2);

enum WorkerCommand {
    Advance(Option<RuntimeApprovalResponse>),
    ReadArtifactPage {
        reference: RuntimeArtifactRef,
        offset: u64,
        maximum_bytes: u32,
    },
    ReleaseArtifact(RuntimeArtifactRef),
    Stop,
}

struct WorkerBoundary {
    step: RuntimeCoordinatorStep,
    artifacts: Vec<RuntimeArtifactRef>,
}

enum WorkerResponse {
    Boundary(WorkerBoundary),
    ArtifactPage(RuntimeArtifactPage),
    ArtifactState(RuntimeArtifactState),
}

#[derive(Default)]
struct EventPumpState {
    sequence: RuntimeEventSequence,
    events: Vec<RuntimeEvent>,
    disconnected: bool,
    failed: bool,
}

struct EventPump {
    state: Arc<Mutex<EventPumpState>>,
    stop: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
}

impl EventPump {
    fn spawn(
        subscription: RuntimeEventSubscription,
        request: &RuntimeRunRequest,
        initial_events: Vec<RuntimeEvent>,
    ) -> Result<Self, RuntimeTransportError> {
        let mut sequence = RuntimeEventSequence::new();
        if initial_events.len() > request.limits.max_events as usize
            || initial_events.iter().any(|event| {
                event.run_id != request.run_id
                    || event.session_id != request.session_id
                    || event.task_id != request.task.task_id
                    || event.policy_id != request.policy_id
                    || sequence.push(event).is_err()
            })
        {
            return Err(RuntimeTransportError::RuntimeEvidenceDenied);
        }
        let state = Arc::new(Mutex::new(EventPumpState {
            sequence,
            events: initial_events,
            disconnected: false,
            failed: false,
        }));
        let stop = Arc::new(AtomicBool::new(false));
        let worker_state = Arc::clone(&state);
        let worker_stop = Arc::clone(&stop);
        let run_id = request.run_id.clone();
        let session_id = request.session_id.clone();
        let task_id = request.task.task_id.clone();
        let policy_id = request.policy_id.clone();
        let maximum_events = request.limits.max_events as usize;
        let worker = thread::Builder::new()
            .name("agentmage-coding-event-pump".to_owned())
            .spawn(move || {
                while !worker_stop.load(Ordering::Acquire) {
                    match subscription.try_next() {
                        Ok(Some(event)) => {
                            let mut current = match worker_state.lock() {
                                Ok(current) => current,
                                Err(_) => return,
                            };
                            if event.run_id != run_id
                                || event.session_id != session_id
                                || event.task_id != task_id
                                || event.policy_id != policy_id
                                || current.events.len() >= maximum_events
                                || current.sequence.push(&event).is_err()
                            {
                                current.failed = true;
                                return;
                            }
                            let terminal = current.sequence.is_terminal();
                            current.events.push(event);
                            if terminal {
                                return;
                            }
                        }
                        Ok(None) => thread::sleep(WAIT_SLICE),
                        Err(_) => {
                            if let Ok(mut current) = worker_state.lock() {
                                current.disconnected = true;
                            }
                            return;
                        }
                    }
                }
            })
            .map_err(|_| RuntimeTransportError::RuntimeFailed)?;
        Ok(Self {
            state,
            stop,
            worker: Some(worker),
        })
    }

    fn snapshot(&self) -> Result<(Vec<RuntimeEvent>, bool, bool), RuntimeTransportError> {
        let state = self
            .state
            .lock()
            .map_err(|_| RuntimeTransportError::RuntimeEvidenceDenied)?;
        if state.failed || state.disconnected && !state.sequence.is_terminal() {
            return Err(RuntimeTransportError::RuntimeEvidenceDenied);
        }
        Ok((
            state.events.clone(),
            state.sequence.is_terminal(),
            state.disconnected,
        ))
    }
}

impl Drop for EventPump {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

struct SharedCancellation {
    signal: Mutex<Option<CancellationSignal>>,
}

impl SharedCancellation {
    const fn new() -> Self {
        Self {
            signal: Mutex::new(None),
        }
    }

    fn request(&self, signal: CancellationSignal) -> Result<(), RuntimeTransportError> {
        let mut current = self
            .signal
            .lock()
            .map_err(|_| RuntimeTransportError::RuntimeFailed)?;
        match current.as_ref() {
            Some(existing) if existing != &signal => Err(RuntimeTransportError::RequestDenied),
            Some(_) => Ok(()),
            None => {
                *current = Some(signal);
                Ok(())
            }
        }
    }
}

impl ModelCancellationProbe for SharedCancellation {
    fn observe(&self) -> Result<Option<CancellationSignal>, ModelRuntimeFailure> {
        self.signal
            .lock()
            .map(|signal| signal.clone())
            .map_err(|_| ModelRuntimeFailure {
                code: "coding.live.cancellation-unavailable".to_owned(),
                retryable_after_correction: false,
                dependency_recovery_required: true,
                contract_error: None,
            })
    }
}

/// Bounded development-host registry whose control owner never executes model or tool work.
pub struct LiveCodingRuntimeService<F>
where
    F: NativeChatRuntimeFactory,
    F::Coordinator: LiveCodingCoordinatorPort,
{
    factory: F,
    prepared: BTreeMap<String, PreparedLiveRun>,
    active: BTreeMap<String, LiveCodingSession>,
}

struct PreparedLiveRun {
    request: RuntimeRunRequest,
    slow_subscriber_probe: bool,
}

impl<F> LiveCodingRuntimeService<F>
where
    F: NativeChatRuntimeFactory,
    F::Coordinator: LiveCodingCoordinatorPort,
{
    /// Creates one empty, single-client host registry.
    #[must_use]
    pub fn new(factory: F) -> Self {
        Self {
            factory,
            prepared: BTreeMap::new(),
            active: BTreeMap::new(),
        }
    }
}

impl<F> RuntimeTransportPort for LiveCodingRuntimeService<F>
where
    F: NativeChatRuntimeFactory,
    F::Coordinator: LiveCodingCoordinatorPort,
{
    fn prepare(
        &mut self,
        input: RuntimePrepareInput,
    ) -> Result<RuntimeRunRequest, RuntimeTransportError> {
        if self.prepared.len() + self.active.len() >= MAX_LIVE_RUNS {
            return Err(RuntimeTransportError::CapacityExceeded);
        }
        if input.resume && !self.active.is_empty()
            || input
                .engineering_session_id
                .as_ref()
                .is_some_and(|session_id| {
                    self.active
                        .values()
                        .any(|session| &session.request.session_id == session_id)
                })
        {
            return Err(RuntimeTransportError::RequestDenied);
        }
        let request = self.factory.prepare_runtime_request(&input)?;
        verify_runtime_run_request(&request).map_err(|_| RuntimeTransportError::RequestDenied)?;
        if !matches!(
            request.mode,
            RuntimeSessionMode::EphemeralReadOnly | RuntimeSessionMode::ControlledWrite
        ) || request.event_cursor.is_some() != input.resume
            || request.model_profile.profile_id.as_str() != input.profile_id
            || request.workspace_id.as_str() != input.workspace_id
            || request.task.objective != input.prompt
            || input
                .engineering_session_id
                .as_ref()
                .is_some_and(|session_id| request.session_id != *session_id)
            || self.prepared.contains_key(request.run_id.as_str())
            || self.active.contains_key(request.run_id.as_str())
        {
            return Err(RuntimeTransportError::RequestDenied);
        }
        self.prepared.insert(
            request.run_id.as_str().to_owned(),
            PreparedLiveRun {
                request: request.clone(),
                slow_subscriber_probe: input.slow_subscriber_probe,
            },
        );
        Ok(request)
    }

    fn start(
        &mut self,
        request: RuntimeRunRequest,
    ) -> Result<RuntimeTransportStep, RuntimeTransportError> {
        verify_runtime_run_request(&request).map_err(|_| RuntimeTransportError::RequestDenied)?;
        let key = request.run_id.as_str().to_owned();
        let prepared = self
            .prepared
            .get(&key)
            .ok_or(RuntimeTransportError::RequestDenied)?;
        if prepared.request != request || self.active.contains_key(&key) {
            return Err(RuntimeTransportError::RequestDenied);
        }
        let slow_subscriber_probe = prepared.slow_subscriber_probe;
        let coordinator = self.factory.compose_runtime(&request)?;
        let mut session = LiveCodingSession::spawn(request, coordinator, slow_subscriber_probe)
            .map_err(|error| {
                eprintln!("coding.live.spawn-denied");
                error
            })?;
        self.prepared.remove(&key);
        let step = session.start().map_err(|error| {
            eprintln!("coding.live.start-denied");
            error
        })?;
        self.active.insert(key, session);
        Ok(step)
    }

    fn advance(
        &mut self,
        run_id: &RuntimeRunId,
        request_sha256: &str,
        after_event_cursor: Option<&RuntimeEventCursor>,
        response: Option<&RuntimeApprovalResponse>,
    ) -> Result<RuntimeTransportStep, RuntimeTransportError> {
        self.active
            .get_mut(run_id.as_str())
            .ok_or(RuntimeTransportError::RunUnavailable)?
            .advance(request_sha256, after_event_cursor, response)
    }

    fn cancel(
        &mut self,
        run_id: &RuntimeRunId,
        request_sha256: &str,
        cancellation_id: CancellationId,
        after_event_cursor: Option<&RuntimeEventCursor>,
    ) -> Result<RuntimeTransportStep, RuntimeTransportError> {
        self.active
            .get_mut(run_id.as_str())
            .ok_or(RuntimeTransportError::RunUnavailable)?
            .cancel(request_sha256, cancellation_id, after_event_cursor)
    }

    fn read_artifact_page(
        &mut self,
        run_id: &RuntimeRunId,
        request_sha256: &str,
        reference: &RuntimeArtifactRef,
        offset: u64,
        maximum_bytes: u32,
    ) -> Result<RuntimeArtifactPage, RuntimeTransportError> {
        self.active
            .get_mut(run_id.as_str())
            .ok_or(RuntimeTransportError::RunUnavailable)?
            .read_artifact_page(request_sha256, reference, offset, maximum_bytes)
    }

    fn release_artifact(
        &mut self,
        run_id: &RuntimeRunId,
        request_sha256: &str,
        reference: &RuntimeArtifactRef,
    ) -> Result<RuntimeArtifactState, RuntimeTransportError> {
        self.active
            .get_mut(run_id.as_str())
            .ok_or(RuntimeTransportError::RunUnavailable)?
            .release_artifact(request_sha256, reference)
    }

    fn revoke_session_preauthorization(
        &mut self,
        session_id: &agentmage_kernel_contracts::SessionId,
        preauthorization_sha256: &str,
    ) -> Result<(), RuntimeTransportError> {
        self.factory
            .revoke_session_preauthorization(session_id, preauthorization_sha256)
    }

    fn release(
        &mut self,
        run_id: &RuntimeRunId,
        request_sha256: &str,
    ) -> Result<(), RuntimeTransportError> {
        let key = run_id.as_str();
        if let Some(request) = self.prepared.get(key) {
            if request.request.request_sha256 != request_sha256 {
                return Err(RuntimeTransportError::RequestDenied);
            }
            self.prepared.remove(key);
            return Ok(());
        }
        let session = self
            .active
            .get(key)
            .ok_or(RuntimeTransportError::RunUnavailable)?;
        if !session.is_terminal() || session.request.request_sha256 != request_sha256 {
            return Err(RuntimeTransportError::RequestDenied);
        }
        self.active.remove(key);
        Ok(())
    }
}

struct LiveCodingSession {
    request: RuntimeRunRequest,
    commands: SyncSender<WorkerCommand>,
    results: Receiver<Result<WorkerResponse, CodingClientError>>,
    event_pump: EventPump,
    slow_subscriber_probe: Option<RuntimeEventSubscription>,
    slow_subscriber_verified: bool,
    cancellation: Arc<SharedCancellation>,
    worker: Option<JoinHandle<()>>,
    events: Vec<RuntimeEvent>,
    latest_presented_cursor: Option<RuntimeEventCursor>,
    artifacts: Vec<RuntimeArtifactRef>,
    pending_approval: Option<RuntimeApprovalChallenge>,
    outcome: Option<RuntimeOutcome>,
    busy: bool,
    event_stream_terminal: bool,
}

impl LiveCodingSession {
    fn spawn<R>(
        request: RuntimeRunRequest,
        mut runtime: R,
        slow_subscriber_probe: bool,
    ) -> Result<Self, RuntimeTransportError>
    where
        R: LiveCodingCoordinatorPort,
    {
        let queue_by_bytes = MAX_RUNTIME_CLIENT_QUEUE_BYTES / MAX_RUNTIME_EVENT_ENVELOPE_BYTES;
        let capacity = usize::try_from(
            u64::from(request.limits.max_events)
                .min(u64::from(MAX_RUNTIME_CLIENT_QUEUE_EVENTS))
                .min(queue_by_bytes),
        )
        .ok()
        .filter(|capacity| *capacity > 0)
        .ok_or(RuntimeTransportError::RequestDenied)?;
        let initial_events = runtime.runtime_events().to_vec();
        let slow_subscriber_probe = slow_subscriber_probe
            .then(|| runtime.subscribe_live_events(1).map_err(map_client_error))
            .transpose()?;
        let subscription = runtime
            .subscribe_live_events(capacity)
            .map_err(map_client_error)?;
        let event_pump = EventPump::spawn(subscription, &request, initial_events)?;
        let cancellation = Arc::new(SharedCancellation::new());
        let worker_cancellation = Arc::clone(&cancellation);
        let (command_tx, command_rx) = sync_channel::<WorkerCommand>(1);
        let (result_tx, result_rx) = sync_channel(1);
        let worker = thread::Builder::new()
            .name("agentmage-coding-runtime".to_owned())
            .spawn(move || {
                while let Ok(command) = command_rx.recv() {
                    let result = match command {
                        WorkerCommand::Advance(response) => runtime
                            .advance(response.as_ref(), Some(worker_cancellation.as_ref()))
                            .map(|step| {
                                WorkerResponse::Boundary(WorkerBoundary {
                                    step,
                                    artifacts: runtime.runtime_artifacts().to_vec(),
                                })
                            }),
                        WorkerCommand::ReadArtifactPage {
                            reference,
                            offset,
                            maximum_bytes,
                        } => runtime
                            .read_artifact_page(&reference, offset, maximum_bytes)
                            .map(WorkerResponse::ArtifactPage),
                        WorkerCommand::ReleaseArtifact(reference) => runtime
                            .release_artifact(&reference)
                            .map(WorkerResponse::ArtifactState),
                        WorkerCommand::Stop => return,
                    };
                    if result_tx.send(result).is_err() {
                        return;
                    }
                }
            })
            .map_err(|_| RuntimeTransportError::RuntimeFailed)?;
        Ok(Self {
            request,
            commands: command_tx,
            results: result_rx,
            event_pump,
            slow_subscriber_probe,
            slow_subscriber_verified: false,
            cancellation,
            worker: Some(worker),
            events: Vec::new(),
            latest_presented_cursor: None,
            artifacts: Vec::new(),
            pending_approval: None,
            outcome: None,
            busy: false,
            event_stream_terminal: false,
        })
    }

    fn start(&mut self) -> Result<RuntimeTransportStep, RuntimeTransportError> {
        self.dispatch(None)?;
        self.refresh(START_WAIT)?;
        if self.events.is_empty() {
            return Err(RuntimeTransportError::RuntimeFailed);
        }
        self.project(None, true)
    }

    fn advance(
        &mut self,
        request_sha256: &str,
        after_event_cursor: Option<&RuntimeEventCursor>,
        response: Option<&RuntimeApprovalResponse>,
    ) -> Result<RuntimeTransportStep, RuntimeTransportError> {
        self.verify_binding(request_sha256)?;
        self.refresh(Duration::ZERO)?;
        let _ = self.project(after_event_cursor, false)?;
        if let Some(response) = response {
            let challenge = self
                .pending_approval
                .as_ref()
                .ok_or(RuntimeTransportError::ApprovalDenied)?;
            let structural_time = challenge.expires_at_epoch_ms.saturating_sub(1);
            verify_runtime_approval_response(challenge, response, structural_time)
                .map_err(|_| RuntimeTransportError::ApprovalDenied)?;
            self.pending_approval = None;
            self.dispatch(Some(response.clone()))?;
        }
        self.refresh(CONTROL_WAIT)?;
        self.project(after_event_cursor, true)
    }

    fn cancel(
        &mut self,
        request_sha256: &str,
        cancellation_id: CancellationId,
        after_event_cursor: Option<&RuntimeEventCursor>,
    ) -> Result<RuntimeTransportStep, RuntimeTransportError> {
        self.verify_binding(request_sha256)?;
        self.refresh(Duration::ZERO)?;
        let _ = self.project(after_event_cursor, false)?;
        if self.outcome.is_some() {
            return self.project(after_event_cursor, true);
        }
        let correlation_id = self
            .events
            .first()
            .map(|event| event.correlation_id.clone())
            .ok_or(RuntimeTransportError::RuntimeEvidenceDenied)?;
        self.cancellation.request(CancellationSignal {
            schema_version: self.request.schema_version,
            cancellation_id,
            correlation_id,
            task_id: self.request.task.task_id.clone(),
            reason: CancellationReason::UserRequested,
            requested_by: BoundaryKind::Shell,
        })?;
        if !self.busy {
            self.pending_approval = None;
            self.dispatch(None)?;
        }
        self.refresh(CONTROL_WAIT)?;
        self.project(after_event_cursor, true)
    }

    fn dispatch(
        &mut self,
        response: Option<RuntimeApprovalResponse>,
    ) -> Result<(), RuntimeTransportError> {
        if self.busy || self.outcome.is_some() {
            return Err(RuntimeTransportError::RequestDenied);
        }
        self.commands
            .send(WorkerCommand::Advance(response))
            .map_err(|_| RuntimeTransportError::RuntimeFailed)?;
        self.busy = true;
        Ok(())
    }

    fn refresh(&mut self, wait: Duration) -> Result<(), RuntimeTransportError> {
        let deadline = Instant::now() + wait;
        let boundary_deadline = deadline + START_WAIT;
        loop {
            let mut progressed = self.sync_events()?;
            match self.results.try_recv() {
                Ok(result) => {
                    progressed = true;
                    let WorkerResponse::Boundary(boundary) = result.map_err(map_client_error)?
                    else {
                        return Err(RuntimeTransportError::RuntimeEvidenceDenied);
                    };
                    self.accept_boundary(boundary)?;
                    self.sync_events()?;
                }
                Err(TryRecvError::Empty) => {}
                Err(TryRecvError::Disconnected) if self.outcome.is_none() => {
                    return Err(RuntimeTransportError::RuntimeFailed);
                }
                Err(TryRecvError::Disconnected) => {}
            }
            let boundary_visible = self.boundary_visible();
            if progressed && boundary_visible
                || Instant::now() >= deadline && (self.busy || boundary_visible)
            {
                return Ok(());
            }
            if Instant::now() >= boundary_deadline {
                eprintln!("coding.live.boundary-visibility-timeout");
                return Err(RuntimeTransportError::RuntimeEvidenceDenied);
            }
            let wait_until = if Instant::now() < deadline {
                deadline
            } else {
                boundary_deadline
            };
            thread::sleep(WAIT_SLICE.min(wait_until.saturating_duration_since(Instant::now())));
        }
    }

    fn sync_events(&mut self) -> Result<bool, RuntimeTransportError> {
        let (events, terminal, _disconnected) = self.event_pump.snapshot().map_err(|error| {
            eprintln!("coding.live.event-pump-denied");
            error
        })?;
        if events.len() < self.events.len() || events[..self.events.len()] != self.events {
            return Err(RuntimeTransportError::RuntimeEvidenceDenied);
        }
        let progressed = events.len() > self.events.len();
        self.events = events;
        self.event_stream_terminal = terminal;
        Ok(progressed)
    }

    fn boundary_visible(&self) -> bool {
        if self.outcome.is_some() {
            return self.event_stream_terminal;
        }
        let Some(challenge) = &self.pending_approval else {
            return true;
        };
        matches!(
            self.events.last(),
            Some(RuntimeEvent {
                kind: RuntimeEventKind::PermissionRequested {
                    approval_id,
                    preview_sha256,
                    expires_at_epoch_ms,
                    ..
                },
                ..
            }) if approval_id == &challenge.approval_id
                && preview_sha256 == &challenge.preview_sha256
                && expires_at_epoch_ms == &challenge.expires_at_epoch_ms
        )
    }

    fn accept_boundary(&mut self, boundary: WorkerBoundary) -> Result<(), RuntimeTransportError> {
        self.busy = false;
        self.artifacts = boundary.artifacts;
        match boundary.step {
            RuntimeCoordinatorStep::AwaitingApproval { challenge } => {
                verify_runtime_approval_challenge(&challenge)
                    .map_err(|_| RuntimeTransportError::RuntimeEvidenceDenied)?;
                if challenge.run_id != self.request.run_id
                    || challenge.task_id != self.request.task.task_id
                    || self.outcome.is_some()
                {
                    return Err(RuntimeTransportError::RuntimeEvidenceDenied);
                }
                self.pending_approval = Some(challenge);
            }
            RuntimeCoordinatorStep::Complete { outcome } => {
                verify_runtime_outcome(&outcome, &self.request)
                    .map_err(|_| RuntimeTransportError::RuntimeEvidenceDenied)?;
                self.pending_approval = None;
                self.outcome = Some(outcome);
            }
        }
        Ok(())
    }

    fn project(
        &mut self,
        after_event_cursor: Option<&RuntimeEventCursor>,
        record_presentation: bool,
    ) -> Result<RuntimeTransportStep, RuntimeTransportError> {
        if self.outcome.is_some() != self.event_stream_terminal
            || self.pending_approval.is_some() && self.outcome.is_some()
        {
            eprintln!("coding.live.terminal-binding-denied");
            return Err(RuntimeTransportError::RuntimeEvidenceDenied);
        }
        if self.outcome.is_some()
            && self.slow_subscriber_probe.is_some()
            && !self.slow_subscriber_verified
        {
            let subscriber = self
                .slow_subscriber_probe
                .as_ref()
                .ok_or(RuntimeTransportError::RuntimeEvidenceDenied)?;
            if subscriber
                .try_next()
                .map_err(|_| RuntimeTransportError::RuntimeEvidenceDenied)?
                .is_none()
                || subscriber.try_next().is_ok()
            {
                eprintln!("coding.live.slow-subscriber-not-disconnected");
                return Err(RuntimeTransportError::RuntimeEvidenceDenied);
            }
            self.slow_subscriber_verified = true;
            eprintln!("coding.live.slow-subscriber-disconnected");
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
            }) = self.events.last()
            else {
                eprintln!("coding.live.approval-event-denied");
                return Err(RuntimeTransportError::RuntimeEvidenceDenied);
            };
            if approval_id != &challenge.approval_id
                || preview_sha256 != &challenge.preview_sha256
                || expires_at_epoch_ms != &challenge.expires_at_epoch_ms
            {
                eprintln!("coding.live.approval-binding-denied");
                return Err(RuntimeTransportError::RuntimeEvidenceDenied);
            }
        }
        let visible_event_len = self
            .events
            .iter()
            .position(|event| {
                matches!(
                    &event.kind,
                    RuntimeEventKind::ArtifactCreated { artifact_id, .. }
                        if !self.artifacts.iter().any(|reference| reference.artifact_id == *artifact_id)
                )
            })
            .unwrap_or(self.events.len());
        let first = match after_event_cursor {
            None => 0,
            Some(cursor) => {
                let index = usize::try_from(cursor.sequence)
                    .map_err(|_| RuntimeTransportError::EventCursorDenied)?;
                if index < visible_event_len.saturating_sub(MAX_LIVE_CURSOR_AGE_EVENTS)
                    && self.latest_presented_cursor.as_ref() != Some(cursor)
                {
                    return Err(RuntimeTransportError::EventCursorExpired);
                }
                if index >= visible_event_len {
                    return Err(RuntimeTransportError::EventCursorDenied);
                }
                let event = self
                    .events
                    .get(index)
                    .ok_or(RuntimeTransportError::EventCursorDenied)?;
                if cursor.run_id != self.request.run_id
                    || cursor.event_id != event.event_id
                    || cursor.sequence != event.sequence
                    || cursor.event_sha256 != event.event_sha256
                {
                    return Err(RuntimeTransportError::EventCursorDenied);
                }
                index
                    .checked_add(1)
                    .ok_or(RuntimeTransportError::EventCursorDenied)?
            }
        };
        let events = self.events[first..visible_event_len].to_vec();
        if record_presentation {
            if let Some(event) = events.last() {
                self.latest_presented_cursor = Some(RuntimeEventCursor {
                    run_id: event.run_id.clone(),
                    event_id: event.event_id.clone(),
                    sequence: event.sequence,
                    event_sha256: event.event_sha256.clone(),
                });
            } else if let Some(cursor) = after_event_cursor {
                self.latest_presented_cursor = Some(cursor.clone());
            }
        }
        Ok(RuntimeTransportStep {
            run_id: self.request.run_id.clone(),
            request_sha256: self.request.request_sha256.clone(),
            events,
            artifacts: self.artifacts.clone(),
            approval: self.pending_approval.clone(),
            outcome: self.outcome.clone(),
        })
    }

    fn verify_binding(&self, request_sha256: &str) -> Result<(), RuntimeTransportError> {
        if self.request.request_sha256 == request_sha256 {
            Ok(())
        } else {
            Err(RuntimeTransportError::RequestDenied)
        }
    }

    const fn is_terminal(&self) -> bool {
        self.outcome.is_some()
    }

    fn read_artifact_page(
        &mut self,
        request_sha256: &str,
        reference: &RuntimeArtifactRef,
        offset: u64,
        maximum_bytes: u32,
    ) -> Result<RuntimeArtifactPage, RuntimeTransportError> {
        self.verify_binding(request_sha256)?;
        self.refresh(Duration::ZERO)?;
        if !self.is_terminal() || self.busy {
            return Err(RuntimeTransportError::RequestDenied);
        }
        self.commands
            .send(WorkerCommand::ReadArtifactPage {
                reference: reference.clone(),
                offset,
                maximum_bytes,
            })
            .map_err(|_| RuntimeTransportError::RuntimeFailed)?;
        match self.results.recv_timeout(START_WAIT) {
            Ok(Ok(WorkerResponse::ArtifactPage(page))) => Ok(page),
            Ok(Ok(_)) => Err(RuntimeTransportError::RuntimeEvidenceDenied),
            Ok(Err(error)) => Err(map_client_error(error)),
            Err(_) => Err(RuntimeTransportError::RuntimeFailed),
        }
    }

    fn release_artifact(
        &mut self,
        request_sha256: &str,
        reference: &RuntimeArtifactRef,
    ) -> Result<RuntimeArtifactState, RuntimeTransportError> {
        self.verify_binding(request_sha256)?;
        self.refresh(Duration::ZERO)?;
        if !self.is_terminal() || self.busy {
            return Err(RuntimeTransportError::RequestDenied);
        }
        self.commands
            .send(WorkerCommand::ReleaseArtifact(reference.clone()))
            .map_err(|_| RuntimeTransportError::RuntimeFailed)?;
        match self.results.recv_timeout(START_WAIT) {
            Ok(Ok(WorkerResponse::ArtifactState(state))) => Ok(state),
            Ok(Ok(_)) => Err(RuntimeTransportError::RuntimeEvidenceDenied),
            Ok(Err(error)) => Err(map_client_error(error)),
            Err(_) => Err(RuntimeTransportError::RuntimeFailed),
        }
    }
}

impl Drop for LiveCodingSession {
    fn drop(&mut self) {
        let _ = self.commands.try_send(WorkerCommand::Stop);
        if !self.busy
            && let Some(worker) = self.worker.take()
        {
            let _ = worker.join();
        }
    }
}

const fn map_client_error(error: CodingClientError) -> RuntimeTransportError {
    match error {
        CodingClientError::Runtime => RuntimeTransportError::RuntimeFailed,
        CodingClientError::EventStream
        | CodingClientError::Approval
        | CodingClientError::Presentation => RuntimeTransportError::RuntimeEvidenceDenied,
    }
}
