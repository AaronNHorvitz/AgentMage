//! Hash-chained runtime events and bounded ordered client publication.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Mutex;
use std::sync::mpsc::{Receiver, SyncSender, TryRecvError, TrySendError, sync_channel};

use agentmage_kernel_contracts::{
    AgentStateKind, CONTRACT_SCHEMA_VERSION, RuntimeEvent, RuntimeEventCursor, RuntimeEventKind,
    RuntimeEventPersistenceClass, RuntimeEventRetentionKind, RuntimePayloadReference,
    RuntimePermissionDisposition, to_canonical_json,
};
use serde::Serialize;
use sha2::{Digest, Sha256};

const ZERO_SHA256: &str = "0000000000000000000000000000000000000000000000000000000000000000";
const MAX_IDENTIFIER_BYTES: usize = 128;
const MAX_CODE_BYTES: usize = 128;
const MAX_MEDIA_TYPE_BYTES: usize = 128;
const MAX_SUBSCRIBERS: usize = 32;
const MAX_SUBSCRIBER_CAPACITY: usize = 4_096;
/// Maximum canonical events returned by one client drain or replay page.
pub const MAX_RUNTIME_EVENT_BATCH_EVENTS: usize = 256;
/// Maximum canonical event-envelope bytes returned by one client drain or replay page.
pub const MAX_RUNTIME_EVENT_BATCH_BYTES: usize = 1024 * 1024;

/// Stable fail-closed reason a runtime event or stream was refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuntimeEventError {
    /// The event uses an unsupported schema version.
    VersionMismatch,
    /// One identity, timestamp, code, retention value, or payload reference is invalid.
    InvalidValue,
    /// A digest is malformed or does not match canonical bytes.
    DigestMismatch,
    /// The event sequence or previous-event digest is not contiguous.
    OrderingMismatch,
    /// Run, session, task, correlation, or policy identity changed inside one stream.
    BindingMismatch,
    /// Causation is absent, forward-pointing, or names an unknown event.
    CausationMismatch,
    /// The event is not legal in the current run, turn, model, tool, or permission state.
    IllegalTransition,
    /// An event was submitted after the terminal runtime event.
    TerminalStream,
    /// A subscriber count or queue capacity exceeds its closed bound.
    SubscriberLimit,
    /// The publisher lock is unavailable because another holder panicked.
    PublisherUnavailable,
    /// A removed or closed subscriber cannot receive more events.
    SubscriberDisconnected,
    /// A client drain or replay page requested invalid count or byte ceilings.
    BatchLimit,
    /// A reconnect cursor does not identify the exact retained event in this run.
    ReplayCursorMismatch,
    /// Canonical serialization failed.
    Serialization,
}

impl RuntimeEventError {
    /// Returns one stable content-free diagnostic code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::VersionMismatch => "runtime.event.version_mismatch",
            Self::InvalidValue => "runtime.event.value_invalid",
            Self::DigestMismatch => "runtime.event.digest_mismatch",
            Self::OrderingMismatch => "runtime.event.ordering_mismatch",
            Self::BindingMismatch => "runtime.event.binding_mismatch",
            Self::CausationMismatch => "runtime.event.causation_mismatch",
            Self::IllegalTransition => "runtime.event.transition_illegal",
            Self::TerminalStream => "runtime.event.stream_terminal",
            Self::SubscriberLimit => "runtime.event.subscriber_limit",
            Self::PublisherUnavailable => "runtime.event.publisher_unavailable",
            Self::SubscriberDisconnected => "runtime.event.subscriber_disconnected",
            Self::BatchLimit => "runtime.event.batch_limit",
            Self::ReplayCursorMismatch => "runtime.event.replay_cursor_mismatch",
            Self::Serialization => "runtime.event.serialization_failed",
        }
    }
}

/// Closed count and byte ceilings for one client event batch.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RuntimeEventBatchLimits {
    /// Maximum canonical event envelopes returned by the operation.
    pub max_events: usize,
    /// Maximum aggregate canonical envelope bytes returned by the operation.
    pub max_bytes: usize,
}

impl RuntimeEventBatchLimits {
    fn validate(self) -> Result<(), RuntimeEventError> {
        if self.max_events == 0
            || self.max_events > MAX_RUNTIME_EVENT_BATCH_EVENTS
            || self.max_bytes == 0
            || self.max_bytes > MAX_RUNTIME_EVENT_BATCH_BYTES
        {
            return Err(RuntimeEventError::BatchLimit);
        }
        Ok(())
    }
}

/// Content-free adjacent progress or metric coalescing for client rendering only.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RuntimeStatusSummary {
    /// Adjacent equal progress codes represented by their exact canonical sequence range.
    Progress {
        /// Stable progress code.
        code: String,
        /// First canonical sequence represented by this summary.
        first_sequence: u64,
        /// Last canonical sequence represented by this summary.
        last_sequence: u64,
        /// Number of adjacent canonical progress events represented.
        occurrences: u32,
    },
    /// Adjacent equal metric identities represented by their latest value and sequence range.
    Metric {
        /// Stable metric identity.
        name: String,
        /// Latest observed metric value.
        latest_value: i64,
        /// First canonical sequence represented by this summary.
        first_sequence: u64,
        /// Last canonical sequence represented by this summary.
        last_sequence: u64,
        /// Number of adjacent canonical metric samples represented.
        samples: u32,
    },
}

/// One bounded canonical client batch plus optional content-free rendering summaries.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeEventBatch {
    /// Complete canonical events in exact order; correctness records are never coalesced away.
    pub events: Vec<RuntimeEvent>,
    /// Aggregate canonical bytes represented by `events`.
    pub canonical_bytes: usize,
    /// Adjacent progress and metric summaries derived without changing the canonical events.
    pub status_summaries: Vec<RuntimeStatusSummary>,
    /// Last canonical event returned, or the supplied replay cursor when no newer event exists.
    pub next_cursor: Option<RuntimeEventCursor>,
    /// Whether another bounded drain or replay request may return additional events.
    pub has_more: bool,
    /// Whether the returned/replayed history reaches one verified terminal event.
    pub terminal: bool,
    /// Whether the live subscription was removed or closed and must reconnect by cursor.
    pub disconnected: bool,
}

/// Result of publishing one verified event to bounded subscribers.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RuntimeEventDelivery {
    /// Subscribers that accepted the event without blocking.
    pub delivered: usize,
    /// Subscribers removed because their bounded queue was full.
    pub lagged: usize,
    /// Subscribers removed because their receiver was disconnected.
    pub disconnected: usize,
}

/// Receiver for one ordered, bounded runtime-event client stream.
#[derive(Debug)]
pub struct RuntimeEventSubscription {
    receiver: Receiver<RuntimeEvent>,
    pending: Mutex<Option<RuntimeEvent>>,
}

impl RuntimeEventSubscription {
    /// Returns the next event without blocking, or `None` when no event is currently queued.
    pub fn try_next(&self) -> Result<Option<RuntimeEvent>, RuntimeEventError> {
        let mut pending = self
            .pending
            .lock()
            .map_err(|_| RuntimeEventError::PublisherUnavailable)?;
        if pending.is_some() {
            return Ok(pending.take());
        }
        match self.receiver.try_recv() {
            Ok(event) => Ok(Some(event)),
            Err(TryRecvError::Empty) => Ok(None),
            Err(TryRecvError::Disconnected) => Err(RuntimeEventError::SubscriberDisconnected),
        }
    }

