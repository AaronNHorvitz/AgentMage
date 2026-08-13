//! SQLCipher-backed authority state and fail-closed restart recovery.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::fs::{self, File, OpenOptions};
use std::io::Read as _;
use std::io::Write as _;
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use agentmage_kernel_contracts::{
    AuthorityTransactionId, AuthorityTransactionRecord, AuthorityTransactionState, CapabilityGrant,
    GrantId, GrantNonce, Receipt, StrictLocalStorageObservation, from_json, to_canonical_json,
};
use rusqlite::{
    Connection, OpenFlags, OptionalExtension, Transaction, TransactionBehavior, params,
};
use serde::Serialize;
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

const SCHEMA_VERSION: i64 = 3;
const ZERO_SHA256: &str = "0000000000000000000000000000000000000000000000000000000000000000";
const KEY_BYTES: usize = 32;
const MAX_DERIVED_EXPORT_RECORDS: usize = 100_000;
const MAX_DERIVED_EXPORT_BYTES: usize = 64 * 1024 * 1024;
static NEXT_EXPORT_TEMPORARY: AtomicU64 = AtomicU64::new(1);
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
const MIGRATION_3_SCHEMA_SQL: &str =
    include_str!("../migrations/operational-store/0003-retention-lifecycle.sql");

/// Closed record families governed by the canonical retention engine.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RetentionRecordFamily {
    /// Resumable sessions.
    Sessions,
    /// Session objectives.
    Objectives,
    /// Revisioned plans.
    Plans,
    /// Plan tasks.
    Tasks,
    /// Proposed or executed actions.
    Actions,
    /// Evidence records.
    Evidence,
    /// Recorded decisions.
    Decisions,
    /// Capability grants.
    Grants,
    /// Terminal receipts.
    Receipts,
    /// Atomic checkpoints.
    Checkpoints,
    /// Content-free file observations.
    Files,
}

impl RetentionRecordFamily {
    const fn code(self) -> &'static str {
        match self {
            Self::Sessions => "sessions",
            Self::Objectives => "objectives",
            Self::Plans => "plans",
            Self::Tasks => "tasks",
            Self::Actions => "actions",
            Self::Evidence => "evidence",
            Self::Decisions => "decisions",
            Self::Grants => "grants",
            Self::Receipts => "receipts",
            Self::Checkpoints => "checkpoints",
            Self::Files => "files",
        }
    }
}

/// Closed data-sensitivity values retained with lifecycle metadata.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RetentionSensitivity {
    /// Public data.
    Public,
    /// Internal operational data.
    Internal,
    /// Private user data.
    Private,
    /// Restricted data admitted by a separately reviewed policy.
    Restricted,
}

impl RetentionSensitivity {
    const fn code(self) -> &'static str {
        match self {
            Self::Public => "public",
            Self::Internal => "internal",
            Self::Private => "private",
            Self::Restricted => "restricted",
        }
    }
}

/// Retention states that may be assigned before a hold or expiration.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RetentionDisposition {
    /// Memory-only data; retained only as content-free lifecycle metadata.
    Ephemeral,
    /// Data retained for a bounded session period.
    Session,
    /// Data explicitly retained until its bounded expiration.
    Retained,
}

impl RetentionDisposition {
    const fn code(self) -> &'static str {
        match self {
            Self::Ephemeral => "ephemeral",
            Self::Session => "session",
            Self::Retained => "retained",
        }
    }
}

/// A user or legal hold that pauses expiration without changing content.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RetentionHoldKind {
    /// A user-selected preservation hold.
    User,
    /// A legally directed preservation hold.
    Legal,
}

impl RetentionHoldKind {
    const fn code(self) -> &'static str {
        match self {
            Self::User => "user",
            Self::Legal => "legal",
        }
    }

    const fn event_code(self) -> &'static str {
        match self {
            Self::User => "user_hold_applied",
            Self::Legal => "legal_hold_applied",
        }
    }
}

/// Validated initial lifecycle assignment for one canonical record.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RetentionAssignment {
    retention_id: String,
    record_family: RetentionRecordFamily,
    record_id: String,
    sensitivity: RetentionSensitivity,
    disposition: RetentionDisposition,
    expires_at_epoch_ms: Option<u64>,
    policy_sha256: [u8; 32],
}

impl RetentionAssignment {
    /// Creates one bounded assignment. Ephemeral records cannot receive expiration.
    pub fn new(
        retention_id: impl Into<String>,
        record_family: RetentionRecordFamily,
        record_id: impl Into<String>,
        sensitivity: RetentionSensitivity,
        disposition: RetentionDisposition,
        expires_at_epoch_ms: Option<u64>,
        policy_sha256: [u8; 32],
    ) -> Result<Self, OperationalStoreError> {
        let retention_id = retention_id.into();
        let record_id = record_id.into();
        if !valid_lifecycle_identifier(&retention_id)
            || !valid_lifecycle_identifier(&record_id)
            || (disposition == RetentionDisposition::Ephemeral && expires_at_epoch_ms.is_some())
        {
            return Err(OperationalStoreError::LifecycleRejected);
        }
        Ok(Self {
            retention_id,
            record_family,
            record_id,
            sensitivity,
            disposition,
            expires_at_epoch_ms,
            policy_sha256,
        })
    }
}

/// Content-free result of one committed retention transition.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RetentionTransitionReceipt {
    /// Opaque lifecycle identity, never record content.
    pub retention_id: String,
    /// Committed lifecycle revision.
    pub revision: u64,
    /// Closed transition name.
    pub event: &'static str,
    /// Hash-chain identity of the committed event.
    pub event_sha256: String,
}

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

/// Fixed-scope platform key lifecycle used only for complete cryptographic erasure.
pub trait OperationalStoreKeyLifecycle {
    /// Destroys the exact scoped key and verifies that a subsequent lookup fails.
    fn destroy_key_and_verify_absent(&mut self) -> Result<(), OperationalStoreKeyError>;
}

/// Content-free proof that one separately keyed encrypted backup was verified.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EncryptedBackupReceipt {
    /// Store schema copied into the backup.
    pub schema_version: u32,
    /// Canonical generation copied into the backup.
    pub generation: u64,
    /// SHA-256 of the encrypted backup file, not plaintext records.
    pub encrypted_file_sha256: String,
}

/// Content-free proof that a backup produced one verified fresh restore candidate.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RestoreCandidateReceipt {
    /// Restored schema version.
    pub schema_version: u32,
    /// Restored canonical generation.
    pub generation: u64,
    /// SHA-256 of the separately encrypted candidate file.
    pub encrypted_file_sha256: String,
}

/// Result of destroying one whole-store key scope.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CryptographicErasureReceipt {
    /// The platform adapter destroyed and then failed to retrieve the scoped key.
    pub key_destroyed_and_absent: bool,
    /// Whether the encrypted database and known SQLite sidecars were removed.
    pub encrypted_files_removed: bool,
    /// Always false: SSD and copy-on-write media do not support this promise here.
    pub physical_overwrite_claim: bool,
}

/// Content-free receipt for one explicit derived JSON Lines export.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DerivedJsonLinesExportReceipt {
    /// Canonical generation observed before export construction.
    pub canonical_generation: u64,
    /// Number of derived metadata rows after the one-line header.
    pub record_count: usize,
    /// SHA-256 of the complete JSON Lines bytes.
    pub export_sha256: String,
}

#[derive(Serialize)]
struct DerivedExportHeader<'a> {
    record_type: &'static str,
    schema_version: u32,
    canonical_schema_version: u32,
    canonical_generation: u64,
    canonical_state_sha256: &'a str,
    record_count: usize,
    content_mode: &'static str,
    executable: bool,
    startup_authority: bool,
}

