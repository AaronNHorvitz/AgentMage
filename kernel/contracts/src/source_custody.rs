//! Logical source-artifact ownership and retention over the single runtime payload store.
//!
//! A source artifact is admitted once and its bytes are retained exactly once, by content
//! address, in the existing encrypted runtime artifact store. This module adds only the
//! *logical* claim on that immutable payload: which single scope owns the reference, how long
//! the reference must survive, and what blocks its release or collection. It introduces no
//! payload location, no second physical store, and no additional `RuntimeArtifactKind` variant;
//! source retention reuses the existing families listed in [`SOURCE_BACKING_ARTIFACT_KINDS`].

use crate::{
    ConversationId, PolicyId, RuntimeArtifactIntegrityState, RuntimeArtifactKind,
    RuntimeArtifactLifecycleState, RuntimeArtifactManifest, RuntimeArtifactRef,
    RuntimeEventRetentionKind, RuntimeRunId, SessionId, SourceArtifactId, SourceCustodyId, TaskId,
};

/// Stable identity of the single physical store that retains every source-artifact payload.
///
/// A custody record naming any other store is rejected, so source retention can never introduce
/// a second content-addressed backend.
pub const SOURCE_CUSTODY_STORE_ID: &str = "runtime-artifact-store";

/// Existing artifact families admitted as source-artifact payload backing.
///
/// Captured file, paste, URI, directory, and archive bytes reuse
/// `RuntimeArtifactKind::GeneratedFile`; captured tool output reuses
/// `RuntimeArtifactKind::StandardOutput`; captured model output reuses
/// `RuntimeArtifactKind::ModelOutput`. No source-specific variant exists.
pub const SOURCE_BACKING_ARTIFACT_KINDS: [RuntimeArtifactKind; 3] = [
    RuntimeArtifactKind::StandardOutput,
    RuntimeArtifactKind::GeneratedFile,
    RuntimeArtifactKind::ModelOutput,
];

/// Maximum UTF-8 bytes admitted for one content-free custody reason code.
pub const MAX_SOURCE_CUSTODY_REASON_CODE_BYTES: usize = 128;

/// Closed logical owner scope for one retained source artifact.
///
/// Exactly one scope owns a claim. A record that names more than one owning identity is
/// rejected, so no second ownership authority can appear for the same payload.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceCustodyOwnerScope {
    /// The owning local session retains the reference.
    Session,
    /// One exact task retains the reference.
    Task,
    /// One exact runtime run retains the reference.
    Run,
    /// One exact persisted conversation retains the reference.
    Conversation,
}

/// Closed reason that blocks release or collection of a retained source artifact.
///
/// A hold is never a substitute for a lifecycle state: `Checkpoint` and `User` belong only to an
/// active claim, `Review` belongs only to a quarantined claim, and a released or deleted claim
/// carries no hold at all.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceCustodyHold {
    /// No hold applies and ordinary retention governs the reference.
    None,
    /// The current durable checkpoint names this exact active reference.
    Checkpoint,
    /// The user explicitly asked to keep this active reference.
    User,
    /// Unresolved integrity or policy review must complete before disposal.
    Review,
}

/// Content-free disposition derived from ownership, retention, holds, and payload state.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceCustodyDisposition {
    /// The logical reference remains current and may be opened under policy.
    Retain,
    /// The claim ended and the reference may be released without collecting the payload.
    ReleaseEligible,
    /// No active logical owner remains and the immutable payload may be collected.
    DeleteEligible,
    /// Integrity loss or unresolved review must be answered before any disposal.
    Blocked,
    /// Canonical metadata already records completed payload deletion.
    Completed,
}

/// Stable fail-closed source-custody failure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceCustodyError {
    /// The record does not carry the supported contract schema version.
    SchemaVersionUnsupported,
    /// The record names a store other than the single canonical payload store.
    ForeignStore,
    /// The backing artifact family is outside the closed source-backing set.
    UnsupportedBackingKind,
    /// The record does not name exactly one owning identity for its declared scope.
    OwnerAmbiguous,
    /// Retention class, expiration, or durability is outside the closed contract.
    RetentionInvalid,
    /// Lifecycle, integrity, hold, revision, or owner accounting is contradictory.
    LifecycleInvalid,
    /// An identity, digest, reason code, or time field is malformed or empty.
    FieldMalformed,
    /// The claim does not bind the exact immutable payload manifest.
    ManifestMismatch,
}