    /// Drains one nonblocking count-and-byte-bounded canonical batch.
    pub fn try_next_batch(
        &self,
        limits: RuntimeEventBatchLimits,
    ) -> Result<RuntimeEventBatch, RuntimeEventError> {
        limits.validate()?;
        let mut pending = self
            .pending
            .lock()
            .map_err(|_| RuntimeEventError::PublisherUnavailable)?;
        let mut events = Vec::with_capacity(limits.max_events);
        let mut canonical_bytes = 0_usize;
        let mut disconnected = false;
        let mut terminal = false;
        while events.len() < limits.max_events {
            let next = if let Some(event) = pending.take() {
                Some(event)
            } else {
                match self.receiver.try_recv() {
                    Ok(event) => Some(event),
                    Err(TryRecvError::Empty) => None,
                    Err(TryRecvError::Disconnected) => {
                        disconnected = true;
                        None
                    }
                }
            };
            let Some(event) = next else {
                break;
            };
            let event_bytes = canonical_event_bytes(&event)?;
            let next_bytes = canonical_bytes
                .checked_add(event_bytes)
                .ok_or(RuntimeEventError::BatchLimit)?;
            if next_bytes > limits.max_bytes {
                if events.is_empty() {
                    *pending = Some(event);
                    return Err(RuntimeEventError::BatchLimit);
                }
                *pending = Some(event);
                break;
            }
            terminal = matches!(event.kind, RuntimeEventKind::RunTerminal { .. });
            canonical_bytes = next_bytes;
            events.push(event);
            if terminal {
                break;
            }
        }
        let has_more = pending.is_some() || (!terminal && events.len() == limits.max_events);
        Ok(event_batch(
            events,
            canonical_bytes,
            None,
            has_more,
            terminal,
            disconnected,
        ))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct StreamBinding {
    run_id: String,
    session_id: String,
    task_id: String,
    correlation_id: String,
    policy_id: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ToolPhase {
    Requested,
    Started,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ToolState {
    phase: ToolPhase,
    operation_id: String,
}

/// Incremental verifier for one immutable runtime-event chain.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RuntimeEventSequence {
    binding: Option<StreamBinding>,
    next_sequence: u64,
    previous_event_sha256: String,
    event_ids: BTreeSet<String>,
    active_turn_id: Option<String>,
    active_model_run_id: Option<String>,
    tools: BTreeMap<String, ToolState>,
    permissions: BTreeMap<String, String>,
    cancellation_id: Option<String>,
    cancellation_observed: bool,
    last_occurred_at_epoch_ms: u64,
    terminal: bool,
}

impl RuntimeEventSequence {
    /// Creates an empty verifier that accepts only `RunStarted` as its first event.
    #[must_use]
    pub fn new() -> Self {
        Self {
            previous_event_sha256: ZERO_SHA256.to_owned(),
            ..Self::default()
        }
    }

    /// Verifies and appends one exact event without retaining event payload bytes.
    pub fn push(&mut self, event: &RuntimeEvent) -> Result<(), RuntimeEventError> {
        verify_runtime_event(event)?;
        if self.terminal {
            return Err(RuntimeEventError::TerminalStream);
        }
        if event.sequence != self.next_sequence
            || event.previous_event_sha256 != self.previous_event_sha256
        {
            return Err(RuntimeEventError::OrderingMismatch);
        }
        if self.event_ids.contains(event.event_id.as_str()) {
            return Err(RuntimeEventError::OrderingMismatch);
        }
        if self.next_sequence == 0 {
            self.admit_first(event)?;
        } else {
            self.verify_binding(event)?;
            let Some(causation_event_id) = &event.causation_event_id else {
                return Err(RuntimeEventError::CausationMismatch);
            };
            if !self.event_ids.contains(causation_event_id.as_str()) {
                return Err(RuntimeEventError::CausationMismatch);
            }
            if event.occurred_at_epoch_ms < self.last_occurred_at_epoch_ms {
                return Err(RuntimeEventError::OrderingMismatch);
            }
            self.apply_transition(event)?;
        }
        self.event_ids.insert(event.event_id.as_str().to_owned());
        self.previous_event_sha256.clone_from(&event.event_sha256);
        self.last_occurred_at_epoch_ms = event.occurred_at_epoch_ms;
        self.next_sequence = self
            .next_sequence
            .checked_add(1)
            .ok_or(RuntimeEventError::OrderingMismatch)?;
        Ok(())
    }

    /// Returns the number of verified events admitted by this sequence.
    #[must_use]
    pub const fn event_count(&self) -> u64 {
        self.next_sequence
    }

    /// Returns whether one terminal event has closed the sequence.
    #[must_use]
    pub const fn is_terminal(&self) -> bool {
        self.terminal
    }

    /// Returns the last admitted event digest, or all zeroes before the first event.
    #[must_use]
    pub fn last_event_sha256(&self) -> &str {
        &self.previous_event_sha256
    }

    fn admit_first(&mut self, event: &RuntimeEvent) -> Result<(), RuntimeEventError> {
        if !matches!(event.kind, RuntimeEventKind::RunStarted { .. })
            || event.turn_id.is_some()
            || event.operation_id.is_some()
            || event.causation_event_id.is_some()
        {
            return Err(RuntimeEventError::IllegalTransition);
        }
        self.binding = Some(StreamBinding {
            run_id: event.run_id.as_str().to_owned(),
            session_id: event.session_id.as_str().to_owned(),
            task_id: event.task_id.as_str().to_owned(),
            correlation_id: event.correlation_id.as_str().to_owned(),
            policy_id: event.policy_id.as_str().to_owned(),
        });
        Ok(())
    }

    fn verify_binding(&self, event: &RuntimeEvent) -> Result<(), RuntimeEventError> {
        let binding = self
            .binding
            .as_ref()
            .ok_or(RuntimeEventError::BindingMismatch)?;
        if binding.run_id != event.run_id.as_str()
            || binding.session_id != event.session_id.as_str()
            || binding.task_id != event.task_id.as_str()
            || binding.correlation_id != event.correlation_id.as_str()
            || binding.policy_id != event.policy_id.as_str()
        {
            return Err(RuntimeEventError::BindingMismatch);
        }
        Ok(())
    }

    fn apply_transition(&mut self, event: &RuntimeEvent) -> Result<(), RuntimeEventError> {
        match &event.kind {
            RuntimeEventKind::RunStarted { .. } => Err(RuntimeEventError::IllegalTransition),
            RuntimeEventKind::TurnStarted => {
                if self.active_turn_id.is_some()
                    || self.active_model_run_id.is_some()
                    || !self.tools.is_empty()
                    || !self.permissions.is_empty()
                {
                    return Err(RuntimeEventError::IllegalTransition);
                }
                self.active_turn_id = Some(required_turn(event)?.to_owned());
                Ok(())
            }
            RuntimeEventKind::TurnCompleted { .. } => {
                self.require_active_turn(event)?;
                if self.active_model_run_id.is_some()
                    || !self.tools.is_empty()
                    || !self.permissions.is_empty()
                {
                    return Err(RuntimeEventError::IllegalTransition);
                }
                self.active_turn_id = None;
                Ok(())
            }
            RuntimeEventKind::ModelRequested { model_run_id, .. } => {
                self.require_active_turn(event)?;
                if self.active_model_run_id.is_some() {
                    return Err(RuntimeEventError::IllegalTransition);
                }
                self.active_model_run_id = Some(model_run_id.as_str().to_owned());
                Ok(())
            }
            RuntimeEventKind::ModelCompleted { model_run_id, .. }
            | RuntimeEventKind::ModelFailed { model_run_id, .. } => {
                self.require_active_turn(event)?;
                if self.active_model_run_id.as_deref() != Some(model_run_id.as_str()) {
                    return Err(RuntimeEventError::IllegalTransition);
                }
                self.active_model_run_id = None;
                Ok(())
            }
            RuntimeEventKind::ToolRequested { tool_call_id, .. } => {
                self.require_active_turn(event)?;
                let operation_id = required_operation(event)?.to_owned();
                if self.tools.contains_key(tool_call_id.as_str()) {
                    return Err(RuntimeEventError::IllegalTransition);
                }
                self.tools.insert(
                    tool_call_id.as_str().to_owned(),
                    ToolState {
                        phase: ToolPhase::Requested,
                        operation_id,
                    },
                );
                Ok(())
            }
            RuntimeEventKind::ToolStarted { tool_call_id, .. } => {
                self.require_active_turn(event)?;
                let operation_id = required_operation(event)?;
                if self
                    .permissions
                    .values()
                    .any(|pending| pending == operation_id)
                {
                    return Err(RuntimeEventError::IllegalTransition);
                }
                let Some(tool) = self.tools.get_mut(tool_call_id.as_str()) else {
                    return Err(RuntimeEventError::IllegalTransition);
                };
                if tool.phase != ToolPhase::Requested || tool.operation_id != operation_id {
                    return Err(RuntimeEventError::IllegalTransition);
                }
                tool.phase = ToolPhase::Started;
                Ok(())
            }
            RuntimeEventKind::ToolCompleted { tool_call_id, .. }
            | RuntimeEventKind::ToolFailed { tool_call_id, .. } => {
                self.require_active_turn(event)?;
                let operation_id = required_operation(event)?;
                let Some(tool) = self.tools.get(tool_call_id.as_str()) else {
                    return Err(RuntimeEventError::IllegalTransition);
                };
                if tool.phase != ToolPhase::Started || tool.operation_id != operation_id {
                    return Err(RuntimeEventError::IllegalTransition);
                }
                self.tools.remove(tool_call_id.as_str());
                Ok(())
            }
            RuntimeEventKind::PermissionRequested { approval_id, .. } => {
                self.require_active_turn(event)?;
                let operation_id = required_operation(event)?.to_owned();
                if self
                    .permissions
                    .insert(approval_id.as_str().to_owned(), operation_id)
                    .is_some()
                {
                    return Err(RuntimeEventError::IllegalTransition);
                }
                Ok(())
            }
            RuntimeEventKind::PermissionDecided {
                approval_id,
                disposition,
                ..
            } => {
                self.require_active_turn(event)?;
                let operation_id = required_operation(event)?;
                if self
                    .permissions
                    .get(approval_id.as_str())
                    .map(String::as_str)
                    != Some(operation_id)
                {
                    return Err(RuntimeEventError::IllegalTransition);
                }
                self.permissions.remove(approval_id.as_str());
                if *disposition == RuntimePermissionDisposition::Deny {
                    let denied_tool = self
                        .tools
                        .iter()
                        .find(|(_, tool)| {
                            tool.phase == ToolPhase::Requested
                                && tool.operation_id.as_str() == operation_id
                        })
                        .map(|(tool_call_id, _)| tool_call_id.clone())
                        .ok_or(RuntimeEventError::IllegalTransition)?;
                    self.tools.remove(&denied_tool);
                }
                Ok(())
            }
            RuntimeEventKind::FileObserved { .. } | RuntimeEventKind::FileModified { .. } => {
                self.require_started_operation(event)
            }
            RuntimeEventKind::ArtifactCreated { .. } => {
                if event.turn_id.is_some() {
                    self.require_active_turn(event)?;
                    if event.operation_id.is_some() {
                        self.require_started_operation(event)?;
                    }
                } else if event.operation_id.is_some()
                    || self.active_turn_id.is_some()
                    || self.active_model_run_id.is_some()
                    || !self.tools.is_empty()
                    || !self.permissions.is_empty()
                {
                    return Err(RuntimeEventError::IllegalTransition);
                }
                Ok(())
            }
            RuntimeEventKind::CheckpointCommitted { .. } => {
                if self.active_turn_id.is_some()
                    || self.active_model_run_id.is_some()
                    || !self.tools.is_empty()
                    || !self.permissions.is_empty()
                {
                    return Err(RuntimeEventError::IllegalTransition);
                }
                Ok(())
            }
            RuntimeEventKind::CancellationRequested { cancellation_id } => {
                if self.cancellation_id.is_some() {
                    return Err(RuntimeEventError::IllegalTransition);
                }
                self.cancellation_id = Some(cancellation_id.as_str().to_owned());
                Ok(())
            }
            RuntimeEventKind::CancellationObserved { cancellation_id } => {
                if self.cancellation_id.as_deref() != Some(cancellation_id.as_str()) {
                    return Err(RuntimeEventError::IllegalTransition);
                }
                self.active_model_run_id = None;
                self.tools.clear();
                self.permissions.clear();
                self.active_turn_id = None;
                self.cancellation_observed = true;
                Ok(())
            }
            RuntimeEventKind::Progress { .. } | RuntimeEventKind::Metric { .. } => Ok(()),
            RuntimeEventKind::RunTerminal { state, .. } => {
                if !state.is_terminal()
                    || self.active_turn_id.is_some()
                    || self.active_model_run_id.is_some()
                    || !self.tools.is_empty()
                    || !self.permissions.is_empty()
                    || (*state == AgentStateKind::Cancelled && !self.cancellation_observed)
                {
                    return Err(RuntimeEventError::IllegalTransition);
                }
                self.terminal = true;
                Ok(())
            }
        }
    }

    fn require_active_turn(&self, event: &RuntimeEvent) -> Result<(), RuntimeEventError> {
        if self.active_turn_id.as_deref() != Some(required_turn(event)?) {
            return Err(RuntimeEventError::IllegalTransition);
        }
        Ok(())
    }

    fn require_started_operation(&self, event: &RuntimeEvent) -> Result<(), RuntimeEventError> {
        self.require_active_turn(event)?;
        let operation_id = required_operation(event)?;
        if !self.tools.values().any(|tool| {
            tool.phase == ToolPhase::Started && tool.operation_id.as_str() == operation_id
        }) {
            return Err(RuntimeEventError::IllegalTransition);
        }
        Ok(())
    }
}

struct RuntimeEventPublisherState {
    sequence: RuntimeEventSequence,
    subscribers: BTreeMap<u64, SyncSender<RuntimeEvent>>,
    next_subscriber_id: u64,
}

/// Narrow in-process publisher for verified runtime events.
///
/// Publication never blocks on a client. A subscriber whose bounded queue fills is removed and
/// must reconnect using a future durable-journal cursor rather than silently missing events.
pub struct RuntimeEventPublisher {
    state: Mutex<RuntimeEventPublisherState>,
}

impl RuntimeEventPublisher {
    /// Creates an empty publisher that accepts only a valid `RunStarted` event first.
    #[must_use]
    pub fn new() -> Self {
        Self {
            state: Mutex::new(RuntimeEventPublisherState {
                sequence: RuntimeEventSequence::new(),
                subscribers: BTreeMap::new(),
                next_subscriber_id: 1,
            }),
        }
    }

    /// Registers one bounded subscriber before or during a nonterminal stream.
    pub fn subscribe(
        &self,
        capacity: usize,
    ) -> Result<RuntimeEventSubscription, RuntimeEventError> {
        if capacity == 0 || capacity > MAX_SUBSCRIBER_CAPACITY {
            return Err(RuntimeEventError::SubscriberLimit);
        }
        let mut state = self
            .state
            .lock()
            .map_err(|_| RuntimeEventError::PublisherUnavailable)?;
        if state.subscribers.len() >= MAX_SUBSCRIBERS {
            return Err(RuntimeEventError::SubscriberLimit);
        }
        let subscriber_id = state.next_subscriber_id;
        state.next_subscriber_id = state
            .next_subscriber_id
            .checked_add(1)
            .ok_or(RuntimeEventError::SubscriberLimit)?;
        let (sender, receiver) = sync_channel(capacity);
        state.subscribers.insert(subscriber_id, sender);
        Ok(RuntimeEventSubscription {
            receiver,
            pending: Mutex::new(None),
        })
    }

    /// Verifies one event, advances the canonical sequence, and offers it to every subscriber.
    pub fn publish(&self, event: RuntimeEvent) -> Result<RuntimeEventDelivery, RuntimeEventError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| RuntimeEventError::PublisherUnavailable)?;
        state.sequence.push(&event)?;
        let mut delivery = RuntimeEventDelivery::default();
        state
            .subscribers
            .retain(|_, sender| match sender.try_send(event.clone()) {
                Ok(()) => {
                    delivery.delivered += 1;
                    true
                }
                Err(TrySendError::Full(_)) => {
                    delivery.lagged += 1;
                    false
                }
                Err(TrySendError::Disconnected(_)) => {
                    delivery.disconnected += 1;
                    false
                }
            });
        Ok(delivery)
    }

    /// Returns the current verified event count.
    pub fn event_count(&self) -> Result<u64, RuntimeEventError> {
        self.state
            .lock()
            .map(|state| state.sequence.event_count())
            .map_err(|_| RuntimeEventError::PublisherUnavailable)
    }
}

impl Default for RuntimeEventPublisher {
    fn default() -> Self {
        Self::new()
    }
}

/// Returns one fully verified, bounded replay page after an optional exact reconnect cursor.
pub fn replay_runtime_events(
    events: &[RuntimeEvent],
    after: Option<&RuntimeEventCursor>,
    limits: RuntimeEventBatchLimits,
) -> Result<RuntimeEventBatch, RuntimeEventError> {
    limits.validate()?;
    let mut sequence = RuntimeEventSequence::new();
    for event in events {
        sequence.push(event)?;
    }
    let start = match after {
        Some(cursor) => {
            let index = usize::try_from(cursor.sequence)
                .map_err(|_| RuntimeEventError::ReplayCursorMismatch)?;
            let event = events
                .get(index)
                .ok_or(RuntimeEventError::ReplayCursorMismatch)?;
            if event.run_id != cursor.run_id
                || event.event_id != cursor.event_id
                || event.sequence != cursor.sequence
                || event.event_sha256 != cursor.event_sha256
            {
                return Err(RuntimeEventError::ReplayCursorMismatch);
            }
            index
                .checked_add(1)
                .ok_or(RuntimeEventError::ReplayCursorMismatch)?
        }
        None => 0,
    };
    let mut page = Vec::with_capacity(limits.max_events.min(events.len().saturating_sub(start)));
    let mut canonical_bytes = 0_usize;
    let mut next_index = start;
    while next_index < events.len() && page.len() < limits.max_events {
        let event = events[next_index].clone();
        let event_bytes = canonical_event_bytes(&event)?;
        let next_bytes = canonical_bytes
            .checked_add(event_bytes)
            .ok_or(RuntimeEventError::BatchLimit)?;
        if next_bytes > limits.max_bytes {
            if page.is_empty() {
                return Err(RuntimeEventError::BatchLimit);
            }
            break;
        }
        canonical_bytes = next_bytes;
        page.push(event);
        next_index += 1;
    }
    let has_more = next_index < events.len();
    let terminal = !has_more && sequence.is_terminal();
    Ok(event_batch(
        page,
        canonical_bytes,
        after.cloned(),
        has_more,
        terminal,
        false,
    ))
}

fn event_batch(
    events: Vec<RuntimeEvent>,
    canonical_bytes: usize,
    fallback_cursor: Option<RuntimeEventCursor>,
    has_more: bool,
    terminal: bool,
    disconnected: bool,
) -> RuntimeEventBatch {
    let next_cursor = events.last().map(runtime_event_cursor).or(fallback_cursor);
    let status_summaries = coalesced_status_summaries(&events);
    RuntimeEventBatch {
        events,
        canonical_bytes,
        status_summaries,
        next_cursor,
        has_more,
        terminal,
        disconnected,
    }
}

fn canonical_event_bytes(event: &RuntimeEvent) -> Result<usize, RuntimeEventError> {
    to_canonical_json(event)
        .map(|bytes| bytes.len())
        .map_err(|_| RuntimeEventError::Serialization)
}

fn runtime_event_cursor(event: &RuntimeEvent) -> RuntimeEventCursor {
    RuntimeEventCursor {
        run_id: event.run_id.clone(),
        event_id: event.event_id.clone(),
        sequence: event.sequence,
        event_sha256: event.event_sha256.clone(),
    }
}

fn coalesced_status_summaries(events: &[RuntimeEvent]) -> Vec<RuntimeStatusSummary> {
    let mut summaries = Vec::new();
    let mut prior_was_status = false;
    for event in events {
        match &event.kind {
            RuntimeEventKind::Progress { code } => {
                if let Some(RuntimeStatusSummary::Progress {
                    code: prior_code,
                    last_sequence,
                    occurrences,
                    ..
                }) = summaries.last_mut()
                    && prior_was_status
                    && prior_code == code
                {
                    *last_sequence = event.sequence;
                    *occurrences = occurrences.saturating_add(1);
                } else {
                    summaries.push(RuntimeStatusSummary::Progress {
                        code: code.clone(),
                        first_sequence: event.sequence,
                        last_sequence: event.sequence,
                        occurrences: 1,
                    });
                }
                prior_was_status = true;
            }
            RuntimeEventKind::Metric { name, value } => {
                if let Some(RuntimeStatusSummary::Metric {
                    name: prior_name,
                    latest_value,
                    last_sequence,
                    samples,
                    ..
                }) = summaries.last_mut()
                    && prior_was_status
                    && prior_name == name
                {
                    *latest_value = *value;
                    *last_sequence = event.sequence;
                    *samples = samples.saturating_add(1);
                } else {
                    summaries.push(RuntimeStatusSummary::Metric {
                        name: name.clone(),
                        latest_value: *value,
                        first_sequence: event.sequence,
                        last_sequence: event.sequence,
                        samples: 1,
                    });
                }
                prior_was_status = true;
            }
            _ => prior_was_status = false,
        }
    }
    summaries
}

/// Seals one statically valid runtime event with its canonical SHA-256 digest.
pub fn seal_runtime_event(mut event: RuntimeEvent) -> Result<RuntimeEvent, RuntimeEventError> {
    event.schema_version = CONTRACT_SCHEMA_VERSION;
    event.event_sha256 = ZERO_SHA256.to_owned();
    validate_runtime_event_shape(&event)?;
    event.event_sha256 = canonical_sha256(&event)?;
    Ok(event)
}

/// Verifies one runtime event's closed shape and canonical digest.
pub fn verify_runtime_event(event: &RuntimeEvent) -> Result<(), RuntimeEventError> {
    validate_runtime_event_shape(event)?;
    let mut canonical = event.clone();
    canonical.event_sha256 = ZERO_SHA256.to_owned();
    if canonical_sha256(&canonical)? != event.event_sha256 {
        return Err(RuntimeEventError::DigestMismatch);
    }
    Ok(())
}

fn validate_runtime_event_shape(event: &RuntimeEvent) -> Result<(), RuntimeEventError> {
    if event.schema_version != CONTRACT_SCHEMA_VERSION {
        return Err(RuntimeEventError::VersionMismatch);
    }
    if !valid_identifier(event.event_id.as_str())
        || !valid_identifier(event.run_id.as_str())
        || !valid_identifier(event.session_id.as_str())
        || !valid_identifier(event.task_id.as_str())
        || !valid_identifier(event.correlation_id.as_str())
        || !valid_identifier(event.policy_id.as_str())
        || event
            .turn_id
            .as_ref()
            .is_some_and(|value| !valid_identifier(value.as_str()))
        || event
            .operation_id
            .as_ref()
            .is_some_and(|value| !valid_identifier(value.as_str()))
        || event
            .causation_event_id
            .as_ref()
            .is_some_and(|value| !valid_identifier(value.as_str()))
        || event.occurred_at_epoch_ms == 0
        || !valid_sha256(&event.previous_event_sha256)
        || !valid_sha256(&event.event_sha256)
        || !valid_retention(event)
        || event.persistence != runtime_event_persistence(&event.kind)
        || !valid_kind(&event.kind)
        || !valid_payload_reference(event.payload_reference.as_ref())
        || !valid_payload_placement(event)
    {
        return Err(RuntimeEventError::InvalidValue);
    }
    Ok(())
}

fn valid_retention(event: &RuntimeEvent) -> bool {
    match event.retention.kind {
        RuntimeEventRetentionKind::Ephemeral
        | RuntimeEventRetentionKind::Session
        | RuntimeEventRetentionKind::UserHold => event.retention.expires_at_epoch_ms.is_none(),
        RuntimeEventRetentionKind::UntilExpiration => event
            .retention
            .expires_at_epoch_ms
            .is_some_and(|expires| expires > event.occurred_at_epoch_ms),
    }
}

/// Returns the one canonical persistence projection for a runtime event kind.
#[must_use]
pub const fn runtime_event_persistence(kind: &RuntimeEventKind) -> RuntimeEventPersistenceClass {
    match kind {
        RuntimeEventKind::TurnStarted
        | RuntimeEventKind::TurnCompleted { .. }
        | RuntimeEventKind::ModelRequested { .. }
        | RuntimeEventKind::ModelCompleted { .. }
        | RuntimeEventKind::ModelFailed { .. }
        | RuntimeEventKind::ToolRequested { .. }
        | RuntimeEventKind::FileObserved { .. }
        | RuntimeEventKind::Progress { .. } => RuntimeEventPersistenceClass::Progress,
        RuntimeEventKind::Metric { .. } => RuntimeEventPersistenceClass::Metric,
        RuntimeEventKind::RunStarted { .. }
        | RuntimeEventKind::ToolStarted { .. }
        | RuntimeEventKind::ToolCompleted { .. }
        | RuntimeEventKind::ToolFailed { .. }
        | RuntimeEventKind::PermissionRequested { .. }
        | RuntimeEventKind::PermissionDecided { .. }
        | RuntimeEventKind::FileModified { .. }
        | RuntimeEventKind::ArtifactCreated { .. }
        | RuntimeEventKind::CheckpointCommitted { .. }
        | RuntimeEventKind::CancellationRequested { .. }
        | RuntimeEventKind::CancellationObserved { .. }
        | RuntimeEventKind::RunTerminal { .. } => RuntimeEventPersistenceClass::Correctness,
    }
}

fn valid_kind(kind: &RuntimeEventKind) -> bool {
    match kind {
        RuntimeEventKind::RunStarted { request_sha256 }
        | RuntimeEventKind::TurnCompleted {
            outcome_sha256: request_sha256,
        }
        | RuntimeEventKind::RunTerminal {
            outcome_sha256: request_sha256,
            ..
        } => valid_sha256(request_sha256),
        RuntimeEventKind::TurnStarted => true,
        RuntimeEventKind::ModelRequested {
            model_run_id,
            request_sha256,
        } => valid_identifier(model_run_id.as_str()) && valid_sha256(request_sha256),
        RuntimeEventKind::ModelCompleted {
            model_run_id,
            result_sha256,
        } => valid_identifier(model_run_id.as_str()) && valid_sha256(result_sha256),
        RuntimeEventKind::ModelFailed {
            model_run_id,
            failure_code,
        } => valid_identifier(model_run_id.as_str()) && valid_code(failure_code),
        RuntimeEventKind::ToolRequested {
            tool_call_id,
            arguments_sha256,
        } => valid_identifier(tool_call_id.as_str()) && valid_sha256(arguments_sha256),
        RuntimeEventKind::ToolStarted {
            tool_call_id,
            authority_sha256,
        } => valid_identifier(tool_call_id.as_str()) && valid_sha256(authority_sha256),
        RuntimeEventKind::ToolCompleted {
            tool_call_id,
            receipt_id,
            result_sha256,
        } => {
            valid_identifier(tool_call_id.as_str())
                && valid_identifier(receipt_id.as_str())
                && valid_sha256(result_sha256)
        }
        RuntimeEventKind::ToolFailed {
            tool_call_id,
            receipt_id,
            failure_code,
        } => {
            valid_identifier(tool_call_id.as_str())
                && receipt_id
                    .as_ref()
                    .is_none_or(|value| valid_identifier(value.as_str()))
                && valid_code(failure_code)
        }
        RuntimeEventKind::PermissionRequested {
            approval_id,
            preview_sha256,
            expires_at_epoch_ms,
            ..
        } => {
            valid_identifier(approval_id.as_str())
                && valid_sha256(preview_sha256)
                && *expires_at_epoch_ms > 0
        }
        RuntimeEventKind::PermissionDecided {
            approval_id,
            disposition,
            grant_id,
            decision_sha256,
        } => {
            valid_identifier(approval_id.as_str())
                && valid_sha256(decision_sha256)
                && match disposition {
                    RuntimePermissionDisposition::Allow => grant_id
                        .as_ref()
                        .is_some_and(|value| valid_identifier(value.as_str())),
                    RuntimePermissionDisposition::Ask | RuntimePermissionDisposition::Deny => {
                        grant_id.is_none()
                    }
                }
        }
        RuntimeEventKind::FileObserved {
            object_identity_sha256,
            observation_sha256,
        } => valid_sha256(object_identity_sha256) && valid_sha256(observation_sha256),
        RuntimeEventKind::FileModified {
            object_identity_sha256,
            postcondition_sha256,
            receipt_id,
        } => {
            valid_sha256(object_identity_sha256)
                && valid_sha256(postcondition_sha256)
                && valid_identifier(receipt_id.as_str())
        }
        RuntimeEventKind::ArtifactCreated {
            artifact_id,
            manifest_sha256,
        } => valid_identifier(artifact_id.as_str()) && valid_sha256(manifest_sha256),
        RuntimeEventKind::CheckpointCommitted {
            checkpoint_id,
            checkpoint_sha256,
        } => valid_identifier(checkpoint_id.as_str()) && valid_sha256(checkpoint_sha256),
        RuntimeEventKind::CancellationRequested { cancellation_id }
        | RuntimeEventKind::CancellationObserved { cancellation_id } => {
            valid_identifier(cancellation_id.as_str())
        }
        RuntimeEventKind::Progress { code } => valid_code(code),
        RuntimeEventKind::Metric { name, .. } => valid_code(name),
    }
}

fn valid_payload_reference(reference: Option<&RuntimePayloadReference>) -> bool {
    reference.is_none_or(|reference| {
        valid_identifier(reference.artifact_id.as_str())
            && valid_sha256(&reference.sha256)
            && reference.byte_size > 0
            && !reference.media_type.is_empty()
            && reference.media_type.len() <= MAX_MEDIA_TYPE_BYTES
            && reference.media_type.bytes().all(|byte| {
                byte.is_ascii_alphanumeric() || matches!(byte, b'/' | b'+' | b'.' | b'-')
            })
    })
}

fn valid_payload_placement(event: &RuntimeEvent) -> bool {
    match (&event.kind, &event.payload_reference) {
        (RuntimeEventKind::ArtifactCreated { artifact_id, .. }, Some(reference)) => {
            artifact_id == &reference.artifact_id
        }
        (RuntimeEventKind::ArtifactCreated { .. }, None) => false,
        (
            RuntimeEventKind::ModelCompleted { .. }
            | RuntimeEventKind::ToolCompleted { .. }
            | RuntimeEventKind::ToolFailed { .. },
            _,
        ) => true,
        (_, None) => true,
        (_, Some(_)) => false,
    }
}

fn required_turn(event: &RuntimeEvent) -> Result<&str, RuntimeEventError> {
    event
        .turn_id
        .as_ref()
        .map(|value| value.as_str())
        .ok_or(RuntimeEventError::IllegalTransition)
}

fn required_operation(event: &RuntimeEvent) -> Result<&str, RuntimeEventError> {
    event
        .operation_id
        .as_ref()
        .map(|value| value.as_str())
        .ok_or(RuntimeEventError::IllegalTransition)
}

fn valid_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_IDENTIFIER_BYTES
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':'))
}

