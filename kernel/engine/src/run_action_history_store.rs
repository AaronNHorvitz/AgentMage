//! Durable run action histories in the operational store (CAP-42, Decision 0129).
//!
//! The store owner persists each run's action history chains (Decision 0127)
//! as an immutable root, one record per position and one head. Every append
//! replays the stored chain, appends through [`ActionHistory`], and inserts the
//! record and advances the head in one immediate transaction whose head update
//! names the head it read, so a committed record never exists without the head
//! that ends it. Every read and every store open replays every chain, which
//! must reproduce exactly, be stored in its canonical encoding and end at the
//! retained head. A head's complete flag can only be cleared and its closed
//! flag only set; a closed chain takes no further record, and only then may
//! retention turn its expired entries into places.
//!
//! As for job ledgers (Decision 0118), the store authenticates writers by its
//! key, and within the process only a chain's recorded owner may append to,
//! mark or close it. A history grants no authority and performs no effect.

use std::sync::{Arc, Mutex};

use rusqlite::{OptionalExtension as _, TransactionBehavior, params};

use crate::action_history::{
    ActionHistory, ActionHistoryError, ActionHistoryHead, ActionHistoryRecord, ActionRecordDraft,
};
use crate::operational_store::OperationalStore;

/// Most positions one stored chain holds.
pub const MAX_RUN_ACTION_HISTORY_POSITIONS: usize = 512;
/// Largest canonical encoding of one stored record.
pub const MAX_RUN_ACTION_RECORD_BYTES: usize = 8_192;
const GENESIS_SHA256: &str = "0000000000000000000000000000000000000000000000000000000000000000";
const MAX_IDENTIFIER_BYTES: usize = 128;

/// Which owner component's chain of a run.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum RunActionChainName {
    /// Tool calls, file writes and commands, kept by the tool boundary.
    Effects,
    /// Job control requests, kept by the live runtime service.
    JobControl,
    /// Model route selections, kept by the runtime factory.
    Routes,
}

impl RunActionChainName {
    /// Every chain, in stable order.
    pub const ALL: [Self; 3] = [Self::Effects, Self::JobControl, Self::Routes];

    /// The stored chain name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Effects => "effects",
            Self::JobControl => "job-control",
            Self::Routes => "routes",
        }
    }

    fn parse(value: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|chain| chain.as_str() == value)
    }
}

/// Content-free run action history store failure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RunActionHistoryStoreError {
    /// The run, chain or owner identity is malformed; nothing was read or written.
    InvalidInput,
    /// No chain exists for the run.
    NotFound,
    /// The chain already exists; nothing was written.
    Exists,
    /// The caller is not the chain's recorded owner; nothing was written.
    NotOwner,
    /// The chain is closed and takes no further change; nothing was written.
    Closed,
    /// Retention applies only to a closed chain; nothing was written.
    NotClosed,
    /// The chain holds its most positions; nothing was written.
    Full,
    /// The history refused the entry; nothing was written.
    History(ActionHistoryError),
    /// Retained rows do not form the chain's exact history. The store is poisoned.
    Integrity,
    /// The store could not read or commit; nothing was written.
    Storage,
    /// The store is poisoned or its lock is unavailable.
    Unavailable,
}

/// One stored chain, replayed to its head.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StoredRunActionHistory {
    /// Every position in order.
    pub records: Vec<ActionHistoryRecord>,
    /// The stored head the positions end at.
    pub head: ActionHistoryHead,
    /// False once the owner knew it missed an entry, or once a run was
    /// resumed after the host that kept the chain ended.
    pub complete: bool,
    /// Whether the run ended and the chain takes no further record.
    pub closed: bool,
}

/// Shared handle to the durable run action histories of one operational store.
///
/// It takes the store's lock for each operation, like the job ledgers.
#[derive(Clone)]
pub struct DurableRunActionHistories {
    store: Arc<Mutex<OperationalStore>>,
}

impl std::fmt::Debug for DurableRunActionHistories {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("DurableRunActionHistories")
    }
}

impl DurableRunActionHistories {
    pub(crate) const fn new(store: Arc<Mutex<OperationalStore>>) -> Self {
        Self { store }
    }

    /// Creates one empty, complete, open chain for a run under its owner.
    pub fn create(
        &self,
        run_id: &str,
        chain: RunActionChainName,
        owner_id: &str,
    ) -> Result<(), RunActionHistoryStoreError> {
        validate_identity(run_id, owner_id)?;
        self.with_store(|store| create(store, run_id, chain, owner_id))
    }

