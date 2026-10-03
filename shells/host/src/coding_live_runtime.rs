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
//!
//! A suspension the ledger applied stops the run at its next committed safe boundary; the service
//! then records the owner's observation and releases the run's composition. A resumption or
//! cancellation of the suspended job continues the run inside this host through a new composition
//! bound to the boundary's cursor (Decision 0122).
//!
//! The factory that composed a run hands over the model route it chose, and the service declares
//! it with the run (Decision 0128).

use std::collections::{BTreeMap, BTreeSet};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
    mpsc::{Receiver, SyncSender, TryRecvError, sync_channel},
};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

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
    runtime_loop::{
        RuntimeCoordinatorStep, RuntimeSuspendableStep, RuntimeSuspensionPoint,
        RuntimeSuspensionProbe,
    },
};

use crate::coding_action_history::{
    DecidedJobControl, MAX_RUN_ACTION_ENTRIES, PersistedRunChain, RUN_ACTION_HISTORY_OWNER,
    RunActionChain, RunActionHistory, RunActionHistorySource, RunActionRecorder, StoredChainStart,
    job_control_draft,
};
use crate::coding_client::{CodingClientError, LiveCodingCoordinatorPort};
use crate::coding_context::RunContextInspectionSource;
use crate::coding_recoverability::{RecoverabilityReport, RunRecoverabilitySource};
use crate::coding_route::{RunRouteDeclaration, RunRouteReceipt};
use crate::coding_session_recoverability::PersistedRunEffects;
use crate::native_chat_runtime::NativeChatRuntimeFactory;
use crate::runtime_transport::{
    RUN_DECLARATIONS_SCHEMA_VERSION, RuntimeClientScope, RuntimeJobControl, RuntimeJobStatus,
    RuntimePrepareInput, RuntimeRunDeclarations, RuntimeTransportError, RuntimeTransportPort,
    RuntimeTransportStep, is_suspension_event, resumed_run_request,
};
use agentmage_kernel_engine::run_action_history_store::{
    DurableRunActionHistories, RunActionChainName, RunActionHistoryStoreError,
};
use agentmage_kernel_engine::run_effect_record_store::RunEffectRecordStoreError;

/// Owner identity of the coding host in every job ledger it keeps. The store
/// authenticates writers by its key, so every host over one store is the same
/// owner (Decisions 0119 and 0120).
pub const CODING_HOST_JOB_OWNER: &str = "agentmage-coding-host";

/// Declarations a live coordinator can make about its ended run (Decision 0116).
pub trait LiveRunDeclarationPort {
    /// Recoverability of the ended run's effects, or `None` while the run can
    /// still advance or when its effects cannot be declared completely.
    fn run_recoverability(&self, request: &RuntimeRunRequest) -> Option<RecoverabilityReport>;

    /// Recoverability of every effect of the ended run's session, from the
    /// stored records of each of its runs read at `now_epoch_ms`, or `None`
    /// while the run can still advance or when the session cannot be declared
    /// completely (Decision 0143).
    fn session_recoverability(
        &self,
        request: &RuntimeRunRequest,
        now_epoch_ms: u64,
    ) -> Option<RecoverabilityReport>;

    /// The view of every context composed in the ended run, or `None` while
    /// the run can still advance or when not every view was retained.
    fn run_context_inspections(&self) -> Option<Vec<ContextInspection>>;

    /// The action history the tool boundary kept for the ended run, or `None`
    /// while the run can still advance or when it could not keep every entry
    /// (Decision 0127).
    fn run_action_history(&self, request: &RuntimeRunRequest) -> Option<RunActionHistory>;
}

impl<M, X, T, V, C> LiveRunDeclarationPort
    for agentmage_kernel_engine::runtime_loop::ReusableRuntimeCoordinator<M, X, T, V, C>
where
    M: agentmage_kernel_engine::runtime_loop::RuntimeModelPort,
    X: agentmage_kernel_engine::runtime_loop::RuntimeContextPort + RunContextInspectionSource,
    T: agentmage_kernel_engine::runtime_loop::RuntimeToolBoundary
        + RunRecoverabilitySource
        + RunActionHistorySource,
    V: agentmage_kernel_engine::runtime_loop::RuntimeVerifierPort,
    C: agentmage_kernel_engine::runtime_loop::RuntimeClock,
{
    fn run_recoverability(&self, request: &RuntimeRunRequest) -> Option<RecoverabilityReport> {
        self.ended_tool_boundary()?
            .declare_run_recoverability(request)
            .ok()
    }

    fn session_recoverability(
        &self,
        request: &RuntimeRunRequest,
        now_epoch_ms: u64,
    ) -> Option<RecoverabilityReport> {
        self.ended_tool_boundary()?
            .declare_session_recoverability(request, now_epoch_ms)
            .ok()
    }

    fn run_context_inspections(&self) -> Option<Vec<ContextInspection>> {
        self.ended_context_port()?
            .run_context_inspections()
            .map(<[ContextInspection]>::to_vec)
    }

    fn run_action_history(&self, request: &RuntimeRunRequest) -> Option<RunActionHistory> {
        self.ended_tool_boundary()?
            .declare_run_action_history(request)
    }
}

/// The declarations of one ended run. A run resumed from an event cursor was
/// composed again after a restart, so its owners hold only what happened
/// since; every part they keep is unavailable rather than partial (review V1
/// of `8fbd2bc6`, Decisions 0117 and 0127). The session's recoverability
/// comes from the stored records of every run of the session instead, so a
/// run continued in this host declares it, and a run resumed after a restart,
/// whose stored record is marked incomplete, does not (Decision 0143). The
/// service adds the job control history it keeps itself and the route the
/// factory handed over (Decision 0128).
fn declare_run<R: LiveRunDeclarationPort>(
    runtime: &R,
    request: &RuntimeRunRequest,
) -> RuntimeRunDeclarations {
    let resumed = request.event_cursor.is_some();
    RuntimeRunDeclarations {
        schema_version: RUN_DECLARATIONS_SCHEMA_VERSION,
        run_id: request.run_id.clone(),
        request_sha256: request.request_sha256.clone(),
        recoverability: if resumed {
            None
        } else {
            runtime.run_recoverability(request)
        },
        session_recoverability: host_now_epoch_ms()
            .and_then(|now| runtime.session_recoverability(request, now)),
        context_inspections: if resumed {
            None
        } else {
            runtime.run_context_inspections()
        },
        effect_history: if resumed {
            None
        } else {
            runtime.run_action_history(request)
        },
        job_control_history: None,
        route_receipt: None,
        route_history: None,
        recipe_plan: None,
    }
}