fn valid_code(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_CODE_BYTES
        && value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'.' | b'_' | b'-')
        })
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn canonical_sha256(value: &impl Serialize) -> Result<String, RuntimeEventError> {
    let bytes = serde_json::to_vec(value).map_err(|_| RuntimeEventError::Serialization)?;
    Ok(Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect())
}

#[cfg(test)]
mod tests {
    use super::{
        MAX_RUNTIME_EVENT_BATCH_BYTES, MAX_RUNTIME_EVENT_BATCH_EVENTS, RuntimeEventBatchLimits,
        RuntimeEventDelivery, RuntimeEventError, RuntimeEventPublisher, RuntimeEventSequence,
        RuntimeStatusSummary, ZERO_SHA256, replay_runtime_events, runtime_event_persistence,
        seal_runtime_event, verify_runtime_event,
    };
    use agentmage_kernel_contracts::{
        AgentStateKind, ApprovalId, CONTRACT_SCHEMA_VERSION, ContextSensitivity, CorrelationId,
        GrantId, GrantOperation, ModelRunId, PolicyId, ReceiptId, RuntimeEvent, RuntimeEventId,
        RuntimeEventKind, RuntimeEventPersistenceClass, RuntimeEventRetention,
        RuntimeEventRetentionKind, RuntimeOperationId, RuntimePayloadReference,
        RuntimePermissionDisposition, RuntimeRunId, RuntimeTurnId, SessionId, TaskId, ToolCallId,
    };

