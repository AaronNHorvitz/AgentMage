//! Persistent session supervision and ordered event replay.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use agentmage_kernel_contracts::{
    CONTRACT_SCHEMA_VERSION, CorrelationId, EngineeringEvent, EngineeringEventKind,
    EngineeringSessionMode, EngineeringSessionSnapshot, EngineeringTerminalState, RuntimeEventId,
    SessionId,
};
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::engineering_plan::verify_plan_approval;

const ZERO_SHA256: &str = "0000000000000000000000000000000000000000000000000000000000000000";
const MAX_SESSION_TITLE_BYTES: usize = 256;
const MAX_REPLAY_EVENTS: usize = 100_000;

/// Stable supervisor or persistence refusal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PersistentSupervisorError {
    /// A session, event, or transition violated the closed contract.
    InvalidInput,
    /// The requested session was absent.
    NotFound,
    /// The requested session identity already exists.
    Duplicate,
    /// The persistence boundary failed closed.
    Storage,
    /// The event chain or snapshot digest failed verification.
    Integrity,
    /// The requested lifecycle transition is not legal.
    TransitionDenied,
    /// A configured event or session resource ceiling was reached.
    ResourceExceeded,
}

impl PersistentSupervisorError {
    /// Returns one stable redacted error code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidInput => "engineering.supervisor.input.invalid",
            Self::NotFound => "engineering.supervisor.session.not-found",
            Self::Duplicate => "engineering.supervisor.session.duplicate",
            Self::Storage => "engineering.supervisor.storage.failed",
            Self::Integrity => "engineering.supervisor.integrity.failed",
            Self::TransitionDenied => "engineering.supervisor.transition.denied",
            Self::ResourceExceeded => "engineering.supervisor.resource.exceeded",
        }
    }
}

/// Persistence boundary for exact session snapshots and append-only events.
pub trait EngineeringSupervisorStore {
    /// Atomically inserts one new session and its initial event.
    fn create_session_with_event(
        &mut self,
        snapshot: &EngineeringSessionSnapshot,
        event: &EngineeringEvent,
    ) -> Result<(), PersistentSupervisorError>;

    /// Atomically appends one event and advances its current snapshot.
    fn append_event_and_save_session(
        &mut self,
        event: &EngineeringEvent,
        snapshot: &EngineeringSessionSnapshot,
    ) -> Result<(), PersistentSupervisorError>;

    /// Loads one current session snapshot.
    fn load_session(
        &self,
        session_id: &SessionId,
    ) -> Result<Option<EngineeringSessionSnapshot>, PersistentSupervisorError>;

    /// Lists all current session snapshots in stable identity order.
    fn list_sessions(&self) -> Result<Vec<EngineeringSessionSnapshot>, PersistentSupervisorError>;

    /// Loads all events after an exclusive sequence cursor.
    fn load_events(
        &self,
        session_id: &SessionId,
        after_sequence: Option<u64>,
    ) -> Result<Vec<EngineeringEvent>, PersistentSupervisorError>;
}

#[derive(Default)]
struct MemoryState {
    sessions: BTreeMap<SessionId, EngineeringSessionSnapshot>,
    events: BTreeMap<SessionId, Vec<EngineeringEvent>>,
}

/// Cloneable deterministic store used by supervisor and reconnection tests.
#[derive(Clone, Default)]
pub struct MemoryEngineeringSupervisorStore {
    state: Arc<Mutex<MemoryState>>,
}

