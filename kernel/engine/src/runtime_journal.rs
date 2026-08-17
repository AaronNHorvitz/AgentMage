//! Bounded durable runtime-event batching over the canonical encrypted store.

use std::collections::{BTreeMap, VecDeque};
use std::fmt;
use std::sync::mpsc::{self, Receiver, SyncSender, TrySendError};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};

use agentmage_kernel_contracts::{
    AgentStateKind, ContextSensitivity, RuntimeEvent, RuntimeEventCursor, RuntimeEventKind,
    RuntimeEventPersistenceClass, RuntimeEventRetentionKind, RuntimeRunId, from_json,
    to_canonical_json,
};
use rusqlite::{OptionalExtension, Transaction, TransactionBehavior, params};

use crate::operational_store::OperationalStore;
use crate::runtime_event::{RuntimeEventError, RuntimeEventSequence, verify_runtime_event};

const MAX_QUEUE_EVENTS: usize = 4_096;
const MAX_QUEUE_BYTES: usize = 16 * 1024 * 1024;
const MAX_BATCH_EVENTS: usize = 512;
const MAX_BATCH_BYTES: usize = 4 * 1024 * 1024;
const MAX_FLUSH_INTERVAL_MS: u64 = 60_000;
const WORKER_CONTROL_SLOTS: usize = 8;

/// Fixed ceilings for deferred progress and metric persistence.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RuntimeJournalLimits {
    /// Maximum event envelopes waiting for one canonical writer.
    pub queue_event_capacity: usize,
    /// Maximum canonical event bytes waiting for one canonical writer.
    pub queue_byte_capacity: usize,
    /// Maximum event envelopes written in one progress batch.
    pub batch_event_capacity: usize,
    /// Maximum canonical event bytes written in one progress batch.
    pub batch_byte_capacity: usize,
    /// Maximum logical age of queued progress before an explicit due flush.
    pub flush_interval_ms: u64,
}

impl Default for RuntimeJournalLimits {
    fn default() -> Self {
        Self {
            queue_event_capacity: 1_024,
            queue_byte_capacity: 4 * 1024 * 1024,
            batch_event_capacity: 128,
            batch_byte_capacity: 512 * 1024,
            flush_interval_ms: 250,
        }
    }
}

/// Stable fail-closed journal result.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuntimeJournalError {
    /// Queue or batch limits are zero, inconsistent, or above the closed maximum.
    InvalidLimits,
    /// The event contract, retention assignment, timestamp, or serialized size is invalid.
    InvalidEvent,
    /// The event is not contiguous with the verified in-memory or durable run history.
    OrderingMismatch,
    /// Canonical event bytes could not be encoded or decoded.
    Serialization,
    /// The encrypted journal transaction failed or its retained rows did not reconcile.
    Storage,
    /// A retained journal row, digest, binding, or terminal head failed restart verification.
    Integrity,
    /// The bounded producer queue has no capacity for another event.
    QueueSaturated,
    /// The dedicated writer thread, channel, or shared store lock is unavailable.
    WorkerUnavailable,
}

impl RuntimeJournalError {
    /// Returns a stable content-free diagnostic code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidLimits => "runtime.journal.limits_invalid",
            Self::InvalidEvent => "runtime.journal.event_invalid",
            Self::OrderingMismatch => "runtime.journal.ordering_mismatch",
            Self::Serialization => "runtime.journal.serialization_failed",
            Self::Storage => "runtime.journal.storage_failed",
            Self::Integrity => "runtime.journal.integrity_failed",
            Self::QueueSaturated => "runtime.journal.queue_saturated",
            Self::WorkerUnavailable => "runtime.journal.worker_unavailable",
        }
    }

    /// Returns whether the failure makes the current writer result ambiguous until restart.
    #[must_use]
    pub const fn poisons_writer(self) -> bool {
        matches!(
            self,
            Self::Storage | Self::Integrity | Self::WorkerUnavailable
        )
    }
}

impl From<RuntimeEventError> for RuntimeJournalError {
    fn from(error: RuntimeEventError) -> Self {
        match error {
            RuntimeEventError::OrderingMismatch
            | RuntimeEventError::BindingMismatch
            | RuntimeEventError::CausationMismatch
            | RuntimeEventError::IllegalTransition
            | RuntimeEventError::TerminalStream => Self::OrderingMismatch,
            RuntimeEventError::Serialization => Self::Serialization,
            RuntimeEventError::VersionMismatch
            | RuntimeEventError::InvalidValue
            | RuntimeEventError::DigestMismatch
            | RuntimeEventError::SubscriberLimit
            | RuntimeEventError::PublisherUnavailable
            | RuntimeEventError::SubscriberDisconnected
            | RuntimeEventError::BatchLimit
            | RuntimeEventError::ReplayCursorMismatch => Self::InvalidEvent,
        }
    }
}

/// Observable result of one event submission to the bounded journal writer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RuntimeJournalAppend {
    /// Whether this call forced committed journal progress before returning.
    pub durable: bool,
    /// Number of events committed by this call.
    pub committed_events: usize,
    /// Number of events still queued under the declared bounded-loss progress policy.
    pub queued_events: usize,
    /// Canonical bytes still queued under the declared bounded-loss progress policy.
    pub queued_bytes: usize,
}

#[derive(Clone, Debug)]
struct PendingEvent {
    event: RuntimeEvent,
    canonical_bytes: usize,
}

/// One-process, one-writer runtime journal queue.
///
/// Correctness events synchronously commit themselves and every queued predecessor in one
/// transaction. Progress and metrics are deferred under fixed count, byte, batch, and age bounds.
#[derive(Debug)]
pub struct RuntimeJournalWriter {
    limits: RuntimeJournalLimits,
    pending: Vec<PendingEvent>,
    pending_bytes: usize,
    sequences: BTreeMap<String, RuntimeEventSequence>,
    oldest_pending_epoch_ms: Option<u64>,
}

impl RuntimeJournalWriter {
    /// Creates an empty writer after validating every queue and batch ceiling.
    pub fn new(limits: RuntimeJournalLimits) -> Result<Self, RuntimeJournalError> {
        if limits.queue_event_capacity == 0
            || limits.queue_event_capacity > MAX_QUEUE_EVENTS
            || limits.queue_byte_capacity == 0
            || limits.queue_byte_capacity > MAX_QUEUE_BYTES
            || limits.batch_event_capacity == 0
            || limits.batch_event_capacity > MAX_BATCH_EVENTS
            || limits.batch_event_capacity > limits.queue_event_capacity
            || limits.batch_byte_capacity == 0
            || limits.batch_byte_capacity > MAX_BATCH_BYTES
            || limits.batch_byte_capacity > limits.queue_byte_capacity
            || limits.flush_interval_ms == 0
            || limits.flush_interval_ms > MAX_FLUSH_INTERVAL_MS
        {
            return Err(RuntimeJournalError::InvalidLimits);
        }
        Ok(Self {
            limits,
            pending: Vec::new(),
            pending_bytes: 0,
            sequences: BTreeMap::new(),
            oldest_pending_epoch_ms: None,
        })
    }

    /// Returns whether no deferred progress is waiting for canonical persistence.
    #[must_use]
    pub const fn has_pending_events(&self) -> bool {
        !self.pending.is_empty()
    }

    /// Verifies and queues one event, synchronously flushing correctness boundaries.
    pub(crate) fn append(
        &mut self,
        store: &mut OperationalStore,
        event: RuntimeEvent,
    ) -> Result<RuntimeJournalAppend, RuntimeJournalError> {
        verify_runtime_event(&event)?;
        if event.retention.kind == RuntimeEventRetentionKind::Ephemeral {
            return Err(RuntimeJournalError::InvalidEvent);
        }
        let bytes = to_canonical_json(&event).map_err(|_| RuntimeJournalError::Serialization)?;
        let canonical_bytes = bytes.len();
        if canonical_bytes == 0 || canonical_bytes > self.limits.queue_byte_capacity {
            return Err(RuntimeJournalError::InvalidEvent);
        }
        self.ensure_sequence(store, &event.run_id)?;
        let admission = self
            .sequences
            .get_mut(event.run_id.as_str())
            .ok_or(RuntimeJournalError::Integrity)?
            .push(&event);
        if let Err(error) = admission {
            self.rebuild_sequences(store)?;
            return Err(error.into());
        }

        let queue_full = self.pending.len() == self.limits.queue_event_capacity
            || self.pending_bytes.saturating_add(canonical_bytes) > self.limits.queue_byte_capacity;
        let mut committed_events = 0;
        if queue_full {
            match self.flush_all(store) {
                Ok(committed) => committed_events += committed,
                Err(error) => {
                    self.rebuild_sequences(store)?;
                    return Err(error);
                }
            }
        }
        self.oldest_pending_epoch_ms
            .get_or_insert(event.occurred_at_epoch_ms);
        self.pending_bytes = self.pending_bytes.saturating_add(canonical_bytes);
        let correctness = event.persistence == RuntimeEventPersistenceClass::Correctness;
        self.pending.push(PendingEvent {
            event,
            canonical_bytes,
        });

        let should_batch = self.pending.len() >= self.limits.batch_event_capacity
            || self.pending_bytes >= self.limits.batch_byte_capacity;
        committed_events += if correctness {
            self.flush_all(store)?
        } else if should_batch {
            self.flush_one_batch(store)?
        } else {
            0
        };
        Ok(RuntimeJournalAppend {
            durable: correctness || committed_events > 0,
            committed_events,
            queued_events: self.pending.len(),
            queued_bytes: self.pending_bytes,
        })
    }

