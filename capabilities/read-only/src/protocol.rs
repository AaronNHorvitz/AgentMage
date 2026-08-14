//! Pure deterministic execution over an already authorized sealed workspace projection.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write;

pub use agentmage_kernel_contracts::{SnapshotEntry, SnapshotEntryKind, WorkspaceSnapshot};
use agentmage_kernel_contracts::{WorkspaceId, WorkspacePath};
use serde::de::DeserializeOwned;
use serde::de::{self, Deserializer, MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::ReadOnlyToolKind;

/// Hard maximum exact objects admitted by one read-only call.
pub const MAX_READ_ONLY_FILES: u32 = 256;
/// Hard maximum projected input bytes admitted by one read-only call.
pub const MAX_READ_ONLY_INPUT_BYTES: u64 = 16 * 1024 * 1024;
/// Hard maximum recursive depth admitted by one read-only call.
pub const MAX_READ_ONLY_DEPTH: u16 = 32;
/// Hard maximum matches returned by one read-only call.
pub const MAX_READ_ONLY_MATCHES: u32 = 1_000;
/// Hard maximum serialized item bytes returned by one read-only call.
pub const MAX_READ_ONLY_OUTPUT_BYTES: u64 = 2 * 1024 * 1024;
/// Hard maximum nested tool-call depth admitted by the v0.1 dispatcher.
pub const MAX_READ_ONLY_CALL_DEPTH: u8 = 8;

/// Exact caller-selected limits, each constrained by a product hard ceiling.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReadOnlyLimits {
    /// Maximum observed files or directories.
    pub files: u32,
    /// Maximum total projected regular-file bytes.
    pub input_bytes: u64,
    /// Maximum descendant depth relative to each requested root.
    pub depth: u16,
    /// Maximum returned search matches.
    pub matches: u32,
    /// Maximum serialized result-item bytes.
    pub output_bytes: u64,
}

impl Default for ReadOnlyLimits {
    fn default() -> Self {
        Self {
            files: 128,
            input_bytes: 4 * 1024 * 1024,
            depth: 16,
            matches: 256,
            output_bytes: 1024 * 1024,
        }
    }
}

/// Explicit interpretation requested for projected file bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReadOnlyEncoding {
    /// Strict UTF-8 text; invalid input is never lossily decoded.
    Utf8,
    /// Binary bytes; content is never returned directly.
    Binary,
}

/// Closed request shared by every exact read-only tool identity.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReadOnlyRequest {
    /// Request schema version. Only version one is admitted.
    pub schema_version: u16,
    /// Exact workspace-relative component lists.
    pub paths: Vec<Vec<String>>,
    /// Exact case-sensitive search query when required by the tool.
    pub query: Option<String>,
    /// Optional zero-based byte offset for text reads.
    pub byte_offset: Option<u64>,
    /// Optional maximum byte count for text reads.
    pub byte_count: Option<u64>,
    /// Explicit byte interpretation.
    pub encoding: ReadOnlyEncoding,
    /// Exact caller-selected limits.
    pub limits: ReadOnlyLimits,
    /// Current nested call depth supplied by the kernel dispatcher.
    pub call_depth: u8,
}

/// Terminal state returned by every read-only attempt.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReadOnlyOutcome {
    /// The request completed with all requested results.
    Succeeded,
    /// The valid request found no matching object or content.
    NoResult,
    /// Policy or a hard bound denied execution.
    Denied,
    /// Some exact requested objects produced results and others could not.
    Partial,
    /// A declared result limit stopped otherwise valid processing.
    Truncated,
    /// The closed request bytes were malformed or used an unsupported shape.
    Malformed,
    /// The cancellation boundary stopped processing.
    Cancelled,
    /// The authorized projection was inconsistent or corrupt.
    Failed,
}

