//! Live host scheduling for a reusable coding coordinator.
//!
//! Model and tool work runs on one owned worker. The authenticated IPC owner remains available to
//! drain bounded canonical events, validate cursors, present approvals, and set a cancellation
//! signal. This module adds no effect, model-selection, approval, or storage authority.
//!
//! When the composed runtime's store keeps durable job ledgers, the service is the owner of each
//! run's job (Decision 0120): it records the job's start and terminal outcome, decides each client
//! control request through the ledger under the client scope the IPC endpoint derived from its
//! authenticated peer, and stops work only for a cancellation the ledger applied.

use std::collections::BTreeMap;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
    mpsc::{Receiver, SyncSender, TryRecvError, sync_channel},
};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use agentmage_kernel_contracts::{
    AgentStateKind, BoundaryKind, CancellationId, CancellationReason, CancellationSignal,
    ModelCancellationProbe, ModelRuntimeFailure, RuntimeApprovalChallenge, RuntimeApprovalResponse,
    RuntimeArtifactRef, RuntimeEvent, RuntimeEventCursor, RuntimeEventKind, RuntimeOutcome,
    RuntimeRunId, RuntimeRunRequest, RuntimeSessionMode,
};
use agentmage_kernel_engine::{
    context_inspection::ContextInspection,
    job_control::{
        JobControlAction, JobControlDecision, JobControlError, JobControlRequest, JobOwnerEvent,
        JobPhase,
    },
    job_ledger_store::{DurableJobLedgers, JobLedgerStoreError},
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
use crate::coding_context::RunContextInspectionSource;
use crate::coding_recoverability::{RecoverabilityReport, RunRecoverabilitySource};
use crate::native_chat_runtime::NativeChatRuntimeFactory;
use crate::runtime_transport::{
    RuntimeClientScope, RuntimeJobControl, RuntimeJobStatus, RuntimePrepareInput,
    RuntimeRunDeclarations, RuntimeTransportError, RuntimeTransportPort, RuntimeTransportStep,
};

/// Owner identity of the coding host in every job ledger it keeps. The store
/// authenticates writers by its key, so every host over one store is the same
/// owner (Decisions 0119 and 0120).
pub const CODING_HOST_JOB_OWNER: &str = "agentmage-coding-host";

/// Declarations a live coordinator can make about its ended run (Decision 0116).
pub trait LiveRunDeclarationPort {
    /// Recoverability of the ended run's effects, or `None` while the run can
    /// still advance or when its effects cannot be declared completely.
    fn run_recoverability(&self, request: &RuntimeRunRequest) -> Option<RecoverabilityReport>;

    /// The view of every context composed in the ended run, or `None` while
    /// the run can still advance or when not every view was retained.
    fn run_context_inspections(&self) -> Option<Vec<ContextInspection>>;
}

impl<M, X, T, V, C> LiveRunDeclarationPort
    for agentmage_kernel_engine::runtime_loop::ReusableRuntimeCoordinator<M, X, T, V, C>
where
    M: agentmage_kernel_engine::runtime_loop::RuntimeModelPort,
    X: agentmage_kernel_engine::runtime_loop::RuntimeContextPort + RunContextInspectionSource,
    T: agentmage_kernel_engine::runtime_loop::RuntimeToolBoundary + RunRecoverabilitySource,
    V: agentmage_kernel_engine::runtime_loop::RuntimeVerifierPort,
    C: agentmage_kernel_engine::runtime_loop::RuntimeClock,
{
    fn run_recoverability(&self, request: &RuntimeRunRequest) -> Option<RecoverabilityReport> {
        self.ended_tool_boundary()?
            .declare_run_recoverability(request)
            .ok()
    }

    fn run_context_inspections(&self) -> Option<Vec<ContextInspection>> {
        self.ended_context_port()?
            .run_context_inspections()
            .map(<[ContextInspection]>::to_vec)
    }
}

/// The declarations of one ended run. A run resumed from an event cursor was
/// composed again after a restart, so its owners hold only what happened
/// since; both parts are unavailable rather than partial (review V1 of
/// `8fbd2bc6`, Decision 0117).
fn declare_run<R: LiveRunDeclarationPort>(
    runtime: &R,
    request: &RuntimeRunRequest,
) -> RuntimeRunDeclarations {
    let resumed = request.event_cursor.is_some();
    RuntimeRunDeclarations {
        schema_version: 1,
        run_id: request.run_id.clone(),
        request_sha256: request.request_sha256.clone(),
        recoverability: if resumed {
            None
        } else {
            runtime.run_recoverability(request)
        },
        context_inspections: if resumed {
            None
        } else {
            runtime.run_context_inspections()
        },
    }
}

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
    Declare,
    Stop,
}

