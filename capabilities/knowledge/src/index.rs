//! Disposable SQLite index derived exclusively from canonical knowledge records.

use std::fmt::Write;

use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{
    KnowledgeError, KnowledgeRecord, KnowledgeRecordId, KnowledgeRecordKind, validate_import,
};

const MAX_INDEX_RECORDS: u64 = 100_000;
const MAX_QUERY_BYTES: usize = 512;
const MAX_QUERY_RESULTS: u32 = 1_000;

/// Bounded lexical index result without canonical Markdown bytes.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KnowledgeIndexHit {
    /// Stable canonical record identity.
    pub record_id: KnowledgeRecordId,
    /// Closed canonical record kind.
    pub kind: KnowledgeRecordKind,
    /// User-visible title.
    pub title: String,
    /// Digest of the exact indexed canonical record value.
    pub canonical_record_sha256: String,
}

/// Deterministic result of one complete index rebuild.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KnowledgeIndexReport {
    /// Number of indexed canonical records.
    pub record_count: u64,
    /// Number of indexed stable relationships.
    pub relationship_count: u64,
    /// Digest of the canonical input set.
    pub canonical_set_sha256: String,
    /// Digest of the complete derived projection.
    pub index_sha256: String,
    /// Explicit non-authoritative classification.
    pub canonical: bool,
}

/// Content-free disposable-index failure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KnowledgeIndexError {
    /// Canonical input or query is invalid.
    InvalidInput,
    /// SQLite rejected schema or one atomic operation.
    StorageFailed,
    /// A derived record is malformed or no longer hash-bound.
    CorruptIndex,
}

impl KnowledgeIndexError {
    /// Returns a stable content-free error code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidInput => "knowledge.index.input_invalid",
            Self::StorageFailed => "knowledge.index.storage_failed",
            Self::CorruptIndex => "knowledge.index.corrupt",
        }
    }
}

impl std::fmt::Display for KnowledgeIndexError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.code())
    }
}

impl std::error::Error for KnowledgeIndexError {}

impl From<KnowledgeError> for KnowledgeIndexError {
    fn from(_: KnowledgeError) -> Self {
        Self::InvalidInput
    }
}

/// Disposable in-memory SQLite search projection.
pub struct KnowledgeIndex {
    connection: Connection,
}

impl KnowledgeIndex {
    /// Creates an empty index with no path or filesystem authority.
    pub fn in_memory() -> Result<Self, KnowledgeIndexError> {
        let connection =
            Connection::open_in_memory().map_err(|_| KnowledgeIndexError::StorageFailed)?;
        connection
            .execute_batch(
                "PRAGMA foreign_keys = ON;
                 PRAGMA trusted_schema = OFF;
                 CREATE TABLE index_metadata (
                   key TEXT PRIMARY KEY NOT NULL,
                   value TEXT NOT NULL
                 ) STRICT;
                 CREATE TABLE knowledge_records (
                   record_id TEXT PRIMARY KEY NOT NULL,
                   kind TEXT NOT NULL,
                   title TEXT NOT NULL,
                   canonical_record_sha256 TEXT NOT NULL CHECK(length(canonical_record_sha256)=64),
                   record_json BLOB NOT NULL
                 ) STRICT;
                 CREATE TABLE knowledge_links (
                   source_id TEXT NOT NULL REFERENCES knowledge_records(record_id) ON DELETE CASCADE,
                   kind TEXT NOT NULL,
                   target_id TEXT NOT NULL REFERENCES knowledge_records(record_id),
                   PRIMARY KEY(source_id, kind, target_id)
                 ) STRICT;
                 INSERT INTO index_metadata(key, value) VALUES
                   ('authority', 'derived_only'),
                   ('schema_version', '1');",
            )
            .map_err(|_| KnowledgeIndexError::StorageFailed)?;
        Ok(Self { connection })
    }

