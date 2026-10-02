//! Durable run effect records in the operational store (Decision 0143).
//!
//! The effect owner of a coding run records what each executed call could do
//! and how it ended, and each change record it published, so the
//! recoverability of a whole session can be declared after the host that ran
//! an earlier run has ended. Each run has an immutable root that names its
//! session, task, owner and position in the session, one record per position
//! and one head. Every append replays the stored chain and inserts the record
//! and advances the head in one immediate transaction whose head update names
//! the head it read, so a committed record never exists without the head that
//! ends it. Every read and every store open replays every chain: each record
//! must decode to a closed entry, be stored in its canonical encoding and
//! chain to the retained head. A head's complete flag can only be cleared and
//! its closed flag only set; a closed chain takes no further record.
//!
//! As for the run action histories (Decision 0129), the store authenticates
//! writers by its key, and within the process only a run's recorded owner may
//! append to, mark or close its chain. A record grants no authority, performs
//! no effect and says nothing about the current state of a file.

use std::sync::{Arc, Mutex};

use agentmage_kernel_contracts::{OperationOutcome, RuntimeArtifactRef};
use rusqlite::{OptionalExtension as _, TransactionBehavior, params};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::operational_store::OperationalStore;

/// Most positions one run's chain holds.
pub const MAX_RUN_EFFECT_POSITIONS: usize = 1_024;
/// Largest canonical encoding of one stored entry.
pub const MAX_RUN_EFFECT_RECORD_BYTES: usize = 4_096;
/// Most runs one session holds.
pub const MAX_SESSION_RUNS: usize = 64;
/// Most positions one session read loads across its runs.
pub const MAX_SESSION_POSITIONS: usize = 2_048;
const GENESIS_SHA256: &str = "0000000000000000000000000000000000000000000000000000000000000000";
const ENTRY_DOMAIN: &str = "agentmage-run-effect-record";
const ENTRY_VERSION: u16 = 1;
const MAX_IDENTIFIER_BYTES: usize = 128;
const MAX_PATH_COMPONENTS: usize = 64;
const MAX_PATH_COMPONENT_BYTES: usize = 255;

/// What one executed call can do to the workspace.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RunEffectKind {
    /// A write of an existing file: a patch, a selected write or an inverse write.
    Write,
    /// A controlled creation of the entry's path.
    Create,
    /// A registered command or validation run.
    Command,
    /// A read, an inspection or a history read.
    NoEffect,
}

/// One stored entry: an executed call or a published change record.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "entry", rename_all = "kebab-case", deny_unknown_fields)]
pub enum RunEffectEntry {
    /// One call whose authority was consumed.
    Execution {
        /// Runtime operation identity of the call.
        operation_id: String,
        /// What the call can do.
        kind: RunEffectKind,
        /// The created path's canonical components; present only for a creation.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        path: Option<Vec<String>>,
        /// The result's outcome, or none when the call failed after its
        /// authority was consumed and its effect is unknown.
        outcome: Option<OperationOutcome>,
        /// Whether the result reported a state change; false when unknown.
        changed: bool,
    },
    /// One change record the owner published and verified.
    Publication {
        /// Exact canonical reference of the change record.
        reference: RuntimeArtifactRef,
        /// Policy revision the record was published under, which a read of
        /// the record must name.
        policy_sha256: String,
    },
}

impl RunEffectEntry {
    fn validate(&self) -> Result<(), RunEffectRecordStoreError> {
        let valid = match self {
            Self::Execution {
                operation_id,
                kind,
                path,
                outcome,
                changed,
            } => {
                identifier(operation_id)
                    && match (kind, path) {
                        (RunEffectKind::Create, Some(path)) => valid_path(path),
                        (_, None) => !matches!(kind, RunEffectKind::Create),
                        _ => false,
                    }
                    && (outcome.is_some() || !changed)
            }
            Self::Publication {
                reference,
                policy_sha256,
            } => {
                digest(policy_sha256)
                    && identifier(reference.artifact_id.as_str())
                    && digest(&reference.manifest_sha256)
                    && digest(&reference.payload_sha256)
                    && reference.byte_size > 0
                    && !reference.media_type.is_empty()
                    && reference.media_type.len() <= MAX_IDENTIFIER_BYTES
            }
        };
        if valid {
            Ok(())
        } else {
            Err(RunEffectRecordStoreError::InvalidInput)
        }
    }
}

