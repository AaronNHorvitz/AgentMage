//! Durable owner states in the operational store (Decision 0131).
//!
//! An owner that keeps one complete state, such as the catalog host's memory
//! catalog, commits each next state under the revision it read. The store
//! keeps the state bytes and their digest; it does not interpret them, and
//! the owner decodes and re-verifies the state's meaning. A first state has
//! revision one, each later state raises the revision by exactly one, and a
//! state is never deleted; store triggers enforce the same rules. Every store
//! open and every load checks each digest, so a changed state poisons the
//! store. Owners are named in a closed list, so a later owner adds a name and
//! not a migration. As for the other store owners, the store's key
//! authenticates writers. A state grants no authority and performs no effect.

use std::sync::{Arc, Mutex};

use rusqlite::{OptionalExtension as _, TransactionBehavior, params};
use sha2::{Digest, Sha256};

use crate::operational_store::OperationalStore;

/// Largest state one owner keeps.
pub const MAX_OWNER_STATE_BYTES: usize = 64 * 1024 * 1024;

/// The owners that keep a state, in a closed list.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum OwnerStateName {
    /// The catalog host's memory catalog.
    MemoryCatalog,
    /// The catalog host's extension catalog (Decision 0132).
    ExtensionCatalog,
}

impl OwnerStateName {
    /// Every owner, in stable order.
    pub const ALL: [Self; 2] = [Self::MemoryCatalog, Self::ExtensionCatalog];

    /// The stored owner identity.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::MemoryCatalog => "memory-catalog",
            Self::ExtensionCatalog => "extension-catalog",
        }
    }

    fn parse(value: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|owner| owner.as_str() == value)
    }
}

/// One owner's state as the store keeps it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StoredOwnerState {
    /// Revision of the state; zero before the first commit.
    pub revision: u64,
    /// The state bytes; empty before the first commit.
    pub state: Vec<u8>,
}

/// Content-free owner state store failure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OwnerStateStoreError {
    /// The state is empty; nothing was written.
    InvalidInput,
    /// The state exceeds its bound; nothing was written.
    ResourceLimit,
    /// The owner's state changed since the revision it read; nothing was
    /// written.
    Stale,
    /// A retained state does not match its digest or names no known owner.
    /// The store is poisoned.
    Integrity,
    /// The store could not read or commit; nothing was written.
    Storage,
    /// The store is poisoned or its lock is unavailable.
    Unavailable,
}

/// Shared handle to the owner states of one operational store. It takes the
/// store's lock for each operation, like the job ledgers.
#[derive(Clone)]
pub struct DurableOwnerStates {
    store: Arc<Mutex<OperationalStore>>,
}

impl std::fmt::Debug for DurableOwnerStates {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("DurableOwnerStates")
    }
}

impl DurableOwnerStates {
    pub(crate) const fn new(store: Arc<Mutex<OperationalStore>>) -> Self {
        Self { store }
    }

    /// One owner's state, checked against its digest; revision zero and no
    /// bytes before the owner's first commit.
    pub fn load(&self, owner: OwnerStateName) -> Result<StoredOwnerState, OwnerStateStoreError> {
        self.with_store(|store| load(&store.connection, owner))
    }

    /// Replaces the owner's state read at `read_revision` with `state`, and
    /// returns the revision the store then holds.
    pub fn commit(
        &self,
        owner: OwnerStateName,
        read_revision: u64,
        state: &[u8],
    ) -> Result<u64, OwnerStateStoreError> {
        if state.is_empty() {
            return Err(OwnerStateStoreError::InvalidInput);
        }
        if state.len() > MAX_OWNER_STATE_BYTES {
            return Err(OwnerStateStoreError::ResourceLimit);
        }
        self.with_store(|store| commit(store, owner, read_revision, state))
    }

    fn with_store<T>(
        &self,
        operation: impl FnOnce(&mut OperationalStore) -> Result<T, OwnerStateStoreError>,
    ) -> Result<T, OwnerStateStoreError> {
        let mut store = self
            .store
            .lock()
            .map_err(|_| OwnerStateStoreError::Unavailable)?;
        if store.poisoned {
            return Err(OwnerStateStoreError::Unavailable);
        }
        let result = operation(&mut store);
        if matches!(result, Err(OwnerStateStoreError::Integrity)) {
            store.poisoned = true;
        }
        result
    }
}

