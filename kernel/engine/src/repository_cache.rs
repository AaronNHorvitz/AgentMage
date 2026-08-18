//! Encrypted, derivative repository-map cache records and exact invalidation.

use std::collections::BTreeSet;

use rusqlite::{OptionalExtension, TransactionBehavior, params};
use sha2::{Digest, Sha256};

use crate::operational_store::{OperationalStore, OperationalStoreError};

const MAX_CACHE_ENTRIES_PER_SCOPE: usize = 100_000;
const MAX_CACHE_ENTRIES_TOTAL: usize = 250_000;
const MAX_KEY_BYTES: usize = 64 * 1024;
const MAX_RECORD_BYTES: usize = 4 * 1024 * 1024;
const MAX_RETENTION_MILLISECONDS: u64 = 30 * 24 * 60 * 60 * 1_000;

/// Exact validated derivative cache row supplied by an approved host.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RepositoryCacheRecord {
    scope_sha256: String,
    key_sha256: String,
    key_json: Vec<u8>,
    record_sha256: String,
    payload_sha256: String,
    record_json: Vec<u8>,
    created_at_epoch_ms: u64,
    expires_at_epoch_ms: u64,
}

impl RepositoryCacheRecord {
    /// Validates exact key and payload bytes plus bounded retention before persistence.
    pub fn new(
        scope_sha256: String,
        key_json: Vec<u8>,
        record_sha256: String,
        record_json: Vec<u8>,
        created_at_epoch_ms: u64,
        expires_at_epoch_ms: u64,
    ) -> Result<Self, OperationalStoreError> {
        let retention = expires_at_epoch_ms.checked_sub(created_at_epoch_ms);
        if !is_sha256(&scope_sha256)
            || !is_sha256(&record_sha256)
            || key_json.is_empty()
            || key_json.len() > MAX_KEY_BYTES
            || record_json.is_empty()
            || record_json.len() > MAX_RECORD_BYTES
            || created_at_epoch_ms == 0
            || retention.is_none_or(|value| value > MAX_RETENTION_MILLISECONDS)
        {
            return Err(OperationalStoreError::LifecycleRejected);
        }
        Ok(Self {
            scope_sha256,
            key_sha256: sha256(&key_json),
            key_json,
            record_sha256,
            payload_sha256: sha256(&record_json),
            record_json,
            created_at_epoch_ms,
            expires_at_epoch_ms,
        })
    }

    /// Returns the exact complete-key digest.
    #[must_use]
    pub fn key_sha256(&self) -> &str {
        &self.key_sha256
    }

    /// Returns the exact cache-scope digest.
    #[must_use]
    pub fn scope_sha256(&self) -> &str {
        &self.scope_sha256
    }
}

/// Verified exact cache hit returned without authority semantics.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RepositoryCacheHit {
    /// Internal integrity digest retained by the repository-map record.
    pub record_sha256: String,
    /// Exact serialized repository-map record bytes.
    pub record_json: Vec<u8>,
}

/// Content-free result of one exact scope reconciliation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RepositoryCacheReconciliation {
    /// Rows retained because their complete key is current and unexpired.
    pub retained: u64,
    /// Rows removed because their key is stale or their retention expired.
    pub removed: u64,
}