#[derive(Serialize)]
struct DerivedExportRow {
    record_type: &'static str,
    schema_version: u32,
    family: &'static str,
    record_identity_sha256: String,
    revision: u64,
    retained_sha256: String,
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
    /// A retention assignment or state transition was invalid or stale.
    LifecycleRejected,
    /// A verified backup could not be restored into a fresh candidate.
    RestoreFailure,
    /// The scoped encryption key could not be destroyed and verified absent.
    KeyErasureFailure,
    /// A derived export request was unsafe, oversized, or could not publish atomically.
    ExportRejected,
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
            Self::LifecycleRejected => "operational_store.lifecycle.rejected",
            Self::RestoreFailure => "operational_store.restore.failed",
            Self::KeyErasureFailure => "operational_store.key_erasure.failed",
            Self::ExportRejected => "operational_store.export.rejected",
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
    path: PathBuf,
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
    ) -> Result<EncryptedBackupReceipt, OperationalStoreError> {
        if self.poisoned {
            return Err(OperationalStoreError::Poisoned);
        }
        if evaluate_storage(observation) != StrictLocalStorageDecision::Eligible {
            return Err(OperationalStoreError::StorageRejected);
        }
        let mut destination_created = false;
        let result = provider
            .with_key(|key| {
                prepare_new_store_file(destination)?;
                destination_created = true;
                let mut destination_connection = open_connection(destination, key)?;
                claim_exclusive_writer(&destination_connection)?;
                verify_runtime_configuration(&destination_connection)?;
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
                let mut candidate = OperationalStore {
                    connection: destination_connection,
                    path: destination.to_path_buf(),
                    generation: 0,
                    poisoned: false,
                };
                let _ = candidate.load_authority()?;
                if candidate.generation != self.generation {
                    return Err(OperationalStoreError::IntegrityFailure);
                }
                drop(candidate);
                Ok(EncryptedBackupReceipt {
                    schema_version: u32::try_from(SCHEMA_VERSION)
                        .map_err(|_| OperationalStoreError::IntegrityFailure)?,
                    generation: self.generation,
                    encrypted_file_sha256: sha256_file(destination)?,
                })
            })
            .map_err(|_| OperationalStoreError::KeyUnavailable)?;
        if result.is_err() && destination_created {
            remove_sqlite_artifacts(destination);
        }
        result
    }

    /// Restores a verified encrypted backup into a new candidate without replacing live state.
    pub fn restore_to_fresh_candidate<
        SP: OperationalStoreKeyProvider,
        DP: OperationalStoreKeyProvider,
    >(
        backup: &Path,
        backup_observation: &StrictLocalStorageObservation,
        backup_provider: &mut SP,
        destination: &Path,
        destination_observation: &StrictLocalStorageObservation,
        destination_provider: &mut DP,
    ) -> Result<RestoreCandidateReceipt, OperationalStoreError> {
        if backup == destination {
            return Err(OperationalStoreError::RestoreFailure);
        }
        if evaluate_storage(backup_observation) != StrictLocalStorageDecision::Eligible
            || evaluate_storage(destination_observation) != StrictLocalStorageDecision::Eligible
        {
            return Err(OperationalStoreError::StorageRejected);
        }
        verify_store_file(backup).map_err(|_| OperationalStoreError::RestoreFailure)?;
        let mut destination_created = false;
        let result = backup_provider
            .with_key(|backup_key| {
                let source = open_current_keyed(backup, backup_key)
                    .map_err(|_| OperationalStoreError::RestoreFailure)?;
                destination_provider
                    .with_key(|destination_key| {
                        prepare_new_store_file(destination)
                            .map_err(|_| OperationalStoreError::RestoreFailure)?;
                        destination_created = true;
                        let mut destination_connection =
                            open_connection(destination, destination_key)
                                .map_err(|_| OperationalStoreError::RestoreFailure)?;
                        claim_exclusive_writer(&destination_connection)
                            .map_err(|_| OperationalStoreError::RestoreFailure)?;
                        verify_runtime_configuration(&destination_connection)
                            .map_err(|_| OperationalStoreError::RestoreFailure)?;
                        {
                            let copy = rusqlite::backup::Backup::new(
                                &source.connection,
                                &mut destination_connection,
                            )
                            .map_err(|_| OperationalStoreError::RestoreFailure)?;
                            copy.run_to_completion(64, Duration::from_millis(1), None)
                                .map_err(|_| OperationalStoreError::RestoreFailure)?;
                        }
                        let mut candidate = OperationalStore {
                            connection: destination_connection,
                            path: destination.to_path_buf(),
                            generation: 0,
                            poisoned: false,
                        };
                        let _ = candidate
                            .load_authority()
                            .map_err(|_| OperationalStoreError::RestoreFailure)?;
                        if candidate.generation != source.generation {
                            return Err(OperationalStoreError::RestoreFailure);
                        }
                        let generation = candidate.generation;
                        drop(candidate);
                        Ok(RestoreCandidateReceipt {
                            schema_version: u32::try_from(SCHEMA_VERSION)
                                .map_err(|_| OperationalStoreError::RestoreFailure)?,
                            generation,
                            encrypted_file_sha256: sha256_file(destination)
                                .map_err(|_| OperationalStoreError::RestoreFailure)?,
                        })
                    })
                    .map_err(|_| OperationalStoreError::KeyUnavailable)?
            })
            .map_err(|_| OperationalStoreError::KeyUnavailable)?;
        if result.is_err() && destination_created {
            remove_sqlite_artifacts(destination);
        }
        result
    }

    /// Consumes the live store, destroys its whole-store key, and removes known ciphertext files.
    pub fn cryptographic_erase<L: OperationalStoreKeyLifecycle>(
        self,
        lifecycle: &mut L,
    ) -> Result<CryptographicErasureReceipt, OperationalStoreError> {
        let path = self.path.clone();
        drop(self);
        lifecycle
            .destroy_key_and_verify_absent()
            .map_err(|_| OperationalStoreError::KeyErasureFailure)?;
        remove_sqlite_artifacts(&path);
        Ok(CryptographicErasureReceipt {
            key_destroyed_and_absent: true,
            encrypted_files_removed: !sqlite_artifacts_exist(&path),
            physical_overwrite_claim: false,
        })
    }

    /// Writes a versioned content-free JSON Lines derivative with no import authority.
    pub fn export_json_lines(
        &self,
        destination: &Path,
        observation: &StrictLocalStorageObservation,
    ) -> Result<DerivedJsonLinesExportReceipt, OperationalStoreError> {
        if self.poisoned {
            return Err(OperationalStoreError::Poisoned);
        }
        if evaluate_storage(observation) != StrictLocalStorageDecision::Eligible {
            return Err(OperationalStoreError::StorageRejected);
        }
        verify_integrity(&self.connection)?;
        let canonical_state_sha256: String = self
            .connection
            .query_row(
                "SELECT state_sha256 FROM store_metadata WHERE singleton = 1",
                [],
                |row| row.get(0),
            )
            .map_err(|_| OperationalStoreError::IntegrityFailure)?;
        let rows = derived_export_rows(&self.connection)?;
        if rows.len() > MAX_DERIVED_EXPORT_RECORDS {
            return Err(OperationalStoreError::ExportRejected);
        }
        let header = DerivedExportHeader {
            record_type: "agentmage.derived_export.header",
            schema_version: 1,
            canonical_schema_version: u32::try_from(SCHEMA_VERSION)
                .map_err(|_| OperationalStoreError::ExportRejected)?,
            canonical_generation: self.generation,
            canonical_state_sha256: &canonical_state_sha256,
            record_count: rows.len(),
            content_mode: "identity-and-retained-hashes-only",
            executable: false,
            startup_authority: false,
        };
        let mut bytes = json_line(&header)?;
        for row in &rows {
            let line = json_line(row)?;
            if bytes.len().saturating_add(line.len()) > MAX_DERIVED_EXPORT_BYTES {
                return Err(OperationalStoreError::ExportRejected);
            }
            bytes.extend_from_slice(&line);
        }
        let export_sha256 = sha256_hex(&bytes);
        write_derived_export(destination, &bytes)?;
        Ok(DerivedJsonLinesExportReceipt {
            canonical_generation: self.generation,
            record_count: rows.len(),
            export_sha256,
        })
    }

    /// Registers one canonical record with a bounded retention policy.
    pub fn assign_retention(
        &mut self,
        assignment: &RetentionAssignment,
        occurred_at_epoch_ms: u64,
    ) -> Result<RetentionTransitionReceipt, OperationalStoreError> {
        self.ensure_lifecycle_usable()?;
        if assignment.disposition == RetentionDisposition::Ephemeral {
            return Err(OperationalStoreError::LifecycleRejected);
        }
        let occurred_at = lifecycle_time(occurred_at_epoch_ms)?;
        let expires_at = assignment
            .expires_at_epoch_ms
            .map(lifecycle_time)
            .transpose()?;
        if expires_at.is_some_and(|expires_at| expires_at < occurred_at) {
            return Err(OperationalStoreError::LifecycleRejected);
        }
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| OperationalStoreError::LifecycleRejected)?;
        if !retention_record_exists(
            &transaction,
            assignment.record_family,
            &assignment.record_id,
        )? {
            return Err(OperationalStoreError::LifecycleRejected);
        }
        let policy_sha256 = hex_digest(&assignment.policy_sha256);
        transaction
            .execute(
                "INSERT INTO retention(
                    retention_id, record_family, record_id, sensitivity, disposition,
                    expires_at_epoch_ms, legal_hold, policy_sha256, hold_kind,
                    prior_disposition, revision, updated_at_epoch_ms, erased_at_epoch_ms
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, 0, ?7, 'none', NULL, 1, ?8, NULL)",
                params![
                    &assignment.retention_id,
                    assignment.record_family.code(),
                    &assignment.record_id,
                    assignment.sensitivity.code(),
                    assignment.disposition.code(),
                    expires_at,
                    &policy_sha256,
                    occurred_at,
                ],
            )
            .map_err(|_| OperationalStoreError::LifecycleRejected)?;
        let receipt = append_retention_event(
            &transaction,
            &assignment.retention_id,
            1,
            "assigned",
            occurred_at_epoch_ms,
            ZERO_SHA256,
        )?;
        transaction
            .commit()
            .map_err(|_| OperationalStoreError::LifecycleRejected)?;
        Ok(receipt)
    }

    /// Applies one exact user or legal hold using optimistic revision control.
    pub fn apply_retention_hold(
        &mut self,
        retention_id: &str,
        expected_revision: u64,
        hold: RetentionHoldKind,
        occurred_at_epoch_ms: u64,
    ) -> Result<RetentionTransitionReceipt, OperationalStoreError> {
        self.ensure_lifecycle_usable()?;
        validate_lifecycle_transition_input(retention_id, expected_revision)?;
        let revision = expected_revision
            .checked_add(1)
            .ok_or(OperationalStoreError::LifecycleRejected)?;
        let expected_revision_i64 = lifecycle_revision(expected_revision)?;
        let revision_i64 = lifecycle_revision(revision)?;
        let occurred_at = lifecycle_time(occurred_at_epoch_ms)?;
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| OperationalStoreError::LifecycleRejected)?;
        let previous = retention_event_head(&transaction, retention_id, expected_revision)?;
        let changed = transaction
            .execute(
                "UPDATE retention
                 SET disposition = 'held', legal_hold = 1, hold_kind = ?1,
                     prior_disposition = disposition, revision = ?2,
                     updated_at_epoch_ms = ?3
                 WHERE retention_id = ?4 AND revision = ?5 AND hold_kind = 'none'
                   AND disposition IN ('session', 'retained')
                   AND updated_at_epoch_ms <= ?3",
                params![
                    hold.code(),
                    revision_i64,
                    occurred_at,
                    retention_id,
                    expected_revision_i64,
                ],
            )
            .map_err(|_| OperationalStoreError::LifecycleRejected)?;
        if changed != 1 {
            return Err(OperationalStoreError::LifecycleRejected);
        }
        let receipt = append_retention_event(
            &transaction,
            retention_id,
            revision,
            hold.event_code(),
            occurred_at_epoch_ms,
            &previous,
        )?;
        transaction
            .commit()
            .map_err(|_| OperationalStoreError::LifecycleRejected)?;
        Ok(receipt)
    }

    /// Releases the exact current hold and restores its pre-hold disposition.
    pub fn release_retention_hold(
        &mut self,
        retention_id: &str,
        expected_revision: u64,
        hold: RetentionHoldKind,
        occurred_at_epoch_ms: u64,
    ) -> Result<RetentionTransitionReceipt, OperationalStoreError> {
        self.ensure_lifecycle_usable()?;
        validate_lifecycle_transition_input(retention_id, expected_revision)?;
        let revision = expected_revision
            .checked_add(1)
            .ok_or(OperationalStoreError::LifecycleRejected)?;
        let expected_revision_i64 = lifecycle_revision(expected_revision)?;
        let revision_i64 = lifecycle_revision(revision)?;
        let occurred_at = lifecycle_time(occurred_at_epoch_ms)?;
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| OperationalStoreError::LifecycleRejected)?;
        let previous = retention_event_head(&transaction, retention_id, expected_revision)?;
        let changed = transaction
            .execute(
                "UPDATE retention
                 SET disposition = prior_disposition, legal_hold = 0, hold_kind = 'none',
                     prior_disposition = NULL, revision = ?1, updated_at_epoch_ms = ?2
                 WHERE retention_id = ?3 AND revision = ?4 AND hold_kind = ?5
                   AND disposition = 'held' AND prior_disposition IN ('session', 'retained')
                   AND updated_at_epoch_ms <= ?2",
                params![
                    revision_i64,
                    occurred_at,
                    retention_id,
                    expected_revision_i64,
                    hold.code(),
                ],
            )
            .map_err(|_| OperationalStoreError::LifecycleRejected)?;
        if changed != 1 {
            return Err(OperationalStoreError::LifecycleRejected);
        }
        let receipt = append_retention_event(
            &transaction,
            retention_id,
            revision,
            "hold_released",
            occurred_at_epoch_ms,
            &previous,
        )?;
        transaction
            .commit()
            .map_err(|_| OperationalStoreError::LifecycleRejected)?;
        Ok(receipt)
    }

    /// Expires every due, unheld record in one transaction and returns ordered receipts.
    pub fn expire_due(
        &mut self,
        occurred_at_epoch_ms: u64,
    ) -> Result<Vec<RetentionTransitionReceipt>, OperationalStoreError> {
        self.ensure_lifecycle_usable()?;
        let occurred_at = lifecycle_time(occurred_at_epoch_ms)?;
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| OperationalStoreError::LifecycleRejected)?;
        let due: Vec<(String, i64)> = transaction
            .prepare(
                "SELECT retention_id, revision FROM retention
                 WHERE hold_kind = 'none' AND legal_hold = 0
                   AND disposition IN ('session', 'retained')
                   AND expires_at_epoch_ms IS NOT NULL AND expires_at_epoch_ms <= ?1
                   AND updated_at_epoch_ms <= ?1
                 ORDER BY retention_id",
            )
            .and_then(|mut statement| {
                statement
                    .query_map([occurred_at], |row| Ok((row.get(0)?, row.get(1)?)))?
                    .collect()
            })
            .map_err(|_| OperationalStoreError::LifecycleRejected)?;
        let mut receipts = Vec::with_capacity(due.len());
        for (retention_id, expected_revision_i64) in due {
            let expected_revision = u64::try_from(expected_revision_i64)
                .map_err(|_| OperationalStoreError::LifecycleRejected)?;
            let revision = expected_revision
                .checked_add(1)
                .ok_or(OperationalStoreError::LifecycleRejected)?;
            let previous = retention_event_head(&transaction, &retention_id, expected_revision)?;
            let changed = transaction
                .execute(
                    "UPDATE retention
                     SET disposition = 'expired', revision = ?1, updated_at_epoch_ms = ?2
                     WHERE retention_id = ?3 AND revision = ?4 AND hold_kind = 'none'
                       AND legal_hold = 0 AND disposition IN ('session', 'retained')
                       AND expires_at_epoch_ms IS NOT NULL AND expires_at_epoch_ms <= ?2",
                    params![
                        lifecycle_revision(revision)?,
                        occurred_at,
                        &retention_id,
                        expected_revision_i64,
                    ],
                )
                .map_err(|_| OperationalStoreError::LifecycleRejected)?;
            if changed != 1 {
                return Err(OperationalStoreError::LifecycleRejected);
            }
            receipts.push(append_retention_event(
                &transaction,
                &retention_id,
                revision,
                "expired",
                occurred_at_epoch_ms,
                &previous,
            )?);
        }
        transaction
            .commit()
            .map_err(|_| OperationalStoreError::LifecycleRejected)?;
        Ok(receipts)
    }

    fn ensure_lifecycle_usable(&self) -> Result<(), OperationalStoreError> {
        if self.poisoned {
            Err(OperationalStoreError::Poisoned)
        } else {
            Ok(())
        }
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
    ) -> Result<EncryptedBackupReceipt, DurableAuthorityError> {
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
    claim_exclusive_writer(&connection)?;
    verify_runtime_configuration(&connection)?;
    migrate(&connection)?;
    let mut store = OperationalStore {
        connection,
        path: path.to_path_buf(),
        generation: 0,
        poisoned: false,
    };
    let _ = store.load_authority()?;
    Ok(store)
}

