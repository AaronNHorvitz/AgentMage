//! Durable run effect records and the recoverability of a whole coding
//! session (Decision 0143).
//!
//! The effect owner of a coding run, the Linux coding boundary, keeps each
//! executed call and each change record it published in memory and then in
//! the run's chain in the operational store. At the end of each run it
//! declares the recoverability of every effect of the session: the stored
//! chain of every run of the session, oldest first, with each change record
//! read back from the canonical artifact store under the run and policy
//! revision that published it, assessed against the held worktree's current
//! bytes. A session is never declared when one of its chains is incomplete,
//! when a run other than the declaring one was not released, when a record
//! cannot be read or verified, or when it holds more than one declaration
//! assesses.
//!
//! Classification only, as for a run (Decision 0116): a declaration grants
//! nothing and never reports an effect as undone.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use agentmage_kernel_contracts::{RuntimeArtifactRef, RuntimeRunRequest};
use agentmage_kernel_engine::run_effect_record_store::{
    DurableRunEffectRecords, RunEffectEntry, RunEffectKind, RunEffectRecordStoreError,
    StoredRunEffects,
};

use crate::coding_action_history::StoredChainStart;
use crate::coding_history::{
    CODING_CHANGE_RECORD_MEDIA_TYPE, CodingChangeRecord, RetainedCodingChange,
    verify_retained_change,
};
use crate::coding_recoverability::{
    ExecutedEffect, ExecutedEffectKind, ExecutedEffectOutcome, RecoverabilityError,
    RecoverabilityReport, assess_recoverability, run_effects,
};

/// Owner identity of the coding host in every run effect chain, the identity
/// it already uses for job ledgers and run action histories.
pub const RUN_EFFECT_RECORD_OWNER: &str = "agentmage-coding-host";
/// Largest change record one session declaration reads.
pub const MAX_CHANGE_RECORD_READ_BYTES: u64 = 4 * 1024 * 1024;
/// Most change record bytes one session declaration reads in total.
pub const MAX_SESSION_CHANGE_RECORD_BYTES: u64 = 32 * 1024 * 1024;

/// What the host knows about one run's stored chain that the store may not
/// hold yet (review F1 of `6c0f51fe`). Every handle of the run shares it: the
/// recorder that appends, the service that closes and, across an in-host
/// continuation, the run's next recorder.
#[derive(Debug, Default)]
struct ChainKnowledge {
    /// An entry this host kept in memory did not reach the store.
    missed: AtomicBool,
    /// The store holds the chain's incomplete mark.
    marked: AtomicBool,
}

/// The store writes one run's chain takes. The coding host writes through the
/// operational store; a test refuses writes as a failing disk would.
trait ChainWrites: std::fmt::Debug + Send + Sync {
    fn append(&self, run_id: &str, entry: &RunEffectEntry)
    -> Result<(), RunEffectRecordStoreError>;
    fn mark_incomplete(&self, run_id: &str) -> Result<(), RunEffectRecordStoreError>;
    fn close(&self, run_id: &str) -> Result<(), RunEffectRecordStoreError>;
}

impl ChainWrites for DurableRunEffectRecords {
    fn append(
        &self,
        run_id: &str,
        entry: &RunEffectEntry,
    ) -> Result<(), RunEffectRecordStoreError> {
        Self::append(self, run_id, RUN_EFFECT_RECORD_OWNER, entry).map(|_| ())
    }

    fn mark_incomplete(&self, run_id: &str) -> Result<(), RunEffectRecordStoreError> {
        Self::mark_incomplete(self, run_id, RUN_EFFECT_RECORD_OWNER)
    }

    fn close(&self, run_id: &str) -> Result<(), RunEffectRecordStoreError> {
        Self::close(self, run_id, RUN_EFFECT_RECORD_OWNER).map(|_| ())
    }
}

/// Store writes refused while `refuse` is set, as a failing disk would refuse
/// them, and otherwise the operational store's; for the host's tests.
#[cfg(test)]
#[derive(Debug)]
struct RefusingWrites {
    records: DurableRunEffectRecords,
    refuse: Arc<AtomicBool>,
}