struct WorkerBoundary {
    step: RuntimeCoordinatorStep,
    artifacts: Vec<RuntimeArtifactRef>,
}

// The synchronous channel has capacity one, so keeping the typed boundary
// inline cannot produce an unbounded queue of large values.
#[allow(clippy::large_enum_variant)]
enum WorkerResponse {
    Boundary(WorkerBoundary),
    ArtifactPage(RuntimeArtifactPage),
    ArtifactState(RuntimeArtifactState),
    Declarations(RuntimeRunDeclarations),
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
            // The same passive view is polled inside bounded native effects.
            // Contention/poison is unavailable control, never permission to
            // continue uncancelled or to wait outside the effect deadline.
            .try_lock()
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
    F::Coordinator: LiveCodingCoordinatorPort + LiveRunDeclarationPort,
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
    F::Coordinator: LiveCodingCoordinatorPort + LiveRunDeclarationPort,
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
    F::Coordinator: LiveCodingCoordinatorPort + LiveRunDeclarationPort,
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
        // A later composition or subscription failure cannot authorize another
        // attempt with this request. Invalid requests above leave it untouched.
        self.prepared.remove(&key);
        let coordinator = self.factory.compose_runtime(&request)?;
        let job_ledgers = self.factory.take_job_ledgers(&request.run_id);
        let mut session = LiveCodingSession::spawn(request, coordinator, slow_subscriber_probe)
            .inspect_err(|_| {
                eprintln!("coding.live.spawn-denied");
            })?;
        if let Some(ledgers) = job_ledgers {
            session.begin_job(ledgers).inspect_err(|_| {
                eprintln!("coding.live.job-start-denied");
            })?;
        }
        let step = session.start().inspect_err(|_| {
            eprintln!("coding.live.start-denied");
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

    fn run_declarations(
        &mut self,
        run_id: &RuntimeRunId,
        request_sha256: &str,
    ) -> Result<RuntimeRunDeclarations, RuntimeTransportError> {
        self.active
            .get_mut(run_id.as_str())
            .ok_or(RuntimeTransportError::RunUnavailable)?
            .run_declarations(request_sha256)
    }

    fn job_status(
        &mut self,
        run_id: &RuntimeRunId,
        request_sha256: &str,
    ) -> Result<RuntimeJobStatus, RuntimeTransportError> {
        self.active
            .get_mut(run_id.as_str())
            .ok_or(RuntimeTransportError::RunUnavailable)?
            .job_status(request_sha256)
    }

    fn control_job_for_client(
        &mut self,
        client: &RuntimeClientScope,
        run_id: &RuntimeRunId,
        request_sha256: &str,
        request: &JobControlRequest,
    ) -> Result<RuntimeJobControl, RuntimeTransportError> {
        self.active
            .get_mut(run_id.as_str())
            .ok_or(RuntimeTransportError::RunUnavailable)?
            .control_job(client, request_sha256, request)
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
    job: Option<LiveJob>,
}

/// The durable job of one live run, owned by this service (Decision 0120).
struct LiveJob {
    ledgers: DurableJobLedgers,
    /// Whether the terminal outcome is recorded.
    ended: bool,
    /// An owner observation could not be recorded, so the ledger no longer
    /// follows the run and is neither shown nor controlled.
    failed: bool,
}

impl LiveCodingSession {
    fn spawn<R>(
        request: RuntimeRunRequest,
        mut runtime: R,
        slow_subscriber_probe: bool,
    ) -> Result<Self, RuntimeTransportError>
    where
        R: LiveCodingCoordinatorPort + LiveRunDeclarationPort,
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
        let worker_request = request.clone();
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
                        WorkerCommand::Declare => Ok(WorkerResponse::Declarations(declare_run(
                            &runtime,
                            &worker_request,
                        ))),
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
            job: None,
        })
    }

