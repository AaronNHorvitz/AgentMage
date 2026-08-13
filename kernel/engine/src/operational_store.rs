//! SQLCipher-backed authority state and fail-closed restart recovery.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::fs::{self, OpenOptions};
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
use std::path::{Component, Path};
use std::time::Duration;

use agentmage_kernel_contracts::{
    AuthorityTransactionId, AuthorityTransactionRecord, AuthorityTransactionState, CapabilityGrant,
    GrantId, GrantNonce, Receipt, StrictLocalStorageObservation, from_json, to_canonical_json,
};
use rusqlite::{
    Connection, OpenFlags, OptionalExtension, Transaction, TransactionBehavior, params,
};
use sha2::{Digest, Sha256};
use zeroize::Zeroizing;

use crate::authority_transaction::{
    AuthorityTransactionCoordinator, AuthorityTransactionError, AuthorityTransactionRequest,
    EffectDriver,
};
use crate::grants::{
    DerivedOperationGrantRequest, GrantIssueError, GrantIssuer, SessionReadGrantRequest,
};
use crate::policy::PolicyEngine;
use crate::strict_local::{StrictLocalStorageDecision, evaluate_storage};
use crate::tooling::ToolRegistry;

const SCHEMA_VERSION: i64 = 2;
const ZERO_SHA256: &str = "0000000000000000000000000000000000000000000000000000000000000000";
const KEY_BYTES: usize = 32;
const MIGRATION_1_SCHEMA_SQL: &str = "CREATE TABLE schema_history (
    version INTEGER PRIMARY KEY,
    migration_sha256 TEXT NOT NULL CHECK(length(migration_sha256) = 64)
) STRICT;
CREATE TABLE store_metadata (
    singleton INTEGER PRIMARY KEY CHECK(singleton = 1),
    generation INTEGER NOT NULL CHECK(generation >= 0),
    state_sha256 TEXT NOT NULL CHECK(length(state_sha256) = 64)
) STRICT;
CREATE TABLE grant_identities (
    grant_id TEXT PRIMARY KEY,
    nonce TEXT NOT NULL UNIQUE
) STRICT;
CREATE TABLE grant_revisions (
    grant_id TEXT NOT NULL,
    revision INTEGER NOT NULL CHECK(revision > 0),
    record_sha256 TEXT NOT NULL CHECK(length(record_sha256) = 64),
    record_json BLOB NOT NULL,
    PRIMARY KEY(grant_id, revision),
    FOREIGN KEY(grant_id) REFERENCES grant_identities(grant_id)
) STRICT;
CREATE TABLE grant_heads (
    grant_id TEXT PRIMARY KEY,
    current_revision INTEGER NOT NULL CHECK(current_revision > 0),
    FOREIGN KEY(grant_id, current_revision)
        REFERENCES grant_revisions(grant_id, revision)
) STRICT;
CREATE TABLE transaction_identities (
    transaction_id TEXT PRIMARY KEY,
    attempt_id TEXT NOT NULL UNIQUE
) STRICT;
CREATE TABLE transaction_revisions (
    transaction_id TEXT NOT NULL,
    revision INTEGER NOT NULL CHECK(revision > 0),
    state TEXT NOT NULL,
    record_sha256 TEXT NOT NULL CHECK(length(record_sha256) = 64),
    record_json BLOB NOT NULL,
    PRIMARY KEY(transaction_id, revision),
    FOREIGN KEY(transaction_id)
        REFERENCES transaction_identities(transaction_id)
) STRICT;
CREATE TABLE transaction_heads (
    transaction_id TEXT PRIMARY KEY,
    current_revision INTEGER NOT NULL CHECK(current_revision > 0),
    FOREIGN KEY(transaction_id, current_revision)
        REFERENCES transaction_revisions(transaction_id, revision)
) STRICT;
CREATE TABLE receipts (
    sequence INTEGER PRIMARY KEY CHECK(sequence > 0),
    receipt_id TEXT NOT NULL UNIQUE,
    transaction_id TEXT NOT NULL UNIQUE,
    receipt_sha256 TEXT NOT NULL UNIQUE CHECK(length(receipt_sha256) = 64),
    previous_receipt_sha256 TEXT NOT NULL CHECK(length(previous_receipt_sha256) = 64),
    receipt_json BLOB NOT NULL,
    FOREIGN KEY(transaction_id)
        REFERENCES transaction_identities(transaction_id)
) STRICT;
CREATE TABLE checkpoints (
    generation INTEGER PRIMARY KEY CHECK(generation >= 0),
    state_sha256 TEXT NOT NULL CHECK(length(state_sha256) = 64)
) STRICT;";
const MIGRATION_2_SCHEMA_SQL: &str =
    include_str!("../migrations/operational-store/0002-domain-schema.sql");

/// Stable key-broker failure that reveals no key or provider detail.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OperationalStoreKeyError {
    /// The platform secret authority could not supply the fixed operational-store key.
    Unavailable,
}

/// Platform boundary that exposes a database key only during one bounded callback.
pub trait OperationalStoreKeyProvider {
    /// Invokes `operation` with the exact 256-bit key or fails without invoking it.
    fn with_key<T>(
        &mut self,
        operation: impl FnOnce(&[u8]) -> T,
    ) -> Result<T, OperationalStoreKeyError>;
}

/// Stable, content-free encrypted-store failure class.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OperationalStoreError {
    /// The selected root is remote, synchronized, linked, or otherwise ineligible.
    StorageRejected,
    /// The platform secret authority did not provide a key.
    KeyUnavailable,
    /// The supplied key did not have the required fixed shape.
    InvalidKey,
    /// The encrypted database could not be opened exclusively.
    OpenFailed,
    /// The linked SQLite implementation is not SQLCipher.
    CipherUnavailable,
    /// The schema version is unsupported or a migration failed.
    MigrationFailed,
    /// Canonical rows, constraints, hashes, or encrypted pages failed verification.
    IntegrityFailure,
    /// Another writer owns the canonical store.
    ConcurrentWriter,
    /// An atomic state publication failed.
    PersistenceFailure,
    /// A prior persistence ambiguity requires process restart and recovery.
    Poisoned,
}

impl OperationalStoreError {
    /// Returns the stable diagnostic code without path, SQL, or key material.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::StorageRejected => "operational_store.storage.rejected",
            Self::KeyUnavailable => "operational_store.key.unavailable",
            Self::InvalidKey => "operational_store.key.invalid",
            Self::OpenFailed => "operational_store.open.failed",
            Self::CipherUnavailable => "operational_store.cipher.unavailable",
            Self::MigrationFailed => "operational_store.migration.failed",
            Self::IntegrityFailure => "operational_store.integrity.failed",
            Self::ConcurrentWriter => "operational_store.writer.concurrent",
            Self::PersistenceFailure => "operational_store.persistence.failed",
            Self::Poisoned => "operational_store.poisoned",
        }
    }
}

impl fmt::Display for OperationalStoreError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.code())
    }
}

impl std::error::Error for OperationalStoreError {}

/// One exclusive SQLCipher connection to canonical authority state.
pub struct OperationalStore {
    connection: Connection,
    generation: u64,
    poisoned: bool,
}

impl fmt::Debug for OperationalStore {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("OperationalStore")
            .field("generation", &self.generation)
            .field("poisoned", &self.poisoned)
            .finish_non_exhaustive()
    }
}

impl OperationalStore {
    /// Opens one encrypted store only after strict-local admission and key retrieval.
    pub fn open<P: OperationalStoreKeyProvider>(
        path: &Path,
        observation: &StrictLocalStorageObservation,
        provider: &mut P,
    ) -> Result<Self, OperationalStoreError> {
        if evaluate_storage(observation) != StrictLocalStorageDecision::Eligible {
            return Err(OperationalStoreError::StorageRejected);
        }
        provider
            .with_key(|key| {
                prepare_store_file(path)?;
                open_keyed(path, key)
            })
            .map_err(|_| OperationalStoreError::KeyUnavailable)?
    }

    /// Returns the last atomically published state generation.
    #[must_use]
    pub const fn generation(&self) -> u64 {
        self.generation
    }

    /// Produces a separately keyed encrypted online backup and verifies its pages.
    pub fn backup<P: OperationalStoreKeyProvider>(
        &self,
        destination: &Path,
        observation: &StrictLocalStorageObservation,
        provider: &mut P,
    ) -> Result<(), OperationalStoreError> {
        if self.poisoned {
            return Err(OperationalStoreError::Poisoned);
        }
        if evaluate_storage(observation) != StrictLocalStorageDecision::Eligible {
            return Err(OperationalStoreError::StorageRejected);
        }
        prepare_new_store_file(destination)?;
        let result = provider
            .with_key(|key| {
                let mut destination_connection = open_connection(destination, key)?;
                {
                    let backup = rusqlite::backup::Backup::new(
                        &self.connection,
                        &mut destination_connection,
                    )
                    .map_err(|_| OperationalStoreError::PersistenceFailure)?;
                    backup
                        .run_to_completion(64, Duration::from_millis(1), None)
                        .map_err(|_| OperationalStoreError::PersistenceFailure)?;
                }
                verify_integrity(&destination_connection)
            })
            .map_err(|_| OperationalStoreError::KeyUnavailable)?;
        if result.is_err() {
            let _ = fs::remove_file(destination);
        }
        result
    }

