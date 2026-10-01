//! Durable documentation pack catalog in the operational store (CAP-22,
//! Decision 0130).
//!
//! The store keeps one documentation catalog: each kept version's manifest
//! bytes and file bytes, the day it was superseded, every deletion record and
//! a head that names the catalog's revision, its last change and the digest of
//! its state. The store does not interpret a manifest; the catalog's owner
//! restores and re-verifies the catalog from what it loads. A commit names the
//! revision it read and the complete next state. In one immediate transaction
//! it appends the new deletion records, deletes each removed version with its
//! files, marks newly superseded versions, inserts new versions with their
//! files and advances the head. It refuses every other change: a changed
//! version or file, a removal without its deletion record, a deletion record
//! for a version that stays, an edited deletion history or a day that goes
//! backwards. Store triggers forbid the transitions Decision 0130 lists; a
//! row the triggers admit but the commit never writes, such as a file added
//! to a kept version, a deletion record for a version that stays or a version
//! inserted already superseded, fails the head digest at the next open
//! (Decision 0132).
//!
//! Every store open checks the catalog's metadata against its head digest;
//! every load also checks each file's bytes against its digest. A mismatch
//! poisons the store. As for the other store owners, the store's key
//! authenticates writers: the head detects a torn or partial change, not a
//! rewrite by a key holder. A catalog grants no authority and performs no
//! effect.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use rusqlite::{OptionalExtension as _, TransactionBehavior, params};
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::operational_store::OperationalStore;

/// Identity of the one documentation catalog a store keeps.
const CATALOG_ID: &str = "documentation";
/// Most kept versions.
pub const MAX_DOC_PACK_STORED_VERSIONS: usize = 4_096;
/// Most kept files across every version.
pub const MAX_DOC_PACK_STORED_FILES: usize = 100_000;
/// Most file bytes across every version.
pub const MAX_DOC_PACK_STORED_BYTES: u64 = 256 * 1024 * 1024;
/// Largest stored manifest.
pub const MAX_DOC_PACK_MANIFEST_BYTES: usize = 4 * 1024 * 1024;
/// Largest stored file.
pub const MAX_DOC_PACK_FILE_BYTES: usize = 8 * 1024 * 1024;
/// Most deletion records.
pub const MAX_DOC_PACK_DELETIONS: usize = 100_000;
const MAX_PACK_ID_BYTES: usize = 64;
const MAX_PATH_BYTES: usize = 4_096;

/// Identity of one kept or deleted pack version.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
pub struct DocPackVersionKey {
    /// Pack identity.
    pub pack_id: String,
    /// Major version.
    pub major: u32,
    /// Minor version.
    pub minor: u32,
    /// Patch version.
    pub patch: u32,
}

/// One file of a stored version.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StoredDocPackFile {
    /// Relative path inside the pack.
    pub path: String,
    /// SHA-256 of the bytes.
    pub sha256: String,
    /// Exact bytes.
    pub content: Vec<u8>,
}

/// One kept version.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StoredDocPackVersion {
    /// Pack and version.
    pub key: DocPackVersionKey,
    /// The manifest's own digest.
    pub manifest_sha256: String,
    /// The manifest's exact bytes.
    pub manifest_json: Vec<u8>,
    /// ISO date it was superseded; none while it is current.
    pub superseded_on: Option<String>,
    /// Files sorted by path.
    pub files: Vec<StoredDocPackFile>,
}

/// Why a version was deleted.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StoredDocPackDeletionReason {
    /// The person deleted it.
    Person,
    /// Its retention after supersession ended.
    Retention,
}

impl StoredDocPackDeletionReason {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Person => "person",
            Self::Retention => "retention",
        }
    }

    fn parse(value: &str) -> Option<Self> {
        match value {
            "person" => Some(Self::Person),
            "retention" => Some(Self::Retention),
            _ => None,
        }
    }
}

