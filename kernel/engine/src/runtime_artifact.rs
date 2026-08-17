//! Closed runtime artifact manifests, references, and resume bindings.

use std::collections::BTreeSet;

use agentmage_kernel_contracts::{
    CONTRACT_SCHEMA_VERSION, RuntimeArtifactIntegrityState, RuntimeArtifactKind,
    RuntimeArtifactManifest, RuntimeArtifactRef, RuntimeEventRetentionKind,
    RuntimePayloadReference, RuntimeResumeBinding, to_canonical_json,
};
use sha2::{Digest, Sha256};

const ZERO_SHA256: &str = "0000000000000000000000000000000000000000000000000000000000000000";
/// Maximum bytes admitted for one runtime payload in the initial store profile.
pub const MAX_RUNTIME_ARTIFACT_BYTES: u64 = 64 * 1024 * 1024;
/// Maximum UTF-8 bytes retained in one encrypted artifact preview.
pub const MAX_RUNTIME_ARTIFACT_PREVIEW_BYTES: usize = 4 * 1024;
/// Maximum exact artifact references bound to one resumable checkpoint.
pub const MAX_RUNTIME_ARTIFACTS_PER_CHECKPOINT: usize = 1_024;

/// Stable fail-closed artifact-contract result.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuntimeArtifactError {
    /// A field, relationship, size, retention, or media type is outside the closed contract.
    InvalidManifest,
    /// A path-free reference does not match its verified immutable manifest.
    ReferenceMismatch,
    /// A resumable checkpoint binding is malformed, unordered, duplicated, or inconsistent.
    InvalidResumeBinding,
    /// Canonical contract serialization failed.
    Serialization,
    /// A canonical manifest or resume-binding digest does not match.
    DigestMismatch,
}

impl RuntimeArtifactError {
    /// Returns a stable content-free diagnostic code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidManifest => "runtime.artifact.manifest_invalid",
            Self::ReferenceMismatch => "runtime.artifact.reference_mismatch",
            Self::InvalidResumeBinding => "runtime.artifact.resume_binding_invalid",
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
        | RuntimeArtifactKind::TestLog
        | RuntimeArtifactKind::ModelOutput => matches!(
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
    use agentmage_kernel_contracts::{
        CONTRACT_SCHEMA_VERSION, ContextSensitivity, PolicyId, RuntimeArtifactId,
        RuntimeArtifactIntegrityState, RuntimeArtifactKind, RuntimeArtifactManifest,
        RuntimeArtifactPreview, RuntimeEventCursor, RuntimeEventId, RuntimeEventRetention,
        RuntimeEventRetentionKind, RuntimeOperationId, RuntimeResumeBinding, RuntimeRunId,
        RuntimeTurnId, SessionCheckpointId, SessionId, TaskId,
    };

    use super::{
        MAX_RUNTIME_ARTIFACT_BYTES, RuntimeArtifactError, runtime_artifact_ref,
        runtime_payload_reference, seal_runtime_artifact_manifest, seal_runtime_resume_binding,
        verify_runtime_artifact_manifest, verify_runtime_artifact_ref,
        verify_runtime_resume_binding,
    };

    fn digest(value: char) -> String {
        value.to_string().repeat(64)
    }

    fn manifest() -> RuntimeArtifactManifest {
        seal_runtime_artifact_manifest(RuntimeArtifactManifest {
            schema_version: CONTRACT_SCHEMA_VERSION,
            artifact_id: RuntimeArtifactId::from_raw("runtime-artifact-1"),
            kind: RuntimeArtifactKind::TestLog,
            payload_sha256: digest('a'),
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
}