    fn load_authority(
        &mut self,
    ) -> Result<(GrantIssuer, AuthorityTransactionCoordinator), OperationalStoreError> {
        verify_integrity(&self.connection)?;
        let generation: i64 = self
            .connection
            .query_row(
                "SELECT generation FROM store_metadata WHERE singleton = 1",
                [],
                |row| row.get(0),
            )
            .map_err(|_| OperationalStoreError::IntegrityFailure)?;
        self.generation =
            u64::try_from(generation).map_err(|_| OperationalStoreError::IntegrityFailure)?;

        let grant_histories = load_grants(&self.connection)?;
        let grant_hashes = load_grant_hashes(&self.connection)?;
        let nonces = load_nonces(&self.connection)?;
        let issuer = GrantIssuer::from_durable_parts(grant_histories, grant_hashes, nonces)
            .map_err(|_| OperationalStoreError::IntegrityFailure)?;
        let transaction_histories = load_transactions(&self.connection)?;
        let receipts = load_receipts(&self.connection)?;
        let coordinator =
            AuthorityTransactionCoordinator::from_durable_parts(transaction_histories, receipts)
                .map_err(|_| OperationalStoreError::IntegrityFailure)?;
        let expected = authority_state_sha256(&issuer, &coordinator)?;
        let retained: String = self
            .connection
            .query_row(
                "SELECT state_sha256 FROM store_metadata WHERE singleton = 1",
                [],
                |row| row.get(0),
            )
            .map_err(|_| OperationalStoreError::IntegrityFailure)?;
        if retained == ZERO_SHA256 && self.generation == 0 {
            let transaction = self
                .connection
                .unchecked_transaction()
                .map_err(|_| OperationalStoreError::IntegrityFailure)?;
            transaction
                .execute(
                    "UPDATE store_metadata SET state_sha256 = ?1 WHERE singleton = 1 AND generation = 0 AND state_sha256 = ?2",
                    params![&expected, ZERO_SHA256],
                )
                .map_err(|_| OperationalStoreError::IntegrityFailure)?;
            transaction
                .execute(
                    "UPDATE checkpoints SET state_sha256 = ?1 WHERE generation = 0 AND state_sha256 = ?2",
                    params![&expected, ZERO_SHA256],
                )
                .map_err(|_| OperationalStoreError::IntegrityFailure)?;
            transaction
                .commit()
                .map_err(|_| OperationalStoreError::IntegrityFailure)?;
        } else if retained != expected {
            return Err(OperationalStoreError::IntegrityFailure);
        }
        verify_checkpoint_head(&self.connection, self.generation, &expected)?;
        Ok((issuer, coordinator))
    }

    pub(crate) fn persist_authority(
        &mut self,
        issuer: &GrantIssuer,
        coordinator: &AuthorityTransactionCoordinator,
    ) -> Result<(), OperationalStoreError> {
        if self.poisoned {
            return Err(OperationalStoreError::Poisoned);
        }
        let next_generation = self
            .generation
            .checked_add(1)
            .ok_or(OperationalStoreError::PersistenceFailure)?;
        let state_sha256 = authority_state_sha256(issuer, coordinator)?;
        let result = persist_snapshot(
            &mut self.connection,
            self.generation,
            next_generation,
            &state_sha256,
            issuer,
            coordinator,
        );
        match result {
            Ok(()) => {
                self.generation = next_generation;
                Ok(())
            }
            Err(error) => {
                self.poisoned = true;
                Err(error)
            }
        }
    }
}

/// Composed durable grant issuer and sole public effect-launch boundary.
pub struct DurableAuthorityRuntime {
    store: OperationalStore,
    issuer: GrantIssuer,
    coordinator: AuthorityTransactionCoordinator,
    poisoned: bool,
}

impl fmt::Debug for DurableAuthorityRuntime {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("DurableAuthorityRuntime")
            .field("generation", &self.store.generation())
            .field("poisoned", &self.poisoned)
            .finish_non_exhaustive()
    }
}

/// Stable failure from the composed durable authority boundary.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DurableAuthorityError {
    /// Encrypted state could not be opened, read, or written safely.
    Store(OperationalStoreError),
    /// Grant issuance or derivation was denied by its closed contract.
    Grant(GrantIssueError),
    /// The authority transaction failed before a safe terminal result.
    Transaction(AuthorityTransactionError),
    /// The process must reopen canonical state before any further effect.
    Poisoned,
}

impl DurableAuthorityRuntime {
    /// Opens canonical state and resolves every interrupted transaction before use.
    pub fn open<P: OperationalStoreKeyProvider>(
        path: &Path,
        observation: &StrictLocalStorageObservation,
        provider: &mut P,
        recovery_epoch_ms: u64,
    ) -> Result<Self, DurableAuthorityError> {
        let mut store = OperationalStore::open(path, observation, provider)
            .map_err(DurableAuthorityError::Store)?;
        let (issuer, coordinator) = store
            .load_authority()
            .map_err(DurableAuthorityError::Store)?;
        let mut runtime = Self {
            store,
            issuer,
            coordinator,
            poisoned: false,
        };
        runtime.recover_interrupted(recovery_epoch_ms)?;
        Ok(runtime)
    }

    /// Returns one current canonical grant after restart validation.
    #[must_use]
    pub fn current_grant(&self, grant_id: &GrantId) -> Option<&CapabilityGrant> {
        self.issuer.current(grant_id)
    }

    /// Returns one current canonical transaction revision.
    #[must_use]
    pub fn current_transaction(
        &self,
        transaction_id: &AuthorityTransactionId,
    ) -> Option<&AuthorityTransactionRecord> {
        self.coordinator.current(transaction_id)
    }

    /// Returns canonical terminal receipts in hash-chain sequence.
    #[must_use]
    pub fn receipts(&self) -> &[Receipt] {
        self.coordinator.receipts()
    }

    /// Issues one parent grant and publishes it atomically before returning it.
    pub fn issue_session_read(
        &mut self,
        request: SessionReadGrantRequest,
    ) -> Result<CapabilityGrant, DurableAuthorityError> {
        self.ensure_usable()?;
        let mut candidate = self.issuer.clone();
        let grant = candidate
            .issue_session_read(request)
            .map_err(DurableAuthorityError::Grant)?;
        self.store
            .persist_authority(&candidate, &self.coordinator)
            .map_err(|error| self.poison(error))?;
        self.issuer = candidate;
        Ok(grant)
    }

    /// Derives one exact operation grant and publishes parent and child atomically.
    pub fn derive_operation(
        &mut self,
        parent_grant_id: &GrantId,
        request: DerivedOperationGrantRequest,
    ) -> Result<CapabilityGrant, DurableAuthorityError> {
        self.ensure_usable()?;
        let mut candidate = self.issuer.clone();
        let grant = candidate
            .derive_operation(parent_grant_id, request)
            .map_err(DurableAuthorityError::Grant)?;
        self.store
            .persist_authority(&candidate, &self.coordinator)
            .map_err(|error| self.poison(error))?;
        self.issuer = candidate;
        Ok(grant)
    }

    /// Executes one effect only after every pre-launch state is durably committed.
    pub fn execute_effect<D: EffectDriver>(
        &mut self,
        registry: &ToolRegistry,
        policy: &PolicyEngine,
        request: AuthorityTransactionRequest,
        driver: &mut D,
    ) -> Result<Receipt, DurableAuthorityError> {
        self.ensure_usable()?;
        let store = &mut self.store;
        let result = self.coordinator.execute_with_checkpoint(
            registry,
            &mut self.issuer,
            policy,
            &request,
            driver,
            &mut |issuer, coordinator| {
                store
                    .persist_authority(issuer, coordinator)
                    .map_err(|_| AuthorityTransactionError::PersistenceFailure)
            },
        );
        match result {
            Ok(receipt) => Ok(receipt),
            Err(error) => {
                if error == AuthorityTransactionError::PersistenceFailure {
                    self.poisoned = true;
                }
                Err(DurableAuthorityError::Transaction(error))
            }
        }
    }

    /// Creates and verifies a separately keyed encrypted backup.
    pub fn backup<P: OperationalStoreKeyProvider>(
        &self,
        destination: &Path,
        observation: &StrictLocalStorageObservation,
        provider: &mut P,
    ) -> Result<(), DurableAuthorityError> {
        self.ensure_usable()?;
        self.store
            .backup(destination, observation, provider)
            .map_err(DurableAuthorityError::Store)
    }