    struct FixtureStream {
        next_sequence: u64,
        previous_sha256: String,
        causation_event_id: Option<RuntimeEventId>,
        occurred_at_epoch_ms: u64,
    }

    impl FixtureStream {
        fn new() -> Self {
            Self {
                next_sequence: 0,
                previous_sha256: ZERO_SHA256.to_owned(),
                causation_event_id: None,
                occurred_at_epoch_ms: 1_000,
            }
        }

        fn event(
            &mut self,
            kind: RuntimeEventKind,
            turn_id: Option<&str>,
            operation_id: Option<&str>,
        ) -> RuntimeEvent {
            let persistence = match kind {
                RuntimeEventKind::TurnStarted
                | RuntimeEventKind::TurnCompleted { .. }
                | RuntimeEventKind::ModelRequested { .. }
                | RuntimeEventKind::ModelCompleted { .. }
                | RuntimeEventKind::ModelFailed { .. }
                | RuntimeEventKind::ToolRequested { .. }
                | RuntimeEventKind::FileObserved { .. }
                | RuntimeEventKind::Progress { .. } => RuntimeEventPersistenceClass::Progress,
                RuntimeEventKind::Metric { .. } => RuntimeEventPersistenceClass::Metric,
                _ => RuntimeEventPersistenceClass::Correctness,
            };
            let event_id = RuntimeEventId::from_raw(format!("event-{}", self.next_sequence));
            let payload_reference = match &kind {
                RuntimeEventKind::ArtifactCreated { artifact_id, .. } => {
                    Some(RuntimePayloadReference {
                        artifact_id: artifact_id.clone(),
                        sha256: hash('d'),
                        byte_size: 16,
                        media_type: "application/json".to_owned(),
                    })
                }
                _ => None,
            };
            let event = seal_runtime_event(RuntimeEvent {
                schema_version: CONTRACT_SCHEMA_VERSION,
                event_id: event_id.clone(),
                run_id: RuntimeRunId::from_raw("run-0001"),
                session_id: SessionId::from_raw("session-0001"),
                task_id: TaskId::from_raw("task-0001"),
                turn_id: turn_id.map(RuntimeTurnId::from_raw),
                operation_id: operation_id.map(RuntimeOperationId::from_raw),
                correlation_id: CorrelationId::from_raw("correlation-0001"),
                causation_event_id: self.causation_event_id.clone(),
                sequence: self.next_sequence,
                occurred_at_epoch_ms: self.occurred_at_epoch_ms,
                sensitivity: ContextSensitivity::Internal,
                retention: RuntimeEventRetention {
                    kind: RuntimeEventRetentionKind::Ephemeral,
                    expires_at_epoch_ms: None,
                },
                persistence,
                policy_id: PolicyId::from_raw("policy-0001"),
                payload_reference,
                kind,
                previous_event_sha256: self.previous_sha256.clone(),
                event_sha256: ZERO_SHA256.to_owned(),
            })
            .expect("fixture event must seal");
            self.next_sequence += 1;
            self.previous_sha256.clone_from(&event.event_sha256);
            self.causation_event_id = Some(event_id);
            self.occurred_at_epoch_ms += 1;
            event
        }
    }

