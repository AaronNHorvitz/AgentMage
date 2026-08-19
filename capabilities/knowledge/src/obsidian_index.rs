//! Disposable Obsidian index, bounded queries, previews, and watcher updates.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fmt::Write;

use agentmage_kernel_contracts::{WorkspacePath, WorkspaceScopePath};
use rusqlite::{Connection, Transaction, params};
use sha2::{Digest, Sha256};

use crate::{
    KnowledgeFileType, KnowledgeIndexPublication, KnowledgeIndexPublicationState,
    KnowledgeSourceAuthority, KnowledgeSourceDocument, KnowledgeSourceFragment,
    KnowledgeSourceFragmentKind, MarkdownDocument, OBSIDIAN_PARSER_VERSION,
    ObsidianFrontmatterValue, ObsidianParsedNote, ObsidianSourceRange, ObsidianVaultSnapshot,
};

const MAX_QUERY_BYTES: usize = 512;
const MAX_QUERY_RESULTS: u32 = 1_000;
const MAX_TRAVERSAL_DEPTH: u32 = 32;
const MAX_WATCH_EVENTS: usize = 10_000;
const MAX_REPLACEMENT_BYTES: usize = 1024 * 1024;
const MAX_RETRIEVAL_ELEMENTS: usize = 100_000;

/// Current or explicitly historical note classification.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ObsidianTemporalClass {
    /// Current unless explicit metadata or path conventions say otherwise.
    Current,
    /// Historical, archived, or superseded source retained for evidence.
    Historical,
}

/// Closed indexed element class.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ObsidianIndexElementKind {
    /// Display title derived from the first heading or filename.
    Title,
    /// Frontmatter property.
    Property,
    /// ATX heading.
    Heading,
    /// Checkbox task.
    Task,
    /// Inline or frontmatter tag.
    Tag,
    /// Callout marker.
    Callout,
    /// Explicit block identifier.
    Block,
    /// Recognized timestamp.
    Timestamp,
    /// Embedded note or attachment target.
    Embed,
}

/// Closed visible index-conflict class.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ObsidianIndexConflictKind {
    /// Two canonical paths differ only by ASCII case.
    CaseCollision,
    /// Parser reported an unresolved wiki target.
    UnresolvedLink,
    /// Parser reported a target with multiple candidates.
    AmbiguousLink,
}

/// One visible conflict derived from canonical source snapshots.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObsidianIndexConflict {
    /// Conflict class.
    pub kind: ObsidianIndexConflictKind,
    /// Digest of the source path or normalized collision key.
    pub source_sha256: String,
    /// Number of candidate objects involved.
    pub candidate_count: usize,
}

/// One source-traceable deterministic index hit.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObsidianIndexHit {
    /// Canonical vault-relative workspace path.
    pub path: WorkspacePath,
    /// Current or historical classification.
    pub temporal_class: ObsidianTemporalClass,
    /// Optional normalized frontmatter record type.
    pub note_kind: Option<String>,
    /// Exact source-content digest.
    pub content_sha256: String,
    /// Exact parser generation.
    pub parser_version: u16,
    /// Matched element class.
    pub element_kind: ObsidianIndexElementKind,
    /// Exact matched source range.
    pub source_range: ObsidianSourceRange,
    /// Matched bounded text.
    pub text: String,
}

/// Complete result of one atomic disposable-index publication.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObsidianIndexReport {
    /// Monotonic derived projection revision.
    pub revision: u64,
    /// Indexed note count.
    pub note_count: u64,
    /// Verified attachment count.
    pub attachment_count: u64,
    /// Indexed element count.
    pub element_count: u64,
    /// Resolved note-link count.
    pub link_count: u64,
    /// Visible conflict count.
    pub conflict_count: u64,
    /// Digest of the complete source snapshot.
    pub snapshot_sha256: String,
    /// Digest of the complete derived SQL projection.
    pub index_sha256: String,
    /// Fixed false authority marker.
    pub canonical: bool,
}

/// Freshness relation between current canonical snapshots and the derived index.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ObsidianVaultFreshness {
    /// Snapshot digest and parser generation match the published projection.
    Current,
    /// Snapshot differs and must be rebuilt before query or preview use.
    Stale,
}

/// Closed receipt operation class.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ObsidianAccessKind {
    /// Complete index rebuild.
    Rebuild,
    /// Watcher-observed atomic index replacement.
    WatchUpdate,
    /// Bounded lexical query.
    Query,
    /// Current-note reader.
    CurrentReader,
    /// Bounded relationship traversal.
    Traversal,
    /// Per-file change preview.
    Preview,
}

/// Content-free proof of one local derived-index access.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObsidianAccessReceipt {
    /// Monotonic process-local receipt sequence.
    pub sequence: u64,
    /// Operation class.
    pub operation: ObsidianAccessKind,
    /// Published index revision.
    pub index_revision: u64,
    /// Source snapshot digest.
    pub snapshot_sha256: String,
    /// Complete index digest.
    pub index_sha256: String,
    /// Sorted digests of accessed paths, never raw host paths.
    pub accessed_path_sha256: Vec<String>,
    /// Digest binding every receipt field.
    pub receipt_sha256: String,
    /// Always false: no source file mutation exists in this module.
    pub source_files_mutated: bool,
    /// Always false: no external process exists in this module.
    pub external_process_started: bool,
    /// Always false: no network route exists in this module.
    pub network_accessed: bool,
}

/// Atomic update report and receipt.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObsidianIndexUpdate {
    /// Published derived projection.
    pub report: ObsidianIndexReport,
    /// Content-free access receipt.
    pub receipt: ObsidianAccessReceipt,
}

/// Result of reconciling one canonical Markdown write with the disposable index.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObsidianPostWriteIndexResult {
    /// Canonical-first publication decision.
    pub state: KnowledgeIndexPublicationState,
    /// Current projection after reconciliation.
    pub report: ObsidianIndexReport,
    /// Atomic index update only when a canonical commit was exact and current.
    pub update: Option<ObsidianIndexUpdate>,
}

/// Bounded query result and receipt.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObsidianQueryResult {
    /// Stable ordered hits.
    pub hits: Vec<ObsidianIndexHit>,
    /// Content-free access receipt.
    pub receipt: ObsidianAccessReceipt,
}

/// Complete bounded retrieval projection read from the disposable index.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObsidianRetrievalDocuments {
    /// Source-traceable derived documents in canonical path order.
    pub documents: Vec<KnowledgeSourceDocument>,
    /// Content-free proof of the local derived-index read.
    pub receipt: ObsidianAccessReceipt,
}

/// Bounded relationship traversal and receipt.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObsidianTraversalResult {
    /// Stable breadth-first target paths excluding the start path.
    pub paths: Vec<WorkspacePath>,
    /// Content-free access receipt.
    pub receipt: ObsidianAccessReceipt,
}

/// Exact compare-and-swap preview preserving bytes outside one section.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObsidianFileChangePreview {
    /// Canonical source path.
    pub path: WorkspacePath,
    /// Expected exact source digest.
    pub expected_content_sha256: String,
    /// Proposed exact bytes with no apply authority.
    pub proposed_content: Vec<u8>,
    /// Digest of proposed bytes.
    pub proposed_content_sha256: String,
    /// Source range replaced by the proposal.
    pub replaced_range: ObsidianSourceRange,
    /// Digest of untouched prefix bytes.
    pub preserved_prefix_sha256: String,
    /// Digest of untouched suffix bytes.
    pub preserved_suffix_sha256: String,
    /// Fixed false authority marker.
    pub write_enabled: bool,
}

/// Preview result and receipt.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObsidianPreviewResult {
    /// Exact per-file preview.
    pub preview: ObsidianFileChangePreview,
    /// Content-free access receipt.
    pub receipt: ObsidianAccessReceipt,
}

/// Closed local watcher observation class.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ObsidianWatchEventKind {
    /// Newly observed source object.
    Created,
    /// Existing source object changed.
    Modified,
    /// Existing source object disappeared.
    Deleted,
}

/// One already-observed watcher event with no watcher or filesystem handle.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObsidianWatchEvent {
    /// Canonical affected path.
    pub path: WorkspacePath,
    /// Observed event class.
    pub kind: ObsidianWatchEventKind,
    /// Current digest for create/modify; absent for delete.
    pub content_sha256: Option<String>,
}

/// Content-free disposable vault-index failure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ObsidianIndexError {
    /// Input, bounds, state, or preview target is invalid.
    InvalidInput,
    /// Expected index or source revision became stale.
    Stale,
    /// Derived SQLite operation failed.
    StorageFailed,
    /// Derived projection no longer matches its recorded digest.
    CorruptIndex,
}

impl ObsidianIndexError {
    /// Returns a stable redacted failure code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidInput => "knowledge.obsidian.index.input_invalid",
            Self::Stale => "knowledge.obsidian.index.stale",
            Self::StorageFailed => "knowledge.obsidian.index.storage_failed",
            Self::CorruptIndex => "knowledge.obsidian.index.corrupt",
        }
    }
}

impl std::fmt::Display for ObsidianIndexError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.code())
    }
}

impl std::error::Error for ObsidianIndexError {}

/// Disposable in-memory SQLite vault index with no host path or source-write method.
pub struct ObsidianVaultIndex {
    connection: Connection,
    next_receipt_sequence: u64,
}