    fn recover_interrupted(
        &mut self,
        occurred_at_epoch_ms: u64,
    ) -> Result<(), DurableAuthorityError> {
        let transaction_ids = self.coordinator.nonterminal_ids();
        for transaction_id in transaction_ids {
            let store = &mut self.store;
            self.coordinator
                .recover_with_checkpoint(
                    &mut self.issuer,
                    &transaction_id,
                    occurred_at_epoch_ms,
                    &mut |issuer, coordinator| {
                        store
                            .persist_authority(issuer, coordinator)
                            .map_err(|_| AuthorityTransactionError::PersistenceFailure)
                    },
                )
                .map_err(DurableAuthorityError::Transaction)?;
        }
        Ok(())
    }

    fn ensure_usable(&self) -> Result<(), DurableAuthorityError> {
        if self.poisoned || self.store.poisoned {
            Err(DurableAuthorityError::Poisoned)
        } else {
            Ok(())
        }
    }

    fn poison(&mut self, error: OperationalStoreError) -> DurableAuthorityError {
        self.poisoned = true;
        DurableAuthorityError::Store(error)
    }
}

fn open_keyed(path: &Path, key: &[u8]) -> Result<OperationalStore, OperationalStoreError> {
    let connection = open_connection(path, key)?;
    migrate(&connection)?;
    claim_exclusive_writer(&connection)?;
    let mut store = OperationalStore {
        connection,
        generation: 0,
        poisoned: false,
    };
    let _ = store.load_authority()?;
    Ok(store)
}

fn open_connection(path: &Path, key: &[u8]) -> Result<Connection, OperationalStoreError> {
    if key.len() != KEY_BYTES {
        return Err(OperationalStoreError::InvalidKey);
    }
    let mut flags = OpenFlags::SQLITE_OPEN_READ_WRITE
        | OpenFlags::SQLITE_OPEN_CREATE
        | OpenFlags::SQLITE_OPEN_NO_MUTEX;
    if !path.parent().is_some_and(is_linux_held_descriptor_path) {
        flags |= OpenFlags::SQLITE_OPEN_NOFOLLOW;
    }
    let connection = Connection::open_with_flags(path, flags).map_err(classify_open_error)?;
    connection
        .busy_timeout(Duration::ZERO)
        .map_err(|_| OperationalStoreError::OpenFailed)?;
    connection
        .execute_batch(
            "PRAGMA cipher_log_level = NONE;
             PRAGMA cipher_log_source = NONE;",
        )
        .map_err(|_| OperationalStoreError::CipherUnavailable)?;
    let key = Zeroizing::new(raw_key_pragma_value(key));
    connection
        .pragma_update(None, "key", key.as_str())
        .map_err(|_| OperationalStoreError::OpenFailed)?;
    let cipher_version: Option<String> = connection
        .query_row("PRAGMA cipher_version", [], |row| row.get(0))
        .optional()
        .map_err(|_| OperationalStoreError::CipherUnavailable)?;
    if cipher_version.as_deref().is_none_or(str::is_empty) {
        return Err(OperationalStoreError::CipherUnavailable);
    }
    connection
        .execute_batch(
            "PRAGMA cipher_memory_security = ON;
             PRAGMA foreign_keys = ON;
             PRAGMA trusted_schema = OFF;
             PRAGMA secure_delete = ON;
             PRAGMA temp_store = MEMORY;
             PRAGMA synchronous = FULL;
             PRAGMA wal_autocheckpoint = 1;",
        )
        .map_err(|_| OperationalStoreError::OpenFailed)?;
    Ok(connection)
}

fn claim_exclusive_writer(connection: &Connection) -> Result<(), OperationalStoreError> {
    let mode: String = connection
        .pragma_update_and_check(None, "locking_mode", "EXCLUSIVE", |row| row.get(0))
        .map_err(classify_open_error)?;
    if !mode.eq_ignore_ascii_case("exclusive") {
        return Err(OperationalStoreError::ConcurrentWriter);
    }
    let journal: String = connection
        .pragma_update_and_check(None, "journal_mode", "WAL", |row| row.get(0))
        .map_err(classify_open_error)?;
    if !journal.eq_ignore_ascii_case("wal") {
        return Err(OperationalStoreError::OpenFailed);
    }
    connection
        .execute_batch("BEGIN EXCLUSIVE; COMMIT;")
        .map_err(classify_open_error)
}

fn migrate(connection: &Connection) -> Result<(), OperationalStoreError> {
    let mut version: i64 = connection
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .map_err(|_| OperationalStoreError::MigrationFailed)?;
    if version > SCHEMA_VERSION {
        return Err(OperationalStoreError::MigrationFailed);
    }
    if version == 0 {
        let transaction = connection
            .unchecked_transaction()
            .map_err(|_| OperationalStoreError::MigrationFailed)?;
        transaction
            .execute_batch(MIGRATION_1_SCHEMA_SQL)
            .map_err(|_| OperationalStoreError::MigrationFailed)?;
        transaction
            .execute(
                "INSERT INTO schema_history(version, migration_sha256) VALUES (1, ?1)",
                [sha256_hex(MIGRATION_1_SCHEMA_SQL.as_bytes())],
            )
            .map_err(|_| OperationalStoreError::MigrationFailed)?;
        transaction
            .execute(
                "INSERT INTO store_metadata(singleton, generation, state_sha256)
                 VALUES (1, 0, ?1)",
                [ZERO_SHA256],
            )
            .map_err(|_| OperationalStoreError::MigrationFailed)?;
        transaction
            .execute(
                "INSERT INTO checkpoints(generation, state_sha256) VALUES (0, ?1)",
                [ZERO_SHA256],
            )
            .map_err(|_| OperationalStoreError::MigrationFailed)?;
        transaction
            .pragma_update(None, "user_version", 1)
            .map_err(|_| OperationalStoreError::MigrationFailed)?;
        transaction
            .commit()
            .map_err(|_| OperationalStoreError::MigrationFailed)?;
        version = 1;
    }
    if version == 1 {
        let transaction = connection
            .unchecked_transaction()
            .map_err(|_| OperationalStoreError::MigrationFailed)?;
        transaction
            .execute_batch(MIGRATION_2_SCHEMA_SQL)
            .map_err(|_| OperationalStoreError::MigrationFailed)?;
        transaction
            .execute(
                "INSERT INTO schema_history(version, migration_sha256) VALUES (2, ?1)",
                [sha256_hex(MIGRATION_2_SCHEMA_SQL.as_bytes())],
            )
            .map_err(|_| OperationalStoreError::MigrationFailed)?;
        transaction
            .pragma_update(None, "user_version", SCHEMA_VERSION)
            .map_err(|_| OperationalStoreError::MigrationFailed)?;
        transaction
            .commit()
            .map_err(|_| OperationalStoreError::MigrationFailed)?;
    }
    verify_schema_history(connection)
}

fn verify_schema_history(connection: &Connection) -> Result<(), OperationalStoreError> {
    let rows: Vec<(i64, String)> = connection
        .prepare("SELECT version, migration_sha256 FROM schema_history ORDER BY version")
        .and_then(|mut statement| {
            statement
                .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
                .collect()
        })
        .map_err(|_| OperationalStoreError::MigrationFailed)?;
    if rows
        != [
            (1, sha256_hex(MIGRATION_1_SCHEMA_SQL.as_bytes())),
            (2, sha256_hex(MIGRATION_2_SCHEMA_SQL.as_bytes())),
        ]
    {
        return Err(OperationalStoreError::MigrationFailed);
    }
    Ok(())
}

fn persist_snapshot(
    connection: &mut Connection,
    expected_generation: u64,
    next_generation: u64,
    state_sha256: &str,
    issuer: &GrantIssuer,
    coordinator: &AuthorityTransactionCoordinator,
) -> Result<(), OperationalStoreError> {
    let expected_generation = i64::try_from(expected_generation)
        .map_err(|_| OperationalStoreError::PersistenceFailure)?;
    let next_generation =
        i64::try_from(next_generation).map_err(|_| OperationalStoreError::PersistenceFailure)?;
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|_| OperationalStoreError::PersistenceFailure)?;
    let retained: i64 = transaction
        .query_row(
            "SELECT generation FROM store_metadata WHERE singleton = 1",
            [],
            |row| row.get(0),
        )
        .map_err(|_| OperationalStoreError::PersistenceFailure)?;
    if retained != expected_generation {
        return Err(OperationalStoreError::PersistenceFailure);
    }
    persist_grants(&transaction, issuer)?;
    persist_transactions(&transaction, coordinator)?;
    let changed = transaction
        .execute(
            "UPDATE store_metadata
             SET generation = ?1, state_sha256 = ?2
             WHERE singleton = 1 AND generation = ?3",
            params![next_generation, state_sha256, expected_generation],
        )
        .map_err(|_| OperationalStoreError::PersistenceFailure)?;
    if changed != 1 {
        return Err(OperationalStoreError::PersistenceFailure);
    }
    transaction
        .execute(
            "INSERT INTO checkpoints(generation, state_sha256) VALUES (?1, ?2)",
            params![next_generation, state_sha256],
        )
        .map_err(|_| OperationalStoreError::PersistenceFailure)?;
    transaction
        .commit()
        .map_err(|_| OperationalStoreError::PersistenceFailure)
}