impl EngineeringSupervisorStore for MemoryEngineeringSupervisorStore {
    fn create_session_with_event(
        &mut self,
        snapshot: &EngineeringSessionSnapshot,
        event: &EngineeringEvent,
    ) -> Result<(), PersistentSupervisorError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| PersistentSupervisorError::Storage)?;
        if state.sessions.contains_key(&snapshot.session_id)
            || event.session_id != snapshot.session_id
            || event.sequence != 0
        {
            return Err(PersistentSupervisorError::Duplicate);
        }
        state
            .sessions
            .insert(snapshot.session_id.clone(), snapshot.clone());
        state
            .events
            .insert(snapshot.session_id.clone(), vec![event.clone()]);
        Ok(())
    }

    fn append_event_and_save_session(
        &mut self,
        event: &EngineeringEvent,
        snapshot: &EngineeringSessionSnapshot,
    ) -> Result<(), PersistentSupervisorError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| PersistentSupervisorError::Storage)?;
        if !state.sessions.contains_key(&snapshot.session_id) {
            return Err(PersistentSupervisorError::NotFound);
        }
        let events = state.events.entry(event.session_id.clone()).or_default();
        if events.len() >= MAX_REPLAY_EVENTS {
            return Err(PersistentSupervisorError::ResourceExceeded);
        }
        if event.session_id != snapshot.session_id || event.sequence != events.len() as u64 {
            return Err(PersistentSupervisorError::Integrity);
        }
        events.push(event.clone());
        state
            .sessions
            .insert(snapshot.session_id.clone(), snapshot.clone());
        Ok(())
    }

    fn load_session(
        &self,
        session_id: &SessionId,
    ) -> Result<Option<EngineeringSessionSnapshot>, PersistentSupervisorError> {
        let state = self
            .state
            .lock()
            .map_err(|_| PersistentSupervisorError::Storage)?;
        Ok(state.sessions.get(session_id).cloned())
    }

    fn list_sessions(&self) -> Result<Vec<EngineeringSessionSnapshot>, PersistentSupervisorError> {
        let state = self
            .state
            .lock()
            .map_err(|_| PersistentSupervisorError::Storage)?;
        Ok(state.sessions.values().cloned().collect())
    }

    fn load_events(
        &self,
        session_id: &SessionId,
        after_sequence: Option<u64>,
    ) -> Result<Vec<EngineeringEvent>, PersistentSupervisorError> {
        let state = self
            .state
            .lock()
            .map_err(|_| PersistentSupervisorError::Storage)?;
        let events = state.events.get(session_id).cloned().unwrap_or_default();
        Ok(events
            .into_iter()
            .filter(|event| after_sequence.is_none_or(|after| event.sequence > after))
            .collect())
    }
}

/// Rust-owned supervisor over one durable persistence implementation.
pub struct PersistentTaskSupervisor<S>
where
    S: EngineeringSupervisorStore,
{
    store: S,
}