impl OperationalStore {
    /// Inserts or replaces one encrypted derivative row without changing canonical authority.
    pub fn put_repository_cache_record(
        &mut self,
        record: &RepositoryCacheRecord,
    ) -> Result<(), OperationalStoreError> {
        if self.poisoned {
            return Err(OperationalStoreError::Poisoned);
        }
        verify_record(record)?;
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| OperationalStoreError::PersistenceFailure)?;
        transaction
            .execute(
                "DELETE FROM repository_map_cache
                 WHERE scope_sha256 = ?1 AND expires_at_epoch_ms <= ?2",
                params![record.scope_sha256, to_i64(record.created_at_epoch_ms)?],
            )
            .map_err(|_| OperationalStoreError::PersistenceFailure)?;
        transaction
            .execute(
                "INSERT INTO repository_map_cache(
                     scope_sha256, key_sha256, key_json, record_sha256,
                     payload_sha256, record_json, created_at_epoch_ms, expires_at_epoch_ms
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
                 ON CONFLICT(scope_sha256, key_sha256) DO UPDATE SET
                     key_json=excluded.key_json,
                     record_sha256=excluded.record_sha256,
                     payload_sha256=excluded.payload_sha256,
                     record_json=excluded.record_json,
                     created_at_epoch_ms=excluded.created_at_epoch_ms,
                     expires_at_epoch_ms=excluded.expires_at_epoch_ms",
                params![
                    record.scope_sha256,
                    record.key_sha256,
                    record.key_json,
                    record.record_sha256,
                    record.payload_sha256,
                    record.record_json,
                    to_i64(record.created_at_epoch_ms)?,
                    to_i64(record.expires_at_epoch_ms)?,
                ],
            )
            .map_err(|_| OperationalStoreError::PersistenceFailure)?;
        let count: i64 = transaction
            .query_row(
                "SELECT COUNT(*) FROM repository_map_cache WHERE scope_sha256 = ?1",
                [&record.scope_sha256],
                |row| row.get(0),
            )
            .map_err(|_| OperationalStoreError::PersistenceFailure)?;
        if usize::try_from(count).map_or(true, |count| count > MAX_CACHE_ENTRIES_PER_SCOPE) {
            return Err(OperationalStoreError::PersistenceFailure);
        }
        let total: i64 = transaction
            .query_row("SELECT COUNT(*) FROM repository_map_cache", [], |row| {
                row.get(0)
            })
            .map_err(|_| OperationalStoreError::PersistenceFailure)?;
        if usize::try_from(total).map_or(true, |count| count > MAX_CACHE_ENTRIES_TOTAL) {
            return Err(OperationalStoreError::PersistenceFailure);
        }
        transaction
            .commit()
            .map_err(|_| OperationalStoreError::PersistenceFailure)
    }

    /// Reads one exact unexpired derivative row and verifies both retained hashes.
    pub fn repository_cache_record(
        &self,
        scope_sha256: &str,
        key_sha256: &str,
        now_epoch_ms: u64,
    ) -> Result<Option<RepositoryCacheHit>, OperationalStoreError> {
        if self.poisoned {
            return Err(OperationalStoreError::Poisoned);
        }
        if !is_sha256(scope_sha256) || !is_sha256(key_sha256) || now_epoch_ms == 0 {
            return Err(OperationalStoreError::LifecycleRejected);
        }
        let row = self
            .connection
            .query_row(
                "SELECT key_json, record_sha256, payload_sha256, record_json,
                        created_at_epoch_ms, expires_at_epoch_ms
                 FROM repository_map_cache
                 WHERE scope_sha256 = ?1 AND key_sha256 = ?2
                       AND expires_at_epoch_ms > ?3",
                params![scope_sha256, key_sha256, to_i64(now_epoch_ms)?],
                |row| {
                    Ok((
                        row.get::<_, Vec<u8>>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, Vec<u8>>(3)?,
                        row.get::<_, i64>(4)?,
                        row.get::<_, i64>(5)?,
                    ))
                },
            )
            .optional()
            .map_err(|_| OperationalStoreError::IntegrityFailure)?;
        let Some((key_json, record_sha256, payload_sha256, record_json, created, expires)) = row
        else {
            return Ok(None);
        };
        if sha256(&key_json) != key_sha256
            || !is_sha256(&record_sha256)
            || sha256(&record_json) != payload_sha256
            || created <= 0
            || expires < created
        {
            return Err(OperationalStoreError::IntegrityFailure);
        }
        Ok(Some(RepositoryCacheHit {
            record_sha256,
            record_json,
        }))
    }

    /// Removes every stale or expired row in one scope before later retrieval or citation.
    pub fn reconcile_repository_cache(
        &mut self,
        scope_sha256: &str,
        current_key_sha256: &[String],
        now_epoch_ms: u64,
    ) -> Result<RepositoryCacheReconciliation, OperationalStoreError> {
        if self.poisoned
            || !is_sha256(scope_sha256)
            || now_epoch_ms == 0
            || current_key_sha256.len() > MAX_CACHE_ENTRIES_PER_SCOPE
        {
            return Err(if self.poisoned {
                OperationalStoreError::Poisoned
            } else {
                OperationalStoreError::LifecycleRejected
            });
        }
        let current = current_key_sha256.iter().collect::<BTreeSet<_>>();
        if current.len() != current_key_sha256.len() || current.iter().any(|key| !is_sha256(key)) {
            return Err(OperationalStoreError::LifecycleRejected);
        }
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| OperationalStoreError::PersistenceFailure)?;
        transaction
            .execute(
                "CREATE TEMP TABLE current_repository_cache_keys (
                     key_sha256 TEXT PRIMARY KEY CHECK(length(key_sha256) = 64)
                 ) STRICT",
                [],
            )
            .map_err(|_| OperationalStoreError::PersistenceFailure)?;
        {
            let mut statement = transaction
                .prepare("INSERT INTO current_repository_cache_keys(key_sha256) VALUES (?1)")
                .map_err(|_| OperationalStoreError::PersistenceFailure)?;
            for key in current {
                statement
                    .execute([key])
                    .map_err(|_| OperationalStoreError::PersistenceFailure)?;
            }
        }
        let removed = transaction
            .execute(
                "DELETE FROM repository_map_cache
                 WHERE scope_sha256 = ?1 AND (
                     expires_at_epoch_ms <= ?2 OR key_sha256 NOT IN (
                         SELECT key_sha256 FROM current_repository_cache_keys
                     )
                 )",
                params![scope_sha256, to_i64(now_epoch_ms)?],
            )
            .map_err(|_| OperationalStoreError::PersistenceFailure)?;
        let retained: i64 = transaction
            .query_row(
                "SELECT COUNT(*) FROM repository_map_cache WHERE scope_sha256 = ?1",
                [scope_sha256],
                |row| row.get(0),
            )
            .map_err(|_| OperationalStoreError::PersistenceFailure)?;
        transaction
            .execute("DROP TABLE current_repository_cache_keys", [])
            .map_err(|_| OperationalStoreError::PersistenceFailure)?;
        transaction
            .commit()
            .map_err(|_| OperationalStoreError::PersistenceFailure)?;
        Ok(RepositoryCacheReconciliation {
            retained: u64::try_from(retained)
                .map_err(|_| OperationalStoreError::IntegrityFailure)?,
            removed: removed as u64,
        })
    }
}