fn persist_grants(
    transaction: &Transaction<'_>,
    issuer: &GrantIssuer,
) -> Result<(), OperationalStoreError> {
    let parts = issuer.durable_parts();
    let histories = parts.histories;
    let revision_hashes = parts.revision_hashes;
    let used_nonces = parts.used_nonces;
    for (grant_id, history) in histories {
        let nonce = history
            .first()
            .map(|record| record.nonce.as_str())
            .ok_or(OperationalStoreError::IntegrityFailure)?;
        transaction
            .execute(
                "INSERT INTO grant_identities(grant_id, nonce) VALUES (?1, ?2)
                 ON CONFLICT(grant_id) DO NOTHING",
                params![grant_id.as_str(), nonce],
            )
            .map_err(|_| OperationalStoreError::PersistenceFailure)?;
        let retained_nonce: String = transaction
            .query_row(
                "SELECT nonce FROM grant_identities WHERE grant_id = ?1",
                [grant_id.as_str()],
                |row| row.get(0),
            )
            .map_err(|_| OperationalStoreError::IntegrityFailure)?;
        if retained_nonce != nonce || !used_nonces.contains(&GrantNonce::from_raw(retained_nonce)) {
            return Err(OperationalStoreError::IntegrityFailure);
        }
        for record in history {
            let canonical =
                to_canonical_json(record).map_err(|_| OperationalStoreError::IntegrityFailure)?;
            let digest = revision_hashes
                .get(&(grant_id.clone(), record.revision))
                .ok_or(OperationalStoreError::IntegrityFailure)?;
            insert_immutable_record(
                transaction,
                "INSERT INTO grant_revisions(grant_id, revision, record_sha256, record_json)
                 VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT(grant_id, revision) DO NOTHING",
                "SELECT record_sha256, record_json FROM grant_revisions
                 WHERE grant_id = ?1 AND revision = ?2",
                grant_id.as_str(),
                record.revision,
                digest,
                &canonical,
            )?;
        }
        let revision = history
            .last()
            .map(|record| record.revision)
            .ok_or(OperationalStoreError::IntegrityFailure)?;
        transaction
            .execute(
                "INSERT INTO grant_heads(grant_id, current_revision) VALUES (?1, ?2)
                 ON CONFLICT(grant_id) DO UPDATE SET current_revision = excluded.current_revision",
                params![grant_id.as_str(), revision],
            )
            .map_err(|_| OperationalStoreError::PersistenceFailure)?;
    }
    Ok(())
}

fn persist_transactions(
    transaction: &Transaction<'_>,
    coordinator: &AuthorityTransactionCoordinator,
) -> Result<(), OperationalStoreError> {
    let (histories, receipts) = coordinator.durable_parts();
    for (transaction_id, history) in histories {
        let attempt_id = history
            .first()
            .map(|record| record.operation_attempt_id.as_str())
            .ok_or(OperationalStoreError::IntegrityFailure)?;
        transaction
            .execute(
                "INSERT INTO transaction_identities(transaction_id, attempt_id) VALUES (?1, ?2)
                 ON CONFLICT(transaction_id) DO NOTHING",
                params![transaction_id.as_str(), attempt_id],
            )
            .map_err(|_| OperationalStoreError::PersistenceFailure)?;
        let retained_attempt: String = transaction
            .query_row(
                "SELECT attempt_id FROM transaction_identities WHERE transaction_id = ?1",
                [transaction_id.as_str()],
                |row| row.get(0),
            )
            .map_err(|_| OperationalStoreError::IntegrityFailure)?;
        if retained_attempt != attempt_id {
            return Err(OperationalStoreError::IntegrityFailure);
        }
        for record in history {
            let canonical =
                to_canonical_json(record).map_err(|_| OperationalStoreError::IntegrityFailure)?;
            let digest = sha256_hex(&canonical);
            insert_immutable_transaction(
                transaction,
                transaction_id.as_str(),
                record,
                &digest,
                &canonical,
            )?;
        }
        let revision = history
            .last()
            .map(|record| record.revision)
            .ok_or(OperationalStoreError::IntegrityFailure)?;
        transaction
            .execute(
                "INSERT INTO transaction_heads(transaction_id, current_revision) VALUES (?1, ?2)
                 ON CONFLICT(transaction_id) DO UPDATE SET current_revision = excluded.current_revision",
                params![transaction_id.as_str(), revision],
            )
            .map_err(|_| OperationalStoreError::PersistenceFailure)?;
    }
    for receipt in receipts {
        let sequence =
            i64::try_from(receipt.sequence).map_err(|_| OperationalStoreError::IntegrityFailure)?;
        let canonical =
            to_canonical_json(receipt).map_err(|_| OperationalStoreError::IntegrityFailure)?;
        transaction
            .execute(
                "INSERT INTO receipts(
                    sequence, receipt_id, transaction_id, receipt_sha256,
                    previous_receipt_sha256, receipt_json
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                 ON CONFLICT(sequence) DO NOTHING",
                params![
                    sequence,
                    receipt.receipt_id.as_str(),
                    receipt.authority_transaction_id.as_str(),
                    receipt.receipt_sha256,
                    receipt.previous_receipt_sha256,
                    canonical,
                ],
            )
            .map_err(|_| OperationalStoreError::PersistenceFailure)?;
        let retained: (String, Vec<u8>) = transaction
            .query_row(
                "SELECT receipt_sha256, receipt_json FROM receipts WHERE sequence = ?1",
                [sequence],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .map_err(|_| OperationalStoreError::IntegrityFailure)?;
        if retained.0 != receipt.receipt_sha256 || retained.1 != canonical {
            return Err(OperationalStoreError::IntegrityFailure);
        }
    }
    Ok(())
}

fn insert_immutable_record(
    transaction: &Transaction<'_>,
    insert_sql: &str,
    select_sql: &str,
    identity: &str,
    revision: u32,
    digest: &str,
    canonical: &[u8],
) -> Result<(), OperationalStoreError> {
    transaction
        .execute(insert_sql, params![identity, revision, digest, canonical])
        .map_err(|_| OperationalStoreError::PersistenceFailure)?;
    let retained: (String, Vec<u8>) = transaction
        .query_row(select_sql, params![identity, revision], |row| {
            Ok((row.get(0)?, row.get(1)?))
        })
        .map_err(|_| OperationalStoreError::IntegrityFailure)?;
    if retained.0 != digest || retained.1 != canonical {
        return Err(OperationalStoreError::IntegrityFailure);
    }
    Ok(())
}

fn insert_immutable_transaction(
    transaction: &Transaction<'_>,
    transaction_id: &str,
    record: &AuthorityTransactionRecord,
    digest: &str,
    canonical: &[u8],
) -> Result<(), OperationalStoreError> {
    transaction
        .execute(
            "INSERT INTO transaction_revisions(
                transaction_id, revision, state, record_sha256, record_json
             ) VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(transaction_id, revision) DO NOTHING",
            params![
                transaction_id,
                record.revision,
                transaction_state_code(record.state),
                digest,
                canonical,
            ],
        )
        .map_err(|_| OperationalStoreError::PersistenceFailure)?;
    let retained: (String, String, Vec<u8>) = transaction
        .query_row(
            "SELECT state, record_sha256, record_json FROM transaction_revisions
             WHERE transaction_id = ?1 AND revision = ?2",
            params![transaction_id, record.revision],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .map_err(|_| OperationalStoreError::IntegrityFailure)?;
    if retained.0 != transaction_state_code(record.state)
        || retained.1 != digest
        || retained.2 != canonical
    {
        return Err(OperationalStoreError::IntegrityFailure);
    }
    Ok(())
}

fn load_grants(
    connection: &Connection,
) -> Result<BTreeMap<GrantId, Vec<CapabilityGrant>>, OperationalStoreError> {
    let mut statement = connection
        .prepare(
            "SELECT grant_id, revision, record_sha256, record_json
             FROM grant_revisions ORDER BY grant_id, revision",
        )
        .map_err(|_| OperationalStoreError::IntegrityFailure)?;
    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, u32>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, Vec<u8>>(3)?,
            ))
        })
        .map_err(|_| OperationalStoreError::IntegrityFailure)?;
    let mut histories: BTreeMap<GrantId, Vec<CapabilityGrant>> = BTreeMap::new();
    for row in rows {
        let (grant_id, revision, digest, canonical) =
            row.map_err(|_| OperationalStoreError::IntegrityFailure)?;
        if sha256_hex(&canonical) != digest {
            return Err(OperationalStoreError::IntegrityFailure);
        }
        let record = from_json::<CapabilityGrant>(&canonical)
            .map_err(|_| OperationalStoreError::IntegrityFailure)?;
        if record.grant_id.as_str() != grant_id || record.revision != revision {
            return Err(OperationalStoreError::IntegrityFailure);
        }
        histories
            .entry(GrantId::from_raw(grant_id))
            .or_default()
            .push(record);
    }
    verify_grant_heads(connection, &histories)?;
    Ok(histories)
}