impl ObsidianVaultIndex {
    /// Creates an empty in-memory derived index.
    pub fn in_memory() -> Result<Self, ObsidianIndexError> {
        let connection =
            Connection::open_in_memory().map_err(|_| ObsidianIndexError::StorageFailed)?;
        connection
            .execute_batch(
                "PRAGMA foreign_keys = ON;
                 PRAGMA trusted_schema = OFF;
                 CREATE TABLE vault_metadata (
                   key TEXT PRIMARY KEY NOT NULL,
                   value TEXT NOT NULL
                 ) STRICT;
                 CREATE TABLE vault_notes (
                   path_key TEXT PRIMARY KEY NOT NULL,
                   path_json BLOB NOT NULL,
                   content_sha256 TEXT NOT NULL CHECK(length(content_sha256)=64),
                   parser_version INTEGER NOT NULL,
                   temporal_class TEXT NOT NULL,
                   note_kind TEXT,
                   title TEXT NOT NULL
                 ) STRICT;
                 CREATE TABLE vault_attachments (
                   path_key TEXT PRIMARY KEY NOT NULL,
                   path_json BLOB NOT NULL,
                   content_sha256 TEXT NOT NULL CHECK(length(content_sha256)=64),
                   byte_count INTEGER NOT NULL
                 ) STRICT;
                 CREATE TABLE vault_elements (
                   path_key TEXT NOT NULL REFERENCES vault_notes(path_key) ON DELETE CASCADE,
                   ordinal INTEGER NOT NULL,
                   kind TEXT NOT NULL,
                   text TEXT NOT NULL,
                   start_line INTEGER NOT NULL,
                   start_column INTEGER NOT NULL,
                   end_line INTEGER NOT NULL,
                   end_column INTEGER NOT NULL,
                   PRIMARY KEY(path_key, ordinal)
                 ) STRICT;
                 CREATE TABLE vault_links (
                   source_key TEXT NOT NULL REFERENCES vault_notes(path_key) ON DELETE CASCADE,
                   target_key TEXT NOT NULL,
                   target_is_attachment INTEGER NOT NULL,
                   line_number INTEGER NOT NULL,
                   PRIMARY KEY(source_key, target_key, line_number)
                 ) STRICT;
                 CREATE TABLE vault_coverage (
                   path_key TEXT NOT NULL REFERENCES vault_notes(path_key) ON DELETE CASCADE,
                   ordinal INTEGER NOT NULL,
                   syntax TEXT NOT NULL,
                   supported INTEGER NOT NULL,
                   start_line INTEGER NOT NULL,
                   start_column INTEGER NOT NULL,
                   end_line INTEGER NOT NULL,
                   end_column INTEGER NOT NULL,
                   PRIMARY KEY(path_key, ordinal)
                 ) STRICT;
                 CREATE TABLE vault_conflicts (
                   ordinal INTEGER PRIMARY KEY NOT NULL,
                   kind TEXT NOT NULL,
                   source_sha256 TEXT NOT NULL CHECK(length(source_sha256)=64),
                   candidate_count INTEGER NOT NULL
                 ) STRICT;
                 INSERT INTO vault_metadata(key, value) VALUES
                   ('authority', 'derived_only'),
                   ('schema_version', '1'),
                   ('parser_version', '2'),
                   ('revision', '0'),
                   ('snapshot_sha256', ''),
                   ('index_sha256', '');",
            )
            .map_err(|_| ObsidianIndexError::StorageFailed)?;
        Ok(Self {
            connection,
            next_receipt_sequence: 1,
        })
    }

    /// Atomically replaces every derived row from one complete parsed snapshot.
    pub fn rebuild(
        &mut self,
        snapshot: &ObsidianVaultSnapshot,
    ) -> Result<ObsidianIndexUpdate, ObsidianIndexError> {
        let revision = self
            .revision()?
            .checked_add(1)
            .ok_or(ObsidianIndexError::InvalidInput)?;
        let report = self.replace_projection(snapshot, revision)?;
        let paths = snapshot
            .notes()
            .iter()
            .map(|note| note.path.clone())
            .chain(snapshot.attachments().iter().map(|item| item.path.clone()))
            .collect::<Vec<_>>();
        let receipt = self.receipt(ObsidianAccessKind::Rebuild, &report, paths.iter())?;
        Ok(ObsidianIndexUpdate { report, receipt })
    }

    /// Returns the fixed non-authoritative index classification.
    pub fn authority_class(&self) -> Result<String, ObsidianIndexError> {
        self.metadata("authority")
    }

