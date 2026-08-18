//! Disposable SQLite cache keyed by every repository-map validity dimension.

use std::fmt::Write;

use agentmage_kernel_contracts::WorkspacePath;
use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{RepositoryFileRecord, grammar_set_sha256, verify_repository_file_record};

const MAX_CACHE_ENTRIES: u64 = 250_000;

/// Exact cache key required before a structural record may be returned or cited.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RepositoryMapCacheKey {
    /// Exact workspace-relative path.
    pub path: WorkspacePath,
    /// Exact source content digest.
    pub content_sha256: String,
    /// Exact branch/commit/worktree Git identity digest.
    pub git_identity_sha256: String,
    /// Exact grammar descriptor identity.
    pub grammar_sha256: String,
    /// Exact parser runtime version.
    pub parser_version: String,
    /// Exact policy revision.
    pub policy_sha256: String,
}

impl RepositoryMapCacheKey {
    /// Constructs the only exact key admitted for one verified file record.
    pub fn for_record(record: &RepositoryFileRecord) -> Result<Self, RepositoryMapCacheError> {
        if !verify_repository_file_record(record) {
            return Err(RepositoryMapCacheError::InvalidInput);
        }
        let (grammar_sha256, parser_version) = record.structure.as_ref().map_or_else(
            || (grammar_set_sha256(), "inventory-v1".to_owned()),
            |structure| {
                (
                    structure.grammar_sha256.clone(),
                    crate::grammar_descriptor(structure.language).parser_version,
                )
            },
        );
        Ok(Self {
            path: record.path.clone(),
            content_sha256: record.content_sha256.clone(),
            git_identity_sha256: record_git_identity_sha256(record),
            grammar_sha256,
            parser_version,
            policy_sha256: record.policy_sha256.clone(),
        })
    }

    /// Returns canonical JSON bytes for encrypted derivative persistence.
    pub fn canonical_json(&self) -> Result<Vec<u8>, RepositoryMapCacheError> {
        validate_key(self)?;
        serde_json::to_vec(self).map_err(|_| RepositoryMapCacheError::InvalidInput)
    }

    /// Returns SHA-256 of the complete canonical key representation.
    pub fn key_sha256(&self) -> Result<String, RepositoryMapCacheError> {
        key_sha256(self)
    }
}

/// Disposable in-process SQLite repository-map cache.
pub struct RepositoryMapCache {
    connection: Connection,
    max_entries: u64,
}

/// Content-free cache failure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RepositoryMapCacheError {
    /// Cache key, record, or capacity is malformed.
    InvalidInput,
    /// SQLite rejected schema creation or one atomic operation.
    StorageFailed,
    /// Serialized cached content was malformed or no longer hash-bound.
    CorruptRecord,
}

impl RepositoryMapCacheError {
    /// Stable content-free failure code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidInput => "repository.cache.input_invalid",
            Self::StorageFailed => "repository.cache.storage_failed",
            Self::CorruptRecord => "repository.cache.record_corrupt",
        }
    }
}

impl RepositoryMapCache {
    /// Creates an ephemeral SQLite cache with no path or persistence authority.
    pub fn in_memory(max_entries: u64) -> Result<Self, RepositoryMapCacheError> {
        if max_entries == 0 || max_entries > MAX_CACHE_ENTRIES {
            return Err(RepositoryMapCacheError::InvalidInput);
        }
        let connection =
            Connection::open_in_memory().map_err(|_| RepositoryMapCacheError::StorageFailed)?;
        connection
            .execute_batch(
                "PRAGMA foreign_keys = ON;
                 CREATE TABLE repository_map_cache (
                   key_sha256 TEXT PRIMARY KEY NOT NULL CHECK(length(key_sha256) = 64),
                   path_json BLOB NOT NULL,
                   content_sha256 TEXT NOT NULL CHECK(length(content_sha256) = 64),
                   git_identity_sha256 TEXT NOT NULL CHECK(length(git_identity_sha256) = 64),
                   grammar_sha256 TEXT NOT NULL CHECK(length(grammar_sha256) = 64),
                   parser_version TEXT NOT NULL,
                   policy_sha256 TEXT NOT NULL CHECK(length(policy_sha256) = 64),
                   record_json BLOB NOT NULL,
                   record_sha256 TEXT NOT NULL CHECK(length(record_sha256) = 64)
                 ) STRICT;",
            )
            .map_err(|_| RepositoryMapCacheError::StorageFailed)?;
        Ok(Self {
            connection,
            max_entries,
        })
    }