/// One content-free deletion record.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StoredDocPackDeletion {
    /// Pack and version.
    pub key: DocPackVersionKey,
    /// The deleted manifest's digest.
    pub manifest_sha256: String,
    /// ISO date of deletion.
    pub deleted_on: String,
    /// Why.
    pub reason: StoredDocPackDeletionReason,
}

/// The complete content of a catalog.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DocPackCatalogContents {
    /// Every kept version, ordered by key.
    pub versions: Vec<StoredDocPackVersion>,
    /// Every deletion, in order.
    pub deletions: Vec<StoredDocPackDeletion>,
    /// ISO date of the last change; none before the first.
    pub last_changed_on: Option<String>,
}

/// A catalog as the store keeps it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StoredDocPackCatalog {
    /// Revision of the head; zero before the first commit.
    pub revision: u64,
    /// The catalog's content.
    pub contents: DocPackCatalogContents,
    /// Digest of the state the head names.
    pub state_sha256: String,
}

/// Content-free documentation pack store failure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DocPackStoreError {
    /// The next state is malformed or out of order; nothing was written.
    InvalidInput,
    /// The catalog changed since the revision the caller read; nothing was
    /// written.
    Stale,
    /// The next state is not a change the catalog's own operations make;
    /// nothing was written.
    InvalidChange,
    /// The next state exceeds a store bound; nothing was written.
    ResourceLimit,
    /// Retained rows do not form the catalog the head names. The store is
    /// poisoned.
    Integrity,
    /// The store could not read or commit; nothing was written.
    Storage,
    /// The store is poisoned or its lock is unavailable.
    Unavailable,
}

/// Shared handle to the documentation catalog of one operational store. It
/// takes the store's lock for each operation, like the job ledgers.
#[derive(Clone)]
pub struct DurableDocPackCatalog {
    store: Arc<Mutex<OperationalStore>>,
}

impl std::fmt::Debug for DurableDocPackCatalog {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("DurableDocPackCatalog")
    }
}

impl DurableDocPackCatalog {
    pub(crate) const fn new(store: Arc<Mutex<OperationalStore>>) -> Self {
        Self { store }
    }

    /// The whole catalog, with every file checked against its digest.
    pub fn load(&self) -> Result<StoredDocPackCatalog, DocPackStoreError> {
        self.with_store(|store| load(store, true))
    }

    /// Replaces the catalog read at `read_revision` with `next` in one
    /// transaction, and returns what the store then holds.
    pub fn commit(
        &self,
        read_revision: u64,
        next: &DocPackCatalogContents,
    ) -> Result<StoredDocPackCatalog, DocPackStoreError> {
        validate_contents(next)?;
        self.with_store(|store| commit(store, read_revision, next))
    }

    fn with_store<T>(
        &self,
        operation: impl FnOnce(&mut OperationalStore) -> Result<T, DocPackStoreError>,
    ) -> Result<T, DocPackStoreError> {
        let mut store = self
            .store
            .lock()
            .map_err(|_| DocPackStoreError::Unavailable)?;
        if store.poisoned {
            return Err(DocPackStoreError::Unavailable);
        }
        let result = operation(&mut store);
        if matches!(result, Err(DocPackStoreError::Integrity)) {
            store.poisoned = true;
        }
        result
    }
}

/// Checks the catalog's metadata against its head at store open, without
/// reading file bytes.
pub(crate) fn verify_all(store: &OperationalStore) -> Result<(), DocPackStoreError> {
    load(store, false).map(|_| ())
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn valid_date(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.len() == 10
        && bytes.iter().enumerate().all(|(index, byte)| {
            if matches!(index, 4 | 7) {
                *byte == b'-'
            } else {
                byte.is_ascii_digit()
            }
        })
}

fn valid_key(key: &DocPackVersionKey) -> bool {
    !key.pack_id.is_empty()
        && key.pack_id.len() <= MAX_PACK_ID_BYTES
        && key.pack_id.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'.' | b'-')
        })
}

fn valid_path(path: &str) -> bool {
    !path.is_empty()
        && path.len() <= MAX_PATH_BYTES
        && !path.contains('\0')
        && path
            .split('/')
            .all(|component| !component.is_empty() && component != "." && component != "..")
}