/// Content-free run effect record store failure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RunEffectRecordStoreError {
    /// An identity or entry is malformed; nothing was read or written.
    InvalidInput,
    /// No chain exists for the run.
    NotFound,
    /// The run already has a chain; nothing was written.
    Exists,
    /// The caller is not the chain's recorded owner; nothing was written.
    NotOwner,
    /// The chain is closed and takes no further change; nothing was written.
    Closed,
    /// The chain, or the session, holds its most positions or runs; nothing
    /// was written or loaded.
    Full,
    /// Retained rows do not form the chain's exact history. The store is poisoned.
    Integrity,
    /// The store could not read or commit; nothing was written.
    Storage,
    /// The store is poisoned or its lock is unavailable.
    Unavailable,
}

/// One run's stored chain, replayed to its head.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StoredRunEffects {
    /// Session the run belongs to.
    pub session_id: String,
    /// Task the run belongs to.
    pub task_id: String,
    /// The run.
    pub run_id: String,
    /// One-based position of the run in its session.
    pub session_position: u64,
    /// Every entry in order.
    pub entries: Vec<RunEffectEntry>,
    /// Number of entries the head counts.
    pub entry_count: u64,
    /// Digest of the last entry, or the genesis digest.
    pub head_sha256: String,
    /// False once the owner knew it missed an entry, or once a run was
    /// resumed after the host that kept the chain ended.
    pub complete: bool,
    /// Whether the run was released and the chain takes no further record.
    pub closed: bool,
}

/// Shared handle to the durable run effect records of one operational store.
///
/// It takes the store's lock for each operation, like the run action histories.
#[derive(Clone)]
pub struct DurableRunEffectRecords {
    store: Arc<Mutex<OperationalStore>>,
}

impl std::fmt::Debug for DurableRunEffectRecords {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("DurableRunEffectRecords")
    }
}

impl DurableRunEffectRecords {
    pub(crate) const fn new(store: Arc<Mutex<OperationalStore>>) -> Self {
        Self { store }
    }

    /// Creates one empty, complete, open chain for a run under its owner, at
    /// the next position of its session.
    pub fn create(
        &self,
        session_id: &str,
        task_id: &str,
        run_id: &str,
        owner_id: &str,
    ) -> Result<(), RunEffectRecordStoreError> {
        if ![session_id, task_id, run_id, owner_id]
            .into_iter()
            .all(identifier)
        {
            return Err(RunEffectRecordStoreError::InvalidInput);
        }
        self.with_store(|store| create(store, session_id, task_id, run_id, owner_id))
    }

    /// Attaches the owner to an existing open chain, marking it incomplete
    /// first when the owner may have missed an entry.
    pub fn attach(
        &self,
        run_id: &str,
        owner_id: &str,
        mark_incomplete: bool,
    ) -> Result<StoredRunEffects, RunEffectRecordStoreError> {
        validate_identity(run_id, owner_id)?;
        self.with_store(|store| {
            let loaded = owned_open(store, run_id, owner_id)?;
            if mark_incomplete && loaded.stored.complete {
                set_head(store, &loaded.stored, Change::ClearComplete)?;
            }
            Ok(load(store, run_id)?.stored)
        })
    }

    /// Appends one entry to the owner's open chain and returns the new head.
    pub fn append(
        &self,
        run_id: &str,
        owner_id: &str,
        entry: &RunEffectEntry,
    ) -> Result<(u64, String), RunEffectRecordStoreError> {
        validate_identity(run_id, owner_id)?;
        entry.validate()?;
        self.with_store(|store| append(store, run_id, owner_id, entry))
    }

    /// Records that the owner missed an entry of its open chain.
    pub fn mark_incomplete(
        &self,
        run_id: &str,
        owner_id: &str,
    ) -> Result<(), RunEffectRecordStoreError> {
        validate_identity(run_id, owner_id)?;
        self.with_store(|store| {
            let loaded = owned_open(store, run_id, owner_id)?;
            if loaded.stored.complete {
                set_head(store, &loaded.stored, Change::ClearComplete)?;
            }
            Ok(())
        })
    }