    /// Returns the exact table closure for authority review.
    pub fn table_names(&self) -> Result<Vec<String>, ObsidianIndexError> {
        let mut statement = self
            .connection
            .prepare(
                "SELECT name FROM sqlite_schema
                 WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name",
            )
            .map_err(|_| ObsidianIndexError::StorageFailed)?;
        statement
            .query_map([], |row| row.get(0))
            .map_err(|_| ObsidianIndexError::StorageFailed)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| ObsidianIndexError::StorageFailed)
    }

    /// Compares one parsed canonical snapshot with the published projection identity.
    pub fn freshness(
        &self,
        snapshot: &ObsidianVaultSnapshot,
    ) -> Result<ObsidianVaultFreshness, ObsidianIndexError> {
        let expected = obsidian_snapshot_sha256(snapshot)?;
        Ok(
            if self.metadata("snapshot_sha256")? == expected
                && self.metadata("parser_version")? == OBSIDIAN_PARSER_VERSION.to_string()
            {
                ObsidianVaultFreshness::Current
            } else {
                ObsidianVaultFreshness::Stale
            },
        )
    }

    /// Reads a bounded source-traceable retrieval projection back from derived SQLite rows.
    pub fn retrieval_documents(
        &mut self,
        root: &WorkspaceScopePath,
        verified_on: &str,
        default_source_date: &str,
    ) -> Result<ObsidianRetrievalDocuments, ObsidianIndexError> {
        self.verify_integrity()?;
        let mut statement = self
            .connection
            .prepare(
                "SELECT n.path_key, n.path_json, n.temporal_class, n.note_kind,
                        n.content_sha256, e.kind, e.text, e.start_line, e.start_column,
                        e.end_line, e.end_column
                 FROM vault_notes n JOIN vault_elements e ON e.path_key=n.path_key
                 ORDER BY n.path_key, e.ordinal",
            )
            .map_err(|_| ObsidianIndexError::StorageFailed)?;
        let rows = statement
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, Vec<u8>>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, Option<String>>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, String>(6)?,
                    row.get::<_, i64>(7)?,
                    row.get::<_, i64>(8)?,
                    row.get::<_, i64>(9)?,
                    row.get::<_, i64>(10)?,
                ))
            })
            .map_err(|_| ObsidianIndexError::StorageFailed)?;
        let mut documents = BTreeMap::<String, KnowledgeSourceDocument>::new();
        let mut element_count = 0_usize;
        for row in rows {
            let (key, path_json, temporal, note_kind, content_sha256, kind, text, sl, sc, el, ec) =
                row.map_err(|_| ObsidianIndexError::CorruptIndex)?;
            element_count = element_count
                .checked_add(1)
                .ok_or(ObsidianIndexError::InvalidInput)?;
            if element_count > MAX_RETRIEVAL_ELEMENTS {
                return Err(ObsidianIndexError::InvalidInput);
            }
            let path: WorkspacePath =
                serde_json::from_slice(&path_json).map_err(|_| ObsidianIndexError::CorruptIndex)?;
            if !root.contains_path(&path) {
                return Err(ObsidianIndexError::CorruptIndex);
            }
            let historical = match parse_temporal(&temporal) {
                Some(ObsidianTemporalClass::Current) => false,
                Some(ObsidianTemporalClass::Historical) => true,
                None => return Err(ObsidianIndexError::CorruptIndex),
            };
            let element_kind = parse_element(&kind).ok_or(ObsidianIndexError::CorruptIndex)?;
            let fragment = KnowledgeSourceFragment {
                kind: retrieval_fragment_kind(element_kind),
                text,
                source_range: range_from_sql(sl, sc, el, ec)?,
                fact_key: None,
            };
            let expected_path = path.clone();
            let expected_note_kind = note_kind.clone();
            let document = documents
                .entry(key)
                .or_insert_with(|| KnowledgeSourceDocument {
                    root: root.clone(),
                    path,
                    file_type: KnowledgeFileType::Markdown,
                    authority: KnowledgeSourceAuthority::DerivedProjection,
                    content_sha256: content_sha256.clone(),
                    current_content_sha256: content_sha256.clone(),
                    verified_on: verified_on.to_owned(),
                    source_date: default_source_date.to_owned(),
                    note_kind,
                    historical,
                    denied: false,
                    fragments: Vec::new(),
                });
            if document.content_sha256 != content_sha256
                || document.historical != historical
                || document.path != expected_path
                || document.note_kind != expected_note_kind
                || document.fragments.len() >= MAX_RETRIEVAL_ELEMENTS
            {
                return Err(ObsidianIndexError::CorruptIndex);
            }
            document.fragments.push(fragment);
        }
        drop(statement);
        let documents = documents.into_values().collect::<Vec<_>>();
        let report = self.current_report()?;
        let paths = documents
            .iter()
            .map(|document| &document.path)
            .collect::<Vec<_>>();
        let receipt = self.receipt(ObsidianAccessKind::Query, &report, paths)?;
        Ok(ObsidianRetrievalDocuments { documents, receipt })
    }

    /// Performs bounded deterministic lexical search over indexed elements.
    pub fn query(
        &mut self,
        query: &str,
        limit: u32,
    ) -> Result<ObsidianQueryResult, ObsidianIndexError> {
        if query.trim().is_empty()
            || query.len() > MAX_QUERY_BYTES
            || limit == 0
            || limit > MAX_QUERY_RESULTS
        {
            return Err(ObsidianIndexError::InvalidInput);
        }
        let report = self.current_report()?;
        let (hits, paths) = self.query_rows(
            "WHERE instr(lower(e.text), lower(?1)) > 0",
            Some(query),
            None,
            limit,
        )?;
        let receipt = self.receipt(ObsidianAccessKind::Query, &report, paths.iter())?;
        Ok(ObsidianQueryResult { hits, receipt })
    }

    /// Reads only current notes, optionally restricted to one normalized frontmatter kind.
    pub fn current_notes(
        &mut self,
        note_kind: Option<&str>,
        limit: u32,
    ) -> Result<ObsidianQueryResult, ObsidianIndexError> {
        if limit == 0
            || limit > MAX_QUERY_RESULTS
            || note_kind.is_some_and(|value| {
                value.is_empty()
                    || value.len() > 128
                    || !value
                        .bytes()
                        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
            })
        {
            return Err(ObsidianIndexError::InvalidInput);
        }
        let report = self.current_report()?;
        let (hits, paths) = self.query_rows(
            "WHERE n.temporal_class='current' AND e.kind='title'
               AND (?2 IS NULL OR n.note_kind=?2)",
            None,
            note_kind,
            limit,
        )?;
        let receipt = self.receipt(ObsidianAccessKind::CurrentReader, &report, paths.iter())?;
        Ok(ObsidianQueryResult { hits, receipt })
    }

    /// Traverses resolved note relationships breadth-first under fixed bounds.
    pub fn traverse(
        &mut self,
        start: &WorkspacePath,
        max_depth: u32,
        max_results: u32,
    ) -> Result<ObsidianTraversalResult, ObsidianIndexError> {
        if max_depth == 0
            || max_depth > MAX_TRAVERSAL_DEPTH
            || max_results == 0
            || max_results > MAX_QUERY_RESULTS
        {
            return Err(ObsidianIndexError::InvalidInput);
        }
        let report = self.current_report()?;
        let start_key = path_key(start)?;
        let exists: bool = self
            .connection
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM vault_notes WHERE path_key=?1)",
                [&start_key],
                |row| row.get(0),
            )
            .map_err(|_| ObsidianIndexError::StorageFailed)?;
        if !exists {
            return Err(ObsidianIndexError::InvalidInput);
        }
        let mut visited = BTreeSet::from([start_key.clone()]);
        let mut queue = VecDeque::from([(start_key, 0_u32)]);
        let mut paths = Vec::new();
        while let Some((source, depth)) = queue.pop_front() {
            if depth >= max_depth || paths.len() >= max_results as usize {
                continue;
            }
            let mut statement = self
                .connection
                .prepare(
                    "SELECT target_key FROM vault_links
                     WHERE source_key=?1 AND target_is_attachment=0 ORDER BY target_key",
                )
                .map_err(|_| ObsidianIndexError::StorageFailed)?;
            let targets = statement
                .query_map([source], |row| row.get::<_, String>(0))
                .map_err(|_| ObsidianIndexError::StorageFailed)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|_| ObsidianIndexError::CorruptIndex)?;
            for target in targets {
                if visited.insert(target.clone()) {
                    let path: WorkspacePath = serde_json::from_str(&target)
                        .map_err(|_| ObsidianIndexError::CorruptIndex)?;
                    paths.push(path);
                    queue.push_back((target, depth + 1));
                    if paths.len() >= max_results as usize {
                        break;
                    }
                }
            }
        }
        let receipt = self.receipt(ObsidianAccessKind::Traversal, &report, paths.iter())?;
        Ok(ObsidianTraversalResult { paths, receipt })
    }

    /// Returns every visible case or link conflict in deterministic order.
    pub fn conflicts(&self) -> Result<Vec<ObsidianIndexConflict>, ObsidianIndexError> {
        self.verify_integrity()?;
        let mut statement = self
            .connection
            .prepare(
                "SELECT kind, source_sha256, candidate_count
                 FROM vault_conflicts ORDER BY ordinal",
            )
            .map_err(|_| ObsidianIndexError::StorageFailed)?;
        let rows = statement
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                ))
            })
            .map_err(|_| ObsidianIndexError::StorageFailed)?;
        let mut values = Vec::new();
        for row in rows {
            let (kind, source_sha256, candidate_count) =
                row.map_err(|_| ObsidianIndexError::CorruptIndex)?;
            values.push(ObsidianIndexConflict {
                kind: parse_conflict(&kind).ok_or(ObsidianIndexError::CorruptIndex)?,
                source_sha256,
                candidate_count: usize::try_from(candidate_count)
                    .map_err(|_| ObsidianIndexError::CorruptIndex)?,
            });
        }
        Ok(values)
    }

    /// Atomically applies an exact watcher-observed change set to derived state only.
    pub fn apply_watch_batch(
        &mut self,
        expected_revision: u64,
        events: &[ObsidianWatchEvent],
        snapshot: &ObsidianVaultSnapshot,
    ) -> Result<ObsidianIndexUpdate, ObsidianIndexError> {
        if events.is_empty()
            || events.len() > MAX_WATCH_EVENTS
            || self.revision()? != expected_revision
        {
            return Err(ObsidianIndexError::Stale);
        }
        self.verify_integrity()?;
        let before = self.indexed_digests()?;
        let after = snapshot_digests(snapshot)?;
        let expected = changed_objects(&before, &after);
        let mut observed = BTreeMap::new();
        for event in events {
            let key = path_key(&event.path)?;
            let expected_event = expected.get(&key).ok_or(ObsidianIndexError::InvalidInput)?;
            if observed.insert(key.clone(), event.kind).is_some()
                || expected_event != &event.kind
                || match event.kind {
                    ObsidianWatchEventKind::Created | ObsidianWatchEventKind::Modified => {
                        event.content_sha256.as_ref() != after.get(&key)
                    }
                    ObsidianWatchEventKind::Deleted => event.content_sha256.is_some(),
                }
            {
                return Err(ObsidianIndexError::InvalidInput);
            }
        }
        if observed != expected {
            return Err(ObsidianIndexError::InvalidInput);
        }
        let revision = expected_revision
            .checked_add(1)
            .ok_or(ObsidianIndexError::InvalidInput)?;
        let report = self.replace_projection(snapshot, revision)?;
        let paths = events.iter().map(|event| &event.path).collect::<Vec<_>>();
        let receipt = self.receipt(ObsidianAccessKind::WatchUpdate, &report, paths)?;
        Ok(ObsidianIndexUpdate { report, receipt })
    }

    /// Publishes a derived projection only after one exact canonical Markdown commit.
    pub fn publish_after_canonical_write(
        &mut self,
        expected_revision: u64,
        publication: &KnowledgeIndexPublication,
        snapshot: &ObsidianVaultSnapshot,
    ) -> Result<ObsidianPostWriteIndexResult, ObsidianIndexError> {
        if self.revision()? != expected_revision {
            return Err(ObsidianIndexError::Stale);
        }
        match publication.state {
            KnowledgeIndexPublicationState::PreservedAfterFailure => {
                return Ok(ObsidianPostWriteIndexResult {
                    state: publication.state,
                    report: self.current_report()?,
                    update: None,
                });
            }
            KnowledgeIndexPublicationState::RebuildRequired => {
                return Err(ObsidianIndexError::Stale);
            }
            KnowledgeIndexPublicationState::ReadyAfterCommit => {}
        }
        let mutation = &publication.mutation;
        let note = snapshot
            .notes()
            .iter()
            .find(|note| &note.path == mutation.path())
            .ok_or(ObsidianIndexError::InvalidInput)?;
        if note.content_sha256 != mutation.proposed_source_sha256()
            || publication.observed_source_sha256.as_deref()
                != Some(mutation.proposed_source_sha256())
        {
            return Err(ObsidianIndexError::Stale);
        }
        let parsed = MarkdownDocument::parse(note.path.clone(), note.source_bytes().to_vec())
            .map_err(|_| ObsidianIndexError::InvalidInput)?;
        if parsed.stable_id() != Some(mutation.stable_id()) {
            return Err(ObsidianIndexError::InvalidInput);
        }
        let key = path_key(mutation.path())?;
        let before = self.indexed_digests()?;
        let kind = match (
            mutation.expected_source_sha256(),
            before.get(&key).map(String::as_str),
        ) {
            (None, None) => ObsidianWatchEventKind::Created,
            (Some(expected), Some(current)) if expected == current => {
                ObsidianWatchEventKind::Modified
            }
            _ => return Err(ObsidianIndexError::Stale),
        };
        let event = ObsidianWatchEvent {
            path: mutation.path().clone(),
            kind,
            content_sha256: Some(mutation.proposed_source_sha256().to_owned()),
        };
        let update = self.apply_watch_batch(expected_revision, &[event], snapshot)?;
        Ok(ObsidianPostWriteIndexResult {
            state: publication.state,
            report: update.report.clone(),
            update: Some(update),
        })
    }

    /// Previews one section-body replacement while preserving every other source byte.
    pub fn preview_section_change(
        &mut self,
        snapshot: &ObsidianVaultSnapshot,
        path: &WorkspacePath,
        heading: &str,
        replacement: &str,
    ) -> Result<ObsidianPreviewResult, ObsidianIndexError> {
        if heading.trim().is_empty()
            || heading.len() > MAX_QUERY_BYTES
            || replacement.len() > MAX_REPLACEMENT_BYTES
            || replacement.contains('\0')
        {
            return Err(ObsidianIndexError::InvalidInput);
        }
        if self.freshness(snapshot)? != ObsidianVaultFreshness::Current {
            return Err(ObsidianIndexError::Stale);
        }
        let report = self.current_report()?;
        let note = snapshot
            .notes()
            .iter()
            .find(|note| &note.path == path)
            .ok_or(ObsidianIndexError::InvalidInput)?;
        let preview = preview_section(note, heading, replacement)?;
        let receipt = self.receipt(ObsidianAccessKind::Preview, &report, [path])?;
        Ok(ObsidianPreviewResult { preview, receipt })
    }

    fn current_report(&self) -> Result<ObsidianIndexReport, ObsidianIndexError> {
        self.verify_integrity()?;
        let count = |table: &str| -> Result<u64, ObsidianIndexError> {
            let query = format!("SELECT count(*) FROM {table}");
            let value: i64 = self
                .connection
                .query_row(&query, [], |row| row.get(0))
                .map_err(|_| ObsidianIndexError::StorageFailed)?;
            u64::try_from(value).map_err(|_| ObsidianIndexError::CorruptIndex)
        };
        Ok(ObsidianIndexReport {
            revision: self.revision()?,
            note_count: count("vault_notes")?,
            attachment_count: count("vault_attachments")?,
            element_count: count("vault_elements")?,
            link_count: count("vault_links")?,
            conflict_count: count("vault_conflicts")?,
            snapshot_sha256: self.metadata("snapshot_sha256")?,
            index_sha256: self.metadata("index_sha256")?,
            canonical: false,
        })
    }

    fn verify_integrity(&self) -> Result<(), ObsidianIndexError> {
        let expected = self.metadata("index_sha256")?;
        if expected.is_empty() || projection_sha256(&self.connection)? != expected {
            return Err(ObsidianIndexError::CorruptIndex);
        }
        Ok(())
    }

    fn indexed_digests(&self) -> Result<BTreeMap<String, String>, ObsidianIndexError> {
        let mut values = BTreeMap::new();
        for query in [
            "SELECT path_key, content_sha256 FROM vault_notes ORDER BY path_key",
            "SELECT path_key, content_sha256 FROM vault_attachments ORDER BY path_key",
        ] {
            let mut statement = self
                .connection
                .prepare(query)
                .map_err(|_| ObsidianIndexError::StorageFailed)?;
            let rows = statement
                .query_map([], |row| {
                    Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
                })
                .map_err(|_| ObsidianIndexError::StorageFailed)?;
            for row in rows {
                let (key, digest) = row.map_err(|_| ObsidianIndexError::CorruptIndex)?;
                if values.insert(key, digest).is_some() {
                    return Err(ObsidianIndexError::CorruptIndex);
                }
            }
        }
        Ok(values)
    }

    fn query_rows(
        &self,
        where_clause: &str,
        query: Option<&str>,
        note_kind: Option<&str>,
        limit: u32,
    ) -> Result<(Vec<ObsidianIndexHit>, Vec<WorkspacePath>), ObsidianIndexError> {
        let sql = format!(
            "SELECT n.path_json, n.temporal_class, n.note_kind, n.content_sha256,
                    n.parser_version, e.kind, e.text, e.start_line, e.start_column,
                    e.end_line, e.end_column
             FROM vault_elements e JOIN vault_notes n ON n.path_key=e.path_key
             {where_clause}
             ORDER BY CASE n.temporal_class WHEN 'current' THEN 0 ELSE 1 END,
                      n.path_key, e.ordinal LIMIT ?3"
        );
        let mut statement = self
            .connection
            .prepare(&sql)
            .map_err(|_| ObsidianIndexError::StorageFailed)?;
        let rows = statement
            .query_map(params![query, note_kind, limit], |row| {
                Ok((
                    row.get::<_, Vec<u8>>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, i64>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, String>(6)?,
                    row.get::<_, i64>(7)?,
                    row.get::<_, i64>(8)?,
                    row.get::<_, i64>(9)?,
                    row.get::<_, i64>(10)?,
                ))
            })
            .map_err(|_| ObsidianIndexError::StorageFailed)?;
        let mut hits = Vec::new();
        let mut paths = Vec::new();
        for row in rows {
            let (
                path_json,
                temporal,
                note_kind,
                content_sha256,
                parser,
                kind,
                text,
                sl,
                sc,
                el,
                ec,
            ) = row.map_err(|_| ObsidianIndexError::CorruptIndex)?;
            let path: WorkspacePath =
                serde_json::from_slice(&path_json).map_err(|_| ObsidianIndexError::CorruptIndex)?;
            let source_range = range_from_sql(sl, sc, el, ec)?;
            hits.push(ObsidianIndexHit {
                path: path.clone(),
                temporal_class: parse_temporal(&temporal)
                    .ok_or(ObsidianIndexError::CorruptIndex)?,
                note_kind,
                content_sha256,
                parser_version: u16::try_from(parser)
                    .map_err(|_| ObsidianIndexError::CorruptIndex)?,
                element_kind: parse_element(&kind).ok_or(ObsidianIndexError::CorruptIndex)?,
                source_range,
                text,
            });
            paths.push(path);
        }
        Ok((hits, paths))
    }

    fn revision(&self) -> Result<u64, ObsidianIndexError> {
        self.metadata("revision")?
            .parse()
            .map_err(|_| ObsidianIndexError::CorruptIndex)
    }

    fn metadata(&self, key: &str) -> Result<String, ObsidianIndexError> {
        self.connection
            .query_row(
                "SELECT value FROM vault_metadata WHERE key=?1",
                [key],
                |row| row.get(0),
            )
            .map_err(|_| ObsidianIndexError::CorruptIndex)
    }

    fn replace_projection(
        &mut self,
        snapshot: &ObsidianVaultSnapshot,
        revision: u64,
    ) -> Result<ObsidianIndexReport, ObsidianIndexError> {
        let snapshot_sha256 = obsidian_snapshot_sha256(snapshot)?;
        let conflicts = conflicts(snapshot)?;
        let transaction = self
            .connection
            .transaction()
            .map_err(|_| ObsidianIndexError::StorageFailed)?;
        clear_projection(&transaction)?;
        let mut element_count = 0_u64;
        for note in snapshot.notes() {
            let path_key = path_key(&note.path)?;
            let title = note_title(note);
            let temporal_class = temporal_class(note);
            let note_kind = note_kind(note);
            transaction
                .execute(
                    "INSERT INTO vault_notes(
                       path_key, path_json, content_sha256, parser_version,
                       temporal_class, note_kind, title
                     ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                    params![
                        path_key,
                        serde_json::to_vec(&note.path)
                            .map_err(|_| ObsidianIndexError::InvalidInput)?,
                        note.content_sha256,
                        i64::from(OBSIDIAN_PARSER_VERSION),
                        temporal_wire(temporal_class),
                        note_kind,
                        title,
                    ],
                )
                .map_err(|_| ObsidianIndexError::StorageFailed)?;
            for (ordinal, element) in index_elements(note).into_iter().enumerate() {
                transaction
                    .execute(
                        "INSERT INTO vault_elements(
                           path_key, ordinal, kind, text, start_line, start_column,
                           end_line, end_column
                         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                        params![
                            path_key,
                            i64::try_from(ordinal).map_err(|_| ObsidianIndexError::InvalidInput)?,
                            element_wire(element.kind),
                            element.text,
                            i64::from(element.range.start_line),
                            i64::from(element.range.start_column),
                            i64::from(element.range.end_line),
                            i64::from(element.range.end_column),
                        ],
                    )
                    .map_err(|_| ObsidianIndexError::StorageFailed)?;
                element_count = element_count
                    .checked_add(1)
                    .ok_or(ObsidianIndexError::InvalidInput)?;
            }
            for (ordinal, coverage) in note.coverage.iter().enumerate() {
                transaction
                    .execute(
                        "INSERT INTO vault_coverage(
                           path_key, ordinal, syntax, supported, start_line, start_column,
                           end_line, end_column
                         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                        params![
                            path_key,
                            i64::try_from(ordinal).map_err(|_| ObsidianIndexError::InvalidInput)?,
                            coverage.syntax,
                            i64::from(coverage.supported),
                            i64::from(coverage.source_range.start_line),
                            i64::from(coverage.source_range.start_column),
                            i64::from(coverage.source_range.end_line),
                            i64::from(coverage.source_range.end_column),
                        ],
                    )
                    .map_err(|_| ObsidianIndexError::StorageFailed)?;
            }
        }
        for attachment in snapshot.attachments() {
            transaction
                .execute(
                    "INSERT INTO vault_attachments(
                       path_key, path_json, content_sha256, byte_count
                     ) VALUES (?1, ?2, ?3, ?4)",
                    params![
                        path_key(&attachment.path)?,
                        serde_json::to_vec(&attachment.path)
                            .map_err(|_| ObsidianIndexError::InvalidInput)?,
                        attachment.content_sha256,
                        i64::try_from(attachment.byte_count)
                            .map_err(|_| ObsidianIndexError::InvalidInput)?,
                    ],
                )
                .map_err(|_| ObsidianIndexError::StorageFailed)?;
        }
        let mut link_rows = BTreeSet::new();
        for link in snapshot.resolved_links() {
            link_rows.insert((
                path_key(&link.source_path)?,
                path_key(&link.target_path)?,
                false,
                link.line_number,
            ));
        }
        for note in snapshot.notes() {
            for embed in &note.embeds {
                if let Some(target) = &embed.target_path {
                    link_rows.insert((
                        path_key(&note.path)?,
                        path_key(target)?,
                        embed.attachment,
                        embed.source_range.start_line,
                    ));
                }
            }
        }
        for (source, target, attachment, line_number) in &link_rows {
            transaction
                .execute(
                    "INSERT INTO vault_links(
                       source_key, target_key, target_is_attachment, line_number
                     ) VALUES (?1, ?2, ?3, ?4)",
                    params![
                        source,
                        target,
                        i64::from(*attachment),
                        i64::from(*line_number)
                    ],
                )
                .map_err(|_| ObsidianIndexError::StorageFailed)?;
        }
        for (ordinal, conflict) in conflicts.iter().enumerate() {
            transaction
                .execute(
                    "INSERT INTO vault_conflicts(
                       ordinal, kind, source_sha256, candidate_count
                     ) VALUES (?1, ?2, ?3, ?4)",
                    params![
                        i64::try_from(ordinal).map_err(|_| ObsidianIndexError::InvalidInput)?,
                        conflict_wire(conflict.kind),
                        conflict.source_sha256,
                        i64::try_from(conflict.candidate_count)
                            .map_err(|_| ObsidianIndexError::InvalidInput)?,
                    ],
                )
                .map_err(|_| ObsidianIndexError::StorageFailed)?;
        }
        for (key, value) in [
            ("revision", revision.to_string()),
            ("snapshot_sha256", snapshot_sha256.clone()),
            ("parser_version", OBSIDIAN_PARSER_VERSION.to_string()),
        ] {
            transaction
                .execute(
                    "UPDATE vault_metadata SET value=?2 WHERE key=?1",
                    params![key, value],
                )
                .map_err(|_| ObsidianIndexError::StorageFailed)?;
        }
        let index_sha256 = projection_sha256(&transaction)?;
        transaction
            .execute(
                "UPDATE vault_metadata SET value=?1 WHERE key='index_sha256'",
                [&index_sha256],
            )
            .map_err(|_| ObsidianIndexError::StorageFailed)?;
        transaction
            .commit()
            .map_err(|_| ObsidianIndexError::StorageFailed)?;
        Ok(ObsidianIndexReport {
            revision,
            note_count: u64::try_from(snapshot.notes().len())
                .map_err(|_| ObsidianIndexError::InvalidInput)?,
            attachment_count: u64::try_from(snapshot.attachments().len())
                .map_err(|_| ObsidianIndexError::InvalidInput)?,
            element_count,
            link_count: u64::try_from(link_rows.len())
                .map_err(|_| ObsidianIndexError::InvalidInput)?,
            conflict_count: u64::try_from(conflicts.len())
                .map_err(|_| ObsidianIndexError::InvalidInput)?,
            snapshot_sha256,
            index_sha256,
            canonical: false,
        })
    }

    fn receipt<'a, I>(
        &mut self,
        operation: ObsidianAccessKind,
        report: &ObsidianIndexReport,
        paths: I,
    ) -> Result<ObsidianAccessReceipt, ObsidianIndexError>
    where
        I: IntoIterator<Item = &'a WorkspacePath>,
    {
        let sequence = self.next_receipt_sequence;
        self.next_receipt_sequence = sequence
            .checked_add(1)
            .ok_or(ObsidianIndexError::InvalidInput)?;
        let mut accessed_path_sha256 = paths
            .into_iter()
            .map(path_key)
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .map(|value| sha256(value.as_bytes()))
            .collect::<Vec<_>>();
        accessed_path_sha256.sort();
        accessed_path_sha256.dedup();
        let receipt_sha256 = receipt_sha256(
            sequence,
            operation,
            report.revision,
            &report.snapshot_sha256,
            &report.index_sha256,
            &accessed_path_sha256,
        );
        Ok(ObsidianAccessReceipt {
            sequence,
            operation,
            index_revision: report.revision,
            snapshot_sha256: report.snapshot_sha256.clone(),
            index_sha256: report.index_sha256.clone(),
            accessed_path_sha256,
            receipt_sha256,
            source_files_mutated: false,
            external_process_started: false,
            network_accessed: false,
        })
    }
}

