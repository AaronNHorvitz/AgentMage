//! Closed runtime artifact manifests, references, and resume bindings.

use std::collections::{BTreeMap, BTreeSet};
use std::io::Read;

use agentmage_kernel_contracts::{
    AgentStateKind, CONTRACT_SCHEMA_VERSION, ContextSensitivity, RuntimeArtifactCleanupState,
    RuntimeArtifactId, RuntimeArtifactIntegrityState, RuntimeArtifactKind,
    RuntimeArtifactLifecycleState, RuntimeArtifactManifest, RuntimeArtifactOperatorView,
    RuntimeArtifactRef, RuntimeContinuationState, RuntimeEventRetentionKind,
    RuntimePayloadReference, RuntimeResumeBinding, SessionCheckpoint, SessionId, TaskId, from_json,
    to_canonical_json,
};
use rusqlite::{Connection, OptionalExtension, Transaction, TransactionBehavior, params};
use sha2::{Digest, Sha256};

use crate::operational_store::OperationalStore;

const ZERO_SHA256: &str = "0000000000000000000000000000000000000000000000000000000000000000";
/// Maximum bytes admitted for one runtime payload in the initial store profile.
pub const MAX_RUNTIME_ARTIFACT_BYTES: u64 = 64 * 1024 * 1024;
/// Maximum UTF-8 bytes retained in one encrypted artifact preview.
pub const MAX_RUNTIME_ARTIFACT_PREVIEW_BYTES: usize = 4 * 1024;
/// Maximum exact artifact references bound to one resumable checkpoint.
pub const MAX_RUNTIME_ARTIFACTS_PER_CHECKPOINT: usize = 1_024;
/// Exact media type used only for sealed runtime continuation-state artifacts.
pub const RUNTIME_CONTINUATION_MEDIA_TYPE: &str =
    "application/vnd.agentmage.runtime-continuation+json";
const MAX_RUNTIME_CONTINUATION_RESULTS: usize = 1_024;
const MAX_RUNTIME_CONTINUATION_TRANSITIONS: usize = 4_096;

/// Stable fail-closed artifact-contract result.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuntimeArtifactError {
    /// A field, relationship, size, retention, or media type is outside the closed contract.
    InvalidManifest,
    /// A path-free reference does not match its verified immutable manifest.
    ReferenceMismatch,
    /// A resumable checkpoint binding is malformed, unordered, duplicated, or inconsistent.
    InvalidResumeBinding,
    /// A persisted coordinator continuation is malformed, unsafe, or internally inconsistent.
    InvalidContinuation,
    /// Canonical contract serialization failed.
    Serialization,
    /// A canonical manifest or resume-binding digest does not match.
    DigestMismatch,
}

/// Closed platform-payload failure visible to the kernel without paths or raw content.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuntimeArtifactPayloadError {
    /// The staging request or returned observation was malformed.
    Invalid,
    /// The payload exceeded a declared byte, object, or allocation ceiling.
    ResourceLimit,
    /// The exact content-addressed object does not exist.
    Missing,
    /// Retained bytes do not match their immutable digest or size.
    Corrupt,
    /// An existing staging or object identity conflicts with the request.
    Conflict,
    /// A write, synchronization, placement, quarantine, or deletion failed.
    Durability,
    /// The continuously held private data root failed revalidation.
    UnsafeRoot,
}

impl RuntimeArtifactPayloadError {
    /// Returns one stable content-free diagnostic code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::Invalid => "runtime.artifact.payload_invalid",
            Self::ResourceLimit => "runtime.artifact.payload_limit",
            Self::Missing => "runtime.artifact.payload_missing",
            Self::Corrupt => "runtime.artifact.payload_corrupt",
            Self::Conflict => "runtime.artifact.payload_conflict",
            Self::Durability => "runtime.artifact.payload_durability",
            Self::UnsafeRoot => "runtime.artifact.payload_root_unsafe",
        }
    }
}

/// Content-free digest and size observed for staged or retained payload bytes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeArtifactPayloadObservation {
    /// Lowercase SHA-256 digest of the complete payload.
    pub payload_sha256: String,
    /// Exact complete payload size.
    pub byte_size: u64,
}

/// Result of atomic placement into the private content-addressed object namespace.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeArtifactPayloadPlacement {
    /// Verified immutable object identity after placement.
    pub observation: RuntimeArtifactPayloadObservation,
    /// Whether an equal existing object was reused instead of replaced.
    pub deduplicated: bool,
}

/// One path-free object returned by a private payload-store inventory.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeArtifactPayloadInventoryEntry {
    /// Lowercase SHA-256 object identity.
    pub payload_sha256: String,
    /// Exact plaintext byte size for a verified object, otherwise zero.
    pub byte_size: u64,
    /// Whether complete authenticated verification succeeded during inventory.
    pub integrity: RuntimeArtifactPayloadInventoryIntegrity,
}

/// Closed integrity result for one path-free private payload inventory entry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuntimeArtifactPayloadInventoryIntegrity {
    /// The complete authenticated payload matched its content-addressed name.
    Verified,
    /// The named object exists but its encrypted representation cannot be trusted.
    Corrupt,
}

/// Platform-owned private payload effects consumed only by the trusted runtime boundary.
pub trait RuntimeArtifactPayloadStore {
    /// Opaque one-use staged object retained only by the platform adapter.
    type Staged;

    /// Streams one bounded payload into private staging and returns its complete observation.
    fn stage(
        &mut self,
        artifact_id: &RuntimeArtifactId,
        source: &mut dyn Read,
        maximum_bytes: u64,
    ) -> Result<(Self::Staged, RuntimeArtifactPayloadObservation), RuntimeArtifactPayloadError>;

    /// Removes one staged object after validation refuses publication.
    fn discard_staged(&mut self, staged: Self::Staged) -> Result<(), RuntimeArtifactPayloadError>;

    /// Atomically places a staged object without replacing an unequal retained object.
    fn place(
        &mut self,
        staged: Self::Staged,
        expected: &RuntimeArtifactPayloadObservation,
    ) -> Result<RuntimeArtifactPayloadPlacement, RuntimeArtifactPayloadError>;

    /// Re-reads and verifies one complete immutable object.
    fn verify(
        &self,
        expected: &RuntimeArtifactPayloadObservation,
    ) -> Result<(), RuntimeArtifactPayloadError>;

    /// Reads one complete object only when it fits the caller's declared ceiling.
    fn read_complete(
        &self,
        expected: &RuntimeArtifactPayloadObservation,
        maximum_bytes: u64,
    ) -> Result<Vec<u8>, RuntimeArtifactPayloadError>;

    /// Reads one bounded range after verifying the complete immutable object identity.
    fn read_range(
        &self,
        expected: &RuntimeArtifactPayloadObservation,
        offset: u64,
        maximum_bytes: u64,
    ) -> Result<Vec<u8>, RuntimeArtifactPayloadError>;

    /// Isolates a corrupt or uncertain retained object from the active namespace.
    fn quarantine(
        &mut self,
        expected: &RuntimeArtifactPayloadObservation,
    ) -> Result<(), RuntimeArtifactPayloadError>;

    /// Removes one exact retained object without accepting a path from the caller.
    fn delete(
        &mut self,
        expected: &RuntimeArtifactPayloadObservation,
    ) -> Result<(), RuntimeArtifactPayloadError>;

    /// Returns a sorted complete inventory of active content-addressed objects.
    fn inventory(
        &self,
    ) -> Result<Vec<RuntimeArtifactPayloadInventoryEntry>, RuntimeArtifactPayloadError>;

    /// Removes every interrupted staging object not represented by canonical metadata.
    fn cleanup_staging(&mut self) -> Result<u64, RuntimeArtifactPayloadError>;
}

/// Stable artifact-store failure spanning contract, payload, metadata, and authority checks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuntimeArtifactStoreError {
    /// The path-free manifest, reference, or resume binding failed its closed contract.
    Contract(RuntimeArtifactError),
    /// The platform payload store failed without exposing a native path.
    Payload(RuntimeArtifactPayloadError),
    /// Encrypted canonical metadata could not be committed safely.
    Storage,
    /// Retained metadata, hash chains, or payload observations are inconsistent.
    Integrity,
    /// The requested artifact does not exist in canonical metadata.
    NotFound,
    /// Session, task, policy, retention, or lifecycle state denies access.
    NotAuthorized,
}

impl RuntimeArtifactStoreError {
    /// Returns one stable content-free diagnostic code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::Contract(error) => error.code(),
            Self::Payload(error) => error.code(),
            Self::Storage => "runtime.artifact.metadata_storage",
            Self::Integrity => "runtime.artifact.metadata_integrity",
            Self::NotFound => "runtime.artifact.not_found",
            Self::NotAuthorized => "runtime.artifact.not_authorized",
        }
    }

    /// Returns whether the current in-process authority must reopen before another effect.
    #[must_use]
    pub const fn poisons_runtime(self) -> bool {
        matches!(self, Self::Storage | Self::Integrity)
            || matches!(
                self,
                Self::Payload(
                    RuntimeArtifactPayloadError::Corrupt
                        | RuntimeArtifactPayloadError::Conflict
                        | RuntimeArtifactPayloadError::Durability
                        | RuntimeArtifactPayloadError::UnsafeRoot
                )
            )
    }
}

impl From<RuntimeArtifactError> for RuntimeArtifactStoreError {
    fn from(error: RuntimeArtifactError) -> Self {
        Self::Contract(error)
    }
}

impl From<RuntimeArtifactPayloadError> for RuntimeArtifactStoreError {
    fn from(error: RuntimeArtifactPayloadError) -> Self {
        Self::Payload(error)
    }
}

/// Verified publication result returned without payload bytes or path authority.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeArtifactPublication {
    /// Immutable manifest committed to encrypted metadata.
    pub manifest: RuntimeArtifactManifest,
    /// Path-free reference suitable for events and checkpoints.
    pub reference: RuntimeArtifactRef,
    /// Whether equal payload bytes already occupied the content address.
    pub payload_deduplicated: bool,
}

/// Privacy-safe current metadata projection for one artifact.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeArtifactState {
    /// Path-free immutable artifact reference.
    pub reference: RuntimeArtifactRef,
    /// Current metadata lifecycle state.
    pub lifecycle: RuntimeArtifactLifecycleState,
    /// Current payload integrity state.
    pub integrity: RuntimeArtifactIntegrityState,
    /// Monotonic lifecycle revision.
    pub revision: u64,
    /// Stable content-free reason code for the current state.
    pub reason_code: String,
    /// Last trusted lifecycle-transition time.
    pub updated_at_epoch_ms: u64,
}

/// Exact owner, policy, time, and resource bindings for one complete artifact read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeArtifactReadRequest {
    /// Owning session that must match immutable artifact metadata.
    pub session_id: SessionId,
    /// Owning task that must match immutable artifact metadata.
    pub task_id: TaskId,
    /// Exact current policy revision digest.
    pub policy_sha256: String,
    /// Complete path-free artifact reference.
    pub reference: RuntimeArtifactRef,
    /// Trusted read time used for expiration enforcement.
    pub now_epoch_ms: u64,
    /// Maximum complete payload bytes the caller is prepared to accept.
    pub maximum_bytes: u64,
}

/// Exact owner, policy, time, range, and resource bindings for one artifact preview page.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeArtifactPageRequest {
    /// Owning session that must match immutable artifact metadata.
    pub session_id: SessionId,
    /// Owning task that must match immutable artifact metadata.
    pub task_id: TaskId,
    /// Exact current policy revision digest.
    pub policy_sha256: String,
    /// Complete path-free artifact reference.
    pub reference: RuntimeArtifactRef,
    /// Trusted read time used for expiration enforcement.
    pub now_epoch_ms: u64,
    /// Zero-based payload byte offset.
    pub offset: u64,
    /// Maximum bytes returned by this page.
    pub maximum_bytes: u32,
}

/// One bounded verified artifact page without path or future-read authority.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeArtifactPage {
    /// Exact path-free artifact reference used for the read.
    pub reference: RuntimeArtifactRef,
    /// Zero-based offset of the first returned byte.
    pub offset: u64,
    /// Bounded immutable payload bytes for this page.
    pub bytes: Vec<u8>,
    /// Lowercase SHA-256 digest of the exact page bytes.
    pub page_sha256: String,
    /// Next page offset, absent when this page reaches the complete payload size.
    pub next_offset: Option<u64>,
    /// Whether this page reaches the complete immutable payload size.
    pub complete: bool,
}

/// Content-free startup reconciliation counts.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RuntimeArtifactReconciliation {
    /// Active payloads verified exactly.
    pub verified_payloads: u64,
    /// Missing or corrupt payload identities moved to blocked metadata state.
    pub quarantined_payloads: u64,
    /// Unreferenced or already-deleted objects removed from active storage.
    pub deleted_orphans: u64,
    /// Interrupted private staging objects removed.
    pub cleaned_staging: u64,
}

/// Content-free canonical row counts for artifact storage governance.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RuntimeArtifactRowCounts {
    /// Distinct retained payload metadata rows, including deleted history.
    pub payloads: u64,
    /// Immutable logical artifact manifest rows.
    pub artifacts: u64,
    /// Append-only artifact lifecycle event rows.
    pub lifecycle_events: u64,
    /// Retained runtime resume binding rows.
    pub resume_bindings: u64,
    /// Artifact-reference rows across retained resume bindings.
    pub resume_artifacts: u64,
}

impl RuntimeArtifactError {
    /// Returns a stable content-free diagnostic code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidManifest => "runtime.artifact.manifest_invalid",
            Self::ReferenceMismatch => "runtime.artifact.reference_mismatch",
            Self::InvalidResumeBinding => "runtime.artifact.resume_binding_invalid",
            Self::InvalidContinuation => "runtime.artifact.continuation_invalid",
            Self::Serialization => "runtime.artifact.serialization_failed",
            Self::DigestMismatch => "runtime.artifact.digest_mismatch",
        }
    }
}

/// Seals an otherwise valid immutable artifact manifest.
pub fn seal_runtime_artifact_manifest(
    mut manifest: RuntimeArtifactManifest,
) -> Result<RuntimeArtifactManifest, RuntimeArtifactError> {
    manifest.manifest_sha256 = ZERO_SHA256.to_owned();
    validate_manifest_fields(&manifest)?;
    manifest.manifest_sha256 = manifest_digest(&manifest)?;
    Ok(manifest)
}

/// Verifies all fields and the canonical digest of one immutable artifact manifest.
pub fn verify_runtime_artifact_manifest(
    manifest: &RuntimeArtifactManifest,
) -> Result<(), RuntimeArtifactError> {
    validate_manifest_fields(manifest)?;
    let expected = manifest_digest(manifest)?;
    if manifest.manifest_sha256 != expected {
        return Err(RuntimeArtifactError::DigestMismatch);
    }
    Ok(())
}

/// Creates the path-free reference corresponding to one verified manifest.
pub fn runtime_artifact_ref(
    manifest: &RuntimeArtifactManifest,
) -> Result<RuntimeArtifactRef, RuntimeArtifactError> {
    verify_runtime_artifact_manifest(manifest)?;
    Ok(RuntimeArtifactRef {
        schema_version: CONTRACT_SCHEMA_VERSION,
        artifact_id: manifest.artifact_id.clone(),
        manifest_sha256: manifest.manifest_sha256.clone(),
        payload_sha256: manifest.payload_sha256.clone(),
        byte_size: manifest.byte_size,
        media_type: manifest.media_type.clone(),
    })
}

/// Verifies that a path-free reference resolves exactly to one manifest.
pub fn verify_runtime_artifact_ref(
    reference: &RuntimeArtifactRef,
    manifest: &RuntimeArtifactManifest,
) -> Result<(), RuntimeArtifactError> {
    verify_runtime_artifact_manifest(manifest)?;
    if reference.schema_version != CONTRACT_SCHEMA_VERSION
        || reference.artifact_id != manifest.artifact_id
        || reference.manifest_sha256 != manifest.manifest_sha256
        || reference.payload_sha256 != manifest.payload_sha256
        || reference.byte_size != manifest.byte_size
        || reference.media_type != manifest.media_type
    {
        return Err(RuntimeArtifactError::ReferenceMismatch);
    }
    Ok(())
}

/// Projects a verified manifest to the smaller runtime-event payload reference.
pub fn runtime_payload_reference(
    manifest: &RuntimeArtifactManifest,
) -> Result<RuntimePayloadReference, RuntimeArtifactError> {
    let reference = runtime_artifact_ref(manifest)?;
    Ok(RuntimePayloadReference {
        artifact_id: reference.artifact_id,
        sha256: reference.payload_sha256,
        byte_size: reference.byte_size,
        media_type: reference.media_type,
    })
}

/// Seals an otherwise valid checkpoint-to-runtime persistence binding.
pub fn seal_runtime_resume_binding(
    mut binding: RuntimeResumeBinding,
) -> Result<RuntimeResumeBinding, RuntimeArtifactError> {
    binding.binding_sha256 = ZERO_SHA256.to_owned();
    validate_resume_binding_fields(&binding)?;
    binding.binding_sha256 = resume_binding_digest(&binding)?;
    Ok(binding)
}

/// Verifies one exact checkpoint cursor and artifact-reference set.
pub fn verify_runtime_resume_binding(
    binding: &RuntimeResumeBinding,
) -> Result<(), RuntimeArtifactError> {
    validate_resume_binding_fields(binding)?;
    let expected = resume_binding_digest(binding)?;
    if binding.binding_sha256 != expected {
        return Err(RuntimeArtifactError::DigestMismatch);
    }
    Ok(())
}

/// Seals one safe-boundary coordinator continuation after canonical collection ordering.
pub fn seal_runtime_continuation_state(
    mut continuation: RuntimeContinuationState,
) -> Result<RuntimeContinuationState, RuntimeArtifactError> {
    continuation
        .evidence
        .sort_by(|left, right| left.evidence_id.as_str().cmp(right.evidence_id.as_str()));
    continuation
        .receipt_ids
        .sort_by(|left, right| left.as_str().cmp(right.as_str()));
    continuation
        .artifacts
        .sort_by(|left, right| left.artifact_id.as_str().cmp(right.artifact_id.as_str()));
    continuation.continuation_sha256 = ZERO_SHA256.to_owned();
    validate_runtime_continuation_state(&continuation)?;
    continuation.continuation_sha256 = continuation_digest(&continuation)?;
    Ok(continuation)
}

/// Verifies one exact safe-boundary coordinator continuation and its canonical digest.
pub fn verify_runtime_continuation_state(
    continuation: &RuntimeContinuationState,
) -> Result<(), RuntimeArtifactError> {
    validate_runtime_continuation_state(continuation)?;
    if continuation.continuation_sha256 != continuation_digest(continuation)? {
        return Err(RuntimeArtifactError::DigestMismatch);
    }
    Ok(())
}

/// Encodes one verified continuation canonically up to the private artifact ceiling.
pub fn encode_runtime_continuation_state(
    continuation: &RuntimeContinuationState,
) -> Result<Vec<u8>, RuntimeArtifactError> {
    verify_runtime_continuation_state(continuation)?;
    continuation_json(continuation)
}

/// Decodes one canonical continuation artifact and verifies every internal binding.
pub fn decode_runtime_continuation_state(
    bytes: &[u8],
) -> Result<RuntimeContinuationState, RuntimeArtifactError> {
    if bytes.is_empty() || bytes.len() as u64 > MAX_RUNTIME_ARTIFACT_BYTES {
        return Err(RuntimeArtifactError::InvalidContinuation);
    }
    let continuation = serde_json::from_slice::<RuntimeContinuationState>(bytes)
        .map_err(|_| RuntimeArtifactError::Serialization)?;
    verify_runtime_continuation_state(&continuation)?;
    if continuation_json(&continuation)? != bytes {
        return Err(RuntimeArtifactError::InvalidContinuation);
    }
    Ok(continuation)
}