/// Checks every retained state at store open: a known owner and a matching
/// digest.
pub(crate) fn verify_all(store: &OperationalStore) -> Result<(), OwnerStateStoreError> {
    let owners = store
        .connection
        .prepare("SELECT owner_id FROM owner_states ORDER BY owner_id")
        .and_then(|mut statement| {
            statement
                .query_map([], |row| row.get::<_, String>(0))?
                .collect::<Result<Vec<_>, _>>()
        })
        .map_err(|_| OwnerStateStoreError::Storage)?;
    for owner in owners {
        let owner = OwnerStateName::parse(&owner).ok_or(OwnerStateStoreError::Integrity)?;
        load(&store.connection, owner)?;
    }
    Ok(())
}

fn sha256_hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    let mut output = String::with_capacity(64);
    for byte in Sha256::digest(bytes) {
        let _ = write!(output, "{byte:02x}");
    }
    output
}

fn load(
    connection: &rusqlite::Connection,
    owner: OwnerStateName,
) -> Result<StoredOwnerState, OwnerStateStoreError> {
    let row = connection
        .query_row(
            "SELECT revision, state, state_sha256 FROM owner_states WHERE owner_id = ?1",
            params![owner.as_str()],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, Vec<u8>>(1)?,
                    row.get::<_, String>(2)?,
                ))
            },
        )
        .optional()
        .map_err(|_| OwnerStateStoreError::Storage)?;
    let Some((revision, state, state_sha256)) = row else {
        return Ok(StoredOwnerState {
            revision: 0,
            state: Vec::new(),
        });
    };
    let revision = u64::try_from(revision).map_err(|_| OwnerStateStoreError::Integrity)?;
    if revision == 0
        || state.is_empty()
        || state.len() > MAX_OWNER_STATE_BYTES
        || sha256_hex(&state) != state_sha256
    {
        return Err(OwnerStateStoreError::Integrity);
    }
    Ok(StoredOwnerState { revision, state })
}

fn commit(
    store: &mut OperationalStore,
    owner: OwnerStateName,
    read_revision: u64,
    state: &[u8],
) -> Result<u64, OwnerStateStoreError> {
    let transaction = store
        .connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|_| OwnerStateStoreError::Storage)?;
    let current = load(&transaction, owner)?;
    if current.revision != read_revision {
        return Err(OwnerStateStoreError::Stale);
    }
    let revision = read_revision
        .checked_add(1)
        .ok_or(OwnerStateStoreError::ResourceLimit)?;
    let next = i64::try_from(revision).map_err(|_| OwnerStateStoreError::ResourceLimit)?;
    let digest = sha256_hex(state);
    let written = if read_revision == 0 {
        transaction
            .execute(
                "INSERT INTO owner_states(owner_id, revision, state, state_sha256)
                 VALUES (?1, ?2, ?3, ?4)",
                params![owner.as_str(), next, state, digest],
            )
            .map_err(|_| OwnerStateStoreError::Storage)?
    } else {
        transaction
            .execute(
                "UPDATE owner_states SET revision = ?1, state = ?2, state_sha256 = ?3
                 WHERE owner_id = ?4 AND revision = ?5",
                params![
                    next,
                    state,
                    digest,
                    owner.as_str(),
                    i64::try_from(read_revision).map_err(|_| OwnerStateStoreError::Integrity)?
                ],
            )
            .map_err(|_| OwnerStateStoreError::Storage)?
    };
    // The revision was read under the same lock; any other row means the
    // store changed underneath its only writer.
    if written != 1 {
        return Err(OwnerStateStoreError::Integrity);
    }
    transaction
        .commit()
        .map_err(|_| OwnerStateStoreError::Storage)?;
    Ok(revision)
}

#[cfg(test)]
#[path = "owner_state_store_tests.rs"]
mod tests;