/// Projects one immutable canonical snapshot into bounded deterministic retrieval documents.
pub fn retrieval_documents_from_snapshot(
    snapshot: &ObsidianVaultSnapshot,
    root: &WorkspaceScopePath,
    verified_on: &str,
    default_source_date: &str,
) -> Result<Vec<KnowledgeSourceDocument>, ObsidianIndexError> {
    snapshot
        .notes()
        .iter()
        .map(|note| {
            if !root.contains_path(&note.path) {
                return Err(ObsidianIndexError::InvalidInput);
            }
            let fragments = index_elements(note)
                .into_iter()
                .map(|element| KnowledgeSourceFragment {
                    kind: retrieval_fragment_kind(element.kind),
                    text: element.text,
                    source_range: element.range,
                    fact_key: None,
                })
                .collect::<Vec<_>>();
            Ok(KnowledgeSourceDocument {
                root: root.clone(),
                path: note.path.clone(),
                file_type: KnowledgeFileType::Markdown,
                authority: KnowledgeSourceAuthority::CanonicalMarkdown,
                content_sha256: note.content_sha256.clone(),
                current_content_sha256: note.content_sha256.clone(),
                verified_on: verified_on.to_owned(),
                source_date: default_source_date.to_owned(),
                note_kind: note_kind(note),
                historical: temporal_class(note) == ObsidianTemporalClass::Historical,
                denied: false,
                fragments,
            })
        })
        .collect()
}