/// One typed result item from a read-only operation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ReadOnlyItem {
    /// One listed or recursively enumerated object.
    Entry {
        /// Canonical relative path.
        path: Vec<String>,
        /// Exact projected object kind.
        object_kind: SnapshotEntryKind,
    },
    /// One exact bounded UTF-8 range.
    Text {
        /// Canonical relative path.
        path: Vec<String>,
        /// Inclusive byte start in the exact file.
        byte_start: u64,
        /// Exclusive byte end in the exact file.
        byte_end: u64,
        /// Exact UTF-8 content.
        content: String,
        /// SHA-256 of the complete projected file.
        file_sha256: String,
    },
    /// One exact case-sensitive search match.
    Match {
        /// Canonical relative path.
        path: Vec<String>,
        /// One-based line, or zero for a filename match.
        line: u64,
        /// One-based UTF-8 character column, or zero for a filename match.
        column: u64,
        /// Exact matched text without surrounding private content.
        matched: String,
    },
    /// Content-free object metadata.
    Metadata {
        /// Canonical relative path.
        path: Vec<String>,
        /// Exact projected object kind.
        object_kind: SnapshotEntryKind,
        /// Exact regular-file size or zero for a directory.
        byte_len: u64,
        /// Stable executable-bit observation.
        executable: bool,
    },
    /// SHA-256 for one exact regular file.
    FileHash {
        /// Canonical relative path.
        path: Vec<String>,
        /// Exact regular-file size.
        byte_len: u64,
        /// Exact SHA-256 digest.
        sha256: String,
    },
    /// SHA-256 for one canonical bounded tree projection.
    TreeHash {
        /// Canonical relative root.
        path: Vec<String>,
        /// Number of canonical projected objects in the digest.
        entries: u32,
        /// SHA-256 of canonical path, kind, size, and file-digest records.
        sha256: String,
    },
    /// Content-free binary-file observation.
    BinaryMetadata {
        /// Canonical relative path.
        path: Vec<String>,
        /// Exact binary-file size.
        byte_len: u64,
        /// Exact SHA-256 digest.
        sha256: String,
        /// Deterministic format hint from fixed magic bytes.
        format_hint: String,
    },
}

/// Hash-bound terminal result produced before host or model rendering.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReadOnlyResult {
    /// Result schema version.
    pub schema_version: u16,
    /// Exact tool identity.
    pub tool: String,
    /// Closed terminal outcome.
    pub outcome: ReadOnlyOutcome,
    /// Deterministically ordered bounded result items.
    pub items: Vec<ReadOnlyItem>,
    /// Count of projected objects observed before termination.
    pub observed_files: u32,
    /// Count of projected regular-file bytes observed before termination.
    pub observed_bytes: u64,
    /// Exact serialized byte count of `items`.
    pub output_bytes: u64,
    /// Whether a declared result limit stopped processing.
    pub truncated: bool,
    /// SHA-256 of every preceding result field in canonical struct order.
    pub result_sha256: String,
}

impl ReadOnlyResult {
    /// Verifies schema, tool identity, bounds, outcome invariants, and the exact result digest.
    #[must_use]
    pub fn verify(&self, kind: ReadOnlyToolKind) -> bool {
        if self.schema_version != 1
            || self.tool != kind.id()
            || self.output_bytes > MAX_READ_ONLY_OUTPUT_BYTES
            || self.output_bytes
                != serde_json::to_vec(&self.items)
                    .map(|bytes| bytes.len() as u64)
                    .unwrap_or(u64::MAX)
            || self.truncated != (self.outcome == ReadOnlyOutcome::Truncated)
            || (matches!(
                self.outcome,
                ReadOnlyOutcome::Malformed | ReadOnlyOutcome::Denied | ReadOnlyOutcome::Failed
            ) && (!self.items.is_empty()
                || self.observed_files != 0
                || self.observed_bytes != 0))
            || (self.outcome == ReadOnlyOutcome::NoResult && !self.items.is_empty())
            || (matches!(
                self.outcome,
                ReadOnlyOutcome::Succeeded | ReadOnlyOutcome::Partial
            ) && self.items.is_empty())
        {
            return false;
        }
        result(
            kind,
            self.outcome,
            self.items.clone(),
            self.observed_files,
            self.observed_bytes,
            self.truncated,
        ) == *self
    }
}

/// Closed request-validation failure before any projection is processed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReadOnlyRequestError {
    /// JSON, required fields, duplicate keys, or the closed request shape is malformed.
    Malformed,
    /// The request exceeds a hard bound or conflicts with the exact selected tool.
    Denied,
}

impl ReadOnlyRequestError {
    /// Stable content-free error code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::Malformed => "read_only.request.malformed",
            Self::Denied => "read_only.request.denied",
        }
    }
}

/// Cancellation boundary checked before and during deterministic processing.
pub trait ReadOnlyCancellation {
    /// Returns true once the current attempt must stop.
    fn is_cancelled(&self) -> bool;
}

/// Cancellation boundary that never cancels, useful for bounded synchronous calls.
#[derive(Clone, Copy, Debug, Default)]
pub struct NeverCancelled;

impl ReadOnlyCancellation for NeverCancelled {
    fn is_cancelled(&self) -> bool {
        false
    }
}