    /// Atomically inserts or replaces one record under its complete exact key.
    pub fn put(
        &mut self,
        key: &RepositoryMapCacheKey,
        record: &RepositoryFileRecord,
    ) -> Result<(), RepositoryMapCacheError> {
        validate_key_record(key, record)?;
        let key_sha256 = key_sha256(key)?;
        let path_json =
            serde_json::to_vec(&key.path).map_err(|_| RepositoryMapCacheError::InvalidInput)?;
        let record_json =
            serde_json::to_vec(record).map_err(|_| RepositoryMapCacheError::InvalidInput)?;
        let transaction = self
            .connection
            .transaction()
            .map_err(|_| RepositoryMapCacheError::StorageFailed)?;
        transaction
            .execute(
                "INSERT INTO repository_map_cache (
                   key_sha256, path_json, content_sha256, git_identity_sha256,
                   grammar_sha256, parser_version, policy_sha256, record_json, record_sha256
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
                 ON CONFLICT(key_sha256) DO UPDATE SET
                   path_json=excluded.path_json,
                   content_sha256=excluded.content_sha256,
                   git_identity_sha256=excluded.git_identity_sha256,
                   grammar_sha256=excluded.grammar_sha256,
                   parser_version=excluded.parser_version,
                   policy_sha256=excluded.policy_sha256,
                   record_json=excluded.record_json,
                   record_sha256=excluded.record_sha256",
                params![
                    key_sha256,
                    path_json,
                    key.content_sha256,
                    key.git_identity_sha256,
                    key.grammar_sha256,
                    key.parser_version,
                    key.policy_sha256,
                    record_json,
                    record.record_sha256,
                ],
            )
            .map_err(|_| RepositoryMapCacheError::StorageFailed)?;
        let count: i64 = transaction
            .query_row("SELECT count(*) FROM repository_map_cache", [], |row| {
                row.get(0)
            })
            .map_err(|_| RepositoryMapCacheError::StorageFailed)?;
        if u64::try_from(count).map_or(true, |count| count > self.max_entries) {
            return Err(RepositoryMapCacheError::StorageFailed);
        }
        transaction
            .commit()
            .map_err(|_| RepositoryMapCacheError::StorageFailed)
    }

    /// Returns one record only when every key dimension matches exactly.
    pub fn get(
        &self,
        key: &RepositoryMapCacheKey,
    ) -> Result<Option<RepositoryFileRecord>, RepositoryMapCacheError> {
        validate_key(key)?;
        let key_sha256 = key_sha256(key)?;
        let mut statement = self
            .connection
            .prepare(
                "SELECT record_json, record_sha256 FROM repository_map_cache WHERE key_sha256=?1",
            )
            .map_err(|_| RepositoryMapCacheError::StorageFailed)?;
        let mut rows = statement
            .query([key_sha256])
            .map_err(|_| RepositoryMapCacheError::StorageFailed)?;
        let Some(row) = rows
            .next()
            .map_err(|_| RepositoryMapCacheError::StorageFailed)?
        else {
            return Ok(None);
        };
        let bytes: Vec<u8> = row
            .get(0)
            .map_err(|_| RepositoryMapCacheError::CorruptRecord)?;
        let expected: String = row
            .get(1)
            .map_err(|_| RepositoryMapCacheError::CorruptRecord)?;
        let record: RepositoryFileRecord =
            serde_json::from_slice(&bytes).map_err(|_| RepositoryMapCacheError::CorruptRecord)?;
        if !verify_repository_file_record(&record)
            || record.record_sha256 != expected
            || record.path != key.path
            || record.content_sha256 != key.content_sha256
        {
            return Err(RepositoryMapCacheError::CorruptRecord);
        }
        Ok(Some(record))
    }