    /// Atomically replaces the complete derived index from canonical records.
    pub fn rebuild(
        &mut self,
        records: &[KnowledgeRecord],
    ) -> Result<KnowledgeIndexReport, KnowledgeIndexError> {
        let import = validate_import(records)?;
        if import.record_count > MAX_INDEX_RECORDS {
            return Err(KnowledgeIndexError::InvalidInput);
        }
        let mut sorted: Vec<&KnowledgeRecord> = records.iter().collect();
        sorted.sort_by(|left, right| left.record_id.cmp(&right.record_id));
        let transaction = self
            .connection
            .transaction()
            .map_err(|_| KnowledgeIndexError::StorageFailed)?;
        transaction
            .execute("DELETE FROM knowledge_links", [])
            .and_then(|_| transaction.execute("DELETE FROM knowledge_records", []))
            .map_err(|_| KnowledgeIndexError::StorageFailed)?;
        for record in &sorted {
            let record_json =
                serde_json::to_vec(record).map_err(|_| KnowledgeIndexError::InvalidInput)?;
            transaction
                .execute(
                    "INSERT INTO knowledge_records(
                       record_id, kind, title, canonical_record_sha256, record_json
                     ) VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![
                        record.record_id.as_str(),
                        kind_wire(record.kind),
                        record.title,
                        sha256(&record_json),
                        record_json,
                    ],
                )
                .map_err(|_| KnowledgeIndexError::StorageFailed)?;
        }
        for relationship in &import.relationships {
            transaction
                .execute(
                    "INSERT INTO knowledge_links(source_id, kind, target_id) VALUES (?1, ?2, ?3)",
                    params![
                        relationship.source_id.as_str(),
                        link_wire(relationship.kind),
                        relationship.target_id.as_str(),
                    ],
                )
                .map_err(|_| KnowledgeIndexError::StorageFailed)?;
        }
        transaction
            .commit()
            .map_err(|_| KnowledgeIndexError::StorageFailed)?;
        let index_sha256 = self.projection_sha256()?;
        Ok(KnowledgeIndexReport {
            record_count: import.record_count,
            relationship_count: import.relationships.len() as u64,
            canonical_set_sha256: import.import_sha256,
            index_sha256,
            canonical: false,
        })
    }

    /// Removes the complete derived projection while retaining only schema metadata.
    pub fn clear(&mut self) -> Result<(), KnowledgeIndexError> {
        let transaction = self
            .connection
            .transaction()
            .map_err(|_| KnowledgeIndexError::StorageFailed)?;
        transaction
            .execute("DELETE FROM knowledge_links", [])
            .and_then(|_| transaction.execute("DELETE FROM knowledge_records", []))
            .map_err(|_| KnowledgeIndexError::StorageFailed)?;
        transaction
            .commit()
            .map_err(|_| KnowledgeIndexError::StorageFailed)
    }

    /// Returns one hash-verified derived record by stable identity.
    pub fn record(
        &self,
        record_id: &KnowledgeRecordId,
    ) -> Result<Option<KnowledgeRecord>, KnowledgeIndexError> {
        let mut statement = self
            .connection
            .prepare(
                "SELECT record_json, canonical_record_sha256
                 FROM knowledge_records WHERE record_id=?1",
            )
            .map_err(|_| KnowledgeIndexError::StorageFailed)?;
        let mut rows = statement
            .query([record_id.as_str()])
            .map_err(|_| KnowledgeIndexError::StorageFailed)?;
        let Some(row) = rows
            .next()
            .map_err(|_| KnowledgeIndexError::StorageFailed)?
        else {
            return Ok(None);
        };
        let bytes: Vec<u8> = row.get(0).map_err(|_| KnowledgeIndexError::CorruptIndex)?;
        let expected: String = row.get(1).map_err(|_| KnowledgeIndexError::CorruptIndex)?;
        if sha256(&bytes) != expected {
            return Err(KnowledgeIndexError::CorruptIndex);
        }
        let record: KnowledgeRecord =
            serde_json::from_slice(&bytes).map_err(|_| KnowledgeIndexError::CorruptIndex)?;
        crate::validate_record(&record).map_err(|_| KnowledgeIndexError::CorruptIndex)?;
        if &record.record_id != record_id {
            return Err(KnowledgeIndexError::CorruptIndex);
        }
        Ok(Some(record))
    }

    /// Performs bounded deterministic lexical search over title and canonical record JSON.
    pub fn search(
        &self,
        query: &str,
        limit: u32,
    ) -> Result<Vec<KnowledgeIndexHit>, KnowledgeIndexError> {
        if query.trim().is_empty()
            || query.len() > MAX_QUERY_BYTES
            || limit == 0
            || limit > MAX_QUERY_RESULTS
        {
            return Err(KnowledgeIndexError::InvalidInput);
        }
        let mut statement = self
            .connection
            .prepare(
                "SELECT record_id, kind, title, canonical_record_sha256, record_json
                 FROM knowledge_records
                 WHERE instr(lower(title), lower(?1)) > 0
                    OR instr(lower(CAST(record_json AS TEXT)), lower(?1)) > 0
                 ORDER BY record_id ASC LIMIT ?2",
            )
            .map_err(|_| KnowledgeIndexError::StorageFailed)?;
        let rows = statement
            .query_map(params![query, limit], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, Vec<u8>>(4)?,
                ))
            })
            .map_err(|_| KnowledgeIndexError::StorageFailed)?;
        let mut hits = Vec::new();
        for row in rows {
            let (identity, kind, title, expected, bytes) =
                row.map_err(|_| KnowledgeIndexError::CorruptIndex)?;
            if sha256(&bytes) != expected {
                return Err(KnowledgeIndexError::CorruptIndex);
            }
            hits.push(KnowledgeIndexHit {
                record_id: KnowledgeRecordId::parse(identity)
                    .map_err(|_| KnowledgeIndexError::CorruptIndex)?,
                kind: parse_kind(&kind).ok_or(KnowledgeIndexError::CorruptIndex)?,
                title,
                canonical_record_sha256: expected,
            });
        }
        Ok(hits)
    }

    /// Returns the exact derived table closure for authority-boundary review.
    pub fn table_names(&self) -> Result<Vec<String>, KnowledgeIndexError> {
        let mut statement = self
            .connection
            .prepare(
                "SELECT name FROM sqlite_schema
                 WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name",
            )
            .map_err(|_| KnowledgeIndexError::StorageFailed)?;
        statement
            .query_map([], |row| row.get(0))
            .map_err(|_| KnowledgeIndexError::StorageFailed)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| KnowledgeIndexError::StorageFailed)
    }

    /// Returns the fixed non-authoritative index classification.
    pub fn authority_class(&self) -> Result<String, KnowledgeIndexError> {
        self.connection
            .query_row(
                "SELECT value FROM index_metadata WHERE key='authority'",
                [],
                |row| row.get(0),
            )
            .map_err(|_| KnowledgeIndexError::CorruptIndex)
    }

    fn projection_sha256(&self) -> Result<String, KnowledgeIndexError> {
        let mut statement = self
            .connection
            .prepare(
                "SELECT record_id, canonical_record_sha256
                 FROM knowledge_records ORDER BY record_id",
            )
            .map_err(|_| KnowledgeIndexError::StorageFailed)?;
        let rows = statement
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .map_err(|_| KnowledgeIndexError::StorageFailed)?;
        let mut material = Vec::new();
        for row in rows {
            let (identity, digest) = row.map_err(|_| KnowledgeIndexError::StorageFailed)?;
            material.extend_from_slice(identity.as_bytes());
            material.push(0);
            material.extend_from_slice(digest.as_bytes());
            material.push(b'\n');
        }
        Ok(sha256(&material))
    }
}