const MAX_LIVE_RUNS: usize = 4;

/// The host clock in Unix epoch milliseconds, for job control history entries.
fn host_now_epoch_ms() -> Option<u64> {
    let elapsed = SystemTime::now().duration_since(UNIX_EPOCH).ok()?;
    u64::try_from(elapsed.as_millis()).ok()
}
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
    step: RuntimeSuspendableStep,
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

impl EventPump {
    fn shutdown(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

impl Drop for EventPump {
    fn drop(&mut self) {
        self.shutdown();
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

/// Where a run's suspension stands between the service and its worker
/// (Decision 0122).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SuspensionState {
    /// No suspension is requested.
    Idle,
    /// The ledger applied a suspension; the worker may stop at its next
    /// committed safe boundary.
    Requested,
    /// The service is deciding a request that could withdraw or cancel the
    /// suspension, so the worker may not stop for it meanwhile.
    Withheld,
    /// The worker stopped for it at a boundary the service has not yet seen.
    Claimed,
}

/// The suspension a worker observes. It stops only for a requested
/// suspension, and never for one the service withheld while deciding a
/// request that could withdraw or cancel it, so a run never stops for a
/// suspension the ledger no longer asks for.
struct SharedSuspension {
    state: Mutex<SuspensionState>,
}

impl SharedSuspension {
    const fn new() -> Self {
        Self {
            state: Mutex::new(SuspensionState::Idle),
        }
    }

    /// Withholds a requested suspension; answers the state found.
    fn withhold(&self) -> Result<SuspensionState, RuntimeTransportError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| RuntimeTransportError::RuntimeFailed)?;
        let found = *state;
        if found == SuspensionState::Requested {
            *state = SuspensionState::Withheld;
        }
        Ok(found)
    }

    /// Follows the job's phase after a decision: a suspending job's worker may
    /// stop at its next boundary, and any other phase withdraws that. A
    /// claimed suspension stays claimed; its boundary is handled on arrival.
    fn follow(&self, phase: JobPhase) -> Result<(), RuntimeTransportError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| RuntimeTransportError::RuntimeFailed)?;
        *state = match (*state, phase) {
            (SuspensionState::Claimed, _) => SuspensionState::Claimed,
            (_, JobPhase::Suspending) => SuspensionState::Requested,
            _ => SuspensionState::Idle,
        };
        Ok(())
    }

    /// Settles the suspension after a decision: it follows the job's phase
    /// when that was read. Without the phase (review F1 of `3c69304c`), an
    /// applied decision that left the job anything but suspending withdrew
    /// the suspension, so it is withdrawn. A refusal or a failed write changed
    /// nothing, so a withheld suspension returns to the worker. Without the
    /// phase no decision requests a suspension, because a retry answers with
    /// the decision its request was first given.
    fn settle(
        &self,
        phase: Option<JobPhase>,
        decision: Option<&JobControlDecision>,
    ) -> Result<(), RuntimeTransportError> {
        match (phase, decision) {
            (Some(phase), _) => self.follow(phase),
            (None, Some(JobControlDecision::Applied { phase, .. }))
                if *phase != JobPhase::Suspending =>
            {
                self.follow(*phase)
            }
            (None, _) => self.restore(),
        }
    }

    /// Returns a withheld suspension to the worker.
    fn restore(&self) -> Result<(), RuntimeTransportError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| RuntimeTransportError::RuntimeFailed)?;
        if *state == SuspensionState::Withheld {
            *state = SuspensionState::Requested;
        }
        Ok(())
    }

    /// Whether the worker stopped for the requested suspension.
    fn claimed(&self) -> Result<bool, RuntimeTransportError> {
        self.state
            .lock()
            .map(|state| *state == SuspensionState::Claimed)
            .map_err(|_| RuntimeTransportError::RuntimeFailed)
    }

    /// Clears the state once the run is suspended and its worker has ended.
    fn clear(&self) -> Result<(), RuntimeTransportError> {
        *self
            .state
            .lock()
            .map_err(|_| RuntimeTransportError::RuntimeFailed)? = SuspensionState::Idle;
        Ok(())
    }
}