    /// Flushes progress when the oldest queued event reaches the configured logical age.
    pub(crate) fn flush_due(
        &mut self,
        store: &mut OperationalStore,
        now_epoch_ms: u64,
    ) -> Result<usize, RuntimeJournalError> {
        if now_epoch_ms == 0 {
            return Err(RuntimeJournalError::InvalidEvent);
        }
        if self.oldest_pending_epoch_ms.is_some_and(|oldest| {
            now_epoch_ms.saturating_sub(oldest) >= self.limits.flush_interval_ms
        }) {
            self.flush_all(store)
        } else {
            Ok(0)
        }
    }

    /// Flushes every queued envelope for shutdown, checkpoint, or explicit synchronization.
    pub(crate) fn flush_all(
        &mut self,
        store: &mut OperationalStore,
    ) -> Result<usize, RuntimeJournalError> {
        let mut committed = 0;
        while !self.pending.is_empty() {
            committed += self.flush_one_batch(store)?;
        }
        Ok(committed)
    }

    fn flush_one_batch(
        &mut self,
        store: &mut OperationalStore,
    ) -> Result<usize, RuntimeJournalError> {
        if self.pending.is_empty() {
            return Ok(0);
        }
        let mut count = 0_usize;
        let mut bytes = 0_usize;
        for item in &self.pending {
            if count > 0
                && (count == self.limits.batch_event_capacity
                    || bytes.saturating_add(item.canonical_bytes) > self.limits.batch_byte_capacity)
            {
                break;
            }
            count += 1;
            bytes = bytes.saturating_add(item.canonical_bytes);
        }
        let events = self.pending[..count]
            .iter()
            .map(|item| item.event.clone())
            .collect::<Vec<_>>();
        append_batch(store, &events)?;
        self.pending.drain(..count);
        self.pending_bytes = self.pending_bytes.saturating_sub(bytes);
        self.oldest_pending_epoch_ms = self
            .pending
            .first()
            .map(|item| item.event.occurred_at_epoch_ms);
        Ok(count)
    }

    fn ensure_sequence(
        &mut self,
        store: &OperationalStore,
        run_id: &RuntimeRunId,
    ) -> Result<(), RuntimeJournalError> {
        if self.sequences.contains_key(run_id.as_str()) {
            return Ok(());
        }
        let mut sequence = RuntimeEventSequence::new();
        for event in load_run_events(store, run_id)? {
            sequence
                .push(&event)
                .map_err(|_| RuntimeJournalError::Integrity)?;
        }
        self.sequences.insert(run_id.as_str().to_owned(), sequence);
        Ok(())
    }

    fn rebuild_sequences(&mut self, store: &OperationalStore) -> Result<(), RuntimeJournalError> {
        self.sequences.clear();
        let pending = self.pending.clone();
        for item in pending {
            self.ensure_sequence(store, &item.event.run_id)?;
            self.sequences
                .get_mut(item.event.run_id.as_str())
                .ok_or(RuntimeJournalError::Integrity)?
                .push(&item.event)
                .map_err(|_| RuntimeJournalError::Integrity)?;
        }
        Ok(())
    }
}

impl Default for RuntimeJournalWriter {
    fn default() -> Self {
        Self::new(RuntimeJournalLimits::default()).expect("default journal limits are valid")
    }
}

#[derive(Debug)]
struct RuntimeJournalWorkerState {
    limits: RuntimeJournalLimits,
    outstanding_events: usize,
    outstanding_bytes: usize,
    sticky_error: Option<RuntimeJournalError>,
    closed: bool,
}

impl RuntimeJournalWorkerState {
    fn reserve(&mut self, canonical_bytes: usize) -> Result<(), RuntimeJournalError> {
        if let Some(error) = self.sticky_error {
            return Err(error);
        }
        if self.closed {
            return Err(RuntimeJournalError::WorkerUnavailable);
        }
        if self.outstanding_events >= self.limits.queue_event_capacity
            || self.outstanding_bytes.saturating_add(canonical_bytes)
                > self.limits.queue_byte_capacity
        {
            return Err(RuntimeJournalError::QueueSaturated);
        }
        self.outstanding_events += 1;
        self.outstanding_bytes += canonical_bytes;
        Ok(())
    }

    fn rollback_reservation(&mut self, canonical_bytes: usize) -> Result<(), RuntimeJournalError> {
        if self.outstanding_events == 0 || self.outstanding_bytes < canonical_bytes {
            return Err(RuntimeJournalError::Integrity);
        }
        self.outstanding_events -= 1;
        self.outstanding_bytes -= canonical_bytes;
        Ok(())
    }

    fn commit(
        &mut self,
        committed_bytes: usize,
        committed_events: usize,
    ) -> Result<(), RuntimeJournalError> {
        if self.outstanding_events < committed_events || self.outstanding_bytes < committed_bytes {
            return Err(RuntimeJournalError::Integrity);
        }
        self.outstanding_events -= committed_events;
        self.outstanding_bytes -= committed_bytes;
        Ok(())
    }

    fn fail(&mut self, error: RuntimeJournalError) {
        self.sticky_error.get_or_insert(error);
    }
}

type AppendReply = SyncSender<Result<RuntimeJournalAppend, RuntimeJournalError>>;
type CountReply = SyncSender<Result<usize, RuntimeJournalError>>;
type EventsReply = SyncSender<Result<Vec<RuntimeEvent>, RuntimeJournalError>>;
type UnitReply = SyncSender<Result<(), RuntimeJournalError>>;

enum RuntimeJournalWorkerCommand {
    Append {
        event: Box<RuntimeEvent>,
        canonical_bytes: usize,
        reply: Option<AppendReply>,
    },
    FlushDue {
        now_epoch_ms: u64,
        reply: CountReply,
    },
    FlushAll {
        reply: CountReply,
    },
    Load {
        run_id: RuntimeRunId,
        reply: EventsReply,
    },
    Configure {
        limits: RuntimeJournalLimits,
        reply: UnitReply,
    },
    Reconcile {
        reply: UnitReply,
    },
    Shutdown {
        reply: UnitReply,
    },
}

/// Dedicated bounded writer for asynchronous progress and acknowledged correctness events.
pub(crate) struct RuntimeJournalWorker {
    sender: SyncSender<RuntimeJournalWorkerCommand>,
    state: Arc<Mutex<RuntimeJournalWorkerState>>,
    join: Option<JoinHandle<()>>,
}

impl fmt::Debug for RuntimeJournalWorker {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let state = self.state.lock().ok();
        formatter
            .debug_struct("RuntimeJournalWorker")
            .field(
                "outstanding_events",
                &state.as_ref().map(|state| state.outstanding_events),
            )
            .field(
                "outstanding_bytes",
                &state.as_ref().map(|state| state.outstanding_bytes),
            )
            .field(
                "sticky_error",
                &state.as_ref().and_then(|state| state.sticky_error),
            )
            .finish_non_exhaustive()
    }
}

impl RuntimeJournalWorker {
    /// Starts one named writer thread over the sole shared encrypted store.
    pub(crate) fn new(store: Arc<Mutex<OperationalStore>>) -> Result<Self, RuntimeJournalError> {
        let limits = RuntimeJournalLimits::default();
        let state = Arc::new(Mutex::new(RuntimeJournalWorkerState {
            limits,
            outstanding_events: 0,
            outstanding_bytes: 0,
            sticky_error: None,
            closed: false,
        }));
        let (sender, receiver) = mpsc::sync_channel(
            MAX_QUEUE_EVENTS
                .checked_add(WORKER_CONTROL_SLOTS)
                .ok_or(RuntimeJournalError::InvalidLimits)?,
        );
        let worker_state = Arc::clone(&state);
        let join = thread::Builder::new()
            .name("agentmage-runtime-journal".to_owned())
            .spawn(move || runtime_journal_worker_loop(store, receiver, worker_state, limits))
            .map_err(|_| RuntimeJournalError::WorkerUnavailable)?;
        Ok(Self {
            sender,
            state,
            join: Some(join),
        })
    }