    fn hash(character: char) -> String {
        character.to_string().repeat(64)
    }

    fn valid_sequence() -> Vec<RuntimeEvent> {
        let mut fixtures = FixtureStream::new();
        vec![
            fixtures.event(
                RuntimeEventKind::RunStarted {
                    request_sha256: hash('1'),
                },
                None,
                None,
            ),
            fixtures.event(RuntimeEventKind::TurnStarted, Some("turn-0001"), None),
            fixtures.event(
                RuntimeEventKind::ModelRequested {
                    model_run_id: ModelRunId::from_raw("model-run-0001"),
                    request_sha256: hash('2'),
                },
                Some("turn-0001"),
                None,
            ),
            fixtures.event(
                RuntimeEventKind::ModelCompleted {
                    model_run_id: ModelRunId::from_raw("model-run-0001"),
                    result_sha256: hash('3'),
                },
                Some("turn-0001"),
                None,
            ),
            fixtures.event(
                RuntimeEventKind::ToolRequested {
                    tool_call_id: ToolCallId::from_raw("tool-call-0001"),
                    arguments_sha256: hash('4'),
                },
                Some("turn-0001"),
                Some("operation-0001"),
            ),
            fixtures.event(
                RuntimeEventKind::PermissionRequested {
                    approval_id: ApprovalId::from_raw("approval-0001"),
                    operation: GrantOperation::WorkspaceRead,
                    preview_sha256: hash('5'),
                    expires_at_epoch_ms: 10_000,
                },
                Some("turn-0001"),
                Some("operation-0001"),
            ),
            fixtures.event(
                RuntimeEventKind::PermissionDecided {
                    approval_id: ApprovalId::from_raw("approval-0001"),
                    disposition: agentmage_kernel_contracts::RuntimePermissionDisposition::Allow,
                    grant_id: Some(GrantId::from_raw("grant-0001")),
                    decision_sha256: hash('6'),
                },
                Some("turn-0001"),
                Some("operation-0001"),
            ),
            fixtures.event(
                RuntimeEventKind::ToolStarted {
                    tool_call_id: ToolCallId::from_raw("tool-call-0001"),
                    authority_sha256: hash('7'),
                },
                Some("turn-0001"),
                Some("operation-0001"),
            ),
            fixtures.event(
                RuntimeEventKind::FileObserved {
                    object_identity_sha256: hash('8'),
                    observation_sha256: hash('9'),
                },
                Some("turn-0001"),
                Some("operation-0001"),
            ),
            fixtures.event(
                RuntimeEventKind::ToolCompleted {
                    tool_call_id: ToolCallId::from_raw("tool-call-0001"),
                    receipt_id: ReceiptId::from_raw("receipt-0001"),
                    result_sha256: hash('a'),
                },
                Some("turn-0001"),
                Some("operation-0001"),
            ),
            fixtures.event(
                RuntimeEventKind::TurnCompleted {
                    outcome_sha256: hash('b'),
                },
                Some("turn-0001"),
                None,
            ),
            fixtures.event(
                RuntimeEventKind::RunTerminal {
                    state: AgentStateKind::Success,
                    outcome_sha256: hash('c'),
                },
                None,
                None,
            ),
        ]
    }