fn sha256_hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    let mut output = String::with_capacity(64);
    for byte in Sha256::digest(bytes) {
        let _ = write!(output, "{byte:02x}");
    }
    output
}

/// Every shape and bound of a next state, before the store is touched.
fn validate_contents(contents: &DocPackCatalogContents) -> Result<(), DocPackStoreError> {
    let invalid = DocPackStoreError::InvalidInput;
    if contents.versions.len() > MAX_DOC_PACK_STORED_VERSIONS
        || contents.deletions.len() > MAX_DOC_PACK_DELETIONS
    {
        return Err(DocPackStoreError::ResourceLimit);
    }
    if contents
        .last_changed_on
        .as_deref()
        .is_some_and(|day| !valid_date(day))
        || contents.last_changed_on.is_none()
            && (!contents.versions.is_empty() || !contents.deletions.is_empty())
    {
        return Err(invalid);
    }
    let mut files = 0_usize;
    let mut bytes = 0_u64;
    for (index, version) in contents.versions.iter().enumerate() {
        if !valid_key(&version.key)
            || index > 0 && contents.versions[index - 1].key >= version.key
            || !valid_sha256(&version.manifest_sha256)
            || version.manifest_json.is_empty()
            || version
                .superseded_on
                .as_deref()
                .is_some_and(|day| !valid_date(day))
            || version.files.is_empty()
        {
            return Err(invalid);
        }
        if version.manifest_json.len() > MAX_DOC_PACK_MANIFEST_BYTES {
            return Err(DocPackStoreError::ResourceLimit);
        }
        for (position, file) in version.files.iter().enumerate() {
            if !valid_path(&file.path)
                || position > 0 && version.files[position - 1].path >= file.path
                || file.content.is_empty()
                || sha256_hex(&file.content) != file.sha256
            {
                return Err(invalid);
            }
            if file.content.len() > MAX_DOC_PACK_FILE_BYTES {
                return Err(DocPackStoreError::ResourceLimit);
            }
            bytes = bytes
                .checked_add(file.content.len() as u64)
                .ok_or(DocPackStoreError::ResourceLimit)?;
        }
        files = files.saturating_add(version.files.len());
    }
    if files > MAX_DOC_PACK_STORED_FILES || bytes > MAX_DOC_PACK_STORED_BYTES {
        return Err(DocPackStoreError::ResourceLimit);
    }
    for deletion in &contents.deletions {
        if !valid_key(&deletion.key)
            || !valid_sha256(&deletion.manifest_sha256)
            || !valid_date(&deletion.deleted_on)
        {
            return Err(invalid);
        }
    }
    Ok(())
}

/// One stored version's metadata, which the head digest covers.
#[derive(Serialize)]
struct VersionDigest<'a> {
    pack_id: &'a str,
    version: [u32; 3],
    manifest_sha256: &'a str,
    manifest_json_sha256: String,
    superseded_on: Option<&'a str>,
    deletions_before: u64,
    files: Vec<FileDigest<'a>>,
}

#[derive(Serialize)]
struct FileDigest<'a> {
    path: &'a str,
    sha256: &'a str,
    byte_len: u64,
}

#[derive(Serialize)]
struct DeletionDigest<'a> {
    sequence: u64,
    pack_id: &'a str,
    version: [u32; 3],
    manifest_sha256: &'a str,
    deleted_on: &'a str,
    reason: StoredDocPackDeletionReason,
}

#[derive(Serialize)]
struct StateDigest<'a> {
    record_type: &'static str,
    revision: u64,
    last_changed_on: Option<&'a str>,
    versions: Vec<VersionDigest<'a>>,
    deletions: Vec<DeletionDigest<'a>>,
}

/// The metadata of one version row as loaded or about to be stored.
struct VersionMeta {
    key: DocPackVersionKey,
    manifest_sha256: String,
    manifest_json: Vec<u8>,
    superseded_on: Option<String>,
    deletions_before: u64,
    files: Vec<(String, String, u64)>,
}