    /// Validates and accepts one event, waiting only for correctness durability.
    pub(crate) fn append(
        &self,
        event: RuntimeEvent,
    ) -> Result<RuntimeJournalAppend, RuntimeJournalError> {
        verify_runtime_event(&event)?;
        if event.retention.kind == RuntimeEventRetentionKind::Ephemeral {
            return Err(RuntimeJournalError::InvalidEvent);
        }
        let canonical_bytes = to_canonical_json(&event)
            .map_err(|_| RuntimeJournalError::Serialization)?
            .len();
        if canonical_bytes == 0 {
            return Err(RuntimeJournalError::InvalidEvent);
        }
        {
            let mut state = self
                .state
                .lock()
                .map_err(|_| RuntimeJournalError::WorkerUnavailable)?;
            if canonical_bytes > state.limits.queue_byte_capacity {
                return Err(RuntimeJournalError::InvalidEvent);
            }
            state.reserve(canonical_bytes)?;
        }

        let correctness = event.persistence == RuntimeEventPersistenceClass::Correctness;
        let (reply, receiver) = if correctness {
            let (reply, receiver) = mpsc::sync_channel(1);
            (Some(reply), Some(receiver))
        } else {
            (None, None)
        };
        let command = RuntimeJournalWorkerCommand::Append {
            event: Box::new(event),
            canonical_bytes,
            reply,
        };
        if let Err(error) = self.sender.try_send(command) {
            let mut state = self
                .state
                .lock()
                .map_err(|_| RuntimeJournalError::WorkerUnavailable)?;
            if let Err(error) = state.rollback_reservation(canonical_bytes) {
                state.fail(error);
                return Err(error);
            }
            let error = match error {
                TrySendError::Full(_) => RuntimeJournalError::QueueSaturated,
                TrySendError::Disconnected(_) => RuntimeJournalError::WorkerUnavailable,
            };
            if error.poisons_writer() {
                state.fail(error);
            }
            return Err(error);
        }

        if let Some(receiver) = receiver {
            return receiver
                .recv()
                .map_err(|_| RuntimeJournalError::WorkerUnavailable)?;
        }
        let state = self
            .state
            .lock()
            .map_err(|_| RuntimeJournalError::WorkerUnavailable)?;
        if let Some(error) = state.sticky_error {
            return Err(error);
        }
        Ok(RuntimeJournalAppend {
            durable: false,
            committed_events: 0,
            queued_events: state.outstanding_events,
            queued_bytes: state.outstanding_bytes,
        })
    }

    /// Flushes progress that reached its logical age and waits for exact completion.
    pub(crate) fn flush_due(&self, now_epoch_ms: u64) -> Result<usize, RuntimeJournalError> {
        let (reply, receiver) = mpsc::sync_channel(1);
        self.send_control(RuntimeJournalWorkerCommand::FlushDue {
            now_epoch_ms,
            reply,
        })?;
        receiver
            .recv()
            .map_err(|_| RuntimeJournalError::WorkerUnavailable)?
    }

    /// Flushes every accepted event and waits for one exact durable result.
    pub(crate) fn flush_all(&self) -> Result<usize, RuntimeJournalError> {
        let (reply, receiver) = mpsc::sync_channel(1);
        self.send_control(RuntimeJournalWorkerCommand::FlushAll { reply })?;
        receiver
            .recv()
            .map_err(|_| RuntimeJournalError::WorkerUnavailable)?
    }

    /// Loads one verified durable run after every earlier worker command is observed.
    pub(crate) fn load(
        &self,
        run_id: &RuntimeRunId,
    ) -> Result<Vec<RuntimeEvent>, RuntimeJournalError> {
        let (reply, receiver) = mpsc::sync_channel(1);
        self.send_control(RuntimeJournalWorkerCommand::Load {
            run_id: run_id.clone(),
            reply,
        })?;
        receiver
            .recv()
            .map_err(|_| RuntimeJournalError::WorkerUnavailable)?
    }

    /// Replaces limits only while no accepted event remains outstanding.
    pub(crate) fn configure(
        &self,
        limits: RuntimeJournalLimits,
    ) -> Result<(), RuntimeJournalError> {
        let (reply, receiver) = mpsc::sync_channel(1);
        self.send_control(RuntimeJournalWorkerCommand::Configure { limits, reply })?;
        receiver
            .recv()
            .map_err(|_| RuntimeJournalError::WorkerUnavailable)?
    }

    /// Rebuilds verified sequence state after an owning transaction appended journal rows.
    pub(crate) fn reconcile(&self) -> Result<(), RuntimeJournalError> {
        let (reply, receiver) = mpsc::sync_channel(1);
        self.send_control(RuntimeJournalWorkerCommand::Reconcile { reply })?;
        receiver
            .recv()
            .map_err(|_| RuntimeJournalError::WorkerUnavailable)?
    }

    fn send_control(
        &self,
        command: RuntimeJournalWorkerCommand,
    ) -> Result<(), RuntimeJournalError> {
        let state = self
            .state
            .lock()
            .map_err(|_| RuntimeJournalError::WorkerUnavailable)?;
        if let Some(error) = state.sticky_error {
            return Err(error);
        }
        if state.closed {
            return Err(RuntimeJournalError::WorkerUnavailable);
        }
        drop(state);
        self.sender
            .send(command)
            .map_err(|_| RuntimeJournalError::WorkerUnavailable)
    }
}

impl Drop for RuntimeJournalWorker {
    fn drop(&mut self) {
        let (reply, receiver) = mpsc::sync_channel(1);
        let _ = self
            .sender
            .send(RuntimeJournalWorkerCommand::Shutdown { reply });
        let _ = receiver.recv();
        if let Some(join) = self.join.take() {
            let _ = join.join();
        }
    }
}

fn runtime_journal_worker_loop(
    store: Arc<Mutex<OperationalStore>>,
    receiver: Receiver<RuntimeJournalWorkerCommand>,
    state: Arc<Mutex<RuntimeJournalWorkerState>>,
    initial_limits: RuntimeJournalLimits,
) {
    let mut writer = match RuntimeJournalWriter::new(initial_limits) {
        Ok(writer) => writer,
        Err(error) => {
            set_worker_failure(&state, error);
            return;
        }
    };
    let mut pending_sizes = VecDeque::new();
    while let Ok(command) = receiver.recv() {
        let stop = matches!(command, RuntimeJournalWorkerCommand::Shutdown { .. });
        handle_worker_command(command, &store, &state, &mut writer, &mut pending_sizes);
        if stop || worker_has_failed(&state) {
            return;
        }
    }
    set_worker_failure(&state, RuntimeJournalError::WorkerUnavailable);
}

fn handle_worker_command(
    command: RuntimeJournalWorkerCommand,
    store: &Arc<Mutex<OperationalStore>>,
    state: &Arc<Mutex<RuntimeJournalWorkerState>>,
    writer: &mut RuntimeJournalWriter,
    pending_sizes: &mut VecDeque<usize>,
) {
    match command {
        RuntimeJournalWorkerCommand::Append {
            event,
            canonical_bytes,
            reply,
        } => {
            let asynchronous = reply.is_none();
            pending_sizes.push_back(canonical_bytes);
            let result = with_worker_store(store, |store| writer.append(store, *event));
            let result = match result {
                Ok(append) => commit_worker_events(state, pending_sizes, append.committed_events)
                    .and_then(|()| worker_queue_snapshot(state))
                    .map(|snapshot| RuntimeJournalAppend {
                        durable: append.durable,
                        committed_events: append.committed_events,
                        queued_events: snapshot.0,
                        queued_bytes: snapshot.1,
                    }),
                Err(error) if !error.poisons_writer() => {
                    match rollback_worker_event(state, pending_sizes, canonical_bytes) {
                        Ok(()) => Err(error),
                        Err(rollback_error) => Err(rollback_error),
                    }
                }
                Err(error) => Err(error),
            };
            if let Err(error) = result {
                if error.poisons_writer() {
                    set_worker_failure(state, error);
                } else if asynchronous {
                    set_worker_failure(state, RuntimeJournalError::Integrity);
                }
            }
            if let Some(reply) = reply {
                let _ = reply.send(result);
            }
        }
        RuntimeJournalWorkerCommand::FlushDue {
            now_epoch_ms,
            reply,
        } => {
            let result = with_worker_store(store, |store| writer.flush_due(store, now_epoch_ms));
            finish_count_result(result, state, pending_sizes, &reply);
        }
        RuntimeJournalWorkerCommand::FlushAll { reply } => {
            let result = with_worker_store(store, |store| writer.flush_all(store));
            finish_count_result(result, state, pending_sizes, &reply);
        }
        RuntimeJournalWorkerCommand::Load { run_id, reply } => {
            let result = with_worker_store(store, |store| load_run_events(store, &run_id));
            if let Err(error) = &result
                && error.poisons_writer()
            {
                set_worker_failure(state, *error);
            }
            let _ = reply.send(result);
        }
        RuntimeJournalWorkerCommand::Configure { limits, reply } => {
            let result = RuntimeJournalWriter::new(limits).and_then(|replacement| {
                let snapshot = worker_queue_snapshot(state)?;
                if snapshot.0 > 0 || writer.has_pending_events() {
                    return Err(RuntimeJournalError::InvalidLimits);
                }
                let mut worker_state = state
                    .lock()
                    .map_err(|_| RuntimeJournalError::WorkerUnavailable)?;
                if let Some(error) = worker_state.sticky_error {
                    return Err(error);
                }
                if worker_state.closed {
                    return Err(RuntimeJournalError::WorkerUnavailable);
                }
                worker_state.limits = limits;
                *writer = replacement;
                Ok(())
            });
            if let Err(error) = &result
                && error.poisons_writer()
            {
                set_worker_failure(state, *error);
            }
            let _ = reply.send(result);
        }
        RuntimeJournalWorkerCommand::Reconcile { reply } => {
            let result = worker_queue_snapshot(state).and_then(|snapshot| {
                if snapshot.0 > 0 || writer.has_pending_events() || !pending_sizes.is_empty() {
                    return Err(RuntimeJournalError::Integrity);
                }
                with_worker_store(store, |store| {
                    verify_all(store)?;
                    writer.rebuild_sequences(store)
                })
            });
            if let Err(error) = &result
                && error.poisons_writer()
            {
                set_worker_failure(state, *error);
            }
            let _ = reply.send(result);
        }
        RuntimeJournalWorkerCommand::Shutdown { reply } => {
            let result = with_worker_store(store, |store| writer.flush_all(store))
                .and_then(|count| commit_worker_events(state, pending_sizes, count));
            if let Err(error) = &result
                && error.poisons_writer()
            {
                set_worker_failure(state, *error);
            }
            let close_result = state
                .lock()
                .map_err(|_| RuntimeJournalError::WorkerUnavailable)
                .map(|mut state| state.closed = true);
            let result = result.and(close_result);
            let _ = reply.send(result);
        }
    }
}

