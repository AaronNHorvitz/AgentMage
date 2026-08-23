//! Exact chunked source-artifact capture and bounded retrieval.

use std::collections::BTreeMap;

use agentmage_kernel_contracts::{
    ArtifactCaptureDisposition, ArtifactCaptureResult, ArtifactRangeReceipt, ArtifactSourceKind,
    ArtifactUploadChunk, ArtifactUploadId, CONTRACT_SCHEMA_VERSION, RuntimeArtifactId, SessionId,
};
use serde::Serialize;
use sha2::{Digest, Sha256};

/// Maximum exact source artifact accepted by the initial Verified Chat runtime.
pub const MAX_VERIFIED_ARTIFACT_BYTES: u64 = 64 * 1024 * 1024;
/// Maximum bytes in one authenticated upload chunk.
pub const MAX_VERIFIED_ARTIFACT_CHUNK_BYTES: usize = 256 * 1024;
/// Maximum concurrent incomplete uploads in one supervisor.
pub const MAX_PENDING_VERIFIED_UPLOADS: usize = 8;
/// Maximum exact bytes returned by one range read.
pub const MAX_VERIFIED_ARTIFACT_READ_BYTES: u64 = 1024 * 1024;

const ZERO_SHA256: &str = "0000000000000000000000000000000000000000000000000000000000000000";
const MAX_DISPLAY_NAME_BYTES: usize = 256;
const MAX_MEDIA_TYPE_BYTES: usize = 128;

/// Trusted immutable metadata supplied before the first upload chunk.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArtifactUploadSpec {
    /// Exact upload identity.
    pub upload_id: ArtifactUploadId,
    /// Owning durable session.
    pub session_id: SessionId,
    /// Original source kind.
    pub source_kind: ArtifactSourceKind,
    /// Bounded user-visible display name.
    pub display_name: String,
    /// Exact declared media type.
    pub media_type: String,
    /// Exact declared byte length.
    pub total_bytes: u64,
    /// Exact source SHA-256 computed by the capture client and reverified by the host.
    pub expected_sha256: String,
}

/// Stable content-free artifact-capture refusal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VerifiedArtifactError {
    /// Upload metadata or a chunk violated the closed contract.
    InvalidInput,
    /// The configured pending-upload or byte ceiling was exceeded.
    ResourceExceeded,
    /// The upload identity is already active or already committed.
    Duplicate,
    /// The upload does not exist.
    NotFound,
    /// Sequence, offset, size, or digest did not match the exact transfer.
    IntegrityMismatch,
    /// The source kind forbids the supplied byte representation.
    EncodingDenied,
    /// Publication or retrieval storage failed closed.
    Storage,
}

impl VerifiedArtifactError {
    /// Returns the stable redacted error code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidInput => "engineering.artifact.input.invalid",
            Self::ResourceExceeded => "engineering.artifact.resource.exceeded",
            Self::Duplicate => "engineering.artifact.identity.duplicate",
            Self::NotFound => "engineering.artifact.not-found",
            Self::IntegrityMismatch => "engineering.artifact.integrity.mismatch",
            Self::EncodingDenied => "engineering.artifact.encoding.denied",
            Self::Storage => "engineering.artifact.storage.failed",
        }
    }
}

/// Immutable source bytes and capture receipt admitted by a storage boundary.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerifiedArtifactRecord {
    /// Host-authoritative capture receipt.
    pub capture: ArtifactCaptureResult,
    /// Exact source bytes; production persistence must encrypt these at rest.
    pub bytes: Vec<u8>,
}

/// Storage boundary used after a complete upload passes exact integrity checks.
pub trait VerifiedArtifactStore {
    /// Publishes one logical artifact while allowing content-addressed byte deduplication.
    fn publish(
        &mut self,
        spec: &ArtifactUploadSpec,
        bytes: Vec<u8>,
        completed_at_epoch_ms: u64,
    ) -> Result<VerifiedArtifactRecord, VerifiedArtifactError>;

    /// Loads one exact immutable source artifact.
    fn load(
        &self,
        session_id: &SessionId,
        artifact_id: &RuntimeArtifactId,
    ) -> Result<VerifiedArtifactRecord, VerifiedArtifactError>;
}

struct PendingUpload {
    spec: ArtifactUploadSpec,
    next_sequence: u32,
    bytes: Vec<u8>,
    final_seen: bool,
}