/// Publishes one verified payload and immutable manifest through the canonical ordering.
pub(crate) fn publish_runtime_artifact<S: RuntimeArtifactPayloadStore>(
    store: &mut OperationalStore,
    payloads: &mut S,
    manifest: RuntimeArtifactManifest,
    source: &mut dyn Read,
) -> Result<RuntimeArtifactPublication, RuntimeArtifactStoreError> {
    verify_runtime_artifact_manifest(&manifest)?;
    verify_manifest_producer(&store.connection, &manifest)?;
    let expected = payload_observation(&manifest);
    let (staged, observed) =
        payloads.stage(&manifest.artifact_id, source, MAX_RUNTIME_ARTIFACT_BYTES)?;
    if observed != expected {
        payloads.discard_staged(staged)?;
        return Err(RuntimeArtifactStoreError::Payload(
            RuntimeArtifactPayloadError::Invalid,
        ));
    }
    let placement = payloads.place(staged, &expected)?;
    if placement.observation != expected {
        return Err(RuntimeArtifactStoreError::Payload(
            RuntimeArtifactPayloadError::Conflict,
        ));
    }
    let reference = runtime_artifact_ref(&manifest)?;
    persist_artifact_manifest(store, &manifest)?;
    payloads.verify(&expected)?;
    Ok(RuntimeArtifactPublication {
        manifest,
        reference,
        payload_deduplicated: placement.deduplicated,
    })
}

/// Persists one exact checkpoint cursor and artifact set inside its checkpoint transaction.
pub(crate) fn persist_runtime_resume_binding(
    transaction: &Transaction<'_>,
    checkpoint: &SessionCheckpoint,
    binding: &RuntimeResumeBinding,
) -> Result<(), RuntimeArtifactStoreError> {
    verify_runtime_resume_binding(binding)?;
    if binding.checkpoint_id != checkpoint.checkpoint_id
        || binding.checkpoint_sha256 != checkpoint.checkpoint_sha256
        || binding.session_id != checkpoint.session_id
        || binding.task_id != checkpoint.task_id
        || binding.event_cursor.run_id != binding.run_id
    {
        return Err(RuntimeArtifactStoreError::NotAuthorized);
    }
    let run = transaction
        .query_row(
            "SELECT session_id, task_id, policy_id, last_sequence, last_event_id, last_event_sha256
             FROM runtime_runs WHERE run_id = ?1",
            [binding.run_id.as_str()],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                ))
            },
        )
        .optional()
        .map_err(|_| RuntimeArtifactStoreError::Storage)?
        .ok_or(RuntimeArtifactStoreError::NotFound)?;
    if run.0 != binding.session_id.as_str()
        || run.1 != binding.task_id.as_str()
        || run.2 != checkpoint.policy_id.as_str()
        || u64::try_from(run.3).ok() != Some(binding.event_cursor.sequence)
        || run.4 != binding.event_cursor.event_id.as_str()
        || run.5 != binding.event_cursor.event_sha256
    {
        return Err(RuntimeArtifactStoreError::NotAuthorized);
    }
    for reference in &binding.artifacts {
        verify_resume_artifact_reference(transaction, checkpoint, binding, reference)?;
    }
    let record_json = to_canonical_json(binding)
        .map_err(|_| RuntimeArtifactStoreError::Contract(RuntimeArtifactError::Serialization))?;
    transaction
        .execute(
            "INSERT INTO runtime_resume_bindings(
                 checkpoint_sha256, checkpoint_id, session_id, task_id, run_id,
                 event_sequence, event_id, event_sha256, binding_sha256, record_json
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                &binding.checkpoint_sha256,
                binding.checkpoint_id.as_str(),
                binding.session_id.as_str(),
                binding.task_id.as_str(),
                binding.run_id.as_str(),
                sql_u64(binding.event_cursor.sequence)?,
                binding.event_cursor.event_id.as_str(),
                &binding.event_cursor.event_sha256,
                &binding.binding_sha256,
                record_json,
            ],
        )
        .map_err(|_| RuntimeArtifactStoreError::Storage)?;
    for (ordinal, reference) in binding.artifacts.iter().enumerate() {
        transaction
            .execute(
                "INSERT INTO runtime_resume_artifacts(
                     checkpoint_sha256, ordinal, artifact_id, manifest_sha256,
                     payload_sha256, byte_size, media_type
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    &binding.checkpoint_sha256,
                    i64::try_from(ordinal).map_err(|_| RuntimeArtifactStoreError::Storage)?,
                    reference.artifact_id.as_str(),
                    &reference.manifest_sha256,
                    &reference.payload_sha256,
                    sql_u64(reference.byte_size)?,
                    &reference.media_type,
                ],
            )
            .map_err(|_| RuntimeArtifactStoreError::Storage)?;
    }
    Ok(())
}

/// Loads the current checkpoint binding and requires every referenced artifact to remain usable.
pub(crate) fn current_runtime_resume_binding(
    store: &OperationalStore,
) -> Result<Option<RuntimeResumeBinding>, RuntimeArtifactStoreError> {
    let checkpoint_sha256: String = store
        .connection
        .query_row(
            "SELECT session_checkpoint_sha256 FROM store_metadata WHERE singleton = 1",
            [],
            |row| row.get(0),
        )
        .map_err(|_| RuntimeArtifactStoreError::Storage)?;
    if checkpoint_sha256 == ZERO_SHA256 {
        return Ok(None);
    }
    let exists = store
        .connection
        .query_row(
            "SELECT 1 FROM runtime_resume_bindings WHERE checkpoint_sha256 = ?1",
            [&checkpoint_sha256],
            |_| Ok(()),
        )
        .optional()
        .map_err(|_| RuntimeArtifactStoreError::Storage)?;
    if exists.is_none() {
        return Ok(None);
    }
    let binding = load_runtime_resume_binding(store, &checkpoint_sha256)?;
    for reference in &binding.artifacts {
        let state = runtime_artifact_state(store, reference)?;
        if state.lifecycle != RuntimeArtifactLifecycleState::Active
            || state.integrity != RuntimeArtifactIntegrityState::Verified
        {
            return Err(RuntimeArtifactStoreError::NotAuthorized);
        }
    }
    Ok(Some(binding))
}

fn verify_resume_artifact_reference(
    transaction: &Transaction<'_>,
    checkpoint: &SessionCheckpoint,
    binding: &RuntimeResumeBinding,
    reference: &RuntimeArtifactRef,
) -> Result<(), RuntimeArtifactStoreError> {
    let retained = transaction
        .query_row(
            "SELECT a.manifest_sha256, a.payload_sha256, a.byte_size, a.media_type,
                    a.session_id, a.task_id, a.producer_run_id, a.policy_sha256,
                    s.lifecycle_state, s.integrity_state
             FROM runtime_artifacts a
             JOIN runtime_artifact_states s ON s.artifact_id = a.artifact_id
             WHERE a.artifact_id = ?1",
            [reference.artifact_id.as_str()],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, String>(6)?,
                    row.get::<_, String>(7)?,
                    row.get::<_, String>(8)?,
                    row.get::<_, String>(9)?,
                ))
            },
        )
        .optional()
        .map_err(|_| RuntimeArtifactStoreError::Storage)?
        .ok_or(RuntimeArtifactStoreError::NotFound)?;
    if retained.0 != reference.manifest_sha256
        || retained.1 != reference.payload_sha256
        || u64::try_from(retained.2).ok() != Some(reference.byte_size)
        || retained.3 != reference.media_type
        || retained.4 != binding.session_id.as_str()
        || retained.5 != binding.task_id.as_str()
        || retained.6 != binding.run_id.as_str()
        || retained.7 != checkpoint.policy_sha256
        || retained.8 != "active"
        || retained.9 != "verified"
    {
        return Err(RuntimeArtifactStoreError::NotAuthorized);
    }
    Ok(())
}

/// Returns a privacy-safe state projection after exact reference reconciliation.
pub(crate) fn runtime_artifact_state(
    store: &OperationalStore,
    reference: &RuntimeArtifactRef,
) -> Result<RuntimeArtifactState, RuntimeArtifactStoreError> {
    let manifest = load_artifact_manifest(store, &reference.artifact_id)?;
    verify_runtime_artifact_ref(reference, &manifest)?;
    load_artifact_state(store, reference)
}

/// Returns the complete privacy-safe operator projection for one exact reference.
pub(crate) fn runtime_artifact_operator_view(
    store: &OperationalStore,
    reference: &RuntimeArtifactRef,
) -> Result<RuntimeArtifactOperatorView, RuntimeArtifactStoreError> {
    let manifest = load_artifact_manifest(store, &reference.artifact_id)?;
    verify_runtime_artifact_ref(reference, &manifest)?;
    let state = load_artifact_state(store, reference)?;
    let checkpoint_reference_count = store
        .connection
        .query_row(
            "SELECT COUNT(*)
             FROM runtime_resume_artifacts r
             JOIN store_metadata m ON m.session_checkpoint_sha256 = r.checkpoint_sha256
             WHERE m.singleton = 1 AND r.artifact_id = ?1 AND r.manifest_sha256 = ?2",
            params![reference.artifact_id.as_str(), &reference.manifest_sha256],
            |row| row.get::<_, i64>(0),
        )
        .map_err(|_| RuntimeArtifactStoreError::Storage)
        .and_then(|count| u32::try_from(count).map_err(|_| RuntimeArtifactStoreError::Integrity))?;
    let shared_active_reference_count = store
        .connection
        .query_row(
            "SELECT active_reference_count FROM runtime_payloads WHERE payload_sha256 = ?1",
            [&reference.payload_sha256],
            |row| row.get::<_, i64>(0),
        )
        .map_err(|_| RuntimeArtifactStoreError::Storage)
        .and_then(|count| u32::try_from(count).map_err(|_| RuntimeArtifactStoreError::Integrity))?;
    let cleanup = match state.lifecycle {
        RuntimeArtifactLifecycleState::Active => RuntimeArtifactCleanupState::Retained,
        RuntimeArtifactLifecycleState::Released => RuntimeArtifactCleanupState::Eligible,
        RuntimeArtifactLifecycleState::Quarantined => RuntimeArtifactCleanupState::Blocked,
        RuntimeArtifactLifecycleState::Deleted => RuntimeArtifactCleanupState::Completed,
    };
    Ok(RuntimeArtifactOperatorView {
        schema_version: CONTRACT_SCHEMA_VERSION,
        reference: reference.clone(),
        kind: manifest.kind,
        sensitivity: manifest.sensitivity,
        retention: manifest.retention,
        session_id: manifest.session_id,
        task_id: manifest.task_id,
        producer_run_id: manifest.producer_run_id,
        producer_turn_id: manifest.producer_turn_id,
        producer_operation_id: manifest.producer_operation_id,
        receipt_id: manifest.receipt_id,
        policy_id: manifest.policy_id,
        created_at_epoch_ms: manifest.created_at_epoch_ms,
        lifecycle: state.lifecycle,
        integrity: state.integrity,
        lifecycle_revision: state.revision,
        reason_code: state.reason_code,
        updated_at_epoch_ms: state.updated_at_epoch_ms,
        checkpoint_reference_count,
        shared_active_reference_count,
        cleanup,
    })
}

/// Opens complete verified bytes only for the exact owning session, task, and policy revision.
pub(crate) fn read_runtime_artifact<S: RuntimeArtifactPayloadStore>(
    store: &OperationalStore,
    payloads: &S,
    request: &RuntimeArtifactReadRequest,
) -> Result<Vec<u8>, RuntimeArtifactStoreError> {
    let manifest = authorize_runtime_artifact_read(
        store,
        &request.session_id,
        &request.task_id,
        &request.policy_sha256,
        &request.reference,
        request.now_epoch_ms,
    )?;
    if request.maximum_bytes == 0 || request.maximum_bytes < manifest.byte_size {
        return Err(RuntimeArtifactStoreError::Payload(
            RuntimeArtifactPayloadError::ResourceLimit,
        ));
    }
    let expected = payload_observation(&manifest);
    payloads.verify(&expected)?;
    let bytes = payloads.read_complete(&expected, request.maximum_bytes)?;
    if bytes.len() as u64 != expected.byte_size || sha256(&bytes) != expected.payload_sha256 {
        return Err(RuntimeArtifactStoreError::Payload(
            RuntimeArtifactPayloadError::Corrupt,
        ));
    }
    Ok(bytes)
}

/// Opens one verified bounded artifact page under the exact owner and policy revision.
pub(crate) fn read_runtime_artifact_page<S: RuntimeArtifactPayloadStore>(
    store: &OperationalStore,
    payloads: &S,
    request: &RuntimeArtifactPageRequest,
) -> Result<RuntimeArtifactPage, RuntimeArtifactStoreError> {
    let manifest = authorize_runtime_artifact_read(
        store,
        &request.session_id,
        &request.task_id,
        &request.policy_sha256,
        &request.reference,
        request.now_epoch_ms,
    )?;
    let maximum_bytes = usize::try_from(request.maximum_bytes).map_err(|_| {
        RuntimeArtifactStoreError::Payload(RuntimeArtifactPayloadError::ResourceLimit)
    })?;
    if maximum_bytes == 0
        || maximum_bytes > MAX_RUNTIME_ARTIFACT_PREVIEW_BYTES
        || request.offset >= manifest.byte_size
    {
        return Err(RuntimeArtifactStoreError::Payload(
            RuntimeArtifactPayloadError::ResourceLimit,
        ));
    }
    let expected = payload_observation(&manifest);
    payloads.verify(&expected)?;
    let bytes = payloads.read_range(&expected, request.offset, u64::from(request.maximum_bytes))?;
    let byte_count = u64::try_from(bytes.len())
        .map_err(|_| RuntimeArtifactStoreError::Payload(RuntimeArtifactPayloadError::Corrupt))?;
    let expected_byte_count =
        u64::from(request.maximum_bytes).min(manifest.byte_size - request.offset);
    let end = request
        .offset
        .checked_add(byte_count)
        .ok_or(RuntimeArtifactStoreError::Payload(
            RuntimeArtifactPayloadError::Corrupt,
        ))?;
    if bytes.is_empty() || byte_count != expected_byte_count || end > manifest.byte_size {
        return Err(RuntimeArtifactStoreError::Payload(
            RuntimeArtifactPayloadError::Corrupt,
        ));
    }
    let complete = end == manifest.byte_size;
    Ok(RuntimeArtifactPage {
        reference: request.reference.clone(),
        offset: request.offset,
        page_sha256: sha256(&bytes),
        bytes,
        next_offset: (!complete).then_some(end),
        complete,
    })
}

fn authorize_runtime_artifact_read(
    store: &OperationalStore,
    session_id: &SessionId,
    task_id: &TaskId,
    policy_sha256: &str,
    reference: &RuntimeArtifactRef,
    now_epoch_ms: u64,
) -> Result<RuntimeArtifactManifest, RuntimeArtifactStoreError> {
    let manifest = load_artifact_manifest(store, &reference.artifact_id)?;
    verify_runtime_artifact_ref(reference, &manifest)?;
    let state = load_artifact_state(store, reference)?;
    if &manifest.session_id != session_id
        || &manifest.task_id != task_id
        || manifest.policy_sha256 != policy_sha256
        || state.lifecycle != RuntimeArtifactLifecycleState::Active
        || state.integrity != RuntimeArtifactIntegrityState::Verified
        || manifest
            .retention
            .expires_at_epoch_ms
            .is_some_and(|expires| expires <= now_epoch_ms)
    {
        return Err(RuntimeArtifactStoreError::NotAuthorized);
    }
    Ok(manifest)
}

/// Releases one active reference under its exact owner and current policy revision.
pub(crate) fn release_runtime_artifact(
    store: &mut OperationalStore,
    session_id: &SessionId,
    task_id: &TaskId,
    policy_sha256: &str,
    reference: &RuntimeArtifactRef,
    occurred_at_epoch_ms: u64,
) -> Result<RuntimeArtifactState, RuntimeArtifactStoreError> {
    let manifest = load_artifact_manifest(store, &reference.artifact_id)?;
    verify_runtime_artifact_ref(reference, &manifest)?;
    if &manifest.session_id != session_id
        || &manifest.task_id != task_id
        || manifest.policy_sha256 != policy_sha256
    {
        return Err(RuntimeArtifactStoreError::NotAuthorized);
    }
    let current_checkpoint_references: i64 = store
        .connection
        .query_row(
            "SELECT COUNT(*)
             FROM runtime_resume_artifacts r
             JOIN store_metadata m ON m.session_checkpoint_sha256 = r.checkpoint_sha256
             WHERE m.singleton = 1 AND r.artifact_id = ?1 AND r.manifest_sha256 = ?2",
            params![reference.artifact_id.as_str(), &reference.manifest_sha256],
            |row| row.get(0),
        )
        .map_err(|_| RuntimeArtifactStoreError::Storage)?;
    if current_checkpoint_references != 0 {
        return Err(RuntimeArtifactStoreError::NotAuthorized);
    }
    transition_artifact(
        store,
        &manifest,
        RuntimeArtifactLifecycleState::Released,
        RuntimeArtifactIntegrityState::Verified,
        "artifact.reference_released",
        occurred_at_epoch_ms,
    )?;
    load_artifact_state(store, reference)
}