    fn status_sequence() -> Vec<RuntimeEvent> {
        let mut fixtures = FixtureStream::new();
        vec![
            fixtures.event(
                RuntimeEventKind::RunStarted {
                    request_sha256: hash('1'),
                },
                None,
                None,
            ),
            fixtures.event(RuntimeEventKind::TurnStarted, Some("turn-status"), None),
            fixtures.event(
                RuntimeEventKind::Progress {
                    code: "runtime.fixture.progress".to_owned(),
                },
                Some("turn-status"),
                None,
            ),
            fixtures.event(
                RuntimeEventKind::Progress {
                    code: "runtime.fixture.progress".to_owned(),
                },
                Some("turn-status"),
                None,
            ),
            fixtures.event(
                RuntimeEventKind::Metric {
                    name: "runtime.fixture.metric".to_owned(),
                    value: 1,
                },
                Some("turn-status"),
                None,
            ),
            fixtures.event(
                RuntimeEventKind::Metric {
                    name: "runtime.fixture.metric".to_owned(),
                    value: 2,
                },
                Some("turn-status"),
                None,
            ),
            fixtures.event(
                RuntimeEventKind::Progress {
                    code: "runtime.fixture.progress".to_owned(),
                },
                Some("turn-status"),
                None,
            ),
            fixtures.event(
                RuntimeEventKind::TurnCompleted {
                    outcome_sha256: hash('2'),
                },
                Some("turn-status"),
                None,
            ),
            fixtures.event(
                RuntimeEventKind::RunTerminal {
                    state: AgentStateKind::Success,
                    outcome_sha256: hash('3'),
                },
                None,
                None,
            ),
        ]
    }

    #[test]
    fn story_21_2_runtime_event_contract_is_closed_and_hash_bound() {
        let events = valid_sequence();
        for event in &events {
            verify_runtime_event(event).expect("sealed event must verify");
            let bytes = agentmage_kernel_contracts::to_canonical_json(event)
                .expect("event must serialize canonically");
            let decoded = agentmage_kernel_contracts::from_json::<RuntimeEvent>(&bytes)
                .expect("event must parse through the closed boundary");
            assert_eq!(&decoded, event);
        }

        let mut tampered = events[1].clone();
        tampered.occurred_at_epoch_ms += 1;
        assert_eq!(
            verify_runtime_event(&tampered),
            Err(RuntimeEventError::DigestMismatch)
        );

        let mut value = serde_json::to_value(&events[0]).expect("JSON value");
        value["raw_prompt"] = serde_json::Value::String("secret".to_owned());
        let bytes = serde_json::to_vec(&value).expect("JSON bytes");
        assert!(agentmage_kernel_contracts::from_json::<RuntimeEvent>(&bytes).is_err());
    }

    #[test]
    fn story_21_2_json_fixture_uses_the_rust_canonical_digest() {
        let bytes = include_bytes!("../../../schemas/runtime/examples/runtime-event.valid.json");
        let event = agentmage_kernel_contracts::from_json::<RuntimeEvent>(bytes)
            .expect("runtime event fixture must parse");
        verify_runtime_event(&event).expect("runtime event fixture digest must verify");
    }

    #[test]
    fn story_21_2_sequence_accepts_one_complete_runtime_history() {
        let events = valid_sequence();
        let mut sequence = RuntimeEventSequence::new();
        for event in &events {
            sequence.push(event).expect("event must be legal");
        }
        assert_eq!(sequence.event_count(), events.len() as u64);
        assert_eq!(
            sequence.last_event_sha256(),
            events.last().unwrap().event_sha256
        );
        assert!(sequence.is_terminal());
    }

    #[test]
    fn story_21_2_sequence_rejects_reorder_replay_binding_and_post_terminal_events() {
        let events = valid_sequence();

        let mut reordered = RuntimeEventSequence::new();
        reordered.push(&events[0]).expect("start");
        assert_eq!(
            reordered.push(&events[2]),
            Err(RuntimeEventError::OrderingMismatch)
        );

        let mut replayed = RuntimeEventSequence::new();
        replayed.push(&events[0]).expect("start");
        assert_eq!(
            replayed.push(&events[0]),
            Err(RuntimeEventError::OrderingMismatch)
        );

        let mut changed = events[1].clone();
        changed.session_id = SessionId::from_raw("session-other");
        changed = seal_runtime_event(changed).expect("changed event seals");
        let mut binding = RuntimeEventSequence::new();
        binding.push(&events[0]).expect("start");
        assert_eq!(
            binding.push(&changed),
            Err(RuntimeEventError::BindingMismatch)
        );

        let mut terminal = RuntimeEventSequence::new();
        for event in &events {
            terminal.push(event).expect("valid sequence");
        }
        assert_eq!(
            terminal.push(events.last().unwrap()),
            Err(RuntimeEventError::TerminalStream)
        );
    }

    #[test]
    fn story_21_2_sequence_rejects_unknown_causation_and_illegal_tool_start() {
        let events = valid_sequence();
        let mut unknown_cause = events[1].clone();
        unknown_cause.causation_event_id = Some(RuntimeEventId::from_raw("event-unknown"));
        unknown_cause = seal_runtime_event(unknown_cause).expect("changed event seals");
        let mut sequence = RuntimeEventSequence::new();
        sequence.push(&events[0]).expect("start");
        assert_eq!(
            sequence.push(&unknown_cause),
            Err(RuntimeEventError::CausationMismatch)
        );

        let mut fixtures = FixtureStream::new();
        let start = fixtures.event(
            RuntimeEventKind::RunStarted {
                request_sha256: hash('1'),
            },
            None,
            None,
        );
        let turn = fixtures.event(RuntimeEventKind::TurnStarted, Some("turn-0001"), None);
        let started = fixtures.event(
            RuntimeEventKind::ToolStarted {
                tool_call_id: ToolCallId::from_raw("tool-call-unknown"),
                authority_sha256: hash('2'),
            },
            Some("turn-0001"),
            Some("operation-0001"),
        );
        let mut sequence = RuntimeEventSequence::new();
        sequence.push(&start).expect("start");
        sequence.push(&turn).expect("turn");
        assert_eq!(
            sequence.push(&started),
            Err(RuntimeEventError::IllegalTransition)
        );
    }