impl SourceCustodyError {
    /// Returns one stable content-free diagnostic code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::SchemaVersionUnsupported => "source.custody.schema_version_unsupported",
            Self::ForeignStore => "source.custody.foreign_store",
            Self::UnsupportedBackingKind => "source.custody.backing_kind_unsupported",
            Self::OwnerAmbiguous => "source.custody.owner_ambiguous",
            Self::RetentionInvalid => "source.custody.retention_invalid",
            Self::LifecycleInvalid => "source.custody.lifecycle_invalid",
            Self::FieldMalformed => "source.custody.field_malformed",
            Self::ManifestMismatch => "source.custody.manifest_mismatch",
        }
    }
}

/// Exact retention assigned to one logical source-artifact custody claim.
///
/// This is the claim's own retention, not the producing manifest's. Several claims may name the
/// same immutable payload with different retention, and the payload survives while any claim
/// does. Ephemeral retention is not a durable custody class and is rejected.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceCustodyRetention {
    /// Closed durable retention class.
    pub kind: RuntimeEventRetentionKind,
    /// Exclusive expiration for `UntilExpiration`; absent for every other class.
    #[serde(deserialize_with = "crate::serialization::deserialize_required_option")]
    pub expires_at_epoch_ms: Option<u64>,
}

/// One logical ownership and retention claim on one immutable content-addressed payload.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceArtifactCustody {
    /// Contract schema version.
    pub schema_version: u16,
    /// Stable identity of this custody claim.
    pub custody_id: SourceCustodyId,
    /// Admitted logical source artifact this claim retains.
    pub source_artifact_id: SourceArtifactId,
    /// Physical store identity; must equal [`SOURCE_CUSTODY_STORE_ID`].
    pub store_id: String,
    /// Path-free reference to the exact immutable payload retained by that store.
    pub payload: RuntimeArtifactRef,
    /// Backing artifact family; must equal the bound manifest and remain inside
    /// [`SOURCE_BACKING_ARTIFACT_KINDS`].
    pub payload_kind: RuntimeArtifactKind,
    /// Closed logical owner scope.
    pub owner_scope: SourceCustodyOwnerScope,
    /// Owning local session; knowledge of this identity grants no access by itself.
    pub session_id: SessionId,
    /// Owning task; present only for task scope.
    #[serde(deserialize_with = "crate::serialization::deserialize_required_option")]
    pub task_id: Option<TaskId>,
    /// Owning runtime run; present only for run scope.
    #[serde(deserialize_with = "crate::serialization::deserialize_required_option")]
    pub run_id: Option<RuntimeRunId>,
    /// Owning persisted conversation; present only for conversation scope.
    #[serde(deserialize_with = "crate::serialization::deserialize_required_option")]
    pub conversation_id: Option<ConversationId>,
    /// Exact retention assigned to this claim.
    pub retention: SourceCustodyRetention,
    /// Closed reason that blocks release or collection.
    pub hold: SourceCustodyHold,
    /// Governing deterministic policy identity.
    pub policy_id: PolicyId,
    /// Digest of the exact governing policy revision.
    pub policy_sha256: String,
    /// Trusted admission time in Unix epoch milliseconds.
    pub admitted_at_epoch_ms: u64,
    /// Current metadata lifecycle state of this logical reference.
    pub lifecycle: RuntimeArtifactLifecycleState,
    /// Current payload integrity state observed for the bound content address.
    pub integrity: RuntimeArtifactIntegrityState,
    /// Monotonic one-based lifecycle revision.
    pub lifecycle_revision: u64,
    /// Stable content-free reason for the current state.
    pub reason_code: String,
    /// Last trusted lifecycle-transition time.
    pub updated_at_epoch_ms: u64,
    /// Active logical claims naming this exact content address, including this one while active.
    pub active_owner_count: u32,
    /// Digest of this canonical record with this field set to all zeroes.
    pub custody_sha256: String,
}