/// Reconciles staging, retained objects, metadata state, expiry, and unreferenced payloads.
pub(crate) fn reconcile_runtime_artifacts<S: RuntimeArtifactPayloadStore>(
    store: &mut OperationalStore,
    payloads: &mut S,
    now_epoch_ms: u64,
) -> Result<RuntimeArtifactReconciliation, RuntimeArtifactStoreError> {
    verify_all(store)?;
    let mut report = RuntimeArtifactReconciliation {
        cleaned_staging: payloads.cleanup_staging()?,
        ..RuntimeArtifactReconciliation::default()
    };
    let now = sql_u64(now_epoch_ms)?;
    let expired = store
        .connection
        .prepare(
            "SELECT a.artifact_id
             FROM runtime_artifacts a
             JOIN runtime_artifact_states s ON s.artifact_id = a.artifact_id
             WHERE s.lifecycle_state = 'active'
               AND a.retention_kind = 'until_expiration'
               AND a.retention_expires_at_epoch_ms <= ?1
             ORDER BY a.artifact_id",
        )
        .and_then(|mut statement| {
            statement
                .query_map([now], |row| row.get(0))?
                .collect::<Result<Vec<String>, _>>()
        })
        .map_err(|_| RuntimeArtifactStoreError::Storage)?;
    for artifact_id in expired {
        let artifact_id = RuntimeArtifactId::from_raw(artifact_id);
        let manifest = load_artifact_manifest(store, &artifact_id)?;
        transition_artifact(
            store,
            &manifest,
            RuntimeArtifactLifecycleState::Released,
            RuntimeArtifactIntegrityState::Verified,
            "artifact.retention_expired",
            now_epoch_ms,
        )?;
    }

    let inventory = payloads.inventory()?;
    validate_payload_inventory(&inventory)?;
    let inventory_by_digest = inventory
        .iter()
        .map(|entry| (entry.payload_sha256.as_str(), entry))
        .collect::<BTreeMap<_, _>>();
    let retained_payloads = load_payload_rows(store)?;
    for retained in &retained_payloads {
        let expected = RuntimeArtifactPayloadObservation {
            payload_sha256: retained.payload_sha256.clone(),
            byte_size: retained.byte_size,
        };
        match retained.lifecycle_state.as_str() {
            "active" if retained.active_reference_count > 0 => match payloads.verify(&expected) {
                Ok(()) => report.verified_payloads += 1,
                Err(RuntimeArtifactPayloadError::Missing) => {
                    quarantine_payload_metadata(
                        store,
                        &retained.payload_sha256,
                        RuntimeArtifactIntegrityState::Missing,
                        "artifact.payload_missing",
                        now_epoch_ms,
                    )?;
                    report.quarantined_payloads += 1;
                }
                Err(RuntimeArtifactPayloadError::Corrupt) => {
                    payloads.quarantine(&expected)?;
                    quarantine_payload_metadata(
                        store,
                        &retained.payload_sha256,
                        RuntimeArtifactIntegrityState::Corrupt,
                        "artifact.payload_corrupt",
                        now_epoch_ms,
                    )?;
                    report.quarantined_payloads += 1;
                }
                Err(error) => return Err(error.into()),
            },
            "active" => {
                delete_payload_metadata(store, &retained.payload_sha256, now_epoch_ms)?;
                match inventory_by_digest.get(retained.payload_sha256.as_str()) {
                    Some(entry)
                        if entry.integrity == RuntimeArtifactPayloadInventoryIntegrity::Corrupt =>
                    {
                        payloads.quarantine(&expected)?;
                        report.quarantined_payloads += 1;
                    }
                    _ => match payloads.delete(&expected) {
                        Ok(()) | Err(RuntimeArtifactPayloadError::Missing) => {
                            report.deleted_orphans += 1;
                        }
                        Err(error) => return Err(error.into()),
                    },
                }
            }
            "quarantined" => {
                if inventory_by_digest.contains_key(retained.payload_sha256.as_str()) {
                    payloads.quarantine(&expected)?;
                }
            }
            "deleted" => {
                if let Some(entry) = inventory_by_digest.get(retained.payload_sha256.as_str()) {
                    if entry.integrity == RuntimeArtifactPayloadInventoryIntegrity::Corrupt {
                        payloads.quarantine(&expected)?;
                        report.quarantined_payloads += 1;
                    } else {
                        payloads.delete(&expected)?;
                        report.deleted_orphans += 1;
                    }
                }
            }
            _ => return Err(RuntimeArtifactStoreError::Integrity),
        }
    }
    let retained_identities = retained_payloads
        .iter()
        .map(|row| row.payload_sha256.as_str())
        .collect::<BTreeSet<_>>();
    for orphan in inventory {
        if !retained_identities.contains(orphan.payload_sha256.as_str()) {
            let expected = RuntimeArtifactPayloadObservation {
                payload_sha256: orphan.payload_sha256,
                byte_size: orphan.byte_size.max(1),
            };
            if orphan.integrity == RuntimeArtifactPayloadInventoryIntegrity::Corrupt {
                payloads.quarantine(&expected)?;
                report.quarantined_payloads += 1;
            } else {
                payloads.delete(&expected)?;
                report.deleted_orphans += 1;
            }
        }
    }
    verify_all(store)?;
    Ok(report)
}

/// Returns content-free canonical table counts without exposing payload or record content.
pub(crate) fn runtime_artifact_row_counts(
    store: &OperationalStore,
) -> Result<RuntimeArtifactRowCounts, RuntimeArtifactStoreError> {
    fn count(store: &OperationalStore, statement: &str) -> Result<u64, RuntimeArtifactStoreError> {
        let value = store
            .connection
            .query_row(statement, [], |row| row.get::<_, i64>(0))
            .map_err(|_| RuntimeArtifactStoreError::Storage)?;
        u64::try_from(value).map_err(|_| RuntimeArtifactStoreError::Integrity)
    }

    Ok(RuntimeArtifactRowCounts {
        payloads: count(store, "SELECT COUNT(*) FROM runtime_payloads")?,
        artifacts: count(store, "SELECT COUNT(*) FROM runtime_artifacts")?,
        lifecycle_events: count(store, "SELECT COUNT(*) FROM runtime_artifact_events")?,
        resume_bindings: count(store, "SELECT COUNT(*) FROM runtime_resume_bindings")?,
        resume_artifacts: count(store, "SELECT COUNT(*) FROM runtime_resume_artifacts")?,
    })
}

/// Verifies every immutable manifest, lifecycle chain, payload count, and resume projection.
pub(crate) fn verify_all(store: &OperationalStore) -> Result<(), RuntimeArtifactStoreError> {
    let artifact_ids = store
        .connection
        .prepare("SELECT artifact_id FROM runtime_artifacts ORDER BY artifact_id")
        .and_then(|mut statement| {
            statement
                .query_map([], |row| row.get::<_, String>(0))?
                .collect::<Result<Vec<_>, _>>()
        })
        .map_err(|_| RuntimeArtifactStoreError::Integrity)?;
    let mut active_counts = BTreeMap::<String, u64>::new();
    for artifact_id in &artifact_ids {
        let artifact_id = RuntimeArtifactId::from_raw(artifact_id.clone());
        let manifest = load_artifact_manifest(store, &artifact_id)
            .map_err(|_| RuntimeArtifactStoreError::Integrity)?;
        let reference =
            runtime_artifact_ref(&manifest).map_err(|_| RuntimeArtifactStoreError::Integrity)?;
        let state = load_artifact_state(store, &reference)
            .map_err(|_| RuntimeArtifactStoreError::Integrity)?;
        if !valid_lifecycle_pair(state.lifecycle, state.integrity) {
            return Err(RuntimeArtifactStoreError::Integrity);
        }
        if state.lifecycle == RuntimeArtifactLifecycleState::Active {
            *active_counts
                .entry(manifest.payload_sha256.clone())
                .or_default() += 1;
        }
    }
    verify_artifact_event_chains(store, &artifact_ids)?;
    let payload_rows = load_payload_rows(store)?;
    let payload_ids = payload_rows
        .iter()
        .map(|row| row.payload_sha256.as_str())
        .collect::<BTreeSet<_>>();
    for row in &payload_rows {
        let expected = active_counts.get(&row.payload_sha256).copied().unwrap_or(0);
        if row.active_reference_count != expected
            || !matches!(
                row.lifecycle_state.as_str(),
                "active" | "quarantined" | "deleted"
            )
            || row.lifecycle_state != "active" && expected != 0
        {
            return Err(RuntimeArtifactStoreError::Integrity);
        }
    }
    if active_counts
        .keys()
        .any(|payload_sha256| !payload_ids.contains(payload_sha256.as_str()))
    {
        return Err(RuntimeArtifactStoreError::Integrity);
    }
    verify_resume_bindings(store)?;
    Ok(())
}

fn quarantine_payload_metadata(
    store: &mut OperationalStore,
    payload_sha256: &str,
    integrity: RuntimeArtifactIntegrityState,
    reason_code: &str,
    occurred_at_epoch_ms: u64,
) -> Result<(), RuntimeArtifactStoreError> {
    let manifests = load_payload_manifests(store, payload_sha256)?;
    let transaction = store
        .connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|_| RuntimeArtifactStoreError::Storage)?;
    for manifest in &manifests {
        let current: String = transaction
            .query_row(
                "SELECT lifecycle_state FROM runtime_artifact_states WHERE artifact_id = ?1",
                [manifest.artifact_id.as_str()],
                |row| row.get(0),
            )
            .map_err(|_| RuntimeArtifactStoreError::Integrity)?;
        if current == "active" {
            transition_artifact_in_transaction(
                &transaction,
                manifest,
                RuntimeArtifactLifecycleState::Quarantined,
                integrity,
                reason_code,
                occurred_at_epoch_ms,
            )?;
        }
    }
    let changed = transaction
        .execute(
            "UPDATE runtime_payloads
             SET lifecycle_state = 'quarantined', active_reference_count = 0,
                 updated_at_epoch_ms = MAX(updated_at_epoch_ms, ?1)
             WHERE payload_sha256 = ?2 AND lifecycle_state = 'active'",
            params![sql_u64(occurred_at_epoch_ms)?, payload_sha256],
        )
        .map_err(|_| RuntimeArtifactStoreError::Storage)?;
    if changed != 1 {
        return Err(RuntimeArtifactStoreError::Integrity);
    }
    transaction
        .commit()
        .map_err(|_| RuntimeArtifactStoreError::Storage)
}

fn verify_manifest_producer(
    connection: &Connection,
    manifest: &RuntimeArtifactManifest,
) -> Result<(), RuntimeArtifactStoreError> {
    let owner = connection
        .query_row(
            "SELECT session_id, task_id, policy_id FROM runtime_runs WHERE run_id = ?1",
            [manifest.producer_run_id.as_str()],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            },
        )
        .optional()
        .map_err(|_| RuntimeArtifactStoreError::Storage)?;
    match owner {
        Some((session_id, task_id, policy_id))
            if session_id == manifest.session_id.as_str()
                && task_id == manifest.task_id.as_str()
                && policy_id == manifest.policy_id.as_str() =>
        {
            Ok(())
        }
        _ => Err(RuntimeArtifactStoreError::NotAuthorized),
    }
}

fn delete_payload_metadata(
    store: &mut OperationalStore,
    payload_sha256: &str,
    occurred_at_epoch_ms: u64,
) -> Result<(), RuntimeArtifactStoreError> {
    let manifests = load_payload_manifests(store, payload_sha256)?;
    let transaction = store
        .connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|_| RuntimeArtifactStoreError::Storage)?;
    let active_count: i64 = transaction
        .query_row(
            "SELECT active_reference_count FROM runtime_payloads WHERE payload_sha256 = ?1",
            [payload_sha256],
            |row| row.get(0),
        )
        .map_err(|_| RuntimeArtifactStoreError::Integrity)?;
    if active_count != 0 {
        return Err(RuntimeArtifactStoreError::NotAuthorized);
    }
    for manifest in &manifests {
        let current: String = transaction
            .query_row(
                "SELECT lifecycle_state FROM runtime_artifact_states WHERE artifact_id = ?1",
                [manifest.artifact_id.as_str()],
                |row| row.get(0),
            )
            .map_err(|_| RuntimeArtifactStoreError::Integrity)?;
        if matches!(current.as_str(), "released" | "quarantined") {
            transition_artifact_in_transaction(
                &transaction,
                manifest,
                RuntimeArtifactLifecycleState::Deleted,
                RuntimeArtifactIntegrityState::Deleted,
                "artifact.payload_deleted",
                occurred_at_epoch_ms,
            )?;
        } else if current != "deleted" {
            return Err(RuntimeArtifactStoreError::Integrity);
        }
    }
    transaction
        .execute(
            "UPDATE runtime_payloads
             SET lifecycle_state = 'deleted', updated_at_epoch_ms = MAX(updated_at_epoch_ms, ?1)
             WHERE payload_sha256 = ?2",
            params![sql_u64(occurred_at_epoch_ms)?, payload_sha256],
        )
        .map_err(|_| RuntimeArtifactStoreError::Storage)?;
    transaction
        .commit()
        .map_err(|_| RuntimeArtifactStoreError::Storage)
}

fn load_payload_manifests(
    store: &OperationalStore,
    payload_sha256: &str,
) -> Result<Vec<RuntimeArtifactManifest>, RuntimeArtifactStoreError> {
    let artifact_ids = store
        .connection
        .prepare(
            "SELECT artifact_id FROM runtime_artifacts
             WHERE payload_sha256 = ?1 ORDER BY artifact_id",
        )
        .and_then(|mut statement| {
            statement
                .query_map([payload_sha256], |row| row.get::<_, String>(0))?
                .collect::<Result<Vec<_>, _>>()
        })
        .map_err(|_| RuntimeArtifactStoreError::Storage)?;
    artifact_ids
        .into_iter()
        .map(|artifact_id| load_artifact_manifest(store, &RuntimeArtifactId::from_raw(artifact_id)))
        .collect()
}

struct RetainedPayloadRow {
    payload_sha256: String,
    byte_size: u64,
    lifecycle_state: String,
    active_reference_count: u64,
}

fn load_payload_rows(
    store: &OperationalStore,
) -> Result<Vec<RetainedPayloadRow>, RuntimeArtifactStoreError> {
    store
        .connection
        .prepare(
            "SELECT payload_sha256, byte_size, lifecycle_state, active_reference_count
             FROM runtime_payloads ORDER BY payload_sha256",
        )
        .and_then(|mut statement| {
            statement
                .query_map([], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, i64>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, i64>(3)?,
                    ))
                })?
                .collect::<Result<Vec<_>, _>>()
        })
        .map_err(|_| RuntimeArtifactStoreError::Integrity)?
        .into_iter()
        .map(|row| {
            Ok(RetainedPayloadRow {
                payload_sha256: row.0,
                byte_size: u64::try_from(row.1)
                    .map_err(|_| RuntimeArtifactStoreError::Integrity)?,
                lifecycle_state: row.2,
                active_reference_count: u64::try_from(row.3)
                    .map_err(|_| RuntimeArtifactStoreError::Integrity)?,
            })
        })
        .collect()
}

fn validate_payload_inventory(
    inventory: &[RuntimeArtifactPayloadInventoryEntry],
) -> Result<(), RuntimeArtifactStoreError> {
    let mut prior: Option<&str> = None;
    for entry in inventory {
        if !valid_sha256(&entry.payload_sha256)
            || match entry.integrity {
                RuntimeArtifactPayloadInventoryIntegrity::Verified => {
                    entry.byte_size == 0 || entry.byte_size > MAX_RUNTIME_ARTIFACT_BYTES
                }
                RuntimeArtifactPayloadInventoryIntegrity::Corrupt => entry.byte_size != 0,
            }
            || prior.is_some_and(|value| value >= entry.payload_sha256.as_str())
        {
            return Err(RuntimeArtifactStoreError::Payload(
                RuntimeArtifactPayloadError::Invalid,
            ));
        }
        prior = Some(entry.payload_sha256.as_str());
    }
    Ok(())
}

fn valid_lifecycle_pair(
    lifecycle: RuntimeArtifactLifecycleState,
    integrity: RuntimeArtifactIntegrityState,
) -> bool {
    matches!(
        (lifecycle, integrity),
        (
            RuntimeArtifactLifecycleState::Active | RuntimeArtifactLifecycleState::Released,
            RuntimeArtifactIntegrityState::Verified
        ) | (
            RuntimeArtifactLifecycleState::Quarantined,
            RuntimeArtifactIntegrityState::Quarantined
                | RuntimeArtifactIntegrityState::Missing
                | RuntimeArtifactIntegrityState::Corrupt
        ) | (
            RuntimeArtifactLifecycleState::Deleted,
            RuntimeArtifactIntegrityState::Deleted
        )
    )
}

fn verify_artifact_event_chains(
    store: &OperationalStore,
    artifact_ids: &[String],
) -> Result<(), RuntimeArtifactStoreError> {
    let events = store
        .connection
        .prepare(
            "SELECT artifact_id, revision, lifecycle_state, integrity_state, reason_code,
                    occurred_at_epoch_ms, previous_event_sha256, state_sha256, event_sha256
             FROM runtime_artifact_events ORDER BY artifact_id, revision",
        )
        .and_then(|mut statement| {
            statement
                .query_map([], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, i64>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, String>(4)?,
                        row.get::<_, i64>(5)?,
                        row.get::<_, String>(6)?,
                        row.get::<_, String>(7)?,
                        row.get::<_, String>(8)?,
                    ))
                })?
                .collect::<Result<Vec<_>, _>>()
        })
        .map_err(|_| RuntimeArtifactStoreError::Integrity)?;
    let mut heads = BTreeMap::<String, (u64, String, String)>::new();
    for event in events {
        let artifact_id = RuntimeArtifactId::from_raw(event.0.clone());
        let revision = u64::try_from(event.1).map_err(|_| RuntimeArtifactStoreError::Integrity)?;
        let lifecycle = parse_lifecycle(&event.2)?;
        let integrity = parse_integrity(&event.3)?;
        let occurred_at =
            u64::try_from(event.5).map_err(|_| RuntimeArtifactStoreError::Integrity)?;
        let expected_revision = heads.get(&event.0).map_or(1, |head| head.0 + 1);
        let expected_previous = heads
            .get(&event.0)
            .map_or(ZERO_SHA256, |head| head.1.as_str());
        let state_sha256 = artifact_state_digest(
            &artifact_id,
            revision,
            lifecycle,
            integrity,
            &event.4,
            occurred_at,
        );
        let event_sha256 = artifact_event_digest(
            &artifact_id,
            revision,
            lifecycle,
            integrity,
            &event.4,
            occurred_at,
            &event.6,
            &event.7,
        );
        if revision != expected_revision
            || event.6 != expected_previous
            || event.7 != state_sha256
            || event.8 != event_sha256
            || !valid_reason_code(&event.4)
            || !valid_lifecycle_pair(lifecycle, integrity)
        {
            return Err(RuntimeArtifactStoreError::Integrity);
        }
        heads.insert(event.0, (revision, event.8, event.7));
    }
    if heads.len() != artifact_ids.len() {
        return Err(RuntimeArtifactStoreError::Integrity);
    }
    for artifact_id in artifact_ids {
        let retained = store
            .connection
            .query_row(
                "SELECT revision, state_sha256 FROM runtime_artifact_states
                 WHERE artifact_id = ?1",
                [artifact_id],
                |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?)),
            )
            .map_err(|_| RuntimeArtifactStoreError::Integrity)?;
        let head = heads
            .get(artifact_id)
            .ok_or(RuntimeArtifactStoreError::Integrity)?;
        if u64::try_from(retained.0).ok() != Some(head.0) || retained.1 != head.2 {
            return Err(RuntimeArtifactStoreError::Integrity);
        }
    }
    Ok(())
}

fn verify_resume_bindings(store: &OperationalStore) -> Result<(), RuntimeArtifactStoreError> {
    let checkpoint_sha256s = store
        .connection
        .prepare("SELECT checkpoint_sha256 FROM runtime_resume_bindings ORDER BY checkpoint_sha256")
        .and_then(|mut statement| {
            statement
                .query_map([], |row| row.get::<_, String>(0))?
                .collect::<Result<Vec<_>, _>>()
        })
        .map_err(|_| RuntimeArtifactStoreError::Integrity)?;
    let mut expected_references = 0_usize;
    for checkpoint_sha256 in &checkpoint_sha256s {
        let binding = load_runtime_resume_binding(store, checkpoint_sha256)
            .map_err(|_| RuntimeArtifactStoreError::Integrity)?;
        expected_references = expected_references
            .checked_add(binding.artifacts.len())
            .ok_or(RuntimeArtifactStoreError::Integrity)?;
    }
    let retained_references: i64 = store
        .connection
        .query_row("SELECT COUNT(*) FROM runtime_resume_artifacts", [], |row| {
            row.get(0)
        })
        .map_err(|_| RuntimeArtifactStoreError::Integrity)?;
    if usize::try_from(retained_references).ok() == Some(expected_references) {
        Ok(())
    } else {
        Err(RuntimeArtifactStoreError::Integrity)
    }
}