/// Parses, validates, and executes one exact tool over a sealed projection.
///
/// Malformed or denied requests produce closed content-free results. No method in
/// this module opens a path, reads the environment, starts a process, contacts a
/// network, or mutates the supplied projection.
#[must_use]
pub fn execute_read_only(
    kind: ReadOnlyToolKind,
    request_bytes: &[u8],
    snapshot: &WorkspaceSnapshot,
    cancellation: &impl ReadOnlyCancellation,
) -> ReadOnlyResult {
    let request = match validate_read_only_request(kind, request_bytes) {
        Ok(request) => request,
        Err(ReadOnlyRequestError::Malformed) => {
            return result(kind, ReadOnlyOutcome::Malformed, Vec::new(), 0, 0, false);
        }
        Err(ReadOnlyRequestError::Denied) => {
            return result(kind, ReadOnlyOutcome::Denied, Vec::new(), 0, 0, false);
        }
    };
    if cancellation.is_cancelled() {
        return result(kind, ReadOnlyOutcome::Cancelled, Vec::new(), 0, 0, false);
    }
    let entries = match validated_entries(snapshot, request.limits) {
        Ok(entries) => entries,
        Err(ProjectionFailure::Denied) => {
            return result(kind, ReadOnlyOutcome::Denied, Vec::new(), 0, 0, false);
        }
        Err(ProjectionFailure::Failed) => {
            return result(kind, ReadOnlyOutcome::Failed, Vec::new(), 0, 0, false);
        }
    };
    execute(kind, &request, &entries, cancellation)
}

/// Parses and validates one exact closed request for one exact selected tool.
pub fn validate_read_only_request(
    kind: ReadOnlyToolKind,
    bytes: &[u8],
) -> Result<ReadOnlyRequest, ReadOnlyRequestError> {
    if bytes.is_empty() || bytes.len() > 64 * 1024 {
        return Err(ReadOnlyRequestError::Malformed);
    }
    let request: ReadOnlyRequest =
        parse_closed_json(bytes).map_err(|_| ReadOnlyRequestError::Malformed)?;
    valid_request(kind, &request)
        .then_some(request)
        .ok_or(ReadOnlyRequestError::Denied)
}

pub(crate) fn parse_closed_json<T: DeserializeOwned>(bytes: &[u8]) -> Result<T, ()> {
    serde_json::from_slice::<ClosedJson>(bytes).map_err(|_| ())?;
    serde_json::from_slice(bytes).map_err(|_| ())
}

fn valid_request(kind: ReadOnlyToolKind, request: &ReadOnlyRequest) -> bool {
    if request.schema_version != 1
        || request.paths.is_empty()
        || request.paths.len() > MAX_READ_ONLY_FILES as usize
        || request.limits.files == 0
        || request.limits.files > MAX_READ_ONLY_FILES
        || request.limits.input_bytes == 0
        || request.limits.input_bytes > MAX_READ_ONLY_INPUT_BYTES
        || request.limits.depth > MAX_READ_ONLY_DEPTH
        || request.limits.matches == 0
        || request.limits.matches > MAX_READ_ONLY_MATCHES
        || request.limits.output_bytes == 0
        || request.limits.output_bytes > MAX_READ_ONLY_OUTPUT_BYTES
        || request.call_depth > MAX_READ_ONLY_CALL_DEPTH
        || request
            .byte_offset
            .is_some_and(|value| value > request.limits.input_bytes)
        || request
            .byte_count
            .is_some_and(|value| value > request.limits.input_bytes)
        || request
            .paths
            .iter()
            .any(|path| canonical_path(path).is_none())
    {
        return false;
    }
    let search = matches!(
        kind,
        ReadOnlyToolKind::SearchFilenames | ReadOnlyToolKind::SearchText
    );
    let text = matches!(
        kind,
        ReadOnlyToolKind::ReadText | ReadOnlyToolKind::ReadMultiple | ReadOnlyToolKind::SearchText
    );
    let single = matches!(
        kind,
        ReadOnlyToolKind::ListDirectory
            | ReadOnlyToolKind::DirectoryTree
            | ReadOnlyToolKind::ReadText
            | ReadOnlyToolKind::HashFile
            | ReadOnlyToolKind::HashTree
            | ReadOnlyToolKind::BinaryMetadata
    );
    (!single || request.paths.len() == 1)
        && (search
            == request
                .query
                .as_ref()
                .is_some_and(|query| !query.is_empty() && query.len() <= 4_096))
        && (text == (request.encoding == ReadOnlyEncoding::Utf8))
        && (text || (request.byte_offset.is_none() && request.byte_count.is_none()))
}

#[derive(Clone, Copy)]
enum ProjectionFailure {
    Denied,
    Failed,
}

