//! Durable job control ledgers in the operational store (CAP-35, Decision 0118).
//!
//! The store owner persists each job's [`JobControlLedger`] as an immutable
//! root, append-only entries and one head. Every append inserts its entry and
//! advances the head's entry count and last digest in one immediate
//! transaction whose head update names the head it read, so a committed entry
//! never exists without the head that ends it, and a writer that read an older
//! head changes nothing. Every read replays the retained entries, which must
//! reproduce exactly, be stored in their canonical encoding and end at the
//! retained head.
//!
//! The entry digests are unkeyed. The store authenticates writers by its key:
//! it opens only with the key, holds an exclusive writer lock and refuses pages
//! whose authentication fails, so no process without the store key can append,
//! remove or reorder entries. A key holder is trusted as the owner, and the
//! ledger cannot attribute an entry beyond the key (review F1 of `6355a379`).
//! Within the process, owner observations are accepted only under the job's
//! recorded owner identity, and each client request is recorded under the
//! authenticated client scope the caller supplies, never one the client names.
//! A ledger grants no authority and performs no effect.

use std::sync::{Arc, Mutex};

use rusqlite::{OptionalExtension as _, TransactionBehavior, params};

use crate::job_control::{
    JobControlDecision, JobControlError, JobControlLedger, JobControlRequest, JobLedgerEntry,
    JobLedgerHead, JobObservation, JobOwnerEvent, MAX_JOB_LEDGER_ENTRIES,
};
use crate::operational_store::OperationalStore;

/// Largest canonical encoding of one retained entry.
pub const MAX_JOB_LEDGER_ENTRY_BYTES: usize = 4_096;
const ZERO_SHA256: &str = "0000000000000000000000000000000000000000000000000000000000000000";

/// Content-free job ledger store failure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JobLedgerStoreError {
    /// No ledger exists for the job.
    NotFound,
    /// A ledger already exists for the job; nothing was written.
    Exists,
    /// The observation did not come from the job's recorded owner; nothing
    /// was written.
    NotOwner,
    /// The ledger refused the request or observation; nothing was written.
    Ledger(JobControlError),
    /// Retained rows do not form the job's exact chain. The store is poisoned.
    Integrity,
    /// The store could not read or commit; nothing was written.
    Storage,
    /// The store is poisoned or its lock is unavailable.
    Unavailable,
}

/// Shared handle to the durable job ledgers of one operational store.
///
/// It takes the store's lock for each operation, so a client control request
/// can be recorded while the runtime that shares the store is busy.
#[derive(Clone)]
pub struct DurableJobLedgers {
    store: Arc<Mutex<OperationalStore>>,
}

impl std::fmt::Debug for DurableJobLedgers {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("DurableJobLedgers")
    }
}

impl DurableJobLedgers {
    pub(crate) const fn new(store: Arc<Mutex<OperationalStore>>) -> Self {
        Self { store }
    }

    /// Queues a new job under one owner and persists its first entry.
    pub fn create(
        &self,
        job_id: &str,
        owner_id: &str,
    ) -> Result<JobObservation, JobLedgerStoreError> {
        self.with_store(|store| create(store, job_id, owner_id))
    }

    /// Current reconciled state of one job, replayed from the store.
    pub fn observation(&self, job_id: &str) -> Result<JobObservation, JobLedgerStoreError> {
        self.with_store(|store| load(store, job_id).map(|ledger| ledger.observation()))
    }

    /// Applies one client request at most once, recorded under the scope of
    /// the client the caller authenticated. A retry from the same client
    /// returns the original decision and writes nothing.
    pub fn control(
        &self,
        job_id: &str,
        client_scope: &str,
        request: &JobControlRequest,
    ) -> Result<JobControlDecision, JobLedgerStoreError> {
        self.with_store(|store| {
            let mut ledger = load(store, job_id)?;
            let before = ledger.head();
            let decision = ledger
                .control(client_scope, request)
                .map_err(JobLedgerStoreError::Ledger)?;
            append(store, job_id, &before, &ledger)?;
            Ok(decision)
        })
    }

    /// Records one observation of the job's recorded owner.
    pub fn observe_owner(
        &self,
        job_id: &str,
        owner_id: &str,
        event: JobOwnerEvent,
    ) -> Result<JobObservation, JobLedgerStoreError> {
        self.with_store(|store| {
            let mut ledger = load(store, job_id)?;
            if ledger.owner_id() != owner_id {
                return Err(JobLedgerStoreError::NotOwner);
            }
            let before = ledger.head();
            ledger
                .observe_owner(event)
                .map_err(JobLedgerStoreError::Ledger)?;
            append(store, job_id, &before, &ledger)?;
            Ok(ledger.observation())
        })
    }