pub(crate) fn verify_all(store: &OperationalStore) -> Result<(), OperationalStoreError> {
    let mut statement = store
        .connection
        .prepare(
            "SELECT scope_sha256, key_sha256, key_json, record_sha256,
                    payload_sha256, record_json, created_at_epoch_ms, expires_at_epoch_ms
             FROM repository_map_cache ORDER BY scope_sha256, key_sha256",
        )
        .map_err(|_| OperationalStoreError::IntegrityFailure)?;
    let mut rows = statement
        .query([])
        .map_err(|_| OperationalStoreError::IntegrityFailure)?;
    let mut count = 0_usize;
    while let Some(row) = rows
        .next()
        .map_err(|_| OperationalStoreError::IntegrityFailure)?
    {
        count = count
            .checked_add(1)
            .filter(|count| *count <= MAX_CACHE_ENTRIES_TOTAL)
            .ok_or(OperationalStoreError::IntegrityFailure)?;
        let scope: String = row
            .get(0)
            .map_err(|_| OperationalStoreError::IntegrityFailure)?;
        let key: String = row
            .get(1)
            .map_err(|_| OperationalStoreError::IntegrityFailure)?;
        let key_json: Vec<u8> = row
            .get(2)
            .map_err(|_| OperationalStoreError::IntegrityFailure)?;
        let record: String = row
            .get(3)
            .map_err(|_| OperationalStoreError::IntegrityFailure)?;
        let payload: String = row
            .get(4)
            .map_err(|_| OperationalStoreError::IntegrityFailure)?;
        let record_json: Vec<u8> = row
            .get(5)
            .map_err(|_| OperationalStoreError::IntegrityFailure)?;
        let created: i64 = row
            .get(6)
            .map_err(|_| OperationalStoreError::IntegrityFailure)?;
        let expires: i64 = row
            .get(7)
            .map_err(|_| OperationalStoreError::IntegrityFailure)?;
        if !is_sha256(&scope)
            || sha256(&key_json) != key
            || !is_sha256(&record)
            || sha256(&record_json) != payload
            || created <= 0
            || expires < created
        {
            return Err(OperationalStoreError::IntegrityFailure);
        }
    }
    Ok(())
}

fn verify_record(record: &RepositoryCacheRecord) -> Result<(), OperationalStoreError> {
    if !is_sha256(&record.scope_sha256)
        || sha256(&record.key_json) != record.key_sha256
        || !is_sha256(&record.record_sha256)
        || sha256(&record.record_json) != record.payload_sha256
        || record.key_json.is_empty()
        || record.key_json.len() > MAX_KEY_BYTES
        || record.record_json.is_empty()
        || record.record_json.len() > MAX_RECORD_BYTES
        || record.created_at_epoch_ms == 0
        || record
            .expires_at_epoch_ms
            .checked_sub(record.created_at_epoch_ms)
            .is_none_or(|value| value > MAX_RETENTION_MILLISECONDS)
    {
        return Err(OperationalStoreError::LifecycleRejected);
    }
    Ok(())
}

fn to_i64(value: u64) -> Result<i64, OperationalStoreError> {
    i64::try_from(value).map_err(|_| OperationalStoreError::LifecycleRejected)
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn sha256(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(64);
    for byte in Sha256::digest(bytes) {
        use std::fmt::Write as _;
        write!(&mut output, "{byte:02x}").expect("writing to String cannot fail");
    }
    output
}