fn validated_entries(
    snapshot: &WorkspaceSnapshot,
    limits: ReadOnlyLimits,
) -> Result<Vec<&SnapshotEntry>, ProjectionFailure> {
    if snapshot.entries.len() > limits.files as usize {
        return Err(ProjectionFailure::Denied);
    }
    let mut total = 0_u64;
    let mut seen = BTreeSet::new();
    let mut entries = Vec::with_capacity(snapshot.entries.len());
    for entry in &snapshot.entries {
        if canonical_path(&entry.path).is_none()
            || !seen.insert(entry.path.clone())
            || (entry.kind == SnapshotEntryKind::Directory && !entry.bytes.is_empty())
        {
            return Err(ProjectionFailure::Failed);
        }
        total = total
            .checked_add(entry.bytes.len() as u64)
            .ok_or(ProjectionFailure::Denied)?;
        if total > limits.input_bytes {
            return Err(ProjectionFailure::Denied);
        }
        entries.push(entry);
    }
    entries.sort_by(|left, right| left.path.cmp(&right.path));
    Ok(entries)
}

fn execute(
    kind: ReadOnlyToolKind,
    request: &ReadOnlyRequest,
    entries: &[&SnapshotEntry],
    cancellation: &impl ReadOnlyCancellation,
) -> ReadOnlyResult {
    let mut items = Vec::new();
    let mut observed_files = 0_u32;
    let mut observed_bytes = 0_u64;
    let mut incomplete = false;
    let mut truncated = false;
    let mut tree_records: BTreeMap<Vec<String>, Vec<Vec<u8>>> = BTreeMap::new();

    for entry in entries {
        if cancellation.is_cancelled() {
            return result(
                kind,
                ReadOnlyOutcome::Cancelled,
                items,
                observed_files,
                observed_bytes,
                truncated,
            );
        }
        let roots = request
            .paths
            .iter()
            .filter_map(|root| relation(root, &entry.path, request.limits.depth))
            .collect::<Vec<_>>();
        if roots.is_empty() && !request.paths.iter().any(|path| path == &entry.path) {
            continue;
        }
        observed_files = observed_files.saturating_add(1);
        observed_bytes = observed_bytes.saturating_add(entry.bytes.len() as u64);
        match kind {
            ReadOnlyToolKind::ListDirectory => {
                if roots.contains(&1) {
                    push_item(
                        &mut items,
                        ReadOnlyItem::Entry {
                            path: entry.path.clone(),
                            object_kind: entry.kind,
                        },
                        request.limits.output_bytes,
                        &mut truncated,
                    );
                }
            }
            ReadOnlyToolKind::DirectoryTree => {
                if roots.iter().any(|depth| *depth > 0) {
                    push_item(
                        &mut items,
                        ReadOnlyItem::Entry {
                            path: entry.path.clone(),
                            object_kind: entry.kind,
                        },
                        request.limits.output_bytes,
                        &mut truncated,
                    );
                }
            }
            ReadOnlyToolKind::ReadText | ReadOnlyToolKind::ReadMultiple => {
                if !request.paths.iter().any(|path| path == &entry.path) {
                    continue;
                }
                match text_item(entry, request) {
                    Some(item) => push_item(
                        &mut items,
                        item,
                        request.limits.output_bytes,
                        &mut truncated,
                    ),
                    None => incomplete = true,
                }
            }
            ReadOnlyToolKind::SearchFilenames => {
                let query = request.query.as_deref().expect("validated search query");
                let name = entry.path.last().expect("validated path");
                if name.contains(query) {
                    push_item(
                        &mut items,
                        ReadOnlyItem::Match {
                            path: entry.path.clone(),
                            line: 0,
                            column: 0,
                            matched: query.to_owned(),
                        },
                        request.limits.output_bytes,
                        &mut truncated,
                    );
                    if items.len() >= request.limits.matches as usize {
                        truncated = true;
                    }
                }
            }
            ReadOnlyToolKind::SearchText => {
                let query = request.query.as_deref().expect("validated search query");
                let Ok(text) = std::str::from_utf8(&entry.bytes) else {
                    incomplete = true;
                    continue;
                };
                for (line_index, line) in text.split('\n').enumerate() {
                    for (byte_index, _) in line.match_indices(query) {
                        let column = line[..byte_index].chars().count() as u64 + 1;
                        push_item(
                            &mut items,
                            ReadOnlyItem::Match {
                                path: entry.path.clone(),
                                line: line_index as u64 + 1,
                                column,
                                matched: query.to_owned(),
                            },
                            request.limits.output_bytes,
                            &mut truncated,
                        );
                        if items.len() >= request.limits.matches as usize || truncated {
                            truncated = true;
                            break;
                        }
                    }
                    if truncated {
                        break;
                    }
                }
            }
            ReadOnlyToolKind::Metadata => {
                if request.paths.iter().any(|path| path == &entry.path) {
                    push_item(
                        &mut items,
                        ReadOnlyItem::Metadata {
                            path: entry.path.clone(),
                            object_kind: entry.kind,
                            byte_len: entry.bytes.len() as u64,
                            executable: entry.executable,
                        },
                        request.limits.output_bytes,
                        &mut truncated,
                    );
                }
            }
            ReadOnlyToolKind::HashFile => {
                if request.paths[0] == entry.path && entry.kind == SnapshotEntryKind::RegularFile {
                    push_item(
                        &mut items,
                        ReadOnlyItem::FileHash {
                            path: entry.path.clone(),
                            byte_len: entry.bytes.len() as u64,
                            sha256: sha256_hex(&entry.bytes),
                        },
                        request.limits.output_bytes,
                        &mut truncated,
                    );
                }
            }
            ReadOnlyToolKind::HashTree => {
                if roots.iter().any(|depth| *depth > 0) {
                    tree_records
                        .entry(request.paths[0].clone())
                        .or_default()
                        .push(tree_record(entry));
                }
            }
            ReadOnlyToolKind::BinaryMetadata => {
                if request.paths[0] == entry.path && entry.kind == SnapshotEntryKind::RegularFile {
                    push_item(
                        &mut items,
                        ReadOnlyItem::BinaryMetadata {
                            path: entry.path.clone(),
                            byte_len: entry.bytes.len() as u64,
                            sha256: sha256_hex(&entry.bytes),
                            format_hint: format_hint(&entry.bytes).to_owned(),
                        },
                        request.limits.output_bytes,
                        &mut truncated,
                    );
                }
            }
        }
        if truncated {
            break;
        }
    }

    if kind == ReadOnlyToolKind::HashTree && !tree_records.is_empty() {
        for (path, records) in tree_records {
            let mut digest = Sha256::new();
            for record in &records {
                digest.update((record.len() as u64).to_be_bytes());
                digest.update(record);
            }
            push_item(
                &mut items,
                ReadOnlyItem::TreeHash {
                    path,
                    entries: records.len() as u32,
                    sha256: hex_digest(digest.finalize()),
                },
                request.limits.output_bytes,
                &mut truncated,
            );
        }
    }
    if matches!(
        kind,
        ReadOnlyToolKind::ReadMultiple | ReadOnlyToolKind::Metadata
    ) {
        let returned = items.iter().map(item_path).collect::<BTreeSet<_>>();
        incomplete |= request.paths.iter().any(|path| !returned.contains(path));
    }
    let outcome = if truncated {
        ReadOnlyOutcome::Truncated
    } else if items.is_empty() {
        ReadOnlyOutcome::NoResult
    } else if incomplete {
        ReadOnlyOutcome::Partial
    } else {
        ReadOnlyOutcome::Succeeded
    };
    result(
        kind,
        outcome,
        items,
        observed_files,
        observed_bytes,
        truncated,
    )
}