fn kind_wire(kind: KnowledgeRecordKind) -> &'static str {
    match kind {
        KnowledgeRecordKind::Person => "person",
        KnowledgeRecordKind::Organization => "organization",
        KnowledgeRecordKind::Project => "project",
        KnowledgeRecordKind::Meeting => "meeting",
        KnowledgeRecordKind::Task => "task",
        KnowledgeRecordKind::Decision => "decision",
        KnowledgeRecordKind::Commitment => "commitment",
        KnowledgeRecordKind::Document => "document",
        KnowledgeRecordKind::Correspondence => "correspondence",
        KnowledgeRecordKind::Deadline => "deadline",
        KnowledgeRecordKind::Approval => "approval",
        KnowledgeRecordKind::Risk => "risk",
        KnowledgeRecordKind::Question => "question",
        KnowledgeRecordKind::Handoff => "handoff",
    }
}

fn parse_kind(value: &str) -> Option<KnowledgeRecordKind> {
    crate::knowledge_schemas()
        .iter()
        .map(|schema| schema.kind)
        .find(|kind| kind_wire(*kind) == value)
}

fn link_wire(kind: crate::KnowledgeLinkKind) -> &'static str {
    match kind {
        crate::KnowledgeLinkKind::Related => "related",
        crate::KnowledgeLinkKind::OwnedBy => "owned_by",
        crate::KnowledgeLinkKind::About => "about",
        crate::KnowledgeLinkKind::Supports => "supports",
        crate::KnowledgeLinkKind::DependsOn => "depends_on",
        crate::KnowledgeLinkKind::Supersedes => "supersedes",
        crate::KnowledgeLinkKind::FollowUpTo => "follow_up_to",
    }
}

fn sha256(value: &[u8]) -> String {
    let mut output = String::with_capacity(64);
    for byte in Sha256::digest(value) {
        write!(&mut output, "{byte:02x}").expect("writing to String cannot fail");
    }
    output
}