#[derive(Clone)]
struct IndexElement {
    kind: ObsidianIndexElementKind,
    text: String,
    range: ObsidianSourceRange,
}

fn clear_projection(transaction: &Transaction<'_>) -> Result<(), ObsidianIndexError> {
    transaction
        .execute_batch(
            "DELETE FROM vault_links;
             DELETE FROM vault_elements;
             DELETE FROM vault_coverage;
             DELETE FROM vault_notes;
             DELETE FROM vault_attachments;
             DELETE FROM vault_conflicts;",
        )
        .map_err(|_| ObsidianIndexError::StorageFailed)
}

fn note_title(note: &ObsidianParsedNote) -> String {
    note.headings.first().map_or_else(
        || {
            note.path
                .components()
                .last()
                .map(|component| {
                    component
                        .as_str()
                        .strip_suffix(".md")
                        .unwrap_or(component.as_str())
                        .to_owned()
                })
                .unwrap_or_else(|| "Untitled".to_owned())
        },
        |heading| heading.text.clone(),
    )
}

fn note_kind(note: &ObsidianParsedNote) -> Option<String> {
    ["agentmage_kind", "type"]
        .iter()
        .find_map(|key| note.frontmatter.get(*key))
        .and_then(first_frontmatter_value)
        .map(|value| value.to_ascii_lowercase())
        .filter(|value| {
            !value.is_empty()
                && value.len() <= 128
                && value
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
        })
}

fn temporal_class(note: &ObsidianParsedNote) -> ObsidianTemporalClass {
    let status = note
        .frontmatter
        .get("status")
        .and_then(first_frontmatter_value)
        .map(str::to_ascii_lowercase);
    let historical_status = status
        .as_deref()
        .is_some_and(|value| matches!(value, "historical" | "archived" | "archive" | "superseded"));
    let historical_path = note.path.components().iter().any(|component| {
        matches!(
            component.as_str().to_ascii_lowercase().as_str(),
            "archive" | "archives" | "historical" | "history" | "superseded"
        )
    });
    if historical_status || historical_path {
        ObsidianTemporalClass::Historical
    } else {
        ObsidianTemporalClass::Current
    }
}

fn first_frontmatter_value(value: &ObsidianFrontmatterValue) -> Option<&str> {
    match value {
        ObsidianFrontmatterValue::Scalar(value) => Some(value),
        ObsidianFrontmatterValue::Sequence(values) => values.first().map(String::as_str),
    }
}

fn index_elements(note: &ObsidianParsedNote) -> Vec<IndexElement> {
    let title_range = note.headings.first().map_or(
        ObsidianSourceRange {
            start_line: 1,
            start_column: 1,
            end_line: 1,
            end_column: 1,
        },
        |heading| heading.source_range,
    );
    let mut values = vec![IndexElement {
        kind: ObsidianIndexElementKind::Title,
        text: note_title(note),
        range: title_range,
    }];
    values.extend(note.properties.iter().map(|property| IndexElement {
        kind: ObsidianIndexElementKind::Property,
        text: format!("{}={}", property.key, property.values.join(",")),
        range: property.source_range,
    }));
    values.extend(note.headings.iter().map(|heading| IndexElement {
        kind: ObsidianIndexElementKind::Heading,
        text: heading.text.clone(),
        range: heading.source_range,
    }));
    values.extend(note.tasks.iter().map(|task| IndexElement {
        kind: ObsidianIndexElementKind::Task,
        text: task.text.clone(),
        range: task.source_range,
    }));
    values.extend(note.tags.iter().map(|tag| IndexElement {
        kind: ObsidianIndexElementKind::Tag,
        text: tag.value.clone(),
        range: tag.source_range,
    }));
    values.extend(note.callouts.iter().map(|callout| IndexElement {
        kind: ObsidianIndexElementKind::Callout,
        text: callout.title.as_ref().map_or_else(
            || callout.kind.clone(),
            |title| format!("{} {title}", callout.kind),
        ),
        range: callout.source_range,
    }));
    values.extend(note.blocks.iter().map(|block| IndexElement {
        kind: ObsidianIndexElementKind::Block,
        text: block.identifier.clone(),
        range: block.source_range,
    }));
    values.extend(note.timestamps.iter().map(|timestamp| IndexElement {
        kind: ObsidianIndexElementKind::Timestamp,
        text: format!("{}={}", timestamp.key, timestamp.value),
        range: timestamp.source_range,
    }));
    values.extend(note.embeds.iter().map(|embed| IndexElement {
        kind: ObsidianIndexElementKind::Embed,
        text: embed.target.clone(),
        range: embed.source_range,
    }));
    values.retain(|element| !crate::domain::secret_candidate(&element.text));
    values
}

const fn retrieval_fragment_kind(kind: ObsidianIndexElementKind) -> KnowledgeSourceFragmentKind {
    match kind {
        ObsidianIndexElementKind::Title => KnowledgeSourceFragmentKind::Title,
        ObsidianIndexElementKind::Property => KnowledgeSourceFragmentKind::Field,
        ObsidianIndexElementKind::Heading => KnowledgeSourceFragmentKind::Heading,
        ObsidianIndexElementKind::Task => KnowledgeSourceFragmentKind::Task,
        ObsidianIndexElementKind::Tag => KnowledgeSourceFragmentKind::Tag,
        ObsidianIndexElementKind::Timestamp => KnowledgeSourceFragmentKind::Date,
        ObsidianIndexElementKind::Embed => KnowledgeSourceFragmentKind::Link,
        ObsidianIndexElementKind::Callout | ObsidianIndexElementKind::Block => {
            KnowledgeSourceFragmentKind::Metadata
        }
    }
}