fn load_runtime_resume_binding(
    store: &OperationalStore,
    checkpoint_sha256: &str,
) -> Result<RuntimeResumeBinding, RuntimeArtifactStoreError> {
    let retained = store
        .connection
        .query_row(
            "SELECT checkpoint_id, session_id, task_id, run_id, event_sequence,
                    event_id, event_sha256, binding_sha256, record_json
             FROM runtime_resume_bindings WHERE checkpoint_sha256 = ?1",
            [checkpoint_sha256],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, i64>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, String>(6)?,
                    row.get::<_, String>(7)?,
                    row.get::<_, Vec<u8>>(8)?,
                ))
            },
        )
        .optional()
        .map_err(|_| RuntimeArtifactStoreError::Integrity)?
        .ok_or(RuntimeArtifactStoreError::NotFound)?;
    let binding = from_json::<RuntimeResumeBinding>(&retained.8)
        .map_err(|_| RuntimeArtifactStoreError::Integrity)?;
    verify_runtime_resume_binding(&binding).map_err(|_| RuntimeArtifactStoreError::Integrity)?;
    if binding.checkpoint_sha256 != checkpoint_sha256
        || binding.checkpoint_id.as_str() != retained.0
        || binding.session_id.as_str() != retained.1
        || binding.task_id.as_str() != retained.2
        || binding.run_id.as_str() != retained.3
        || u64::try_from(retained.4).ok() != Some(binding.event_cursor.sequence)
        || binding.event_cursor.event_id.as_str() != retained.5
        || binding.event_cursor.event_sha256 != retained.6
        || binding.binding_sha256 != retained.7
    {
        return Err(RuntimeArtifactStoreError::Integrity);
    }
    let checkpoint_bytes = store
        .connection
        .query_row(
            "SELECT record_json FROM session_checkpoints WHERE checkpoint_sha256 = ?1",
            [checkpoint_sha256],
            |row| row.get::<_, Vec<u8>>(0),
        )
        .map_err(|_| RuntimeArtifactStoreError::Integrity)?;
    let checkpoint = from_json::<SessionCheckpoint>(&checkpoint_bytes)
        .map_err(|_| RuntimeArtifactStoreError::Integrity)?;
    if checkpoint.checkpoint_id != binding.checkpoint_id
        || checkpoint.checkpoint_sha256 != binding.checkpoint_sha256
        || checkpoint.session_id != binding.session_id
        || checkpoint.task_id != binding.task_id
    {
        return Err(RuntimeArtifactStoreError::Integrity);
    }
    let event = store
        .connection
        .query_row(
            "SELECT event_id, event_sha256 FROM runtime_events
             WHERE run_id = ?1 AND sequence = ?2",
            params![
                binding.run_id.as_str(),
                sql_u64(binding.event_cursor.sequence)?
            ],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
        )
        .map_err(|_| RuntimeArtifactStoreError::Integrity)?;
    if event.0 != binding.event_cursor.event_id.as_str()
        || event.1 != binding.event_cursor.event_sha256
    {
        return Err(RuntimeArtifactStoreError::Integrity);
    }
    let rows = store
        .connection
        .prepare(
            "SELECT artifact_id, manifest_sha256, payload_sha256, byte_size, media_type
             FROM runtime_resume_artifacts WHERE checkpoint_sha256 = ?1 ORDER BY ordinal",
        )
        .and_then(|mut statement| {
            statement
                .query_map([checkpoint_sha256], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, i64>(3)?,
                        row.get::<_, String>(4)?,
                    ))
                })?
                .collect::<Result<Vec<_>, _>>()
        })
        .map_err(|_| RuntimeArtifactStoreError::Integrity)?;
    if rows.len() != binding.artifacts.len() {
        return Err(RuntimeArtifactStoreError::Integrity);
    }
    for (row, reference) in rows.iter().zip(&binding.artifacts) {
        if row.0 != reference.artifact_id.as_str()
            || row.1 != reference.manifest_sha256
            || row.2 != reference.payload_sha256
            || u64::try_from(row.3).ok() != Some(reference.byte_size)
            || row.4 != reference.media_type
        {
            return Err(RuntimeArtifactStoreError::Integrity);
        }
        let manifest = load_artifact_manifest(store, &reference.artifact_id)
            .map_err(|_| RuntimeArtifactStoreError::Integrity)?;
        verify_runtime_artifact_ref(reference, &manifest)
            .map_err(|_| RuntimeArtifactStoreError::Integrity)?;
        if manifest.session_id != binding.session_id
            || manifest.task_id != binding.task_id
            || manifest.producer_run_id != binding.run_id
        {
            return Err(RuntimeArtifactStoreError::Integrity);
        }
    }
    Ok(binding)
}

fn persist_artifact_manifest(
    store: &mut OperationalStore,
    manifest: &RuntimeArtifactManifest,
) -> Result<(), RuntimeArtifactStoreError> {
    let record_json = to_canonical_json(manifest)
        .map_err(|_| RuntimeArtifactStoreError::Contract(RuntimeArtifactError::Serialization))?;
    let byte_size = sql_u64(manifest.byte_size)?;
    let created_at = sql_u64(manifest.created_at_epoch_ms)?;
    let expires_at = manifest
        .retention
        .expires_at_epoch_ms
        .map(sql_u64)
        .transpose()?;
    let transaction = store
        .connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|_| RuntimeArtifactStoreError::Storage)?;
    verify_manifest_producer(&transaction, manifest)?;
    let retained_payload = transaction
        .query_row(
            "SELECT byte_size, lifecycle_state FROM runtime_payloads
             WHERE payload_sha256 = ?1",
            [&manifest.payload_sha256],
            |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?)),
        )
        .optional()
        .map_err(|_| RuntimeArtifactStoreError::Storage)?;
    match retained_payload {
        Some((retained_size, retained_state)) => {
            if retained_size != byte_size || retained_state != "active" {
                return Err(RuntimeArtifactStoreError::Integrity);
            }
        }
        None => {
            transaction
                .execute(
                    "INSERT INTO runtime_payloads(
                         payload_sha256, byte_size, lifecycle_state,
                         active_reference_count, created_at_epoch_ms, updated_at_epoch_ms
                     ) VALUES (?1, ?2, 'active', 0, ?3, ?3)",
                    params![&manifest.payload_sha256, byte_size, created_at],
                )
                .map_err(|_| RuntimeArtifactStoreError::Storage)?;
        }
    }
    let retained_manifest = transaction
        .query_row(
            "SELECT manifest_sha256, manifest_json FROM runtime_artifacts
             WHERE artifact_id = ?1",
            [manifest.artifact_id.as_str()],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, Vec<u8>>(1)?)),
        )
        .optional()
        .map_err(|_| RuntimeArtifactStoreError::Storage)?;
    if let Some((retained_sha256, retained_json)) = retained_manifest {
        if retained_sha256 != manifest.manifest_sha256 || retained_json != record_json {
            return Err(RuntimeArtifactStoreError::Integrity);
        }
        transaction
            .commit()
            .map_err(|_| RuntimeArtifactStoreError::Storage)?;
        return Ok(());
    }
    transaction
        .execute(
            "INSERT INTO runtime_artifacts(
                 artifact_id, manifest_sha256, payload_sha256, byte_size, media_type,
                 artifact_kind, sensitivity, retention_kind, retention_expires_at_epoch_ms,
                 session_id, task_id, producer_run_id, producer_turn_id,
                 producer_operation_id, receipt_id, policy_id, policy_sha256,
                 created_at_epoch_ms, manifest_json
             ) VALUES (
                 ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9,
                 ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19
             )",
            params![
                manifest.artifact_id.as_str(),
                &manifest.manifest_sha256,
                &manifest.payload_sha256,
                byte_size,
                &manifest.media_type,
                artifact_kind_text(manifest.kind),
                sensitivity_text(manifest.sensitivity),
                retention_text(manifest.retention.kind),
                expires_at,
                manifest.session_id.as_str(),
                manifest.task_id.as_str(),
                manifest.producer_run_id.as_str(),
                manifest
                    .producer_turn_id
                    .as_ref()
                    .map(|value| value.as_str()),
                manifest
                    .producer_operation_id
                    .as_ref()
                    .map(|value| value.as_str()),
                manifest.receipt_id.as_ref().map(|value| value.as_str()),
                manifest.policy_id.as_str(),
                &manifest.policy_sha256,
                created_at,
                record_json,
            ],
        )
        .map_err(|_| RuntimeArtifactStoreError::Storage)?;
    insert_initial_artifact_state(&transaction, manifest)?;
    let changed = transaction
        .execute(
            "UPDATE runtime_payloads
             SET active_reference_count = active_reference_count + 1,
                 updated_at_epoch_ms = MAX(updated_at_epoch_ms, ?1)
             WHERE payload_sha256 = ?2 AND lifecycle_state = 'active'",
            params![created_at, &manifest.payload_sha256],
        )
        .map_err(|_| RuntimeArtifactStoreError::Storage)?;
    if changed != 1 {
        return Err(RuntimeArtifactStoreError::Integrity);
    }
    transaction
        .commit()
        .map_err(|_| RuntimeArtifactStoreError::Storage)
}

fn insert_initial_artifact_state(
    transaction: &Transaction<'_>,
    manifest: &RuntimeArtifactManifest,
) -> Result<(), RuntimeArtifactStoreError> {
    let revision = 1_u64;
    let reason = "artifact.published";
    let state_sha256 = artifact_state_digest(
        &manifest.artifact_id,
        revision,
        RuntimeArtifactLifecycleState::Active,
        RuntimeArtifactIntegrityState::Verified,
        reason,
        manifest.created_at_epoch_ms,
    );
    transaction
        .execute(
            "INSERT INTO runtime_artifact_states(
                 artifact_id, revision, lifecycle_state, integrity_state,
                 reason_code, updated_at_epoch_ms, state_sha256
             ) VALUES (?1, 1, 'active', 'verified', ?2, ?3, ?4)",
            params![
                manifest.artifact_id.as_str(),
                reason,
                sql_u64(manifest.created_at_epoch_ms)?,
                &state_sha256,
            ],
        )
        .map_err(|_| RuntimeArtifactStoreError::Storage)?;
    let event_sha256 = artifact_event_digest(
        &manifest.artifact_id,
        revision,
        RuntimeArtifactLifecycleState::Active,
        RuntimeArtifactIntegrityState::Verified,
        reason,
        manifest.created_at_epoch_ms,
        ZERO_SHA256,
        &state_sha256,
    );
    transaction
        .execute(
            "INSERT INTO runtime_artifact_events(
                 artifact_id, revision, lifecycle_state, integrity_state, reason_code,
                 occurred_at_epoch_ms, previous_event_sha256, state_sha256, event_sha256
             ) VALUES (?1, 1, 'active', 'verified', ?2, ?3, ?4, ?5, ?6)",
            params![
                manifest.artifact_id.as_str(),
                reason,
                sql_u64(manifest.created_at_epoch_ms)?,
                ZERO_SHA256,
                &state_sha256,
                &event_sha256,
            ],
        )
        .map_err(|_| RuntimeArtifactStoreError::Storage)?;
    Ok(())
}

fn load_artifact_manifest(
    store: &OperationalStore,
    artifact_id: &RuntimeArtifactId,
) -> Result<RuntimeArtifactManifest, RuntimeArtifactStoreError> {
    let row = store
        .connection
        .query_row(
            "SELECT manifest_sha256, payload_sha256, byte_size, media_type,
                    artifact_kind, sensitivity, retention_kind,
                    retention_expires_at_epoch_ms, session_id, task_id,
                    producer_run_id, producer_turn_id, producer_operation_id, receipt_id,
                    policy_id, policy_sha256, created_at_epoch_ms, manifest_json
             FROM runtime_artifacts WHERE artifact_id = ?1",
            [artifact_id.as_str()],
            |row| {
                Ok(RetainedManifestRow {
                    manifest_sha256: row.get(0)?,
                    payload_sha256: row.get(1)?,
                    byte_size: row.get(2)?,
                    media_type: row.get(3)?,
                    artifact_kind: row.get(4)?,
                    sensitivity: row.get(5)?,
                    retention_kind: row.get(6)?,
                    retention_expires_at_epoch_ms: row.get(7)?,
                    session_id: row.get(8)?,
                    task_id: row.get(9)?,
                    producer_run_id: row.get(10)?,
                    producer_turn_id: row.get(11)?,
                    producer_operation_id: row.get(12)?,
                    receipt_id: row.get(13)?,
                    policy_id: row.get(14)?,
                    policy_sha256: row.get(15)?,
                    created_at_epoch_ms: row.get(16)?,
                    manifest_json: row.get(17)?,
                })
            },
        )
        .optional()
        .map_err(|_| RuntimeArtifactStoreError::Storage)?
        .ok_or(RuntimeArtifactStoreError::NotFound)?;
    let manifest = from_json::<RuntimeArtifactManifest>(&row.manifest_json)
        .map_err(|_| RuntimeArtifactStoreError::Integrity)?;
    verify_runtime_artifact_manifest(&manifest)
        .map_err(|_| RuntimeArtifactStoreError::Integrity)?;
    if manifest.artifact_id != *artifact_id || !row.matches(&manifest) {
        return Err(RuntimeArtifactStoreError::Integrity);
    }
    Ok(manifest)
}

fn load_artifact_state(
    store: &OperationalStore,
    reference: &RuntimeArtifactRef,
) -> Result<RuntimeArtifactState, RuntimeArtifactStoreError> {
    let retained = store
        .connection
        .query_row(
            "SELECT revision, lifecycle_state, integrity_state, reason_code,
                    updated_at_epoch_ms, state_sha256
             FROM runtime_artifact_states WHERE artifact_id = ?1",
            [reference.artifact_id.as_str()],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, i64>(4)?,
                    row.get::<_, String>(5)?,
                ))
            },
        )
        .optional()
        .map_err(|_| RuntimeArtifactStoreError::Storage)?
        .ok_or(RuntimeArtifactStoreError::Integrity)?;
    let revision = u64::try_from(retained.0).map_err(|_| RuntimeArtifactStoreError::Integrity)?;
    let lifecycle = parse_lifecycle(&retained.1)?;
    let integrity = parse_integrity(&retained.2)?;
    let updated_at_epoch_ms =
        u64::try_from(retained.4).map_err(|_| RuntimeArtifactStoreError::Integrity)?;
    let expected = artifact_state_digest(
        &reference.artifact_id,
        revision,
        lifecycle,
        integrity,
        &retained.3,
        updated_at_epoch_ms,
    );
    if retained.5 != expected {
        return Err(RuntimeArtifactStoreError::Integrity);
    }
    Ok(RuntimeArtifactState {
        reference: reference.clone(),
        lifecycle,
        integrity,
        revision,
        reason_code: retained.3,
        updated_at_epoch_ms,
    })
}

fn transition_artifact(
    store: &mut OperationalStore,
    manifest: &RuntimeArtifactManifest,
    lifecycle: RuntimeArtifactLifecycleState,
    integrity: RuntimeArtifactIntegrityState,
    reason_code: &str,
    occurred_at_epoch_ms: u64,
) -> Result<(), RuntimeArtifactStoreError> {
    if !valid_reason_code(reason_code) || occurred_at_epoch_ms < manifest.created_at_epoch_ms {
        return Err(RuntimeArtifactStoreError::Contract(
            RuntimeArtifactError::InvalidManifest,
        ));
    }
    let transaction = store
        .connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|_| RuntimeArtifactStoreError::Storage)?;
    transition_artifact_in_transaction(
        &transaction,
        manifest,
        lifecycle,
        integrity,
        reason_code,
        occurred_at_epoch_ms,
    )?;
    transaction
        .commit()
        .map_err(|_| RuntimeArtifactStoreError::Storage)
}

fn transition_artifact_in_transaction(
    transaction: &Transaction<'_>,
    manifest: &RuntimeArtifactManifest,
    lifecycle: RuntimeArtifactLifecycleState,
    integrity: RuntimeArtifactIntegrityState,
    reason_code: &str,
    occurred_at_epoch_ms: u64,
) -> Result<(), RuntimeArtifactStoreError> {
    let current = transaction
        .query_row(
            "SELECT revision, lifecycle_state, integrity_state, updated_at_epoch_ms
             FROM runtime_artifact_states WHERE artifact_id = ?1",
            [manifest.artifact_id.as_str()],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, i64>(3)?,
                ))
            },
        )
        .map_err(|_| RuntimeArtifactStoreError::Integrity)?;
    let revision = u64::try_from(current.0).map_err(|_| RuntimeArtifactStoreError::Integrity)?;
    let current_lifecycle = parse_lifecycle(&current.1)?;
    let current_integrity = parse_integrity(&current.2)?;
    let current_time =
        u64::try_from(current.3).map_err(|_| RuntimeArtifactStoreError::Integrity)?;
    if occurred_at_epoch_ms < current_time {
        return Err(RuntimeArtifactStoreError::Contract(
            RuntimeArtifactError::InvalidManifest,
        ));
    }
    if current_lifecycle == lifecycle && current_integrity == integrity {
        return Ok(());
    }
    if !legal_lifecycle_transition(current_lifecycle, lifecycle, integrity) {
        return Err(RuntimeArtifactStoreError::NotAuthorized);
    }
    let next_revision = revision
        .checked_add(1)
        .ok_or(RuntimeArtifactStoreError::Integrity)?;
    let state_sha256 = artifact_state_digest(
        &manifest.artifact_id,
        next_revision,
        lifecycle,
        integrity,
        reason_code,
        occurred_at_epoch_ms,
    );
    let changed = transaction
        .execute(
            "UPDATE runtime_artifact_states
             SET revision = ?1, lifecycle_state = ?2, integrity_state = ?3,
                 reason_code = ?4, updated_at_epoch_ms = ?5, state_sha256 = ?6
             WHERE artifact_id = ?7 AND revision = ?8",
            params![
                sql_u64(next_revision)?,
                lifecycle_text(lifecycle),
                integrity_text(integrity),
                reason_code,
                sql_u64(occurred_at_epoch_ms)?,
                &state_sha256,
                manifest.artifact_id.as_str(),
                sql_u64(revision)?,
            ],
        )
        .map_err(|_| RuntimeArtifactStoreError::Storage)?;
    if changed != 1 {
        return Err(RuntimeArtifactStoreError::Integrity);
    }
    let previous_event_sha256: String = transaction
        .query_row(
            "SELECT event_sha256 FROM runtime_artifact_events
             WHERE artifact_id = ?1 AND revision = ?2",
            params![manifest.artifact_id.as_str(), sql_u64(revision)?],
            |row| row.get(0),
        )
        .map_err(|_| RuntimeArtifactStoreError::Integrity)?;
    let event_sha256 = artifact_event_digest(
        &manifest.artifact_id,
        next_revision,
        lifecycle,
        integrity,
        reason_code,
        occurred_at_epoch_ms,
        &previous_event_sha256,
        &state_sha256,
    );
    transaction
        .execute(
            "INSERT INTO runtime_artifact_events(
                 artifact_id, revision, lifecycle_state, integrity_state, reason_code,
                 occurred_at_epoch_ms, previous_event_sha256, state_sha256, event_sha256
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                manifest.artifact_id.as_str(),
                sql_u64(next_revision)?,
                lifecycle_text(lifecycle),
                integrity_text(integrity),
                reason_code,
                sql_u64(occurred_at_epoch_ms)?,
                &previous_event_sha256,
                &state_sha256,
                &event_sha256,
            ],
        )
        .map_err(|_| RuntimeArtifactStoreError::Storage)?;
    if current_lifecycle == RuntimeArtifactLifecycleState::Active
        && lifecycle != RuntimeArtifactLifecycleState::Active
    {
        let changed = transaction
            .execute(
                "UPDATE runtime_payloads
                 SET active_reference_count = active_reference_count - 1,
                     updated_at_epoch_ms = MAX(updated_at_epoch_ms, ?1)
                 WHERE payload_sha256 = ?2 AND active_reference_count > 0",
                params![sql_u64(occurred_at_epoch_ms)?, &manifest.payload_sha256],
            )
            .map_err(|_| RuntimeArtifactStoreError::Storage)?;
        if changed != 1 {
            return Err(RuntimeArtifactStoreError::Integrity);
        }
    }
    Ok(())
}