    /// Records this run's job as started, before any work is dispatched. A run
    /// resumed after a host restart continues its running job; a job that is
    /// suspended, cancelling or ended is not continued here.
    fn begin_job(&mut self, ledgers: DurableJobLedgers) -> Result<(), RuntimeTransportError> {
        let job_id = self.request.run_id.as_str();
        let resumed = self.request.event_cursor.is_some();
        let observation = match ledgers.create(job_id, CODING_HOST_JOB_OWNER) {
            Ok(observation) => observation,
            Err(JobLedgerStoreError::Exists) if resumed => {
                ledgers.observation(job_id).map_err(map_ledger_error)?
            }
            Err(error) => return Err(map_ledger_error(error)),
        };
        match observation.phase {
            JobPhase::Queued => {
                ledgers
                    .observe_owner(job_id, CODING_HOST_JOB_OWNER, JobOwnerEvent::Started)
                    .map_err(map_ledger_error)?;
            }
            JobPhase::Running if resumed => {}
            _ => return Err(RuntimeTransportError::RequestDenied),
        }
        self.job = Some(LiveJob {
            ledgers,
            ended: false,
            failed: false,
        });
        Ok(())
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
        // Every cancellation of a job the ledger keeps is a control request
        // decided through it (Decision 0120).
        if self.job.is_some() {
            return Err(RuntimeTransportError::RequestDenied);
        }
        self.refresh(Duration::ZERO)?;
        let _ = self.project(after_event_cursor, false)?;
        if self.outcome.is_some() {
            return self.project(after_event_cursor, true);
        }
        self.signal_cancellation(cancellation_id)?;
        self.refresh(CONTROL_WAIT)?;
        self.project(after_event_cursor, true)
    }

    fn signal_cancellation(
        &mut self,
        cancellation_id: CancellationId,
    ) -> Result<(), RuntimeTransportError> {
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
        Ok(())
    }

    /// The job's replayed state, answered while the run is held.
    fn job_status(
        &mut self,
        request_sha256: &str,
    ) -> Result<RuntimeJobStatus, RuntimeTransportError> {
        self.verify_binding(request_sha256)?;
        self.refresh(Duration::ZERO)?;
        self.replayed_job_status()
    }

    /// Decides one client control request through the durable ledger. Only an
    /// applied cancellation stops work; suspension and resumption are refused
    /// before the ledger until the host can stop at a safe boundary and resume
    /// (AMR-04.6.4).
    fn control_job(
        &mut self,
        client: &RuntimeClientScope,
        request_sha256: &str,
        request: &JobControlRequest,
    ) -> Result<RuntimeJobControl, RuntimeTransportError> {
        self.verify_binding(request_sha256)?;
        // An outcome that already arrived is recorded before the request is decided.
        self.refresh(Duration::ZERO)?;
        let job = self.usable_job()?;
        if request.action != JobControlAction::Cancel {
            return Err(RuntimeTransportError::RequestDenied);
        }
        let decision = job
            .ledgers
            .control(self.request.run_id.as_str(), client.as_str(), request)
            .map_err(map_ledger_error)?;
        // A retry answers the original decision, and the one cancellation a
        // job can apply always yields the same signal.
        if let JobControlDecision::Applied {
            revision,
            phase: JobPhase::Cancelling,
        } = decision
            && self.outcome.is_none()
        {
            self.signal_cancellation(CancellationId::from_raw(format!(
                "coding-job-cancel-{revision}"
            )))?;
            self.refresh(CONTROL_WAIT)?;
        }
        Ok(RuntimeJobControl {
            decision,
            status: self.replayed_job_status()?,
        })
    }

    fn usable_job(&self) -> Result<&LiveJob, RuntimeTransportError> {
        self.job
            .as_ref()
            .filter(|job| !job.failed)
            .ok_or(RuntimeTransportError::JobControlUnavailable)
    }

    fn replayed_job_status(&self) -> Result<RuntimeJobStatus, RuntimeTransportError> {
        let job = self
            .usable_job()?
            .ledgers
            .observation(self.request.run_id.as_str())
            .map_err(map_ledger_error)?;
        Ok(RuntimeJobStatus {
            schema_version: 1,
            run_id: self.request.run_id.clone(),
            request_sha256: self.request.request_sha256.clone(),
            job,
        })
    }