    /// Attaches the owner to an existing open chain, marking it incomplete
    /// first when the owner may have missed an entry.
    pub fn attach(
        &self,
        run_id: &str,
        chain: RunActionChainName,
        owner_id: &str,
        mark_incomplete: bool,
    ) -> Result<StoredRunActionHistory, RunActionHistoryStoreError> {
        validate_identity(run_id, owner_id)?;
        self.with_store(|store| {
            let loaded = owned_open(store, run_id, chain, owner_id)?;
            if mark_incomplete && loaded.stored.complete {
                clear_complete(store, run_id, chain, &loaded.stored)?;
            }
            Ok(load(store, run_id, chain)?.stored)
        })
    }

    /// Redacts and appends one entry to the owner's open chain.
    pub fn append(
        &self,
        run_id: &str,
        chain: RunActionChainName,
        owner_id: &str,
        draft: &ActionRecordDraft,
    ) -> Result<ActionHistoryHead, RunActionHistoryStoreError> {
        validate_identity(run_id, owner_id)?;
        self.with_store(|store| append(store, run_id, chain, owner_id, draft))
    }

    /// Records that the owner missed an entry of its open chain.
    pub fn mark_incomplete(
        &self,
        run_id: &str,
        chain: RunActionChainName,
        owner_id: &str,
    ) -> Result<(), RunActionHistoryStoreError> {
        validate_identity(run_id, owner_id)?;
        self.with_store(|store| {
            let loaded = owned_open(store, run_id, chain, owner_id)?;
            if loaded.stored.complete {
                clear_complete(store, run_id, chain, &loaded.stored)?;
            }
            Ok(())
        })
    }

    /// Closes the owner's chain because its run ended. Closing a closed chain
    /// writes nothing.
    pub fn close(
        &self,
        run_id: &str,
        chain: RunActionChainName,
        owner_id: &str,
    ) -> Result<StoredRunActionHistory, RunActionHistoryStoreError> {
        validate_identity(run_id, owner_id)?;
        self.with_store(|store| close(store, run_id, chain, owner_id))
    }

    /// One chain of a run, replayed from the store.
    pub fn history(
        &self,
        run_id: &str,
        chain: RunActionChainName,
    ) -> Result<StoredRunActionHistory, RunActionHistoryStoreError> {
        validate_run(run_id)?;
        self.with_store(|store| load(store, run_id, chain).map(|loaded| loaded.stored))
    }

    /// Keeps only the place of every entry of a closed chain whose retention
    /// deadline passed. Returns how many entries expired now.
    pub fn apply_retention(
        &self,
        run_id: &str,
        chain: RunActionChainName,
        now_epoch_ms: u64,
    ) -> Result<u64, RunActionHistoryStoreError> {
        validate_run(run_id)?;
        self.with_store(|store| apply_retention(store, run_id, chain, now_epoch_ms))
    }

    fn with_store<T>(
        &self,
        operation: impl FnOnce(&mut OperationalStore) -> Result<T, RunActionHistoryStoreError>,
    ) -> Result<T, RunActionHistoryStoreError> {
        let mut store = self
            .store
            .lock()
            .map_err(|_| RunActionHistoryStoreError::Unavailable)?;
        if store.poisoned {
            return Err(RunActionHistoryStoreError::Unavailable);
        }
        let result = operation(&mut store);
        if matches!(result, Err(RunActionHistoryStoreError::Integrity)) {
            store.poisoned = true;
        }
        result
    }
}

/// Replays every retained chain; any chain that does not reproduce exactly
/// fails the whole store.
pub(crate) fn verify_all(store: &OperationalStore) -> Result<(), RunActionHistoryStoreError> {
    let roots = store
        .connection
        .prepare("SELECT run_id, chain FROM run_action_history_roots ORDER BY run_id, chain")
        .and_then(|mut statement| {
            statement
                .query_map([], |row| {
                    Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
                })?
                .collect::<Result<Vec<_>, _>>()
        })
        .map_err(|_| RunActionHistoryStoreError::Storage)?;
    let heads: i64 = store
        .connection
        .query_row("SELECT COUNT(*) FROM run_action_history_heads", [], |row| {
            row.get(0)
        })
        .map_err(|_| RunActionHistoryStoreError::Storage)?;
    if usize::try_from(heads).ok() != Some(roots.len()) {
        return Err(RunActionHistoryStoreError::Integrity);
    }
    // A record outside every chain has no root, which the store's foreign key
    // check refuses at open; each chain's own records are replayed here.
    for (run_id, chain) in &roots {
        let chain =
            RunActionChainName::parse(chain).ok_or(RunActionHistoryStoreError::Integrity)?;
        load(store, run_id, chain).map_err(|error| match error {
            RunActionHistoryStoreError::NotFound => RunActionHistoryStoreError::Integrity,
            other => other,
        })?;
    }
    Ok(())
}