#[cfg(test)]
mod tests {
    use agentmage_kernel_contracts::DataSensitivity;

    use super::*;
    use crate::{
        KNOWLEDGE_SCHEMA_VERSION, KnowledgeField, KnowledgePrivacy, KnowledgeRetention,
        KnowledgeRetentionKind, knowledge_schema,
    };

    fn record(identity: &str, kind: KnowledgeRecordKind, title: &str) -> KnowledgeRecord {
        KnowledgeRecord {
            schema_version: KNOWLEDGE_SCHEMA_VERSION,
            record_id: KnowledgeRecordId::parse(identity).expect("identity"),
            kind,
            title: title.to_owned(),
            privacy: KnowledgePrivacy::Private,
            sensitivity: DataSensitivity::Durable,
            retention: KnowledgeRetention {
                kind: KnowledgeRetentionKind::UntilSupersededOrDeleted,
                expires_at: None,
            },
            created_at: "2026-08-14T00:00:00Z".to_owned(),
            updated_at: "2026-08-14T00:00:00Z".to_owned(),
            last_verified_at: None,
            fields: knowledge_schema(kind)
                .required_fields
                .iter()
                .map(|name| KnowledgeField {
                    name: (*name).to_owned(),
                    value: "searchable fixture".to_owned(),
                })
                .collect(),
            links: Vec::new(),
            tags: vec!["fixture".to_owned()],
            evidence: Vec::new(),
        }
    }

    #[test]
    fn rebuild_clear_and_rebuild_preserve_exact_projection_without_input_mutation() {
        let records = vec![
            record(
                "knowledge-person-001",
                KnowledgeRecordKind::Person,
                "Person",
            ),
            record(
                "knowledge-project-001",
                KnowledgeRecordKind::Project,
                "Project",
            ),
        ];
        let original = records.clone();
        let mut index = KnowledgeIndex::in_memory().expect("index");
        let first = index.rebuild(&records).expect("rebuild");
        assert!(!first.canonical);
        index.clear().expect("clear");
        assert!(index.search("fixture", 10).expect("search").is_empty());
        let second = index.rebuild(&records).expect("rebuild");
        assert_eq!(first, second);
        assert_eq!(records, original);
    }

    #[test]
    fn schema_contains_only_disposable_knowledge_tables() {
        let index = KnowledgeIndex::in_memory().expect("index");
        assert_eq!(
            index.table_names().expect("tables"),
            vec!["index_metadata", "knowledge_links", "knowledge_records"]
        );
        assert_eq!(index.authority_class().expect("authority"), "derived_only");
    }

    #[test]
    fn lookup_and_search_are_bounded_stable_and_hash_verified() {
        let records = vec![
            record(
                "knowledge-person-002",
                KnowledgeRecordKind::Person,
                "Second",
            ),
            record("knowledge-person-001", KnowledgeRecordKind::Person, "First"),
        ];
        let mut index = KnowledgeIndex::in_memory().expect("index");
        index.rebuild(&records).expect("rebuild");
        let hits = index.search("searchable", 10).expect("search");
        assert_eq!(hits.len(), 2);
        assert!(hits[0].record_id < hits[1].record_id);
        assert_eq!(
            index
                .record(&hits[0].record_id)
                .expect("lookup")
                .expect("record")
                .record_id,
            hits[0].record_id
        );
        for (query, limit) in [("", 1), ("x", 0), ("x", MAX_QUERY_RESULTS + 1)] {
            assert_eq!(
                index.search(query, limit),
                Err(KnowledgeIndexError::InvalidInput)
            );
        }
    }

    #[test]
    fn corrupted_projection_is_rejected_and_canonical_input_rebuilds_it() {
        let record = record(
            "knowledge-person-001",
            KnowledgeRecordKind::Person,
            "Person",
        );
        let records = vec![record.clone()];
        let mut index = KnowledgeIndex::in_memory().expect("index");
        let expected = index.rebuild(&records).expect("rebuild");
        index
            .connection
            .execute(
                "UPDATE knowledge_records SET record_json=X'00' WHERE record_id=?1",
                [record.record_id.as_str()],
            )
            .expect("fault injection");
        assert_eq!(
            index.record(&record.record_id),
            Err(KnowledgeIndexError::CorruptIndex)
        );
        assert_eq!(records, vec![record]);
        assert_eq!(index.rebuild(&records).expect("rebuild"), expected);
    }
}