fn load_grant_hashes(
    connection: &Connection,
) -> Result<BTreeMap<(GrantId, u32), String>, OperationalStoreError> {
    let mut statement = connection
        .prepare(
            "SELECT grant_id, revision, record_sha256
             FROM grant_revisions ORDER BY grant_id, revision",
        )
        .map_err(|_| OperationalStoreError::IntegrityFailure)?;
    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, u32>(1)?,
                row.get::<_, String>(2)?,
            ))
        })
        .map_err(|_| OperationalStoreError::IntegrityFailure)?;
    let mut values = BTreeMap::new();
    for row in rows {
        let (grant_id, revision, digest) =
            row.map_err(|_| OperationalStoreError::IntegrityFailure)?;
        values.insert((GrantId::from_raw(grant_id), revision), digest);
    }
    Ok(values)
}

fn load_nonces(connection: &Connection) -> Result<BTreeSet<GrantNonce>, OperationalStoreError> {
    let mut statement = connection
        .prepare("SELECT nonce FROM grant_identities ORDER BY nonce")
        .map_err(|_| OperationalStoreError::IntegrityFailure)?;
    let rows = statement
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(|_| OperationalStoreError::IntegrityFailure)?;
    rows.map(|row| {
        row.map(GrantNonce::from_raw)
            .map_err(|_| OperationalStoreError::IntegrityFailure)
    })
    .collect()
}

fn verify_grant_heads(
    connection: &Connection,
    histories: &BTreeMap<GrantId, Vec<CapabilityGrant>>,
) -> Result<(), OperationalStoreError> {
    let mut statement = connection
        .prepare("SELECT grant_id, current_revision FROM grant_heads ORDER BY grant_id")
        .map_err(|_| OperationalStoreError::IntegrityFailure)?;
    let rows = statement
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, u32>(1)?))
        })
        .map_err(|_| OperationalStoreError::IntegrityFailure)?;
    let mut heads = BTreeMap::new();
    for row in rows {
        let (grant_id, revision) = row.map_err(|_| OperationalStoreError::IntegrityFailure)?;
        heads.insert(GrantId::from_raw(grant_id), revision);
    }
    if heads.len() != histories.len()
        || histories.iter().any(|(grant_id, history)| {
            history.last().map(|record| record.revision) != heads.get(grant_id).copied()
        })
    {
        return Err(OperationalStoreError::IntegrityFailure);
    }
    let mut identities = BTreeMap::new();
    let mut statement = connection
        .prepare("SELECT grant_id, nonce FROM grant_identities ORDER BY grant_id")
        .map_err(|_| OperationalStoreError::IntegrityFailure)?;
    let rows = statement
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(|_| OperationalStoreError::IntegrityFailure)?;
    for row in rows {
        let (grant_id, nonce) = row.map_err(|_| OperationalStoreError::IntegrityFailure)?;
        identities.insert(GrantId::from_raw(grant_id), GrantNonce::from_raw(nonce));
    }
    if identities.len() != histories.len()
        || histories.iter().any(|(grant_id, history)| {
            history.first().map(|record| &record.nonce) != identities.get(grant_id)
        })
    {
        return Err(OperationalStoreError::IntegrityFailure);
    }
    Ok(())
}

fn load_transactions(
    connection: &Connection,
) -> Result<BTreeMap<AuthorityTransactionId, Vec<AuthorityTransactionRecord>>, OperationalStoreError>
{
    let mut statement = connection
        .prepare(
            "SELECT transaction_id, revision, state, record_sha256, record_json
             FROM transaction_revisions ORDER BY transaction_id, revision",
        )
        .map_err(|_| OperationalStoreError::IntegrityFailure)?;
    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, u32>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, Vec<u8>>(4)?,
            ))
        })
        .map_err(|_| OperationalStoreError::IntegrityFailure)?;
    let mut histories: BTreeMap<AuthorityTransactionId, Vec<AuthorityTransactionRecord>> =
        BTreeMap::new();
    for row in rows {
        let (transaction_id, revision, state, digest, canonical) =
            row.map_err(|_| OperationalStoreError::IntegrityFailure)?;
        if sha256_hex(&canonical) != digest {
            return Err(OperationalStoreError::IntegrityFailure);
        }
        let record = from_json::<AuthorityTransactionRecord>(&canonical)
            .map_err(|_| OperationalStoreError::IntegrityFailure)?;
        if record.authority_transaction_id.as_str() != transaction_id
            || record.revision != revision
            || transaction_state_code(record.state) != state
        {
            return Err(OperationalStoreError::IntegrityFailure);
        }
        histories
            .entry(AuthorityTransactionId::from_raw(transaction_id))
            .or_default()
            .push(record);
    }
    verify_transaction_heads(connection, &histories)?;
    Ok(histories)
}

fn verify_transaction_heads(
    connection: &Connection,
    histories: &BTreeMap<AuthorityTransactionId, Vec<AuthorityTransactionRecord>>,
) -> Result<(), OperationalStoreError> {
    let mut statement = connection
        .prepare(
            "SELECT transaction_id, current_revision FROM transaction_heads
             ORDER BY transaction_id",
        )
        .map_err(|_| OperationalStoreError::IntegrityFailure)?;
    let rows = statement
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, u32>(1)?))
        })
        .map_err(|_| OperationalStoreError::IntegrityFailure)?;
    let mut heads = BTreeMap::new();
    for row in rows {
        let (transaction_id, revision) =
            row.map_err(|_| OperationalStoreError::IntegrityFailure)?;
        heads.insert(AuthorityTransactionId::from_raw(transaction_id), revision);
    }
    if heads.len() != histories.len()
        || histories.iter().any(|(transaction_id, history)| {
            history.last().map(|record| record.revision) != heads.get(transaction_id).copied()
        })
    {
        return Err(OperationalStoreError::IntegrityFailure);
    }
    let mut identities = BTreeMap::new();
    let mut statement = connection
        .prepare(
            "SELECT transaction_id, attempt_id FROM transaction_identities
             ORDER BY transaction_id",
        )
        .map_err(|_| OperationalStoreError::IntegrityFailure)?;
    let rows = statement
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(|_| OperationalStoreError::IntegrityFailure)?;
    for row in rows {
        let (transaction_id, attempt_id) =
            row.map_err(|_| OperationalStoreError::IntegrityFailure)?;
        identities.insert(AuthorityTransactionId::from_raw(transaction_id), attempt_id);
    }
    if identities.len() != histories.len()
        || histories.iter().any(|(transaction_id, history)| {
            history
                .first()
                .map(|record| record.operation_attempt_id.as_str())
                != identities.get(transaction_id).map(String::as_str)
        })
    {
        return Err(OperationalStoreError::IntegrityFailure);
    }
    Ok(())
}

fn load_receipts(connection: &Connection) -> Result<Vec<Receipt>, OperationalStoreError> {
    let mut statement = connection
        .prepare(
            "SELECT sequence, receipt_id, transaction_id, receipt_sha256,
                    previous_receipt_sha256, receipt_json
             FROM receipts ORDER BY sequence",
        )
        .map_err(|_| OperationalStoreError::IntegrityFailure)?;
    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, Vec<u8>>(5)?,
            ))
        })
        .map_err(|_| OperationalStoreError::IntegrityFailure)?;
    let mut receipts = Vec::new();
    for row in rows {
        let (sequence, receipt_id, transaction_id, digest, previous, canonical) =
            row.map_err(|_| OperationalStoreError::IntegrityFailure)?;
        let sequence =
            u64::try_from(sequence).map_err(|_| OperationalStoreError::IntegrityFailure)?;
        let receipt = from_json::<Receipt>(&canonical)
            .map_err(|_| OperationalStoreError::IntegrityFailure)?;
        if receipt.sequence != sequence
            || receipt.receipt_id.as_str() != receipt_id
            || receipt.authority_transaction_id.as_str() != transaction_id
            || receipt.receipt_sha256 != digest
            || receipt.previous_receipt_sha256 != previous
        {
            return Err(OperationalStoreError::IntegrityFailure);
        }
        receipts.push(receipt);
    }
    Ok(receipts)
}