struct Loaded {
    owner_id: String,
    history: ActionHistory,
    stored: StoredRunActionHistory,
}

fn validate_run(run_id: &str) -> Result<(), RunActionHistoryStoreError> {
    if run_id.is_empty() || run_id.len() > MAX_IDENTIFIER_BYTES || run_id.contains('\0') {
        Err(RunActionHistoryStoreError::InvalidInput)
    } else {
        Ok(())
    }
}

fn validate_identity(run_id: &str, owner_id: &str) -> Result<(), RunActionHistoryStoreError> {
    validate_run(run_id)?;
    if owner_id.is_empty() || owner_id.len() > MAX_IDENTIFIER_BYTES || owner_id.contains('\0') {
        return Err(RunActionHistoryStoreError::InvalidInput);
    }
    Ok(())
}

fn create(
    store: &mut OperationalStore,
    run_id: &str,
    chain: RunActionChainName,
    owner_id: &str,
) -> Result<(), RunActionHistoryStoreError> {
    let transaction = store
        .connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|_| RunActionHistoryStoreError::Storage)?;
    let existing = transaction
        .query_row(
            "SELECT 1 FROM run_action_history_roots WHERE run_id = ?1 AND chain = ?2",
            params![run_id, chain.as_str()],
            |row| row.get::<_, i64>(0),
        )
        .optional()
        .map_err(|_| RunActionHistoryStoreError::Storage)?;
    if existing.is_some() {
        return Err(RunActionHistoryStoreError::Exists);
    }
    transaction
        .execute(
            "INSERT INTO run_action_history_roots(run_id, chain, owner_id) VALUES (?1, ?2, ?3)",
            params![run_id, chain.as_str(), owner_id],
        )
        .map_err(|_| RunActionHistoryStoreError::Storage)?;
    transaction
        .execute(
            "INSERT INTO run_action_history_heads(run_id, chain, entry_count, head_sha256, complete, closed)
             VALUES (?1, ?2, 0, ?3, 1, 0)",
            params![run_id, chain.as_str(), GENESIS_SHA256],
        )
        .map_err(|_| RunActionHistoryStoreError::Storage)?;
    transaction
        .commit()
        .map_err(|_| RunActionHistoryStoreError::Storage)
}

/// Loads the owner's chain, refusing another owner and a closed chain.
fn owned_open(
    store: &OperationalStore,
    run_id: &str,
    chain: RunActionChainName,
    owner_id: &str,
) -> Result<Loaded, RunActionHistoryStoreError> {
    let loaded = load(store, run_id, chain)?;
    if loaded.owner_id != owner_id {
        return Err(RunActionHistoryStoreError::NotOwner);
    }
    if loaded.stored.closed {
        return Err(RunActionHistoryStoreError::Closed);
    }
    Ok(loaded)
}

fn append(
    store: &mut OperationalStore,
    run_id: &str,
    chain: RunActionChainName,
    owner_id: &str,
    draft: &ActionRecordDraft,
) -> Result<ActionHistoryHead, RunActionHistoryStoreError> {
    let Loaded {
        mut history,
        stored,
        ..
    } = owned_open(store, run_id, chain, owner_id)?;
    if stored.records.len() >= MAX_RUN_ACTION_HISTORY_POSITIONS {
        return Err(RunActionHistoryStoreError::Full);
    }
    let entry = history
        .append(draft)
        .map_err(RunActionHistoryStoreError::History)?;
    let after = history.head();
    commit_append(
        store,
        run_id,
        chain,
        &stored.head,
        &after,
        &ActionHistoryRecord::Kept(entry),
    )?;
    Ok(after)
}