    #[test]
    fn safe_boundary_artifact_is_legal_only_between_quiescent_turns() {
        let mut fixtures = FixtureStream::new();
        let events = [
            fixtures.event(
                RuntimeEventKind::RunStarted {
                    request_sha256: hash('1'),
                },
                None,
                None,
            ),
            fixtures.event(RuntimeEventKind::TurnStarted, Some("turn-0001"), None),
            fixtures.event(
                RuntimeEventKind::TurnCompleted {
                    outcome_sha256: hash('2'),
                },
                Some("turn-0001"),
                None,
            ),
            fixtures.event(
                RuntimeEventKind::ArtifactCreated {
                    artifact_id: agentmage_kernel_contracts::RuntimeArtifactId::from_raw(
                        "artifact-continuation-0001",
                    ),
                    manifest_sha256: hash('3'),
                },
                None,
                None,
            ),
        ];
        let mut sequence = RuntimeEventSequence::new();
        for event in &events {
            sequence.push(event).expect("safe-boundary event is legal");
        }

        let mut active = RuntimeEventSequence::new();
        active.push(&events[0]).expect("run starts");
        active.push(&events[1]).expect("turn starts");
        let mut artifact_during_turn = events[3].clone();
        artifact_during_turn.sequence = 2;
        artifact_during_turn.causation_event_id = Some(events[1].event_id.clone());
        artifact_during_turn.previous_event_sha256 = events[1].event_sha256.clone();
        artifact_during_turn.event_sha256 = ZERO_SHA256.to_owned();
        artifact_during_turn = seal_runtime_event(artifact_during_turn).expect("artifact reseals");
        assert_eq!(
            active.push(&artifact_during_turn),
            Err(RuntimeEventError::IllegalTransition)
        );
    }

    #[test]
    fn story_21_2_publisher_is_ordered_bounded_and_nonblocking() {
        let events = valid_sequence();
        let publisher = RuntimeEventPublisher::new();
        let subscriber = publisher.subscribe(events.len()).expect("subscriber");
        for event in events.clone() {
            assert_eq!(
                publisher.publish(event).expect("publish"),
                RuntimeEventDelivery {
                    delivered: 1,
                    lagged: 0,
                    disconnected: 0,
                }
            );
        }
        for expected in events {
            assert_eq!(subscriber.try_next().expect("receive"), Some(expected));
        }
        assert_eq!(subscriber.try_next().expect("empty queue"), None);
    }

    #[test]
    fn story_21_2_lagging_subscriber_is_removed_without_blocking_runtime() {
        let events = valid_sequence();
        let publisher = RuntimeEventPublisher::new();
        let subscriber = publisher.subscribe(1).expect("subscriber");
        assert_eq!(
            publisher.publish(events[0].clone()).expect("start"),
            RuntimeEventDelivery {
                delivered: 1,
                lagged: 0,
                disconnected: 0,
            }
        );
        assert_eq!(
            publisher.publish(events[1].clone()).expect("turn"),
            RuntimeEventDelivery {
                delivered: 0,
                lagged: 1,
                disconnected: 0,
            }
        );
        assert_eq!(
            subscriber.try_next().expect("queued start"),
            Some(events[0].clone())
        );
        assert_eq!(
            subscriber.try_next(),
            Err(RuntimeEventError::SubscriberDisconnected)
        );
        for event in events.into_iter().skip(2) {
            publisher.publish(event).expect("publisher continues");
        }
        assert_eq!(publisher.event_count(), Ok(12));
    }

    #[test]
    fn story_21_2_no_runtime_event_variant_contains_streamed_token_text() {
        let event_names = [
            "run_started",
            "turn_started",
            "turn_completed",
            "model_requested",
            "model_completed",
            "model_failed",
            "tool_requested",
            "tool_started",
            "tool_completed",
            "tool_failed",
            "permission_requested",
            "permission_decided",
            "file_observed",
            "file_modified",
            "artifact_created",
            "checkpoint_committed",
            "cancellation_requested",
            "cancellation_observed",
            "progress",
            "metric",
            "run_terminal",
        ];
        assert_eq!(event_names.len(), 21);
        assert!(!event_names.iter().any(|name| name.contains("token")));
        assert_eq!(CONTRACT_SCHEMA_VERSION, 2);
    }

    #[test]
    fn story_21_2_closed_event_families_have_one_canonical_persistence_class() {
        let cases = [
            (
                RuntimeEventKind::RunStarted {
                    request_sha256: hash('1'),
                },
                RuntimeEventPersistenceClass::Correctness,
            ),
            (
                RuntimeEventKind::TurnStarted,
                RuntimeEventPersistenceClass::Progress,
            ),
            (
                RuntimeEventKind::TurnCompleted {
                    outcome_sha256: hash('2'),
                },
                RuntimeEventPersistenceClass::Progress,
            ),
            (
                RuntimeEventKind::ModelRequested {
                    model_run_id: ModelRunId::from_raw("model-run-1"),
                    request_sha256: hash('3'),
                },
                RuntimeEventPersistenceClass::Progress,
            ),
            (
                RuntimeEventKind::ModelCompleted {
                    model_run_id: ModelRunId::from_raw("model-run-1"),
                    result_sha256: hash('4'),
                },
                RuntimeEventPersistenceClass::Progress,
            ),
            (
                RuntimeEventKind::ModelFailed {
                    model_run_id: ModelRunId::from_raw("model-run-1"),
                    failure_code: "model.failed".to_owned(),
                },
                RuntimeEventPersistenceClass::Progress,
            ),
            (
                RuntimeEventKind::ToolRequested {
                    tool_call_id: ToolCallId::from_raw("tool-call-1"),
                    arguments_sha256: hash('5'),
                },
                RuntimeEventPersistenceClass::Progress,
            ),
            (
                RuntimeEventKind::ToolStarted {
                    tool_call_id: ToolCallId::from_raw("tool-call-1"),
                    authority_sha256: hash('6'),
                },
                RuntimeEventPersistenceClass::Correctness,
            ),
            (
                RuntimeEventKind::ToolCompleted {
                    tool_call_id: ToolCallId::from_raw("tool-call-1"),
                    receipt_id: ReceiptId::from_raw("receipt-1"),
                    result_sha256: hash('7'),
                },
                RuntimeEventPersistenceClass::Correctness,
            ),
            (
                RuntimeEventKind::ToolFailed {
                    tool_call_id: ToolCallId::from_raw("tool-call-1"),
                    receipt_id: None,
                    failure_code: "tool.failed".to_owned(),
                },
                RuntimeEventPersistenceClass::Correctness,
            ),
            (
                RuntimeEventKind::PermissionRequested {
                    approval_id: ApprovalId::from_raw("approval-1"),
                    operation: GrantOperation::WorkspaceRead,
                    preview_sha256: hash('8'),
                    expires_at_epoch_ms: 2_000,
                },
                RuntimeEventPersistenceClass::Correctness,
            ),
            (
                RuntimeEventKind::PermissionDecided {
                    approval_id: ApprovalId::from_raw("approval-1"),
                    disposition: RuntimePermissionDisposition::Allow,
                    grant_id: Some(GrantId::from_raw("grant-1")),
                    decision_sha256: hash('9'),
                },
                RuntimeEventPersistenceClass::Correctness,
            ),
            (
                RuntimeEventKind::FileObserved {
                    object_identity_sha256: hash('a'),
                    observation_sha256: hash('b'),
                },
                RuntimeEventPersistenceClass::Progress,
            ),
            (
                RuntimeEventKind::FileModified {
                    object_identity_sha256: hash('a'),
                    postcondition_sha256: hash('c'),
                    receipt_id: ReceiptId::from_raw("receipt-1"),
                },
                RuntimeEventPersistenceClass::Correctness,
            ),
            (
                RuntimeEventKind::ArtifactCreated {
                    artifact_id: agentmage_kernel_contracts::RuntimeArtifactId::from_raw(
                        "artifact-1",
                    ),
                    manifest_sha256: hash('d'),
                },
                RuntimeEventPersistenceClass::Correctness,
            ),
            (
                RuntimeEventKind::CheckpointCommitted {
                    checkpoint_id: agentmage_kernel_contracts::SessionCheckpointId::from_raw(
                        "checkpoint-1",
                    ),
                    checkpoint_sha256: hash('e'),
                },
                RuntimeEventPersistenceClass::Correctness,
            ),
            (
                RuntimeEventKind::CancellationRequested {
                    cancellation_id: agentmage_kernel_contracts::CancellationId::from_raw(
                        "cancellation-1",
                    ),
                },
                RuntimeEventPersistenceClass::Correctness,
            ),
            (
                RuntimeEventKind::CancellationObserved {
                    cancellation_id: agentmage_kernel_contracts::CancellationId::from_raw(
                        "cancellation-1",
                    ),
                },
                RuntimeEventPersistenceClass::Correctness,
            ),
            (
                RuntimeEventKind::Progress {
                    code: "runtime.progress".to_owned(),
                },
                RuntimeEventPersistenceClass::Progress,
            ),
            (
                RuntimeEventKind::Metric {
                    name: "runtime.queue.depth".to_owned(),
                    value: 1,
                },
                RuntimeEventPersistenceClass::Metric,
            ),
            (
                RuntimeEventKind::RunTerminal {
                    state: AgentStateKind::Success,
                    outcome_sha256: hash('f'),
                },
                RuntimeEventPersistenceClass::Correctness,
            ),
        ];
        assert_eq!(cases.len(), 21);
        for (kind, expected) in cases {
            assert_eq!(runtime_event_persistence(&kind), expected);
        }
    }