    /// Removes every record whose complete key is absent from the current key set.
    pub fn invalidate_except(
        &mut self,
        current: &[RepositoryMapCacheKey],
    ) -> Result<u64, RepositoryMapCacheError> {
        if current.len() > self.max_entries as usize {
            return Err(RepositoryMapCacheError::InvalidInput);
        }
        let mut hashes = current
            .iter()
            .map(|key| {
                validate_key(key)?;
                key_sha256(key)
            })
            .collect::<Result<Vec<_>, _>>()?;
        hashes.sort();
        hashes.dedup();
        let transaction = self
            .connection
            .transaction()
            .map_err(|_| RepositoryMapCacheError::StorageFailed)?;
        transaction
            .execute(
                "CREATE TEMP TABLE current_keys (key_sha256 TEXT PRIMARY KEY) STRICT",
                [],
            )
            .map_err(|_| RepositoryMapCacheError::StorageFailed)?;
        {
            let mut statement = transaction
                .prepare("INSERT INTO current_keys (key_sha256) VALUES (?1)")
                .map_err(|_| RepositoryMapCacheError::StorageFailed)?;
            for hash in hashes {
                statement
                    .execute([hash])
                    .map_err(|_| RepositoryMapCacheError::StorageFailed)?;
            }
        }
        let removed = transaction
            .execute(
                "DELETE FROM repository_map_cache
                 WHERE key_sha256 NOT IN (SELECT key_sha256 FROM current_keys)",
                [],
            )
            .map_err(|_| RepositoryMapCacheError::StorageFailed)? as u64;
        transaction
            .execute("DROP TABLE current_keys", [])
            .map_err(|_| RepositoryMapCacheError::StorageFailed)?;
        transaction
            .commit()
            .map_err(|_| RepositoryMapCacheError::StorageFailed)?;
        Ok(removed)
    }

    /// Returns the exact number of currently valid cached records.
    pub fn len(&self) -> Result<u64, RepositoryMapCacheError> {
        let count: i64 = self
            .connection
            .query_row("SELECT count(*) FROM repository_map_cache", [], |row| {
                row.get(0)
            })
            .map_err(|_| RepositoryMapCacheError::StorageFailed)?;
        u64::try_from(count).map_err(|_| RepositoryMapCacheError::StorageFailed)
    }

    /// Reports whether the cache contains no records.
    pub fn is_empty(&self) -> Result<bool, RepositoryMapCacheError> {
        self.len().map(|count| count == 0)
    }
}

fn validate_key_record(
    key: &RepositoryMapCacheKey,
    record: &RepositoryFileRecord,
) -> Result<(), RepositoryMapCacheError> {
    validate_key(key)?;
    let expected = RepositoryMapCacheKey::for_record(record)?;
    if key.path != record.path
        || key.content_sha256 != record.content_sha256
        || key.git_identity_sha256 != expected.git_identity_sha256
        || key.grammar_sha256 != expected.grammar_sha256
        || key.parser_version != expected.parser_version
        || key.policy_sha256 != expected.policy_sha256
        || !verify_repository_file_record(record)
    {
        return Err(RepositoryMapCacheError::InvalidInput);
    }
    Ok(())
}

fn record_git_identity_sha256(record: &RepositoryFileRecord) -> String {
    serde_json::to_vec(&(
        &record.repository_sha256,
        &record.worktree_sha256,
        &record.branch,
        &record.commit_id,
    ))
    .map(|bytes| sha256_hex(&bytes))
    .unwrap_or_else(|_| sha256_hex(b"repository-cache-git-identity-failed"))
}

fn validate_key(key: &RepositoryMapCacheKey) -> Result<(), RepositoryMapCacheError> {
    if !is_sha256(&key.content_sha256)
        || !is_sha256(&key.git_identity_sha256)
        || !is_sha256(&key.grammar_sha256)
        || !is_sha256(&key.policy_sha256)
        || key.parser_version.is_empty()
        || key.parser_version.len() > 64
        || !key
            .parser_version
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"._-".contains(&byte))
    {
        return Err(RepositoryMapCacheError::InvalidInput);
    }
    Ok(())
}

fn key_sha256(key: &RepositoryMapCacheKey) -> Result<String, RepositoryMapCacheError> {
    serde_json::to_vec(key)
        .map(|bytes| sha256_hex(&bytes))
        .map_err(|_| RepositoryMapCacheError::InvalidInput)
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(64);
    for byte in Sha256::digest(bytes) {
        write!(&mut output, "{byte:02x}").expect("writing to String cannot fail");
    }
    output
}

#[cfg(test)]
mod tests {
    use agentmage_kernel_contracts::{WorkspaceId, WorkspacePath};

    use super::{RepositoryMapCache, RepositoryMapCacheKey};
    use crate::{
        GitTrackedState, RepositoryFileInput, RepositoryFileRecord, RepositoryMapInput,
        build_repository_map,
    };

    fn key(record: &RepositoryFileRecord) -> RepositoryMapCacheKey {
        RepositoryMapCacheKey::for_record(record).expect("key")
    }