fn with_worker_store<T>(
    store: &Arc<Mutex<OperationalStore>>,
    operation: impl FnOnce(&mut OperationalStore) -> Result<T, RuntimeJournalError>,
) -> Result<T, RuntimeJournalError> {
    let mut store = store
        .lock()
        .map_err(|_| RuntimeJournalError::WorkerUnavailable)?;
    operation(&mut store)
}

fn finish_count_result(
    result: Result<usize, RuntimeJournalError>,
    state: &Arc<Mutex<RuntimeJournalWorkerState>>,
    pending_sizes: &mut VecDeque<usize>,
    reply: &CountReply,
) {
    let result = result.and_then(|count| {
        commit_worker_events(state, pending_sizes, count)?;
        Ok(count)
    });
    if let Err(error) = &result
        && error.poisons_writer()
    {
        set_worker_failure(state, *error);
    }
    let _ = reply.send(result);
}

fn commit_worker_events(
    state: &Arc<Mutex<RuntimeJournalWorkerState>>,
    pending_sizes: &mut VecDeque<usize>,
    count: usize,
) -> Result<(), RuntimeJournalError> {
    if count > pending_sizes.len() {
        return Err(RuntimeJournalError::Integrity);
    }
    let committed_bytes = pending_sizes
        .iter()
        .take(count)
        .try_fold(0_usize, |total, bytes| total.checked_add(*bytes))
        .ok_or(RuntimeJournalError::Integrity)?;
    state
        .lock()
        .map_err(|_| RuntimeJournalError::WorkerUnavailable)?
        .commit(committed_bytes, count)?;
    pending_sizes.drain(..count);
    Ok(())
}

fn rollback_worker_event(
    state: &Arc<Mutex<RuntimeJournalWorkerState>>,
    pending_sizes: &mut VecDeque<usize>,
    canonical_bytes: usize,
) -> Result<(), RuntimeJournalError> {
    if pending_sizes.back().copied() != Some(canonical_bytes) {
        return Err(RuntimeJournalError::Integrity);
    }
    state
        .lock()
        .map_err(|_| RuntimeJournalError::WorkerUnavailable)?
        .rollback_reservation(canonical_bytes)?;
    pending_sizes.pop_back();
    Ok(())
}

fn worker_queue_snapshot(
    state: &Arc<Mutex<RuntimeJournalWorkerState>>,
) -> Result<(usize, usize), RuntimeJournalError> {
    state
        .lock()
        .map_err(|_| RuntimeJournalError::WorkerUnavailable)
        .map(|state| (state.outstanding_events, state.outstanding_bytes))
}

fn set_worker_failure(state: &Arc<Mutex<RuntimeJournalWorkerState>>, error: RuntimeJournalError) {
    if let Ok(mut state) = state.lock() {
        state.fail(error);
    }
}

fn worker_has_failed(state: &Arc<Mutex<RuntimeJournalWorkerState>>) -> bool {
    state
        .lock()
        .map_or(true, |state| state.sticky_error.is_some())
}

/// Loads and verifies the exact durable history for one run.
pub(crate) fn load_run_events(
    store: &OperationalStore,
    run_id: &RuntimeRunId,
) -> Result<Vec<RuntimeEvent>, RuntimeJournalError> {
    let rows = store
        .connection
        .prepare(
            "SELECT record_json FROM runtime_events
             WHERE run_id = ?1 ORDER BY sequence",
        )
        .and_then(|mut statement| {
            statement
                .query_map([run_id.as_str()], |row| row.get::<_, Vec<u8>>(0))?
                .collect::<Result<Vec<_>, _>>()
        })
        .map_err(|_| RuntimeJournalError::Storage)?;
    rows.into_iter()
        .map(|bytes| from_json::<RuntimeEvent>(&bytes).map_err(|_| RuntimeJournalError::Integrity))
        .collect()
}

/// Returns the last fully committed event for one run after verifying its complete chain.
pub(crate) fn current_cursor(
    store: &OperationalStore,
    run_id: &RuntimeRunId,
) -> Result<Option<RuntimeEventCursor>, RuntimeJournalError> {
    let events = load_run_events(store, run_id)?;
    let mut sequence = RuntimeEventSequence::new();
    for event in &events {
        sequence
            .push(event)
            .map_err(|_| RuntimeJournalError::Integrity)?;
    }
    Ok(events.last().map(|event| RuntimeEventCursor {
        run_id: event.run_id.clone(),
        event_id: event.event_id.clone(),
        sequence: event.sequence,
        event_sha256: event.event_sha256.clone(),
    }))
}

/// Verifies every retained run, event, head, binding, and canonical event byte sequence.
pub(crate) fn verify_all(store: &OperationalStore) -> Result<(), RuntimeJournalError> {
    let run_ids = store
        .connection
        .prepare("SELECT run_id FROM runtime_runs ORDER BY run_id")
        .and_then(|mut statement| {
            statement
                .query_map([], |row| row.get::<_, String>(0))?
                .collect::<Result<Vec<_>, _>>()
        })
        .map_err(|_| RuntimeJournalError::Storage)?;
    let event_run_count: i64 = store
        .connection
        .query_row(
            "SELECT COUNT(DISTINCT run_id) FROM runtime_events",
            [],
            |row| row.get(0),
        )
        .map_err(|_| RuntimeJournalError::Storage)?;
    if usize::try_from(event_run_count).ok() != Some(run_ids.len()) {
        return Err(RuntimeJournalError::Integrity);
    }
    for raw_run_id in run_ids {
        verify_run(store, &RuntimeRunId::from_raw(raw_run_id))?;
    }
    Ok(())
}

fn append_batch(
    store: &mut OperationalStore,
    events: &[RuntimeEvent],
) -> Result<(), RuntimeJournalError> {
    if events.is_empty() {
        return Ok(());
    }
    let transaction = store
        .connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|_| RuntimeJournalError::Storage)?;
    for event in events {
        append_event(&transaction, event)?;
    }
    transaction
        .commit()
        .map_err(|_| RuntimeJournalError::Storage)
}

/// Appends correctness events inside an already-open owning store transaction.
pub(crate) fn append_transaction_events(
    transaction: &Transaction<'_>,
    events: &[RuntimeEvent],
) -> Result<(), RuntimeJournalError> {
    for event in events {
        append_event(transaction, event)?;
    }
    Ok(())
}