fn state_sha256(
    revision: u64,
    last_changed_on: Option<&str>,
    versions: &[VersionMeta],
    deletions: &[StoredDocPackDeletion],
) -> Result<String, DocPackStoreError> {
    let digest = StateDigest {
        record_type: "agentmage-doc-pack-catalog-state",
        revision,
        last_changed_on,
        versions: versions
            .iter()
            .map(|version| VersionDigest {
                pack_id: &version.key.pack_id,
                version: [version.key.major, version.key.minor, version.key.patch],
                manifest_sha256: &version.manifest_sha256,
                manifest_json_sha256: sha256_hex(&version.manifest_json),
                superseded_on: version.superseded_on.as_deref(),
                deletions_before: version.deletions_before,
                files: version
                    .files
                    .iter()
                    .map(|(path, sha256, byte_len)| FileDigest {
                        path,
                        sha256,
                        byte_len: *byte_len,
                    })
                    .collect(),
            })
            .collect(),
        deletions: deletions
            .iter()
            .enumerate()
            .map(|(index, deletion)| DeletionDigest {
                sequence: index as u64 + 1,
                pack_id: &deletion.key.pack_id,
                version: [deletion.key.major, deletion.key.minor, deletion.key.patch],
                manifest_sha256: &deletion.manifest_sha256,
                deleted_on: &deletion.deleted_on,
                reason: deletion.reason,
            })
            .collect(),
    };
    serde_json::to_vec(&digest)
        .map(|bytes| sha256_hex(&bytes))
        .map_err(|_| DocPackStoreError::Storage)
}

struct Head {
    revision: u64,
    last_changed_on: Option<String>,
    state_sha256: String,
}

fn read_head(connection: &rusqlite::Connection) -> Result<Option<Head>, DocPackStoreError> {
    connection
        .query_row(
            "SELECT revision, last_changed_on, state_sha256 FROM doc_pack_catalog_heads
             WHERE catalog_id = ?1",
            params![CATALOG_ID],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, Option<String>>(1)?,
                    row.get::<_, String>(2)?,
                ))
            },
        )
        .optional()
        .map_err(|_| DocPackStoreError::Storage)?
        .map(|(revision, last_changed_on, state_sha256)| {
            Ok(Head {
                revision: u64::try_from(revision).map_err(|_| DocPackStoreError::Integrity)?,
                last_changed_on,
                state_sha256,
            })
        })
        .transpose()
}

fn read_versions(connection: &rusqlite::Connection) -> Result<Vec<VersionMeta>, DocPackStoreError> {
    let limit = i64::try_from(MAX_DOC_PACK_STORED_VERSIONS + 1).unwrap_or(i64::MAX);
    let rows = connection
        .prepare(
            "SELECT pack_id, major, minor, patch, manifest_sha256, manifest_json, superseded_on,
                    deletions_before
             FROM doc_pack_versions ORDER BY pack_id, major, minor, patch LIMIT ?1",
        )
        .and_then(|mut statement| {
            statement
                .query_map(params![limit], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, i64>(1)?,
                        row.get::<_, i64>(2)?,
                        row.get::<_, i64>(3)?,
                        row.get::<_, String>(4)?,
                        row.get::<_, Vec<u8>>(5)?,
                        row.get::<_, Option<String>>(6)?,
                        row.get::<_, i64>(7)?,
                    ))
                })?
                .collect::<Result<Vec<_>, _>>()
        })
        .map_err(|_| DocPackStoreError::Storage)?;
    if rows.len() > MAX_DOC_PACK_STORED_VERSIONS {
        return Err(DocPackStoreError::Integrity);
    }
    let integer = |value: i64| u32::try_from(value).map_err(|_| DocPackStoreError::Integrity);
    let mut versions = Vec::with_capacity(rows.len());
    for (pack_id, major, minor, patch, manifest_sha256, manifest_json, superseded_on, before) in
        rows
    {
        let key = DocPackVersionKey {
            pack_id,
            major: integer(major)?,
            minor: integer(minor)?,
            patch: integer(patch)?,
        };
        let files = connection
            .prepare(
                "SELECT path, sha256, length(content) FROM doc_pack_files
                 WHERE pack_id = ?1 AND major = ?2 AND minor = ?3 AND patch = ?4 ORDER BY path",
            )
            .and_then(|mut statement| {
                statement
                    .query_map(params![key.pack_id, major, minor, patch], |row| {
                        Ok((
                            row.get::<_, String>(0)?,
                            row.get::<_, String>(1)?,
                            row.get::<_, i64>(2)?,
                        ))
                    })?
                    .collect::<Result<Vec<_>, _>>()
            })
            .map_err(|_| DocPackStoreError::Storage)?
            .into_iter()
            .map(|(path, sha256, length)| {
                Ok((
                    path,
                    sha256,
                    u64::try_from(length).map_err(|_| DocPackStoreError::Integrity)?,
                ))
            })
            .collect::<Result<Vec<_>, DocPackStoreError>>()?;
        versions.push(VersionMeta {
            key,
            manifest_sha256,
            manifest_json,
            superseded_on,
            deletions_before: u64::try_from(before).map_err(|_| DocPackStoreError::Integrity)?,
            files,
        });
    }
    Ok(versions)
}

