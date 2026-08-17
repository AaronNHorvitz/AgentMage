//! Bounded durable runtime-event batching over the canonical encrypted store.

use std::collections::BTreeMap;

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
        }
    }

    /// Returns whether the failure makes the current writer result ambiguous until restart.
    #[must_use]
    pub const fn poisons_writer(self) -> bool {
        matches!(self, Self::Storage | Self::Integrity)
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
            | RuntimeEventError::SubscriberDisconnected => Self::InvalidEvent,
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
        self.sequences
            .get_mut(event.run_id.as_str())
            .ok_or(RuntimeJournalError::Integrity)?
            .push(&event)?;

        let queue_full = self.pending.len() == self.limits.queue_event_capacity
            || self.pending_bytes.saturating_add(canonical_bytes) > self.limits.queue_byte_capacity;
        if queue_full && let Err(error) = self.flush_all(store) {
            self.rebuild_sequences(store)?;
            return Err(error);
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
        let committed_events = if correctness {
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
                .push(&item.event)?;
        }
        Ok(())
    }
}

impl Default for RuntimeJournalWriter {
    fn default() -> Self {
        Self::new(RuntimeJournalLimits::default()).expect("default journal limits are valid")
    }
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

    use agentmage_kernel_contracts::{
        AgentStateKind, CONTRACT_SCHEMA_VERSION, ContextSensitivity, CorrelationId, PolicyId,
        RuntimeEvent, RuntimeEventId, RuntimeEventKind, RuntimeEventPersistenceClass,
        RuntimeEventRetention, RuntimeEventRetentionKind, RuntimeRunId, RuntimeTurnId, SessionId,
        StorageFilesystemClass, StrictLocalStorageObservation, TaskId,
    };

    use super::{
        RuntimeJournalError, RuntimeJournalLimits, RuntimeJournalWriter, current_cursor,
        load_run_events,
    };
    use crate::operational_store::{
        OperationalStore, OperationalStoreKeyError, OperationalStoreKeyProvider,
    };
    use crate::runtime_event::{runtime_event_persistence, seal_runtime_event};

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
                        code: "runtime.progress.fixture".to_owned(),
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
}