impl RuntimeSuspensionProbe for SharedSuspension {
    fn suspend_at_safe_boundary(&self) -> bool {
        // A poisoned state never invents a suspension; the run continues.
        let Ok(mut state) = self.state.lock() else {
            return false;
        };
        if *state == SuspensionState::Requested {
            *state = SuspensionState::Claimed;
            true
        } else {
            false
        }
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

    #[cfg(test)]
    pub(crate) const fn factory_for_tests(&self) -> &F {
        &self.factory
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
        ) || !crate::coding_recipe::request_binds_recipe(&request, input.recipe.as_ref())
            || request.event_cursor.is_some() != input.resume
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
        let route = self.factory.take_route_declaration(&request.run_id);
        let recipe_plan = self.factory.take_recipe_plan(&request.run_id);
        let histories = self.factory.take_run_action_histories(&request.run_id);
        let effect_records = self.factory.take_run_effect_records(&request.run_id);
        // A run started from an event cursor here was resumed after a restart;
        // a run continued in this host goes through `continue_from_checkpoint`.
        let start = if request.event_cursor.is_none() {
            StoredChainStart::New
        } else {
            StoredChainStart::AfterRestart
        };
        // A new run that fails before its session owns its stored chains ran
        // nothing, so they close empty rather than stay open in its session.
        // A run resumed after a restart keeps them open, already marked
        // incomplete, so a later host can still resume it (Decision 0145).
        let mut session =
            match LiveCodingSession::spawn(request, coordinator, slow_subscriber_probe) {
                Ok(session) => session,
                Err(error) => {
                    eprintln!("coding.live.spawn-denied");
                    if start == StoredChainStart::New {
                        close_stored_chains(&key, histories.as_ref(), effect_records.as_ref());
                    }
                    return Err(error);
                }
            };
        session.route = route;
        session.recipe_plan = recipe_plan;
        session.run_effect_records = effect_records;
        if let Some(histories) = histories {
            session.attach_run_histories(histories, start);
        }
        if let Some(ledgers) = job_ledgers
            && let Err(error) = session.begin_job(ledgers)
        {
            eprintln!("coding.live.job-start-denied");
            if start == StoredChainStart::New {
                session.close_run_histories();
            }
            return Err(error);
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
        let session = self
            .active
            .get_mut(run_id.as_str())
            .ok_or(RuntimeTransportError::RunUnavailable)?;
        match session.control_job(client, request_sha256, request)? {
            SessionControl::Answered(answer) => Ok(answer),
            SessionControl::Continue {
                decision,
                cancellation,
            } => {
                self.continue_from_checkpoint(run_id.as_str(), cancellation)
                    .inspect_err(|_| eprintln!("coding.live.resume-denied"))?;
                let status = self
                    .active
                    .get(run_id.as_str())
                    .ok_or(RuntimeTransportError::RunUnavailable)?
                    .replayed_job_status()?;
                Ok(RuntimeJobControl { decision, status })
            }
        }
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
        // Dropping the released, ended session closes its stored chains
        // (Decision 0129).
        self.active.remove(key);
        Ok(())
    }
}

impl<F> LiveCodingRuntimeService<F>
where
    F: NativeChatRuntimeFactory,
    F::Coordinator: LiveCodingCoordinatorPort + LiveRunDeclarationPort,
{
    /// Continues a suspended run from its committed checkpoint inside this host
    /// after its job was resumed or cancelled (Decision 0122). The held run's
    /// store handle closes first, because the store admits one connection; the
    /// factory then prepares the same run bound to the boundary's cursor and
    /// composes it afresh. The restored history must be exactly what this host
    /// already presented. A cancelled job's run ends at once with its own
    /// canonical outcome. If any step fails the run is no longer held, and the
    /// job keeps the phase the ledger decided.
    fn continue_from_checkpoint(
        &mut self,
        key: &str,
        cancellation: Option<u64>,
    ) -> Result<(), RuntimeTransportError> {
        let mut held = self
            .active
            .remove(key)
            .ok_or(RuntimeTransportError::RunUnavailable)?;
        let point = held
            .suspended
            .clone()
            .ok_or(RuntimeTransportError::RequestDenied)?;
        let request = held.request.clone();
        let presented = std::mem::take(&mut held.events);
        // The same service decided every control request of this run, so its
        // job control history continues in the resumed session (Decision 0127).
        let mut job_actions = std::mem::take(&mut held.job_actions);
        let recorded_controls = std::mem::take(&mut held.recorded_controls);
        let job_history_from_start = held.job_history_from_start;
        // What this host knows about the run's effect record outlives the
        // store handle, so a miss the store has not marked carries over
        // (Decision 0145).
        let effect_knowledge = held
            .run_effect_records
            .as_ref()
            .map(PersistedRunEffects::knowledge);
        // Every handle of the held run's store closes before it is opened
        // again for the continuation (Decision 0129).
        job_actions.detach_store();
        drop(held);
        let expected = resumed_run_request(&request, &point.event_cursor)?;
        let resumed = self
            .factory
            .prepare_in_host_resume(&request, &point.event_cursor)?;
        if resumed != expected {
            return Err(RuntimeTransportError::RequestDenied);
        }
        let coordinator = self.factory.compose_runtime(&resumed)?;
        let ledgers = self
            .factory
            .take_job_ledgers(&resumed.run_id)
            .ok_or(RuntimeTransportError::JobControlUnavailable)?;
        let route = self.factory.take_route_declaration(&resumed.run_id);
        let recipe_plan = self.factory.take_recipe_plan(&resumed.run_id);
        let histories = self.factory.take_run_action_histories(&resumed.run_id);
        let effect_records = self.factory.take_run_effect_records(&resumed.run_id);
        if let (Some(chain), Some(earlier)) = (&effect_records, &effect_knowledge) {
            chain.continue_from(earlier);
        }
        let mut session = LiveCodingSession::spawn(resumed, coordinator, false)?;
        session.route = route;
        session.recipe_plan = recipe_plan;
        session.run_effect_records = effect_records;
        session.job_actions = job_actions;
        session.recorded_controls = recorded_controls;
        session.job_history_from_start = job_history_from_start;
        if let Some(histories) = histories {
            session.attach_run_histories(histories, StoredChainStart::ContinueInHost);
        }
        session.sync_events()?;
        if session.events != presented {
            return Err(RuntimeTransportError::RuntimeEvidenceDenied);
        }
        session.latest_presented_cursor = Some(point.event_cursor);
        session.continue_job(ledgers, cancellation)?;
        self.active.insert(key.to_owned(), session);
        Ok(())
    }
}

/// Closes every stored chain of one run. A chain the store does not hold, or
/// one already closed, needs nothing. An effect record that missed an entry
/// stays open until the store holds its incomplete mark (Decision 0145).
fn close_stored_chains(
    run_id: &str,
    histories: Option<&DurableRunActionHistories>,
    effect_records: Option<&PersistedRunEffects>,
) {
    if let Some(histories) = histories {
        for chain in RunActionChainName::ALL {
            match histories.close(run_id, chain, RUN_ACTION_HISTORY_OWNER) {
                Ok(_) | Err(RunActionHistoryStoreError::NotFound) => {}
                Err(_) => eprintln!("coding.live.history-close-failed"),
            }
        }
    }
    if let Some(chain) = effect_records {
        match chain.close_released() {
            Ok(()) | Err(RunEffectRecordStoreError::NotFound) => {}
            Err(_) => eprintln!("coding.live.effect-record-close-failed"),
        }
    }
}

/// What the service does after a session decided a control request.
enum SessionControl {
    /// The answer is complete.
    Answered(RuntimeJobControl),
    /// The suspended run's job was resumed, or cancelled at the given
    /// revision: the run continues from its checkpoint before the answer.
    Continue {
        decision: JobControlDecision,
        cancellation: Option<u64>,
    },
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
    suspension: Arc<SharedSuspension>,
    /// Whether the coordinator commits checkpoints, so it can suspend.
    suspendable: bool,
    /// The worker stopped for a suspension at this boundary, whose commit
    /// event has not yet been seen.
    pending_suspension: Option<RuntimeSuspensionPoint>,
    /// The run is suspended here and its composition is released.
    suspended: Option<RuntimeSuspensionPoint>,
    /// The run's job control history: one entry for each control request the
    /// ledger decided (Decision 0127).
    job_actions: RunActionRecorder,
    /// Client scope and request identity of each recorded decision, so a
    /// retry that the ledger answers with its first decision adds nothing.
    recorded_controls: BTreeSet<(String, String)>,
    /// Whether this service decided every control request of the run: false
    /// for a run resumed after a host restart.
    job_history_from_start: bool,
    /// The model route the factory chose for this composition (Decision 0128).
    route: Option<RunRouteDeclaration>,
    /// The recipe plan the factory held this run's writes to (Decision 0133).
    recipe_plan: Option<agentmage_kernel_engine::engineering_recipe::RecipePlan>,
    /// The run's stored action histories, closed when the run is released
    /// (Decision 0129).
    run_histories: Option<DurableRunActionHistories>,
    /// The run's stored effect record, closed with its action histories
    /// (Decisions 0143 and 0145).
    run_effect_records: Option<PersistedRunEffects>,
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

impl LiveJob {
    const fn new(ledgers: DurableJobLedgers) -> Self {
        Self {
            ledgers,
            ended: false,
            failed: false,
        }
    }
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
        let suspendable = runtime.suspendable();
        let job_history_from_start = request.event_cursor.is_none();
        let slow_subscriber_probe = slow_subscriber_probe
            .then(|| runtime.subscribe_live_events(1).map_err(map_client_error))
            .transpose()?;
        let subscription = runtime
            .subscribe_live_events(capacity)
            .map_err(map_client_error)?;
        let event_pump = EventPump::spawn(subscription, &request, initial_events)?;
        let cancellation = Arc::new(SharedCancellation::new());
        let worker_cancellation = Arc::clone(&cancellation);
        let suspension = Arc::new(SharedSuspension::new());
        let worker_suspension = Arc::clone(&suspension);
        let worker_request = request.clone();
        let (command_tx, command_rx) = sync_channel::<WorkerCommand>(1);
        let (result_tx, result_rx) = sync_channel(1);
        let worker = thread::Builder::new()
            .name("agentmage-coding-runtime".to_owned())
            .spawn(move || {
                while let Ok(command) = command_rx.recv() {
                    let result = match command {
                        WorkerCommand::Advance(response) => runtime
                            .advance_or_suspend(
                                response.as_ref(),
                                Some(worker_cancellation.as_ref()),
                                worker_suspension.as_ref(),
                            )
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
            suspension,
            suspendable,
            pending_suspension: None,
            suspended: None,
            job_actions: RunActionRecorder::new(),
            recorded_controls: BTreeSet::new(),
            job_history_from_start,
            route: None,
            recipe_plan: None,
            run_histories: None,
            run_effect_records: None,
        })
    }

    /// Appends this run's job control entries to its stored chain as well
    /// (Decision 0129). A chain that cannot be begun leaves the history
    /// incomplete; the run itself continues.
    fn attach_run_histories(
        &mut self,
        histories: DurableRunActionHistories,
        start: StoredChainStart,
    ) {
        match PersistedRunChain::begin(
            histories.clone(),
            self.request.run_id.as_str(),
            RunActionChain::JobControl,
            start,
        ) {
            Ok(chain) => self.job_actions.attach_store(chain),
            Err(_) => {
                eprintln!("coding.live.history-store-unavailable");
                self.job_actions.mark_incomplete();
            }
        }
        self.run_histories = Some(histories);
    }

    /// Closes every stored chain of this ended run, its effect record
    /// included (Decision 0143).
    fn close_run_histories(&self) {
        close_stored_chains(
            self.request.run_id.as_str(),
            self.run_histories.as_ref(),
            self.run_effect_records.as_ref(),
        );
    }

    /// Records this run's job as started, before any work is dispatched
    /// (Decision 0120). Each rule stands alone (review F2 of `8a3a341e`):
    /// - a new run creates its job, so a run never takes over an existing job
    ///   in any phase;
    /// - a run resumed after a host restart starts its job when it is queued,
    ///   continues it when it is running, and refuses every other phase.
    fn begin_job(&mut self, ledgers: DurableJobLedgers) -> Result<(), RuntimeTransportError> {
        let job_id = self.request.run_id.as_str();
        let phase = if self.request.event_cursor.is_none() {
            ledgers
                .create(job_id, CODING_HOST_JOB_OWNER)
                .map_err(map_ledger_error)?
                .phase
        } else {
            match ledgers.create(job_id, CODING_HOST_JOB_OWNER) {
                Ok(observation) => observation.phase,
                Err(JobLedgerStoreError::Exists) => {
                    match ledgers.observation(job_id).map_err(map_ledger_error)?.phase {
                        JobPhase::Running => {
                            self.job = Some(LiveJob::new(ledgers));
                            return Ok(());
                        }
                        phase => phase,
                    }
                }
                Err(error) => return Err(map_ledger_error(error)),
            }
        };
        if phase != JobPhase::Queued {
            return Err(RuntimeTransportError::RequestDenied);
        }
        ledgers
            .observe_owner(job_id, CODING_HOST_JOB_OWNER, JobOwnerEvent::Started)
            .map_err(map_ledger_error)?;
        self.job = Some(LiveJob::new(ledgers));
        Ok(())
    }

    /// Takes over the job of a run continued from its checkpoint inside this
    /// host (Decision 0122) and dispatches the continuation. A resumed job is
    /// queued and is started here. A job cancelled while suspended already has
    /// its terminal decision, so the run only ends with its own outcome, which
    /// is not recorded again.
    fn continue_job(
        &mut self,
        ledgers: DurableJobLedgers,
        cancellation: Option<u64>,
    ) -> Result<(), RuntimeTransportError> {
        let job_id = self.request.run_id.as_str();
        let phase = ledgers.observation(job_id).map_err(map_ledger_error)?.phase;
        match (cancellation, phase) {
            (None, JobPhase::Queued) => {
                ledgers
                    .observe_owner(job_id, CODING_HOST_JOB_OWNER, JobOwnerEvent::Started)
                    .map_err(map_ledger_error)?;
                self.job = Some(LiveJob::new(ledgers));
                self.dispatch(None)?;
            }
            (Some(revision), JobPhase::Cancelled) => {
                let mut job = LiveJob::new(ledgers);
                job.ended = true;
                self.job = Some(job);
                self.signal_cancellation(CancellationId::from_raw(format!(
                    "coding-job-cancel-{revision}"
                )))?;
            }
            _ => return Err(RuntimeTransportError::RequestDenied),
        }
        self.refresh(CONTROL_WAIT)
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

    /// Decides one client control request through the durable ledger and then
    /// acts on the job's phase, never on the decision alone, so a retry that
    /// answers an earlier decision changes nothing that has since moved on:
    /// - a cancelling job's run gets its cancellation signal, and it stops at
    ///   its next cancellation check (Decision 0120);
    /// - a suspending job's run may stop at its next committed safe boundary;
    ///   any other phase withdraws that (Decision 0122);
    /// - a suspended run whose job is queued again or cancelled continues from
    ///   its checkpoint, which the service does before it answers.
    ///
    /// A run that commits no checkpoints refuses suspension and resumption
    /// before the ledger, so they are not recorded.
    fn control_job(
        &mut self,
        client: &RuntimeClientScope,
        request_sha256: &str,
        request: &JobControlRequest,
    ) -> Result<SessionControl, RuntimeTransportError> {
        self.verify_binding(request_sha256)?;
        // An outcome or a suspension that already arrived is recorded before
        // the request is decided.
        self.refresh(Duration::ZERO)?;
        self.usable_job()?;
        if request.action != JobControlAction::Cancel && !self.suspendable {
            return Err(RuntimeTransportError::RequestDenied);
        }
        // A request that could withdraw or cancel a suspension is decided
        // while the worker may not stop for it. If the worker already stopped,
        // its boundary is recorded first and the request is decided after it.
        let live = self.suspended.is_none() && self.outcome.is_none();
        if live
            && request.action != JobControlAction::Suspend
            && self.suspension.withhold()? == SuspensionState::Claimed
        {
            self.await_suspension()?;
        }
        let decided = self
            .usable_job()?
            .ledgers
            .control(self.request.run_id.as_str(), client.as_str(), request)
            .map_err(map_ledger_error);
        let status = self.replayed_job_status();
        if live && self.suspended.is_none() {
            self.suspension.settle(
                status.as_ref().ok().map(|status| status.job.phase),
                decided.as_ref().ok(),
            )?;
        }
        let decision = decided?;
        self.record_job_control(client, request, decision, status.as_ref().ok());
        let status = status?;
        match status.job.phase {
            // No client decision moves a cancelling job, so its revision names
            // the one cancellation the ledger applied, and every answer about
            // it yields the same signal.
            JobPhase::Cancelling if self.outcome.is_none() && self.suspended.is_none() => {
                self.signal_cancellation(CancellationId::from_raw(format!(
                    "coding-job-cancel-{}",
                    status.job.revision
                )))?;
                self.refresh(CONTROL_WAIT)?;
            }
            JobPhase::Queued if self.suspended.is_some() => {
                return Ok(SessionControl::Continue {
                    decision,
                    cancellation: None,
                });
            }
            JobPhase::Cancelled if self.suspended.is_some() => {
                return Ok(SessionControl::Continue {
                    decision,
                    cancellation: Some(status.job.revision),
                });
            }
            _ => {}
        }
        Ok(SessionControl::Answered(RuntimeJobControl {
            decision,
            status: self.replayed_job_status()?,
        }))
    }

    /// Keeps one decided control request in the run's job control history
    /// (Decision 0127). A retry the ledger answered with its first decision
    /// adds nothing; a decision whose resulting ledger head could not be read,
    /// or whose time could not be taken, leaves the history undeclared.
    fn record_job_control(
        &mut self,
        client: &RuntimeClientScope,
        request: &JobControlRequest,
        decision: JobControlDecision,
        status: Option<&RuntimeJobStatus>,
    ) {
        let key = (client.as_str().to_owned(), request.request_id.clone());
        if self.recorded_controls.contains(&key) {
            return;
        }
        let Some(status) = status else {
            self.job_actions.mark_incomplete();
            return;
        };
        if self.recorded_controls.len() >= MAX_RUN_ACTION_ENTRIES {
            self.job_actions.mark_incomplete();
            return;
        }
        self.recorded_controls.insert(key);
        let draft = host_now_epoch_ms().and_then(|now| {
            job_control_draft(&DecidedJobControl {
                client_scope: client.as_str(),
                request,
                decision,
                ledger_head_sha256: &status.job.head_sha256,
                decided_at_epoch_ms: now,
            })
        });
        self.job_actions.record(draft);
    }

    /// The job control history of this run. A run resumed after a host restart
    /// holds only this host's decisions, and a run without a job has none to
    /// declare, so neither declares one. A run resumed in this host keeps the
    /// history its session carried over.
    fn declare_job_control_history(&self) -> Option<RunActionHistory> {
        if !self.job_history_from_start || self.job.is_none() {
            return None;
        }
        self.job_actions.declare()
    }

    /// The receipt and route history of this composition's model route. A run
    /// resumed from an event cursor was composed again, so, like its other
    /// owners' parts, its route is not declared (Decision 0128).
    fn declare_route(&self) -> (Option<RunRouteReceipt>, Option<RunActionHistory>) {
        match &self.route {
            Some(route) if self.request.event_cursor.is_none() => {
                (Some(route.receipt.clone()), route.history.clone())
            }
            _ => (None, None),
        }
    }

    /// Waits, within the boundary bound, until a suspension the worker
    /// claimed is recorded (Decision 0122).
    fn await_suspension(&mut self) -> Result<(), RuntimeTransportError> {
        let deadline = Instant::now() + START_WAIT;
        while self.suspended.is_none() {
            if Instant::now() >= deadline {
                eprintln!("coding.live.suspension-unobserved");
                return Err(RuntimeTransportError::RuntimeEvidenceDenied);
            }
            self.refresh(CONTROL_WAIT)?;
        }
        Ok(())
    }

    /// Records the owner's observation of a suspension whose commit event is
    /// now the last presented event, then releases the run's composition: the
    /// worker ends and the coordinator's store, model and tools close with it.
    /// The job's ledger handle stays open for the suspended job's control.
    fn finish_suspension(&mut self) -> Result<(), RuntimeTransportError> {
        let point = self
            .pending_suspension
            .take()
            .ok_or(RuntimeTransportError::RuntimeEvidenceDenied)?;
        if !is_suspension_event(self.events.last(), &point) {
            return Err(RuntimeTransportError::RuntimeEvidenceDenied);
        }
        let job_id = self.request.run_id.as_str();
        let job = self
            .job
            .as_mut()
            .filter(|job| !job.failed)
            .ok_or(RuntimeTransportError::RuntimeEvidenceDenied)?;
        if job
            .ledgers
            .observe_owner(
                job_id,
                CODING_HOST_JOB_OWNER,
                JobOwnerEvent::SuspensionObserved,
            )
            .is_err()
        {
            eprintln!("coding.live.job-suspension-unrecorded");
            job.failed = true;
            return Err(RuntimeTransportError::RuntimeEvidenceDenied);
        }
        let _ = self.commands.try_send(WorkerCommand::Stop);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
        self.event_pump.shutdown();
        self.suspension.clear()?;
        self.suspended = Some(point);
        Ok(())
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
        if self.busy
            || self.outcome.is_some()
            || self.pending_suspension.is_some()
            || self.suspended.is_some()
        {
            return Err(RuntimeTransportError::RequestDenied);
        }
        self.commands
            .send(WorkerCommand::Advance(response))
            .map_err(|_| RuntimeTransportError::RuntimeFailed)?;
        self.busy = true;
        Ok(())
    }

    fn refresh(&mut self, wait: Duration) -> Result<(), RuntimeTransportError> {
        // A suspended run's composition is released; nothing can arrive.
        if self.suspended.is_some() {
            return Ok(());
        }
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
                if self.pending_suspension.is_some() {
                    self.finish_suspension()?;
                }
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
        if let Some(point) = &self.pending_suspension {
            return is_suspension_event(self.events.last(), point);
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
            RuntimeSuspendableStep::Boundary(RuntimeCoordinatorStep::AwaitingApproval {
                challenge,
            }) => {
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
            RuntimeSuspendableStep::Boundary(RuntimeCoordinatorStep::Complete { outcome }) => {
                verify_runtime_outcome(&outcome, &self.request)
                    .map_err(|_| RuntimeTransportError::RuntimeEvidenceDenied)?;
                self.pending_approval = None;
                self.outcome = Some(outcome);
                self.record_job_outcome();
            }
            // The worker stops only for the suspension it claimed. Its commit
            // event is recorded once the event pump delivers it.
            RuntimeSuspendableStep::Suspended { point } => {
                if self.outcome.is_some()
                    || self.pending_approval.is_some()
                    || point.event_cursor.run_id != self.request.run_id
                    || !self.suspension.claimed()?
                {
                    return Err(RuntimeTransportError::RuntimeEvidenceDenied);
                }
                self.pending_suspension = Some(point);
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
            suspended: self.suspended.clone(),
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
            Ok(Ok(WorkerResponse::Declarations(mut declarations))) => {
                declarations.job_control_history = self.declare_job_control_history();
                (declarations.route_receipt, declarations.route_history) = self.declare_route();
                declarations.recipe_plan.clone_from(&self.recipe_plan);
                Ok(declarations)
            }
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
        // An ended run's stored chains close when its session is dropped:
        // when the client releases it, or when the host ends while holding
        // it. A run still running stays open, so its chains say so.
        if self.is_terminal() {
            self.close_run_histories();
        }
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
            suspension: Arc::new(SharedSuspension::new()),
            suspendable: false,
            pending_suspension: None,
            suspended: None,
            job_actions: RunActionRecorder::new(),
            recorded_controls: BTreeSet::new(),
            job_history_from_start: true,
            route: None,
            recipe_plan: None,
            run_histories: None,
            run_effect_records: None,
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
                step: RuntimeSuspendableStep::Boundary(RuntimeCoordinatorStep::Complete {
                    outcome,
                }),
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
                step: RuntimeSuspendableStep::Boundary(RuntimeCoordinatorStep::Complete {
                    outcome: outcome.clone(),
                }),
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
        // cancellation is pending or that has ended is not continued. Review
        // F2 of 8a3a341e: each rule is exercised on its own.
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

        // A queued job, created but never started: a new run never takes it
        // over, and a run resumed after a restart starts it.
        let store = crate::runtime_start_tests::JobLedgerStore::new("begin-queued");
        let ledgers = store.ledgers();
        ledgers.create(&job, owner).unwrap();
        let queued = ledgers.observation(&job).unwrap();
        assert_eq!((queued.phase, queued.revision), (JobPhase::Queued, 0));
        assert_eq!(
            begin(false, ledgers.clone()),
            Err(RuntimeTransportError::RequestDenied)
        );
        assert_eq!(ledgers.observation(&job).unwrap(), queued);
        assert_eq!(begin(true, ledgers.clone()), Ok(job.clone()));
        let started = ledgers.observation(&job).unwrap();
        assert_eq!((started.phase, started.revision), (JobPhase::Running, 1));

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

        // A suspended job is continued only through a resumption (Decision
        // 0122), and a pending cancellation or an ended job is not continued.
        let store = crate::runtime_start_tests::JobLedgerStore::new("begin-suspended");
        let suspended = store.ledgers();
        suspended.create(&job, owner).unwrap();
        suspended
            .control(
                &job,
                "peer-client-a",
                &JobControlRequest {
                    schema_version: 1,
                    job_id: job.clone(),
                    request_id: "suspend-1".to_owned(),
                    action: JobControlAction::Suspend,
                    observed_revision: 0,
                },
            )
            .unwrap();
        let held = suspended.observation(&job).unwrap();
        assert_eq!(held.phase, JobPhase::Suspended);
        for resumed in [false, true] {
            assert_eq!(
                begin(resumed, suspended.clone()),
                Err(RuntimeTransportError::RequestDenied)
            );
        }
        assert_eq!(suspended.observation(&job).unwrap(), held);
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

    #[test]
    #[cfg(all(
        target_os = "linux",
        feature = "source-artifacts",
        feature = "workflow-supervisor"
    ))]
    fn a_job_control_history_is_withheld_after_a_restart_or_an_unread_head() {
        // Review F1 of e5f34910: each completeness rule of the job control
        // history is exercised on its own. A decided request is kept and
        // declared; the same history is not declared by a run resumed after
        // a restart; and a decision whose resulting head was not read leaves
        // the history undeclared.
        let (mut session, _result_tx, _) = terminal_pair_fixture(false);
        let store = crate::runtime_start_tests::JobLedgerStore::new("history-completeness");
        session.begin_job(store.ledgers()).unwrap();
        let job = session.request.run_id.as_str().to_owned();
        let client = RuntimeClientScope::derived("peer-client-a".to_owned());
        let control = |request_id: &str, observed_revision: u64| JobControlRequest {
            schema_version: 1,
            job_id: job.clone(),
            request_id: request_id.to_owned(),
            action: JobControlAction::Cancel,
            observed_revision,
        };
        let decide = |session: &LiveCodingSession, request: &JobControlRequest| {
            session
                .usable_job()
                .unwrap()
                .ledgers
                .control(&job, client.as_str(), request)
                .unwrap()
        };
        let first = control("cancel-1", 1);
        let decision = decide(&session, &first);
        let status = session.replayed_job_status().unwrap();
        session.record_job_control(&client, &first, decision, Some(&status));
        let declared = session
            .declare_job_control_history()
            .expect("a decided request is declared");
        assert_eq!(declared.records.len(), 1);

        // The restart rule alone: the same entries, from a run resumed after
        // a restart, are not declared.
        session.job_history_from_start = false;
        assert_eq!(session.declare_job_control_history(), None);
        session.job_history_from_start = true;
        assert_eq!(session.declare_job_control_history(), Some(declared));

        // The unread head alone: a later decision whose status was not read
        // leaves the history incomplete, so it is never declared.
        let second = control("cancel-2", 2);
        let decision = decide(&session, &second);
        session.record_job_control(&client, &second, decision, None);
        assert_eq!(session.job_actions.declare(), None);
        assert_eq!(session.declare_job_control_history(), None);
    }

    #[test]
    #[cfg(all(feature = "source-artifacts", feature = "workflow-supervisor"))]
    fn a_route_is_declared_only_for_the_composition_that_chose_it() {
        // Decision 0128: the service declares the route the factory handed
        // over, and nothing for a run resumed from an event cursor, which was
        // composed again.
        let (mut session, _result_tx, _) = terminal_pair_fixture(false);
        assert_eq!(session.declare_route(), (None, None));
        let route =
            crate::coding_route::route_development_run(&session.request, "contract-test", 5_000)
                .unwrap();
        session.route = Some(route.clone());
        assert_eq!(
            session.declare_route(),
            (Some(route.receipt.clone()), route.history.clone())
        );
        let (_, events, _, _) = crate::runtime_read_tests::completed_native_read_fixture();
        let first = events.first().unwrap();
        session.request.event_cursor = Some(RuntimeEventCursor {
            run_id: first.run_id.clone(),
            event_id: first.event_id.clone(),
            sequence: first.sequence,
            event_sha256: first.event_sha256.clone(),
        });
        assert_eq!(session.declare_route(), (None, None));
    }

    struct CountingDeclarations(std::cell::Cell<usize>, std::cell::Cell<usize>);

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

        fn session_recoverability(
            &self,
            request: &RuntimeRunRequest,
            now_epoch_ms: u64,
        ) -> Option<RecoverabilityReport> {
            assert!(now_epoch_ms > 0);
            self.1.set(self.1.get() + 1);
            crate::coding_recoverability::assess_recoverability(
                request.session_id.as_str(),
                request.task.task_id.as_str(),
                &[],
                &|_| None,
            )
            .ok()
        }

        fn run_context_inspections(&self) -> Option<Vec<ContextInspection>> {
            self.0.set(self.0.get() + 1);
            Some(Vec::new())
        }

        fn run_action_history(&self, _request: &RuntimeRunRequest) -> Option<RunActionHistory> {
            self.0.set(self.0.get() + 1);
            RunActionRecorder::new().declare()
        }
    }

    #[test]
    #[cfg(all(feature = "source-artifacts", feature = "workflow-supervisor"))]
    fn a_resumed_run_declares_neither_part_whatever_its_owners_hold() {
        // Review V1 of 8fbd2bc6: a run resumed from an event cursor was
        // composed again after a restart, so its owners' records begin there.
        let (mut request, events, _, _) =
            crate::runtime_read_tests::completed_native_read_fixture();
        let owners = CountingDeclarations(std::cell::Cell::new(0), std::cell::Cell::new(0));
        let declared = declare_run(&owners, &request);
        assert_eq!(owners.0.get(), 3);
        assert!(declared.recoverability.is_some());
        let session = declared.session_recoverability.clone().unwrap();
        assert_eq!(session.run_id, None);
        assert_eq!(declared.context_inspections, Some(Vec::new()));
        assert_eq!(declared.effect_history, RunActionRecorder::new().declare());
        // The service adds the job control history it keeps itself.
        assert_eq!(declared.job_control_history, None);
        let last = events.last().unwrap();
        request.event_cursor = Some(RuntimeEventCursor {
            run_id: request.run_id.clone(),
            event_id: last.event_id.clone(),
            sequence: last.sequence,
            event_sha256: last.event_sha256.clone(),
        });
        let declared = declare_run(&owners, &request);
        assert_eq!(owners.0.get(), 3);
        assert_eq!(declared.run_id, request.run_id);
        assert_eq!(declared.request_sha256, request.request_sha256);
        assert_eq!(declared.recoverability, None);
        assert_eq!(declared.context_inspections, None);
        assert_eq!(declared.effect_history, None);
        // Decision 0143: the session's declaration comes from the stored
        // records of each of its runs, so the service asks the owner for it
        // whether or not the run was resumed; the owner refuses a session
        // whose stored record a restart marked incomplete.
        assert_eq!(owners.1.get(), 2);
        assert_eq!(declared.session_recoverability, Some(session));
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

    fn suspension_state(suspension: &SharedSuspension) -> SuspensionState {
        *suspension.state.lock().unwrap()
    }

    /// A suspension the ledger applied, which the worker may claim.
    fn requested_suspension() -> SharedSuspension {
        let suspension = SharedSuspension::new();
        suspension.follow(JobPhase::Suspending).unwrap();
        assert_eq!(suspension_state(&suspension), SuspensionState::Requested);
        suspension
    }

    #[test]
    fn a_withheld_suspension_is_never_claimed_and_then_follows_the_job() {
        // Review F3 (a) of `3c69304c`: while the service decides a request
        // that could withdraw the suspension, the worker may not stop for it.
        let suspension = requested_suspension();
        assert_eq!(suspension.withhold().unwrap(), SuspensionState::Requested);
        assert_eq!(suspension_state(&suspension), SuspensionState::Withheld);
        assert!(!suspension.suspend_at_safe_boundary());
        assert_eq!(suspension.withhold().unwrap(), SuspensionState::Withheld);
        assert_eq!(suspension_state(&suspension), SuspensionState::Withheld);
        suspension.follow(JobPhase::Running).unwrap();
        assert_eq!(suspension_state(&suspension), SuspensionState::Idle);
        assert!(!suspension.suspend_at_safe_boundary());

        // A decision that leaves the job suspending hands it back.
        let suspension = requested_suspension();
        suspension.withhold().unwrap();
        suspension.follow(JobPhase::Suspending).unwrap();
        assert_eq!(suspension_state(&suspension), SuspensionState::Requested);
        assert!(suspension.suspend_at_safe_boundary());
        assert!(suspension.claimed().unwrap());
        // A claimed suspension stays claimed whatever is decided after it.
        assert_eq!(suspension.withhold().unwrap(), SuspensionState::Claimed);
        suspension.follow(JobPhase::Cancelling).unwrap();
        assert!(suspension.claimed().unwrap());
        suspension.clear().unwrap();
        assert_eq!(suspension_state(&suspension), SuspensionState::Idle);

        // Restoring returns only a withheld suspension.
        let suspension = requested_suspension();
        suspension.withhold().unwrap();
        suspension.restore().unwrap();
        assert_eq!(suspension_state(&suspension), SuspensionState::Requested);
        let idle = SharedSuspension::new();
        idle.restore().unwrap();
        assert_eq!(idle.withhold().unwrap(), SuspensionState::Idle);
        assert!(!idle.suspend_at_safe_boundary());
    }

    #[test]
    fn without_the_job_phase_a_decision_only_withdraws_a_suspension() {
        // Review F1 of `3c69304c`: when the phase cannot be read after a
        // decision, an applied resumption or cancellation withdrew the
        // suspension; a refusal or a failed write changed nothing.
        let applied = |phase| JobControlDecision::Applied { revision: 3, phase };
        let refused = JobControlDecision::Refused {
            refusal: agentmage_kernel_engine::job_control::JobControlRefusal::StaleRevision,
            revision: 2,
            phase: JobPhase::Suspending,
        };
        for (decision, expected) in [
            (Some(applied(JobPhase::Running)), SuspensionState::Idle),
            (Some(applied(JobPhase::Cancelling)), SuspensionState::Idle),
            (Some(refused), SuspensionState::Requested),
            (None, SuspensionState::Requested),
        ] {
            let suspension = requested_suspension();
            suspension.withhold().unwrap();
            suspension.settle(None, decision.as_ref()).unwrap();
            assert_eq!(suspension_state(&suspension), expected, "{decision:?}");
            assert_eq!(
                suspension.suspend_at_safe_boundary(),
                expected == SuspensionState::Requested
            );
        }
        // Without the phase an applied suspension, which may be a retry that
        // answers an earlier decision, never requests one.
        let idle = SharedSuspension::new();
        idle.settle(None, Some(&applied(JobPhase::Suspending)))
            .unwrap();
        assert_eq!(suspension_state(&idle), SuspensionState::Idle);
        // A phase that was read is followed whatever the decision says.
        let suspension = requested_suspension();
        suspension.withhold().unwrap();
        suspension
            .settle(
                Some(JobPhase::Suspending),
                Some(&applied(JobPhase::Running)),
            )
            .unwrap();
        assert_eq!(suspension_state(&suspension), SuspensionState::Requested);
        let idle = SharedSuspension::new();
        idle.settle(Some(JobPhase::Suspending), None).unwrap();
        assert_eq!(suspension_state(&idle), SuspensionState::Requested);
    }
}