impl<S> PersistentTaskSupervisor<S>
where
    S: EngineeringSupervisorStore,
{
    /// Creates a supervisor over an existing persistence boundary.
    #[must_use]
    pub fn new(store: S) -> Self {
        Self { store }
    }

    /// Creates one durable session and its first hash-chained event.
    pub fn create_session(
        &mut self,
        session_id: SessionId,
        title: String,
        mode: EngineeringSessionMode,
        correlation_id: CorrelationId,
        occurred_at_epoch_ms: u64,
    ) -> Result<EngineeringSessionSnapshot, PersistentSupervisorError> {
        if session_id.as_str().is_empty()
            || title.is_empty()
            || title.len() > MAX_SESSION_TITLE_BYTES
            || title.contains('\0')
            || correlation_id.as_str().is_empty()
            || occurred_at_epoch_ms == 0
        {
            return Err(PersistentSupervisorError::InvalidInput);
        }
        let mut snapshot = EngineeringSessionSnapshot {
            schema_version: CONTRACT_SCHEMA_VERSION,
            session_id: session_id.clone(),
            title,
            mode,
            model_profile_id: None,
            endpoint_profile_id: None,
            active_task_id: None,
            artifacts: Vec::new(),
            last_event_sequence: None,
            terminal: None,
            snapshot_sha256: ZERO_SHA256.to_owned(),
        };
        let event = build_event(
            &snapshot,
            0,
            ZERO_SHA256.to_owned(),
            correlation_id,
            occurred_at_epoch_ms,
            EngineeringEventKind::SessionCreated,
        )?;
        snapshot.last_event_sequence = Some(0);
        snapshot.snapshot_sha256 = snapshot_digest(&snapshot)?;
        self.store.create_session_with_event(&snapshot, &event)?;
        Ok(snapshot)
    }

    /// Loads and verifies one reconstructable session snapshot and its complete event chain.
    pub fn open_session(
        &self,
        session_id: &SessionId,
    ) -> Result<EngineeringSessionSnapshot, PersistentSupervisorError> {
        let snapshot = self
            .store
            .load_session(session_id)?
            .ok_or(PersistentSupervisorError::NotFound)?;
        verify_snapshot(&snapshot)?;
        let events = self.store.load_events(session_id, None)?;
        verify_event_chain(&events)?;
        if snapshot.last_event_sequence != events.last().map(|event| event.sequence) {
            return Err(PersistentSupervisorError::Integrity);
        }
        Ok(snapshot)
    }

    /// Lists every verified session projection.
    pub fn list_sessions(
        &self,
    ) -> Result<Vec<EngineeringSessionSnapshot>, PersistentSupervisorError> {
        let snapshots = self.store.list_sessions()?;
        for snapshot in &snapshots {
            verify_snapshot(snapshot)?;
        }
        Ok(snapshots)
    }

    /// Replays exact ordered events after a client cursor.
    pub fn replay(
        &self,
        session_id: &SessionId,
        after_sequence: Option<u64>,
    ) -> Result<Vec<EngineeringEvent>, PersistentSupervisorError> {
        let all = self.store.load_events(session_id, None)?;
        verify_event_chain(&all)?;
        Ok(all
            .into_iter()
            .filter(|event| after_sequence.is_none_or(|after| event.sequence > after))
            .collect())
    }

    /// Appends one Rust-owned event and atomically advances the reconstructable snapshot.
    pub fn record(
        &mut self,
        session_id: &SessionId,
        correlation_id: CorrelationId,
        occurred_at_epoch_ms: u64,
        kind: EngineeringEventKind,
    ) -> Result<EngineeringEvent, PersistentSupervisorError> {
        let mut snapshot = self.open_session(session_id)?;
        self.append_event(&mut snapshot, correlation_id, occurred_at_epoch_ms, kind)
    }

    /// Pauses one non-terminal task at a safe boundary.
    pub fn pause(
        &mut self,
        session_id: &SessionId,
        correlation_id: CorrelationId,
        occurred_at_epoch_ms: u64,
    ) -> Result<EngineeringEvent, PersistentSupervisorError> {
        let snapshot = self.open_session(session_id)?;
        if snapshot.terminal.is_some() {
            return Err(PersistentSupervisorError::TransitionDenied);
        }
        self.record(
            session_id,
            correlation_id,
            occurred_at_epoch_ms,
            EngineeringEventKind::TaskPaused,
        )
    }

    /// Records one resumed safe boundary without replaying any completed action.
    pub fn resume(
        &mut self,
        session_id: &SessionId,
        correlation_id: CorrelationId,
        occurred_at_epoch_ms: u64,
    ) -> Result<EngineeringEvent, PersistentSupervisorError> {
        let snapshot = self.open_session(session_id)?;
        if snapshot.terminal.is_some() {
            return Err(PersistentSupervisorError::TransitionDenied);
        }
        self.record(
            session_id,
            correlation_id,
            occurred_at_epoch_ms,
            EngineeringEventKind::TaskResumed,
        )
    }

    /// Cancels one non-terminal task and records a truthful terminal state once.
    pub fn cancel(
        &mut self,
        session_id: &SessionId,
        correlation_id: CorrelationId,
        occurred_at_epoch_ms: u64,
    ) -> Result<EngineeringEvent, PersistentSupervisorError> {
        let snapshot = self.open_session(session_id)?;
        if snapshot.terminal.is_some() {
            return Err(PersistentSupervisorError::TransitionDenied);
        }
        self.record(
            session_id,
            correlation_id,
            occurred_at_epoch_ms,
            EngineeringEventKind::Terminal {
                state: EngineeringTerminalState::Cancelled,
            },
        )
    }

    /// Returns the persistence implementation after current in-memory ownership ends.
    pub fn into_store(self) -> S {
        self.store
    }

    fn append_event(
        &mut self,
        snapshot: &mut EngineeringSessionSnapshot,
        correlation_id: CorrelationId,
        occurred_at_epoch_ms: u64,
        kind: EngineeringEventKind,
    ) -> Result<EngineeringEvent, PersistentSupervisorError> {
        let events = self.store.load_events(&snapshot.session_id, None)?;
        verify_event_chain(&events)?;
        let sequence = events.len() as u64;
        let previous_event_sha256 = events.last().map_or_else(
            || ZERO_SHA256.to_owned(),
            |event| event.event_sha256.clone(),
        );
        let event = build_event(
            snapshot,
            sequence,
            previous_event_sha256,
            correlation_id,
            occurred_at_epoch_ms,
            kind,
        )?;
        snapshot.last_event_sequence = Some(sequence);
        if let EngineeringEventKind::Terminal { state } = &event.kind {
            snapshot.terminal = Some(*state);
        }
        snapshot.snapshot_sha256 = ZERO_SHA256.to_owned();
        snapshot.snapshot_sha256 = snapshot_digest(snapshot)?;
        self.store.append_event_and_save_session(&event, snapshot)?;
        Ok(event)
    }
}