    /// Closes the owner's chain because its run was released. Closing a
    /// closed chain writes nothing.
    pub fn close(
        &self,
        run_id: &str,
        owner_id: &str,
    ) -> Result<StoredRunEffects, RunEffectRecordStoreError> {
        validate_identity(run_id, owner_id)?;
        self.with_store(|store| {
            let loaded = load(store, run_id)?;
            if loaded.owner_id != owner_id {
                return Err(RunEffectRecordStoreError::NotOwner);
            }
            if !loaded.stored.closed {
                set_head(store, &loaded.stored, Change::Close)?;
            }
            Ok(load(store, run_id)?.stored)
        })
    }

    /// One run's chain, replayed from the store.
    pub fn run(&self, run_id: &str) -> Result<StoredRunEffects, RunEffectRecordStoreError> {
        if !identifier(run_id) {
            return Err(RunEffectRecordStoreError::InvalidInput);
        }
        self.with_store(|store| load(store, run_id).map(|loaded| loaded.stored))
    }

    /// Every run of one session in position order, each replayed from the
    /// store. A session with more runs, or more positions across its runs,
    /// than one read loads is refused as full.
    pub fn session(
        &self,
        session_id: &str,
    ) -> Result<Vec<StoredRunEffects>, RunEffectRecordStoreError> {
        if !identifier(session_id) {
            return Err(RunEffectRecordStoreError::InvalidInput);
        }
        self.with_store(|store| session(store, session_id))
    }

    fn with_store<T>(
        &self,
        operation: impl FnOnce(&mut OperationalStore) -> Result<T, RunEffectRecordStoreError>,
    ) -> Result<T, RunEffectRecordStoreError> {
        let mut store = self
            .store
            .lock()
            .map_err(|_| RunEffectRecordStoreError::Unavailable)?;
        if store.poisoned {
            return Err(RunEffectRecordStoreError::Unavailable);
        }
        let result = operation(&mut store);
        if matches!(result, Err(RunEffectRecordStoreError::Integrity)) {
            store.poisoned = true;
        }
        result
    }
}

/// Replays every retained chain; any chain that does not reproduce exactly
/// fails the whole store.
pub(crate) fn verify_all(store: &OperationalStore) -> Result<(), RunEffectRecordStoreError> {
    let roots = store
        .connection
        .prepare("SELECT run_id FROM run_effect_record_roots ORDER BY run_id")
        .and_then(|mut statement| {
            statement
                .query_map([], |row| row.get::<_, String>(0))?
                .collect::<Result<Vec<_>, _>>()
        })
        .map_err(|_| RunEffectRecordStoreError::Storage)?;
    let heads: i64 = store
        .connection
        .query_row("SELECT COUNT(*) FROM run_effect_record_heads", [], |row| {
            row.get(0)
        })
        .map_err(|_| RunEffectRecordStoreError::Storage)?;
    if usize::try_from(heads).ok() != Some(roots.len()) {
        return Err(RunEffectRecordStoreError::Integrity);
    }
    // A record outside every chain has no root, which the store's foreign key
    // check refuses at open; each chain's own records are replayed here.
    for run_id in &roots {
        load(store, run_id).map_err(|error| match error {
            RunEffectRecordStoreError::NotFound => RunEffectRecordStoreError::Integrity,
            other => other,
        })?;
    }
    Ok(())
}

/// Digest of one entry at its position, chained to the head before it.
#[must_use]
pub fn run_effect_entry_sha256(
    run_id: &str,
    sequence: u64,
    previous_sha256: &str,
    entry: &RunEffectEntry,
) -> Option<String> {
    let preimage = serde_json::to_vec(&(
        ENTRY_DOMAIN,
        ENTRY_VERSION,
        run_id,
        sequence,
        previous_sha256,
        entry,
    ))
    .ok()?;
    Some(sha256_hex(&preimage))
}

struct Loaded {
    owner_id: String,
    stored: StoredRunEffects,
}