fn text_item(entry: &SnapshotEntry, request: &ReadOnlyRequest) -> Option<ReadOnlyItem> {
    if entry.kind != SnapshotEntryKind::RegularFile {
        return None;
    }
    let offset = usize::try_from(request.byte_offset.unwrap_or(0)).ok()?;
    if offset > entry.bytes.len() || !entry.bytes.is_char_boundary(offset) {
        return None;
    }
    let requested = request
        .byte_count
        .and_then(|count| usize::try_from(count).ok())
        .unwrap_or(entry.bytes.len().saturating_sub(offset));
    let mut end = offset.saturating_add(requested).min(entry.bytes.len());
    while end > offset && !entry.bytes.is_char_boundary(end) {
        end -= 1;
    }
    let content = std::str::from_utf8(&entry.bytes[offset..end])
        .ok()?
        .to_owned();
    Some(ReadOnlyItem::Text {
        path: entry.path.clone(),
        byte_start: offset as u64,
        byte_end: end as u64,
        content,
        file_sha256: sha256_hex(&entry.bytes),
    })
}

trait ByteBoundary {
    fn is_char_boundary(&self, index: usize) -> bool;
}

impl ByteBoundary for [u8] {
    fn is_char_boundary(&self, index: usize) -> bool {
        index == 0
            || index == self.len()
            || self
                .get(index)
                .is_some_and(|byte| byte & 0b1100_0000 != 0b1000_0000)
    }
}

fn push_item(
    items: &mut Vec<ReadOnlyItem>,
    item: ReadOnlyItem,
    output_limit: u64,
    truncated: &mut bool,
) {
    let mut candidate = items.clone();
    candidate.push(item.clone());
    let size = serde_json::to_vec(&candidate)
        .map(|bytes| bytes.len() as u64)
        .unwrap_or(u64::MAX);
    if size > output_limit {
        *truncated = true;
    } else {
        items.push(item);
    }
}

fn relation(root: &[String], path: &[String], max_depth: u16) -> Option<u16> {
    let depth = path.len().checked_sub(root.len())?;
    (path.starts_with(root) && depth <= max_depth as usize)
        .then(|| u16::try_from(depth).ok())
        .flatten()
}