fn conflicts(
    snapshot: &ObsidianVaultSnapshot,
) -> Result<Vec<ObsidianIndexConflict>, ObsidianIndexError> {
    let mut by_case = BTreeMap::<String, BTreeSet<String>>::new();
    for path in snapshot
        .notes()
        .iter()
        .map(|note| &note.path)
        .chain(snapshot.attachments().iter().map(|item| &item.path))
    {
        let exact = path_key(path)?;
        by_case
            .entry(exact.to_ascii_lowercase())
            .or_default()
            .insert(exact);
    }
    let mut values = by_case
        .into_iter()
        .filter(|(_, paths)| paths.len() > 1)
        .map(|(normalized, paths)| ObsidianIndexConflict {
            kind: ObsidianIndexConflictKind::CaseCollision,
            source_sha256: sha256(normalized.as_bytes()),
            candidate_count: paths.len(),
        })
        .collect::<Vec<_>>();
    for issue in snapshot.link_issues() {
        values.push(ObsidianIndexConflict {
            kind: match issue.kind {
                crate::ObsidianLinkIssueKind::Unresolved => {
                    ObsidianIndexConflictKind::UnresolvedLink
                }
                crate::ObsidianLinkIssueKind::Ambiguous => ObsidianIndexConflictKind::AmbiguousLink,
            },
            source_sha256: sha256(
                format!(
                    "{}\0{}\0{}",
                    path_key(&issue.source_path)?,
                    issue.line_number,
                    issue.target
                )
                .as_bytes(),
            ),
            candidate_count: issue.candidate_count,
        });
    }
    values.sort_by(|left, right| {
        left.kind
            .cmp(&right.kind)
            .then_with(|| left.source_sha256.cmp(&right.source_sha256))
    });
    Ok(values)
}

/// Computes the canonical content-free identity of one complete vault snapshot.
pub fn obsidian_snapshot_sha256(
    snapshot: &ObsidianVaultSnapshot,
) -> Result<String, ObsidianIndexError> {
    let mut material = Vec::new();
    material.extend_from_slice(b"agentmage-obsidian-snapshot-v2\0");
    material.extend_from_slice(&OBSIDIAN_PARSER_VERSION.to_be_bytes());
    for note in snapshot.notes() {
        material.extend_from_slice(path_key(&note.path)?.as_bytes());
        material.push(0);
        material.extend_from_slice(note.content_sha256.as_bytes());
        material.push(b'\n');
    }
    for attachment in snapshot.attachments() {
        material.extend_from_slice(path_key(&attachment.path)?.as_bytes());
        material.push(0);
        material.extend_from_slice(attachment.content_sha256.as_bytes());
        material.push(0);
        material.extend_from_slice(&attachment.byte_count.to_be_bytes());
        material.push(b'\n');
    }
    for link in snapshot.resolved_links() {
        material.extend_from_slice(path_key(&link.source_path)?.as_bytes());
        material.push(0);
        material.extend_from_slice(path_key(&link.target_path)?.as_bytes());
        material.push(0);
        material.extend_from_slice(&link.line_number.to_be_bytes());
        material.push(b'\n');
    }
    for issue in snapshot.link_issues() {
        material.extend_from_slice(path_key(&issue.source_path)?.as_bytes());
        material.push(0);
        material.extend_from_slice(issue.target.as_bytes());
        material.push(0);
        material.extend_from_slice(&issue.candidate_count.to_be_bytes());
        material.push(b'\n');
    }
    Ok(sha256(&material))
}

fn projection_sha256(connection: &Connection) -> Result<String, ObsidianIndexError> {
    let mut material = Vec::new();
    for query in [
        "SELECT path_key || char(0) || content_sha256 || char(0) || parser_version || char(0) || temporal_class || char(0) || ifnull(note_kind,'') || char(0) || title FROM vault_notes ORDER BY path_key",
        "SELECT path_key || char(0) || content_sha256 || char(0) || byte_count FROM vault_attachments ORDER BY path_key",
        "SELECT path_key || char(0) || ordinal || char(0) || kind || char(0) || text || char(0) || start_line || ':' || start_column || ':' || end_line || ':' || end_column FROM vault_elements ORDER BY path_key, ordinal",
        "SELECT source_key || char(0) || target_key || char(0) || target_is_attachment || char(0) || line_number FROM vault_links ORDER BY source_key, target_key, line_number",
        "SELECT path_key || char(0) || ordinal || char(0) || syntax || char(0) || supported || char(0) || start_line || ':' || start_column || ':' || end_line || ':' || end_column FROM vault_coverage ORDER BY path_key, ordinal",
        "SELECT ordinal || char(0) || kind || char(0) || source_sha256 || char(0) || candidate_count FROM vault_conflicts ORDER BY ordinal",
    ] {
        let mut statement = connection
            .prepare(query)
            .map_err(|_| ObsidianIndexError::StorageFailed)?;
        let rows = statement
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(|_| ObsidianIndexError::StorageFailed)?;
        for row in rows {
            material.extend_from_slice(
                row.map_err(|_| ObsidianIndexError::CorruptIndex)?
                    .as_bytes(),
            );
            material.push(b'\n');
        }
        material.push(0xff);
    }
    for key in [
        "authority",
        "schema_version",
        "parser_version",
        "revision",
        "snapshot_sha256",
    ] {
        let value: String = connection
            .query_row(
                "SELECT value FROM vault_metadata WHERE key=?1",
                [key],
                |row| row.get(0),
            )
            .map_err(|_| ObsidianIndexError::CorruptIndex)?;
        material.extend_from_slice(key.as_bytes());
        material.push(0);
        material.extend_from_slice(value.as_bytes());
        material.push(b'\n');
    }
    Ok(sha256(&material))
}

fn path_key(path: &WorkspacePath) -> Result<String, ObsidianIndexError> {
    serde_json::to_string(path).map_err(|_| ObsidianIndexError::InvalidInput)
}

fn sha256(value: &[u8]) -> String {
    let mut output = String::with_capacity(64);
    for byte in Sha256::digest(value) {
        write!(&mut output, "{byte:02x}").expect("writing to String cannot fail");
    }
    output
}

fn receipt_sha256(
    sequence: u64,
    operation: ObsidianAccessKind,
    revision: u64,
    snapshot_sha256: &str,
    index_sha256: &str,
    paths: &[String],
) -> String {
    let mut material = Vec::new();
    material.extend_from_slice(b"agentmage-obsidian-access-receipt-v1\0");
    material.extend_from_slice(&sequence.to_be_bytes());
    material.push(access_wire(operation));
    material.extend_from_slice(&revision.to_be_bytes());
    material.extend_from_slice(snapshot_sha256.as_bytes());
    material.extend_from_slice(index_sha256.as_bytes());
    for path in paths {
        material.extend_from_slice(path.as_bytes());
        material.push(b'\n');
    }
    sha256(&material)
}

const fn access_wire(kind: ObsidianAccessKind) -> u8 {
    match kind {
        ObsidianAccessKind::Rebuild => 1,
        ObsidianAccessKind::WatchUpdate => 2,
        ObsidianAccessKind::Query => 3,
        ObsidianAccessKind::CurrentReader => 4,
        ObsidianAccessKind::Traversal => 5,
        ObsidianAccessKind::Preview => 6,
    }
}

const fn temporal_wire(class: ObsidianTemporalClass) -> &'static str {
    match class {
        ObsidianTemporalClass::Current => "current",
        ObsidianTemporalClass::Historical => "historical",
    }
}

fn parse_temporal(value: &str) -> Option<ObsidianTemporalClass> {
    match value {
        "current" => Some(ObsidianTemporalClass::Current),
        "historical" => Some(ObsidianTemporalClass::Historical),
        _ => None,
    }
}

const fn element_wire(kind: ObsidianIndexElementKind) -> &'static str {
    match kind {
        ObsidianIndexElementKind::Title => "title",
        ObsidianIndexElementKind::Property => "property",
        ObsidianIndexElementKind::Heading => "heading",
        ObsidianIndexElementKind::Task => "task",
        ObsidianIndexElementKind::Tag => "tag",
        ObsidianIndexElementKind::Callout => "callout",
        ObsidianIndexElementKind::Block => "block",
        ObsidianIndexElementKind::Timestamp => "timestamp",
        ObsidianIndexElementKind::Embed => "embed",
    }
}

fn parse_element(value: &str) -> Option<ObsidianIndexElementKind> {
    [
        ObsidianIndexElementKind::Title,
        ObsidianIndexElementKind::Property,
        ObsidianIndexElementKind::Heading,
        ObsidianIndexElementKind::Task,
        ObsidianIndexElementKind::Tag,
        ObsidianIndexElementKind::Callout,
        ObsidianIndexElementKind::Block,
        ObsidianIndexElementKind::Timestamp,
        ObsidianIndexElementKind::Embed,
    ]
    .into_iter()
    .find(|kind| element_wire(*kind) == value)
}

const fn conflict_wire(kind: ObsidianIndexConflictKind) -> &'static str {
    match kind {
        ObsidianIndexConflictKind::CaseCollision => "case_collision",
        ObsidianIndexConflictKind::UnresolvedLink => "unresolved_link",
        ObsidianIndexConflictKind::AmbiguousLink => "ambiguous_link",
    }
}

fn parse_conflict(value: &str) -> Option<ObsidianIndexConflictKind> {
    [
        ObsidianIndexConflictKind::CaseCollision,
        ObsidianIndexConflictKind::UnresolvedLink,
        ObsidianIndexConflictKind::AmbiguousLink,
    ]
    .into_iter()
    .find(|kind| conflict_wire(*kind) == value)
}

fn range_from_sql(
    start_line: i64,
    start_column: i64,
    end_line: i64,
    end_column: i64,
) -> Result<ObsidianSourceRange, ObsidianIndexError> {
    let range = ObsidianSourceRange {
        start_line: u32::try_from(start_line).map_err(|_| ObsidianIndexError::CorruptIndex)?,
        start_column: u32::try_from(start_column).map_err(|_| ObsidianIndexError::CorruptIndex)?,
        end_line: u32::try_from(end_line).map_err(|_| ObsidianIndexError::CorruptIndex)?,
        end_column: u32::try_from(end_column).map_err(|_| ObsidianIndexError::CorruptIndex)?,
    };
    if range.start_line == 0
        || range.start_column == 0
        || range.end_line < range.start_line
        || range.end_column == 0
    {
        return Err(ObsidianIndexError::CorruptIndex);
    }
    Ok(range)
}