fn verify_checkpoint_head(
    connection: &Connection,
    generation: u64,
    state_sha256: &str,
) -> Result<(), OperationalStoreError> {
    let expected_count = generation
        .checked_add(1)
        .ok_or(OperationalStoreError::IntegrityFailure)?;
    let (count, maximum, retained): (i64, i64, String) = connection
        .query_row(
            "SELECT COUNT(*), MAX(generation),
                    COALESCE((SELECT state_sha256 FROM checkpoints
                              WHERE generation = ?1), '')
             FROM checkpoints",
            [i64::try_from(generation).map_err(|_| OperationalStoreError::IntegrityFailure)?],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .map_err(|_| OperationalStoreError::IntegrityFailure)?;
    if u64::try_from(count).ok() != Some(expected_count)
        || u64::try_from(maximum).ok() != Some(generation)
        || retained != state_sha256
    {
        return Err(OperationalStoreError::IntegrityFailure);
    }
    Ok(())
}

fn authority_state_sha256(
    issuer: &GrantIssuer,
    coordinator: &AuthorityTransactionCoordinator,
) -> Result<String, OperationalStoreError> {
    let mut digest = Sha256::new();
    digest.update(b"agentmage.authority-state.v1\0");
    let grant_parts = issuer.durable_parts();
    let grant_histories = grant_parts.histories;
    let grant_hashes = grant_parts.revision_hashes;
    let nonces = grant_parts.used_nonces;
    for (grant_id, history) in grant_histories {
        digest.update(grant_id.as_str().as_bytes());
        digest.update([0]);
        for record in history {
            digest.update(
                to_canonical_json(record).map_err(|_| OperationalStoreError::IntegrityFailure)?,
            );
            digest.update([0]);
        }
    }
    for ((grant_id, revision), value) in grant_hashes {
        digest.update(grant_id.as_str().as_bytes());
        digest.update(revision.to_be_bytes());
        digest.update(value.as_bytes());
    }
    for nonce in nonces {
        digest.update(nonce.as_str().as_bytes());
        digest.update([0]);
    }
    let (transaction_histories, receipts) = coordinator.durable_parts();
    for (transaction_id, history) in transaction_histories {
        digest.update(transaction_id.as_str().as_bytes());
        digest.update([0]);
        for record in history {
            digest.update(
                to_canonical_json(record).map_err(|_| OperationalStoreError::IntegrityFailure)?,
            );
            digest.update([0]);
        }
    }
    for receipt in receipts {
        digest.update(
            to_canonical_json(receipt).map_err(|_| OperationalStoreError::IntegrityFailure)?,
        );
        digest.update([0]);
    }
    Ok(hex_digest(&digest.finalize()))
}

fn verify_integrity(connection: &Connection) -> Result<(), OperationalStoreError> {
    let cipher_errors = connection
        .prepare("PRAGMA cipher_integrity_check")
        .and_then(|mut statement| {
            statement
                .query_map([], |row| row.get::<_, String>(0))?
                .collect::<Result<Vec<_>, _>>()
        })
        .map_err(|_| OperationalStoreError::IntegrityFailure)?;
    if !cipher_errors.is_empty() && cipher_errors != ["ok"] {
        return Err(OperationalStoreError::IntegrityFailure);
    }
    let quick: String = connection
        .query_row("PRAGMA quick_check", [], |row| row.get(0))
        .map_err(|_| OperationalStoreError::IntegrityFailure)?;
    if quick != "ok" {
        return Err(OperationalStoreError::IntegrityFailure);
    }
    let foreign_keys: Option<String> = connection
        .query_row("PRAGMA foreign_key_check", [], |row| row.get(0))
        .optional()
        .map_err(|_| OperationalStoreError::IntegrityFailure)?;
    if foreign_keys.is_some() {
        return Err(OperationalStoreError::IntegrityFailure);
    }
    Ok(())
}

fn prepare_store_file(path: &Path) -> Result<(), OperationalStoreError> {
    let parent = path.parent().ok_or(OperationalStoreError::OpenFailed)?;
    let metadata = fs::symlink_metadata(parent).map_err(|_| OperationalStoreError::OpenFailed)?;
    let direct_directory = metadata.is_dir() && !metadata.file_type().is_symlink();
    let held_descriptor_directory = metadata.file_type().is_symlink()
        && is_linux_held_descriptor_path(parent)
        && fs::metadata(parent).is_ok_and(|resolved| resolved.is_dir());
    if !direct_directory && !held_descriptor_directory {
        return Err(OperationalStoreError::OpenFailed);
    }
    if path.exists() {
        verify_store_file(path)
    } else {
        prepare_new_store_file(path)
    }
}

fn is_linux_held_descriptor_path(path: &Path) -> bool {
    let mut components = path.components();
    matches!(components.next(), Some(Component::RootDir))
        && matches!(components.next(), Some(Component::Normal(value)) if value == "proc")
        && matches!(components.next(), Some(Component::Normal(value)) if value == "self")
        && matches!(components.next(), Some(Component::Normal(value)) if value == "fd")
        && matches!(components.next(), Some(Component::Normal(value)) if value.to_str().is_some_and(|value| !value.is_empty() && value.bytes().all(|byte| byte.is_ascii_digit())))
        && components.next().is_none()
}

fn prepare_new_store_file(path: &Path) -> Result<(), OperationalStoreError> {
    if path.exists() {
        return Err(OperationalStoreError::OpenFailed);
    }
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)
        .map_err(|_| OperationalStoreError::OpenFailed)?;
    verify_store_file(path)
}

fn verify_store_file(path: &Path) -> Result<(), OperationalStoreError> {
    let metadata = fs::symlink_metadata(path).map_err(|_| OperationalStoreError::OpenFailed)?;
    if !metadata.is_file() || metadata.file_type().is_symlink() || metadata.mode() & 0o077 != 0 {
        return Err(OperationalStoreError::OpenFailed);
    }
    Ok(())
}

fn classify_open_error(error: rusqlite::Error) -> OperationalStoreError {
    match error.sqlite_error_code() {
        Some(rusqlite::ErrorCode::DatabaseBusy | rusqlite::ErrorCode::DatabaseLocked) => {
            OperationalStoreError::ConcurrentWriter
        }
        _ => OperationalStoreError::OpenFailed,
    }
}

fn raw_key_pragma_value(key: &[u8]) -> String {
    use fmt::Write as _;
    let mut value = String::with_capacity(key.len() * 2 + 3);
    value.push_str("x'");
    for byte in key {
        write!(&mut value, "{byte:02x}").expect("writing to a String cannot fail");
    }
    value.push('\'');
    value
}

const fn transaction_state_code(state: AuthorityTransactionState) -> &'static str {
    match state {
        AuthorityTransactionState::Prepared => "prepared",
        AuthorityTransactionState::GrantConsumed => "grant_consumed",
        AuthorityTransactionState::AttemptRecorded => "attempt_recorded",
        AuthorityTransactionState::LaunchCommitted => "launch_committed",
        AuthorityTransactionState::Reconciling => "reconciling",
        AuthorityTransactionState::Terminal => "terminal",
    }
}

fn sha256_hex(value: &[u8]) -> String {
    hex_digest(&Sha256::digest(value))
}

fn hex_digest(value: &[u8]) -> String {
    use fmt::Write as _;
    let mut encoded = String::with_capacity(value.len() * 2);
    for byte in value {
        write!(&mut encoded, "{byte:02x}").expect("writing to a String cannot fail");
    }
    encoded
}

#[cfg(test)]
mod tests {
    use std::fs::{self, File, OpenOptions};
    use std::io::{Read as _, Seek as _, SeekFrom, Write as _};
    use std::os::fd::AsRawFd;
    use std::path::Path;
    use std::sync::atomic::{AtomicU64, Ordering};

    use agentmage_kernel_contracts::{
        CloudSynchronizationMarker, StorageFilesystemClass, StrictLocalStorageObservation,
    };
    use rusqlite::params;

    use super::{
        MIGRATION_1_SCHEMA_SQL, MIGRATION_2_SCHEMA_SQL, OperationalStore, OperationalStoreError,
        OperationalStoreKeyError, OperationalStoreKeyProvider, SCHEMA_VERSION,
        is_linux_held_descriptor_path, open_connection, prepare_new_store_file, sha256_hex,
    };
    use crate::authority_transaction::AuthorityTransactionCoordinator;
    use crate::grants::GrantIssuer;

    static NEXT_DIRECTORY: AtomicU64 = AtomicU64::new(1);

    struct TestKey([u8; 32]);