    fn record(marker: char, path: &[&str]) -> RepositoryFileRecord {
        let content = format!("fn run_{marker}() {{}}\n").into_bytes();
        build_repository_map(RepositoryMapInput {
            workspace_id: WorkspaceId::from_raw("workspace-cache"),
            repository_sha256: "a".repeat(64),
            worktree_sha256: "b".repeat(64),
            branch: Some("main".to_owned()),
            commit_id: "e".repeat(40),
            policy_sha256: "d".repeat(64),
            freshness_sha256: "f".repeat(64),
            files: vec![RepositoryFileInput {
                path: path.iter().map(|part| (*part).to_owned()).collect(),
                size_bytes: content.len() as u64,
                content_sha256: super::sha256_hex(&content),
                content: Some(content),
                object_kind: crate::RepositoryObjectKind::RegularFile,
                git_state: GitTrackedState::TrackedClean,
                policy_excluded: false,
                generated: false,
                vendored: false,
            }],
        })
        .expect("map record")
        .files
        .remove(0)
    }

    #[test]
    fn exact_complete_key_controls_retrieval_and_changed_inputs_invalidate_first() {
        let mut cache = RepositoryMapCache::in_memory(8).expect("cache");
        let record = record('a', &["src", "lib.rs"]);
        let key = key(&record);
        cache.put(&key, &record).expect("put");
        assert_eq!(cache.len().expect("len"), 1);
        assert_eq!(cache.get(&key).expect("get"), Some(record.clone()));
        for changed in ['1', '2', '3'] {
            let mut changed_key = key.clone();
            changed_key.git_identity_sha256 = changed.to_string().repeat(64);
            assert_eq!(cache.get(&changed_key).expect("miss"), None);
            assert_eq!(
                cache.invalidate_except(&[changed_key]).expect("invalidate"),
                1
            );
            assert!(cache.is_empty().expect("empty"));
            cache.put(&key, &record).expect("restore");
        }
    }

    #[test]
    fn invalid_capacity_key_record_and_over_capacity_write_fail_closed() {
        assert!(RepositoryMapCache::in_memory(0).is_err());
        let mut cache = RepositoryMapCache::in_memory(1).expect("cache");
        let first_record = record('a', &["src", "lib.rs"]);
        let first_key = key(&first_record);
        let mut invalid = first_key.clone();
        invalid.policy_sha256 = "bad".to_owned();
        assert!(cache.put(&invalid, &first_record).is_err());
        let mismatched = record('b', &["src", "other.rs"]);
        assert!(cache.put(&first_key, &mismatched).is_err());
        cache.put(&first_key, &first_record).expect("first");
        let second_record = record('b', &["src", "other.rs"]);
        let second_key = key(&second_record);
        assert!(cache.put(&second_key, &second_record).is_err());
        assert_eq!(cache.len().expect("rollback count"), 1);
    }

    #[test]
    fn every_workspace_path_content_git_grammar_parser_and_policy_change_misses_then_invalidates() {
        let exact_record = record('a', &["src", "lib.rs"]);
        let exact = key(&exact_record);
        let stable_record = record('z', &["tests", "stable.rs"]);
        let stable_key = key(&stable_record);
        let mut changes = Vec::new();

        let mut workspace = exact.clone();
        workspace.path = WorkspacePath::new(
            WorkspaceId::from_raw("workspace-other"),
            vec!["src".to_owned(), "lib.rs".to_owned()],
        )
        .expect("path");
        changes.push(workspace);

        let mut path = exact.clone();
        path.path = WorkspacePath::new(
            WorkspaceId::from_raw("workspace-cache"),
            vec!["src".to_owned(), "other.rs".to_owned()],
        )
        .expect("path");
        changes.push(path);

        let mut content = exact.clone();
        content.content_sha256 = "1".repeat(64);
        changes.push(content);
        let mut git = exact.clone();
        git.git_identity_sha256 = "2".repeat(64);
        changes.push(git);
        let mut grammar = exact.clone();
        grammar.grammar_sha256 = "3".repeat(64);
        changes.push(grammar);
        let mut parser = exact.clone();
        parser.parser_version = "0.26.13".to_owned();
        changes.push(parser);
        let mut policy = exact.clone();
        policy.policy_sha256 = "4".repeat(64);
        changes.push(policy);

        for changed in changes {
            let mut cache = RepositoryMapCache::in_memory(8).expect("cache");
            cache.put(&exact, &exact_record).expect("put");
            cache.put(&stable_key, &stable_record).expect("stable put");
            assert_eq!(cache.get(&changed).expect("miss"), None);
            assert_eq!(
                cache
                    .invalidate_except(&[changed, stable_key.clone()])
                    .expect("invalidate"),
                1
            );
            assert_eq!(cache.len().expect("one retained"), 1);
            assert_eq!(
                cache.get(&stable_key).expect("stable get"),
                Some(stable_record.clone())
            );
        }
    }
}