fn is_lowercase_sha256(value: &str) -> bool {
    let hex = |byte: u8| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte);
    value.len() == 64 && value.bytes().all(hex)
}

fn owner_identities_match_scope(record: &SourceArtifactCustody) -> bool {
    let task = record.task_id.is_some();
    let run = record.run_id.is_some();
    let conversation = record.conversation_id.is_some();
    match record.owner_scope {
        SourceCustodyOwnerScope::Session => !task && !run && !conversation,
        SourceCustodyOwnerScope::Task => task && !run && !conversation,
        SourceCustodyOwnerScope::Run => !task && run && !conversation,
        SourceCustodyOwnerScope::Conversation => !task && !run && conversation,
    }
}

fn validate_fields(record: &SourceArtifactCustody) -> Result<(), SourceCustodyError> {
    let digests = [
        record.policy_sha256.as_str(),
        record.custody_sha256.as_str(),
        record.payload.manifest_sha256.as_str(),
        record.payload.payload_sha256.as_str(),
    ];
    let bounded_reason = !record.reason_code.is_empty()
        && record.reason_code.len() <= MAX_SOURCE_CUSTODY_REASON_CODE_BYTES;
    if !bounded_reason
        || !digests.iter().copied().all(is_lowercase_sha256)
        || record.payload.schema_version != crate::CONTRACT_SCHEMA_VERSION
        || record.payload.artifact_id.as_str().is_empty()
    {
        return Err(SourceCustodyError::FieldMalformed);
    }
    Ok(())
}

fn validate_owner(record: &SourceArtifactCustody) -> Result<(), SourceCustodyError> {
    let required = [
        record.custody_id.as_str(),
        record.source_artifact_id.as_str(),
        record.session_id.as_str(),
        record.policy_id.as_str(),
    ];
    let optional = [
        record.task_id.as_ref().map(TaskId::as_str),
        record.run_id.as_ref().map(RuntimeRunId::as_str),
        record.conversation_id.as_ref().map(ConversationId::as_str),
    ];
    if required.iter().any(|value| value.is_empty()) {
        return Err(SourceCustodyError::FieldMalformed);
    }
    if optional.iter().flatten().any(|value| value.is_empty()) {
        return Err(SourceCustodyError::FieldMalformed);
    }
    if !owner_identities_match_scope(record) {
        return Err(SourceCustodyError::OwnerAmbiguous);
    }
    Ok(())
}

fn validate_retention(record: &SourceArtifactCustody) -> Result<(), SourceCustodyError> {
    let expiry = record.retention.expires_at_epoch_ms;
    let valid = match record.retention.kind {
        RuntimeEventRetentionKind::Ephemeral => false,
        RuntimeEventRetentionKind::UntilExpiration => {
            expiry.is_some_and(|value| value > record.admitted_at_epoch_ms)
        }
        RuntimeEventRetentionKind::Session | RuntimeEventRetentionKind::UserHold => {
            expiry.is_none()
        }
    };
    if valid {
        Ok(())
    } else {
        Err(SourceCustodyError::RetentionInvalid)
    }
}

fn validate_lifecycle(record: &SourceArtifactCustody) -> Result<(), SourceCustodyError> {
    if record.lifecycle_revision == 0
        || record.admitted_at_epoch_ms == 0
        || record.updated_at_epoch_ms < record.admitted_at_epoch_ms
    {
        return Err(SourceCustodyError::LifecycleInvalid);
    }
    let consistent = match record.lifecycle {
        RuntimeArtifactLifecycleState::Active => {
            record.integrity == RuntimeArtifactIntegrityState::Verified
                && record.active_owner_count >= 1
                && record.hold != SourceCustodyHold::Review
        }
        RuntimeArtifactLifecycleState::Quarantined => {
            let isolated = record.integrity != RuntimeArtifactIntegrityState::Verified
                && record.integrity != RuntimeArtifactIntegrityState::Deleted;
            isolated && record.hold == SourceCustodyHold::Review
        }
        RuntimeArtifactLifecycleState::Released => {
            record.integrity == RuntimeArtifactIntegrityState::Verified
                && record.hold == SourceCustodyHold::None
        }
        RuntimeArtifactLifecycleState::Deleted => {
            record.integrity == RuntimeArtifactIntegrityState::Deleted
                && record.hold == SourceCustodyHold::None
                && record.active_owner_count == 0
        }
    };
    if !consistent {
        return Err(SourceCustodyError::LifecycleInvalid);
    }
    Ok(())
}