#[derive(Clone, Copy)]
enum Change {
    ClearComplete,
    Close,
}

fn identifier(value: &str) -> bool {
    !value.is_empty() && value.len() <= MAX_IDENTIFIER_BYTES && !value.contains('\0')
}

fn digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn valid_path(path: &[String]) -> bool {
    !path.is_empty()
        && path.len() <= MAX_PATH_COMPONENTS
        && path.iter().all(|component| {
            !component.is_empty()
                && component.len() <= MAX_PATH_COMPONENT_BYTES
                && component != "."
                && component != ".."
                && !component.contains(['/', '\0'])
        })
}

fn validate_identity(run_id: &str, owner_id: &str) -> Result<(), RunEffectRecordStoreError> {
    if identifier(run_id) && identifier(owner_id) {
        Ok(())
    } else {
        Err(RunEffectRecordStoreError::InvalidInput)
    }
}

fn create(
    store: &mut OperationalStore,
    session_id: &str,
    task_id: &str,
    run_id: &str,
    owner_id: &str,
) -> Result<(), RunEffectRecordStoreError> {
    let transaction = store
        .connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|_| RunEffectRecordStoreError::Storage)?;
    let existing = transaction
        .query_row(
            "SELECT 1 FROM run_effect_record_roots WHERE run_id = ?1",
            params![run_id],
            |row| row.get::<_, i64>(0),
        )
        .optional()
        .map_err(|_| RunEffectRecordStoreError::Storage)?;
    if existing.is_some() {
        return Err(RunEffectRecordStoreError::Exists);
    }
    let runs: i64 = transaction
        .query_row(
            "SELECT COUNT(*) FROM run_effect_record_roots WHERE session_id = ?1",
            params![session_id],
            |row| row.get(0),
        )
        .map_err(|_| RunEffectRecordStoreError::Storage)?;
    let runs = usize::try_from(runs).map_err(|_| RunEffectRecordStoreError::Integrity)?;
    if runs >= MAX_SESSION_RUNS {
        return Err(RunEffectRecordStoreError::Full);
    }
    transaction
        .execute(
            "INSERT INTO run_effect_record_roots(run_id, session_id, task_id, owner_id, session_position)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![run_id, session_id, task_id, owner_id, sql_integer(runs as u64 + 1)?],
        )
        .map_err(|_| RunEffectRecordStoreError::Storage)?;
    transaction
        .execute(
            "INSERT INTO run_effect_record_heads(run_id, entry_count, head_sha256, complete, closed)
             VALUES (?1, 0, ?2, 1, 0)",
            params![run_id, GENESIS_SHA256],
        )
        .map_err(|_| RunEffectRecordStoreError::Storage)?;
    transaction
        .commit()
        .map_err(|_| RunEffectRecordStoreError::Storage)
}

/// Loads the owner's chain, refusing another owner and a closed chain.
fn owned_open(
    store: &OperationalStore,
    run_id: &str,
    owner_id: &str,
) -> Result<Loaded, RunEffectRecordStoreError> {
    let loaded = load(store, run_id)?;
    if loaded.owner_id != owner_id {
        return Err(RunEffectRecordStoreError::NotOwner);
    }
    if loaded.stored.closed {
        return Err(RunEffectRecordStoreError::Closed);
    }
    Ok(loaded)
}