fn read_deletions(
    connection: &rusqlite::Connection,
) -> Result<Vec<StoredDocPackDeletion>, DocPackStoreError> {
    let limit = i64::try_from(MAX_DOC_PACK_DELETIONS + 1).unwrap_or(i64::MAX);
    let rows = connection
        .prepare(
            "SELECT sequence, pack_id, major, minor, patch, manifest_sha256, deleted_on, reason
             FROM doc_pack_deletions ORDER BY sequence LIMIT ?1",
        )
        .and_then(|mut statement| {
            statement
                .query_map(params![limit], |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, i64>(2)?,
                        row.get::<_, i64>(3)?,
                        row.get::<_, i64>(4)?,
                        row.get::<_, String>(5)?,
                        row.get::<_, String>(6)?,
                        row.get::<_, String>(7)?,
                    ))
                })?
                .collect::<Result<Vec<_>, _>>()
        })
        .map_err(|_| DocPackStoreError::Storage)?;
    if rows.len() > MAX_DOC_PACK_DELETIONS {
        return Err(DocPackStoreError::Integrity);
    }
    let integer = |value: i64| u32::try_from(value).map_err(|_| DocPackStoreError::Integrity);
    rows.into_iter()
        .enumerate()
        .map(
            |(
                index,
                (sequence, pack_id, major, minor, patch, manifest_sha256, deleted_on, reason),
            )| {
                if u64::try_from(sequence).ok() != Some(index as u64 + 1) {
                    return Err(DocPackStoreError::Integrity);
                }
                Ok(StoredDocPackDeletion {
                    key: DocPackVersionKey {
                        pack_id,
                        major: integer(major)?,
                        minor: integer(minor)?,
                        patch: integer(patch)?,
                    },
                    manifest_sha256,
                    deleted_on,
                    reason: StoredDocPackDeletionReason::parse(&reason)
                        .ok_or(DocPackStoreError::Integrity)?,
                })
            },
        )
        .collect()
}