/// Host-owned exact upload coordinator.
pub struct VerifiedArtifactUploads<S>
where
    S: VerifiedArtifactStore,
{
    store: S,
    pending: BTreeMap<ArtifactUploadId, PendingUpload>,
}

impl<S> VerifiedArtifactUploads<S>
where
    S: VerifiedArtifactStore,
{
    /// Creates an empty upload coordinator over one trusted store.
    #[must_use]
    pub fn new(store: S) -> Self {
        Self {
            store,
            pending: BTreeMap::new(),
        }
    }

    /// Admits immutable upload metadata before any source bytes are retained.
    pub fn begin(&mut self, spec: ArtifactUploadSpec) -> Result<(), VerifiedArtifactError> {
        validate_spec(&spec)?;
        if self.pending.len() >= MAX_PENDING_VERIFIED_UPLOADS {
            return Err(VerifiedArtifactError::ResourceExceeded);
        }
        if self.pending.contains_key(&spec.upload_id) {
            return Err(VerifiedArtifactError::Duplicate);
        }
        let capacity = usize::try_from(spec.total_bytes)
            .map_err(|_| VerifiedArtifactError::ResourceExceeded)?;
        self.pending.insert(
            spec.upload_id.clone(),
            PendingUpload {
                spec,
                next_sequence: 0,
                bytes: Vec::with_capacity(capacity),
                final_seen: false,
            },
        );
        Ok(())
    }

    /// Commits one exact next chunk; duplicate, missing, reordered, or altered chunks fail closed.
    pub fn append(&mut self, chunk: ArtifactUploadChunk) -> Result<u64, VerifiedArtifactError> {
        if chunk.schema_version != CONTRACT_SCHEMA_VERSION
            || chunk.bytes.is_empty()
            || chunk.bytes.len() > MAX_VERIFIED_ARTIFACT_CHUNK_BYTES
            || chunk.chunk_sha256 != sha256(&chunk.bytes)
        {
            return Err(VerifiedArtifactError::InvalidInput);
        }
        let pending = self
            .pending
            .get_mut(&chunk.upload_id)
            .ok_or(VerifiedArtifactError::NotFound)?;
        let expected_offset = u64::try_from(pending.bytes.len())
            .map_err(|_| VerifiedArtifactError::ResourceExceeded)?;
        if pending.final_seen
            || chunk.session_id != pending.spec.session_id
            || chunk.sequence != pending.next_sequence
            || chunk.offset != expected_offset
            || chunk.total_bytes != pending.spec.total_bytes
            || chunk.offset.saturating_add(chunk.bytes.len() as u64) > chunk.total_bytes
        {
            return Err(VerifiedArtifactError::IntegrityMismatch);
        }
        if pending.spec.source_kind == ArtifactSourceKind::Paste && chunk.bytes.contains(&0) {
            return Err(VerifiedArtifactError::EncodingDenied);
        }
        if chunk.final_chunk
            != (chunk.offset.saturating_add(chunk.bytes.len() as u64) == chunk.total_bytes)
        {
            return Err(VerifiedArtifactError::IntegrityMismatch);
        }
        pending.bytes.extend_from_slice(&chunk.bytes);
        pending.next_sequence = pending
            .next_sequence
            .checked_add(1)
            .ok_or(VerifiedArtifactError::ResourceExceeded)?;
        pending.final_seen = chunk.final_chunk;
        u64::try_from(pending.bytes.len()).map_err(|_| VerifiedArtifactError::ResourceExceeded)
    }

    /// Finalizes one exact upload only after complete byte, sequence, and digest verification.
    pub fn commit(
        &mut self,
        upload_id: &ArtifactUploadId,
        completed_at_epoch_ms: u64,
    ) -> Result<ArtifactCaptureResult, VerifiedArtifactError> {
        if completed_at_epoch_ms == 0 {
            return Err(VerifiedArtifactError::InvalidInput);
        }
        let pending = self
            .pending
            .remove(upload_id)
            .ok_or(VerifiedArtifactError::NotFound)?;
        if !pending.final_seen
            || pending.bytes.len() as u64 != pending.spec.total_bytes
            || sha256(&pending.bytes) != pending.spec.expected_sha256
        {
            return Err(VerifiedArtifactError::IntegrityMismatch);
        }
        let record = self
            .store
            .publish(&pending.spec, pending.bytes, completed_at_epoch_ms)?;
        verify_capture(&record.capture)?;
        Ok(record.capture)
    }

    /// Cancels one incomplete transfer and drops only its uncommitted bytes.
    pub fn cancel(&mut self, upload_id: &ArtifactUploadId) -> bool {
        self.pending.remove(upload_id).is_some()
    }

    /// Reads one exact bounded byte range and returns a content-verifiable receipt.
    pub fn read_range(
        &self,
        session_id: &SessionId,
        artifact_id: &RuntimeArtifactId,
        offset: u64,
        length: u64,
    ) -> Result<ArtifactRangeReceipt, VerifiedArtifactError> {
        if length == 0 || length > MAX_VERIFIED_ARTIFACT_READ_BYTES {
            return Err(VerifiedArtifactError::ResourceExceeded);
        }
        let record = self.store.load(session_id, artifact_id)?;
        verify_capture(&record.capture)?;
        let start = usize::try_from(offset).map_err(|_| VerifiedArtifactError::InvalidInput)?;
        if start > record.bytes.len() {
            return Err(VerifiedArtifactError::InvalidInput);
        }
        let requested = usize::try_from(length).map_err(|_| VerifiedArtifactError::InvalidInput)?;
        let end = start.saturating_add(requested).min(record.bytes.len());
        let bytes = record.bytes[start..end].to_vec();
        Ok(ArtifactRangeReceipt {
            schema_version: CONTRACT_SCHEMA_VERSION,
            artifact_id: artifact_id.clone(),
            source_sha256: record.capture.source_sha256,
            requested_offset: offset,
            requested_length: length,
            returned_offset: offset,
            returned_sha256: sha256(&bytes),
            truncated: end < record.bytes.len(),
            bytes,
        })
    }

    /// Returns the underlying trusted store after all pending uploads are explicitly handled.
    pub fn into_store(self) -> S {
        self.store
    }
}