    /// Records the run's verified outcome as the job's terminal owner
    /// observation (see [`job_outcome_event`]).
    fn record_job_outcome(&mut self) {
        let (Some(job), Some(outcome)) = (self.job.as_mut(), self.outcome.as_ref()) else {
            return;
        };
        if job.ended || job.failed {
            return;
        }
        let job_id = self.request.run_id.as_str();
        let recorded = job.ledgers.observation(job_id).and_then(|observation| {
            job.ledgers.observe_owner(
                job_id,
                CODING_HOST_JOB_OWNER,
                job_outcome_event(outcome.state, observation.phase),
            )
        });
        if recorded.is_ok() {
            job.ended = true;
        } else {
            eprintln!("coding.live.job-outcome-unrecorded");
            job.failed = true;
        }
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
            // Event and worker-result channels may arrive in either order. A
            // busy worker is not permission to present an unmatched terminal.
            // Keep the original bounded visibility deadline and project guard.
            let terminal_pair_visible = self.event_stream_terminal == self.outcome.is_some();
            if terminal_pair_visible
                && (progressed && boundary_visible
                    || Instant::now() >= deadline && (self.busy || boundary_visible))
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
        let (events, terminal, _disconnected) = self.event_pump.snapshot().inspect_err(|_| {
            eprintln!("coding.live.event-pump-denied");
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
                self.record_job_outcome();
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

    fn run_declarations(
        &mut self,
        request_sha256: &str,
    ) -> Result<RuntimeRunDeclarations, RuntimeTransportError> {
        self.verify_binding(request_sha256)?;
        self.refresh(Duration::ZERO)?;
        if !self.is_terminal() || self.busy {
            return Err(RuntimeTransportError::RequestDenied);
        }
        self.commands
            .send(WorkerCommand::Declare)
            .map_err(|_| RuntimeTransportError::RuntimeFailed)?;
        match self.results.recv_timeout(START_WAIT) {
            Ok(Ok(WorkerResponse::Declarations(declarations))) => Ok(declarations),
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

/// The terminal owner observation for a run's verified outcome: completed for
/// a success or no-op, whatever request is pending; cancellation observed for
/// a cancelled run whose cancellation the ledger applied; failed for any other
/// outcome, including a cancelled run without an applied request.
const fn job_outcome_event(state: AgentStateKind, phase: JobPhase) -> JobOwnerEvent {
    match (state, phase) {
        (AgentStateKind::Success | AgentStateKind::NoOp, _) => JobOwnerEvent::Completed,
        (AgentStateKind::Cancelled, JobPhase::Cancelling) => JobOwnerEvent::CancellationObserved,
        _ => JobOwnerEvent::Failed,
    }
}

const fn map_ledger_error(error: JobLedgerStoreError) -> RuntimeTransportError {
    match error {
        JobLedgerStoreError::Integrity => RuntimeTransportError::RuntimeEvidenceDenied,
        JobLedgerStoreError::Storage | JobLedgerStoreError::Unavailable => {
            RuntimeTransportError::JobControlUnavailable
        }
        JobLedgerStoreError::Ledger(JobControlError::Full) => {
            RuntimeTransportError::CapacityExceeded
        }
        JobLedgerStoreError::NotFound
        | JobLedgerStoreError::Exists
        | JobLedgerStoreError::NotOwner
        | JobLedgerStoreError::Ledger(_) => RuntimeTransportError::RequestDenied,
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

#[cfg(test)]
mod tests {
    use super::*;
    use agentmage_kernel_contracts::{CONTRACT_SCHEMA_VERSION, CorrelationId, TaskId};

    fn signal() -> CancellationSignal {
        CancellationSignal {
            schema_version: CONTRACT_SCHEMA_VERSION,
            cancellation_id: CancellationId::from_raw("live-owner-cancel"),
            correlation_id: CorrelationId::from_raw("live-owner-correlation"),
            task_id: TaskId::from_raw("live-owner-task"),
            reason: CancellationReason::UserRequested,
            requested_by: BoundaryKind::Shell,
        }
    }

    #[cfg(all(feature = "source-artifacts", feature = "workflow-supervisor"))]
    fn terminal_pair_fixture(
        terminal_event: bool,
    ) -> (
        LiveCodingSession,
        SyncSender<Result<WorkerResponse, CodingClientError>>,
        RuntimeOutcome,
    ) {
        let (request, mut events, outcome, observed_result) =
            crate::runtime_read_tests::completed_native_read_fixture();
        assert!(observed_result);
        assert!(
            !events
                .iter()
                .any(|event| matches!(event.kind, RuntimeEventKind::ArtifactCreated { .. })),
            "this bounded fixture has no omitted artifact references"
        );
        assert!(matches!(
            events.last().unwrap().kind,
            RuntimeEventKind::RunTerminal { .. }
        ));
        if !terminal_event {
            events.pop();
        }
        let mut sequence = RuntimeEventSequence::new();
        for event in &events {
            sequence.push(event).unwrap();
        }
        assert_eq!(sequence.is_terminal(), terminal_event);
        let (commands, _command_rx) = sync_channel(1);
        let (result_tx, results) = sync_channel(1);
        let session = LiveCodingSession {
            request,
            commands,
            results,
            event_pump: EventPump {
                state: Arc::new(Mutex::new(EventPumpState {
                    sequence,
                    events,
                    disconnected: false,
                    failed: false,
                })),
                stop: Arc::new(AtomicBool::new(false)),
                worker: None,
            },
            slow_subscriber_probe: None,
            slow_subscriber_verified: false,
            cancellation: Arc::new(SharedCancellation::new()),
            worker: None,
            events: Vec::new(),
            latest_presented_cursor: None,
            artifacts: Vec::new(),
            pending_approval: None,
            outcome: None,
            busy: true,
            event_stream_terminal: false,
            job: None,
        };
        (session, result_tx, outcome)
    }

    #[test]
    #[cfg(all(feature = "source-artifacts", feature = "workflow-supervisor"))]
    fn terminal_event_without_worker_outcome_cannot_escape_refresh_as_presentable() {
        let (mut session, result_tx, _outcome) = terminal_pair_fixture(true);
        // Keep the channel OPEN and deliberately withhold the outcome. This
        // deterministically represents the event-pump/worker-response ordering,
        // without timing sleeps or malformed event/request fixtures.
        assert_eq!(
            session.refresh(Duration::ZERO),
            Err(RuntimeTransportError::RuntimeEvidenceDenied)
        );
        assert!(session.busy && session.event_stream_terminal && session.outcome.is_none());
        assert!(matches!(
            session.project(None, true),
            Err(RuntimeTransportError::RuntimeEvidenceDenied)
        ));
        drop(result_tx);
    }

    #[test]
    #[cfg(all(feature = "source-artifacts", feature = "workflow-supervisor"))]
    fn worker_outcome_without_terminal_event_keeps_existing_bounded_refusal() {
        let (mut session, result_tx, outcome) = terminal_pair_fixture(false);
        result_tx
            .send(Ok(WorkerResponse::Boundary(WorkerBoundary {
                step: RuntimeCoordinatorStep::Complete { outcome },
                artifacts: Vec::new(),
            })))
            .unwrap();
        assert_eq!(
            session.refresh(Duration::ZERO),
            Err(RuntimeTransportError::RuntimeEvidenceDenied)
        );
        assert!(!session.busy && !session.event_stream_terminal && session.outcome.is_some());
        assert!(matches!(
            session.project(None, true),
            Err(RuntimeTransportError::RuntimeEvidenceDenied)
        ));
    }

    #[test]
    #[cfg(all(feature = "source-artifacts", feature = "workflow-supervisor"))]
    fn matching_terminal_event_and_verified_worker_outcome_are_presented_together() {
        let (mut session, result_tx, outcome) = terminal_pair_fixture(true);
        result_tx
            .send(Ok(WorkerResponse::Boundary(WorkerBoundary {
                step: RuntimeCoordinatorStep::Complete {
                    outcome: outcome.clone(),
                },
                artifacts: Vec::new(),
            })))
            .unwrap();
        session.refresh(Duration::ZERO).unwrap();
        assert!(!session.busy && session.event_stream_terminal);
        let step = session.project(None, true).unwrap();
        assert_eq!(step.outcome, Some(outcome));
        assert!(matches!(
            step.events.last().unwrap().kind,
            RuntimeEventKind::RunTerminal { .. }
        ));
    }

    #[test]
    #[cfg(all(
        target_os = "linux",
        feature = "source-artifacts",
        feature = "workflow-supervisor"
    ))]
    fn a_resumed_run_continues_only_its_running_job() {
        // Decision 0120: a run resumed after a host restart continues its job
        // when it is running; a new run never reuses a job, and a job whose
        // cancellation is pending or that has ended is not continued.
        let (_, events, _, _) = crate::runtime_read_tests::completed_native_read_fixture();
        let begin = |resumed: bool, ledgers: DurableJobLedgers| {
            let (mut session, _result_tx, _) = terminal_pair_fixture(false);
            if resumed {
                let first = events.first().unwrap();
                session.request.event_cursor = Some(RuntimeEventCursor {
                    run_id: first.run_id.clone(),
                    event_id: first.event_id.clone(),
                    sequence: first.sequence,
                    event_sha256: first.event_sha256.clone(),
                });
            }
            let job = session.request.run_id.as_str().to_owned();
            session.begin_job(ledgers).map(|()| job)
        };
        let job = events.first().unwrap().run_id.as_str().to_owned();
        let owner = CODING_HOST_JOB_OWNER;

        // No job yet: a new or resumed run starts one.
        for resumed in [false, true] {
            let store =
                crate::runtime_start_tests::JobLedgerStore::new(&format!("begin-{resumed}"));
            let ledgers = store.ledgers();
            assert_eq!(begin(resumed, ledgers.clone()), Ok(job.clone()));
            let observation = ledgers.observation(&job).unwrap();
            assert_eq!(
                (observation.phase, observation.revision),
                (JobPhase::Running, 1)
            );
        }

        // A running job continues without a second start, and a new run
        // never takes over an existing job.
        let store = crate::runtime_start_tests::JobLedgerStore::new("begin-running");
        let ledgers = store.ledgers();
        ledgers.create(&job, owner).unwrap();
        ledgers
            .observe_owner(&job, owner, JobOwnerEvent::Started)
            .unwrap();
        let running = ledgers.observation(&job).unwrap();
        assert_eq!(begin(true, ledgers.clone()), Ok(job.clone()));
        assert_eq!(ledgers.observation(&job).unwrap(), running);
        assert_eq!(
            begin(false, ledgers.clone()),
            Err(RuntimeTransportError::RequestDenied)
        );

        // A pending cancellation or an ended job is not continued.
        ledgers
            .control(
                &job,
                "peer-client-a",
                &JobControlRequest {
                    schema_version: 1,
                    job_id: job.clone(),
                    request_id: "cancel-1".to_owned(),
                    action: JobControlAction::Cancel,
                    observed_revision: 1,
                },
            )
            .unwrap();
        assert_eq!(
            begin(true, ledgers.clone()),
            Err(RuntimeTransportError::RequestDenied)
        );
        ledgers
            .observe_owner(&job, owner, JobOwnerEvent::Completed)
            .unwrap();
        let completed = ledgers.observation(&job).unwrap();
        assert_eq!(
            begin(true, ledgers.clone()),
            Err(RuntimeTransportError::RequestDenied)
        );
        assert_eq!(ledgers.observation(&job).unwrap(), completed);
    }

    struct CountingDeclarations(std::cell::Cell<usize>);

    impl LiveRunDeclarationPort for CountingDeclarations {
        fn run_recoverability(&self, request: &RuntimeRunRequest) -> Option<RecoverabilityReport> {
            self.0.set(self.0.get() + 1);
            crate::coding_recoverability::assess_run_recoverability(
                request.session_id.as_str(),
                request.task.task_id.as_str(),
                request.run_id.as_str(),
                &[],
                &|_| None,
            )
            .ok()
        }

        fn run_context_inspections(&self) -> Option<Vec<ContextInspection>> {
            self.0.set(self.0.get() + 1);
            Some(Vec::new())
        }
    }

    #[test]
    #[cfg(all(feature = "source-artifacts", feature = "workflow-supervisor"))]
    fn a_resumed_run_declares_neither_part_whatever_its_owners_hold() {
        // Review V1 of 8fbd2bc6: a run resumed from an event cursor was
        // composed again after a restart, so its owners' records begin there.
        let (mut request, events, _, _) =
            crate::runtime_read_tests::completed_native_read_fixture();
        let owners = CountingDeclarations(std::cell::Cell::new(0));
        let declared = declare_run(&owners, &request);
        assert_eq!(owners.0.get(), 2);
        assert!(declared.recoverability.is_some());
        assert_eq!(declared.context_inspections, Some(Vec::new()));
        let last = events.last().unwrap();
        request.event_cursor = Some(RuntimeEventCursor {
            run_id: request.run_id.clone(),
            event_id: last.event_id.clone(),
            sequence: last.sequence,
            event_sha256: last.event_sha256.clone(),
        });
        let declared = declare_run(&owners, &request);
        assert_eq!(owners.0.get(), 2);
        assert_eq!(declared.run_id, request.run_id);
        assert_eq!(declared.request_sha256, request.request_sha256);
        assert_eq!(declared.recoverability, None);
        assert_eq!(declared.context_inspections, None);
    }

    #[test]
    fn each_verified_outcome_ends_the_job_truthfully() {
        use agentmage_kernel_engine::job_control::JobControlLedger;
        for phase in [JobPhase::Running, JobPhase::Cancelling] {
            for state in [AgentStateKind::Success, AgentStateKind::NoOp] {
                assert_eq!(job_outcome_event(state, phase), JobOwnerEvent::Completed);
            }
            for state in [
                AgentStateKind::Declined,
                AgentStateKind::Blocked,
                AgentStateKind::Exhausted,
            ] {
                assert_eq!(job_outcome_event(state, phase), JobOwnerEvent::Failed);
            }
        }
        assert_eq!(
            job_outcome_event(AgentStateKind::Cancelled, JobPhase::Cancelling),
            JobOwnerEvent::CancellationObserved
        );
        // A cancelled run without an applied request is not reported as a
        // cancellation the ledger decided.
        assert_eq!(
            job_outcome_event(AgentStateKind::Cancelled, JobPhase::Running),
            JobOwnerEvent::Failed
        );
        // Every event is a transition the ledger accepts from that phase.
        for (phase, state) in [
            (JobPhase::Running, AgentStateKind::Success),
            (JobPhase::Running, AgentStateKind::Cancelled),
            (JobPhase::Cancelling, AgentStateKind::Cancelled),
            (JobPhase::Cancelling, AgentStateKind::Declined),
        ] {
            let mut ledger = JobControlLedger::create("job-outcome", "owner").unwrap();
            ledger.observe_owner(JobOwnerEvent::Started).unwrap();
            if phase == JobPhase::Cancelling {
                ledger
                    .control(
                        "peer-client",
                        &JobControlRequest {
                            schema_version: 1,
                            job_id: "job-outcome".to_owned(),
                            request_id: "cancel".to_owned(),
                            action: JobControlAction::Cancel,
                            observed_revision: 1,
                        },
                    )
                    .unwrap();
            }
            ledger
                .observe_owner(job_outcome_event(state, phase))
                .unwrap();
            assert!(ledger.observation().phase.is_terminal());
        }
    }

    fn unavailable(error: ModelRuntimeFailure) {
        assert_eq!(error.code, "coding.live.cancellation-unavailable");
        assert!(!error.retryable_after_correction);
        assert!(error.dependency_recovery_required);
        assert!(error.contract_error.is_none());
    }

    #[test]
    fn passive_live_cancellation_preserves_first_exact_signal() {
        let owner = SharedCancellation::new();
        assert!(owner.observe().unwrap().is_none());
        owner.request(signal()).unwrap();
        owner.request(signal()).unwrap();
        let mut conflicting = signal();
        conflicting.cancellation_id = CancellationId::from_raw("live-owner-other");
        assert!(matches!(
            owner.request(conflicting),
            Err(RuntimeTransportError::RequestDenied)
        ));
        assert_eq!(owner.observe().unwrap(), Some(signal()));
    }

    #[test]
    fn passive_live_cancellation_busy_owner_is_not_an_uncancelled_observation() {
        let owner = SharedCancellation::new();
        {
            let mut held = owner.signal.lock().unwrap();
            *held = Some(signal());
            unavailable(owner.observe().unwrap_err());
        }
        assert_eq!(owner.observe().unwrap(), Some(signal()));
    }

    #[test]
    fn passive_live_cancellation_poison_remains_unavailable() {
        let owner = SharedCancellation::new();
        let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let mut held = owner.signal.lock().unwrap();
            *held = Some(signal());
            panic!("synthetic cancellation owner poison");
        }));
        assert!(panic.is_err());
        unavailable(owner.observe().unwrap_err());
        unavailable(owner.observe().unwrap_err());
        assert!(owner.signal.is_poisoned());
        assert!(owner.request(signal()).is_err());
    }
}