fn append_event(
    transaction: &Transaction<'_>,
    event: &RuntimeEvent,
) -> Result<(), RuntimeJournalError> {
    verify_runtime_event(event)?;
    if event.retention.kind == RuntimeEventRetentionKind::Ephemeral {
        return Err(RuntimeJournalError::InvalidEvent);
    }
    let occurred_at =
        i64::try_from(event.occurred_at_epoch_ms).map_err(|_| RuntimeJournalError::InvalidEvent)?;
    let sequence = i64::try_from(event.sequence).map_err(|_| RuntimeJournalError::InvalidEvent)?;
    let existing = transaction
        .query_row(
            "SELECT session_id, task_id, correlation_id, policy_id, last_sequence,
                    last_event_sha256, terminal
             FROM runtime_runs WHERE run_id = ?1",
            [event.run_id.as_str()],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, i64>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, i64>(6)?,
                ))
            },
        )
        .optional()
        .map_err(|_| RuntimeJournalError::Storage)?;

    if let Some((session, task, correlation, policy, last_sequence, last_sha256, terminal)) =
        existing
    {
        if session != event.session_id.as_str()
            || task != event.task_id.as_str()
            || correlation != event.correlation_id.as_str()
            || policy != event.policy_id.as_str()
            || terminal != 0
            || last_sequence.checked_add(1) != Some(sequence)
            || last_sha256 != event.previous_event_sha256
        {
            return Err(RuntimeJournalError::OrderingMismatch);
        }
    } else {
        let RuntimeEventKind::RunStarted { request_sha256 } = &event.kind else {
            return Err(RuntimeJournalError::OrderingMismatch);
        };
        if event.sequence != 0 {
            return Err(RuntimeJournalError::OrderingMismatch);
        }
        transaction
            .execute(
                "INSERT INTO runtime_runs(
                    run_id, session_id, task_id, correlation_id, policy_id, request_sha256,
                    first_event_sha256, last_sequence, last_event_id, last_event_sha256,
                    event_count, terminal, terminal_state, terminal_outcome_sha256,
                    created_at_epoch_ms, updated_at_epoch_ms
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 0, ?8, ?7, 1, 0, NULL, NULL, ?9, ?9)",
                params![
                    event.run_id.as_str(),
                    event.session_id.as_str(),
                    event.task_id.as_str(),
                    event.correlation_id.as_str(),
                    event.policy_id.as_str(),
                    request_sha256,
                    &event.event_sha256,
                    event.event_id.as_str(),
                    occurred_at,
                ],
            )
            .map_err(|_| RuntimeJournalError::Storage)?;
    }

    let record_json = to_canonical_json(event).map_err(|_| RuntimeJournalError::Serialization)?;
    let payload = event.payload_reference.as_ref();
    transaction
        .execute(
            "INSERT INTO runtime_events(
                run_id, sequence, event_id, occurred_at_epoch_ms, persistence_class,
                sensitivity, retention_kind, retention_expires_at_epoch_ms,
                event_sha256, previous_event_sha256, payload_artifact_id, payload_sha256,
                payload_byte_size, payload_media_type, record_json
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
            params![
                event.run_id.as_str(),
                sequence,
                event.event_id.as_str(),
                occurred_at,
                persistence_code(event.persistence),
                sensitivity_code(event.sensitivity),
                retention_code(event.retention.kind),
                event
                    .retention
                    .expires_at_epoch_ms
                    .map(i64::try_from)
                    .transpose()
                    .map_err(|_| RuntimeJournalError::InvalidEvent)?,
                &event.event_sha256,
                &event.previous_event_sha256,
                payload.map(|value| value.artifact_id.as_str()),
                payload.map(|value| value.sha256.as_str()),
                payload
                    .map(|value| i64::try_from(value.byte_size))
                    .transpose()
                    .map_err(|_| RuntimeJournalError::InvalidEvent)?,
                payload.map(|value| value.media_type.as_str()),
                record_json,
            ],
        )
        .map_err(|_| RuntimeJournalError::Storage)?;

    if event.sequence > 0 {
        let (terminal, state, outcome) = terminal_columns(&event.kind)?;
        let changed = transaction
            .execute(
                "UPDATE runtime_runs
                 SET last_sequence = ?1, last_event_id = ?2, last_event_sha256 = ?3,
                     event_count = event_count + 1, terminal = ?4, terminal_state = ?5,
                     terminal_outcome_sha256 = ?6, updated_at_epoch_ms = ?7
                 WHERE run_id = ?8 AND last_sequence = ?9 AND last_event_sha256 = ?10
                   AND terminal = 0",
                params![
                    sequence,
                    event.event_id.as_str(),
                    &event.event_sha256,
                    terminal,
                    state,
                    outcome,
                    occurred_at,
                    event.run_id.as_str(),
                    sequence - 1,
                    &event.previous_event_sha256,
                ],
            )
            .map_err(|_| RuntimeJournalError::Storage)?;
        if changed != 1 {
            return Err(RuntimeJournalError::OrderingMismatch);
        }
    }
    Ok(())
}

fn verify_run(store: &OperationalStore, run_id: &RuntimeRunId) -> Result<(), RuntimeJournalError> {
    let events = load_run_events(store, run_id)?;
    let mut sequence = RuntimeEventSequence::new();
    for event in &events {
        sequence
            .push(event)
            .map_err(|_| RuntimeJournalError::Integrity)?;
        let canonical = to_canonical_json(event).map_err(|_| RuntimeJournalError::Integrity)?;
        let retained = store
            .connection
            .query_row(
                "SELECT occurred_at_epoch_ms, persistence_class, sensitivity, retention_kind,
                        retention_expires_at_epoch_ms, event_sha256, previous_event_sha256,
                        payload_artifact_id, payload_sha256, payload_byte_size,
                        payload_media_type, record_json
                 FROM runtime_events WHERE run_id = ?1 AND sequence = ?2",
                params![
                    run_id.as_str(),
                    i64::try_from(event.sequence).map_err(|_| RuntimeJournalError::Integrity)?
                ],
                |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, Option<i64>>(4)?,
                        row.get::<_, String>(5)?,
                        row.get::<_, String>(6)?,
                        row.get::<_, Option<String>>(7)?,
                        row.get::<_, Option<String>>(8)?,
                        row.get::<_, Option<i64>>(9)?,
                        row.get::<_, Option<String>>(10)?,
                        row.get::<_, Vec<u8>>(11)?,
                    ))
                },
            )
            .map_err(|_| RuntimeJournalError::Integrity)?;
        let payload = event.payload_reference.as_ref();
        if u64::try_from(retained.0).ok() != Some(event.occurred_at_epoch_ms)
            || retained.1 != persistence_code(event.persistence)
            || retained.2 != sensitivity_code(event.sensitivity)
            || retained.3 != retention_code(event.retention.kind)
            || retained.4.and_then(|value| u64::try_from(value).ok())
                != event.retention.expires_at_epoch_ms
            || retained.5 != event.event_sha256
            || retained.6 != event.previous_event_sha256
            || retained.7.as_deref() != payload.map(|value| value.artifact_id.as_str())
            || retained.8.as_deref() != payload.map(|value| value.sha256.as_str())
            || retained.9.and_then(|value| u64::try_from(value).ok())
                != payload.map(|value| value.byte_size)
            || retained.10.as_deref() != payload.map(|value| value.media_type.as_str())
            || retained.11 != canonical
        {
            return Err(RuntimeJournalError::Integrity);
        }
    }
    let first = events.first().ok_or(RuntimeJournalError::Integrity)?;
    let last = events.last().ok_or(RuntimeJournalError::Integrity)?;
    let head = store
        .connection
        .query_row(
            "SELECT session_id, task_id, correlation_id, policy_id, request_sha256,
                    first_event_sha256, last_sequence, last_event_id, last_event_sha256,
                    event_count, terminal, terminal_state, terminal_outcome_sha256
             FROM runtime_runs WHERE run_id = ?1",
            [run_id.as_str()],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, i64>(6)?,
                    row.get::<_, String>(7)?,
                    row.get::<_, String>(8)?,
                    row.get::<_, i64>(9)?,
                    row.get::<_, i64>(10)?,
                    row.get::<_, Option<String>>(11)?,
                    row.get::<_, Option<String>>(12)?,
                ))
            },
        )
        .map_err(|_| RuntimeJournalError::Integrity)?;
    let RuntimeEventKind::RunStarted { request_sha256 } = &first.kind else {
        return Err(RuntimeJournalError::Integrity);
    };
    let (expected_terminal, expected_state, expected_outcome) = terminal_columns(&last.kind)?;
    if head.0 != first.session_id.as_str()
        || head.1 != first.task_id.as_str()
        || head.2 != first.correlation_id.as_str()
        || head.3 != first.policy_id.as_str()
        || head.4 != *request_sha256
        || head.5 != first.event_sha256
        || u64::try_from(head.6).ok() != Some(last.sequence)
        || head.7 != last.event_id.as_str()
        || head.8 != last.event_sha256
        || usize::try_from(head.9).ok() != Some(events.len())
        || head.10 != expected_terminal
        || head.11.as_deref() != expected_state.as_deref()
        || head.12.as_deref() != expected_outcome.as_deref()
        || sequence.is_terminal() != (expected_terminal == 1)
    {
        return Err(RuntimeJournalError::Integrity);
    }
    Ok(())
}

fn terminal_columns(
    kind: &RuntimeEventKind,
) -> Result<(i64, Option<String>, Option<String>), RuntimeJournalError> {
    match kind {
        RuntimeEventKind::RunTerminal {
            state,
            outcome_sha256,
        } => Ok((
            1,
            Some(terminal_state_code(*state)?.to_owned()),
            Some(outcome_sha256.clone()),
        )),
        _ => Ok((0, None, None)),
    }
}