fn legal_lifecycle_transition(
    current: RuntimeArtifactLifecycleState,
    next: RuntimeArtifactLifecycleState,
    integrity: RuntimeArtifactIntegrityState,
) -> bool {
    matches!(
        (current, next, integrity),
        (
            RuntimeArtifactLifecycleState::Active,
            RuntimeArtifactLifecycleState::Released,
            RuntimeArtifactIntegrityState::Verified
        ) | (
            RuntimeArtifactLifecycleState::Active,
            RuntimeArtifactLifecycleState::Quarantined,
            RuntimeArtifactIntegrityState::Quarantined
                | RuntimeArtifactIntegrityState::Missing
                | RuntimeArtifactIntegrityState::Corrupt
        ) | (
            RuntimeArtifactLifecycleState::Released | RuntimeArtifactLifecycleState::Quarantined,
            RuntimeArtifactLifecycleState::Deleted,
            RuntimeArtifactIntegrityState::Deleted
        )
    )
}

struct RetainedManifestRow {
    manifest_sha256: String,
    payload_sha256: String,
    byte_size: i64,
    media_type: String,
    artifact_kind: String,
    sensitivity: String,
    retention_kind: String,
    retention_expires_at_epoch_ms: Option<i64>,
    session_id: String,
    task_id: String,
    producer_run_id: String,
    producer_turn_id: Option<String>,
    producer_operation_id: Option<String>,
    receipt_id: Option<String>,
    policy_id: String,
    policy_sha256: String,
    created_at_epoch_ms: i64,
    manifest_json: Vec<u8>,
}

impl RetainedManifestRow {
    fn matches(&self, manifest: &RuntimeArtifactManifest) -> bool {
        self.manifest_sha256 == manifest.manifest_sha256
            && self.payload_sha256 == manifest.payload_sha256
            && u64::try_from(self.byte_size).ok() == Some(manifest.byte_size)
            && self.media_type == manifest.media_type
            && self.artifact_kind == artifact_kind_text(manifest.kind)
            && self.sensitivity == sensitivity_text(manifest.sensitivity)
            && self.retention_kind == retention_text(manifest.retention.kind)
            && self
                .retention_expires_at_epoch_ms
                .and_then(|value| u64::try_from(value).ok())
                == manifest.retention.expires_at_epoch_ms
            && self.session_id == manifest.session_id.as_str()
            && self.task_id == manifest.task_id.as_str()
            && self.producer_run_id == manifest.producer_run_id.as_str()
            && self.producer_turn_id.as_deref()
                == manifest
                    .producer_turn_id
                    .as_ref()
                    .map(|value| value.as_str())
            && self.producer_operation_id.as_deref()
                == manifest
                    .producer_operation_id
                    .as_ref()
                    .map(|value| value.as_str())
            && self.receipt_id.as_deref()
                == manifest.receipt_id.as_ref().map(|value| value.as_str())
            && self.policy_id == manifest.policy_id.as_str()
            && self.policy_sha256 == manifest.policy_sha256
            && u64::try_from(self.created_at_epoch_ms).ok() == Some(manifest.created_at_epoch_ms)
    }
}

fn payload_observation(manifest: &RuntimeArtifactManifest) -> RuntimeArtifactPayloadObservation {
    RuntimeArtifactPayloadObservation {
        payload_sha256: manifest.payload_sha256.clone(),
        byte_size: manifest.byte_size,
    }
}

fn sql_u64(value: u64) -> Result<i64, RuntimeArtifactStoreError> {
    i64::try_from(value)
        .map_err(|_| RuntimeArtifactStoreError::Contract(RuntimeArtifactError::InvalidManifest))
}

const fn artifact_kind_text(kind: RuntimeArtifactKind) -> &'static str {
    match kind {
        RuntimeArtifactKind::Patch => "patch",
        RuntimeArtifactKind::StandardOutput => "standard_output",
        RuntimeArtifactKind::StandardError => "standard_error",
        RuntimeArtifactKind::TestLog => "test_log",
        RuntimeArtifactKind::GeneratedFile => "generated_file",
        RuntimeArtifactKind::Report => "report",
        RuntimeArtifactKind::ModelOutput => "model_output",
    }
}

const fn sensitivity_text(sensitivity: ContextSensitivity) -> &'static str {
    match sensitivity {
        ContextSensitivity::Public => "public",
        ContextSensitivity::Internal => "internal",
        ContextSensitivity::Private => "private",
        ContextSensitivity::Restricted => "restricted",
    }
}

const fn retention_text(retention: RuntimeEventRetentionKind) -> &'static str {
    match retention {
        RuntimeEventRetentionKind::Ephemeral => "ephemeral",
        RuntimeEventRetentionKind::Session => "session",
        RuntimeEventRetentionKind::UntilExpiration => "until_expiration",
        RuntimeEventRetentionKind::UserHold => "user_hold",
    }
}

const fn lifecycle_text(lifecycle: RuntimeArtifactLifecycleState) -> &'static str {
    match lifecycle {
        RuntimeArtifactLifecycleState::Active => "active",
        RuntimeArtifactLifecycleState::Quarantined => "quarantined",
        RuntimeArtifactLifecycleState::Released => "released",
        RuntimeArtifactLifecycleState::Deleted => "deleted",
    }
}

const fn integrity_text(integrity: RuntimeArtifactIntegrityState) -> &'static str {
    match integrity {
        RuntimeArtifactIntegrityState::Verified => "verified",
        RuntimeArtifactIntegrityState::Quarantined => "quarantined",
        RuntimeArtifactIntegrityState::Missing => "missing",
        RuntimeArtifactIntegrityState::Corrupt => "corrupt",
        RuntimeArtifactIntegrityState::Deleted => "deleted",
    }
}

fn parse_lifecycle(
    value: &str,
) -> Result<RuntimeArtifactLifecycleState, RuntimeArtifactStoreError> {
    match value {
        "active" => Ok(RuntimeArtifactLifecycleState::Active),
        "quarantined" => Ok(RuntimeArtifactLifecycleState::Quarantined),
        "released" => Ok(RuntimeArtifactLifecycleState::Released),
        "deleted" => Ok(RuntimeArtifactLifecycleState::Deleted),
        _ => Err(RuntimeArtifactStoreError::Integrity),
    }
}

fn parse_integrity(
    value: &str,
) -> Result<RuntimeArtifactIntegrityState, RuntimeArtifactStoreError> {
    match value {
        "verified" => Ok(RuntimeArtifactIntegrityState::Verified),
        "quarantined" => Ok(RuntimeArtifactIntegrityState::Quarantined),
        "missing" => Ok(RuntimeArtifactIntegrityState::Missing),
        "corrupt" => Ok(RuntimeArtifactIntegrityState::Corrupt),
        "deleted" => Ok(RuntimeArtifactIntegrityState::Deleted),
        _ => Err(RuntimeArtifactStoreError::Integrity),
    }
}

fn valid_reason_code(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'.' | b'_' | b'-')
        })
}

fn artifact_state_digest(
    artifact_id: &RuntimeArtifactId,
    revision: u64,
    lifecycle: RuntimeArtifactLifecycleState,
    integrity: RuntimeArtifactIntegrityState,
    reason_code: &str,
    updated_at_epoch_ms: u64,
) -> String {
    digest_fields(&[
        "agentmage.runtime-artifact-state.v1",
        artifact_id.as_str(),
        &revision.to_string(),
        lifecycle_text(lifecycle),
        integrity_text(integrity),
        reason_code,
        &updated_at_epoch_ms.to_string(),
    ])
}

#[allow(clippy::too_many_arguments)]
fn artifact_event_digest(
    artifact_id: &RuntimeArtifactId,
    revision: u64,
    lifecycle: RuntimeArtifactLifecycleState,
    integrity: RuntimeArtifactIntegrityState,
    reason_code: &str,
    occurred_at_epoch_ms: u64,
    previous_event_sha256: &str,
    state_sha256: &str,
) -> String {
    digest_fields(&[
        "agentmage.runtime-artifact-event.v1",
        artifact_id.as_str(),
        &revision.to_string(),
        lifecycle_text(lifecycle),
        integrity_text(integrity),
        reason_code,
        &occurred_at_epoch_ms.to_string(),
        previous_event_sha256,
        state_sha256,
    ])
}

fn digest_fields(fields: &[&str]) -> String {
    let mut digest = Sha256::new();
    for field in fields {
        digest.update(field.as_bytes());
        digest.update([0]);
    }
    digest
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn continuation_digest(
    continuation: &RuntimeContinuationState,
) -> Result<String, RuntimeArtifactError> {
    let mut candidate = continuation.clone();
    candidate.continuation_sha256 = ZERO_SHA256.to_owned();
    continuation_json(&candidate).map(|bytes| sha256(&bytes))
}

fn continuation_json(
    continuation: &RuntimeContinuationState,
) -> Result<Vec<u8>, RuntimeArtifactError> {
    let bytes =
        serde_json::to_vec(continuation).map_err(|_| RuntimeArtifactError::Serialization)?;
    if bytes.is_empty() || bytes.len() as u64 > MAX_RUNTIME_ARTIFACT_BYTES {
        return Err(RuntimeArtifactError::Serialization);
    }
    Ok(bytes)
}

fn validate_runtime_continuation_state(
    continuation: &RuntimeContinuationState,
) -> Result<(), RuntimeArtifactError> {
    if continuation.schema_version != CONTRACT_SCHEMA_VERSION
        || !valid_sha256(&continuation.request_sha256)
        || !valid_identifier(continuation.run_id.as_str())
        || !valid_identifier(continuation.session_id.as_str())
        || !valid_identifier(continuation.task_id.as_str())
        || continuation.event_cursor.run_id != continuation.run_id
        || !valid_identifier(continuation.event_cursor.event_id.as_str())
        || !valid_sha256(&continuation.event_cursor.event_sha256)
        || continuation.agent_state != AgentStateKind::Observation
        || continuation.agent_state_revision == 0
        || continuation.state_transitions.len() > MAX_RUNTIME_CONTINUATION_TRANSITIONS
        || continuation.agent_state_revision != continuation.state_transitions.len() as u64 + 1
        || !valid_continuation_transitions(continuation)
        || continuation.turn_count == 0
        || continuation.model_call_count != continuation.turn_count
        || continuation.context_refresh_count != continuation.turn_count
        || continuation.tool_call_count as usize != continuation.tool_results.len()
        || continuation.tool_call_count as usize != continuation.tool_attempts.len()
        || continuation.tool_results.len() > MAX_RUNTIME_CONTINUATION_RESULTS
        || continuation.receipt_ids.len() != continuation.tool_results.len()
        || continuation.no_progress_turns > continuation.turn_count
        || !valid_continuation_resources(continuation)
        || !valid_continuation_tool_results(&continuation.tool_results)
        || !valid_continuation_tool_attempts(&continuation.tool_attempts)
        || !continuation
            .tool_results
            .iter()
            .zip(&continuation.tool_attempts)
            .all(|(result, attempt)| result.tool_call_id == attempt.tool_call_id)
        || !valid_sorted_evidence(&continuation.evidence)
        || !valid_sorted_receipts(&continuation.receipt_ids)
        || !valid_sorted_artifacts(&continuation.artifacts)
        || !valid_sha256(&continuation.continuation_sha256)
    {
        return Err(RuntimeArtifactError::InvalidContinuation);
    }
    Ok(())
}

fn valid_continuation_resources(continuation: &RuntimeContinuationState) -> bool {
    let resources = &continuation.resources;
    let artifact_bytes = continuation
        .artifacts
        .iter()
        .try_fold(0_u64, |total, artifact| {
            total.checked_add(artifact.byte_size)
        });
    resources.plan_steps == u64::from(continuation.turn_count)
        && resources.model_calls == u64::from(continuation.model_call_count)
        && resources.tool_calls == u64::from(continuation.tool_call_count)
        && u32::try_from(continuation.event_cursor.sequence.saturating_add(1)).ok()
            == Some(resources.event_count)
        && resources.event_bytes >= u64::from(resources.event_count)
        && usize::try_from(resources.artifact_count).ok() == Some(continuation.artifacts.len())
        && artifact_bytes == Some(resources.artifact_bytes)
        && resources.disk_bytes >= resources.artifact_bytes
        && resources.denial_count == 0
        && resources.parser_failure_count == 0
        && resources.retry_count == 0
}

fn valid_continuation_transitions(continuation: &RuntimeContinuationState) -> bool {
    let mut current = AgentStateKind::Observation;
    for (index, transition) in continuation.state_transitions.iter().enumerate() {
        let Some(revision) = u64::try_from(index)
            .ok()
            .and_then(|value| value.checked_add(2))
        else {
            return false;
        };
        if transition.schema_version != CONTRACT_SCHEMA_VERSION
            || transition.revision != revision
            || transition.from != current
            || transition.to.is_terminal()
            || !crate::agent_state::legal_transition(transition.from, transition.to)
        {
            return false;
        }
        current = transition.to;
    }
    current == continuation.agent_state
}

fn valid_continuation_tool_results(results: &[agentmage_kernel_contracts::ToolResult]) -> bool {
    let mut call_ids = BTreeSet::new();
    results.iter().all(|result| {
        result.schema_version == CONTRACT_SCHEMA_VERSION
            && valid_identifier(result.tool_call_id.as_str())
            && valid_identifier(result.correlation_id.as_str())
            && result.outcome == agentmage_kernel_contracts::OperationOutcome::Succeeded
            && result.validation_issues.is_empty()
            && result.error.is_none()
            && result.state_change != agentmage_kernel_contracts::StateChange::Uncertain
            && result.output.as_ref().is_none_or(|payload| {
                payload.schema.schema_version > 0
                    && valid_identifier(payload.schema.schema_id.as_str())
                    && valid_sha256(&payload.schema.schema_sha256)
                    && valid_media_type(&payload.media_type)
                    && !payload.bytes.is_empty()
                    && payload.bytes.len() as u64 <= MAX_RUNTIME_ARTIFACT_BYTES
                    && payload.sha256 == sha256(&payload.bytes)
            })
            && valid_sorted_evidence(&result.evidence)
            && call_ids.insert(result.tool_call_id.as_str())
    })
}

fn valid_continuation_tool_attempts(
    attempts: &[agentmage_kernel_contracts::RuntimeToolAttemptState],
) -> bool {
    let mut call_ids = BTreeSet::new();
    attempts.iter().enumerate().all(|(index, attempt)| {
        attempt.schema_version == CONTRACT_SCHEMA_VERSION
            && attempt.sequence == index as u64 + 1
            && valid_identifier(attempt.tool_call_id.as_str())
            && valid_sha256(&attempt.semantic_sha256)
            && attempt.occurrence > 0
            && attempt.call_depth <= 32
            && call_ids.insert(attempt.tool_call_id.as_str())
    })
}

fn valid_sorted_evidence(evidence: &[agentmage_kernel_contracts::EvidenceReference]) -> bool {
    evidence
        .windows(2)
        .all(|pair| pair[0].evidence_id.as_str() < pair[1].evidence_id.as_str())
        && evidence.iter().all(|item| {
            item.schema_version == CONTRACT_SCHEMA_VERSION
                && valid_identifier(item.evidence_id.as_str())
                && !item.source_id.is_empty()
                && !item.object_id.is_empty()
                && valid_sha256(&item.content_sha256)
        })
}

fn valid_sorted_receipts(receipts: &[agentmage_kernel_contracts::ReceiptId]) -> bool {
    receipts
        .windows(2)
        .all(|pair| pair[0].as_str() < pair[1].as_str())
        && receipts
            .iter()
            .all(|receipt| valid_identifier(receipt.as_str()))
}

fn valid_sorted_artifacts(artifacts: &[RuntimeArtifactRef]) -> bool {
    artifacts.len() <= MAX_RUNTIME_ARTIFACTS_PER_CHECKPOINT
        && artifacts
            .windows(2)
            .all(|pair| pair[0].artifact_id.as_str() < pair[1].artifact_id.as_str())
        && artifacts.iter().all(|reference| {
            reference.schema_version == CONTRACT_SCHEMA_VERSION
                && valid_identifier(reference.artifact_id.as_str())
                && valid_sha256(&reference.manifest_sha256)
                && valid_sha256(&reference.payload_sha256)
                && reference.byte_size > 0
                && reference.byte_size <= MAX_RUNTIME_ARTIFACT_BYTES
                && valid_media_type(&reference.media_type)
        })
}

fn validate_manifest_fields(
    manifest: &RuntimeArtifactManifest,
) -> Result<(), RuntimeArtifactError> {
    if manifest.schema_version != CONTRACT_SCHEMA_VERSION
        || !valid_identifier(manifest.artifact_id.as_str())
        || !valid_sha256(&manifest.payload_sha256)
        || manifest.byte_size == 0
        || manifest.byte_size > MAX_RUNTIME_ARTIFACT_BYTES
        || !valid_media_type(&manifest.media_type)
        || !valid_identifier(manifest.session_id.as_str())
        || !valid_identifier(manifest.task_id.as_str())
        || !valid_identifier(manifest.producer_run_id.as_str())
        || manifest
            .producer_turn_id
            .as_ref()
            .is_some_and(|value| !valid_identifier(value.as_str()))
        || manifest
            .producer_operation_id
            .as_ref()
            .is_some_and(|value| !valid_identifier(value.as_str()))
        || manifest
            .receipt_id
            .as_ref()
            .is_some_and(|value| !valid_identifier(value.as_str()))
        || manifest.receipt_id.is_some() && manifest.producer_operation_id.is_none()
        || manifest.producer_operation_id.is_some() && manifest.producer_turn_id.is_none()
        || !valid_identifier(manifest.policy_id.as_str())
        || !valid_sha256(&manifest.policy_sha256)
        || manifest.created_at_epoch_ms == 0
        || manifest.integrity != RuntimeArtifactIntegrityState::Verified
        || !valid_retention(manifest)
        || !valid_kind_media(manifest.kind, &manifest.media_type)
        || manifest.preview.as_ref().is_some_and(|preview| {
            preview.text.is_empty()
                || preview.text.len() > MAX_RUNTIME_ARTIFACT_PREVIEW_BYTES
                || usize::try_from(preview.byte_size).ok() != Some(preview.text.len())
                || u64::from(preview.byte_size) > manifest.byte_size
                || preview.truncated != (u64::from(preview.byte_size) < manifest.byte_size)
                || !valid_sha256(&preview.sha256)
                || preview.sha256 != sha256(preview.text.as_bytes())
        })
    {
        return Err(RuntimeArtifactError::InvalidManifest);
    }
    Ok(())
}

fn validate_resume_binding_fields(
    binding: &RuntimeResumeBinding,
) -> Result<(), RuntimeArtifactError> {
    if binding.schema_version != CONTRACT_SCHEMA_VERSION
        || !valid_identifier(binding.checkpoint_id.as_str())
        || !valid_sha256(&binding.checkpoint_sha256)
        || !valid_identifier(binding.session_id.as_str())
        || !valid_identifier(binding.task_id.as_str())
        || !valid_identifier(binding.run_id.as_str())
        || binding.event_cursor.run_id != binding.run_id
        || !valid_identifier(binding.event_cursor.event_id.as_str())
        || !valid_sha256(&binding.event_cursor.event_sha256)
        || binding.artifacts.len() > MAX_RUNTIME_ARTIFACTS_PER_CHECKPOINT
    {
        return Err(RuntimeArtifactError::InvalidResumeBinding);
    }
    let mut prior: Option<&str> = None;
    let mut identities = BTreeSet::new();
    for reference in &binding.artifacts {
        if reference.schema_version != CONTRACT_SCHEMA_VERSION
            || !valid_identifier(reference.artifact_id.as_str())
            || !valid_sha256(&reference.manifest_sha256)
            || !valid_sha256(&reference.payload_sha256)
            || reference.byte_size == 0
            || reference.byte_size > MAX_RUNTIME_ARTIFACT_BYTES
            || !valid_media_type(&reference.media_type)
            || prior.is_some_and(|value| value >= reference.artifact_id.as_str())
            || !identities.insert(reference.artifact_id.as_str())
        {
            return Err(RuntimeArtifactError::InvalidResumeBinding);
        }
        prior = Some(reference.artifact_id.as_str());
    }
    Ok(())
}

fn valid_retention(manifest: &RuntimeArtifactManifest) -> bool {
    match manifest.retention.kind {
        RuntimeEventRetentionKind::Ephemeral => false,
        RuntimeEventRetentionKind::Session | RuntimeEventRetentionKind::UserHold => {
            manifest.retention.expires_at_epoch_ms.is_none()
        }
        RuntimeEventRetentionKind::UntilExpiration => manifest
            .retention
            .expires_at_epoch_ms
            .is_some_and(|expires| expires > manifest.created_at_epoch_ms),
    }
}

fn valid_kind_media(kind: RuntimeArtifactKind, media_type: &str) -> bool {
    match kind {
        RuntimeArtifactKind::Patch => matches!(media_type, "text/x-diff" | "text/plain"),
        RuntimeArtifactKind::StandardOutput
        | RuntimeArtifactKind::StandardError
        | RuntimeArtifactKind::TestLog => matches!(
            media_type,
            "text/plain" | "application/json" | "application/x-ndjson" | "application/octet-stream"
        ),
        RuntimeArtifactKind::ModelOutput => matches!(
            media_type,
            "text/plain" | "application/json" | "application/x-ndjson"
        ),
        RuntimeArtifactKind::GeneratedFile | RuntimeArtifactKind::Report => true,
    }
}

fn valid_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b':' | b'/' | b'-')
        })
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn valid_media_type(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value.bytes().all(|byte| {
            byte.is_ascii_lowercase()
                || byte.is_ascii_digit()
                || matches!(byte, b'/' | b'.' | b'+' | b'-')
        })
        && value.split('/').filter(|part| !part.is_empty()).count() == 2
        && value.matches('/').count() == 1
}