/// Inserts one record and advances the head from `before` to `after` in one
/// immediate transaction. A head other than `before` commits nothing.
fn commit_append(
    store: &mut OperationalStore,
    run_id: &str,
    chain: RunActionChainName,
    before: &ActionHistoryHead,
    after: &ActionHistoryHead,
    record: &ActionHistoryRecord,
) -> Result<(), RunActionHistoryStoreError> {
    let bytes = encode(record)?;
    let transaction = store
        .connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|_| RunActionHistoryStoreError::Storage)?;
    transaction
        .execute(
            "INSERT INTO run_action_history_records(run_id, chain, sequence, entry_sha256, expired, record_json)
             VALUES (?1, ?2, ?3, ?4, 0, ?5)",
            params![
                run_id,
                chain.as_str(),
                sql_integer(after.count)?,
                &after.head_sha256,
                bytes
            ],
        )
        .map_err(|_| RunActionHistoryStoreError::Storage)?;
    let changed = transaction
        .execute(
            "UPDATE run_action_history_heads SET entry_count = ?1, head_sha256 = ?2
             WHERE run_id = ?3 AND chain = ?4 AND entry_count = ?5 AND head_sha256 = ?6
               AND closed = 0",
            params![
                sql_integer(after.count)?,
                &after.head_sha256,
                run_id,
                chain.as_str(),
                sql_integer(before.count)?,
                &before.head_sha256
            ],
        )
        .map_err(|_| RunActionHistoryStoreError::Storage)?;
    // The head was read under the same lock; any other head means the store
    // changed underneath its only writer.
    if changed != 1 {
        return Err(RunActionHistoryStoreError::Integrity);
    }
    transaction
        .commit()
        .map_err(|_| RunActionHistoryStoreError::Storage)
}

fn clear_complete(
    store: &mut OperationalStore,
    run_id: &str,
    chain: RunActionChainName,
    stored: &StoredRunActionHistory,
) -> Result<(), RunActionHistoryStoreError> {
    let changed = store
        .connection
        .execute(
            "UPDATE run_action_history_heads SET complete = 0
             WHERE run_id = ?1 AND chain = ?2 AND entry_count = ?3 AND head_sha256 = ?4
               AND complete = 1 AND closed = 0",
            params![
                run_id,
                chain.as_str(),
                sql_integer(stored.head.count)?,
                &stored.head.head_sha256
            ],
        )
        .map_err(|_| RunActionHistoryStoreError::Storage)?;
    if changed != 1 {
        return Err(RunActionHistoryStoreError::Integrity);
    }
    Ok(())
}

fn close(
    store: &mut OperationalStore,
    run_id: &str,
    chain: RunActionChainName,
    owner_id: &str,
) -> Result<StoredRunActionHistory, RunActionHistoryStoreError> {
    let loaded = load(store, run_id, chain)?;
    if loaded.owner_id != owner_id {
        return Err(RunActionHistoryStoreError::NotOwner);
    }
    if loaded.stored.closed {
        return Ok(loaded.stored);
    }
    let changed = store
        .connection
        .execute(
            "UPDATE run_action_history_heads SET closed = 1
             WHERE run_id = ?1 AND chain = ?2 AND entry_count = ?3 AND head_sha256 = ?4
               AND closed = 0",
            params![
                run_id,
                chain.as_str(),
                sql_integer(loaded.stored.head.count)?,
                &loaded.stored.head.head_sha256
            ],
        )
        .map_err(|_| RunActionHistoryStoreError::Storage)?;
    if changed != 1 {
        return Err(RunActionHistoryStoreError::Integrity);
    }
    Ok(load(store, run_id, chain)?.stored)
}

fn apply_retention(
    store: &mut OperationalStore,
    run_id: &str,
    chain: RunActionChainName,
    now_epoch_ms: u64,
) -> Result<u64, RunActionHistoryStoreError> {
    let Loaded {
        mut history,
        stored,
        ..
    } = load(store, run_id, chain)?;
    if !stored.closed {
        return Err(RunActionHistoryStoreError::NotClosed);
    }
    let expired = history.apply_retention(now_epoch_ms);
    if expired == 0 {
        return Ok(0);
    }
    let transaction = store
        .connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|_| RunActionHistoryStoreError::Storage)?;
    for (before, after) in stored.records.iter().zip(history.records()) {
        let (
            ActionHistoryRecord::Kept(_),
            ActionHistoryRecord::Expired {
                sequence,
                entry_sha256,
                ..
            },
        ) = (before, after)
        else {
            continue;
        };
        let changed = transaction
            .execute(
                "UPDATE run_action_history_records SET expired = 1, record_json = ?1
                 WHERE run_id = ?2 AND chain = ?3 AND sequence = ?4 AND entry_sha256 = ?5
                   AND expired = 0",
                params![
                    encode(after)?,
                    run_id,
                    chain.as_str(),
                    sql_integer(*sequence)?,
                    entry_sha256
                ],
            )
            .map_err(|_| RunActionHistoryStoreError::Storage)?;
        if changed != 1 {
            return Err(RunActionHistoryStoreError::Integrity);
        }
    }
    transaction
        .commit()
        .map_err(|_| RunActionHistoryStoreError::Storage)?;
    // The rewritten places must still form the chain that ends at its head.
    load(store, run_id, chain)?;
    Ok(expired)
}