fn open_current_keyed(path: &Path, key: &[u8]) -> Result<OperationalStore, OperationalStoreError> {
    let connection = open_connection(path, key)?;
    claim_exclusive_writer(&connection)?;
    verify_runtime_configuration(&connection)?;
    let version: i64 = connection
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .map_err(|_| OperationalStoreError::IntegrityFailure)?;
    if version != SCHEMA_VERSION {
        return Err(OperationalStoreError::IntegrityFailure);
    }
    verify_schema_history(&connection)?;
    let mut store = OperationalStore {
        connection,
        path: path.to_path_buf(),
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

fn verify_runtime_configuration(connection: &Connection) -> Result<(), OperationalStoreError> {
    let foreign_keys: i64 = connection
        .pragma_query_value(None, "foreign_keys", |row| row.get(0))
        .map_err(|_| OperationalStoreError::OpenFailed)?;
    let trusted_schema: i64 = connection
        .pragma_query_value(None, "trusted_schema", |row| row.get(0))
        .map_err(|_| OperationalStoreError::OpenFailed)?;
    let secure_delete: i64 = connection
        .pragma_query_value(None, "secure_delete", |row| row.get(0))
        .map_err(|_| OperationalStoreError::OpenFailed)?;
    let temp_store: i64 = connection
        .pragma_query_value(None, "temp_store", |row| row.get(0))
        .map_err(|_| OperationalStoreError::OpenFailed)?;
    let synchronous: i64 = connection
        .pragma_query_value(None, "synchronous", |row| row.get(0))
        .map_err(|_| OperationalStoreError::OpenFailed)?;
    let wal_autocheckpoint: i64 = connection
        .pragma_query_value(None, "wal_autocheckpoint", |row| row.get(0))
        .map_err(|_| OperationalStoreError::OpenFailed)?;
    let busy_timeout: i64 = connection
        .pragma_query_value(None, "busy_timeout", |row| row.get(0))
        .map_err(|_| OperationalStoreError::OpenFailed)?;
    let locking_mode: String = connection
        .pragma_query_value(None, "locking_mode", |row| row.get(0))
        .map_err(|_| OperationalStoreError::OpenFailed)?;
    let journal_mode: String = connection
        .pragma_query_value(None, "journal_mode", |row| row.get(0))
        .map_err(|_| OperationalStoreError::OpenFailed)?;
    if foreign_keys != 1
        || trusted_schema != 0
        || secure_delete != 1
        || temp_store != 2
        || synchronous != 2
        || wal_autocheckpoint != 1
        || busy_timeout != 0
        || !locking_mode.eq_ignore_ascii_case("exclusive")
        || !journal_mode.eq_ignore_ascii_case("wal")
    {
        return Err(OperationalStoreError::OpenFailed);
    }
    Ok(())
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
            .pragma_update(None, "user_version", 2)
            .map_err(|_| OperationalStoreError::MigrationFailed)?;
        transaction
            .commit()
            .map_err(|_| OperationalStoreError::MigrationFailed)?;
        version = 2;
    }
    if version == 2 {
        let transaction = connection
            .unchecked_transaction()
            .map_err(|_| OperationalStoreError::MigrationFailed)?;
        transaction
            .execute_batch(MIGRATION_3_SCHEMA_SQL)
            .map_err(|_| OperationalStoreError::MigrationFailed)?;
        let migrated_retention_ids = transaction
            .prepare("SELECT retention_id FROM retention ORDER BY retention_id")
            .and_then(|mut statement| {
                statement
                    .query_map([], |row| row.get::<_, String>(0))?
                    .collect::<Result<Vec<_>, _>>()
            })
            .map_err(|_| OperationalStoreError::MigrationFailed)?;
        for retention_id in migrated_retention_ids {
            append_retention_event(&transaction, &retention_id, 1, "assigned", 0, ZERO_SHA256)
                .map_err(|_| OperationalStoreError::MigrationFailed)?;
        }
        transaction
            .execute(
                "INSERT INTO schema_history(version, migration_sha256) VALUES (3, ?1)",
                [sha256_hex(MIGRATION_3_SCHEMA_SQL.as_bytes())],
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
            (3, sha256_hex(MIGRATION_3_SCHEMA_SQL.as_bytes())),
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
    verify_retention_lifecycle(connection)?;
    Ok(())
}

fn valid_lifecycle_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':' | b'/')
        })
}

fn validate_lifecycle_transition_input(
    retention_id: &str,
    expected_revision: u64,
) -> Result<(), OperationalStoreError> {
    if valid_lifecycle_identifier(retention_id) && expected_revision > 0 {
        Ok(())
    } else {
        Err(OperationalStoreError::LifecycleRejected)
    }
}

fn lifecycle_time(value: u64) -> Result<i64, OperationalStoreError> {
    i64::try_from(value).map_err(|_| OperationalStoreError::LifecycleRejected)
}

fn lifecycle_revision(value: u64) -> Result<i64, OperationalStoreError> {
    i64::try_from(value).map_err(|_| OperationalStoreError::LifecycleRejected)
}

fn retention_record_exists(
    transaction: &Transaction<'_>,
    family: RetentionRecordFamily,
    record_id: &str,
) -> Result<bool, OperationalStoreError> {
    let query = match family {
        RetentionRecordFamily::Sessions => "SELECT 1 FROM sessions WHERE session_id = ?1 LIMIT 1",
        RetentionRecordFamily::Objectives => {
            "SELECT 1 FROM objectives WHERE objective_id = ?1 LIMIT 1"
        }
        RetentionRecordFamily::Plans => "SELECT 1 FROM plans WHERE plan_id = ?1 LIMIT 1",
        RetentionRecordFamily::Tasks => "SELECT 1 FROM tasks WHERE task_id = ?1 LIMIT 1",
        RetentionRecordFamily::Actions => "SELECT 1 FROM actions WHERE action_id = ?1 LIMIT 1",
        RetentionRecordFamily::Evidence => "SELECT 1 FROM evidence WHERE evidence_id = ?1 LIMIT 1",
        RetentionRecordFamily::Decisions => {
            "SELECT 1 FROM decisions WHERE decision_id = ?1 LIMIT 1"
        }
        RetentionRecordFamily::Grants => {
            "SELECT 1 FROM grant_identities WHERE grant_id = ?1 LIMIT 1"
        }
        RetentionRecordFamily::Receipts => "SELECT 1 FROM receipts WHERE receipt_id = ?1 LIMIT 1",
        RetentionRecordFamily::Checkpoints => {
            "SELECT 1 FROM checkpoints WHERE CAST(generation AS TEXT) = ?1 LIMIT 1"
        }
        RetentionRecordFamily::Files => "SELECT 1 FROM files WHERE file_id = ?1 LIMIT 1",
    };
    transaction
        .query_row(query, [record_id], |_| Ok(()))
        .optional()
        .map(|value| value.is_some())
        .map_err(|_| OperationalStoreError::LifecycleRejected)
}

struct RetentionStateRow {
    record_family: String,
    record_id: String,
    sensitivity: String,
    disposition: String,
    expires_at_epoch_ms: Option<i64>,
    legal_hold: i64,
    policy_sha256: String,
    hold_kind: String,
    prior_disposition: Option<String>,
    revision: i64,
    updated_at_epoch_ms: i64,
    erased_at_epoch_ms: Option<i64>,
}

fn retention_state_sha256(
    connection: &Connection,
    retention_id: &str,
) -> Result<String, OperationalStoreError> {
    let state = connection
        .query_row(
            "SELECT record_family, record_id, sensitivity, disposition,
                    expires_at_epoch_ms, legal_hold, policy_sha256, hold_kind,
                    prior_disposition, revision, updated_at_epoch_ms, erased_at_epoch_ms
             FROM retention WHERE retention_id = ?1",
            [retention_id],
            |row| {
                Ok(RetentionStateRow {
                    record_family: row.get(0)?,
                    record_id: row.get(1)?,
                    sensitivity: row.get(2)?,
                    disposition: row.get(3)?,
                    expires_at_epoch_ms: row.get(4)?,
                    legal_hold: row.get(5)?,
                    policy_sha256: row.get(6)?,
                    hold_kind: row.get(7)?,
                    prior_disposition: row.get(8)?,
                    revision: row.get(9)?,
                    updated_at_epoch_ms: row.get(10)?,
                    erased_at_epoch_ms: row.get(11)?,
                })
            },
        )
        .map_err(|_| OperationalStoreError::LifecycleRejected)?;
    let mut digest = Sha256::new();
    for value in [
        state.record_family,
        state.record_id,
        state.sensitivity,
        state.disposition,
        state
            .expires_at_epoch_ms
            .map_or_else(|| "none".to_owned(), |value| value.to_string()),
        state.legal_hold.to_string(),
        state.policy_sha256,
        state.hold_kind,
        state.prior_disposition.unwrap_or_else(|| "none".to_owned()),
        state.revision.to_string(),
        state.updated_at_epoch_ms.to_string(),
        state
            .erased_at_epoch_ms
            .map_or_else(|| "none".to_owned(), |value| value.to_string()),
    ] {
        digest.update(value.as_bytes());
        digest.update([0]);
    }
    Ok(hex_digest(&digest.finalize()))
}

fn retention_event_head(
    transaction: &Transaction<'_>,
    retention_id: &str,
    expected_revision: u64,
) -> Result<String, OperationalStoreError> {
    transaction
        .query_row(
            "SELECT event_sha256 FROM retention_events
             WHERE retention_id = ?1 AND revision = ?2",
            params![retention_id, lifecycle_revision(expected_revision)?],
            |row| row.get(0),
        )
        .map_err(|_| OperationalStoreError::LifecycleRejected)
}

fn retention_event_sha256(
    retention_id: &str,
    revision: u64,
    event: &str,
    occurred_at_epoch_ms: u64,
    previous_event_sha256: &str,
    state_sha256: &str,
) -> String {
    let mut digest = Sha256::new();
    for value in [
        retention_id.to_owned(),
        revision.to_string(),
        event.to_owned(),
        occurred_at_epoch_ms.to_string(),
        previous_event_sha256.to_owned(),
        state_sha256.to_owned(),
    ] {
        digest.update(value.as_bytes());
        digest.update([0]);
    }
    hex_digest(&digest.finalize())
}