    #[test]
    fn story_21_2_every_stream_binding_dimension_is_immutable() {
        let events = valid_sequence();
        let mut mutations = Vec::new();

        let mut run = events[1].clone();
        run.run_id = RuntimeRunId::from_raw("run-other");
        mutations.push(run);

        let mut session = events[1].clone();
        session.session_id = SessionId::from_raw("session-other");
        mutations.push(session);

        let mut task = events[1].clone();
        task.task_id = TaskId::from_raw("task-other");
        mutations.push(task);

        let mut correlation = events[1].clone();
        correlation.correlation_id = CorrelationId::from_raw("correlation-other");
        mutations.push(correlation);

        let mut policy = events[1].clone();
        policy.policy_id = PolicyId::from_raw("policy-other");
        mutations.push(policy);

        for changed in mutations {
            let changed =
                seal_runtime_event(changed).expect("binding mutation remains well formed");
            let mut sequence = RuntimeEventSequence::new();
            sequence.push(&events[0]).expect("run starts");
            assert_eq!(
                sequence.push(&changed),
                Err(RuntimeEventError::BindingMismatch)
            );
        }
    }

    #[test]
    fn story_21_2_subscriber_bounds_and_disconnect_are_non_authoritative() {
        let events = valid_sequence();
        let publisher = RuntimeEventPublisher::new();
        assert!(matches!(
            publisher.subscribe(0),
            Err(RuntimeEventError::SubscriberLimit)
        ));
        assert!(matches!(
            publisher.subscribe(4_097),
            Err(RuntimeEventError::SubscriberLimit)
        ));

        let subscriber = publisher.subscribe(1).expect("bounded subscriber");
        drop(subscriber);
        assert_eq!(
            publisher
                .publish(events[0].clone())
                .expect("runtime continues"),
            RuntimeEventDelivery {
                delivered: 0,
                lagged: 0,
                disconnected: 1,
            }
        );
        for event in events.into_iter().skip(1) {
            publisher
                .publish(event)
                .expect("disconnected client has no authority");
        }
        assert_eq!(publisher.event_count(), Ok(12));
    }

    #[test]
    fn story_50_2_replay_pages_reconstruct_the_exact_terminal_chain() {
        let events = valid_sequence();
        let limits = RuntimeEventBatchLimits {
            max_events: 3,
            max_bytes: MAX_RUNTIME_EVENT_BATCH_BYTES,
        };
        let mut cursor = None;
        let mut replayed = Vec::new();
        loop {
            let batch = replay_runtime_events(&events, cursor.as_ref(), limits)
                .expect("bounded replay page");
            assert!(batch.events.len() <= limits.max_events);
            assert!(batch.canonical_bytes <= limits.max_bytes);
            replayed.extend(batch.events.clone());
            cursor = batch.next_cursor;
            if !batch.has_more {
                assert!(batch.terminal);
                break;
            }
        }
        assert_eq!(replayed, events);

        let terminal_cursor = cursor.expect("terminal replay cursor");
        let empty = replay_runtime_events(&events, Some(&terminal_cursor), limits)
            .expect("terminal cursor is idempotent");
        assert!(empty.events.is_empty());
        assert_eq!(empty.next_cursor, Some(terminal_cursor));
        assert!(empty.terminal);
        assert!(!empty.has_more);
    }

    #[test]
    fn story_50_2_replay_rejects_cursor_drift_and_undersized_pages() {
        let events = valid_sequence();
        let first_bytes = agentmage_kernel_contracts::to_canonical_json(&events[0])
            .expect("fixture serializes")
            .len();
        let exact = replay_runtime_events(
            &events,
            None,
            RuntimeEventBatchLimits {
                max_events: 2,
                max_bytes: first_bytes,
            },
        )
        .expect("exact first-event byte ceiling admits");
        assert_eq!(exact.events, events[..1]);
        assert!(exact.has_more);
        assert_eq!(
            replay_runtime_events(
                &events,
                None,
                RuntimeEventBatchLimits {
                    max_events: 1,
                    max_bytes: first_bytes - 1,
                },
            ),
            Err(RuntimeEventError::BatchLimit)
        );

        let mut cursor = exact.next_cursor.expect("first cursor");
        cursor.event_sha256 = hash('f');
        assert_eq!(
            replay_runtime_events(
                &events,
                Some(&cursor),
                RuntimeEventBatchLimits {
                    max_events: 1,
                    max_bytes: MAX_RUNTIME_EVENT_BATCH_BYTES,
                },
            ),
            Err(RuntimeEventError::ReplayCursorMismatch)
        );
        assert_eq!(
            replay_runtime_events(
                &events,
                None,
                RuntimeEventBatchLimits {
                    max_events: MAX_RUNTIME_EVENT_BATCH_EVENTS + 1,
                    max_bytes: MAX_RUNTIME_EVENT_BATCH_BYTES,
                },
            ),
            Err(RuntimeEventError::BatchLimit)
        );
    }

    #[test]
    fn story_50_2_status_coalescing_never_removes_canonical_events() {
        let events = status_sequence();
        let batch = replay_runtime_events(
            &events,
            None,
            RuntimeEventBatchLimits {
                max_events: events.len(),
                max_bytes: MAX_RUNTIME_EVENT_BATCH_BYTES,
            },
        )
        .expect("status replay");
        assert_eq!(batch.events, events);
        assert_eq!(
            batch.status_summaries,
            [
                RuntimeStatusSummary::Progress {
                    code: "runtime.fixture.progress".to_owned(),
                    first_sequence: 2,
                    last_sequence: 3,
                    occurrences: 2,
                },
                RuntimeStatusSummary::Metric {
                    name: "runtime.fixture.metric".to_owned(),
                    latest_value: 2,
                    first_sequence: 4,
                    last_sequence: 5,
                    samples: 2,
                },
                RuntimeStatusSummary::Progress {
                    code: "runtime.fixture.progress".to_owned(),
                    first_sequence: 6,
                    last_sequence: 6,
                    occurrences: 1,
                },
            ]
        );
        assert!(batch.terminal);
    }

    #[test]
    fn story_50_2_slow_client_drains_then_reconnects_without_event_loss() {
        let events = valid_sequence();
        let publisher = RuntimeEventPublisher::new();
        let subscriber = publisher.subscribe(2).expect("bounded subscriber");
        for event in events.clone() {
            publisher.publish(event).expect("runtime never blocks");
        }
        let queued = subscriber
            .try_next_batch(RuntimeEventBatchLimits {
                max_events: MAX_RUNTIME_EVENT_BATCH_EVENTS,
                max_bytes: MAX_RUNTIME_EVENT_BATCH_BYTES,
            })
            .expect("queued prefix drains");
        assert_eq!(queued.events, events[..2]);
        assert!(queued.disconnected);
        assert!(!queued.terminal);

        let replay = replay_runtime_events(
            &events,
            queued.next_cursor.as_ref(),
            RuntimeEventBatchLimits {
                max_events: MAX_RUNTIME_EVENT_BATCH_EVENTS,
                max_bytes: MAX_RUNTIME_EVENT_BATCH_BYTES,
            },
        )
        .expect("exact cursor reconnects");
        let mut reconstructed = queued.events;
        reconstructed.extend(replay.events);
        assert_eq!(reconstructed, events);
        assert!(replay.terminal);
    }

    #[test]
    fn story_50_2_live_batch_stashes_the_first_event_beyond_its_byte_ceiling() {
        let events = valid_sequence();
        let publisher = RuntimeEventPublisher::new();
        let subscriber = publisher
            .subscribe(events.len())
            .expect("bounded subscriber");
        for event in events.iter().cloned() {
            publisher.publish(event).expect("publish fixture");
        }
        let first_bytes = agentmage_kernel_contracts::to_canonical_json(&events[0])
            .expect("fixture serializes")
            .len();
        let first = subscriber
            .try_next_batch(RuntimeEventBatchLimits {
                max_events: events.len(),
                max_bytes: first_bytes,
            })
            .expect("first byte-bounded batch");
        assert_eq!(first.events, events[..1]);
        assert!(first.has_more);
        let remaining = subscriber
            .try_next_batch(RuntimeEventBatchLimits {
                max_events: events.len(),
                max_bytes: MAX_RUNTIME_EVENT_BATCH_BYTES,
            })
            .expect("stashed event leads the next batch");
        assert_eq!(remaining.events, events[1..]);
        assert!(remaining.terminal);
    }
}