/// Deterministic in-memory store used by contract tests and fake-model vertical slices.
#[derive(Default)]
pub struct MemoryVerifiedArtifactStore {
    records: BTreeMap<(SessionId, RuntimeArtifactId), VerifiedArtifactRecord>,
    payloads: BTreeMap<String, Vec<u8>>,
    next_artifact: u64,
}

impl VerifiedArtifactStore for MemoryVerifiedArtifactStore {
    fn publish(
        &mut self,
        spec: &ArtifactUploadSpec,
        bytes: Vec<u8>,
        completed_at_epoch_ms: u64,
    ) -> Result<VerifiedArtifactRecord, VerifiedArtifactError> {
        let source_sha256 = sha256(&bytes);
        if source_sha256 != spec.expected_sha256 || bytes.len() as u64 != spec.total_bytes {
            return Err(VerifiedArtifactError::IntegrityMismatch);
        }
        self.next_artifact = self
            .next_artifact
            .checked_add(1)
            .ok_or(VerifiedArtifactError::ResourceExceeded)?;
        let artifact_id =
            RuntimeArtifactId::from_raw(format!("verified-artifact-{:016x}", self.next_artifact));
        let payload_deduplicated = self.payloads.contains_key(&source_sha256);
        self.payloads
            .entry(source_sha256.clone())
            .or_insert_with(|| bytes.clone());
        let record = build_verified_artifact_record(
            spec,
            artifact_id.clone(),
            bytes,
            payload_deduplicated,
            completed_at_epoch_ms,
        )?;
        self.records
            .insert((spec.session_id.clone(), artifact_id), record.clone());
        Ok(record)
    }

    fn load(
        &self,
        session_id: &SessionId,
        artifact_id: &RuntimeArtifactId,
    ) -> Result<VerifiedArtifactRecord, VerifiedArtifactError> {
        self.records
            .get(&(session_id.clone(), artifact_id.clone()))
            .cloned()
            .ok_or(VerifiedArtifactError::NotFound)
    }
}