fn append_retention_event(
    transaction: &Transaction<'_>,
    retention_id: &str,
    revision: u64,
    event: &'static str,
    occurred_at_epoch_ms: u64,
    previous_event_sha256: &str,
) -> Result<RetentionTransitionReceipt, OperationalStoreError> {
    let state_sha256 = retention_state_sha256(transaction, retention_id)?;
    let event_sha256 = retention_event_sha256(
        retention_id,
        revision,
        event,
        occurred_at_epoch_ms,
        previous_event_sha256,
        &state_sha256,
    );
    transaction
        .execute(
            "INSERT INTO retention_events(
                retention_id, revision, event_kind, occurred_at_epoch_ms,
                previous_event_sha256, state_sha256, event_sha256
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                retention_id,
                lifecycle_revision(revision)?,
                event,
                lifecycle_time(occurred_at_epoch_ms)?,
                previous_event_sha256,
                &state_sha256,
                &event_sha256,
            ],
        )
        .map_err(|_| OperationalStoreError::LifecycleRejected)?;
    Ok(RetentionTransitionReceipt {
        retention_id: retention_id.to_owned(),
        revision,
        event,
        event_sha256,
    })
}

fn verify_retention_lifecycle(connection: &Connection) -> Result<(), OperationalStoreError> {
    let inconsistent: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM retention
             WHERE (hold_kind = 'none' AND (legal_hold != 0 OR prior_disposition IS NOT NULL OR disposition = 'held'))
                OR (hold_kind != 'none' AND (legal_hold != 1 OR prior_disposition NOT IN ('session', 'retained') OR disposition != 'held'))
                OR (disposition = 'expired' AND (hold_kind != 'none' OR legal_hold != 0))
                OR (disposition = 'deleted' AND erased_at_epoch_ms IS NULL)",
            [],
            |row| row.get(0),
        )
        .map_err(|_| OperationalStoreError::IntegrityFailure)?;
    if inconsistent != 0 {
        return Err(OperationalStoreError::IntegrityFailure);
    }

    let mut statement = connection
        .prepare(
            "SELECT retention_id, revision, event_kind, occurred_at_epoch_ms,
                    previous_event_sha256, state_sha256, event_sha256
             FROM retention_events ORDER BY retention_id, revision",
        )
        .map_err(|_| OperationalStoreError::IntegrityFailure)?;
    let events = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, i64>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, String>(6)?,
            ))
        })
        .map_err(|_| OperationalStoreError::IntegrityFailure)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| OperationalStoreError::IntegrityFailure)?;
    let mut heads = BTreeMap::<String, (u64, String, String)>::new();
    for (retention_id, revision, event, occurred_at, previous, state, retained_hash) in events {
        let revision =
            u64::try_from(revision).map_err(|_| OperationalStoreError::IntegrityFailure)?;
        let occurred_at =
            u64::try_from(occurred_at).map_err(|_| OperationalStoreError::IntegrityFailure)?;
        let expected_previous = heads
            .get(&retention_id)
            .map_or(ZERO_SHA256, |head| head.1.as_str());
        let expected_revision = heads.get(&retention_id).map_or(1, |head| head.0 + 1);
        let expected_hash = retention_event_sha256(
            &retention_id,
            revision,
            &event,
            occurred_at,
            &previous,
            &state,
        );
        if revision != expected_revision
            || previous != expected_previous
            || retained_hash != expected_hash
        {
            return Err(OperationalStoreError::IntegrityFailure);
        }
        heads.insert(retention_id, (revision, retained_hash, state));
    }
    let retention_rows = connection
        .prepare("SELECT retention_id, revision FROM retention ORDER BY retention_id")
        .and_then(|mut statement| {
            statement
                .query_map([], |row| {
                    Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
                })?
                .collect::<Result<Vec<_>, _>>()
        })
        .map_err(|_| OperationalStoreError::IntegrityFailure)?;
    let retention_count = retention_rows.len();
    for (retention_id, revision) in retention_rows {
        let revision =
            u64::try_from(revision).map_err(|_| OperationalStoreError::IntegrityFailure)?;
        let head = heads
            .get(&retention_id)
            .ok_or(OperationalStoreError::IntegrityFailure)?;
        let current_state = retention_state_sha256(connection, &retention_id)
            .map_err(|_| OperationalStoreError::IntegrityFailure)?;
        if head.0 != revision || head.2 != current_state {
            return Err(OperationalStoreError::IntegrityFailure);
        }
    }
    if heads.len() != retention_count {
        return Err(OperationalStoreError::IntegrityFailure);
    }
    Ok(())
}

struct DerivedExportQuery {
    family: &'static str,
    sql: &'static str,
}

const DERIVED_EXPORT_QUERIES: &[DerivedExportQuery] = &[
    DerivedExportQuery {
        family: "actions",
        sql: "SELECT action_id, 0, record_sha256 FROM actions",
    },
    DerivedExportQuery {
        family: "checkpoints",
        sql: "SELECT CAST(generation AS TEXT), generation, state_sha256 FROM checkpoints",
    },
    DerivedExportQuery {
        family: "decisions",
        sql: "SELECT decision_id, 0, record_sha256 FROM decisions",
    },
    DerivedExportQuery {
        family: "evidence",
        sql: "SELECT evidence_id, 0, record_sha256 FROM evidence",
    },
    DerivedExportQuery {
        family: "files",
        sql: "SELECT file_id, 0, record_sha256 FROM files",
    },
    DerivedExportQuery {
        family: "grants",
        sql: "SELECT grant_id, revision, record_sha256 FROM grant_revisions",
    },
    DerivedExportQuery {
        family: "objectives",
        sql: "SELECT objective_id, 0, record_sha256 FROM objectives",
    },
    DerivedExportQuery {
        family: "plans",
        sql: "SELECT plan_id, revision, record_sha256 FROM plans",
    },
    DerivedExportQuery {
        family: "receipts",
        sql: "SELECT receipt_id, sequence, receipt_sha256 FROM receipts",
    },
    DerivedExportQuery {
        family: "retention_events",
        sql: "SELECT retention_id, revision, event_sha256 FROM retention_events",
    },
    DerivedExportQuery {
        family: "sessions",
        sql: "SELECT session_id, 0, record_sha256 FROM sessions",
    },
    DerivedExportQuery {
        family: "tasks",
        sql: "SELECT task_id, 0, record_sha256 FROM tasks",
    },
    DerivedExportQuery {
        family: "transactions",
        sql: "SELECT transaction_id, revision, record_sha256 FROM transaction_revisions",
    },
];

fn derived_export_rows(
    connection: &Connection,
) -> Result<Vec<DerivedExportRow>, OperationalStoreError> {
    let mut rows = Vec::new();
    for query in DERIVED_EXPORT_QUERIES {
        let records = connection
            .prepare(query.sql)
            .and_then(|mut statement| {
                statement
                    .query_map([], |row| {
                        Ok((
                            row.get::<_, String>(0)?,
                            row.get::<_, i64>(1)?,
                            row.get::<_, String>(2)?,
                        ))
                    })?
                    .collect::<Result<Vec<_>, _>>()
            })
            .map_err(|_| OperationalStoreError::ExportRejected)?;
        for (identity, revision, retained_sha256) in records {
            if !valid_sha256_text(&retained_sha256) {
                return Err(OperationalStoreError::ExportRejected);
            }
            let revision =
                u64::try_from(revision).map_err(|_| OperationalStoreError::ExportRejected)?;
            let mut identity_digest = Sha256::new();
            identity_digest.update(query.family.as_bytes());
            identity_digest.update([0]);
            identity_digest.update(identity.as_bytes());
            rows.push(DerivedExportRow {
                record_type: "agentmage.derived_export.record",
                schema_version: 1,
                family: query.family,
                record_identity_sha256: hex_digest(&identity_digest.finalize()),
                revision,
                retained_sha256,
            });
            if rows.len() > MAX_DERIVED_EXPORT_RECORDS {
                return Err(OperationalStoreError::ExportRejected);
            }
        }
    }
    rows.sort_by(|left, right| {
        (left.family, &left.record_identity_sha256, left.revision).cmp(&(
            right.family,
            &right.record_identity_sha256,
            right.revision,
        ))
    });
    Ok(rows)
}

fn valid_sha256_text(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn json_line<T: Serialize>(value: &T) -> Result<Vec<u8>, OperationalStoreError> {
    let mut bytes = serde_json::to_vec(value).map_err(|_| OperationalStoreError::ExportRejected)?;
    bytes.push(b'\n');
    Ok(bytes)
}

fn write_derived_export(destination: &Path, bytes: &[u8]) -> Result<(), OperationalStoreError> {
    let parent = destination
        .parent()
        .ok_or(OperationalStoreError::ExportRejected)?;
    let metadata =
        fs::symlink_metadata(parent).map_err(|_| OperationalStoreError::ExportRejected)?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() || destination.exists() {
        return Err(OperationalStoreError::ExportRejected);
    }
    let mut temporary = None;
    let mut file = None;
    for _ in 0..16 {
        let sequence = NEXT_EXPORT_TEMPORARY.fetch_add(1, Ordering::Relaxed);
        let candidate = parent.join(format!(
            ".agentmage-derived-export-{}-{sequence}.tmp",
            std::process::id()
        ));
        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&candidate)
        {
            Ok(created) => {
                temporary = Some(candidate);
                file = Some(created);
                break;
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(_) => return Err(OperationalStoreError::ExportRejected),
        }
    }
    let temporary = temporary.ok_or(OperationalStoreError::ExportRejected)?;
    let mut file = file.ok_or(OperationalStoreError::ExportRejected)?;
    let mut published_identity = None;
    let result = (|| {
        file.write_all(bytes)
            .map_err(|_| OperationalStoreError::ExportRejected)?;
        file.sync_all()
            .map_err(|_| OperationalStoreError::ExportRejected)?;
        fs::hard_link(&temporary, destination)
            .map_err(|_| OperationalStoreError::ExportRejected)?;
        let source_metadata =
            fs::symlink_metadata(&temporary).map_err(|_| OperationalStoreError::ExportRejected)?;
        let destination_metadata =
            fs::symlink_metadata(destination).map_err(|_| OperationalStoreError::ExportRejected)?;
        if !destination_metadata.is_file()
            || destination_metadata.file_type().is_symlink()
            || destination_metadata.mode() & 0o077 != 0
            || source_metadata.dev() != destination_metadata.dev()
            || source_metadata.ino() != destination_metadata.ino()
        {
            return Err(OperationalStoreError::ExportRejected);
        }
        published_identity = Some((destination_metadata.dev(), destination_metadata.ino()));
        fs::remove_file(&temporary).map_err(|_| OperationalStoreError::ExportRejected)?;
        File::open(parent)
            .and_then(|directory| directory.sync_all())
            .map_err(|_| OperationalStoreError::ExportRejected)
    })();
    drop(file);
    let _ = fs::remove_file(&temporary);
    if result.is_err()
        && published_identity.is_some_and(|(device, inode)| {
            fs::symlink_metadata(destination)
                .is_ok_and(|current| current.dev() == device && current.ino() == inode)
        })
    {
        let _ = fs::remove_file(destination);
    }
    result
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

fn sha256_file(path: &Path) -> Result<String, OperationalStoreError> {
    verify_store_file(path)?;
    let mut file = OpenOptions::new()
        .read(true)
        .open(path)
        .map_err(|_| OperationalStoreError::IntegrityFailure)?;
    let mut digest = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|_| OperationalStoreError::IntegrityFailure)?;
        if read == 0 {
            break;
        }
        digest.update(&buffer[..read]);
    }
    Ok(hex_digest(&digest.finalize()))
}

fn sqlite_artifact_paths(path: &Path) -> [PathBuf; 3] {
    let mut wal = path.as_os_str().to_os_string();
    wal.push("-wal");
    let mut shared_memory = path.as_os_str().to_os_string();
    shared_memory.push("-shm");
    [
        path.to_path_buf(),
        PathBuf::from(wal),
        PathBuf::from(shared_memory),
    ]
}