/// Loads the catalog and checks it against its head. With `content`, every
/// file's bytes are read and checked against their digest.
fn load(
    store: &OperationalStore,
    content: bool,
) -> Result<StoredDocPackCatalog, DocPackStoreError> {
    let connection = &store.connection;
    let head = read_head(connection)?;
    let versions = read_versions(connection)?;
    let deletions = read_deletions(connection)?;
    let Some(head) = head else {
        // Before the first commit the catalog is empty.
        if !versions.is_empty() || !deletions.is_empty() {
            return Err(DocPackStoreError::Integrity);
        }
        let files: i64 = connection
            .query_row("SELECT COUNT(*) FROM doc_pack_files", [], |row| row.get(0))
            .map_err(|_| DocPackStoreError::Storage)?;
        if files != 0 {
            return Err(DocPackStoreError::Integrity);
        }
        return Ok(StoredDocPackCatalog {
            revision: 0,
            contents: DocPackCatalogContents::default(),
            state_sha256: state_sha256(0, None, &[], &[])?,
        });
    };
    let digest = state_sha256(
        head.revision,
        head.last_changed_on.as_deref(),
        &versions,
        &deletions,
    )?;
    if digest != head.state_sha256 {
        return Err(DocPackStoreError::Integrity);
    }
    let mut contents = DocPackCatalogContents {
        versions: Vec::with_capacity(versions.len()),
        deletions,
        last_changed_on: head.last_changed_on,
    };
    for version in versions {
        let mut files = Vec::with_capacity(version.files.len());
        for (path, sha256, _) in version.files {
            let bytes = if content {
                connection
                    .query_row(
                        "SELECT content FROM doc_pack_files
                         WHERE pack_id = ?1 AND major = ?2 AND minor = ?3 AND patch = ?4
                           AND path = ?5",
                        params![
                            version.key.pack_id,
                            i64::from(version.key.major),
                            i64::from(version.key.minor),
                            i64::from(version.key.patch),
                            path
                        ],
                        |row| row.get::<_, Vec<u8>>(0),
                    )
                    .map_err(|_| DocPackStoreError::Storage)?
            } else {
                Vec::new()
            };
            files.push(StoredDocPackFile {
                path,
                sha256,
                content: bytes,
            });
        }
        contents.versions.push(StoredDocPackVersion {
            key: version.key,
            manifest_sha256: version.manifest_sha256,
            manifest_json: version.manifest_json,
            superseded_on: version.superseded_on,
            files,
        });
    }
    // Every stored shape must be one a commit could have written, each file
    // matching its digest.
    if content {
        validate_contents(&contents).map_err(|_| DocPackStoreError::Integrity)?;
    }
    Ok(StoredDocPackCatalog {
        revision: head.revision,
        contents,
        state_sha256: head.state_sha256,
    })
}

fn sql_integer(value: u64) -> Result<i64, DocPackStoreError> {
    i64::try_from(value).map_err(|_| DocPackStoreError::Integrity)
}