    fn with_store<T>(
        &self,
        operation: impl FnOnce(&mut OperationalStore) -> Result<T, JobLedgerStoreError>,
    ) -> Result<T, JobLedgerStoreError> {
        let mut store = self
            .store
            .lock()
            .map_err(|_| JobLedgerStoreError::Unavailable)?;
        if store.poisoned {
            return Err(JobLedgerStoreError::Unavailable);
        }
        let result = operation(&mut store);
        if matches!(result, Err(JobLedgerStoreError::Integrity)) {
            store.poisoned = true;
        }
        result
    }
}

/// Replays every retained ledger; any ledger that does not reproduce exactly
/// fails the whole store.
pub(crate) fn verify_all(store: &OperationalStore) -> Result<(), JobLedgerStoreError> {
    let jobs = store
        .connection
        .prepare("SELECT job_id FROM job_ledger_roots ORDER BY job_id")
        .and_then(|mut statement| {
            statement
                .query_map([], |row| row.get::<_, String>(0))?
                .collect::<Result<Vec<_>, _>>()
        })
        .map_err(|_| JobLedgerStoreError::Storage)?;
    let heads: i64 = store
        .connection
        .query_row("SELECT COUNT(*) FROM job_ledger_heads", [], |row| {
            row.get(0)
        })
        .map_err(|_| JobLedgerStoreError::Storage)?;
    if usize::try_from(heads).ok() != Some(jobs.len()) {
        return Err(JobLedgerStoreError::Integrity);
    }
    for job in &jobs {
        load(store, job).map_err(|error| match error {
            JobLedgerStoreError::NotFound => JobLedgerStoreError::Integrity,
            other => other,
        })?;
    }
    Ok(())
}

fn create(
    store: &mut OperationalStore,
    job_id: &str,
    owner_id: &str,
) -> Result<JobObservation, JobLedgerStoreError> {
    let ledger = JobControlLedger::create(job_id, owner_id).map_err(JobLedgerStoreError::Ledger)?;
    let first = ledger
        .entries()
        .first()
        .ok_or(JobLedgerStoreError::Integrity)?;
    let bytes = encode(first)?;
    let transaction = store
        .connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|_| JobLedgerStoreError::Storage)?;
    let existing = transaction
        .query_row(
            "SELECT 1 FROM job_ledger_roots WHERE job_id = ?1",
            [job_id],
            |row| row.get::<_, i64>(0),
        )
        .optional()
        .map_err(|_| JobLedgerStoreError::Storage)?;
    if existing.is_some() {
        return Err(JobLedgerStoreError::Exists);
    }
    transaction
        .execute(
            "INSERT INTO job_ledger_roots(job_id, owner_id, created_sha256) VALUES (?1, ?2, ?3)",
            params![job_id, owner_id, &first.entry_sha256],
        )
        .map_err(|_| JobLedgerStoreError::Storage)?;
    transaction
        .execute(
            "INSERT INTO job_ledger_entries(job_id, sequence, prior_entry_sha256, entry_sha256, entry_json)
             VALUES (?1, 0, ?2, ?3, ?4)",
            params![job_id, &first.prior_entry_sha256, &first.entry_sha256, bytes],
        )
        .map_err(|_| JobLedgerStoreError::Storage)?;
    transaction
        .execute(
            "INSERT INTO job_ledger_heads(job_id, entry_count, last_sequence, head_sha256)
             VALUES (?1, 1, 0, ?2)",
            params![job_id, &first.entry_sha256],
        )
        .map_err(|_| JobLedgerStoreError::Storage)?;
    transaction
        .commit()
        .map_err(|_| JobLedgerStoreError::Storage)?;
    Ok(ledger.observation())
}

/// Persists the entries `ledger` holds after `before` together with its new
/// head. A retry that added no entry writes nothing.
fn append(
    store: &mut OperationalStore,
    job_id: &str,
    before: &JobLedgerHead,
    ledger: &JobControlLedger,
) -> Result<(), JobLedgerStoreError> {
    let start = usize::try_from(before.entry_count).map_err(|_| JobLedgerStoreError::Integrity)?;
    let added = ledger
        .entries()
        .get(start..)
        .ok_or(JobLedgerStoreError::Integrity)?;
    if added.is_empty() {
        return Ok(());
    }
    let after = ledger.head();
    let before_count = sql_integer(before.entry_count)?;
    let after_count = sql_integer(after.entry_count)?;
    let transaction = store
        .connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|_| JobLedgerStoreError::Storage)?;
    for entry in added {
        transaction
            .execute(
                "INSERT INTO job_ledger_entries(job_id, sequence, prior_entry_sha256, entry_sha256, entry_json)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    job_id,
                    sql_integer(entry.sequence)?,
                    &entry.prior_entry_sha256,
                    &entry.entry_sha256,
                    encode(entry)?
                ],
            )
            .map_err(|_| JobLedgerStoreError::Storage)?;
    }
    let changed = transaction
        .execute(
            "UPDATE job_ledger_heads SET entry_count = ?1, last_sequence = ?2, head_sha256 = ?3
             WHERE job_id = ?4 AND entry_count = ?5 AND head_sha256 = ?6",
            params![
                after_count,
                after_count - 1,
                &after.head_sha256,
                job_id,
                before_count,
                &before.head_sha256
            ],
        )
        .map_err(|_| JobLedgerStoreError::Storage)?;
    // The head was read under the same lock; any other head means the store
    // changed underneath its only writer.
    if changed != 1 {
        return Err(JobLedgerStoreError::Integrity);
    }
    transaction
        .commit()
        .map_err(|_| JobLedgerStoreError::Storage)
}