fn remove_sqlite_artifacts(path: &Path) {
    for artifact in sqlite_artifact_paths(path) {
        if fs::symlink_metadata(&artifact)
            .is_ok_and(|metadata| metadata.is_file() && !metadata.file_type().is_symlink())
        {
            let _ = fs::remove_file(artifact);
        }
    }
}

fn sqlite_artifacts_exist(path: &Path) -> bool {
    sqlite_artifact_paths(path)
        .into_iter()
        .any(|artifact| fs::symlink_metadata(artifact).is_ok())
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
    use std::collections::BTreeMap;
    use std::env;
    use std::fs::{self, File, OpenOptions};
    use std::io::{Read as _, Seek as _, SeekFrom, Write as _};
    use std::os::fd::AsRawFd;
    use std::os::unix::fs::PermissionsExt as _;
    use std::path::{Path, PathBuf};
    use std::process::Command;
    use std::sync::atomic::{AtomicU64, Ordering};

    use agentmage_kernel_contracts::{
        CloudSynchronizationMarker, StorageFilesystemClass, StrictLocalStorageObservation,
    };
    use rusqlite::params;
    use serde_json::Value;

    use super::{
        MIGRATION_1_SCHEMA_SQL, MIGRATION_2_SCHEMA_SQL, MIGRATION_3_SCHEMA_SQL, OperationalStore,
        OperationalStoreError, OperationalStoreKeyError, OperationalStoreKeyLifecycle,
        OperationalStoreKeyProvider, RetentionAssignment, RetentionDisposition, RetentionHoldKind,
        RetentionRecordFamily, RetentionSensitivity, SCHEMA_VERSION, ZERO_SHA256,
        is_linux_held_descriptor_path, open_connection, prepare_new_store_file, sha256_file,
        sha256_hex, sqlite_artifact_paths, verify_runtime_configuration,
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

    struct ErasableTestKey(Option<[u8; 32]>);

    impl OperationalStoreKeyProvider for ErasableTestKey {
        fn with_key<T>(
            &mut self,
            operation: impl FnOnce(&[u8]) -> T,
        ) -> Result<T, OperationalStoreKeyError> {
            self.0
                .as_ref()
                .map(|key| operation(key))
                .ok_or(OperationalStoreKeyError::Unavailable)
        }
    }

    impl OperationalStoreKeyLifecycle for ErasableTestKey {
        fn destroy_key_and_verify_absent(&mut self) -> Result<(), OperationalStoreKeyError> {
            self.0
                .take()
                .map(|_| ())
                .ok_or(OperationalStoreKeyError::Unavailable)
        }
    }

    struct FileBackedTestKey {
        path: PathBuf,
    }

    impl OperationalStoreKeyProvider for FileBackedTestKey {
        fn with_key<T>(
            &mut self,
            operation: impl FnOnce(&[u8]) -> T,
        ) -> Result<T, OperationalStoreKeyError> {
            let key = fs::read(&self.path).map_err(|_| OperationalStoreKeyError::Unavailable)?;
            if key.len() != 32 {
                return Err(OperationalStoreKeyError::Unavailable);
            }
            Ok(operation(&key))
        }
    }

    impl OperationalStoreKeyLifecycle for FileBackedTestKey {
        fn destroy_key_and_verify_absent(&mut self) -> Result<(), OperationalStoreKeyError> {
            fs::remove_file(&self.path).map_err(|_| OperationalStoreKeyError::Unavailable)?;
            if self.path.exists() {
                Err(OperationalStoreKeyError::Unavailable)
            } else {
                Ok(())
            }
        }
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
    enum SeededCrashBoundary {
        Transaction,
        Checkpoint,
        Migration,
        KeyRetrieval,
        Backup,
        Restore,
        Expiry,
        Deletion,
    }

    impl SeededCrashBoundary {
        const ALL: [Self; 8] = [
            Self::Transaction,
            Self::Checkpoint,
            Self::Migration,
            Self::KeyRetrieval,
            Self::Backup,
            Self::Restore,
            Self::Expiry,
            Self::Deletion,
        ];

        const fn code(self) -> &'static str {
            match self {
                Self::Transaction => "transaction",
                Self::Checkpoint => "checkpoint",
                Self::Migration => "migration",
                Self::KeyRetrieval => "key-retrieval",
                Self::Backup => "backup",
                Self::Restore => "restore",
                Self::Expiry => "expiry",
                Self::Deletion => "deletion",
            }
        }

        fn from_code(code: &str) -> Self {
            Self::ALL
                .into_iter()
                .find(|boundary| boundary.code() == code)
                .expect("declared crash boundary")
        }
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
    enum SeededCrashPosition {
        Before,
        After,
    }

    impl SeededCrashPosition {
        const ALL: [Self; 2] = [Self::Before, Self::After];

        const fn code(self) -> &'static str {
            match self {
                Self::Before => "before",
                Self::After => "after",
            }
        }

        fn from_code(code: &str) -> Self {
            Self::ALL
                .into_iter()
                .find(|position| position.code() == code)
                .expect("declared crash position")
        }
    }

    const SEEDED_CRASH_CHILD_EXIT: i32 = 86;
    const SEEDED_CRASH_RUNS: u64 = 128;

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

    fn assert_artifacts_exclude_canary(store: &Path, backup: &Path, export: &Path, canary: &str) {
        for artifact in sqlite_artifact_paths(store)
            .into_iter()
            .chain(sqlite_artifact_paths(backup))
            .chain([export.to_path_buf()])
        {
            if let Ok(bytes) = fs::read(artifact) {
                assert!(
                    !bytes
                        .windows(canary.len())
                        .any(|window| window == canary.as_bytes())
                );
            }
        }
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

    fn create_version_two_store(path: &Path, key: &[u8; 32]) {
        create_version_one_store(path, key);
        let connection = open_connection(path, key).expect("version two encrypted connection");
        let transaction = connection
            .unchecked_transaction()
            .expect("version two migration transaction");
        transaction
            .execute_batch(MIGRATION_2_SCHEMA_SQL)
            .expect("version two schema");
        transaction
            .execute(
                "INSERT INTO schema_history(version, migration_sha256) VALUES (2, ?1)",
                [sha256_hex(MIGRATION_2_SCHEMA_SQL.as_bytes())],
            )
            .expect("version two history");
        transaction
            .pragma_update(None, "user_version", 2)
            .expect("version two user version");
        transaction.commit().expect("version two migration commit");
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
        let backup = directory.join("authority.backup.db");
        assert_eq!(
            store
                .backup(&backup, &observation(), &mut MissingKey)
                .expect_err("missing backup key must fail"),
            OperationalStoreError::KeyUnavailable
        );
        assert!(!backup.exists());
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
    fn plaintext_sqlite_substitution_never_becomes_operational_authority() {
        let directory = temporary_directory();
        let path = directory.join("plaintext-substitution.db");
        let mut plaintext = b"SQLite format 3\0".to_vec();
        plaintext.resize(4096, 0);
        fs::write(&path, &plaintext).expect("plaintext substitution fixture");
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600))
            .expect("private fixture mode");

        assert!(OperationalStore::open(&path, &observation(), &mut TestKey([6; 32])).is_err());
        assert_eq!(fs::read(&path).expect("unchanged substitution"), plaintext);
        assert!(!path.with_extension("db-wal").exists());
        assert!(!path.with_extension("db-shm").exists());
        fs::remove_dir_all(directory).expect("cleanup");
    }

    #[test]
    fn exclusive_writer_and_encrypted_backup_are_verified() {
        let directory = temporary_directory();
        let path = directory.join("authority.db");
        let backup = directory.join("authority.backup.db");
        let store = OperationalStore::open(&path, &observation(), &mut TestKey([8; 32]))
            .expect("first writer");
        assert!(matches!(
            OperationalStore::open(&path, &observation(), &mut TestKey([8; 32]))
                .expect_err("second writer must fail"),
            OperationalStoreError::OpenFailed | OperationalStoreError::ConcurrentWriter
        ));
        let backup_receipt = store
            .backup(&backup, &observation(), &mut TestKey([9; 32]))
            .expect("encrypted backup");
        assert_eq!(backup_receipt.schema_version, 3);
        assert_eq!(backup_receipt.generation, 0);
        assert_eq!(backup_receipt.encrypted_file_sha256.len(), 64);
        assert_eq!(
            backup_receipt.encrypted_file_sha256,
            sha256_file(&backup).expect("closed backup digest")
        );
        assert!(
            sqlite_artifact_paths(&backup)[1..]
                .iter()
                .all(|sidecar| !sidecar.exists())
        );
        let bytes = fs::read(&backup).expect("backup bytes");
        assert!(!bytes.starts_with(b"SQLite format 3\0"));
        let occupied = directory.join("occupied.backup.db");
        fs::write(&occupied, b"do-not-overwrite").expect("occupied backup fixture");
        assert!(
            store
                .backup(&occupied, &observation(), &mut TestKey([9; 32]))
                .is_err()
        );
        assert_eq!(
            fs::read(&occupied).expect("occupied backup remains"),
            b"do-not-overwrite"
        );
        drop(store);
        let restored = OperationalStore::open(&backup, &observation(), &mut TestKey([9; 32]))
            .expect("backup reopens");
        assert_eq!(restored.generation(), 0);
        drop(restored);
        fs::remove_dir_all(directory).expect("cleanup");
    }

    #[test]
    fn encrypted_backup_restores_only_to_a_verified_fresh_candidate() {
        let directory = temporary_directory();
        let source = directory.join("authority.db");
        let backup = directory.join("authority.backup.db");
        let candidate = directory.join("authority.candidate.db");
        let store = OperationalStore::open(&source, &observation(), &mut TestKey([30; 32]))
            .expect("source store");
        let backup_receipt = store
            .backup(&backup, &observation(), &mut TestKey([31; 32]))
            .expect("verified backup");
        drop(store);

        let restore_receipt = OperationalStore::restore_to_fresh_candidate(
            &backup,
            &observation(),
            &mut TestKey([31; 32]),
            &candidate,
            &observation(),
            &mut TestKey([32; 32]),
        )
        .expect("verified restore candidate");
        assert_eq!(
            (restore_receipt.schema_version, restore_receipt.generation),
            (backup_receipt.schema_version, backup_receipt.generation)
        );
        assert_eq!(restore_receipt.encrypted_file_sha256.len(), 64);
        assert_ne!(
            restore_receipt.encrypted_file_sha256,
            backup_receipt.encrypted_file_sha256
        );
        drop(
            OperationalStore::open(&candidate, &observation(), &mut TestKey([32; 32]))
                .expect("candidate opens under destination key"),
        );

        let occupied = directory.join("occupied.db");
        fs::write(&occupied, b"do-not-overwrite").expect("occupied fixture");
        assert_eq!(
            OperationalStore::restore_to_fresh_candidate(
                &backup,
                &observation(),
                &mut TestKey([31; 32]),
                &occupied,
                &observation(),
                &mut TestKey([33; 32]),
            )
            .expect_err("occupied destination must fail"),
            OperationalStoreError::RestoreFailure
        );
        assert_eq!(
            fs::read(&occupied).expect("occupied remains"),
            b"do-not-overwrite"
        );
        fs::remove_dir_all(directory).expect("cleanup");
    }

    #[test]
    fn synthetic_canary_is_absent_from_encrypted_and_derived_artifacts() {
        let directory = temporary_directory();
        let path = directory.join("authority.db");
        let backup = directory.join("authority.backup.db");
        let export = directory.join("authority-derived.jsonl");
        let canary = "AM_SYNTHETIC_SECRET_SPRINT11_STORAGE_BOUNDARY";
        let record = format!(r#"{{"private":"{canary}"}}"#);
        let record_sha256 = sha256_hex(record.as_bytes());
        let store = OperationalStore::open(&path, &observation(), &mut TestKey([61; 32]))
            .expect("encrypted store");
        store
            .connection
            .execute(
                "INSERT INTO sessions VALUES (?1, ?2, 'active', 1, 1, ?3, ?4)",
                params![canary, canary, record_sha256, record.as_bytes()],
            )
            .expect("synthetic canary fixture");

        let backup_receipt = store
            .backup(&backup, &observation(), &mut TestKey([62; 32]))
            .expect("encrypted canary backup");
        let export_receipt = store
            .export_json_lines(&export, &observation())
            .expect("content-free derivative");
        let diagnostics = format!(
            "{store:?}{backup_receipt:?}{export_receipt:?}{}{}",
            OperationalStoreError::IntegrityFailure,
            OperationalStoreError::RestoreFailure
        );
        assert!(!diagnostics.contains(canary));
        assert_artifacts_exclude_canary(&path, &backup, &export, canary);
        drop(store);
        assert_artifacts_exclude_canary(&path, &backup, &export, canary);

        let restored = OperationalStore::open(&backup, &observation(), &mut TestKey([62; 32]))
            .expect("encrypted backup reopens");
        assert_eq!(restored.generation(), backup_receipt.generation);
        drop(restored);
        fs::remove_dir_all(directory).expect("cleanup");
    }

    #[test]
    fn corrupted_or_wrongly_keyed_backup_leaves_no_restore_candidate() {
        let directory = temporary_directory();
        let source = directory.join("authority.db");
        let backup = directory.join("authority.backup.db");
        let wrong_key_candidate = directory.join("wrong-key.db");
        let corrupt_candidate = directory.join("corrupt-candidate.db");
        let store = OperationalStore::open(&source, &observation(), &mut TestKey([34; 32]))
            .expect("source store");
        store
            .backup(&backup, &observation(), &mut TestKey([35; 32]))
            .expect("verified backup");
        drop(store);
        assert!(
            OperationalStore::restore_to_fresh_candidate(
                &backup,
                &observation(),
                &mut TestKey([36; 32]),
                &wrong_key_candidate,
                &observation(),
                &mut TestKey([37; 32]),
            )
            .is_err()
        );
        assert!(!wrong_key_candidate.exists());

        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&backup)
            .expect("backup fixture");
        file.seek(SeekFrom::Start(128)).expect("seek backup page");
        let mut byte = [0_u8; 1];
        file.read_exact(&mut byte).expect("read backup byte");
        byte[0] ^= 0xff;
        file.seek(SeekFrom::Start(128))
            .expect("seek backup page again");
        file.write_all(&byte).expect("corrupt backup byte");
        file.sync_all().expect("sync corruption");
        drop(file);
        assert!(
            OperationalStore::restore_to_fresh_candidate(
                &backup,
                &observation(),
                &mut TestKey([35; 32]),
                &corrupt_candidate,
                &observation(),
                &mut TestKey([38; 32]),
            )
            .is_err()
        );
        assert!(!corrupt_candidate.exists());
        fs::remove_dir_all(directory).expect("cleanup");
    }

    #[test]
    fn whole_store_cryptographic_erasure_consumes_key_scope_without_overwrite_claim() {
        let directory = temporary_directory();
        let path = directory.join("authority.db");
        let mut provider = ErasableTestKey(Some([40; 32]));
        let store =
            OperationalStore::open(&path, &observation(), &mut provider).expect("encrypted store");
        let receipt = store
            .cryptographic_erase(&mut provider)
            .expect("key erasure");
        assert!(receipt.key_destroyed_and_absent);
        assert!(receipt.encrypted_files_removed);
        assert!(!receipt.physical_overwrite_claim);
        assert!(!path.exists());
        assert_eq!(
            OperationalStore::open(&path, &observation(), &mut provider)
                .expect_err("destroyed key cannot reopen"),
            OperationalStoreError::KeyUnavailable
        );
        assert!(!path.exists());

        let failed_path = directory.join("failed-erasure.db");
        let failed_store =
            OperationalStore::open(&failed_path, &observation(), &mut TestKey([41; 32]))
                .expect("second encrypted store");
        assert_eq!(
            failed_store
                .cryptographic_erase(&mut ErasableTestKey(None))
                .expect_err("absent lifecycle authority must fail"),
            OperationalStoreError::KeyErasureFailure
        );
        assert!(failed_path.exists());
        fs::remove_dir_all(directory).expect("cleanup");
    }

    #[test]
    fn json_lines_export_is_deterministic_content_free_and_export_only() {
        let directory = temporary_directory();
        let path = directory.join("authority.db");
        let first_export = directory.join("first.jsonl");
        let second_export = directory.join("second.jsonl");
        let startup_candidate = directory.join("export-as-authority.db");
        let store = OperationalStore::open(&path, &observation(), &mut TestKey([50; 32]))
            .expect("encrypted store");
        let digest = "c".repeat(64);
        let raw_identifier = "private-session-identifier";
        let raw_record = br#"{"private":"must-not-export"}"#;
        store
            .connection
            .execute(
                "INSERT INTO sessions VALUES (?1, 'profile-1', 'active', 1, 1, ?2, ?3)",
                params![raw_identifier, &digest, raw_record.as_slice()],
            )
            .expect("session fixture");
        let before_generation = store.generation();
        let before_state: String = store
            .connection
            .query_row(
                "SELECT state_sha256 FROM store_metadata WHERE singleton = 1",
                [],
                |row| row.get(0),
            )
            .expect("state identity");

        let first_receipt = store
            .export_json_lines(&first_export, &observation())
            .expect("first derived export");
        let first_bytes = fs::read(&first_export).expect("first export bytes");
        assert!(
            !first_bytes
                .windows(raw_identifier.len())
                .any(|window| window == raw_identifier.as_bytes())
        );
        assert!(
            !first_bytes
                .windows("must-not-export".len())
                .any(|window| window == b"must-not-export")
        );
        let lines: Vec<Value> = first_bytes
            .split(|byte| *byte == b'\n')
            .filter(|line| !line.is_empty())
            .map(|line| serde_json::from_slice(line).expect("valid JSON line"))
            .collect();
        assert_eq!(lines.len(), first_receipt.record_count + 1);
        assert_eq!(lines[0]["record_type"], "agentmage.derived_export.header");
        assert_eq!(lines[0]["executable"], false);
        assert_eq!(lines[0]["startup_authority"], false);
        assert_eq!(
            lines[0]["content_mode"],
            "identity-and-retained-hashes-only"
        );
        for row in &lines[1..] {
            assert_eq!(row["record_type"], "agentmage.derived_export.record");
            assert!(row.get("record_id").is_none());
            assert!(row.get("record_json").is_none());
        }

        let second_receipt = store
            .export_json_lines(&second_export, &observation())
            .expect("second derived export");
        assert_eq!(first_receipt, second_receipt);
        assert_eq!(first_bytes, fs::read(&second_export).expect("second bytes"));
        assert_eq!(store.generation(), before_generation);
        assert_eq!(
            store
                .connection
                .query_row(
                    "SELECT state_sha256 FROM store_metadata WHERE singleton = 1",
                    [],
                    |row| row.get::<_, String>(0),
                )
                .expect("unchanged state"),
            before_state
        );

        fs::write(&first_export, b"hostile derived replacement\n").expect("mutate derivative");
        fs::remove_file(&second_export).expect("delete derivative");
        fs::write(&startup_candidate, &first_bytes).expect("startup candidate bytes");
        fs::set_permissions(&startup_candidate, fs::Permissions::from_mode(0o600))
            .expect("private startup candidate");
        drop(store);
        assert!(
            OperationalStore::open(&startup_candidate, &observation(), &mut TestKey([50; 32]),)
                .is_err()
        );
        let reopened = OperationalStore::open(&path, &observation(), &mut TestKey([50; 32]))
            .expect("canonical store ignores derivatives");
        assert_eq!(reopened.generation(), before_generation);
        drop(reopened);
        fs::remove_dir_all(directory).expect("cleanup");
    }

    #[test]
    fn json_lines_export_rejects_occupied_or_ineligible_destinations_without_change() {
        let directory = temporary_directory();
        let path = directory.join("authority.db");
        let occupied = directory.join("occupied.jsonl");
        let remote_export = directory.join("remote.jsonl");
        let store = OperationalStore::open(&path, &observation(), &mut TestKey([51; 32]))
            .expect("encrypted store");
        fs::write(&occupied, b"do-not-overwrite").expect("occupied fixture");
        assert_eq!(
            store
                .export_json_lines(&occupied, &observation())
                .expect_err("occupied export must fail"),
            OperationalStoreError::ExportRejected
        );
        assert_eq!(
            fs::read(&occupied).expect("occupied remains"),
            b"do-not-overwrite"
        );
        let remote = StrictLocalStorageObservation {
            filesystem: StorageFilesystemClass::Remote,
            synchronization_marker: None,
            root_identity_sha256: [7; 32],
            symlink_free: true,
        };
        assert_eq!(
            store
                .export_json_lines(&remote_export, &remote)
                .expect_err("remote export must fail"),
            OperationalStoreError::StorageRejected
        );
        assert!(!remote_export.exists());
        drop(store);
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
                "CREATE TRIGGER reject_checkpoint
                 BEFORE INSERT ON checkpoints
                 WHEN NEW.generation > 0
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
    fn required_writer_and_database_configuration_is_verified() {
        let directory = temporary_directory();
        let path = directory.join("authority.db");
        let store = OperationalStore::open(&path, &observation(), &mut TestKey([17; 32]))
            .expect("configured encrypted store");
        verify_runtime_configuration(&store.connection).expect("exact runtime configuration");
        store
            .connection
            .pragma_update(None, "foreign_keys", false)
            .expect("configuration mutation");
        assert_eq!(
            verify_runtime_configuration(&store.connection)
                .expect_err("weakened configuration must fail"),
            OperationalStoreError::OpenFailed
        );
        drop(store);
        fs::remove_dir_all(directory).expect("cleanup");
    }

    #[test]
    fn version_three_schema_is_normalized_closed_and_relational() {
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
                "retention_events",
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
                "INSERT INTO retention(
                    retention_id, record_family, record_id, sensitivity, disposition,
                    expires_at_epoch_ms, legal_hold, policy_sha256
                 ) VALUES (?1, ?2, ?3, ?4, ?5, NULL, ?6, ?7)",
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
                    "INSERT INTO retention(
                        retention_id, record_family, record_id, sensitivity, disposition,
                        expires_at_epoch_ms, legal_hold, policy_sha256
                     ) VALUES ('bad-retention', 'unknown', 'x', 'private', 'retained', NULL, 0, ?1)",
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
    fn version_one_upgrades_through_three_with_exact_history() {
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
                (3, sha256_hex(MIGRATION_3_SCHEMA_SQL.as_bytes())),
            ]
        );
        drop(store);
        fs::remove_dir_all(directory).expect("cleanup");
    }

    #[test]
    fn version_two_retention_rows_upgrade_to_three_with_initial_event() {
        let directory = temporary_directory();
        let path = directory.join("authority.db");
        create_version_two_store(&path, &[17; 32]);
        let connection = open_connection(&path, &[17; 32]).expect("version two fixture");
        let digest = "d".repeat(64);
        connection
            .execute(
                "INSERT INTO sessions VALUES ('legacy-session', 'profile-1', 'active', 1, 1, ?1, X'7B7D')",
                [&digest],
            )
            .expect("legacy session");
        connection
            .execute(
                "INSERT INTO retention VALUES (
                    'legacy-retention', 'sessions', 'legacy-session', 'private',
                    'retained', 100, 0, ?1
                )",
                [&digest],
            )
            .expect("legacy retention");
        drop(connection);

        let store = OperationalStore::open(&path, &observation(), &mut TestKey([17; 32]))
            .expect("version two upgrades");
        let retained: (String, Option<String>, i64, i64) = store
            .connection
            .query_row(
                "SELECT hold_kind, prior_disposition, revision, updated_at_epoch_ms
                 FROM retention WHERE retention_id = 'legacy-retention'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .expect("migrated retention");
        assert_eq!(retained, ("none".to_owned(), None, 1, 0));
        let event: (String, i64, String) = store
            .connection
            .query_row(
                "SELECT event_kind, occurred_at_epoch_ms, previous_event_sha256
                 FROM retention_events WHERE retention_id = 'legacy-retention'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .expect("migration event");
        assert_eq!(event, ("assigned".to_owned(), 0, ZERO_SHA256.to_owned()));
        drop(store);
        fs::remove_dir_all(directory).expect("cleanup");
    }

    #[test]
    fn failed_version_three_migration_rolls_back_all_alterations() {
        let directory = temporary_directory();
        let path = directory.join("authority.db");
        create_version_two_store(&path, &[18; 32]);
        let connection = open_connection(&path, &[18; 32]).expect("version two fixture");
        connection
            .execute_batch("CREATE TABLE retention_events(incompatible INTEGER) STRICT;")
            .expect("conflicting version three table");
        drop(connection);
        assert_eq!(
            OperationalStore::open(&path, &observation(), &mut TestKey([18; 32]))
                .expect_err("conflicting migration must fail"),
            OperationalStoreError::MigrationFailed
        );
        let connection = open_connection(&path, &[18; 32]).expect("rolled-back version two");
        let version: i64 = connection
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .expect("rolled-back version");
        let history_count: i64 = connection
            .query_row("SELECT COUNT(*) FROM schema_history", [], |row| row.get(0))
            .expect("rolled-back history");
        let v3_columns: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM pragma_table_info('retention')
                 WHERE name IN ('hold_kind', 'prior_disposition', 'revision',
                                'updated_at_epoch_ms', 'erased_at_epoch_ms')",
                [],
                |row| row.get(0),
            )
            .expect("rolled-back columns");
        let v3_index: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM sqlite_schema
                 WHERE type = 'index' AND name = 'retention_hold_expiry_idx'",
                [],
                |row| row.get(0),
            )
            .expect("rolled-back index");
        assert_eq!((version, history_count, v3_columns, v3_index), (2, 2, 0, 0));
        drop(connection);
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

    #[test]
    fn retention_holds_expiration_and_stale_revisions_are_atomic() {
        let directory = temporary_directory();
        let path = directory.join("authority.db");
        let mut store = OperationalStore::open(&path, &observation(), &mut TestKey([21; 32]))
            .expect("encrypted store");
        let digest = "a".repeat(64);
        store
            .connection
            .execute(
                "INSERT INTO sessions VALUES (?1, ?2, 'active', 1, 1, ?3, X'7B7D')",
                params!["session-retained", "profile-1", &digest],
            )
            .expect("session fixture");
        let assignment = RetentionAssignment::new(
            "retention-session",
            RetentionRecordFamily::Sessions,
            "session-retained",
            RetentionSensitivity::Private,
            RetentionDisposition::Session,
            Some(100),
            [7; 32],
        )
        .expect("valid assignment");
        let assigned = store
            .assign_retention(&assignment, 10)
            .expect("assignment commits");
        assert_eq!((assigned.revision, assigned.event), (1, "assigned"));

        let held = store
            .apply_retention_hold("retention-session", 1, RetentionHoldKind::User, 20)
            .expect("user hold commits");
        assert_eq!((held.revision, held.event), (2, "user_hold_applied"));
        assert!(store.expire_due(100).expect("held expiry scan").is_empty());
        assert_eq!(
            store
                .release_retention_hold("retention-session", 2, RetentionHoldKind::Legal, 110,)
                .expect_err("wrong hold kind must fail"),
            OperationalStoreError::LifecycleRejected
        );
        let released = store
            .release_retention_hold("retention-session", 2, RetentionHoldKind::User, 110)
            .expect("matching release commits");
        assert_eq!((released.revision, released.event), (3, "hold_released"));
        assert_eq!(
            store
                .apply_retention_hold("retention-session", 2, RetentionHoldKind::Legal, 120)
                .expect_err("stale revision must fail"),
            OperationalStoreError::LifecycleRejected
        );
        let expired = store.expire_due(120).expect("due expiry commits");
        assert_eq!(expired.len(), 1);
        assert_eq!((expired[0].revision, expired[0].event), (4, "expired"));

        let state: (String, String, i64, i64) = store
            .connection
            .query_row(
                "SELECT disposition, hold_kind, legal_hold, revision
                 FROM retention WHERE retention_id = 'retention-session'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .expect("retention state");
        assert_eq!(state, ("expired".to_owned(), "none".to_owned(), 0, 4));
        drop(store);
        drop(
            OperationalStore::open(&path, &observation(), &mut TestKey([21; 32]))
                .expect("valid event chain reopens"),
        );
        fs::remove_dir_all(directory).expect("cleanup");
    }

    #[test]
    fn retention_assignment_and_event_tampering_fail_closed() {
        let directory = temporary_directory();
        let path = directory.join("authority.db");
        let mut store = OperationalStore::open(&path, &observation(), &mut TestKey([22; 32]))
            .expect("encrypted store");
        let missing = RetentionAssignment::new(
            "missing-retention",
            RetentionRecordFamily::Sessions,
            "missing-session",
            RetentionSensitivity::Internal,
            RetentionDisposition::Retained,
            Some(200),
            [8; 32],
        )
        .expect("shaped assignment");
        assert_eq!(
            store
                .assign_retention(&missing, 1)
                .expect_err("orphan retention must fail"),
            OperationalStoreError::LifecycleRejected
        );
        let digest = "b".repeat(64);
        store
            .connection
            .execute(
                "INSERT INTO sessions VALUES (?1, ?2, 'active', 1, 1, ?3, X'7B7D')",
                params!["session-legal", "profile-1", &digest],
            )
            .expect("session fixture");
        let assignment = RetentionAssignment::new(
            "legal-retention",
            RetentionRecordFamily::Sessions,
            "session-legal",
            RetentionSensitivity::Restricted,
            RetentionDisposition::Retained,
            Some(200),
            [9; 32],
        )
        .expect("assignment");
        store
            .assign_retention(&assignment, 1)
            .expect("assignment commits");
        store
            .apply_retention_hold("legal-retention", 1, RetentionHoldKind::Legal, 2)
            .expect("legal hold commits");
        store
            .connection
            .execute(
                "UPDATE retention_events SET event_sha256 = ?1
                 WHERE retention_id = 'legal-retention' AND revision = 2",
                ["f".repeat(64)],
            )
            .expect("tamper fixture");
        drop(store);
        assert_eq!(
            OperationalStore::open(&path, &observation(), &mut TestKey([22; 32]))
                .expect_err("tampered event chain must fail"),
            OperationalStoreError::IntegrityFailure
        );
        fs::remove_dir_all(directory).expect("cleanup");
    }

    fn seeded_key(seed: u64) -> [u8; 32] {
        [u8::try_from(seed % 251 + 1).expect("bounded seed byte"); 32]
    }

    fn seeded_crash_stop() -> ! {
        std::process::exit(SEEDED_CRASH_CHILD_EXIT)
    }

    fn seed_retention_fixture(path: &Path, key: [u8; 32]) {
        let mut store = OperationalStore::open(path, &observation(), &mut TestKey(key))
            .expect("retention crash fixture");
        let digest = "a".repeat(64);
        store
            .connection
            .execute(
                "INSERT INTO sessions VALUES (
                    'seeded-expiry-session', 'seeded-profile', 'active', 1, 1, ?1, X'7B7D'
                 )",
                [&digest],
            )
            .expect("seeded expiry session");
        let assignment = RetentionAssignment::new(
            "seeded-expiry-retention",
            RetentionRecordFamily::Sessions,
            "seeded-expiry-session",
            RetentionSensitivity::Private,
            RetentionDisposition::Retained,
            Some(100),
            [17; 32],
        )
        .expect("seeded expiry assignment");
        store
            .assign_retention(&assignment, 10)
            .expect("seeded retention assignment commits");
    }

    fn prepare_seeded_crash_fixture(
        boundary: SeededCrashBoundary,
        directory: &Path,
        key: [u8; 32],
    ) {
        let store_path = directory.join("authority.db");
        match boundary {
            SeededCrashBoundary::Migration => create_version_two_store(&store_path, &key),
            SeededCrashBoundary::KeyRetrieval => {}
            SeededCrashBoundary::Restore => {
                let backup_path = directory.join("authority.backup.db");
                let store = OperationalStore::open(&store_path, &observation(), &mut TestKey(key))
                    .expect("restore source fixture");
                store
                    .backup(&backup_path, &observation(), &mut TestKey([91; 32]))
                    .expect("restore backup fixture");
            }
            SeededCrashBoundary::Expiry => seed_retention_fixture(&store_path, key),
            SeededCrashBoundary::Deletion => {
                let key_path = directory.join("synthetic-key");
                fs::write(&key_path, key).expect("synthetic erasure key fixture");
                drop(
                    OperationalStore::open(
                        &store_path,
                        &observation(),
                        &mut FileBackedTestKey { path: key_path },
                    )
                    .expect("erasure store fixture"),
                );
            }
            SeededCrashBoundary::Transaction
            | SeededCrashBoundary::Checkpoint
            | SeededCrashBoundary::Backup => {
                drop(
                    OperationalStore::open(&store_path, &observation(), &mut TestKey(key))
                        .expect("seeded store fixture"),
                );
            }
        }
    }

    fn run_seeded_crash_child(
        boundary: SeededCrashBoundary,
        position: SeededCrashPosition,
        directory: &Path,
        key: [u8; 32],
    ) -> ! {
        let store_path = directory.join("authority.db");
        let backup_path = directory.join("authority.backup.db");
        let restore_path = directory.join("restored.db");
        match boundary {
            SeededCrashBoundary::Transaction => {
                let store = OperationalStore::open(&store_path, &observation(), &mut TestKey(key))
                    .expect("transaction child opens");
                if position == SeededCrashPosition::Before {
                    seeded_crash_stop();
                }
                store
                    .connection
                    .execute(
                        "INSERT INTO sessions VALUES (
                            'seeded-transaction', 'seeded-profile', 'active', 1, 1, ?1, X'7B7D'
                         )",
                        ["b".repeat(64)],
                    )
                    .expect("transaction child commits");
            }
            SeededCrashBoundary::Checkpoint => {
                let mut store =
                    OperationalStore::open(&store_path, &observation(), &mut TestKey(key))
                        .expect("checkpoint child opens");
                if position == SeededCrashPosition::Before {
                    seeded_crash_stop();
                }
                store
                    .persist_authority(&GrantIssuer::new(), &AuthorityTransactionCoordinator::new())
                    .expect("checkpoint child commits");
            }
            SeededCrashBoundary::Migration => {
                if position == SeededCrashPosition::Before {
                    let connection = open_connection(&store_path, &key)
                        .expect("pre-migration child opens encrypted fixture");
                    let version: i64 = connection
                        .pragma_query_value(None, "user_version", |row| row.get(0))
                        .expect("pre-migration version");
                    assert_eq!(version, 2);
                    seeded_crash_stop();
                }
                drop(
                    OperationalStore::open(&store_path, &observation(), &mut TestKey(key))
                        .expect("migration child commits"),
                );
            }
            SeededCrashBoundary::KeyRetrieval => {
                struct AbruptKeyProvider {
                    key: [u8; 32],
                    position: SeededCrashPosition,
                }

                impl OperationalStoreKeyProvider for AbruptKeyProvider {
                    fn with_key<T>(
                        &mut self,
                        operation: impl FnOnce(&[u8]) -> T,
                    ) -> Result<T, OperationalStoreKeyError> {
                        if self.position == SeededCrashPosition::Before {
                            seeded_crash_stop();
                        }
                        let _result = operation(&self.key);
                        seeded_crash_stop();
                    }
                }

                let _ = OperationalStore::open(
                    &store_path,
                    &observation(),
                    &mut AbruptKeyProvider { key, position },
                );
                unreachable!("abrupt key provider always stops")
            }
            SeededCrashBoundary::Backup => {
                let store = OperationalStore::open(&store_path, &observation(), &mut TestKey(key))
                    .expect("backup child opens");
                if position == SeededCrashPosition::Before {
                    seeded_crash_stop();
                }
                store
                    .backup(&backup_path, &observation(), &mut TestKey([92; 32]))
                    .expect("backup child commits");
            }
            SeededCrashBoundary::Restore => {
                if position == SeededCrashPosition::Before {
                    seeded_crash_stop();
                }
                OperationalStore::restore_to_fresh_candidate(
                    &backup_path,
                    &observation(),
                    &mut TestKey([91; 32]),
                    &restore_path,
                    &observation(),
                    &mut TestKey([93; 32]),
                )
                .expect("restore child commits");
            }
            SeededCrashBoundary::Expiry => {
                let mut store =
                    OperationalStore::open(&store_path, &observation(), &mut TestKey(key))
                        .expect("expiry child opens");
                if position == SeededCrashPosition::Before {
                    seeded_crash_stop();
                }
                assert_eq!(
                    store.expire_due(100).expect("expiry child commits").len(),
                    1
                );
            }
            SeededCrashBoundary::Deletion => {
                let key_path = directory.join("synthetic-key");
                let mut provider = FileBackedTestKey { path: key_path };
                let store = OperationalStore::open(&store_path, &observation(), &mut provider)
                    .expect("deletion child opens");
                if position == SeededCrashPosition::Before {
                    seeded_crash_stop();
                }
                store
                    .cryptographic_erase(&mut provider)
                    .expect("deletion child commits");
            }
        }
        seeded_crash_stop()
    }

    fn recover_seeded_crash_fixture(
        boundary: SeededCrashBoundary,
        directory: &Path,
        key: [u8; 32],
    ) {
        let store_path = directory.join("authority.db");
        let backup_path = directory.join("authority.backup.db");
        let restore_path = directory.join("restored.db");
        match boundary {
            SeededCrashBoundary::Transaction => {
                let store = OperationalStore::open(&store_path, &observation(), &mut TestKey(key))
                    .expect("transaction recovery opens");
                let count: i64 = store
                    .connection
                    .query_row(
                        "SELECT COUNT(*) FROM sessions WHERE session_id = 'seeded-transaction'",
                        [],
                        |row| row.get(0),
                    )
                    .expect("transaction result count");
                if count == 0 {
                    store
                        .connection
                        .execute(
                            "INSERT INTO sessions VALUES (
                                'seeded-transaction', 'seeded-profile', 'active', 1, 1, ?1, X'7B7D'
                             )",
                            ["b".repeat(64)],
                        )
                        .expect("transaction recovery commits once");
                } else {
                    assert_eq!(count, 1);
                }
                assert!(
                    store
                        .connection
                        .execute(
                            "INSERT INTO sessions VALUES (
                                'seeded-transaction', 'seeded-profile', 'active', 1, 1, ?1, X'7B7D'
                             )",
                            ["b".repeat(64)],
                        )
                        .is_err()
                );
                let final_count: i64 = store
                    .connection
                    .query_row(
                        "SELECT COUNT(*) FROM sessions WHERE session_id = 'seeded-transaction'",
                        [],
                        |row| row.get(0),
                    )
                    .expect("final transaction result count");
                assert_eq!(final_count, 1);
            }
            SeededCrashBoundary::Checkpoint => {
                let mut store =
                    OperationalStore::open(&store_path, &observation(), &mut TestKey(key))
                        .expect("checkpoint recovery opens");
                match store.generation() {
                    0 => store
                        .persist_authority(
                            &GrantIssuer::new(),
                            &AuthorityTransactionCoordinator::new(),
                        )
                        .expect("checkpoint recovery commits once"),
                    1 => {}
                    generation => panic!("unexpected recovered generation {generation}"),
                }
                let checkpoints: i64 = store
                    .connection
                    .query_row("SELECT COUNT(*) FROM checkpoints", [], |row| row.get(0))
                    .expect("checkpoint count");
                assert_eq!((store.generation(), checkpoints), (1, 2));
            }
            SeededCrashBoundary::Migration => {
                let store = OperationalStore::open(&store_path, &observation(), &mut TestKey(key))
                    .expect("migration recovery opens");
                let version: i64 = store
                    .connection
                    .pragma_query_value(None, "user_version", |row| row.get(0))
                    .expect("recovered schema version");
                let history: i64 = store
                    .connection
                    .query_row("SELECT COUNT(*) FROM schema_history", [], |row| row.get(0))
                    .expect("migration history count");
                assert_eq!((version, history), (SCHEMA_VERSION, 3));
            }
            SeededCrashBoundary::KeyRetrieval => {
                let store = OperationalStore::open(&store_path, &observation(), &mut TestKey(key))
                    .expect("key retrieval recovery opens");
                assert_eq!(store.generation(), 0);
            }
            SeededCrashBoundary::Backup => {
                let store = OperationalStore::open(&store_path, &observation(), &mut TestKey(key))
                    .expect("backup recovery source opens");
                if !backup_path.exists() {
                    store
                        .backup(&backup_path, &observation(), &mut TestKey([92; 32]))
                        .expect("backup recovery commits once");
                }
                let before = sha256_file(&backup_path).expect("backup identity");
                assert!(
                    store
                        .backup(&backup_path, &observation(), &mut TestKey([92; 32]))
                        .is_err()
                );
                assert_eq!(
                    sha256_file(&backup_path).expect("preserved backup identity"),
                    before
                );
                drop(store);
                let restored =
                    OperationalStore::open(&backup_path, &observation(), &mut TestKey([92; 32]))
                        .expect("backup recovery validates");
                assert_eq!(restored.generation(), 0);
            }
            SeededCrashBoundary::Restore => {
                if !restore_path.exists() {
                    OperationalStore::restore_to_fresh_candidate(
                        &backup_path,
                        &observation(),
                        &mut TestKey([91; 32]),
                        &restore_path,
                        &observation(),
                        &mut TestKey([93; 32]),
                    )
                    .expect("restore recovery commits once");
                }
                let before = sha256_file(&restore_path).expect("restore identity");
                assert!(
                    OperationalStore::restore_to_fresh_candidate(
                        &backup_path,
                        &observation(),
                        &mut TestKey([91; 32]),
                        &restore_path,
                        &observation(),
                        &mut TestKey([93; 32]),
                    )
                    .is_err()
                );
                assert_eq!(
                    sha256_file(&restore_path).expect("preserved restore identity"),
                    before
                );
                let restored =
                    OperationalStore::open(&restore_path, &observation(), &mut TestKey([93; 32]))
                        .expect("restored candidate validates");
                assert_eq!(restored.generation(), 0);
            }
            SeededCrashBoundary::Expiry => {
                let mut store =
                    OperationalStore::open(&store_path, &observation(), &mut TestKey(key))
                        .expect("expiry recovery opens");
                assert!(store.expire_due(100).expect("expiry recovery").len() <= 1);
                assert!(
                    store
                        .expire_due(100)
                        .expect("expiry idempotency")
                        .is_empty()
                );
                let state: (String, i64) = store
                    .connection
                    .query_row(
                        "SELECT disposition, revision FROM retention
                         WHERE retention_id = 'seeded-expiry-retention'",
                        [],
                        |row| Ok((row.get(0)?, row.get(1)?)),
                    )
                    .expect("expiry state");
                let events: i64 = store
                    .connection
                    .query_row(
                        "SELECT COUNT(*) FROM retention_events
                         WHERE retention_id = 'seeded-expiry-retention'",
                        [],
                        |row| row.get(0),
                    )
                    .expect("expiry event count");
                assert_eq!(state, ("expired".to_owned(), 2));
                assert_eq!(events, 2);
            }
            SeededCrashBoundary::Deletion => {
                let key_path = directory.join("synthetic-key");
                if key_path.exists() {
                    let mut provider = FileBackedTestKey {
                        path: key_path.clone(),
                    };
                    let store = OperationalStore::open(&store_path, &observation(), &mut provider)
                        .expect("deletion recovery opens");
                    store
                        .cryptographic_erase(&mut provider)
                        .expect("deletion recovery commits once");
                }
                assert!(!key_path.exists());
                assert!(!super::sqlite_artifacts_exist(&store_path));
            }
        }
    }

    #[test]
    #[ignore = "subprocess crash target; invoked only by seeded_crash_recovery_campaign"]
    fn seeded_crash_recovery_child() {
        if env::var_os("AGENTMAGE_SEEDED_CRASH_CHILD").is_none() {
            return;
        }
        let boundary = SeededCrashBoundary::from_code(
            &env::var("AGENTMAGE_SEEDED_CRASH_BOUNDARY").expect("child boundary"),
        );
        let position = SeededCrashPosition::from_code(
            &env::var("AGENTMAGE_SEEDED_CRASH_POSITION").expect("child position"),
        );
        let directory = PathBuf::from(
            env::var_os("AGENTMAGE_SEEDED_CRASH_DIRECTORY").expect("child directory"),
        );
        let seed = env::var("AGENTMAGE_SEEDED_CRASH_SEED")
            .expect("child seed")
            .parse::<u64>()
            .expect("numeric child seed");
        run_seeded_crash_child(boundary, position, &directory, seeded_key(seed));
    }

    #[test]
    fn seeded_crash_recovery_campaign_never_repeats_a_completed_transition() {
        let mut coverage = BTreeMap::new();
        for seed in 0..SEEDED_CRASH_RUNS {
            let boundary = SeededCrashBoundary::ALL
                [usize::try_from(seed % 8).expect("bounded boundary index")];
            let position = SeededCrashPosition::ALL
                [usize::try_from((seed / 8) % 2).expect("bounded position index")];
            let directory = temporary_directory();
            let key = seeded_key(seed);
            prepare_seeded_crash_fixture(boundary, &directory, key);

            let output = Command::new(env::current_exe().expect("current test executable"))
                .args([
                    "--exact",
                    "operational_store::tests::seeded_crash_recovery_child",
                    "--ignored",
                    "--nocapture",
                ])
                .env("AGENTMAGE_SEEDED_CRASH_CHILD", "1")
                .env("AGENTMAGE_SEEDED_CRASH_BOUNDARY", boundary.code())
                .env("AGENTMAGE_SEEDED_CRASH_POSITION", position.code())
                .env("AGENTMAGE_SEEDED_CRASH_DIRECTORY", &directory)
                .env("AGENTMAGE_SEEDED_CRASH_SEED", seed.to_string())
                .output()
                .expect("seeded crash child launches");
            assert_eq!(
                output.status.code(),
                Some(SEEDED_CRASH_CHILD_EXIT),
                "seed {seed} {boundary:?} {position:?}: {}",
                String::from_utf8_lossy(&output.stderr)
            );

            recover_seeded_crash_fixture(boundary, &directory, key);
            *coverage.entry((boundary, position)).or_insert(0_u64) += 1;
            fs::remove_dir_all(directory).expect("seeded crash cleanup");
        }

        assert_eq!(coverage.len(), 16);
        for boundary in SeededCrashBoundary::ALL {
            for position in SeededCrashPosition::ALL {
                assert_eq!(coverage.get(&(boundary, position)), Some(&8));
            }
        }
    }
}