fn snapshot_digests(
    snapshot: &ObsidianVaultSnapshot,
) -> Result<BTreeMap<String, String>, ObsidianIndexError> {
    let mut values = BTreeMap::new();
    for (path, digest) in snapshot
        .notes()
        .iter()
        .map(|note| (&note.path, &note.content_sha256))
        .chain(
            snapshot
                .attachments()
                .iter()
                .map(|item| (&item.path, &item.content_sha256)),
        )
    {
        if values.insert(path_key(path)?, digest.clone()).is_some() {
            return Err(ObsidianIndexError::InvalidInput);
        }
    }
    Ok(values)
}

fn changed_objects(
    before: &BTreeMap<String, String>,
    after: &BTreeMap<String, String>,
) -> BTreeMap<String, ObsidianWatchEventKind> {
    let keys = before
        .keys()
        .chain(after.keys())
        .cloned()
        .collect::<BTreeSet<_>>();
    keys.into_iter()
        .filter_map(|key| {
            let kind = match (before.get(&key), after.get(&key)) {
                (None, Some(_)) => Some(ObsidianWatchEventKind::Created),
                (Some(_), None) => Some(ObsidianWatchEventKind::Deleted),
                (Some(left), Some(right)) if left != right => {
                    Some(ObsidianWatchEventKind::Modified)
                }
                _ => None,
            }?;
            Some((key, kind))
        })
        .collect()
}

fn preview_section(
    note: &ObsidianParsedNote,
    heading: &str,
    replacement: &str,
) -> Result<ObsidianFileChangePreview, ObsidianIndexError> {
    let matches = note
        .headings
        .iter()
        .enumerate()
        .filter(|(_, candidate)| candidate.text == heading)
        .collect::<Vec<_>>();
    if matches.len() != 1 {
        return Err(ObsidianIndexError::InvalidInput);
    }
    let (heading_index, selected) = matches[0];
    let source =
        std::str::from_utf8(note.source_bytes()).map_err(|_| ObsidianIndexError::InvalidInput)?;
    let offsets = line_offsets(source);
    let content_start = offsets
        .get(selected.line_number as usize)
        .copied()
        .unwrap_or(source.len());
    let next_heading_line = note.headings[heading_index + 1..]
        .iter()
        .find(|candidate| candidate.level <= selected.level)
        .map(|candidate| candidate.line_number);
    let content_end = next_heading_line
        .and_then(|line| offsets.get(line as usize - 1).copied())
        .unwrap_or(source.len());
    if content_start > content_end || content_end > note.source_bytes().len() {
        return Err(ObsidianIndexError::InvalidInput);
    }
    let mut replacement_bytes = replacement.as_bytes().to_vec();
    if !replacement_bytes.is_empty() && !replacement_bytes.ends_with(b"\n") {
        replacement_bytes.push(b'\n');
    }
    let prefix = &note.source_bytes()[..content_start];
    let suffix = &note.source_bytes()[content_end..];
    let mut proposed_content =
        Vec::with_capacity(prefix.len() + replacement_bytes.len() + suffix.len());
    proposed_content.extend_from_slice(prefix);
    proposed_content.extend_from_slice(&replacement_bytes);
    proposed_content.extend_from_slice(suffix);
    let end_line = next_heading_line.map_or_else(
        || u32::try_from(source.lines().count().max(1)).unwrap_or(u32::MAX),
        |line| line.saturating_sub(1).max(selected.line_number),
    );
    Ok(ObsidianFileChangePreview {
        path: note.path.clone(),
        expected_content_sha256: note.content_sha256.clone(),
        proposed_content_sha256: sha256(&proposed_content),
        proposed_content,
        replaced_range: ObsidianSourceRange {
            start_line: selected.line_number.saturating_add(1),
            start_column: 1,
            end_line,
            end_column: 1,
        },
        preserved_prefix_sha256: sha256(prefix),
        preserved_suffix_sha256: sha256(suffix),
        write_enabled: false,
    })
}

fn line_offsets(source: &str) -> Vec<usize> {
    let mut offsets = vec![0];
    for (index, byte) in source.bytes().enumerate() {
        if byte == b'\n' {
            offsets.push(index + 1);
        }
    }
    offsets
}

#[cfg(test)]
mod tests {
    use agentmage_kernel_contracts::{
        StorageFilesystemClass, StrictLocalStorageObservation, WorkspaceId, WorkspacePath,
        WorkspaceScopePath,
    };

    use super::*;
    use crate::{
        CanonicalKnowledgeMutation, CanonicalMarkdownWriteOutcome, KnowledgeNamespaceSnapshot,
        KnowledgeNoteCreateRequest, KnowledgeRecordId, KnowledgeSectionDraft,
        KnowledgeWriteWorkflow, MarkdownLineEnding, ObsidianEntryKind, ObsidianNoteInput,
        ObsidianVaultSelection, ObsidianVaultSnapshot, decide_index_publication,
        preview_knowledge_note_create,
    };

    fn workspace() -> WorkspaceId {
        WorkspaceId::from_raw("workspace-vault-index")
    }

    fn path(components: &[&str]) -> WorkspacePath {
        WorkspacePath::new(workspace(), components.iter().copied()).expect("path")
    }

    fn selection() -> ObsidianVaultSelection {
        ObsidianVaultSelection::admit(
            WorkspaceScopePath::new(workspace(), ["Vault"]).expect("root"),
            StrictLocalStorageObservation {
                filesystem: StorageFilesystemClass::Local,
                synchronization_marker: None,
                root_identity_sha256: [9; 32],
                symlink_free: true,
            },
            Vec::new(),
        )
        .expect("selection")
    }

    fn input(components: &[&str], content: &str) -> ObsidianNoteInput {
        ObsidianNoteInput {
            path: path(components),
            entry_kind: ObsidianEntryKind::RegularFile,
            hidden: false,
            cloud_synchronized: false,
            content_sha256: sha256(content.as_bytes()),
            content: content.as_bytes().to_vec(),
        }
    }

    fn home_source(extra: &str) -> String {
        format!(
            concat!(
                "---\n",
                "type: meeting\n",
                "status: current\n",
                "aliases: [Start]\n",
                "---\n",
                "# Meeting\n",
                "## Raw Notes\n",
                "Original raw line\n",
                "## Decisions\n",
                "Keep this section byte-for-byte.\n",
                "[[Next]]\n",
                "![[diagram.png]]\n",
                "{}",
            ),
            extra
        )
    }

    fn snapshot(
        home_extra: &str,
        include_archive: bool,
        include_new: bool,
    ) -> ObsidianVaultSnapshot {
        let mut values = vec![
            input(&["Vault", "Home.md"], &home_source(home_extra)),
            input(&["Vault", "Next.md"], "# Next\n[[Home]]\n"),
            input(&["Vault", "diagram.png"], "synthetic attachment"),
            input(&["Vault", "Case.md"], "# Upper Case\n"),
            input(&["Vault", "case.md"], "# Lower Case\n"),
        ];
        if include_archive {
            values.push(input(
                &["Vault", "Archive", "Old.md"],
                "---\ntype: handoff\nstatus: archived\n---\n# Old Handoff\n[[Missing]]\n",
            ));
        }
        if include_new {
            values.push(input(&["Vault", "New.md"], "# New Note\n"));
        }
        ObsidianVaultSnapshot::from_snapshots(&selection(), values).expect("snapshot")
    }

    #[test]
    fn schema_is_disposable_complete_and_contains_no_operational_tables() {
        let index = ObsidianVaultIndex::in_memory().expect("index");
        assert_eq!(index.authority_class().expect("authority"), "derived_only");
        assert_eq!(
            index.table_names().expect("tables"),
            vec![
                "vault_attachments",
                "vault_conflicts",
                "vault_coverage",
                "vault_elements",
                "vault_links",
                "vault_metadata",
                "vault_notes",
            ]
        );
    }

    #[test]
    fn rebuild_query_current_reader_conflicts_and_receipts_are_exact() {
        let source = snapshot("", true, false);
        let mut index = ObsidianVaultIndex::in_memory().expect("index");
        let update = index.rebuild(&source).expect("rebuild");
        assert_eq!(update.report.revision, 1);
        assert_eq!(update.report.note_count, 5);
        assert_eq!(update.report.attachment_count, 1);
        assert!(!update.report.canonical);
        assert_eq!(update.receipt.operation, ObsidianAccessKind::Rebuild);
        assert!(!update.receipt.source_files_mutated);
        assert!(!update.receipt.external_process_started);
        assert!(!update.receipt.network_accessed);
        assert_eq!(
            index.freshness(&source).expect("freshness"),
            ObsidianVaultFreshness::Current
        );

        let result = index.query("Meeting", 10).expect("query");
        assert!(result.hits.iter().any(|hit| hit.text == "Meeting"));
        assert!(
            result
                .hits
                .iter()
                .all(|hit| hit.parser_version == OBSIDIAN_PARSER_VERSION)
        );
        let current = index.current_notes(Some("meeting"), 10).expect("current");
        assert_eq!(current.hits.len(), 1);
        assert_eq!(
            current.hits[0].temporal_class,
            ObsidianTemporalClass::Current
        );
        assert!(
            index
                .conflicts()
                .expect("conflicts")
                .iter()
                .any(|conflict| {
                    conflict.kind == ObsidianIndexConflictKind::CaseCollision
                        && conflict.candidate_count == 2
                })
        );
        assert!(
            index
                .conflicts()
                .expect("conflicts")
                .iter()
                .any(|conflict| { conflict.kind == ObsidianIndexConflictKind::UnresolvedLink })
        );
    }

    #[test]
    fn traversal_is_bounded_cycle_safe_and_deterministic() {
        let source = snapshot("", false, false);
        let mut index = ObsidianVaultIndex::in_memory().expect("index");
        index.rebuild(&source).expect("rebuild");
        let result = index
            .traverse(&path(&["Vault", "Home.md"]), 8, 100)
            .expect("traversal");
        assert_eq!(result.paths, [path(&["Vault", "Next.md"])]);
        assert_eq!(result.receipt.operation, ObsidianAccessKind::Traversal);
        assert_eq!(
            index.traverse(&path(&["Vault", "Home.md"]), 0, 100),
            Err(ObsidianIndexError::InvalidInput)
        );
    }