fn canonical_path(path: &[String]) -> Option<WorkspacePath> {
    WorkspacePath::new(
        WorkspaceId::from_raw("workspace-read-only-projection"),
        path.iter().cloned(),
    )
    .ok()
}

fn tree_record(entry: &SnapshotEntry) -> Vec<u8> {
    serde_json::to_vec(&(
        &entry.path,
        entry.kind,
        entry.bytes.len() as u64,
        (entry.kind == SnapshotEntryKind::RegularFile).then(|| sha256_hex(&entry.bytes)),
    ))
    .expect("closed tree record serializes")
}

fn item_path(item: &ReadOnlyItem) -> &Vec<String> {
    match item {
        ReadOnlyItem::Entry { path, .. }
        | ReadOnlyItem::Text { path, .. }
        | ReadOnlyItem::Match { path, .. }
        | ReadOnlyItem::Metadata { path, .. }
        | ReadOnlyItem::FileHash { path, .. }
        | ReadOnlyItem::TreeHash { path, .. }
        | ReadOnlyItem::BinaryMetadata { path, .. } => path,
    }
}

fn format_hint(bytes: &[u8]) -> &'static str {
    if bytes.starts_with(b"\x7fELF") {
        "elf"
    } else if bytes.starts_with(b"%PDF-") {
        "pdf"
    } else if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        "png"
    } else if bytes.starts_with(&[0xff, 0xd8, 0xff]) {
        "jpeg"
    } else if bytes.starts_with(b"PK\x03\x04") {
        "zip"
    } else if bytes.starts_with(b"\0asm") {
        "wasm"
    } else {
        "unknown"
    }
}

fn result(
    kind: ReadOnlyToolKind,
    outcome: ReadOnlyOutcome,
    items: Vec<ReadOnlyItem>,
    observed_files: u32,
    observed_bytes: u64,
    truncated: bool,
) -> ReadOnlyResult {
    #[derive(Serialize)]
    struct Material<'a> {
        schema_version: u16,
        tool: &'a str,
        outcome: ReadOnlyOutcome,
        items: &'a [ReadOnlyItem],
        observed_files: u32,
        observed_bytes: u64,
        output_bytes: u64,
        truncated: bool,
    }
    let output_bytes = serde_json::to_vec(&items)
        .map(|bytes| bytes.len() as u64)
        .unwrap_or(0);
    let material = Material {
        schema_version: 1,
        tool: kind.id(),
        outcome,
        items: &items,
        observed_files,
        observed_bytes,
        output_bytes,
        truncated,
    };
    let result_sha256 = serde_json::to_vec(&material)
        .map(|bytes| sha256_hex(&bytes))
        .unwrap_or_else(|_| sha256_hex(b"read-only-result-serialization-failed"));
    ReadOnlyResult {
        schema_version: 1,
        tool: kind.id().to_owned(),
        outcome,
        items,
        observed_files,
        observed_bytes,
        output_bytes,
        truncated,
        result_sha256,
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    hex_digest(Sha256::digest(bytes))
}

fn hex_digest(bytes: impl AsRef<[u8]>) -> String {
    let bytes = bytes.as_ref();
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        write!(&mut output, "{byte:02x}").expect("writing to String cannot fail");
    }
    output
}

struct ClosedJson;

impl<'de> Deserialize<'de> for ClosedJson {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(ClosedJsonVisitor)
    }
}

struct ClosedJsonVisitor;