fn terminal_state_code(state: AgentStateKind) -> Result<&'static str, RuntimeJournalError> {
    match state {
        AgentStateKind::Success => Ok("SUCCESS"),
        AgentStateKind::NoOp => Ok("NO_OP"),
        AgentStateKind::Blocked => Ok("BLOCKED"),
        AgentStateKind::Declined => Ok("DECLINED"),
        AgentStateKind::Stalled => Ok("STALLED"),
        AgentStateKind::Exhausted => Ok("EXHAUSTED"),
        AgentStateKind::Uncertain => Ok("UNCERTAIN"),
        AgentStateKind::Cancelled => Ok("CANCELLED"),
        AgentStateKind::Failed => Ok("FAILED"),
        AgentStateKind::Observation
        | AgentStateKind::Proposal
        | AgentStateKind::Validation
        | AgentStateKind::Clarification
        | AgentStateKind::Approval
        | AgentStateKind::Execution
        | AgentStateKind::Verification
        | AgentStateKind::Checkpoint => Err(RuntimeJournalError::InvalidEvent),
    }
}

const fn persistence_code(value: RuntimeEventPersistenceClass) -> &'static str {
    match value {
        RuntimeEventPersistenceClass::Correctness => "correctness",
        RuntimeEventPersistenceClass::Progress => "progress",
        RuntimeEventPersistenceClass::Metric => "metric",
    }
}

const fn sensitivity_code(value: ContextSensitivity) -> &'static str {
    match value {
        ContextSensitivity::Public => "public",
        ContextSensitivity::Internal => "internal",
        ContextSensitivity::Private => "private",
        ContextSensitivity::Restricted => "restricted",
    }
}