    #[test]
    fn preview_preserves_heading_prefix_and_every_unselected_section_byte() {
        let source = snapshot("", false, false);
        let mut index = ObsidianVaultIndex::in_memory().expect("index");
        index.rebuild(&source).expect("rebuild");
        let original = source
            .notes()
            .iter()
            .find(|note| note.path == path(&["Vault", "Home.md"]))
            .expect("home");
        let result = index
            .preview_section_change(
                &source,
                &path(&["Vault", "Home.md"]),
                "Raw Notes",
                "Replacement raw line",
            )
            .expect("preview");
        let proposed = String::from_utf8(result.preview.proposed_content.clone()).expect("utf8");
        assert!(proposed.contains("## Raw Notes\nReplacement raw line\n## Decisions\n"));
        assert!(proposed.contains("Keep this section byte-for-byte."));
        assert!(!proposed.contains("Original raw line"));
        assert_eq!(
            result.preview.expected_content_sha256,
            original.content_sha256
        );
        assert!(!result.preview.write_enabled);
        assert_eq!(original.source_bytes(), home_source("").as_bytes());
    }

    #[test]
    fn exact_watcher_batch_updates_only_projection_and_detects_stale_or_incomplete_events() {
        let first = snapshot("", true, false);
        let second = snapshot("## Added\n", false, true);
        let mut index = ObsidianVaultIndex::in_memory().expect("index");
        let initial = index.rebuild(&first).expect("initial");
        let after = snapshot_digests(&second).expect("digests");
        let events = vec![
            ObsidianWatchEvent {
                path: path(&["Vault", "Home.md"]),
                kind: ObsidianWatchEventKind::Modified,
                content_sha256: after
                    .get(&path_key(&path(&["Vault", "Home.md"])).expect("key"))
                    .cloned(),
            },
            ObsidianWatchEvent {
                path: path(&["Vault", "Archive", "Old.md"]),
                kind: ObsidianWatchEventKind::Deleted,
                content_sha256: None,
            },
            ObsidianWatchEvent {
                path: path(&["Vault", "New.md"]),
                kind: ObsidianWatchEventKind::Created,
                content_sha256: after
                    .get(&path_key(&path(&["Vault", "New.md"])).expect("key"))
                    .cloned(),
            },
        ];
        assert_eq!(
            index.apply_watch_batch(0, &events, &second),
            Err(ObsidianIndexError::Stale)
        );
        assert_eq!(
            index.apply_watch_batch(initial.report.revision, &events[..2], &second),
            Err(ObsidianIndexError::InvalidInput)
        );
        let update = index
            .apply_watch_batch(initial.report.revision, &events, &second)
            .expect("watch update");
        assert_eq!(update.report.revision, 2);
        assert_eq!(update.receipt.operation, ObsidianAccessKind::WatchUpdate);
        assert_eq!(
            index.freshness(&second).expect("freshness"),
            ObsidianVaultFreshness::Current
        );
        assert_eq!(
            index.freshness(&first).expect("freshness"),
            ObsidianVaultFreshness::Stale
        );
    }

    #[test]
    fn interrupted_rebuild_rolls_back_to_the_complete_prior_projection() {
        let first = snapshot("", false, false);
        let second = snapshot("## Changed\n", false, false);
        let mut index = ObsidianVaultIndex::in_memory().expect("index");
        let initial = index.rebuild(&first).expect("initial").report;
        index
            .connection
            .execute_batch(
                "CREATE TRIGGER fail_vault_note BEFORE INSERT ON vault_notes
                 BEGIN SELECT RAISE(ABORT, 'synthetic'); END;",
            )
            .expect("trigger");
        assert_eq!(
            index.rebuild(&second),
            Err(ObsidianIndexError::StorageFailed)
        );
        index
            .connection
            .execute_batch("DROP TRIGGER fail_vault_note;")
            .expect("drop trigger");
        let retained = index.current_report().expect("retained");
        assert_eq!(retained.revision, initial.revision);
        assert_eq!(retained.index_sha256, initial.index_sha256);
        assert_eq!(
            index.freshness(&first).expect("freshness"),
            ObsidianVaultFreshness::Current
        );
    }

    #[test]
    fn projection_corruption_is_detected_before_query_or_preview() {
        let source = snapshot("", false, false);
        let mut index = ObsidianVaultIndex::in_memory().expect("index");
        index.rebuild(&source).expect("rebuild");
        index
            .connection
            .execute(
                "UPDATE vault_elements SET text='tampered' WHERE ordinal=0",
                [],
            )
            .expect("corrupt");
        assert_eq!(
            index.query("Meeting", 10),
            Err(ObsidianIndexError::CorruptIndex)
        );
        assert_eq!(
            index.preview_section_change(
                &source,
                &path(&["Vault", "Home.md"]),
                "Raw Notes",
                "replacement",
            ),
            Err(ObsidianIndexError::CorruptIndex)
        );
    }

    #[test]
    fn obvious_secret_candidates_remain_in_source_but_not_searchable_projection_text() {
        let sensitive = [
            "---\n",
            "pass",
            "word: synthetic-value\n",
            "---\n",
            "# Private Note\n",
        ]
        .concat();
        let source = ObsidianVaultSnapshot::from_snapshots(
            &selection(),
            vec![input(&["Vault", "Private.md"], &sensitive)],
        )
        .expect("snapshot");
        assert!(
            source.notes()[0]
                .coverage
                .iter()
                .any(|item| item.syntax == "obvious_secret_candidate")
        );
        assert_eq!(source.notes()[0].source_bytes(), sensitive.as_bytes());
        let mut index = ObsidianVaultIndex::in_memory().expect("index");
        index.rebuild(&source).expect("rebuild");
        assert!(
            index
                .query("synthetic-value", 10)
                .expect("query")
                .hits
                .is_empty()
        );
    }

    #[test]
    fn canonical_commit_publishes_once_while_failure_and_uncertainty_preserve_index() {
        let base = ObsidianVaultSnapshot::from_snapshots(
            &selection(),
            vec![input(
                &["Vault", "Existing.md"],
                "---\nagentmage_id: knowledge-project-001\ntype: project\n---\n# Existing\n",
            )],
        )
        .expect("base snapshot");
        let mut index = ObsidianVaultIndex::in_memory().expect("index");
        let initial = index.rebuild(&base).expect("initial");
        let request = KnowledgeNoteCreateRequest {
            path: path(&["Vault", "New.md"]),
            stable_id: KnowledgeRecordId::parse("knowledge-decision-001").expect("identity"),
            workflow: KnowledgeWriteWorkflow::Decision,
            title: "New decision".to_owned(),
            properties: Vec::new(),
            sections: vec![
                KnowledgeSectionDraft {
                    heading: "Decision".to_owned(),
                    body: "Use the canonical-first path.".to_owned(),
                },
                KnowledgeSectionDraft {
                    heading: "Evidence".to_owned(),
                    body: "Synthetic local fixture.".to_owned(),
                },
            ],
            memory_promotion: None,
            line_ending: MarkdownLineEnding::Lf,
        };
        let preview = preview_knowledge_note_create(
            request,
            &KnowledgeNamespaceSnapshot {
                paths: vec![path(&["Vault", "Existing.md"])],
                stable_ids: vec![
                    KnowledgeRecordId::parse("knowledge-project-001").expect("identity"),
                ],
                link_targets: Vec::new(),
                canonical_snapshot_sha256: initial.report.snapshot_sha256.clone(),
                derived_index_revision: initial.report.revision,
            },
        )
        .expect("preview");
        let proposed = std::str::from_utf8(preview.proposed_markdown()).expect("utf8");
        let after = ObsidianVaultSnapshot::from_snapshots(
            &selection(),
            vec![
                input(
                    &["Vault", "Existing.md"],
                    "---\nagentmage_id: knowledge-project-001\ntype: project\n---\n# Existing\n",
                ),
                input(&["Vault", "New.md"], proposed),
            ],
        )
        .expect("after snapshot");
        let mutation = CanonicalKnowledgeMutation::from_create(&preview);
        let committed = decide_index_publication(
            mutation.clone(),
            CanonicalMarkdownWriteOutcome::Committed,
            Some(preview.proposed_source_sha256.clone()),
        );
        let published = index
            .publish_after_canonical_write(initial.report.revision, &committed, &after)
            .expect("published");
        assert_eq!(
            published.state,
            KnowledgeIndexPublicationState::ReadyAfterCommit
        );
        assert_eq!(published.report.revision, initial.report.revision + 1);
        assert!(published.update.is_some());
        assert_eq!(
            index.freshness(&after).expect("freshness"),
            ObsidianVaultFreshness::Current
        );

        let mut failure_index = ObsidianVaultIndex::in_memory().expect("failure index");
        let failure_initial = failure_index.rebuild(&base).expect("failure initial");
        let failed = decide_index_publication(
            mutation.clone(),
            CanonicalMarkdownWriteOutcome::FailedNoChange,
            None,
        );
        let preserved = failure_index
            .publish_after_canonical_write(failure_initial.report.revision, &failed, &base)
            .expect("preserved");
        assert_eq!(
            preserved.state,
            KnowledgeIndexPublicationState::PreservedAfterFailure
        );
        assert_eq!(preserved.report, failure_initial.report);
        assert!(preserved.update.is_none());

        let uncertain = decide_index_publication(
            mutation,
            CanonicalMarkdownWriteOutcome::Uncertain,
            Some(preview.proposed_source_sha256),
        );
        assert_eq!(
            failure_index.publish_after_canonical_write(
                failure_initial.report.revision,
                &uncertain,
                &after,
            ),
            Err(ObsidianIndexError::Stale)
        );
        assert_eq!(
            failure_index.current_report().expect("retained report"),
            failure_initial.report
        );
    }
}