impl<'de> Visitor<'de> for ClosedJsonVisitor {
    type Value = ClosedJson;

    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("one JSON value without duplicate object keys")
    }

    fn visit_bool<E>(self, _value: bool) -> Result<Self::Value, E> {
        Ok(ClosedJson)
    }

    fn visit_i64<E>(self, _value: i64) -> Result<Self::Value, E> {
        Ok(ClosedJson)
    }

    fn visit_u64<E>(self, _value: u64) -> Result<Self::Value, E> {
        Ok(ClosedJson)
    }

    fn visit_f64<E>(self, _value: f64) -> Result<Self::Value, E> {
        Ok(ClosedJson)
    }

    fn visit_str<E>(self, _value: &str) -> Result<Self::Value, E> {
        Ok(ClosedJson)
    }

    fn visit_string<E>(self, _value: String) -> Result<Self::Value, E> {
        Ok(ClosedJson)
    }

    fn visit_none<E>(self) -> Result<Self::Value, E> {
        Ok(ClosedJson)
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E> {
        Ok(ClosedJson)
    }

    fn visit_seq<A>(self, mut sequence: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        while sequence.next_element::<ClosedJson>()?.is_some() {}
        Ok(ClosedJson)
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut keys = BTreeSet::new();
        while let Some(key) = map.next_key::<String>()? {
            if !keys.insert(key) {
                return Err(de::Error::custom("duplicate JSON object key"));
            }
            map.next_value::<ClosedJson>()?;
        }
        Ok(ClosedJson)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::{
        MAX_READ_ONLY_CALL_DEPTH, NeverCancelled, ReadOnlyCancellation, ReadOnlyEncoding,
        ReadOnlyItem, ReadOnlyLimits, ReadOnlyOutcome, ReadOnlyRequest, SnapshotEntry,
        SnapshotEntryKind, WorkspaceSnapshot, execute_read_only,
    };
    use crate::ReadOnlyToolKind;

    fn snapshot() -> WorkspaceSnapshot {
        WorkspaceSnapshot {
            entries: vec![
                entry(&["src"], SnapshotEntryKind::Directory, b""),
                entry(
                    &["src", "lib.rs"],
                    SnapshotEntryKind::RegularFile,
                    b"fn alpha() {}\nfn beta() { alpha(); }\n",
                ),
                entry(
                    &["src", "data.bin"],
                    SnapshotEntryKind::RegularFile,
                    b"\x7fELF\x02\x01fixture",
                ),
                entry(
                    &["README.md"],
                    SnapshotEntryKind::RegularFile,
                    b"Alpha workspace\n",
                ),
            ],
        }
    }

    fn entry(path: &[&str], kind: SnapshotEntryKind, bytes: &[u8]) -> SnapshotEntry {
        SnapshotEntry {
            path: path.iter().map(|value| (*value).to_owned()).collect(),
            kind,
            bytes: bytes.to_vec(),
            executable: false,
        }
    }

    fn request(paths: &[&[&str]], encoding: ReadOnlyEncoding) -> ReadOnlyRequest {
        ReadOnlyRequest {
            schema_version: 1,
            paths: paths
                .iter()
                .map(|path| path.iter().map(|value| (*value).to_owned()).collect())
                .collect(),
            query: None,
            byte_offset: None,
            byte_count: None,
            encoding,
            limits: ReadOnlyLimits::default(),
            call_depth: 0,
        }
    }

    fn run(kind: ReadOnlyToolKind, request: &ReadOnlyRequest) -> super::ReadOnlyResult {
        execute_read_only(
            kind,
            &serde_json::to_vec(request).expect("request JSON"),
            &snapshot(),
            &NeverCancelled,
        )
    }

    #[test]
    fn every_tool_has_exact_deterministic_golden_behavior() {
        let list = run(
            ReadOnlyToolKind::ListDirectory,
            &request(&[&["src"]], ReadOnlyEncoding::Binary),
        );
        assert_eq!(list.outcome, ReadOnlyOutcome::Succeeded);
        assert_eq!(list.items.len(), 2);
        assert_eq!(
            list.items.iter().map(super::item_path).collect::<Vec<_>>(),
            [
                &vec!["src".to_owned(), "data.bin".to_owned()],
                &vec!["src".to_owned(), "lib.rs".to_owned()]
            ]
        );

        let tree = run(
            ReadOnlyToolKind::DirectoryTree,
            &request(&[&["src"]], ReadOnlyEncoding::Binary),
        );
        assert_eq!(tree.items.len(), 2);

        let text = run(
            ReadOnlyToolKind::ReadText,
            &request(&[&["src", "lib.rs"]], ReadOnlyEncoding::Utf8),
        );
        assert!(matches!(text.items[0], ReadOnlyItem::Text { .. }));
        assert!(text.verify(ReadOnlyToolKind::ReadText));
        let mut forged = text.clone();
        forged.result_sha256 = "0".repeat(64);
        assert!(!forged.verify(ReadOnlyToolKind::ReadText));
        assert!(!text.verify(ReadOnlyToolKind::HashFile));

        let multi = run(
            ReadOnlyToolKind::ReadMultiple,
            &request(
                &[&["README.md"], &["src", "lib.rs"]],
                ReadOnlyEncoding::Utf8,
            ),
        );
        assert_eq!(multi.items.len(), 2);

        let mut filename_request = request(&[&["src"]], ReadOnlyEncoding::Binary);
        filename_request.query = Some("lib".to_owned());
        let filenames = run(ReadOnlyToolKind::SearchFilenames, &filename_request);
        assert_eq!(filenames.items.len(), 1);

        let mut text_request = request(&[&["src"]], ReadOnlyEncoding::Utf8);
        text_request.query = Some("alpha".to_owned());
        let matches = run(ReadOnlyToolKind::SearchText, &text_request);
        assert_eq!(matches.items.len(), 2);

        let metadata = run(
            ReadOnlyToolKind::Metadata,
            &request(&[&["src"], &["src", "lib.rs"]], ReadOnlyEncoding::Binary),
        );
        assert_eq!(metadata.items.len(), 2);

        let file_hash = run(
            ReadOnlyToolKind::HashFile,
            &request(&[&["src", "lib.rs"]], ReadOnlyEncoding::Binary),
        );
        assert!(matches!(file_hash.items[0], ReadOnlyItem::FileHash { .. }));

        let tree_hash = run(
            ReadOnlyToolKind::HashTree,
            &request(&[&["src"]], ReadOnlyEncoding::Binary),
        );
        assert!(matches!(tree_hash.items[0], ReadOnlyItem::TreeHash { .. }));

        let binary = run(
            ReadOnlyToolKind::BinaryMetadata,
            &request(&[&["src", "data.bin"]], ReadOnlyEncoding::Binary),
        );
        assert!(matches!(
            binary.items[0],
            ReadOnlyItem::BinaryMetadata {
                ref format_hint,
                ..
            } if format_hint == "elf"
        ));

        assert_eq!(
            text,
            run(
                ReadOnlyToolKind::ReadText,
                &request(&[&["src", "lib.rs"]], ReadOnlyEncoding::Utf8)
            )
        );
    }

    #[test]
    fn malformed_duplicate_extra_unsupported_and_out_of_budget_requests_never_run() {
        for bytes in [
            b"{}".as_slice(),
            b"{\"schema_version\":1,\"schema_version\":1}".as_slice(),
            b"not-json".as_slice(),
        ] {
            assert_eq!(
                execute_read_only(
                    ReadOnlyToolKind::ReadText,
                    bytes,
                    &snapshot(),
                    &NeverCancelled
                )
                .outcome,
                ReadOnlyOutcome::Malformed
            );
        }
        let mut unsupported = request(&[&["README.md"]], ReadOnlyEncoding::Utf8);
        unsupported.schema_version = 2;
        assert_eq!(
            run(ReadOnlyToolKind::ReadText, &unsupported).outcome,
            ReadOnlyOutcome::Denied
        );
        let mut deep = request(&[&["README.md"]], ReadOnlyEncoding::Utf8);
        deep.call_depth = MAX_READ_ONLY_CALL_DEPTH + 1;
        assert_eq!(
            run(ReadOnlyToolKind::ReadText, &deep).outcome,
            ReadOnlyOutcome::Denied
        );
        let mut tiny = request(&[&["README.md"]], ReadOnlyEncoding::Utf8);
        tiny.limits.input_bytes = 1;
        assert_eq!(
            run(ReadOnlyToolKind::ReadText, &tiny).outcome,
            ReadOnlyOutcome::Denied
        );
    }

    struct CancelsAfter(AtomicUsize);

    impl ReadOnlyCancellation for CancelsAfter {
        fn is_cancelled(&self) -> bool {
            self.0.fetch_add(1, Ordering::SeqCst) > 1
        }
    }

    #[test]
    fn no_result_partial_truncated_cancelled_and_failed_are_explicit() {
        let absent = run(
            ReadOnlyToolKind::ReadText,
            &request(&[&["missing.txt"]], ReadOnlyEncoding::Utf8),
        );
        assert_eq!(absent.outcome, ReadOnlyOutcome::NoResult);

        let partial = run(
            ReadOnlyToolKind::ReadMultiple,
            &request(&[&["README.md"], &["missing.txt"]], ReadOnlyEncoding::Utf8),
        );
        assert_eq!(partial.outcome, ReadOnlyOutcome::Partial);

        let mut tiny_output = request(&[&["src"]], ReadOnlyEncoding::Binary);
        tiny_output.limits.output_bytes = 1;
        let truncated = run(ReadOnlyToolKind::DirectoryTree, &tiny_output);
        assert_eq!(truncated.outcome, ReadOnlyOutcome::Truncated);
        assert!(truncated.truncated);

        let cancelled = execute_read_only(
            ReadOnlyToolKind::DirectoryTree,
            &serde_json::to_vec(&request(&[&["src"]], ReadOnlyEncoding::Binary))
                .expect("request JSON"),
            &snapshot(),
            &CancelsAfter(AtomicUsize::new(0)),
        );
        assert_eq!(cancelled.outcome, ReadOnlyOutcome::Cancelled);

        let corrupt = WorkspaceSnapshot {
            entries: vec![
                entry(&["same"], SnapshotEntryKind::RegularFile, b"first"),
                entry(&["same"], SnapshotEntryKind::RegularFile, b"second"),
            ],
        };
        let failed = execute_read_only(
            ReadOnlyToolKind::ReadText,
            &serde_json::to_vec(&request(&[&["same"]], ReadOnlyEncoding::Utf8))
                .expect("request JSON"),
            &corrupt,
            &NeverCancelled,
        );
        assert_eq!(failed.outcome, ReadOnlyOutcome::Failed);
    }
}