fn append(
    store: &mut OperationalStore,
    run_id: &str,
    owner_id: &str,
    entry: &RunEffectEntry,
) -> Result<(u64, String), RunEffectRecordStoreError> {
    let Loaded { stored, .. } = owned_open(store, run_id, owner_id)?;
    if stored.entries.len() >= MAX_RUN_EFFECT_POSITIONS {
        return Err(RunEffectRecordStoreError::Full);
    }
    let bytes = encode(entry)?;
    let sequence = stored.entry_count + 1;
    let head_sha256 = run_effect_entry_sha256(run_id, sequence, &stored.head_sha256, entry)
        .ok_or(RunEffectRecordStoreError::Storage)?;
    let transaction = store
        .connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|_| RunEffectRecordStoreError::Storage)?;
    transaction
        .execute(
            "INSERT INTO run_effect_records(run_id, sequence, entry_sha256, record_json)
             VALUES (?1, ?2, ?3, ?4)",
            params![run_id, sql_integer(sequence)?, &head_sha256, bytes],
        )
        .map_err(|_| RunEffectRecordStoreError::Storage)?;
    let changed = transaction
        .execute(
            "UPDATE run_effect_record_heads SET entry_count = ?1, head_sha256 = ?2
             WHERE run_id = ?3 AND entry_count = ?4 AND head_sha256 = ?5 AND closed = 0",
            params![
                sql_integer(sequence)?,
                &head_sha256,
                run_id,
                sql_integer(stored.entry_count)?,
                &stored.head_sha256
            ],
        )
        .map_err(|_| RunEffectRecordStoreError::Storage)?;
    // The head was read under the same lock; any other head means the store
    // changed underneath its only writer.
    if changed != 1 {
        return Err(RunEffectRecordStoreError::Integrity);
    }
    transaction
        .commit()
        .map_err(|_| RunEffectRecordStoreError::Storage)?;
    Ok((sequence, head_sha256))
}

fn set_head(
    store: &mut OperationalStore,
    stored: &StoredRunEffects,
    change: Change,
) -> Result<(), RunEffectRecordStoreError> {
    let statement = match change {
        Change::ClearComplete => {
            "UPDATE run_effect_record_heads SET complete = 0
             WHERE run_id = ?1 AND entry_count = ?2 AND head_sha256 = ?3
               AND complete = 1 AND closed = 0"
        }
        Change::Close => {
            "UPDATE run_effect_record_heads SET closed = 1
             WHERE run_id = ?1 AND entry_count = ?2 AND head_sha256 = ?3 AND closed = 0"
        }
    };
    let changed = store
        .connection
        .execute(
            statement,
            params![
                &stored.run_id,
                sql_integer(stored.entry_count)?,
                &stored.head_sha256
            ],
        )
        .map_err(|_| RunEffectRecordStoreError::Storage)?;
    if changed != 1 {
        return Err(RunEffectRecordStoreError::Integrity);
    }
    Ok(())
}

fn session(
    store: &OperationalStore,
    session_id: &str,
) -> Result<Vec<StoredRunEffects>, RunEffectRecordStoreError> {
    let limit = sql_integer(MAX_SESSION_RUNS as u64 + 1)?;
    let runs = store
        .connection
        .prepare(
            "SELECT roots.run_id, heads.entry_count FROM run_effect_record_roots AS roots
             JOIN run_effect_record_heads AS heads ON heads.run_id = roots.run_id
             WHERE roots.session_id = ?1 ORDER BY roots.session_position LIMIT ?2",
        )
        .and_then(|mut statement| {
            statement
                .query_map(params![session_id, limit], |row| {
                    Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
                })?
                .collect::<Result<Vec<_>, _>>()
        })
        .map_err(|_| RunEffectRecordStoreError::Storage)?;
    if runs.len() > MAX_SESSION_RUNS {
        return Err(RunEffectRecordStoreError::Integrity);
    }
    // The heads bound the work before any record is loaded.
    let mut positions = 0_usize;
    for (_, count) in &runs {
        positions = positions
            .checked_add(usize::try_from(*count).map_err(|_| RunEffectRecordStoreError::Integrity)?)
            .ok_or(RunEffectRecordStoreError::Integrity)?;
    }
    if positions > MAX_SESSION_POSITIONS {
        return Err(RunEffectRecordStoreError::Full);
    }
    let mut stored = Vec::with_capacity(runs.len());
    for (index, (run_id, _)) in runs.iter().enumerate() {
        let loaded = load(store, run_id)?;
        if loaded.stored.session_id != session_id
            || Some(loaded.stored.session_position) != u64::try_from(index + 1).ok()
        {
            return Err(RunEffectRecordStoreError::Integrity);
        }
        stored.push(loaded.stored);
    }
    Ok(stored)
}