fn commit(
    store: &mut OperationalStore,
    read_revision: u64,
    next: &DocPackCatalogContents,
) -> Result<StoredDocPackCatalog, DocPackStoreError> {
    let transaction = store
        .connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|_| DocPackStoreError::Storage)?;
    let head = read_head(&transaction)?;
    let current_versions = read_versions(&transaction)?;
    let current_deletions = read_deletions(&transaction)?;
    let current_revision = head.as_ref().map_or(0, |head| head.revision);
    if current_revision != read_revision {
        return Err(DocPackStoreError::Stale);
    }
    let current_last = head.as_ref().and_then(|head| head.last_changed_on.clone());
    // Days never go backwards, and a catalog that changed keeps a day.
    match (&current_last, &next.last_changed_on) {
        (Some(current), Some(next)) if next < current => {
            return Err(DocPackStoreError::InvalidChange);
        }
        (Some(_), None) => return Err(DocPackStoreError::InvalidChange),
        _ => {}
    }
    // Deletions are append-only.
    if next.deletions.len() < current_deletions.len()
        || next.deletions[..current_deletions.len()] != current_deletions[..]
    {
        return Err(DocPackStoreError::InvalidChange);
    }
    let appended = &next.deletions[current_deletions.len()..];
    // Appended deletions are dated in order, on or after the last change
    // and no later than the next one.
    if appended
        .windows(2)
        .any(|pair| pair[1].deleted_on < pair[0].deleted_on)
        || appended.iter().any(|deletion| {
            current_last
                .as_deref()
                .is_some_and(|last| deletion.deleted_on.as_str() < last)
        })
        || appended.first().is_some_and(|first| {
            current_deletions
                .last()
                .is_some_and(|last| first.deleted_on < last.deleted_on)
        })
        || appended.iter().any(|deletion| {
            next.last_changed_on
                .as_deref()
                .is_none_or(|last| deletion.deleted_on.as_str() > last)
        })
    {
        return Err(DocPackStoreError::InvalidChange);
    }
    let current: BTreeMap<&DocPackVersionKey, &VersionMeta> = current_versions
        .iter()
        .map(|version| (&version.key, version))
        .collect();
    let next_by_key: BTreeMap<&DocPackVersionKey, &StoredDocPackVersion> = next
        .versions
        .iter()
        .map(|version| (&version.key, version))
        .collect();
    // Each removed version has exactly one appended deletion naming it with
    // its manifest digest, and each appended deletion names a removed one.
    let removed = current_versions
        .iter()
        .filter(|version| !next_by_key.contains_key(&version.key))
        .collect::<Vec<_>>();
    let mut named = appended
        .iter()
        .map(|deletion| (&deletion.key, deletion.manifest_sha256.as_str()))
        .collect::<Vec<_>>();
    named.sort();
    let mut expected = removed
        .iter()
        .map(|version| (&version.key, version.manifest_sha256.as_str()))
        .collect::<Vec<_>>();
    expected.sort();
    if named != expected {
        return Err(DocPackStoreError::InvalidChange);
    }
    // A version that stays changes only from current to superseded.
    let mut superseded = Vec::new();
    for version in &next.versions {
        let Some(kept) = current.get(&version.key) else {
            continue;
        };
        let files = version
            .files
            .iter()
            .map(|file| {
                (
                    file.path.clone(),
                    file.sha256.clone(),
                    file.content.len() as u64,
                )
            })
            .collect::<Vec<_>>();
        if kept.manifest_sha256 != version.manifest_sha256
            || kept.manifest_json != version.manifest_json
            || kept.files != files
        {
            return Err(DocPackStoreError::InvalidChange);
        }
        match (&kept.superseded_on, &version.superseded_on) {
            (None, None) => {}
            (Some(kept_day), Some(day)) if kept_day == day => {}
            (None, Some(day))
                if next
                    .last_changed_on
                    .as_deref()
                    .is_some_and(|last| day.as_str() <= last)
                    && current_last
                        .as_deref()
                        .is_none_or(|last| day.as_str() >= last) =>
            {
                superseded.push((&version.key, day));
            }
            _ => return Err(DocPackStoreError::InvalidChange),
        }
    }
    // A version is always current when it is added.
    let added = next
        .versions
        .iter()
        .filter(|version| !current.contains_key(&version.key))
        .collect::<Vec<_>>();
    if added.iter().any(|version| version.superseded_on.is_some()) {
        return Err(DocPackStoreError::InvalidChange);
    }
    let changed = !appended.is_empty() || !superseded.is_empty() || !added.is_empty();
    if !changed && next.last_changed_on == current_last {
        // Nothing to write; the head stays.
        drop(transaction);
        return load(store, true);
    }
    for (index, deletion) in appended.iter().enumerate() {
        transaction
            .execute(
                "INSERT INTO doc_pack_deletions(sequence, pack_id, major, minor, patch,
                     manifest_sha256, deleted_on, reason)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    sql_integer((current_deletions.len() + index + 1) as u64)?,
                    deletion.key.pack_id,
                    i64::from(deletion.key.major),
                    i64::from(deletion.key.minor),
                    i64::from(deletion.key.patch),
                    deletion.manifest_sha256,
                    deletion.deleted_on,
                    deletion.reason.as_str()
                ],
            )
            .map_err(|_| DocPackStoreError::Storage)?;
    }
    for version in &removed {
        let key = &version.key;
        let identity = params![
            key.pack_id,
            i64::from(key.major),
            i64::from(key.minor),
            i64::from(key.patch)
        ];
        transaction
            .execute(
                "DELETE FROM doc_pack_files
                 WHERE pack_id = ?1 AND major = ?2 AND minor = ?3 AND patch = ?4",
                identity,
            )
            .map_err(|_| DocPackStoreError::Storage)?;
        let deleted = transaction
            .execute(
                "DELETE FROM doc_pack_versions
                 WHERE pack_id = ?1 AND major = ?2 AND minor = ?3 AND patch = ?4",
                identity,
            )
            .map_err(|_| DocPackStoreError::Storage)?;
        if deleted != 1 {
            return Err(DocPackStoreError::Integrity);
        }
    }
    for (key, day) in &superseded {
        let updated = transaction
            .execute(
                "UPDATE doc_pack_versions SET superseded_on = ?1
                 WHERE pack_id = ?2 AND major = ?3 AND minor = ?4 AND patch = ?5
                   AND superseded_on IS NULL",
                params![
                    day,
                    key.pack_id,
                    i64::from(key.major),
                    i64::from(key.minor),
                    i64::from(key.patch)
                ],
            )
            .map_err(|_| DocPackStoreError::Storage)?;
        if updated != 1 {
            return Err(DocPackStoreError::Integrity);
        }
    }
    let deletions_before = next.deletions.len() as u64;
    for version in &added {
        let key = &version.key;
        transaction
            .execute(
                "INSERT INTO doc_pack_versions(pack_id, major, minor, patch, manifest_sha256,
                     manifest_json, superseded_on, deletions_before)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    key.pack_id,
                    i64::from(key.major),
                    i64::from(key.minor),
                    i64::from(key.patch),
                    version.manifest_sha256,
                    version.manifest_json,
                    version.superseded_on,
                    sql_integer(deletions_before)?
                ],
            )
            .map_err(|_| DocPackStoreError::Storage)?;
        for file in &version.files {
            transaction
                .execute(
                    "INSERT INTO doc_pack_files(pack_id, major, minor, patch, path, sha256, content)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                    params![
                        key.pack_id,
                        i64::from(key.major),
                        i64::from(key.minor),
                        i64::from(key.patch),
                        file.path,
                        file.sha256,
                        file.content
                    ],
                )
                .map_err(|_| DocPackStoreError::Storage)?;
        }
    }
    // The head names the next revision and the digest of the state now
    // stored, which is read back under the same transaction.
    let revision = current_revision + 1;
    let stored_versions = read_versions(&transaction)?;
    let stored_deletions = read_deletions(&transaction)?;
    let digest = state_sha256(
        revision,
        next.last_changed_on.as_deref(),
        &stored_versions,
        &stored_deletions,
    )?;
    let written = if head.is_some() {
        transaction
            .execute(
                "UPDATE doc_pack_catalog_heads
                 SET revision = ?1, last_changed_on = ?2, state_sha256 = ?3
                 WHERE catalog_id = ?4 AND revision = ?5",
                params![
                    sql_integer(revision)?,
                    next.last_changed_on,
                    digest,
                    CATALOG_ID,
                    sql_integer(read_revision)?
                ],
            )
            .map_err(|_| DocPackStoreError::Storage)?
    } else {
        transaction
            .execute(
                "INSERT INTO doc_pack_catalog_heads(catalog_id, revision, last_changed_on,
                     state_sha256)
                 VALUES (?1, ?2, ?3, ?4)",
                params![
                    CATALOG_ID,
                    sql_integer(revision)?,
                    next.last_changed_on,
                    digest
                ],
            )
            .map_err(|_| DocPackStoreError::Storage)?
    };
    // The head was read under the same lock; any other head means the store
    // changed underneath its only writer.
    if written != 1 {
        return Err(DocPackStoreError::Integrity);
    }
    transaction
        .commit()
        .map_err(|_| DocPackStoreError::Storage)?;
    let stored = load(store, true)?;
    if stored.contents != *next {
        return Err(DocPackStoreError::Integrity);
    }
    Ok(stored)
}

#[cfg(test)]
#[path = "doc_pack_store_tests.rs"]
mod tests;