    impl OperationalStoreKeyProvider for TestKey {
        fn with_key<T>(
            &mut self,
            operation: impl FnOnce(&[u8]) -> T,
        ) -> Result<T, OperationalStoreKeyError> {
            Ok(operation(&self.0))
        }
    }

    struct MissingKey;

    impl OperationalStoreKeyProvider for MissingKey {
        fn with_key<T>(
            &mut self,
            _operation: impl FnOnce(&[u8]) -> T,
        ) -> Result<T, OperationalStoreKeyError> {
            Err(OperationalStoreKeyError::Unavailable)
        }
    }

    fn observation() -> StrictLocalStorageObservation {
        StrictLocalStorageObservation {
            filesystem: StorageFilesystemClass::Local,
            synchronization_marker: None,
            root_identity_sha256: [7; 32],
            symlink_free: true,
        }
    }

    fn temporary_directory() -> std::path::PathBuf {
        let sequence = NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "agentmage-operational-store-{}-{sequence}",
            std::process::id()
        ));
        fs::create_dir(&path).expect("temporary directory");
        path
    }

    fn create_version_one_store(path: &Path, key: &[u8; 32]) {
        prepare_new_store_file(path).expect("legacy store file");
        let connection = open_connection(path, key).expect("legacy encrypted connection");
        let transaction = connection
            .unchecked_transaction()
            .expect("legacy migration transaction");
        transaction
            .execute_batch(MIGRATION_1_SCHEMA_SQL)
            .expect("legacy schema");
        transaction
            .execute(
                "INSERT INTO schema_history(version, migration_sha256) VALUES (1, ?1)",
                [sha256_hex(MIGRATION_1_SCHEMA_SQL.as_bytes())],
            )
            .expect("legacy history");
        transaction
            .execute(
                "INSERT INTO store_metadata(singleton, generation, state_sha256)
                 VALUES (1, 0, ?1)",
                [super::ZERO_SHA256],
            )
            .expect("legacy metadata");
        transaction
            .execute(
                "INSERT INTO checkpoints(generation, state_sha256) VALUES (0, ?1)",
                [super::ZERO_SHA256],
            )
            .expect("legacy checkpoint");
        transaction
            .pragma_update(None, "user_version", 1)
            .expect("legacy user version");
        transaction.commit().expect("legacy migration commit");
    }

    #[test]
    fn encrypted_store_requires_key_and_hides_sqlite_header() {
        let directory = temporary_directory();
        let path = directory.join("authority.db");
        assert_eq!(
            OperationalStore::open(&path, &observation(), &mut MissingKey)
                .expect_err("missing key must fail"),
            OperationalStoreError::KeyUnavailable
        );
        assert!(!path.exists());

        let store = OperationalStore::open(&path, &observation(), &mut TestKey([3; 32]))
            .expect("encrypted store");
        assert_eq!(store.generation(), 0);
        let bytes = fs::read(&path).expect("database bytes");
        assert!(!bytes.starts_with(b"SQLite format 3\0"));
        drop(store);
        fs::remove_dir_all(directory).expect("cleanup");
    }

    #[test]
    fn encrypted_store_opens_through_exact_held_linux_directory_descriptor() {
        let directory = temporary_directory();
        let held = File::open(&directory).expect("directory descriptor");
        let path =
            std::path::PathBuf::from(format!("/proc/self/fd/{}/authority.db", held.as_raw_fd()));
        assert!(is_linux_held_descriptor_path(
            path.parent().expect("descriptor parent")
        ));
        let store = OperationalStore::open(&path, &observation(), &mut TestKey([44; 32]))
            .expect("held-descriptor store");
        assert!(directory.join("authority.db").is_file());
        assert_eq!(store.generation(), 0);
        drop(store);
        drop(held);
        fs::remove_dir_all(directory).expect("cleanup");
    }

    #[test]
    fn only_exact_numeric_proc_self_fd_parent_is_a_held_descriptor_path() {
        assert!(is_linux_held_descriptor_path(Path::new("/proc/self/fd/9")));
        for path in [
            "/proc/self/fd",
            "/proc/self/fd/not-a-number",
            "/proc/self/fd/9/child",
            "/tmp/proc/self/fd/9",
        ] {
            assert!(!is_linux_held_descriptor_path(Path::new(path)));
        }
    }

    #[test]
    fn wrong_key_and_wrong_storage_class_fail_closed() {
        let directory = temporary_directory();
        let path = directory.join("authority.db");
        drop(
            OperationalStore::open(&path, &observation(), &mut TestKey([5; 32]))
                .expect("encrypted store"),
        );
        assert!(OperationalStore::open(&path, &observation(), &mut TestKey([6; 32])).is_err());
        let remote = StrictLocalStorageObservation {
            filesystem: StorageFilesystemClass::Remote,
            synchronization_marker: None,
            root_identity_sha256: [7; 32],
            symlink_free: true,
        };
        assert_eq!(
            OperationalStore::open(&directory.join("remote.db"), &remote, &mut TestKey([1; 32]))
                .expect_err("remote storage must fail"),
            OperationalStoreError::StorageRejected
        );
        let synchronized = StrictLocalStorageObservation {
            filesystem: StorageFilesystemClass::Local,
            synchronization_marker: Some(CloudSynchronizationMarker::OtherKnownMarker),
            root_identity_sha256: [7; 32],
            symlink_free: true,
        };
        assert_eq!(
            OperationalStore::open(
                &directory.join("synchronized.db"),
                &synchronized,
                &mut TestKey([1; 32]),
            )
            .expect_err("synchronized storage must fail"),
            OperationalStoreError::StorageRejected
        );
        fs::remove_dir_all(directory).expect("cleanup");
    }

    #[test]
    fn exclusive_writer_and_encrypted_backup_are_verified() {
        let directory = temporary_directory();
        let path = directory.join("authority.db");
        let backup = directory.join("authority.backup.db");
        let store = OperationalStore::open(&path, &observation(), &mut TestKey([8; 32]))
            .expect("first writer");
        assert!(OperationalStore::open(&path, &observation(), &mut TestKey([8; 32])).is_err());
        store
            .backup(&backup, &observation(), &mut TestKey([9; 32]))
            .expect("encrypted backup");
        let bytes = fs::read(&backup).expect("backup bytes");
        assert!(!bytes.starts_with(b"SQLite format 3\0"));
        drop(store);
        let restored = OperationalStore::open(&backup, &observation(), &mut TestKey([9; 32]))
            .expect("backup reopens");
        assert_eq!(restored.generation(), 0);
        drop(restored);
        fs::remove_dir_all(directory).expect("cleanup");
    }

    #[test]
    fn schema_constraints_and_atomic_rollback_reject_partial_authority() {
        let directory = temporary_directory();
        let path = directory.join("authority.db");
        let mut store = OperationalStore::open(&path, &observation(), &mut TestKey([10; 32]))
            .expect("encrypted store");
        let version: i64 = store
            .connection
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .expect("schema version");
        assert_eq!(version, SCHEMA_VERSION);
        assert!(
            store
                .connection
                .execute(
                    "INSERT INTO grant_revisions(
                        grant_id, revision, record_sha256, record_json
                     ) VALUES ('orphan', 1, ?1, X'00')",
                    ["1".repeat(64)],
                )
                .is_err()
        );
        store
            .connection
            .execute_batch(
                "CREATE TRIGGER reject_generation
                 BEFORE UPDATE ON store_metadata
                 BEGIN
                    SELECT RAISE(ABORT, 'synthetic rollback');
                 END;",
            )
            .expect("fault trigger");
        assert_eq!(
            store
                .persist_authority(&GrantIssuer::new(), &AuthorityTransactionCoordinator::new(),)
                .expect_err("publication must roll back"),
            OperationalStoreError::PersistenceFailure
        );
        assert!(store.poisoned);
        let generation: i64 = store
            .connection
            .query_row(
                "SELECT generation FROM store_metadata WHERE singleton = 1",
                [],
                |row| row.get(0),
            )
            .expect("generation remains readable");
        let checkpoint_count: i64 = store
            .connection
            .query_row("SELECT COUNT(*) FROM checkpoints", [], |row| row.get(0))
            .expect("checkpoint count");
        assert_eq!(generation, 0);
        assert_eq!(checkpoint_count, 1);
        drop(store);
        fs::remove_dir_all(directory).expect("cleanup");
    }

    #[test]
    fn version_two_schema_is_normalized_closed_and_relational() {
        let directory = temporary_directory();
        let path = directory.join("authority.db");
        let store = OperationalStore::open(&path, &observation(), &mut TestKey([14; 32]))
            .expect("version two store");
        let tables: Vec<String> = store
            .connection
            .prepare(
                "SELECT name FROM sqlite_schema
                 WHERE type = 'table' AND name NOT LIKE 'sqlite_%'
                 ORDER BY name",
            )
            .and_then(|mut statement| {
                statement
                    .query_map([], |row| row.get(0))?
                    .collect::<Result<Vec<_>, _>>()
            })
            .expect("table closure");
        assert_eq!(
            tables,
            [
                "actions",
                "checkpoints",
                "decisions",
                "evidence",
                "files",
                "grant_heads",
                "grant_identities",
                "grant_revisions",
                "objectives",
                "plans",
                "receipts",
                "retention",
                "schema_history",
                "sessions",
                "store_metadata",
                "tasks",
                "transaction_heads",
                "transaction_identities",
                "transaction_revisions",
            ]
        );

        let digest = "a".repeat(64);
        let record = b"{}".as_slice();
        store
            .connection
            .execute(
                "INSERT INTO sessions VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    "session-1",
                    "profile-1",
                    "active",
                    1_i64,
                    1_i64,
                    &digest,
                    record
                ],
            )
            .expect("session");
        store
            .connection
            .execute(
                "INSERT INTO objectives VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params!["objective-1", "session-1", 1_i64, "active", &digest, record],
            )
            .expect("objective");
        store
            .connection
            .execute(
                "INSERT INTO plans VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params!["plan-1", "objective-1", 1_i64, "active", &digest, record],
            )
            .expect("plan");
        store
            .connection
            .execute(
                "INSERT INTO tasks VALUES (?1, ?2, NULL, ?3, ?4, ?5, ?6)",
                params!["task-1", "plan-1", 1_i64, "active", &digest, record],
            )
            .expect("root task");
        store
            .connection
            .execute(
                "INSERT INTO tasks VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    "task-2", "plan-1", "task-1", 2_i64, "pending", &digest, record
                ],
            )
            .expect("child task");
        store
            .connection
            .execute(
                "INSERT INTO actions VALUES (?1, ?2, NULL, ?3, ?4, ?5, ?6, ?7)",
                params![
                    "action-1",
                    "task-1",
                    1_i64,
                    "workspace_read",
                    "succeeded",
                    &digest,
                    record
                ],
            )
            .expect("action");
        store
            .connection
            .execute(
                "INSERT INTO evidence VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    "evidence-1",
                    "task-1",
                    "action-1",
                    1_i64,
                    "tool_result",
                    "observed",
                    &digest,
                    record
                ],
            )
            .expect("evidence");
        store
            .connection
            .execute(
                "INSERT INTO decisions VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params!["decision-1", "task-1", 1_i64, "accepted", &digest, record],
            )
            .expect("decision");
        store
            .connection
            .execute(
                "INSERT INTO files VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    "file-1",
                    "session-1",
                    "workspace-1",
                    &digest,
                    &digest,
                    &digest,
                    "observed",
                    &digest,
                    record
                ],
            )
            .expect("file");
        store
            .connection
            .execute(
                "INSERT INTO retention VALUES (?1, ?2, ?3, ?4, ?5, NULL, ?6, ?7)",
                params![
                    "retention-1",
                    "sessions",
                    "session-1",
                    "private",
                    "retained",
                    0_i64,
                    &digest
                ],
            )
            .expect("retention");

        for table in [
            "sessions",
            "objectives",
            "plans",
            "tasks",
            "actions",
            "evidence",
            "decisions",
            "files",
            "retention",
        ] {
            let count: i64 = store
                .connection
                .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                    row.get(0)
                })
                .expect("domain row count");
            assert!(count > 0, "{table} must retain a normalized row");
        }
        assert!(
            store
                .connection
                .execute(
                    "INSERT INTO objectives VALUES ('orphan', 'missing', 1, 'active', ?1, X'00')",
                    [&digest],
                )
                .is_err()
        );
        assert!(
            store
                .connection
                .execute(
                    "INSERT INTO evidence VALUES ('cross-task', 'task-2', 'action-1', 2, 'tool_result', 'observed', ?1, X'00')",
                    [&digest],
                )
                .is_err()
        );
        assert!(
            store
                .connection
                .execute(
                    "INSERT INTO retention VALUES ('bad-retention', 'unknown', 'x', 'private', 'retained', NULL, 0, ?1)",
                    [&digest],
                )
                .is_err()
        );
        assert!(
            store
                .connection
                .execute(
                    "INSERT INTO decisions VALUES ('duplicate-ordinal', 'task-1', 1, 'accepted', ?1, X'00')",
                    [&digest],
                )
                .is_err()
        );
        drop(store);
        fs::remove_dir_all(directory).expect("cleanup");
    }

    #[test]
    fn version_one_upgrades_to_two_with_exact_history() {
        let directory = temporary_directory();
        let path = directory.join("authority.db");
        create_version_one_store(&path, &[15; 32]);
        let store = OperationalStore::open(&path, &observation(), &mut TestKey([15; 32]))
            .expect("version one upgrades");
        let version: i64 = store
            .connection
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .expect("schema version");
        assert_eq!(version, SCHEMA_VERSION);
        let history: Vec<(i64, String)> = store
            .connection
            .prepare("SELECT version, migration_sha256 FROM schema_history ORDER BY version")
            .and_then(|mut statement| {
                statement
                    .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
                    .collect::<Result<Vec<_>, _>>()
            })
            .expect("migration history");
        assert_eq!(
            history,
            [
                (1, sha256_hex(MIGRATION_1_SCHEMA_SQL.as_bytes())),
                (2, sha256_hex(MIGRATION_2_SCHEMA_SQL.as_bytes())),
            ]
        );
        drop(store);
        fs::remove_dir_all(directory).expect("cleanup");
    }

    #[test]
    fn failed_version_two_migration_rolls_back_without_partial_schema() {
        let directory = temporary_directory();
        let path = directory.join("authority.db");
        create_version_one_store(&path, &[16; 32]);
        let connection = open_connection(&path, &[16; 32]).expect("migration fixture");
        connection
            .execute_batch("CREATE TABLE sessions(incompatible INTEGER) STRICT;")
            .expect("incompatible table fixture");
        drop(connection);
        assert_eq!(
            OperationalStore::open(&path, &observation(), &mut TestKey([16; 32]))
                .expect_err("conflicting migration must fail"),
            OperationalStoreError::MigrationFailed
        );
        let connection = open_connection(&path, &[16; 32]).expect("inspect rolled-back store");
        let version: i64 = connection
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .expect("rolled-back version");
        let history_count: i64 = connection
            .query_row("SELECT COUNT(*) FROM schema_history", [], |row| row.get(0))
            .expect("rolled-back history");
        let objectives: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM sqlite_schema WHERE type = 'table' AND name = 'objectives'",
                [],
                |row| row.get(0),
            )
            .expect("rolled-back table closure");
        assert_eq!((version, history_count, objectives), (1, 1, 0));
        drop(connection);
        fs::remove_dir_all(directory).expect("cleanup");
    }

    #[test]
    fn future_schema_and_page_corruption_are_refused() {
        let directory = temporary_directory();
        let future_path = directory.join("future.db");
        let future = OperationalStore::open(&future_path, &observation(), &mut TestKey([11; 32]))
            .expect("encrypted store");
        future
            .connection
            .pragma_update(None, "user_version", SCHEMA_VERSION + 1)
            .expect("future version fixture");
        drop(future);
        assert_eq!(
            OperationalStore::open(&future_path, &observation(), &mut TestKey([11; 32]),)
                .expect_err("future schema must fail"),
            OperationalStoreError::MigrationFailed
        );

        let tampered_path = directory.join("tampered-migration.db");
        let tampered =
            OperationalStore::open(&tampered_path, &observation(), &mut TestKey([13; 32]))
                .expect("migration history fixture");
        tampered
            .connection
            .execute(
                "UPDATE schema_history SET migration_sha256 = ?1 WHERE version = 1",
                ["f".repeat(64)],
            )
            .expect("tamper migration history");
        drop(tampered);
        assert_eq!(
            OperationalStore::open(&tampered_path, &observation(), &mut TestKey([13; 32]),)
                .expect_err("tampered migration history must fail"),
            OperationalStoreError::MigrationFailed
        );

        let corrupt_path = directory.join("corrupt.db");
        drop(
            OperationalStore::open(&corrupt_path, &observation(), &mut TestKey([12; 32]))
                .expect("corruption fixture"),
        );
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&corrupt_path)
            .expect("fixture file");
        file.seek(SeekFrom::Start(128)).expect("seek page");
        let mut byte = [0_u8; 1];
        file.read_exact(&mut byte).expect("read page byte");
        byte[0] ^= 0xff;
        file.seek(SeekFrom::Start(128)).expect("seek page again");
        file.write_all(&byte).expect("corrupt page byte");
        file.sync_all().expect("sync corruption");
        drop(file);
        assert!(
            OperationalStore::open(&corrupt_path, &observation(), &mut TestKey([12; 32]),).is_err()
        );
        fs::remove_dir_all(directory).expect("cleanup");
    }
}