fn load(store: &OperationalStore, run_id: &str) -> Result<Loaded, RunEffectRecordStoreError> {
    let (session_id, task_id, owner_id, session_position) = store
        .connection
        .query_row(
            "SELECT session_id, task_id, owner_id, session_position FROM run_effect_record_roots
             WHERE run_id = ?1",
            params![run_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, i64>(3)?,
                ))
            },
        )
        .optional()
        .map_err(|_| RunEffectRecordStoreError::Storage)?
        .ok_or(RunEffectRecordStoreError::NotFound)?;
    let (entry_count, head_sha256, complete, closed) = store
        .connection
        .query_row(
            "SELECT entry_count, head_sha256, complete, closed FROM run_effect_record_heads
             WHERE run_id = ?1",
            params![run_id],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, i64>(3)?,
                ))
            },
        )
        .optional()
        .map_err(|_| RunEffectRecordStoreError::Storage)?
        .ok_or(RunEffectRecordStoreError::Integrity)?;
    let limit = sql_integer(MAX_RUN_EFFECT_POSITIONS as u64 + 1)?;
    let rows = store
        .connection
        .prepare(
            "SELECT sequence, entry_sha256, record_json FROM run_effect_records
             WHERE run_id = ?1 ORDER BY sequence LIMIT ?2",
        )
        .and_then(|mut statement| {
            statement
                .query_map(params![run_id, limit], |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, Vec<u8>>(2)?,
                    ))
                })?
                .collect::<Result<Vec<_>, _>>()
        })
        .map_err(|_| RunEffectRecordStoreError::Storage)?;
    if rows.len() > MAX_RUN_EFFECT_POSITIONS
        || usize::try_from(entry_count).ok() != Some(rows.len())
        || !matches!(complete, 0 | 1)
        || !matches!(closed, 0 | 1)
        || !identifier(&session_id)
        || !identifier(&task_id)
        || !identifier(&owner_id)
        || !(1..=MAX_SESSION_RUNS as i64).contains(&session_position)
    {
        return Err(RunEffectRecordStoreError::Integrity);
    }
    let mut entries = Vec::with_capacity(rows.len());
    let mut previous = GENESIS_SHA256.to_owned();
    for (index, (sequence, entry_sha256, bytes)) in rows.into_iter().enumerate() {
        let position =
            u64::try_from(index + 1).map_err(|_| RunEffectRecordStoreError::Integrity)?;
        if bytes.len() > MAX_RUN_EFFECT_RECORD_BYTES
            || u64::try_from(sequence).ok() != Some(position)
        {
            return Err(RunEffectRecordStoreError::Integrity);
        }
        let entry: RunEffectEntry =
            serde_json::from_slice(&bytes).map_err(|_| RunEffectRecordStoreError::Integrity)?;
        if entry.validate().is_err()
            || !encode(&entry).is_ok_and(|encoded| encoded == bytes)
            || run_effect_entry_sha256(run_id, position, &previous, &entry).as_deref()
                != Some(entry_sha256.as_str())
        {
            return Err(RunEffectRecordStoreError::Integrity);
        }
        previous = entry_sha256;
        entries.push(entry);
    }
    if previous != head_sha256 {
        return Err(RunEffectRecordStoreError::Integrity);
    }
    Ok(Loaded {
        owner_id,
        stored: StoredRunEffects {
            session_id,
            task_id,
            run_id: run_id.to_owned(),
            session_position: u64::try_from(session_position)
                .map_err(|_| RunEffectRecordStoreError::Integrity)?,
            entries,
            entry_count: u64::try_from(entry_count)
                .map_err(|_| RunEffectRecordStoreError::Integrity)?,
            head_sha256,
            complete: complete == 1,
            closed: closed == 1,
        },
    })
}

fn encode(entry: &RunEffectEntry) -> Result<Vec<u8>, RunEffectRecordStoreError> {
    serde_json::to_vec(entry)
        .ok()
        .filter(|bytes| bytes.len() <= MAX_RUN_EFFECT_RECORD_BYTES)
        .ok_or(RunEffectRecordStoreError::InvalidInput)
}

fn sql_integer(value: u64) -> Result<i64, RunEffectRecordStoreError> {
    i64::try_from(value).map_err(|_| RunEffectRecordStoreError::Integrity)
}

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[cfg(test)]
#[path = "run_effect_record_store_tests.rs"]
mod tests;