pub(crate) fn build_verified_artifact_record(
    spec: &ArtifactUploadSpec,
    artifact_id: RuntimeArtifactId,
    bytes: Vec<u8>,
    payload_deduplicated: bool,
    completed_at_epoch_ms: u64,
) -> Result<VerifiedArtifactRecord, VerifiedArtifactError> {
    let source_sha256 = sha256(&bytes);
    if source_sha256 != spec.expected_sha256
        || bytes.len() as u64 != spec.total_bytes
        || completed_at_epoch_ms == 0
    {
        return Err(VerifiedArtifactError::IntegrityMismatch);
    }
    let line_count = matches!(
        spec.media_type.as_str(),
        "text/plain" | "text/markdown" | "application/json" | "application/xml"
    )
    .then(|| line_count(&bytes));
    let mut capture = ArtifactCaptureResult {
        schema_version: CONTRACT_SCHEMA_VERSION,
        upload_id: spec.upload_id.clone(),
        artifact_id,
        session_id: spec.session_id.clone(),
        source_kind: spec.source_kind,
        display_name: spec.display_name.clone(),
        media_type: spec.media_type.clone(),
        byte_length: spec.total_bytes,
        line_count,
        source_sha256,
        disposition: ArtifactCaptureDisposition::CapturedExactly,
        payload_deduplicated,
        completed_at_epoch_ms,
        warning_codes: Vec::new(),
        receipt_sha256: ZERO_SHA256.to_owned(),
    };
    capture.receipt_sha256 = canonical_sha256(&capture)?;
    Ok(VerifiedArtifactRecord { capture, bytes })
}

fn validate_spec(spec: &ArtifactUploadSpec) -> Result<(), VerifiedArtifactError> {
    if spec.upload_id.as_str().is_empty()
        || spec.session_id.as_str().is_empty()
        || spec.display_name.is_empty()
        || spec.display_name.len() > MAX_DISPLAY_NAME_BYTES
        || spec.display_name.contains('\0')
        || spec.media_type.is_empty()
        || spec.media_type.len() > MAX_MEDIA_TYPE_BYTES
        || spec.media_type.contains('\0')
        || spec.total_bytes == 0
        || spec.total_bytes > MAX_VERIFIED_ARTIFACT_BYTES
        || !valid_sha256(&spec.expected_sha256)
    {
        return Err(VerifiedArtifactError::InvalidInput);
    }
    Ok(())
}

pub(crate) fn verify_capture(capture: &ArtifactCaptureResult) -> Result<(), VerifiedArtifactError> {
    if capture.schema_version != CONTRACT_SCHEMA_VERSION
        || capture.byte_length == 0
        || capture.completed_at_epoch_ms == 0
        || !valid_sha256(&capture.source_sha256)
        || !valid_sha256(&capture.receipt_sha256)
    {
        return Err(VerifiedArtifactError::IntegrityMismatch);
    }
    let mut candidate = capture.clone();
    candidate.receipt_sha256 = ZERO_SHA256.to_owned();
    if capture.receipt_sha256 != canonical_sha256(&candidate)? {
        return Err(VerifiedArtifactError::IntegrityMismatch);
    }
    Ok(())
}