fn manifest_digest(manifest: &RuntimeArtifactManifest) -> Result<String, RuntimeArtifactError> {
    let mut candidate = manifest.clone();
    candidate.manifest_sha256 = ZERO_SHA256.to_owned();
    let bytes = to_canonical_json(&candidate).map_err(|_| RuntimeArtifactError::Serialization)?;
    Ok(sha256(&bytes))
}

fn resume_binding_digest(binding: &RuntimeResumeBinding) -> Result<String, RuntimeArtifactError> {
    let mut candidate = binding.clone();
    candidate.binding_sha256 = ZERO_SHA256.to_owned();
    let bytes = to_canonical_json(&candidate).map_err(|_| RuntimeArtifactError::Serialization)?;
    Ok(sha256(&bytes))
}

fn sha256(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::fs;
    use std::io::{Cursor, Read};
    use std::sync::atomic::{AtomicU64, Ordering};

    use agentmage_kernel_contracts::{
        AgentStateKind, AgentStateTransition, CONTRACT_SCHEMA_VERSION, CheckpointFileIdentity,
        ContextSensitivity, CorrelationId, EvidenceId, ModelProfileId, PlanId, PlanStepId,
        PolicyId, RepositorySnapshotId, RuntimeArtifactCleanupState, RuntimeArtifactId,
        RuntimeArtifactIntegrityState, RuntimeArtifactKind, RuntimeArtifactLifecycleState,
        RuntimeArtifactManifest, RuntimeArtifactPreview, RuntimeContinuationState, RuntimeEvent,
        RuntimeEventCursor, RuntimeEventId, RuntimeEventKind, RuntimeEventPersistenceClass,
        RuntimeEventRetention, RuntimeEventRetentionKind, RuntimeOperationId, RuntimeResumeBinding,
        RuntimeRunId, RuntimeTurnId, SessionCheckpoint, SessionCheckpointId, SessionId,
        StorageFilesystemClass, StrictLocalStorageObservation, TaskId, WorkspaceId,
    };

    use super::{
        MAX_RUNTIME_ARTIFACT_BYTES, RuntimeArtifactError, RuntimeArtifactPageRequest,
        RuntimeArtifactPayloadError, RuntimeArtifactPayloadInventoryEntry,
        RuntimeArtifactPayloadInventoryIntegrity, RuntimeArtifactPayloadObservation,
        RuntimeArtifactPayloadPlacement, RuntimeArtifactPayloadStore, RuntimeArtifactReadRequest,
        RuntimeArtifactStoreError, decode_runtime_continuation_state,
        encode_runtime_continuation_state, runtime_artifact_ref, runtime_payload_reference,
        seal_runtime_artifact_manifest, seal_runtime_continuation_state,
        seal_runtime_resume_binding, verify_runtime_artifact_manifest, verify_runtime_artifact_ref,
        verify_runtime_continuation_state, verify_runtime_resume_binding,
    };
    use crate::context_management::finalize_checkpoint;
    use crate::operational_store::{
        DurableAuthorityError, DurableAuthorityRuntime, OperationalStore, OperationalStoreKeyError,
        OperationalStoreKeyProvider,
    };
    use crate::runtime_event::seal_runtime_event;

    static NEXT_DIRECTORY: AtomicU64 = AtomicU64::new(1);

    fn digest(value: char) -> String {
        value.to_string().repeat(64)
    }

    fn manifest() -> RuntimeArtifactManifest {
        seal_runtime_artifact_manifest(RuntimeArtifactManifest {
            schema_version: CONTRACT_SCHEMA_VERSION,
            artifact_id: RuntimeArtifactId::from_raw("runtime-artifact-1"),
            kind: RuntimeArtifactKind::TestLog,
            payload_sha256: "84d89877f0d4041efb6bf91a16f0248f2fd573e6af05c19f96bedb9f882f7882"
                .to_owned(),
            byte_size: 10,
            media_type: "text/plain".to_owned(),
            sensitivity: ContextSensitivity::Private,
            retention: RuntimeEventRetention {
                kind: RuntimeEventRetentionKind::Session,
                expires_at_epoch_ms: None,
            },
            session_id: SessionId::from_raw("session-1"),
            task_id: TaskId::from_raw("task-1"),
            producer_run_id: RuntimeRunId::from_raw("run-1"),
            producer_turn_id: Some(RuntimeTurnId::from_raw("turn-1")),
            producer_operation_id: Some(RuntimeOperationId::from_raw("operation-1")),
            receipt_id: Some(agentmage_kernel_contracts::ReceiptId::from_raw("receipt-1")),
            policy_id: PolicyId::from_raw("policy-1"),
            policy_sha256: digest('b'),
            created_at_epoch_ms: 1,
            integrity: RuntimeArtifactIntegrityState::Verified,
            preview: Some(RuntimeArtifactPreview {
                text: "0123456789".to_owned(),
                byte_size: 10,
                truncated: false,
                sha256: "84d89877f0d4041efb6bf91a16f0248f2fd573e6af05c19f96bedb9f882f7882"
                    .to_owned(),
            }),
            manifest_sha256: digest('0'),
        })
        .expect("valid manifest")
    }

    struct TestKey;

    impl OperationalStoreKeyProvider for TestKey {
        fn with_key<T>(
            &mut self,
            operation: impl FnOnce(&[u8]) -> T,
        ) -> Result<T, OperationalStoreKeyError> {
            Ok(operation(&[31; 32]))
        }
    }

    #[derive(Default)]
    struct FakePayloadStore {
        objects: BTreeMap<String, Vec<u8>>,
        quarantined: BTreeMap<String, Vec<u8>>,
        interrupted_staging: u64,
    }

    impl RuntimeArtifactPayloadStore for FakePayloadStore {
        type Staged = Vec<u8>;

        fn stage(
            &mut self,
            _artifact_id: &RuntimeArtifactId,
            source: &mut dyn Read,
            maximum_bytes: u64,
        ) -> Result<(Self::Staged, RuntimeArtifactPayloadObservation), RuntimeArtifactPayloadError>
        {
            let mut bytes = Vec::new();
            source
                .take(maximum_bytes + 1)
                .read_to_end(&mut bytes)
                .map_err(|_| RuntimeArtifactPayloadError::Durability)?;
            if bytes.is_empty() || bytes.len() as u64 > maximum_bytes {
                return Err(RuntimeArtifactPayloadError::ResourceLimit);
            }
            let observation = RuntimeArtifactPayloadObservation {
                payload_sha256: super::sha256(&bytes),
                byte_size: bytes.len() as u64,
            };
            Ok((bytes, observation))
        }

        fn discard_staged(
            &mut self,
            _staged: Self::Staged,
        ) -> Result<(), RuntimeArtifactPayloadError> {
            Ok(())
        }

        fn place(
            &mut self,
            staged: Self::Staged,
            expected: &RuntimeArtifactPayloadObservation,
        ) -> Result<RuntimeArtifactPayloadPlacement, RuntimeArtifactPayloadError> {
            if staged.len() as u64 != expected.byte_size
                || super::sha256(&staged) != expected.payload_sha256
            {
                return Err(RuntimeArtifactPayloadError::Invalid);
            }
            let deduplicated = match self.objects.get(&expected.payload_sha256) {
                Some(retained) if retained == &staged => true,
                Some(_) => return Err(RuntimeArtifactPayloadError::Conflict),
                None => {
                    self.objects.insert(expected.payload_sha256.clone(), staged);
                    false
                }
            };
            Ok(RuntimeArtifactPayloadPlacement {
                observation: expected.clone(),
                deduplicated,
            })
        }

        fn verify(
            &self,
            expected: &RuntimeArtifactPayloadObservation,
        ) -> Result<(), RuntimeArtifactPayloadError> {
            let bytes = self
                .objects
                .get(&expected.payload_sha256)
                .ok_or(RuntimeArtifactPayloadError::Missing)?;
            if bytes.len() as u64 != expected.byte_size
                || super::sha256(bytes) != expected.payload_sha256
            {
                return Err(RuntimeArtifactPayloadError::Corrupt);
            }
            Ok(())
        }

        fn read_complete(
            &self,
            expected: &RuntimeArtifactPayloadObservation,
            maximum_bytes: u64,
        ) -> Result<Vec<u8>, RuntimeArtifactPayloadError> {
            if expected.byte_size > maximum_bytes {
                return Err(RuntimeArtifactPayloadError::ResourceLimit);
            }
            self.verify(expected)?;
            Ok(self.objects[&expected.payload_sha256].clone())
        }

        fn read_range(
            &self,
            expected: &RuntimeArtifactPayloadObservation,
            offset: u64,
            maximum_bytes: u64,
        ) -> Result<Vec<u8>, RuntimeArtifactPayloadError> {
            self.verify(expected)?;
            let bytes = &self.objects[&expected.payload_sha256];
            let start =
                usize::try_from(offset).map_err(|_| RuntimeArtifactPayloadError::ResourceLimit)?;
            let maximum = usize::try_from(maximum_bytes)
                .map_err(|_| RuntimeArtifactPayloadError::ResourceLimit)?;
            if maximum == 0 || start >= bytes.len() {
                return Err(RuntimeArtifactPayloadError::ResourceLimit);
            }
            Ok(bytes[start..start.saturating_add(maximum).min(bytes.len())].to_vec())
        }

        fn quarantine(
            &mut self,
            expected: &RuntimeArtifactPayloadObservation,
        ) -> Result<(), RuntimeArtifactPayloadError> {
            let bytes = self
                .objects
                .remove(&expected.payload_sha256)
                .ok_or(RuntimeArtifactPayloadError::Missing)?;
            self.quarantined
                .insert(expected.payload_sha256.clone(), bytes);
            Ok(())
        }

        fn delete(
            &mut self,
            expected: &RuntimeArtifactPayloadObservation,
        ) -> Result<(), RuntimeArtifactPayloadError> {
            if self.objects.remove(&expected.payload_sha256).is_some() {
                Ok(())
            } else {
                Err(RuntimeArtifactPayloadError::Missing)
            }
        }

        fn inventory(
            &self,
        ) -> Result<Vec<RuntimeArtifactPayloadInventoryEntry>, RuntimeArtifactPayloadError>
        {
            Ok(self
                .objects
                .iter()
                .map(|(payload_sha256, bytes)| {
                    let verified = !bytes.is_empty()
                        && bytes.len() as u64 <= MAX_RUNTIME_ARTIFACT_BYTES
                        && super::sha256(bytes) == *payload_sha256;
                    RuntimeArtifactPayloadInventoryEntry {
                        payload_sha256: payload_sha256.clone(),
                        byte_size: if verified { bytes.len() as u64 } else { 0 },
                        integrity: if verified {
                            RuntimeArtifactPayloadInventoryIntegrity::Verified
                        } else {
                            RuntimeArtifactPayloadInventoryIntegrity::Corrupt
                        },
                    }
                })
                .collect())
        }

        fn cleanup_staging(&mut self) -> Result<u64, RuntimeArtifactPayloadError> {
            let count = self.interrupted_staging;
            self.interrupted_staging = 0;
            Ok(count)
        }
    }

    fn observation() -> StrictLocalStorageObservation {
        StrictLocalStorageObservation {
            filesystem: StorageFilesystemClass::Local,
            synchronization_marker: None,
            root_identity_sha256: [41; 32],
            symlink_free: true,
        }
    }

    fn temporary_directory() -> std::path::PathBuf {
        let sequence = NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "agentmage-runtime-artifact-{}-{sequence}",
            std::process::id()
        ));
        fs::create_dir(&path).expect("temporary directory");
        path
    }

    fn runtime_with_run(path: &std::path::Path) -> DurableAuthorityRuntime {
        let mut runtime =
            DurableAuthorityRuntime::open(path, &observation(), &mut TestKey, 1).expect("runtime");
        let event = seal_runtime_event(RuntimeEvent {
            schema_version: CONTRACT_SCHEMA_VERSION,
            event_id: RuntimeEventId::from_raw("event-runtime-artifact-start"),
            run_id: RuntimeRunId::from_raw("run-1"),
            session_id: SessionId::from_raw("session-1"),
            task_id: TaskId::from_raw("task-1"),
            turn_id: None,
            operation_id: None,
            correlation_id: CorrelationId::from_raw("correlation-1"),
            causation_event_id: None,
            sequence: 0,
            occurred_at_epoch_ms: 1,
            sensitivity: ContextSensitivity::Private,
            retention: RuntimeEventRetention {
                kind: RuntimeEventRetentionKind::Session,
                expires_at_epoch_ms: None,
            },
            persistence: RuntimeEventPersistenceClass::Correctness,
            policy_id: PolicyId::from_raw("policy-1"),
            payload_reference: None,
            kind: RuntimeEventKind::RunStarted {
                request_sha256: digest('c'),
            },
            previous_event_sha256: digest('0'),
            event_sha256: digest('0'),
        })
        .expect("run event");
        runtime
            .record_runtime_event(event)
            .expect("run start persists");
        runtime
    }

    fn manifest_with_id(artifact_id: &str) -> RuntimeArtifactManifest {
        let mut candidate = manifest();
        candidate.artifact_id = RuntimeArtifactId::from_raw(artifact_id);
        seal_runtime_artifact_manifest(candidate).expect("manifest identity reseals")
    }

    fn checkpoint() -> SessionCheckpoint {
        finalize_checkpoint(SessionCheckpoint {
            schema_version: CONTRACT_SCHEMA_VERSION,
            checkpoint_id: SessionCheckpointId::from_raw("checkpoint-runtime-artifact-1"),
            session_id: SessionId::from_raw("session-1"),
            task_id: TaskId::from_raw("task-1"),
            objective_sha256: digest('1'),
            plan_id: PlanId::from_raw("plan-1"),
            plan_revision: 1,
            plan_step_id: PlanStepId::from_raw("step-1"),
            next_action_sha256: digest('2'),
            workspace_id: WorkspaceId::from_raw("workspace-1"),
            workspace_state_sha256: digest('3'),
            repository_snapshot_id: RepositorySnapshotId::from_raw("snapshot-1"),
            repository_branch: "main".to_owned(),
            repository_map_sha256: digest('4'),
            files: vec![CheckpointFileIdentity {
                object_id: "object-1".to_owned(),
                content_sha256: digest('5'),
                observed_revision: "revision-1".to_owned(),
            }],
            instruction_sha256: digest('6'),
            permission_profile_id: "permission-1".to_owned(),
            permission_profile_sha256: digest('7'),
            policy_id: PolicyId::from_raw("policy-1"),
            policy_sha256: digest('b'),
            model_profile_id: ModelProfileId::from_raw("model-1"),
            model_manifest_sha256: digest('9'),
            model_runtime_sha256: digest('a'),
            evidence_ids: vec![EvidenceId::from_raw("evidence-1")],
            citation_set_sha256: digest('c'),
            blockers: Vec::new(),
            context_packet_sha256: digest('d'),
            action_id: None,
            action_state: None,
            consumed_grant_id: None,
            receipt_id: None,
            receipt_sha256: None,
            ephemeral: false,
            checkpoint_sha256: digest('0'),
        })
        .expect("checkpoint")
    }

    fn continuation() -> RuntimeContinuationState {
        let states = [
            AgentStateKind::Proposal,
            AgentStateKind::Validation,
            AgentStateKind::Approval,
            AgentStateKind::Execution,
            AgentStateKind::Verification,
            AgentStateKind::Checkpoint,
            AgentStateKind::Observation,
        ];
        let mut prior = AgentStateKind::Observation;
        let transitions = states
            .into_iter()
            .enumerate()
            .map(|(index, next)| {
                let transition = AgentStateTransition::new(index as u64 + 2, prior, next);
                prior = next;
                transition
            })
            .collect::<Vec<_>>();
        seal_runtime_continuation_state(RuntimeContinuationState {
            schema_version: CONTRACT_SCHEMA_VERSION,
            request_sha256: digest('a'),
            run_id: RuntimeRunId::from_raw("run-1"),
            session_id: SessionId::from_raw("session-1"),
            task_id: TaskId::from_raw("task-1"),
            event_cursor: RuntimeEventCursor {
                run_id: RuntimeRunId::from_raw("run-1"),
                event_id: RuntimeEventId::from_raw("event-turn-completed-1"),
                sequence: 8,
                event_sha256: digest('b'),
            },
            agent_state: AgentStateKind::Observation,
            agent_state_revision: transitions.len() as u64 + 1,
            state_transitions: transitions,
            turn_count: 1,
            model_call_count: 1,
            tool_call_count: 0,
            context_refresh_count: 1,
            no_progress_turns: 0,
            resources: agentmage_kernel_contracts::RuntimeResourceUsage {
                plan_steps: 1,
                model_calls: 1,
                tool_calls: 0,
                input_bytes: 1,
                output_bytes: 0,
                elapsed_ms: 1,
                peak_memory_bytes: 1,
                disk_bytes: 0,
                process_count: 0,
                event_count: 9,
                event_bytes: 9,
                artifact_count: 0,
                artifact_bytes: 0,
                denial_count: 0,
                parser_failure_count: 0,
                retry_count: 0,
            },
            tool_attempts: Vec::new(),
            tool_results: Vec::new(),
            evidence: Vec::new(),
            receipt_ids: Vec::new(),
            artifacts: Vec::new(),
            continuation_sha256: digest('0'),
        })
        .expect("continuation seals")
    }

    #[test]
    fn manifest_reference_and_event_projection_reconcile_exactly() {
        let manifest = manifest();
        verify_runtime_artifact_manifest(&manifest).expect("manifest verifies");
        let reference = runtime_artifact_ref(&manifest).expect("reference");
        verify_runtime_artifact_ref(&reference, &manifest).expect("reference verifies");
        let payload = runtime_payload_reference(&manifest).expect("event reference");
        assert_eq!(payload.artifact_id, reference.artifact_id);
        assert_eq!(payload.sha256, reference.payload_sha256);
        assert_eq!(payload.byte_size, reference.byte_size);
        assert_eq!(payload.media_type, reference.media_type);
    }

    #[test]
    fn story_21_2_artifact_canaries_require_an_exact_owner_bound_payload_read() {
        let canaries = [
            "SYNTHETIC_SECRET_CANARY_21_2",
            "SYNTHETIC_RESTRICTED_CONTENT_21_2",
            "SYNTHETIC_PROMPT_CANARY_21_2",
            "SYNTHETIC_TOKEN_FRAGMENT_21_2",
            "/synthetic/private/path/canary-21-2",
            "SYNTHETIC_ENV_CANARY_21_2=value",
            "syntheticcredentialcanarydeadbeef",
        ];
        let payload_bytes = canaries.join(" ").into_bytes();
        let maximum_bytes = payload_bytes
            .len()
            .try_into()
            .expect("bounded canary fixture size fits u64");
        let mut candidate = manifest_with_id("runtime-artifact-canary-21-2");
        candidate.payload_sha256 = super::sha256(&payload_bytes);
        candidate.byte_size = maximum_bytes;
        candidate.preview = None;
        let candidate = seal_runtime_artifact_manifest(candidate).expect("canary manifest seals");

        let directory = temporary_directory();
        let path = directory.join("authority.db");
        let mut runtime = runtime_with_run(&path);
        let mut payloads = FakePayloadStore::default();
        let publication = runtime
            .publish_runtime_artifact(&mut payloads, candidate, &mut Cursor::new(&payload_bytes))
            .expect("canary artifact publishes under explicit metadata");
        let payload_reference =
            runtime_payload_reference(&publication.manifest).expect("event reference projects");

        let metadata_bytes = serde_json::to_vec(&(
            &publication.manifest,
            &publication.reference,
            &payload_reference,
        ))
        .expect("artifact metadata projections serialize");
        for canary in &canaries {
            assert!(
                !metadata_bytes
                    .windows(canary.len())
                    .any(|window| window == canary.as_bytes()),
                "path-free artifact metadata retained payload content"
            );
        }

        let read = runtime
            .read_runtime_artifact(
                &payloads,
                &RuntimeArtifactReadRequest {
                    session_id: SessionId::from_raw("session-1"),
                    task_id: TaskId::from_raw("task-1"),
                    policy_sha256: digest('b'),
                    reference: publication.reference.clone(),
                    now_epoch_ms: 2,
                    maximum_bytes,
                },
            )
            .expect("exact owner-bound artifact read succeeds");
        assert_eq!(read, payload_bytes);

        assert_eq!(
            runtime.read_runtime_artifact(
                &payloads,
                &RuntimeArtifactReadRequest {
                    session_id: SessionId::from_raw("session-other"),
                    task_id: TaskId::from_raw("task-1"),
                    policy_sha256: digest('b'),
                    reference: publication.reference,
                    now_epoch_ms: 2,
                    maximum_bytes,
                },
            ),
            Err(DurableAuthorityError::RuntimeArtifact(
                RuntimeArtifactStoreError::NotAuthorized
            ))
        );

        drop(runtime);
        fs::remove_dir_all(directory).expect("cleanup");
    }

    #[test]
    fn manifest_digest_or_reference_drift_fails_closed() {
        let manifest = manifest();
        let mut changed = manifest.clone();
        changed.byte_size += 1;
        assert_eq!(
            verify_runtime_artifact_manifest(&changed),
            Err(RuntimeArtifactError::InvalidManifest)
        );
        let mut reference = runtime_artifact_ref(&manifest).unwrap();
        reference.payload_sha256 = digest('c');
        assert_eq!(
            verify_runtime_artifact_ref(&reference, &manifest),
            Err(RuntimeArtifactError::ReferenceMismatch)
        );
    }

    #[test]
    fn size_media_retention_and_preview_bounds_are_closed() {
        let mut candidate = manifest();
        candidate.byte_size = MAX_RUNTIME_ARTIFACT_BYTES + 1;
        assert_eq!(
            seal_runtime_artifact_manifest(candidate),
            Err(RuntimeArtifactError::InvalidManifest)
        );
        let mut candidate = manifest();
        candidate.media_type = "TEXT/PLAIN".to_owned();
        assert_eq!(
            seal_runtime_artifact_manifest(candidate),
            Err(RuntimeArtifactError::InvalidManifest)
        );
        let mut candidate = manifest();
        candidate.media_type = "application/octet-stream".to_owned();
        candidate.manifest_sha256 = digest('0');
        assert!(seal_runtime_artifact_manifest(candidate).is_ok());
        let mut candidate = manifest();
        candidate.kind = RuntimeArtifactKind::ModelOutput;
        candidate.media_type = "application/octet-stream".to_owned();
        candidate.manifest_sha256 = digest('0');
        assert_eq!(
            seal_runtime_artifact_manifest(candidate),
            Err(RuntimeArtifactError::InvalidManifest)
        );
        let mut candidate = manifest();
        candidate.retention.kind = RuntimeEventRetentionKind::Ephemeral;
        assert_eq!(
            seal_runtime_artifact_manifest(candidate),
            Err(RuntimeArtifactError::InvalidManifest)
        );
        let mut candidate = manifest();
        candidate.preview.as_mut().unwrap().truncated = true;
        assert_eq!(
            seal_runtime_artifact_manifest(candidate),
            Err(RuntimeArtifactError::InvalidManifest)
        );
    }

    #[test]
    fn unknown_versions_and_expired_artifacts_fail_closed() {
        let mut unknown_manifest = manifest();
        unknown_manifest.schema_version += 1;
        assert_eq!(
            verify_runtime_artifact_manifest(&unknown_manifest),
            Err(RuntimeArtifactError::InvalidManifest)
        );
        let mut unknown_reference = runtime_artifact_ref(&manifest()).expect("reference");
        unknown_reference.schema_version += 1;
        assert_eq!(
            verify_runtime_artifact_ref(&unknown_reference, &manifest()),
            Err(RuntimeArtifactError::ReferenceMismatch)
        );
        let mut unknown_continuation = continuation();
        unknown_continuation.schema_version += 1;
        assert_eq!(
            verify_runtime_continuation_state(&unknown_continuation),
            Err(RuntimeArtifactError::InvalidContinuation)
        );

        let directory = temporary_directory();
        let path = directory.join("authority.db");
        let mut runtime = runtime_with_run(&path);
        let mut payloads = FakePayloadStore::default();
        let mut expiring = manifest_with_id("runtime-artifact-expiring-1");
        expiring.retention = RuntimeEventRetention {
            kind: RuntimeEventRetentionKind::UntilExpiration,
            expires_at_epoch_ms: Some(3),
        };
        expiring.manifest_sha256 = digest('0');
        let expiring = seal_runtime_artifact_manifest(expiring).expect("expiring manifest seals");
        let publication = runtime
            .publish_runtime_artifact(&mut payloads, expiring, &mut Cursor::new(b"0123456789"))
            .expect("expiring artifact publishes");
        let read_request = |now_epoch_ms| RuntimeArtifactReadRequest {
            session_id: SessionId::from_raw("session-1"),
            task_id: TaskId::from_raw("task-1"),
            policy_sha256: digest('b'),
            reference: publication.reference.clone(),
            now_epoch_ms,
            maximum_bytes: 10,
        };
        assert_eq!(
            runtime
                .read_runtime_artifact(&payloads, &read_request(2))
                .expect("artifact reads before expiration"),
            b"0123456789"
        );
        assert_eq!(
            runtime.read_runtime_artifact(&payloads, &read_request(3)),
            Err(DurableAuthorityError::RuntimeArtifact(
                RuntimeArtifactStoreError::NotAuthorized
            ))
        );
        let report = runtime
            .reconcile_runtime_artifacts(&mut payloads, 3)
            .expect("expiration reconciles");
        assert_eq!(report.deleted_orphans, 1);
        let expired = runtime
            .runtime_artifact_operator_view(&publication.reference)
            .expect("expired operator view");
        assert_eq!(expired.lifecycle, RuntimeArtifactLifecycleState::Deleted);
        assert_eq!(expired.integrity, RuntimeArtifactIntegrityState::Deleted);
        assert_eq!(expired.cleanup, RuntimeArtifactCleanupState::Completed);
        assert!(payloads.objects.is_empty());
        drop(runtime);
        fs::remove_dir_all(directory).expect("cleanup");
    }

    #[test]
    fn partial_and_identifier_colliding_publications_preserve_canonical_state() {
        let directory = temporary_directory();
        let path = directory.join("authority.db");
        let mut runtime = runtime_with_run(&path);
        let mut payloads = FakePayloadStore::default();

        assert_eq!(
            runtime.publish_runtime_artifact(
                &mut payloads,
                manifest_with_id("runtime-artifact-partial-1"),
                &mut Cursor::new(b"short"),
            ),
            Err(DurableAuthorityError::RuntimeArtifact(
                RuntimeArtifactStoreError::Payload(RuntimeArtifactPayloadError::Invalid)
            ))
        );
        assert!(payloads.objects.is_empty());

        let original = runtime
            .publish_runtime_artifact(
                &mut payloads,
                manifest_with_id("runtime-artifact-collision-1"),
                &mut Cursor::new(b"0123456789"),
            )
            .expect("original artifact publishes");
        let colliding_bytes = b"abcdefghij";
        let mut colliding = original.manifest.clone();
        colliding.payload_sha256 = super::sha256(colliding_bytes);
        colliding.preview = Some(RuntimeArtifactPreview {
            text: "abcdefghij".to_owned(),
            byte_size: 10,
            truncated: false,
            sha256: super::sha256(colliding_bytes),
        });
        colliding.manifest_sha256 = digest('0');
        let colliding = seal_runtime_artifact_manifest(colliding).expect("collision seals");
        assert!(matches!(
            runtime.publish_runtime_artifact(
                &mut payloads,
                colliding,
                &mut Cursor::new(colliding_bytes),
            ),
            Err(DurableAuthorityError::RuntimeArtifact(
                RuntimeArtifactStoreError::Integrity
            ))
        ));
        drop(runtime);

        let mut runtime = DurableAuthorityRuntime::open(&path, &observation(), &mut TestKey, 2)
            .expect("canonical authority reopens");
        let report = runtime
            .reconcile_runtime_artifacts(&mut payloads, 2)
            .expect("colliding orphan reconciles");
        assert_eq!(report.deleted_orphans, 1);
        assert_eq!(payloads.objects.len(), 1);
        assert_eq!(
            runtime
                .read_runtime_artifact(
                    &payloads,
                    &RuntimeArtifactReadRequest {
                        session_id: SessionId::from_raw("session-1"),
                        task_id: TaskId::from_raw("task-1"),
                        policy_sha256: digest('b'),
                        reference: original.reference,
                        now_epoch_ms: 2,
                        maximum_bytes: 10,
                    },
                )
                .expect("original remains readable"),
            b"0123456789"
        );
        drop(runtime);
        fs::remove_dir_all(directory).expect("cleanup");
    }

    #[test]
    fn publication_rejects_mismatched_producer_authority_before_staging() {
        let directory = temporary_directory();
        let path = directory.join("authority.db");
        let mut runtime = runtime_with_run(&path);
        let mut payloads = FakePayloadStore::default();
        let valid = manifest_with_id("runtime-artifact-owner-bound-1");

        let mut candidates = Vec::new();
        let mut changed = valid.clone();
        changed.producer_run_id = RuntimeRunId::from_raw("run-other");
        candidates.push(changed);
        let mut changed = valid.clone();
        changed.session_id = SessionId::from_raw("session-other");
        candidates.push(changed);
        let mut changed = valid.clone();
        changed.task_id = TaskId::from_raw("task-other");
        candidates.push(changed);
        let mut changed = valid.clone();
        changed.policy_id = PolicyId::from_raw("policy-other");
        candidates.push(changed);

        for mut candidate in candidates {
            candidate.manifest_sha256 = digest('0');
            let candidate =
                seal_runtime_artifact_manifest(candidate).expect("candidate remains well formed");
            assert_eq!(
                runtime.publish_runtime_artifact(
                    &mut payloads,
                    candidate,
                    &mut Cursor::new(b"0123456789"),
                ),
                Err(DurableAuthorityError::RuntimeArtifact(
                    RuntimeArtifactStoreError::NotAuthorized
                ))
            );
            assert!(payloads.objects.is_empty());
        }

        runtime
            .publish_runtime_artifact(&mut payloads, valid, &mut Cursor::new(b"0123456789"))
            .expect("the exact producer owner still publishes");
        assert_eq!(payloads.objects.len(), 1);
        drop(runtime);
        fs::remove_dir_all(directory).expect("cleanup");
    }

    #[test]
    fn resume_binding_binds_cursor_and_sorted_exact_artifact_set() {
        let reference = runtime_artifact_ref(&manifest()).unwrap();
        let binding = seal_runtime_resume_binding(RuntimeResumeBinding {
            schema_version: CONTRACT_SCHEMA_VERSION,
            checkpoint_id: SessionCheckpointId::from_raw("checkpoint-1"),
            checkpoint_sha256: digest('c'),
            session_id: SessionId::from_raw("session-1"),
            task_id: TaskId::from_raw("task-1"),
            run_id: RuntimeRunId::from_raw("run-1"),
            event_cursor: RuntimeEventCursor {
                run_id: RuntimeRunId::from_raw("run-1"),
                event_id: RuntimeEventId::from_raw("event-5"),
                sequence: 5,
                event_sha256: digest('d'),
            },
            artifacts: vec![reference],
            binding_sha256: digest('0'),
        })
        .expect("binding seals");
        verify_runtime_resume_binding(&binding).expect("binding verifies");

        let mut changed = binding.clone();
        changed.event_cursor.sequence += 1;
        assert_eq!(
            verify_runtime_resume_binding(&changed),
            Err(RuntimeArtifactError::DigestMismatch)
        );
    }

    #[test]
    fn resume_binding_rejects_run_drift_and_duplicate_references() {
        let reference = runtime_artifact_ref(&manifest()).unwrap();
        let base = RuntimeResumeBinding {
            schema_version: CONTRACT_SCHEMA_VERSION,
            checkpoint_id: SessionCheckpointId::from_raw("checkpoint-1"),
            checkpoint_sha256: digest('c'),
            session_id: SessionId::from_raw("session-1"),
            task_id: TaskId::from_raw("task-1"),
            run_id: RuntimeRunId::from_raw("run-2"),
            event_cursor: RuntimeEventCursor {
                run_id: RuntimeRunId::from_raw("run-1"),
                event_id: RuntimeEventId::from_raw("event-5"),
                sequence: 5,
                event_sha256: digest('d'),
            },
            artifacts: vec![reference.clone(), reference],
            binding_sha256: digest('0'),
        };
        assert_eq!(
            seal_runtime_resume_binding(base),
            Err(RuntimeArtifactError::InvalidResumeBinding)
        );
    }

    #[test]
    fn continuation_round_trips_one_exact_safe_boundary() {
        let continuation = continuation();
        verify_runtime_continuation_state(&continuation).expect("continuation verifies");

        let encoded = encode_runtime_continuation_state(&continuation)
            .expect("continuation serializes canonically");
        let decoded = decode_runtime_continuation_state(&encoded)
            .expect("continuation deserializes under its artifact contract");
        assert_eq!(decoded, continuation);
        assert_eq!(decoded.agent_state, AgentStateKind::Observation);
        assert_eq!(decoded.agent_state_revision, 8);

        let mut noncanonical = encoded;
        noncanonical.push(b' ');
        assert_eq!(
            decode_runtime_continuation_state(&noncanonical),
            Err(RuntimeArtifactError::InvalidContinuation)
        );
    }

    #[test]
    fn continuation_codec_supports_verified_payloads_above_shared_json_limit() {
        let mut continuation = continuation();
        let output_bytes = vec![b'x'; agentmage_kernel_contracts::MAX_CONTRACT_JSON_BYTES + 1];
        let tool_call_id = agentmage_kernel_contracts::ToolCallId::from_raw("call-large-1");
        continuation.tool_call_count = 1;
        continuation.resources.tool_calls = 1;
        continuation.resources.output_bytes = output_bytes
            .len()
            .try_into()
            .expect("fixture output size fits u64");
        continuation.tool_attempts = vec![agentmage_kernel_contracts::RuntimeToolAttemptState {
            schema_version: CONTRACT_SCHEMA_VERSION,
            sequence: 1,
            tool_call_id: tool_call_id.clone(),
            semantic_sha256: digest('e'),
            occurrence: 1,
            call_depth: 0,
        }];
        continuation.tool_results = vec![agentmage_kernel_contracts::ToolResult {
            schema_version: CONTRACT_SCHEMA_VERSION,
            tool_call_id,
            correlation_id: CorrelationId::from_raw("correlation-large-1"),
            outcome: agentmage_kernel_contracts::OperationOutcome::Succeeded,
            output: Some(agentmage_kernel_contracts::ContractPayload {
                schema: agentmage_kernel_contracts::SchemaReference {
                    schema_id: agentmage_kernel_contracts::SchemaId::from_raw("large.output"),
                    schema_version: 1,
                    schema_sha256: digest('f'),
                },
                media_type: "application/octet-stream".to_owned(),
                sha256: super::sha256(&output_bytes),
                bytes: output_bytes,
            }),
            validation_issues: Vec::new(),
            evidence: Vec::new(),
            error: None,
            elapsed_ms: 1,
            state_change: agentmage_kernel_contracts::StateChange::NotChanged,
        }];
        continuation.receipt_ids = vec![agentmage_kernel_contracts::ReceiptId::from_raw(
            "receipt-large-1",
        )];
        continuation.continuation_sha256 = digest('0');
        let continuation =
            seal_runtime_continuation_state(continuation).expect("large continuation seals");
        let encoded = encode_runtime_continuation_state(&continuation)
            .expect("artifact codec exceeds shared JSON limit safely");
        assert!(encoded.len() > agentmage_kernel_contracts::MAX_CONTRACT_JSON_BYTES);
        assert_eq!(
            decode_runtime_continuation_state(&encoded).expect("large continuation decodes"),
            continuation
        );
    }

    #[test]
    fn continuation_rejects_unsafe_or_inconsistent_state() {
        let mut unsafe_state = continuation();
        unsafe_state.agent_state = AgentStateKind::Checkpoint;
        assert_eq!(
            seal_runtime_continuation_state(unsafe_state),
            Err(RuntimeArtifactError::InvalidContinuation)
        );

        let mut broken_history = continuation();
        broken_history.state_transitions[3].from = AgentStateKind::Validation;
        assert_eq!(
            seal_runtime_continuation_state(broken_history),
            Err(RuntimeArtifactError::InvalidContinuation)
        );

        let mut broken_counts = continuation();
        broken_counts.model_call_count += 1;
        assert_eq!(
            seal_runtime_continuation_state(broken_counts),
            Err(RuntimeArtifactError::InvalidContinuation)
        );
    }

    #[test]
    fn continuation_digest_binds_cursor_and_collected_state() {
        let mut changed = continuation();
        changed.event_cursor.sequence += 1;
        changed.resources.event_count += 1;
        changed.resources.event_bytes += 1;
        assert_eq!(
            verify_runtime_continuation_state(&changed),
            Err(RuntimeArtifactError::DigestMismatch)
        );

        let mut changed = continuation();
        changed.no_progress_turns = 1;
        assert_eq!(
            verify_runtime_continuation_state(&changed),
            Err(RuntimeArtifactError::DigestMismatch)
        );
    }

    #[test]
    fn story_50_2_artifact_pages_are_owner_bound_bounded_and_exact() {
        let directory = temporary_directory();
        let path = directory.join("authority.db");
        let mut runtime = runtime_with_run(&path);
        let mut payloads = FakePayloadStore::default();
        let bytes = b"0123456789";
        let publication = runtime
            .publish_runtime_artifact(
                &mut payloads,
                manifest_with_id("runtime-artifact-page-1"),
                &mut Cursor::new(bytes),
            )
            .expect("artifact publishes");
        let request = |offset, maximum_bytes| RuntimeArtifactPageRequest {
            session_id: SessionId::from_raw("session-1"),
            task_id: TaskId::from_raw("task-1"),
            policy_sha256: digest('b'),
            reference: publication.reference.clone(),
            now_epoch_ms: 1,
            offset,
            maximum_bytes,
        };

        let first = runtime
            .read_runtime_artifact_page(&payloads, &request(0, 4))
            .expect("first page reads");
        assert_eq!(first.bytes, b"0123");
        assert_eq!(first.page_sha256, super::sha256(b"0123"));
        assert_eq!(first.next_offset, Some(4));
        assert!(!first.complete);
        let second = runtime
            .read_runtime_artifact_page(&payloads, &request(4, 4))
            .expect("second page reads");
        let third = runtime
            .read_runtime_artifact_page(&payloads, &request(8, 4))
            .expect("terminal page reads");
        assert_eq!(second.bytes, b"4567");
        assert_eq!(third.bytes, b"89");
        assert_eq!(third.next_offset, None);
        assert!(third.complete);

        assert_eq!(
            runtime.read_runtime_artifact_page(&payloads, &request(10, 4)),
            Err(DurableAuthorityError::RuntimeArtifact(
                RuntimeArtifactStoreError::Payload(RuntimeArtifactPayloadError::ResourceLimit)
            ))
        );
        assert_eq!(
            runtime.read_runtime_artifact_page(&payloads, &request(0, 4_097)),
            Err(DurableAuthorityError::RuntimeArtifact(
                RuntimeArtifactStoreError::Payload(RuntimeArtifactPayloadError::ResourceLimit)
            ))
        );

        let mut wrong_owner = request(0, 4);
        wrong_owner.session_id = SessionId::from_raw("session-other");
        assert_eq!(
            runtime.read_runtime_artifact_page(&payloads, &wrong_owner),
            Err(DurableAuthorityError::RuntimeArtifact(
                RuntimeArtifactStoreError::NotAuthorized
            ))
        );

        payloads
            .objects
            .get_mut(&publication.reference.payload_sha256)
            .expect("retained payload exists")[0] = b'x';
        assert_eq!(
            runtime.read_runtime_artifact_page(&payloads, &request(0, 4)),
            Err(DurableAuthorityError::RuntimeArtifact(
                RuntimeArtifactStoreError::Payload(RuntimeArtifactPayloadError::Corrupt)
            ))
        );
        drop(runtime);
        fs::remove_dir_all(directory).expect("cleanup");
    }

    #[test]
    fn publication_deduplicates_without_broadening_owner_or_reference_state() {
        let directory = temporary_directory();
        let path = directory.join("authority.db");
        let mut runtime = runtime_with_run(&path);
        let mut payloads = FakePayloadStore::default();
        let bytes = b"0123456789";
        let first = runtime
            .publish_runtime_artifact(
                &mut payloads,
                manifest_with_id("runtime-artifact-1"),
                &mut Cursor::new(bytes),
            )
            .expect("first artifact publishes");
        assert!(!first.payload_deduplicated);
        assert_eq!(
            runtime
                .read_runtime_artifact(
                    &payloads,
                    &RuntimeArtifactReadRequest {
                        session_id: SessionId::from_raw("session-1"),
                        task_id: TaskId::from_raw("task-1"),
                        policy_sha256: digest('b'),
                        reference: first.reference.clone(),
                        now_epoch_ms: 1,
                        maximum_bytes: 10,
                    },
                )
                .expect("owner reads"),
            bytes
        );
        assert_eq!(
            runtime.read_runtime_artifact(
                &payloads,
                &RuntimeArtifactReadRequest {
                    session_id: SessionId::from_raw("session-other"),
                    task_id: TaskId::from_raw("task-1"),
                    policy_sha256: digest('b'),
                    reference: first.reference.clone(),
                    now_epoch_ms: 1,
                    maximum_bytes: 10,
                },
            ),
            Err(DurableAuthorityError::RuntimeArtifact(
                RuntimeArtifactStoreError::NotAuthorized
            ))
        );

        let duplicate = runtime
            .publish_runtime_artifact(
                &mut payloads,
                first.manifest.clone(),
                &mut Cursor::new(bytes),
            )
            .expect("exact duplicate is idempotent");
        assert!(duplicate.payload_deduplicated);
        assert_eq!(
            runtime
                .runtime_artifact_state(&first.reference)
                .expect("state")
                .revision,
            1
        );
        let second = runtime
            .publish_runtime_artifact(
                &mut payloads,
                manifest_with_id("runtime-artifact-2"),
                &mut Cursor::new(bytes),
            )
            .expect("second logical reference deduplicates bytes");
        assert!(second.payload_deduplicated);
        assert_eq!(payloads.objects.len(), 1);

        runtime
            .release_runtime_artifact(
                &SessionId::from_raw("session-1"),
                &TaskId::from_raw("task-1"),
                &digest('b'),
                &first.reference,
                2,
            )
            .expect("first reference releases");
        let retained = runtime
            .reconcile_runtime_artifacts(&mut payloads, 3)
            .expect("shared payload remains");
        assert_eq!(retained.verified_payloads, 1);
        assert_eq!(payloads.objects.len(), 1);

        runtime
            .release_runtime_artifact(
                &SessionId::from_raw("session-1"),
                &TaskId::from_raw("task-1"),
                &digest('b'),
                &second.reference,
                4,
            )
            .expect("second reference releases");
        let collected = runtime
            .reconcile_runtime_artifacts(&mut payloads, 5)
            .expect("unreferenced payload collects");
        assert_eq!(collected.deleted_orphans, 1);
        assert!(payloads.objects.is_empty());
        assert_eq!(
            runtime
                .runtime_artifact_state(&first.reference)
                .expect("deleted first")
                .lifecycle,
            RuntimeArtifactLifecycleState::Deleted
        );
        drop(runtime);
        fs::remove_dir_all(directory).expect("cleanup");
    }

    #[test]
    fn operator_view_is_path_free_complete_and_tracks_cleanup_state() {
        let directory = temporary_directory();
        let path = directory.join("authority.db");
        let mut runtime = runtime_with_run(&path);
        let mut payloads = FakePayloadStore::default();
        let publication = runtime
            .publish_runtime_artifact(
                &mut payloads,
                manifest_with_id("runtime-artifact-operator-1"),
                &mut Cursor::new(b"0123456789"),
            )
            .expect("artifact publishes");

        let active = runtime
            .runtime_artifact_operator_view(&publication.reference)
            .expect("operator view");
        assert_eq!(active.reference, publication.reference);
        assert_eq!(active.kind, RuntimeArtifactKind::TestLog);
        assert_eq!(active.sensitivity, ContextSensitivity::Private);
        assert_eq!(active.producer_run_id, RuntimeRunId::from_raw("run-1"));
        assert_eq!(active.lifecycle, RuntimeArtifactLifecycleState::Active);
        assert_eq!(active.integrity, RuntimeArtifactIntegrityState::Verified);
        assert_eq!(active.lifecycle_revision, 1);
        assert_eq!(active.checkpoint_reference_count, 0);
        assert_eq!(active.shared_active_reference_count, 1);
        assert_eq!(active.cleanup, RuntimeArtifactCleanupState::Retained);
        let serialized = serde_json::to_vec(&active).expect("operator view serializes");
        assert!(!serialized.windows(10).any(|window| window == b"0123456789"));
        assert!(
            !serialized
                .windows(10)
                .any(|window| window == b"authority.db")
        );

        runtime
            .release_runtime_artifact(
                &SessionId::from_raw("session-1"),
                &TaskId::from_raw("task-1"),
                &digest('b'),
                &publication.reference,
                2,
            )
            .expect("unbound artifact releases");
        let eligible = runtime
            .runtime_artifact_operator_view(&publication.reference)
            .expect("released view");
        assert_eq!(eligible.cleanup, RuntimeArtifactCleanupState::Eligible);
        assert_eq!(eligible.shared_active_reference_count, 0);

        runtime
            .reconcile_runtime_artifacts(&mut payloads, 3)
            .expect("eligible payload collects");
        let completed = runtime
            .runtime_artifact_operator_view(&publication.reference)
            .expect("deleted view");
        assert_eq!(completed.cleanup, RuntimeArtifactCleanupState::Completed);
        assert_eq!(completed.lifecycle, RuntimeArtifactLifecycleState::Deleted);
        assert_eq!(completed.integrity, RuntimeArtifactIntegrityState::Deleted);
        assert!(payloads.objects.is_empty());

        drop(runtime);
        fs::remove_dir_all(directory).expect("cleanup");
    }

    #[test]
    fn current_checkpoint_reference_prevents_release_and_collection() {
        let directory = temporary_directory();
        let path = directory.join("authority.db");
        let mut runtime = runtime_with_run(&path);
        let mut payloads = FakePayloadStore::default();
        let publication = runtime
            .publish_runtime_artifact(
                &mut payloads,
                manifest_with_id("runtime-artifact-checkpoint-root-1"),
                &mut Cursor::new(b"0123456789"),
            )
            .expect("artifact publishes");
        let cursor = runtime
            .runtime_event_cursor(&RuntimeRunId::from_raw("run-1"))
            .expect("cursor loads")
            .expect("cursor exists");
        let checkpoint = checkpoint();
        let binding = seal_runtime_resume_binding(RuntimeResumeBinding {
            schema_version: CONTRACT_SCHEMA_VERSION,
            checkpoint_id: checkpoint.checkpoint_id.clone(),
            checkpoint_sha256: checkpoint.checkpoint_sha256.clone(),
            session_id: checkpoint.session_id.clone(),
            task_id: checkpoint.task_id.clone(),
            run_id: RuntimeRunId::from_raw("run-1"),
            event_cursor: cursor,
            artifacts: vec![publication.reference.clone()],
            binding_sha256: digest('0'),
        })
        .expect("binding");
        runtime
            .checkpoint_runtime_session(&checkpoint, &binding)
            .expect("checkpoint binds artifact");

        let retained = runtime
            .runtime_artifact_operator_view(&publication.reference)
            .expect("operator view");
        assert_eq!(retained.checkpoint_reference_count, 1);
        assert_eq!(retained.cleanup, RuntimeArtifactCleanupState::Retained);
        assert_eq!(
            runtime.release_runtime_artifact(
                &SessionId::from_raw("session-1"),
                &TaskId::from_raw("task-1"),
                &digest('b'),
                &publication.reference,
                2,
            ),
            Err(DurableAuthorityError::RuntimeArtifact(
                RuntimeArtifactStoreError::NotAuthorized
            ))
        );
        let report = runtime
            .reconcile_runtime_artifacts(&mut payloads, 3)
            .expect("checkpoint-rooted payload verifies");
        assert_eq!(report.verified_payloads, 1);
        assert!(
            payloads
                .objects
                .contains_key(&publication.reference.payload_sha256)
        );

        drop(runtime);
        fs::remove_dir_all(directory).expect("cleanup");
    }

    #[test]
    fn missing_and_corrupt_payloads_quarantine_every_active_reference() {
        for corrupt in [false, true] {
            let directory = temporary_directory();
            let path = directory.join("authority.db");
            let mut runtime = runtime_with_run(&path);
            let mut payloads = FakePayloadStore::default();
            let publication = runtime
                .publish_runtime_artifact(
                    &mut payloads,
                    manifest_with_id("runtime-artifact-1"),
                    &mut Cursor::new(b"0123456789"),
                )
                .expect("artifact publishes");
            if corrupt {
                payloads
                    .objects
                    .get_mut(&publication.reference.payload_sha256)
                    .expect("retained object")[0] = b'x';
            } else {
                payloads.objects.clear();
            }
            let report = runtime
                .reconcile_runtime_artifacts(&mut payloads, 2)
                .expect("integrity loss reconciles visibly");
            assert_eq!(report.quarantined_payloads, 1);
            let state = runtime
                .runtime_artifact_state(&publication.reference)
                .expect("quarantined state");
            assert_eq!(state.lifecycle, RuntimeArtifactLifecycleState::Quarantined);
            assert_eq!(
                state.integrity,
                if corrupt {
                    RuntimeArtifactIntegrityState::Corrupt
                } else {
                    RuntimeArtifactIntegrityState::Missing
                }
            );
            assert_eq!(
                runtime.read_runtime_artifact(
                    &payloads,
                    &RuntimeArtifactReadRequest {
                        session_id: SessionId::from_raw("session-1"),
                        task_id: TaskId::from_raw("task-1"),
                        policy_sha256: digest('b'),
                        reference: publication.reference.clone(),
                        now_epoch_ms: 2,
                        maximum_bytes: 10,
                    },
                ),
                Err(DurableAuthorityError::RuntimeArtifact(
                    RuntimeArtifactStoreError::NotAuthorized
                ))
            );
            drop(runtime);
            fs::remove_dir_all(directory).expect("cleanup");
        }
    }

    #[test]
    fn corrupt_unreferenced_inventory_is_quarantined_without_trusting_size() {
        let directory = temporary_directory();
        let path = directory.join("authority.db");
        let mut runtime = runtime_with_run(&path);
        let mut payloads = FakePayloadStore::default();
        let object_name = digest('d');
        payloads
            .objects
            .insert(object_name.clone(), b"not-the-addressed-payload".to_vec());

        let report = runtime
            .reconcile_runtime_artifacts(&mut payloads, 2)
            .expect("corrupt orphan reconciles");
        assert_eq!(report.quarantined_payloads, 1);
        assert_eq!(report.deleted_orphans, 0);
        assert!(payloads.objects.is_empty());
        assert!(payloads.quarantined.contains_key(&object_name));

        drop(runtime);
        fs::remove_dir_all(directory).expect("cleanup");
    }

    #[test]
    fn metadata_projection_tampering_blocks_durable_restart() {
        let directory = temporary_directory();
        let path = directory.join("authority.db");
        let mut runtime = runtime_with_run(&path);
        runtime
            .publish_runtime_artifact(
                &mut FakePayloadStore::default(),
                manifest_with_id("runtime-artifact-1"),
                &mut Cursor::new(b"0123456789"),
            )
            .expect("artifact publishes");
        drop(runtime);
        let store = OperationalStore::open(&path, &observation(), &mut TestKey)
            .expect("store opens before authority verification");
        store
            .connection
            .execute(
                "UPDATE runtime_artifacts SET media_type = 'application/json'
                 WHERE artifact_id = 'runtime-artifact-1'",
                [],
            )
            .expect("tamper projection");
        drop(store);
        assert!(DurableAuthorityRuntime::open(&path, &observation(), &mut TestKey, 3).is_err());
        fs::remove_dir_all(directory).expect("cleanup");
    }

    #[test]
    fn checkpoint_cursor_and_artifact_set_publish_atomically_and_reopen() {
        let directory = temporary_directory();
        let path = directory.join("authority.db");
        let mut runtime = runtime_with_run(&path);
        let mut payloads = FakePayloadStore::default();
        let publication = runtime
            .publish_runtime_artifact(
                &mut payloads,
                manifest_with_id("runtime-artifact-1"),
                &mut Cursor::new(b"0123456789"),
            )
            .expect("artifact publishes");
        let cursor = runtime
            .runtime_event_cursor(&RuntimeRunId::from_raw("run-1"))
            .expect("cursor loads")
            .expect("cursor exists");
        let checkpoint = checkpoint();
        let binding = seal_runtime_resume_binding(RuntimeResumeBinding {
            schema_version: CONTRACT_SCHEMA_VERSION,
            checkpoint_id: checkpoint.checkpoint_id.clone(),
            checkpoint_sha256: checkpoint.checkpoint_sha256.clone(),
            session_id: checkpoint.session_id.clone(),
            task_id: checkpoint.task_id.clone(),
            run_id: RuntimeRunId::from_raw("run-1"),
            event_cursor: cursor,
            artifacts: vec![publication.reference],
            binding_sha256: digest('0'),
        })
        .expect("binding");
        runtime
            .checkpoint_runtime_session(&checkpoint, &binding)
            .expect("checkpoint and binding commit");
        assert_eq!(
            runtime
                .current_runtime_resume_binding()
                .expect("current binding"),
            Some(binding.clone())
        );
        drop(runtime);

        let reopened = DurableAuthorityRuntime::open(&path, &observation(), &mut TestKey, 2)
            .expect("verified runtime reopens");
        assert_eq!(
            reopened
                .current_runtime_resume_binding()
                .expect("reopened binding"),
            Some(binding)
        );
        drop(reopened);
        fs::remove_dir_all(directory).expect("cleanup");
    }

    #[test]
    fn failed_resume_binding_rolls_back_the_checkpoint_and_requires_reopen() {
        let directory = temporary_directory();
        let path = directory.join("authority.db");
        let mut runtime = runtime_with_run(&path);
        let publication = runtime
            .publish_runtime_artifact(
                &mut FakePayloadStore::default(),
                manifest_with_id("runtime-artifact-1"),
                &mut Cursor::new(b"0123456789"),
            )
            .expect("artifact publishes");
        let cursor = runtime
            .runtime_event_cursor(&RuntimeRunId::from_raw("run-1"))
            .expect("cursor loads")
            .expect("cursor exists");
        let checkpoint = checkpoint();
        let binding = seal_runtime_resume_binding(RuntimeResumeBinding {
            schema_version: CONTRACT_SCHEMA_VERSION,
            checkpoint_id: checkpoint.checkpoint_id.clone(),
            checkpoint_sha256: checkpoint.checkpoint_sha256.clone(),
            session_id: checkpoint.session_id.clone(),
            task_id: checkpoint.task_id.clone(),
            run_id: RuntimeRunId::from_raw("run-1"),
            event_cursor: cursor,
            artifacts: vec![publication.reference],
            binding_sha256: digest('0'),
        })
        .expect("binding");
        drop(runtime);

        let store = OperationalStore::open(&path, &observation(), &mut TestKey)
            .expect("store opens for failure injection");
        store
            .connection
            .execute_batch(
                "CREATE TRIGGER reject_runtime_resume_binding
                 BEFORE INSERT ON runtime_resume_bindings
                 BEGIN SELECT RAISE(ABORT, 'synthetic resume failure'); END;",
            )
            .expect("failure trigger");
        drop(store);
        let mut runtime = DurableAuthorityRuntime::open(&path, &observation(), &mut TestKey, 2)
            .expect("runtime reopens before checkpoint");
        assert!(matches!(
            runtime
                .checkpoint_runtime_session(&checkpoint, &binding)
                .expect_err("checkpoint transaction fails"),
            DurableAuthorityError::Store(_)
        ));
        drop(runtime);

        let store = OperationalStore::open(&path, &observation(), &mut TestKey)
            .expect("canonical store reopens after rollback");
        let counts = store
            .connection
            .query_row(
                "SELECT
                    (SELECT COUNT(*) FROM session_checkpoints),
                    (SELECT COUNT(*) FROM runtime_resume_bindings),
                    (SELECT COUNT(*) FROM runtime_resume_artifacts),
                    (SELECT generation FROM store_metadata WHERE singleton = 1)",
                [],
                |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        row.get::<_, i64>(1)?,
                        row.get::<_, i64>(2)?,
                        row.get::<_, i64>(3)?,
                    ))
                },
            )
            .expect("rollback counts");
        assert_eq!(counts, (0, 0, 0, 0));
        drop(store);
        fs::remove_dir_all(directory).expect("cleanup");
    }
}