pub(crate) fn build_event(
    snapshot: &EngineeringSessionSnapshot,
    sequence: u64,
    previous_event_sha256: String,
    correlation_id: CorrelationId,
    occurred_at_epoch_ms: u64,
    kind: EngineeringEventKind,
) -> Result<EngineeringEvent, PersistentSupervisorError> {
    if let EngineeringEventKind::PlanApproved { approval } = &kind
        && (approval.session_id != snapshot.session_id
            || approval.approved_at_epoch_ms != occurred_at_epoch_ms
            || verify_plan_approval(approval).is_err())
    {
        return Err(PersistentSupervisorError::InvalidInput);
    }
    let mut event = EngineeringEvent {
        schema_version: CONTRACT_SCHEMA_VERSION,
        event_id: RuntimeEventId::from_raw(format!(
            "engineering-event-{}-{sequence:016x}",
            snapshot.session_id.as_str()
        )),
        session_id: snapshot.session_id.clone(),
        task_id: snapshot.active_task_id.clone(),
        sequence,
        correlation_id,
        occurred_at_epoch_ms,
        kind,
        previous_event_sha256,
        event_sha256: ZERO_SHA256.to_owned(),
    };
    event.event_sha256 = event_digest(&event)?;
    Ok(event)
}

/// Verifies one complete event chain and every event digest.
pub fn verify_event_chain(events: &[EngineeringEvent]) -> Result<(), PersistentSupervisorError> {
    if events.len() > MAX_REPLAY_EVENTS {
        return Err(PersistentSupervisorError::ResourceExceeded);
    }
    let mut prior = ZERO_SHA256;
    let mut session: Option<&SessionId> = None;
    for (index, event) in events.iter().enumerate() {
        if event.schema_version != CONTRACT_SCHEMA_VERSION
            || event.sequence != index as u64
            || event.occurred_at_epoch_ms == 0
            || event.previous_event_sha256 != prior
            || session.is_some_and(|current| current != &event.session_id)
        {
            return Err(PersistentSupervisorError::Integrity);
        }
        if let EngineeringEventKind::PlanApproved { approval } = &event.kind
            && (approval.session_id != event.session_id
                || approval.approved_at_epoch_ms != event.occurred_at_epoch_ms
                || verify_plan_approval(approval).is_err())
        {
            return Err(PersistentSupervisorError::Integrity);
        }
        let mut candidate = event.clone();
        candidate.event_sha256 = ZERO_SHA256.to_owned();
        if event.event_sha256 != event_digest(&candidate)? {
            return Err(PersistentSupervisorError::Integrity);
        }
        prior = &event.event_sha256;
        session = Some(&event.session_id);
    }
    Ok(())
}