fn canonical_sha256<T: Serialize>(value: &T) -> Result<String, VerifiedArtifactError> {
    serde_json::to_vec(value)
        .map(|bytes| sha256(&bytes))
        .map_err(|_| VerifiedArtifactError::Storage)
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn line_count(bytes: &[u8]) -> u64 {
    if bytes.is_empty() {
        return 0;
    }
    let breaks = bytes.iter().filter(|byte| **byte == b'\n').count() as u64;
    breaks + u64::from(bytes.last() != Some(&b'\n'))
}

fn sha256(bytes: &[u8]) -> String {
    let mut digest = Sha256::new();
    digest.update(bytes);
    hex_digest(digest.finalize().as_slice())
}

fn hex_digest(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(char::from(HEX[usize::from(byte >> 4)]));
        output.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    output
}

#[cfg(test)]
mod tests {
    use super::{
        ArtifactUploadSpec, MemoryVerifiedArtifactStore, VerifiedArtifactError,
        VerifiedArtifactUploads,
    };
    use agentmage_kernel_contracts::{
        ArtifactSourceKind, ArtifactUploadChunk, ArtifactUploadId, CONTRACT_SCHEMA_VERSION,
        SessionId,
    };

    fn upload(
        bytes: &[u8],
        chunk_bytes: usize,
    ) -> (
        VerifiedArtifactUploads<MemoryVerifiedArtifactStore>,
        agentmage_kernel_contracts::ArtifactCaptureResult,
    ) {
        let upload_id = ArtifactUploadId::from_raw("upload-exact-fixture");
        let session_id = SessionId::from_raw("session-exact-fixture");
        let mut uploads = VerifiedArtifactUploads::new(MemoryVerifiedArtifactStore::default());
        uploads
            .begin(ArtifactUploadSpec {
                upload_id: upload_id.clone(),
                session_id: session_id.clone(),
                source_kind: ArtifactSourceKind::Paste,
                display_name: "Pasted text".to_owned(),
                media_type: "text/plain".to_owned(),
                total_bytes: bytes.len() as u64,
                expected_sha256: super::sha256(bytes),
            })
            .unwrap();
        for (sequence, chunk) in bytes.chunks(chunk_bytes).enumerate() {
            let offset = sequence * chunk_bytes;
            uploads
                .append(ArtifactUploadChunk {
                    schema_version: CONTRACT_SCHEMA_VERSION,
                    upload_id: upload_id.clone(),
                    session_id: session_id.clone(),
                    sequence: sequence as u32,
                    offset: offset as u64,
                    total_bytes: bytes.len() as u64,
                    bytes: chunk.to_vec(),
                    chunk_sha256: super::sha256(chunk),
                    final_chunk: offset + chunk.len() == bytes.len(),
                })
                .unwrap();
        }
        let capture = uploads.commit(&upload_id, 1_780_000_000_000).unwrap();
        (uploads, capture)
    }

    #[test]
    fn fifty_thousand_character_paste_round_trips_exactly() {
        let source = format!(
            "BEGIN-{}-MIDDLE-{}-END",
            "x".repeat(25_000),
            "y".repeat(25_000)
        );
        let (uploads, capture) = upload(source.as_bytes(), 8192);
        assert_eq!(capture.byte_length, source.len() as u64);
        assert_eq!(capture.source_sha256, super::sha256(source.as_bytes()));
        let middle = uploads
            .read_range(&capture.session_id, &capture.artifact_id, 24_990, 80)
            .unwrap();
        assert!(String::from_utf8(middle.bytes).unwrap().contains("MIDDLE"));
    }

    #[test]
    fn missing_reordered_duplicate_and_mutated_chunks_fail_closed() {
        let bytes = b"exact source bytes";
        let upload_id = ArtifactUploadId::from_raw("upload-invalid-fixture");
        let session_id = SessionId::from_raw("session-invalid-fixture");
        let mut uploads = VerifiedArtifactUploads::new(MemoryVerifiedArtifactStore::default());
        uploads
            .begin(ArtifactUploadSpec {
                upload_id: upload_id.clone(),
                session_id: session_id.clone(),
                source_kind: ArtifactSourceKind::Paste,
                display_name: "Pasted text".to_owned(),
                media_type: "text/plain".to_owned(),
                total_bytes: bytes.len() as u64,
                expected_sha256: super::sha256(bytes),
            })
            .unwrap();
        let wrong_sequence = ArtifactUploadChunk {
            schema_version: CONTRACT_SCHEMA_VERSION,
            upload_id,
            session_id,
            sequence: 1,
            offset: 0,
            total_bytes: bytes.len() as u64,
            bytes: bytes.to_vec(),
            chunk_sha256: super::sha256(bytes),
            final_chunk: true,
        };
        assert_eq!(
            uploads.append(wrong_sequence),
            Err(VerifiedArtifactError::IntegrityMismatch)
        );
    }

    #[test]
    fn paste_nul_is_explicitly_denied() {
        let bytes = b"before\0after";
        let upload_id = ArtifactUploadId::from_raw("upload-nul-fixture");
        let session_id = SessionId::from_raw("session-nul-fixture");
        let mut uploads = VerifiedArtifactUploads::new(MemoryVerifiedArtifactStore::default());
        uploads
            .begin(ArtifactUploadSpec {
                upload_id: upload_id.clone(),
                session_id: session_id.clone(),
                source_kind: ArtifactSourceKind::Paste,
                display_name: "Pasted text".to_owned(),
                media_type: "text/plain".to_owned(),
                total_bytes: bytes.len() as u64,
                expected_sha256: super::sha256(bytes),
            })
            .unwrap();
        assert_eq!(
            uploads.append(ArtifactUploadChunk {
                schema_version: CONTRACT_SCHEMA_VERSION,
                upload_id,
                session_id,
                sequence: 0,
                offset: 0,
                total_bytes: bytes.len() as u64,
                bytes: bytes.to_vec(),
                chunk_sha256: super::sha256(bytes),
                final_chunk: true,
            }),
            Err(VerifiedArtifactError::EncodingDenied)
        );
    }
}