fn load(
    store: &OperationalStore,
    run_id: &str,
    chain: RunActionChainName,
) -> Result<Loaded, RunActionHistoryStoreError> {
    let owner_id = store
        .connection
        .query_row(
            "SELECT owner_id FROM run_action_history_roots WHERE run_id = ?1 AND chain = ?2",
            params![run_id, chain.as_str()],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(|_| RunActionHistoryStoreError::Storage)?
        .ok_or(RunActionHistoryStoreError::NotFound)?;
    let (entry_count, head_sha256, complete, closed) = store
        .connection
        .query_row(
            "SELECT entry_count, head_sha256, complete, closed FROM run_action_history_heads
             WHERE run_id = ?1 AND chain = ?2",
            params![run_id, chain.as_str()],
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
        .map_err(|_| RunActionHistoryStoreError::Storage)?
        .ok_or(RunActionHistoryStoreError::Integrity)?;
    let limit = sql_integer(MAX_RUN_ACTION_HISTORY_POSITIONS as u64 + 1)?;
    let rows = store
        .connection
        .prepare(
            "SELECT sequence, entry_sha256, expired, record_json FROM run_action_history_records
             WHERE run_id = ?1 AND chain = ?2 ORDER BY sequence LIMIT ?3",
        )
        .and_then(|mut statement| {
            statement
                .query_map(params![run_id, chain.as_str(), limit], |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, i64>(2)?,
                        row.get::<_, Vec<u8>>(3)?,
                    ))
                })?
                .collect::<Result<Vec<_>, _>>()
        })
        .map_err(|_| RunActionHistoryStoreError::Storage)?;
    if rows.len() > MAX_RUN_ACTION_HISTORY_POSITIONS
        || usize::try_from(entry_count).ok() != Some(rows.len())
        || !matches!(complete, 0 | 1)
        || !matches!(closed, 0 | 1)
    {
        return Err(RunActionHistoryStoreError::Integrity);
    }
    let mut records = Vec::with_capacity(rows.len());
    for (index, (sequence, entry_sha256, expired, bytes)) in rows.into_iter().enumerate() {
        if bytes.len() > MAX_RUN_ACTION_RECORD_BYTES {
            return Err(RunActionHistoryStoreError::Integrity);
        }
        let record: ActionHistoryRecord =
            serde_json::from_slice(&bytes).map_err(|_| RunActionHistoryStoreError::Integrity)?;
        let (record_sequence, record_sha256, record_expired) = match &record {
            ActionHistoryRecord::Kept(entry) => (entry.sequence, entry.entry_sha256.as_str(), 0),
            ActionHistoryRecord::Expired {
                sequence,
                entry_sha256,
                ..
            } => (*sequence, entry_sha256.as_str(), 1),
        };
        let position = u64::try_from(index)
            .ok()
            .and_then(|value| value.checked_add(1))
            .ok_or(RunActionHistoryStoreError::Integrity)?;
        if u64::try_from(sequence).ok() != Some(position)
            || record_sequence != position
            || record_sha256 != entry_sha256
            || expired != record_expired
            || encode(&record)? != bytes
        {
            return Err(RunActionHistoryStoreError::Integrity);
        }
        records.push(record);
    }
    let head = ActionHistoryHead {
        count: u64::try_from(entry_count).map_err(|_| RunActionHistoryStoreError::Integrity)?,
        head_sha256,
    };
    let history = ActionHistory::replay(records.clone(), &head)
        .map_err(|_| RunActionHistoryStoreError::Integrity)?;
    Ok(Loaded {
        owner_id,
        history,
        stored: StoredRunActionHistory {
            records,
            head,
            complete: complete == 1,
            closed: closed == 1,
        },
    })
}

fn encode(record: &ActionHistoryRecord) -> Result<Vec<u8>, RunActionHistoryStoreError> {
    serde_json::to_vec(record)
        .ok()
        .filter(|bytes| bytes.len() <= MAX_RUN_ACTION_RECORD_BYTES)
        .ok_or(RunActionHistoryStoreError::Storage)
}

fn sql_integer(value: u64) -> Result<i64, RunActionHistoryStoreError> {
    i64::try_from(value).map_err(|_| RunActionHistoryStoreError::Integrity)
}

#[cfg(test)]
#[path = "run_action_history_store_tests.rs"]
mod tests;