/// Verifies every closed-field, ownership, retention, and lifecycle invariant of one claim.
///
/// Rejection is deterministic and content-free: no field value is echoed, and no unknown field
/// can reach this function because the record denies unknown fields at the parse boundary.
///
/// # Errors
///
/// Returns the exact first violated invariant as a [`SourceCustodyError`].
pub fn validate_source_artifact_custody(
    record: &SourceArtifactCustody,
) -> Result<(), SourceCustodyError> {
    if record.schema_version != crate::CONTRACT_SCHEMA_VERSION {
        return Err(SourceCustodyError::SchemaVersionUnsupported);
    }
    if record.store_id != SOURCE_CUSTODY_STORE_ID {
        return Err(SourceCustodyError::ForeignStore);
    }
    if !SOURCE_BACKING_ARTIFACT_KINDS.contains(&record.payload_kind) {
        return Err(SourceCustodyError::UnsupportedBackingKind);
    }
    validate_fields(record)?;
    validate_owner(record)?;
    validate_retention(record)?;
    validate_lifecycle(record)
}

/// Verifies that one valid claim binds the exact immutable manifest it names.
///
/// The manifest remains the single authority for the payload's family, size, and media type, so
/// a claim can never assert a backing the store does not hold.
///
/// # Errors
///
/// Returns [`SourceCustodyError::ManifestMismatch`] when any bound field differs, or the exact
/// validation failure when the claim itself is invalid.
pub fn bind_source_artifact_custody(
    record: &SourceArtifactCustody,
    manifest: &RuntimeArtifactManifest,
) -> Result<(), SourceCustodyError> {
    validate_source_artifact_custody(record)?;
    if record.payload.artifact_id != manifest.artifact_id
        || record.payload.manifest_sha256 != manifest.manifest_sha256
        || record.payload.payload_sha256 != manifest.payload_sha256
        || record.payload.byte_size != manifest.byte_size
        || record.payload.media_type != manifest.media_type
        || record.payload_kind != manifest.kind
    {
        return Err(SourceCustodyError::ManifestMismatch);
    }
    Ok(())
}

/// Derives the exact content-free disposition of one valid claim at one trusted time.
///
/// The result is total and deterministic: every valid claim resolves to exactly one
/// disposition, and an invalid claim resolves to none.
///
/// # Errors
///
/// Returns the exact validation failure when the claim is not a valid custody record.
pub fn source_custody_disposition(
    record: &SourceArtifactCustody,
    now_epoch_ms: u64,
) -> Result<SourceCustodyDisposition, SourceCustodyError> {
    validate_source_artifact_custody(record)?;
    if record.lifecycle == RuntimeArtifactLifecycleState::Deleted {
        return Ok(SourceCustodyDisposition::Completed);
    }
    if record.lifecycle == RuntimeArtifactLifecycleState::Quarantined {
        return Ok(SourceCustodyDisposition::Blocked);
    }
    if matches!(
        record.hold,
        SourceCustodyHold::Checkpoint | SourceCustodyHold::User
    ) {
        return Ok(SourceCustodyDisposition::Retain);
    }
    if record.lifecycle == RuntimeArtifactLifecycleState::Released {
        if record.active_owner_count == 0 {
            return Ok(SourceCustodyDisposition::DeleteEligible);
        }
        return Ok(SourceCustodyDisposition::ReleaseEligible);
    }
    let expiry = record.retention.expires_at_epoch_ms;
    if expiry.is_some_and(|value| value <= now_epoch_ms) {
        return Ok(SourceCustodyDisposition::ReleaseEligible);
    }
    Ok(SourceCustodyDisposition::Retain)
}