const fn retention_code(value: RuntimeEventRetentionKind) -> &'static str {
    match value {
        RuntimeEventRetentionKind::Ephemeral => "ephemeral",
        RuntimeEventRetentionKind::Session => "session",
        RuntimeEventRetentionKind::UntilExpiration => "until_expiration",
        RuntimeEventRetentionKind::UserHold => "user_hold",
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::mpsc;
    use std::sync::{Arc, Mutex};
    use std::thread;
    use std::time::{Duration, Instant};

    use agentmage_kernel_contracts::{
        AgentStateKind, CONTRACT_SCHEMA_VERSION, ContextSensitivity, CorrelationId, PolicyId,
        RuntimeEvent, RuntimeEventId, RuntimeEventKind, RuntimeEventPersistenceClass,
        RuntimeEventRetention, RuntimeEventRetentionKind, RuntimeRunId, RuntimeTurnId, SessionId,
        StorageFilesystemClass, StrictLocalStorageObservation, TaskId,
    };

    use super::{
        RuntimeJournalError, RuntimeJournalLimits, RuntimeJournalWorker, RuntimeJournalWriter,
        current_cursor, load_run_events,
    };
    use crate::operational_store::{
        OperationalStore, OperationalStoreKeyError, OperationalStoreKeyProvider,
    };
    use crate::runtime_event::{
        RuntimeEventError, RuntimeEventPublisher, runtime_event_persistence, seal_runtime_event,
    };

    const ZERO_SHA256: &str = "0000000000000000000000000000000000000000000000000000000000000000";
    static NEXT_DIRECTORY: AtomicU64 = AtomicU64::new(1);

    struct TestKey;

    impl OperationalStoreKeyProvider for TestKey {
        fn with_key<T>(
            &mut self,
            operation: impl FnOnce(&[u8]) -> T,
        ) -> Result<T, OperationalStoreKeyError> {
            Ok(operation(&[73; 32]))
        }
    }

    struct EventFixture {
        sequence: u64,
        previous_sha256: String,
        causation: Option<RuntimeEventId>,
        now: u64,
    }

    impl EventFixture {
        fn new() -> Self {
            Self {
                sequence: 0,
                previous_sha256: ZERO_SHA256.to_owned(),
                causation: None,
                now: 1_000,
            }
        }

        fn event(&mut self, kind: RuntimeEventKind, turn_id: Option<&str>) -> RuntimeEvent {
            let event_id = RuntimeEventId::from_raw(format!("journal-event-{}", self.sequence));
            let event = seal_runtime_event(RuntimeEvent {
                schema_version: CONTRACT_SCHEMA_VERSION,
                event_id: event_id.clone(),
                run_id: RuntimeRunId::from_raw("journal-run-1"),
                session_id: SessionId::from_raw("journal-session-1"),
                task_id: TaskId::from_raw("journal-task-1"),
                turn_id: turn_id.map(RuntimeTurnId::from_raw),
                operation_id: None,
                correlation_id: CorrelationId::from_raw("journal-correlation-1"),
                causation_event_id: self.causation.clone(),
                sequence: self.sequence,
                occurred_at_epoch_ms: self.now,
                sensitivity: ContextSensitivity::Private,
                retention: RuntimeEventRetention {
                    kind: RuntimeEventRetentionKind::Session,
                    expires_at_epoch_ms: None,
                },
                persistence: runtime_event_persistence(&kind),
                policy_id: PolicyId::from_raw("journal-policy-1"),
                payload_reference: None,
                kind,
                previous_event_sha256: self.previous_sha256.clone(),
                event_sha256: ZERO_SHA256.to_owned(),
            })
            .expect("fixture event seals");
            self.sequence += 1;
            self.now += 1;
            self.previous_sha256.clone_from(&event.event_sha256);
            self.causation = Some(event_id);
            event
        }

        fn complete_run(&mut self) -> Vec<RuntimeEvent> {
            vec![
                self.event(
                    RuntimeEventKind::RunStarted {
                        request_sha256: "a".repeat(64),
                    },
                    None,
                ),
                self.event(RuntimeEventKind::TurnStarted, Some("journal-turn-1")),
                self.event(
                    RuntimeEventKind::Progress {
                        code: "runtime.progress".to_owned(),
                    },
                    Some("journal-turn-1"),
                ),
                self.event(
                    RuntimeEventKind::TurnCompleted {
                        outcome_sha256: "b".repeat(64),
                    },
                    Some("journal-turn-1"),
                ),
                self.event(
                    RuntimeEventKind::RunTerminal {
                        state: AgentStateKind::Success,
                        outcome_sha256: "c".repeat(64),
                    },
                    None,
                ),
            ]
        }
    }

    fn observation() -> StrictLocalStorageObservation {
        StrictLocalStorageObservation {
            filesystem: StorageFilesystemClass::Local,
            synchronization_marker: None,
            root_identity_sha256: [9; 32],
            symlink_free: true,
        }
    }

    fn temporary_directory() -> std::path::PathBuf {
        let sequence = NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "agentmage-runtime-journal-{}-{sequence}",
            std::process::id()
        ));
        fs::create_dir(&path).expect("temporary directory");
        path
    }

    fn elapsed_ms(elapsed: Duration) -> u64 {
        u64::try_from(elapsed.as_millis()).unwrap_or(u64::MAX)
    }

    fn directory_bytes(path: &std::path::Path) -> u64 {
        fs::read_dir(path)
            .expect("load directory remains readable")
            .map(|entry| {
                entry
                    .expect("load directory entry remains readable")
                    .metadata()
                    .expect("load directory metadata remains readable")
                    .len()
            })
            .sum()
    }

    fn resident_memory_kib() -> Option<u64> {
        fs::read_to_string("/proc/self/status")
            .ok()?
            .lines()
            .find_map(|line| {
                line.strip_prefix("VmRSS:")?
                    .split_ascii_whitespace()
                    .next()?
                    .parse::<u64>()
                    .ok()
            })
    }

    #[test]
    fn story_21_2_dedicated_writer_keeps_progress_and_clients_off_slow_store() {
        let directory = temporary_directory();
        let path = directory.join("authority.db");
        let store = Arc::new(Mutex::new(
            OperationalStore::open(&path, &observation(), &mut TestKey).expect("encrypted store"),
        ));
        let worker = RuntimeJournalWorker::new(Arc::clone(&store)).expect("journal worker");
        let events = EventFixture::new().complete_run();
        let publisher = RuntimeEventPublisher::new();
        let subscriber = publisher.subscribe(events.len()).expect("subscriber");

        assert!(worker.append(events[0].clone()).expect("run start").durable);
        publisher.publish(events[0].clone()).expect("publish start");
        assert_eq!(
            subscriber.try_next().expect("receive start"),
            Some(events[0].clone())
        );

        let store_guard = store.lock().expect("hold simulated slow store");
        let (reply, result) = mpsc::sync_channel(1);
        thread::scope(|scope| {
            scope.spawn(|| {
                let started = Instant::now();
                let append = worker.append(events[1].clone());
                let _ = reply.send((append, started.elapsed()));
            });
            let (append, elapsed) = match result.recv_timeout(Duration::from_millis(250)) {
                Ok(result) => result,
                Err(error) => {
                    drop(store_guard);
                    panic!("progress producer blocked on slow store: {error}");
                }
            };
            let append = append.expect("progress accepted");
            assert!(!append.durable);
            assert!(elapsed < Duration::from_millis(250));

            publisher.publish(events[1].clone()).expect("publish turn");
            assert_eq!(
                subscriber.try_next().expect("receive turn"),
                Some(events[1].clone())
            );
            assert!(
                !worker
                    .append(events[2].clone())
                    .expect("progress accepted")
                    .durable
            );
            publisher
                .publish(events[2].clone())
                .expect("publish progress");
            assert_eq!(
                subscriber.try_next().expect("receive progress"),
                Some(events[2].clone())
            );
            drop(store_guard);
        });

        assert!(
            !worker
                .append(events[3].clone())
                .expect("turn end accepted")
                .durable
        );
        publisher
            .publish(events[3].clone())
            .expect("publish turn end");
        assert!(
            worker
                .append(events[4].clone())
                .expect("terminal commits")
                .durable
        );
        publisher
            .publish(events[4].clone())
            .expect("publish terminal");
        assert_eq!(worker.flush_all(), Ok(0));
        assert_eq!(worker.load(&events[0].run_id), Ok(events.clone()));
        drop(worker);
        drop(store);
        let reopened =
            OperationalStore::open(&path, &observation(), &mut TestKey).expect("verified restart");
        assert_eq!(load_run_events(&reopened, &events[0].run_id), Ok(events));
        drop(reopened);
        fs::remove_dir_all(directory).expect("remove directory");
    }

    #[test]
    fn story_50_2_dedicated_writer_saturation_is_bounded_and_recoverable() {
        let directory = temporary_directory();
        let path = directory.join("authority.db");
        let store = Arc::new(Mutex::new(
            OperationalStore::open(&path, &observation(), &mut TestKey).expect("encrypted store"),
        ));
        let worker = RuntimeJournalWorker::new(Arc::clone(&store)).expect("journal worker");
        worker
            .configure(RuntimeJournalLimits {
                queue_event_capacity: 2,
                queue_byte_capacity: 64 * 1024,
                batch_event_capacity: 2,
                batch_byte_capacity: 64 * 1024,
                flush_interval_ms: 250,
            })
            .expect("bounded limits");
        let mut fixture = EventFixture::new();
        let start = fixture.event(
            RuntimeEventKind::RunStarted {
                request_sha256: "a".repeat(64),
            },
            None,
        );
        let turn = fixture.event(RuntimeEventKind::TurnStarted, Some("journal-turn-1"));
        let first = fixture.event(
            RuntimeEventKind::Progress {
                code: "runtime.progress".to_owned(),
            },
            Some("journal-turn-1"),
        );
        let second = fixture.event(
            RuntimeEventKind::Progress {
                code: "runtime.progress".to_owned(),
            },
            Some("journal-turn-1"),
        );

        worker.append(start.clone()).expect("start commits");
        let store_guard = store.lock().expect("hold simulated slow store");
        worker.append(turn.clone()).expect("turn queues");
        worker.append(first.clone()).expect("first progress queues");
        assert_eq!(
            worker.append(second.clone()),
            Err(RuntimeJournalError::QueueSaturated)
        );
        drop(store_guard);
        worker.flush_all().expect("accepted progress flushes");
        worker
            .append(second.clone())
            .expect("rejected event retries exactly");
        worker.flush_all().expect("retried progress flushes");
        assert_eq!(
            worker.load(&start.run_id),
            Ok(vec![start, turn, first, second])
        );
        drop(worker);
        drop(store);
        fs::remove_dir_all(directory).expect("remove directory");
    }

    #[test]
    fn story_21_2_dedicated_writer_poison_is_sticky_without_false_history() {
        let directory = temporary_directory();
        let path = directory.join("authority.db");
        let store = Arc::new(Mutex::new(
            OperationalStore::open(&path, &observation(), &mut TestKey).expect("encrypted store"),
        ));
        let worker = RuntimeJournalWorker::new(Arc::clone(&store)).expect("journal worker");
        let events = EventFixture::new().complete_run();

        worker.append(events[0].clone()).expect("run start commits");
        store
            .lock()
            .expect("store remains available")
            .connection
            .execute_batch(
                "CREATE TRIGGER reject_worker_runtime_sequence_one
                 BEFORE INSERT ON runtime_events WHEN NEW.sequence = 1
                 BEGIN SELECT RAISE(ABORT, 'synthetic worker journal failure'); END;",
            )
            .expect("failure trigger");
        worker
            .append(events[1].clone())
            .expect("progress is accepted asynchronously");

        assert_eq!(worker.flush_all(), Err(RuntimeJournalError::Storage));
        assert_eq!(
            worker.load(&events[0].run_id),
            Err(RuntimeJournalError::Storage)
        );
        assert_eq!(
            load_run_events(
                &store.lock().expect("inspect committed history"),
                &events[0].run_id,
            ),
            Ok(vec![events[0].clone()])
        );

        drop(worker);
        drop(store);
        fs::remove_dir_all(directory).expect("remove directory");
    }

    #[test]
    fn correctness_flushes_every_ordered_predecessor_and_reopens_exactly() {
        let directory = temporary_directory();
        let path = directory.join("authority.db");
        let mut store =
            OperationalStore::open(&path, &observation(), &mut TestKey).expect("encrypted store");
        let mut writer = RuntimeJournalWriter::default();
        let events = EventFixture::new().complete_run();

        let start = writer
            .append(&mut store, events[0].clone())
            .expect("start commits");
        assert_eq!((start.durable, start.committed_events), (true, 1));
        for event in &events[1..4] {
            let progress = writer
                .append(&mut store, event.clone())
                .expect("progress queues");
            assert!(!progress.durable);
        }
        assert_eq!(load_run_events(&store, &events[0].run_id).unwrap().len(), 1);
        let terminal = writer
            .append(&mut store, events[4].clone())
            .expect("terminal flushes");
        assert_eq!((terminal.durable, terminal.committed_events), (true, 4));
        assert_eq!(terminal.queued_events, 0);
        assert_eq!(load_run_events(&store, &events[0].run_id).unwrap(), events);
        assert_eq!(
            current_cursor(&store, &events[0].run_id)
                .expect("cursor")
                .expect("retained cursor")
                .event_sha256,
            events[4].event_sha256
        );

        drop(store);
        let reopened =
            OperationalStore::open(&path, &observation(), &mut TestKey).expect("verified restart");
        assert_eq!(
            load_run_events(&reopened, &events[0].run_id).unwrap(),
            events
        );
        drop(reopened);
        fs::remove_dir_all(directory).expect("cleanup");
    }

    #[test]
    fn queue_pressure_flushes_in_declared_batches_without_growth() {
        let directory = temporary_directory();
        let path = directory.join("authority.db");
        let mut store =
            OperationalStore::open(&path, &observation(), &mut TestKey).expect("encrypted store");
        let mut writer = RuntimeJournalWriter::new(RuntimeJournalLimits {
            queue_event_capacity: 2,
            queue_byte_capacity: 64 * 1024,
            batch_event_capacity: 2,
            batch_byte_capacity: 64 * 1024,
            flush_interval_ms: 10,
        })
        .expect("limits");
        let events = EventFixture::new().complete_run();
        writer.append(&mut store, events[0].clone()).unwrap();
        let first = writer.append(&mut store, events[1].clone()).unwrap();
        assert_eq!(first.queued_events, 1);
        assert!(writer.has_pending_events());
        let second = writer.append(&mut store, events[2].clone()).unwrap();
        assert_eq!((second.committed_events, second.queued_events), (2, 0));
        assert!(!writer.has_pending_events());
        let third = writer.append(&mut store, events[3].clone()).unwrap();
        assert_eq!(third.queued_events, 1);
        assert_eq!(writer.flush_due(&mut store, 1_020).unwrap(), 1);
        assert!(!writer.has_pending_events());
        writer.append(&mut store, events[4].clone()).unwrap();
        assert_eq!(load_run_events(&store, &events[0].run_id).unwrap(), events);
        drop(store);
        fs::remove_dir_all(directory).expect("cleanup");
    }

    #[test]
    fn ephemeral_events_and_failed_batches_leave_no_false_history() {
        let directory = temporary_directory();
        let path = directory.join("authority.db");
        let mut store =
            OperationalStore::open(&path, &observation(), &mut TestKey).expect("encrypted store");
        let mut writer = RuntimeJournalWriter::default();
        let mut events = EventFixture::new().complete_run();
        events[0].retention.kind = RuntimeEventRetentionKind::Ephemeral;
        events[0] = seal_runtime_event(events[0].clone()).expect("reseal ephemeral");
        assert_eq!(
            writer
                .append(&mut store, events[0].clone())
                .expect_err("ephemeral cannot persist"),
            RuntimeJournalError::InvalidEvent
        );
        assert!(
            load_run_events(&store, &events[0].run_id)
                .unwrap()
                .is_empty()
        );

        let events = EventFixture::new().complete_run();
        writer.append(&mut store, events[0].clone()).unwrap();
        writer.append(&mut store, events[1].clone()).unwrap();
        store
            .connection
            .execute_batch(
                "CREATE TRIGGER reject_runtime_sequence_two
                 BEFORE INSERT ON runtime_events WHEN NEW.sequence = 2
                 BEGIN SELECT RAISE(ABORT, 'synthetic journal failure'); END;",
            )
            .expect("failure trigger");
        let failure = writer
            .append(&mut store, events[2].clone())
            .and_then(|_| writer.flush_all(&mut store))
            .expect_err("batch fails atomically");
        assert_eq!(failure, RuntimeJournalError::Storage);
        assert_eq!(
            load_run_events(&store, &events[0].run_id).unwrap(),
            events[..1]
        );
        drop(store);
        fs::remove_dir_all(directory).expect("cleanup");
    }

    #[test]
    fn retained_event_tampering_blocks_restart() {
        let directory = temporary_directory();
        let path = directory.join("authority.db");
        let mut store =
            OperationalStore::open(&path, &observation(), &mut TestKey).expect("encrypted store");
        let events = EventFixture::new().complete_run();
        let mut writer = RuntimeJournalWriter::default();
        for event in &events {
            writer.append(&mut store, event.clone()).unwrap();
        }
        store
            .connection
            .execute(
                "UPDATE runtime_events SET record_json = X'7b7d'
                 WHERE run_id = ?1 AND sequence = 2",
                [events[0].run_id.as_str()],
            )
            .expect("tamper retained bytes");
        drop(store);
        assert!(OperationalStore::open(&path, &observation(), &mut TestKey).is_err());
        fs::remove_dir_all(directory).expect("cleanup");
    }

    #[test]
    fn retained_event_projection_tampering_blocks_restart() {
        let directory = temporary_directory();
        let path = directory.join("authority.db");
        let mut store =
            OperationalStore::open(&path, &observation(), &mut TestKey).expect("encrypted store");
        let events = EventFixture::new().complete_run();
        let mut writer = RuntimeJournalWriter::default();
        for event in &events {
            writer.append(&mut store, event.clone()).unwrap();
        }
        store
            .connection
            .execute(
                "UPDATE runtime_events SET sensitivity = 'public'
                 WHERE run_id = ?1 AND sequence = 2",
                [events[0].run_id.as_str()],
            )
            .expect("tamper indexed projection");
        drop(store);
        assert!(OperationalStore::open(&path, &observation(), &mut TestKey).is_err());
        fs::remove_dir_all(directory).expect("cleanup");
    }

    #[test]
    fn metric_class_remains_content_free_and_deferred() {
        let directory = temporary_directory();
        let path = directory.join("authority.db");
        let mut store =
            OperationalStore::open(&path, &observation(), &mut TestKey).expect("encrypted store");
        let mut fixture = EventFixture::new();
        let start = fixture.event(
            RuntimeEventKind::RunStarted {
                request_sha256: "d".repeat(64),
            },
            None,
        );
        let metric = fixture.event(
            RuntimeEventKind::Metric {
                name: "runtime.queue.depth".to_owned(),
                value: 1,
            },
            None,
        );
        assert_eq!(metric.persistence, RuntimeEventPersistenceClass::Metric);
        let mut writer = RuntimeJournalWriter::default();
        writer.append(&mut store, start.clone()).unwrap();
        let queued = writer.append(&mut store, metric.clone()).unwrap();
        assert_eq!((queued.durable, queued.queued_events), (false, 1));
        writer.flush_all(&mut store).unwrap();
        assert_eq!(
            load_run_events(&store, &start.run_id).unwrap(),
            [start, metric]
        );
        drop(store);
        fs::remove_dir_all(directory).expect("cleanup");
    }

    #[test]
    #[ignore = "explicit reference-hardware campaign; run through runtime_hardening_load.py"]
    fn story_50_2_reference_hardware_load_is_bounded_recoverable_and_nonblocking() {
        const PROGRESS_EVENTS: usize = 8_192;
        const RESTART_CYCLES: usize = 16;

        let mut fixture = EventFixture::new();
        let mut events = Vec::with_capacity(PROGRESS_EVENTS + 4);
        events.push(fixture.event(
            RuntimeEventKind::RunStarted {
                request_sha256: "a".repeat(64),
            },
            None,
        ));
        events.push(fixture.event(RuntimeEventKind::TurnStarted, Some("journal-turn-1")));
        for _ in 0..PROGRESS_EVENTS {
            events.push(fixture.event(
                RuntimeEventKind::Progress {
                    code: "runtime.progress".to_owned(),
                },
                Some("journal-turn-1"),
            ));
        }
        events.push(fixture.event(
            RuntimeEventKind::TurnCompleted {
                outcome_sha256: "b".repeat(64),
            },
            Some("journal-turn-1"),
        ));
        events.push(fixture.event(
            RuntimeEventKind::RunTerminal {
                state: AgentStateKind::Success,
                outcome_sha256: "c".repeat(64),
            },
            None,
        ));

        let publisher = RuntimeEventPublisher::new();
        let slow = publisher.subscribe(1).expect("slow subscriber");
        let fast = publisher.subscribe(1).expect("fast subscriber");
        let publish_started = Instant::now();
        let mut fast_deliveries = 0_usize;
        let mut lagged_subscribers = 0_usize;
        for event in &events {
            let delivery = publisher
                .publish(event.clone())
                .expect("load event publishes");
            lagged_subscribers += delivery.lagged;
            assert_eq!(
                fast.try_next().expect("fast subscriber remains live"),
                Some(event.clone())
            );
            fast_deliveries += 1;
        }
        let publish_elapsed_ms = elapsed_ms(publish_started.elapsed());
        assert_eq!(publisher.event_count(), Ok(events.len() as u64));
        assert_eq!(fast_deliveries, events.len());
        assert_eq!(lagged_subscribers, 1);
        assert_eq!(slow.try_next(), Ok(Some(events[0].clone())));
        assert_eq!(
            slow.try_next(),
            Err(RuntimeEventError::SubscriberDisconnected)
        );

        let directory = temporary_directory();
        let path = directory.join("authority.db");
        let store = Arc::new(Mutex::new(
            OperationalStore::open(&path, &observation(), &mut TestKey).expect("encrypted store"),
        ));
        let worker = RuntimeJournalWorker::new(Arc::clone(&store)).expect("journal worker");
        let journal_started = Instant::now();
        let mut maximum_queued_events = 0_usize;
        let mut maximum_queued_bytes = 0_usize;
        let mut queue_saturations = 0_usize;
        for event in &events {
            let mut retried_after_saturation = false;
            let append = loop {
                match worker.append(event.clone()) {
                    Ok(append) => break append,
                    Err(RuntimeJournalError::QueueSaturated) if !retried_after_saturation => {
                        retried_after_saturation = true;
                        queue_saturations += 1;
                        worker.flush_all().expect("saturated queue flushes exactly");
                    }
                    Err(error) => panic!("load event journals: {error:?}"),
                }
            };
            maximum_queued_events = maximum_queued_events.max(append.queued_events);
            maximum_queued_bytes = maximum_queued_bytes.max(append.queued_bytes);
        }
        assert_eq!(worker.flush_all(), Ok(0));
        let journal_elapsed_ms = elapsed_ms(journal_started.elapsed());
        assert_eq!(
            worker
                .load(&events[0].run_id)
                .expect("load history verifies"),
            events
        );
        assert!(maximum_queued_events <= RuntimeJournalLimits::default().queue_event_capacity);
        assert!(maximum_queued_bytes <= RuntimeJournalLimits::default().queue_byte_capacity);
        let retained_disk_bytes = directory_bytes(&directory);
        drop(worker);
        drop(store);

        let restart_started = Instant::now();
        for _ in 0..RESTART_CYCLES {
            let reopened = OperationalStore::open(&path, &observation(), &mut TestKey)
                .expect("load store reopens with verified history");
            assert_eq!(
                load_run_events(&reopened, &events[0].run_id)
                    .expect("reopened history verifies")
                    .len(),
                events.len()
            );
        }
        let restart_elapsed_ms = elapsed_ms(restart_started.elapsed());
        let resident_memory_kib = resident_memory_kib();
        fs::remove_dir_all(&directory).expect("load campaign cleanup");
        assert!(!directory.exists());

        println!(
            "AGENTMAGE_RUNTIME_LOAD_METRICS={}",
            serde_json::json!({
                "cleanup_verified": true,
                "event_count": events.len(),
                "fast_deliveries": fast_deliveries,
                "journal_elapsed_ms": journal_elapsed_ms,
                "journal_events_per_second": (events.len() as u64)
                    .saturating_mul(1_000)
                    / journal_elapsed_ms.max(1),
                "lagged_subscribers": lagged_subscribers,
                "maximum_queued_bytes": maximum_queued_bytes,
                "maximum_queued_events": maximum_queued_events,
                "publish_elapsed_ms": publish_elapsed_ms,
                "publish_events_per_second": (events.len() as u64)
                    .saturating_mul(1_000)
                    / publish_elapsed_ms.max(1),
                "queue_saturations": queue_saturations,
                "resident_memory_kib": resident_memory_kib,
                "restart_cycles": RESTART_CYCLES,
                "restart_elapsed_ms": restart_elapsed_ms,
                "retained_disk_bytes": retained_disk_bytes,
            })
        );
    }
}