fn load(store: &OperationalStore, job_id: &str) -> Result<JobControlLedger, JobLedgerStoreError> {
    let root = store
        .connection
        .query_row(
            "SELECT owner_id, created_sha256 FROM job_ledger_roots WHERE job_id = ?1",
            [job_id],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
        )
        .optional()
        .map_err(|_| JobLedgerStoreError::Storage)?;
    let Some((owner_id, created_sha256)) = root else {
        return Err(JobLedgerStoreError::NotFound);
    };
    let (entry_count, last_sequence, head_sha256) = store
        .connection
        .query_row(
            "SELECT entry_count, last_sequence, head_sha256 FROM job_ledger_heads WHERE job_id = ?1",
            [job_id],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, String>(2)?,
                ))
            },
        )
        .optional()
        .map_err(|_| JobLedgerStoreError::Storage)?
        .ok_or(JobLedgerStoreError::Integrity)?;
    let limit = sql_integer(MAX_JOB_LEDGER_ENTRIES as u64 + 1)?;
    let rows = store
        .connection
        .prepare(
            "SELECT sequence, prior_entry_sha256, entry_sha256, entry_json
             FROM job_ledger_entries WHERE job_id = ?1 ORDER BY sequence LIMIT ?2",
        )
        .and_then(|mut statement| {
            statement
                .query_map(params![job_id, limit], |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, Vec<u8>>(3)?,
                    ))
                })?
                .collect::<Result<Vec<_>, _>>()
        })
        .map_err(|_| JobLedgerStoreError::Storage)?;
    if rows.len() > MAX_JOB_LEDGER_ENTRIES || last_sequence != entry_count - 1 {
        return Err(JobLedgerStoreError::Integrity);
    }
    let mut entries = Vec::with_capacity(rows.len());
    let mut prior = ZERO_SHA256.to_owned();
    for (index, (sequence, prior_entry_sha256, entry_sha256, bytes)) in rows.into_iter().enumerate()
    {
        if bytes.len() > MAX_JOB_LEDGER_ENTRY_BYTES {
            return Err(JobLedgerStoreError::Integrity);
        }
        let entry: JobLedgerEntry =
            serde_json::from_slice(&bytes).map_err(|_| JobLedgerStoreError::Integrity)?;
        if usize::try_from(sequence).ok() != Some(index)
            || usize::try_from(entry.sequence).ok() != Some(index)
            || entry.job_id != job_id
            || prior_entry_sha256 != prior
            || entry.prior_entry_sha256 != prior_entry_sha256
            || entry.entry_sha256 != entry_sha256
            || encode(&entry)? != bytes
        {
            return Err(JobLedgerStoreError::Integrity);
        }
        prior = entry_sha256;
        entries.push(entry);
    }
    if entries.first().map(|entry| entry.entry_sha256.as_str()) != Some(created_sha256.as_str()) {
        return Err(JobLedgerStoreError::Integrity);
    }
    let retained = JobLedgerHead {
        entry_count: u64::try_from(entry_count).map_err(|_| JobLedgerStoreError::Integrity)?,
        head_sha256,
    };
    let ledger = JobControlLedger::replay(job_id, &entries, &retained)
        .map_err(|_| JobLedgerStoreError::Integrity)?;
    if ledger.owner_id() != owner_id {
        return Err(JobLedgerStoreError::Integrity);
    }
    Ok(ledger)
}

fn encode(entry: &JobLedgerEntry) -> Result<Vec<u8>, JobLedgerStoreError> {
    serde_json::to_vec(entry)
        .ok()
        .filter(|bytes| bytes.len() <= MAX_JOB_LEDGER_ENTRY_BYTES)
        .ok_or(JobLedgerStoreError::Storage)
}

fn sql_integer(value: u64) -> Result<i64, JobLedgerStoreError> {
    i64::try_from(value).map_err(|_| JobLedgerStoreError::Integrity)
}

#[cfg(test)]
#[path = "job_ledger_store_tests.rs"]
mod tests;