#[cfg(test)]
impl RefusingWrites {
    fn refused(&self) -> Result<(), RunEffectRecordStoreError> {
        if self.refuse.load(Ordering::SeqCst) {
            Err(RunEffectRecordStoreError::Storage)
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
impl ChainWrites for RefusingWrites {
    fn append(
        &self,
        run_id: &str,
        entry: &RunEffectEntry,
    ) -> Result<(), RunEffectRecordStoreError> {
        self.refused()?;
        ChainWrites::append(&self.records, run_id, entry)
    }

    fn mark_incomplete(&self, run_id: &str) -> Result<(), RunEffectRecordStoreError> {
        self.refused()?;
        ChainWrites::mark_incomplete(&self.records, run_id)
    }

    fn close(&self, run_id: &str) -> Result<(), RunEffectRecordStoreError> {
        self.refused()?;
        ChainWrites::close(&self.records, run_id)
    }
}

/// The stored chain a run's effect recorder also appends to, and the handle
/// the host service closes it with. Clones share what the host knows about
/// the chain.
#[derive(Clone, Debug)]
pub struct PersistedRunEffects {
    records: DurableRunEffectRecords,
    writes: Arc<dyn ChainWrites>,
    run_id: String,
    knowledge: Arc<ChainKnowledge>,
}

/// What the host knows about a run's stored chain, kept without the store
/// while an in-host continuation reopens it (Decision 0145).
#[derive(Clone, Debug)]
pub struct RunEffectChainKnowledge(Arc<ChainKnowledge>);

impl PersistedRunEffects {
    /// Creates or attaches to the chain of the request's run under the
    /// host's owner identity. A run resumed after a restart is marked
    /// incomplete, because the host that ended may have missed an entry
    /// between an effect and its record. A run recorded before schema 25 has
    /// no chain; it begins here, marked incomplete, so its session is never
    /// declared from a part of its effects. A chain naming another session or
    /// task is refused before anything is marked.
    pub fn begin(
        records: DurableRunEffectRecords,
        request: &RuntimeRunRequest,
        start: StoredChainStart,
    ) -> Result<Self, RunEffectRecordStoreError> {
        let (session_id, task_id, run_id) = (
            request.session_id.as_str(),
            request.task.task_id.as_str(),
            request.run_id.as_str(),
        );
        let after_restart = matches!(
            start,
            StoredChainStart::AfterRestart | StoredChainStart::AfterRestartStoredFirst
        );
        let attached = match start {
            StoredChainStart::New => {
                records.create(session_id, task_id, run_id, RUN_EFFECT_RECORD_OWNER)?;
                None
            }
            StoredChainStart::ContinueInHost
            | StoredChainStart::AfterRestart
            | StoredChainStart::AfterRestartStoredFirst => {
                match records.attach(run_id, RUN_EFFECT_RECORD_OWNER, false) {
                    Ok(stored) => Some(stored),
                    Err(RunEffectRecordStoreError::NotFound) if after_restart => {
                        records.create(session_id, task_id, run_id, RUN_EFFECT_RECORD_OWNER)?;
                        records.mark_incomplete(run_id, RUN_EFFECT_RECORD_OWNER)?;
                        None
                    }
                    Err(error) => return Err(error),
                }
            }
        };
        // A run belongs to the one session and task its chain names.
        if let Some(stored) = attached {
            if stored.session_id != session_id || stored.task_id != task_id {
                return Err(RunEffectRecordStoreError::InvalidInput);
            }
            if after_restart {
                records.mark_incomplete(run_id, RUN_EFFECT_RECORD_OWNER)?;
            }
        }
        Ok(Self {
            writes: Arc::new(records.clone()),
            records,
            run_id: run_id.to_owned(),
            knowledge: Arc::default(),
        })
    }

    /// Appends one entry, or records that it was missed and marks the stored
    /// chain incomplete. A mark the store refused earlier is retried first.
    pub(crate) fn append(&self, entry: &RunEffectEntry) {
        self.store_pending_mark();
        if self.writes.append(&self.run_id, entry).is_err() {
            self.mark_incomplete();
        }
    }

    /// Records that this owner missed an entry and marks the stored chain
    /// incomplete. A mark the store refuses stays pending: each later append
    /// and the closing retry it, and the chain is never closed without it.
    pub(crate) fn mark_incomplete(&self) {
        self.knowledge.missed.store(true, Ordering::SeqCst);
        self.store_pending_mark();
    }

    /// Stores a pending incomplete mark; true when none is pending.
    fn store_pending_mark(&self) -> bool {
        if !self.knowledge.missed.load(Ordering::SeqCst)
            || self.knowledge.marked.load(Ordering::SeqCst)
        {
            return true;
        }
        if self.writes.mark_incomplete(&self.run_id).is_err() {
            eprintln!("coding.recoverability.store-mark-failed");
            return false;
        }
        self.knowledge.marked.store(true, Ordering::SeqCst);
        true
    }

    /// Whether every entry this owner kept reached the store.
    #[must_use]
    pub fn kept_every_entry(&self) -> bool {
        !self.knowledge.missed.load(Ordering::SeqCst)
    }

    /// Closes the chain because its ended run was released. A chain that
    /// missed an entry closes only after the store holds its incomplete mark;
    /// otherwise it stays open, and every later declaration of its session
    /// is refused because an earlier run was never released (Decision 0145).
    /// The service then refuses the release and keeps the run held, so a
    /// later release retries (Decision 0146).
    pub fn close_released(&self) -> Result<(), RunEffectRecordStoreError> {
        if !self.store_pending_mark() {
            return Err(RunEffectRecordStoreError::Storage);
        }
        self.writes.close(&self.run_id)
    }

    /// What the host knows about this chain, kept while the store is closed
    /// for an in-host continuation of the run.
    #[must_use]
    pub fn knowledge(&self) -> RunEffectChainKnowledge {
        RunEffectChainKnowledge(Arc::clone(&self.knowledge))
    }

    /// Continues what the host knew about the same run's chain before an
    /// in-host continuation reopened the store: a miss carries over, and a
    /// mark the store did not take is retried.
    pub fn continue_from(&self, earlier: &RunEffectChainKnowledge) {
        if earlier.0.missed.load(Ordering::SeqCst) {
            if earlier.0.marked.load(Ordering::SeqCst) {
                self.knowledge.marked.store(true, Ordering::SeqCst);
            }
            self.mark_incomplete();
        }
    }

    /// Every stored run of one session, oldest first.
    pub fn session(
        &self,
        session_id: &str,
    ) -> Result<Vec<StoredRunEffects>, RunEffectRecordStoreError> {
        self.records.session(session_id)
    }
}

#[cfg(test)]
impl PersistedRunEffects {
    /// This handle with its store writes refused while `refuse` is set, for
    /// the host's tests (review F1 of `6c0f51fe`).
    pub(crate) fn refusing_writes(self, refuse: &Arc<AtomicBool>) -> Self {
        Self {
            writes: Arc::new(RefusingWrites {
                records: self.records.clone(),
                refuse: Arc::clone(refuse),
            }),
            ..self
        }
    }
}

/// The stored entry of one executed call.
#[must_use]
pub fn execution_entry(effect: &ExecutedEffect) -> RunEffectEntry {
    let (kind, path) = match &effect.kind {
        ExecutedEffectKind::Write => (RunEffectKind::Write, None),
        ExecutedEffectKind::Create { path } => (RunEffectKind::Create, Some(path.clone())),
        ExecutedEffectKind::Command => (RunEffectKind::Command, None),
        ExecutedEffectKind::NoEffect => (RunEffectKind::NoEffect, None),
    };
    let (outcome, changed) = match effect.outcome {
        ExecutedEffectOutcome::Completed { outcome, changed } => (Some(outcome), changed),
        ExecutedEffectOutcome::Unknown => (None, false),
    };
    RunEffectEntry::Execution {
        operation_id: effect.operation_id.clone(),
        kind,
        path,
        outcome,
        changed,
    }
}

fn executed_effect(entry: &RunEffectEntry) -> Option<ExecutedEffect> {
    let RunEffectEntry::Execution {
        operation_id,
        kind,
        path,
        outcome,
        changed,
    } = entry
    else {
        return None;
    };
    let kind = match (kind, path) {
        (RunEffectKind::Write, None) => ExecutedEffectKind::Write,
        (RunEffectKind::Create, Some(path)) => ExecutedEffectKind::Create { path: path.clone() },
        (RunEffectKind::Command, None) => ExecutedEffectKind::Command,
        (RunEffectKind::NoEffect, None) => ExecutedEffectKind::NoEffect,
        _ => return None,
    };
    let outcome = match outcome {
        Some(outcome) => ExecutedEffectOutcome::Completed {
            outcome: *outcome,
            changed: *changed,
        },
        None if !changed => ExecutedEffectOutcome::Unknown,
        None => return None,
    };
    Some(ExecutedEffect {
        operation_id: operation_id.clone(),
        kind,
        outcome,
    })
}

/// Reads one published change record's exact bytes under the run that
/// published it and the policy revision it was published under, or `None`
/// when the record cannot be read.
pub type ChangeRecordReader<'reader> =
    dyn Fn(&StoredRunEffects, &RuntimeArtifactRef, &str) -> Option<Vec<u8>> + 'reader;

/// Declares the recoverability of every effect of the declaring run's
/// session from the stored chains of its runs, oldest first (Decision 0143).
///
/// The declaring run must be the session's last stored run and still open;
/// every earlier run must have been released, and every chain must be
/// complete. Each run's effect list is built as for a run declaration
/// (Decision 0116) from its own executions and the change records it
/// published, each read back and verified as the record of that run, session
/// and task. The report names no run, so it reads as a declaration about the
/// whole session.
pub fn declare_session_recoverability(
    runs: &[StoredRunEffects],
    request: &RuntimeRunRequest,
    read_change: &ChangeRecordReader<'_>,
    current_sha256: &dyn Fn(&[String]) -> Option<String>,
) -> Result<RecoverabilityReport, RecoverabilityError> {
    let session_id = request.session_id.as_str();
    let task_id = request.task.task_id.as_str();
    let Some((declaring, earlier)) = runs.split_last() else {
        return Err(RecoverabilityError::Invalid);
    };
    if declaring.run_id != request.run_id.as_str()
        || declaring.closed
        || earlier.iter().any(|run| !run.closed)
        || runs.iter().zip(1_u64..).any(|(run, position)| {
            !run.complete
                || run.session_id != session_id
                || run.task_id != task_id
                || run.session_position != position
        })
    {
        return Err(RecoverabilityError::Invalid);
    }
    let mut effects = Vec::new();
    let mut read_bytes = 0_u64;
    for run in runs {
        let mut executed = Vec::new();
        let mut changes = Vec::new();
        for entry in &run.entries {
            let RunEffectEntry::Publication {
                reference,
                policy_sha256,
            } = entry
            else {
                executed.push(executed_effect(entry).ok_or(RecoverabilityError::Invalid)?);
                continue;
            };
            read_bytes = read_bytes
                .checked_add(reference.byte_size)
                .filter(|total| {
                    reference.byte_size <= MAX_CHANGE_RECORD_READ_BYTES
                        && *total <= MAX_SESSION_CHANGE_RECORD_BYTES
                })
                .ok_or(RecoverabilityError::Invalid)?;
            if reference.media_type != CODING_CHANGE_RECORD_MEDIA_TYPE {
                return Err(RecoverabilityError::ForeignOrInvalid);
            }
            let bytes =
                read_change(run, reference, policy_sha256).ok_or(RecoverabilityError::Invalid)?;
            let change = RetainedCodingChange {
                reference: reference.clone(),
                record: serde_json::from_slice::<CodingChangeRecord>(&bytes)
                    .map_err(|_| RecoverabilityError::ForeignOrInvalid)?,
            };
            verify_retained_change(&change).map_err(|_| RecoverabilityError::ForeignOrInvalid)?;
            if change.record.session_id != session_id
                || change.record.task_id != task_id
                || change.record.producer_run_id != run.run_id
            {
                return Err(RecoverabilityError::ForeignOrInvalid);
            }
            changes.push(change);
        }
        effects.extend(run_effects(&executed, &changes)?);
    }
    assess_recoverability(session_id, task_id, &effects, current_sha256)
}

#[cfg(all(
    test,
    target_os = "linux",
    feature = "source-artifacts",
    feature = "workflow-supervisor"
))]
#[path = "coding_session_recoverability_tests.rs"]
mod tests;