pub(crate) fn verify_snapshot(
    snapshot: &EngineeringSessionSnapshot,
) -> Result<(), PersistentSupervisorError> {
    if snapshot.schema_version != CONTRACT_SCHEMA_VERSION
        || snapshot.session_id.as_str().is_empty()
        || snapshot.title.is_empty()
        || snapshot.title.len() > MAX_SESSION_TITLE_BYTES
        || !valid_sha256(&snapshot.snapshot_sha256)
    {
        return Err(PersistentSupervisorError::Integrity);
    }
    let mut candidate = snapshot.clone();
    candidate.snapshot_sha256 = ZERO_SHA256.to_owned();
    if snapshot.snapshot_sha256 != snapshot_digest(&candidate)? {
        return Err(PersistentSupervisorError::Integrity);
    }
    Ok(())
}

fn event_digest(event: &EngineeringEvent) -> Result<String, PersistentSupervisorError> {
    canonical_sha256(event)
}

pub(crate) fn snapshot_digest(
    snapshot: &EngineeringSessionSnapshot,
) -> Result<String, PersistentSupervisorError> {
    canonical_sha256(snapshot)
}

fn canonical_sha256<T: Serialize>(value: &T) -> Result<String, PersistentSupervisorError> {
    serde_json::to_vec(value)
        .map(|bytes| sha256(&bytes))
        .map_err(|_| PersistentSupervisorError::Storage)
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn sha256(bytes: &[u8]) -> String {
    let mut digest = Sha256::new();
    digest.update(bytes);
    let digest = digest.finalize();
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(64);
    for byte in digest {
        output.push(char::from(HEX[usize::from(byte >> 4)]));
        output.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    output
}

#[cfg(test)]
mod tests {
    use super::{MemoryEngineeringSupervisorStore, PersistentTaskSupervisor};
    use agentmage_kernel_contracts::{
        CorrelationId, EngineeringSessionMode, EngineeringTerminalState, SessionId,
    };

    #[test]
    fn session_reconstructs_after_supervisor_and_view_disappear() {
        let store = MemoryEngineeringSupervisorStore::default();
        let session_id = SessionId::from_raw("session-restart");
        let mut first = PersistentTaskSupervisor::new(store.clone());
        first
            .create_session(
                session_id.clone(),
                "Persistent task".to_owned(),
                EngineeringSessionMode::Agent,
                CorrelationId::from_raw("correlation-create"),
                10,
            )
            .unwrap();
        drop(first);

        let mut resumed = PersistentTaskSupervisor::new(store);
        let snapshot = resumed.open_session(&session_id).unwrap();
        assert_eq!(snapshot.last_event_sequence, Some(0));
        resumed
            .cancel(
                &session_id,
                CorrelationId::from_raw("correlation-cancel"),
                20,
            )
            .unwrap();
        assert_eq!(
            resumed.open_session(&session_id).unwrap().terminal,
            Some(EngineeringTerminalState::Cancelled)
        );
        assert_eq!(resumed.replay(&session_id, Some(0)).unwrap().len(), 1);
    }

    #[test]
    fn terminal_session_cannot_replay_cancellation() {
        let store = MemoryEngineeringSupervisorStore::default();
        let session_id = SessionId::from_raw("session-terminal");
        let mut supervisor = PersistentTaskSupervisor::new(store);
        supervisor
            .create_session(
                session_id.clone(),
                "Terminal task".to_owned(),
                EngineeringSessionMode::Ask,
                CorrelationId::from_raw("correlation-create"),
                10,
            )
            .unwrap();
        supervisor
            .cancel(
                &session_id,
                CorrelationId::from_raw("correlation-cancel"),
                20,
            )
            .unwrap();
        assert!(
            supervisor
                .cancel(
                    &session_id,
                    CorrelationId::from_raw("correlation-replay"),
                    30,
                )
                .is_err()
        );
    }
}
