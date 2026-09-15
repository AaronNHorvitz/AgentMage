//! Deterministic validation of logical source-artifact ownership and retention.
//!
//! The closed public schema owns field syntax, bounds, and required members. This module
//! owns the cross-field rules that keep one logical source artifact, its owner, its
//! retention assignment, and the one existing content-addressed payload store consistent.
//!
//! Custody never asserts backend facts on its own. A retained record is admitted only
//! against the source artifact it names, the canonically sealed
//! [`RuntimeArtifactManifest`] of the artifact its binding names, and that artifact's
//! current authoritative lifecycle projection. A nonexistent, forged, differently owned,
//! differently retained, differently sized, or differently disposed backend object
//! therefore cannot be described into existence by a well-formed custody record.

use agentmage_kernel_contracts::{
    CONTRACT_SCHEMA_VERSION, CanonicalCaptureState, RuntimeArtifactLifecycleState,
    RuntimeArtifactManifest, RuntimeArtifactOperatorView, RuntimeEventRetentionKind,
    SOURCE_ARTIFACT_CUSTODY_SCHEMA_VERSION, SOURCE_CUSTODY_ARTIFACT_KIND, SourceArtifactCustody,
    SourceCustodyState,
};

use crate::engineering_records::CanonicalRecordError;
use crate::runtime_artifact::{
    valid_lifecycle_pair, verify_runtime_artifact_manifest, verify_runtime_artifact_ref,
};
use crate::runtime_hardening::MAX_RUNTIME_ARTIFACT_BYTES;

/// Smallest payload the existing runtime artifact store can publish as one object.
///
/// The source-capture ceiling is deliberately wider than this backend range. A capture
/// that no single backend object can hold is representable as a source artifact and is
/// simply not retainable.
pub const MIN_SOURCE_CUSTODY_PAYLOAD_BYTES: u64 = 1;

/// Largest payload the existing runtime artifact store can publish as one object.
pub const MAX_SOURCE_CUSTODY_PAYLOAD_BYTES: u64 = MAX_RUNTIME_ARTIFACT_BYTES;

/// Exact identity and capture facts of the source artifact that owns one custody record.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceCaptureFacts<'a> {
    /// Authoritative source-artifact identity these facts belong to.
    pub source_artifact_id: &'a str,
    /// Terminal capture state recorded for that source.
    pub capture_state: CanonicalCaptureState,
    /// Authoritative source digest, present only when bytes were captured.
    pub sha256: Option<&'a str>,
    /// Authoritative source byte length, present only when bytes were captured.
    pub byte_length: Option<u64>,
}

/// Authoritative backend facts for the one artifact a retained custody record names.
///
/// Both members must come from the artifact store itself: the immutable publication
/// manifest, which is verified here rather than trusted, and the current lifecycle
/// projection of that same artifact.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceCustodyBackendFacts<'a> {
    /// Canonically sealed publication manifest of the named artifact.
    pub manifest: &'a RuntimeArtifactManifest,
    /// Current authoritative lifecycle projection of that same artifact.
    pub current: &'a RuntimeArtifactOperatorView,
}

/// Validates one custody record against its source artifact and its backend artifact.
///
/// The record cannot belong to a source artifact other than the one whose capture facts
/// are supplied, cannot claim a payload that source never captured, cannot retain bytes
/// under an in-memory retention class, cannot release a checkpoint-rooted reference, and
/// cannot name payload bytes that differ from the authoritative source content address.
///
/// `backend` must describe the artifact named by the binding and must be present exactly
/// when the record retains a payload. The manifest is admitted only when its own canonical
/// seal verifies, and identity, seal, artifact kind, payload identity, owner identities,
/// and retention assignment are reconciled exactly against it. Custody lifecycle,
/// integrity, and checkpoint rooting are then reconciled exactly against the current
/// authoritative projection, so custody can neither outlive nor predate backend state.
pub fn validate_source_custody(
    custody: &SourceArtifactCustody,
    capture: SourceCaptureFacts<'_>,
    backend: Option<SourceCustodyBackendFacts<'_>>,
) -> Result<(), CanonicalRecordError> {
    if custody.schema_version != SOURCE_ARTIFACT_CUSTODY_SCHEMA_VERSION {
        return Err(error("engineering.custody.version", "schema_version"));
    }
    if custody.source_artifact_id != capture.source_artifact_id {
        return Err(error(
            "engineering.custody.source_identity",
            "source_artifact_id",
        ));
    }
    if custody.artifact_kind != SOURCE_CUSTODY_ARTIFACT_KIND {
        return Err(error(
            "engineering.custody.kind_unsupported",
            "artifact_kind",
        ));
    }

    let retained = custody.state != SourceCustodyState::NotRetained;
    if retained != custody.binding.is_some() {
        return Err(error("engineering.custody.binding_state", "binding"));
    }
    let ephemeral = custody.retention.kind == RuntimeEventRetentionKind::Ephemeral;
    if retained == ephemeral {
        return Err(error("engineering.custody.retention_state", "retention"));
    }
    let expiring = custody.retention.kind == RuntimeEventRetentionKind::UntilExpiration;
    if expiring != custody.retention.expires_at_epoch_ms.is_some() {
        return Err(error("engineering.custody.expiration", "retention"));
    }

    let closed_state = !matches!(
        custody.state,
        SourceCustodyState::NotRetained | SourceCustodyState::Active
    );
    if closed_state != custody.release_reason_code.is_some() {
        return Err(error(
            "engineering.custody.reason_code",
            "release_reason_code",
        ));
    }
    let openable = matches!(
        custody.state,
        SourceCustodyState::Active | SourceCustodyState::Quarantined
    );
    if custody.checkpoint_rooted && !openable {
        return Err(error("engineering.custody.checkpoint_root", "state"));
    }

    if retained && capture.capture_state != CanonicalCaptureState::Captured {
        return Err(error("engineering.custody.uncaptured_retention", "state"));
    }
    if retained != backend.is_some() {
        return Err(error("engineering.custody.manifest_state", "binding"));
    }

    let (Some(binding), Some(backend)) = (&custody.binding, backend) else {
        return Ok(());
    };
    if binding.byte_size < MIN_SOURCE_CUSTODY_PAYLOAD_BYTES
        || binding.byte_size > MAX_SOURCE_CUSTODY_PAYLOAD_BYTES
    {
        return Err(error("engineering.custody.payload_bounds", "binding"));
    }
    let digest_matches = Some(binding.payload_sha256.as_str()) == capture.sha256;
    let size_matches = Some(binding.byte_size) == capture.byte_length;
    if !digest_matches || !size_matches {
        return Err(error("engineering.custody.payload_mismatch", "binding"));
    }

    let manifest = backend.manifest;
    if manifest.schema_version != CONTRACT_SCHEMA_VERSION {
        return Err(error("engineering.custody.manifest_version", "binding"));
    }
    if verify_runtime_artifact_manifest(manifest).is_err() {
        return Err(error("engineering.custody.manifest_unsealed", "binding"));
    }
    if manifest.artifact_id != binding.runtime_artifact_id {
        return Err(error("engineering.custody.manifest_identity", "binding"));
    }
    if manifest.manifest_sha256 != binding.manifest_sha256 {
        return Err(error("engineering.custody.manifest_seal", "binding"));
    }
    if manifest.kind != custody.artifact_kind {
        return Err(error("engineering.custody.manifest_kind", "artifact_kind"));
    }
    let manifest_payload_matches = manifest.payload_sha256 == binding.payload_sha256
        && manifest.byte_size == binding.byte_size;
    if !manifest_payload_matches {
        return Err(error("engineering.custody.manifest_payload", "binding"));
    }
    if manifest.session_id != custody.owner_session_id
        || manifest.task_id != custody.owner_task_id
        || manifest.producer_run_id != custody.owner_run_id
    {
        return Err(error(
            "engineering.custody.manifest_owner",
            "owner_session_id",
        ));
    }
    if manifest.retention != custody.retention {
        return Err(error("engineering.custody.manifest_retention", "retention"));
    }

    let current = backend.current;
    if current.schema_version != CONTRACT_SCHEMA_VERSION {
        return Err(error("engineering.custody.backend_version", "binding"));
    }
    let reference_matches = verify_runtime_artifact_ref(&current.reference, manifest);
    let projects_manifest = reference_matches.is_ok()
        && current.kind == manifest.kind
        && current.retention == manifest.retention
        && current.session_id == manifest.session_id
        && current.task_id == manifest.task_id
        && current.producer_run_id == manifest.producer_run_id;
    if !projects_manifest {
        return Err(error("engineering.custody.backend_identity", "binding"));
    }
    if expected_lifecycle(custody.state) != Some(current.lifecycle) {
        return Err(error("engineering.custody.backend_lifecycle", "state"));
    }
    if !valid_lifecycle_pair(current.lifecycle, current.integrity) {
        return Err(error("engineering.custody.backend_integrity", "state"));
    }
    if custody.checkpoint_rooted != (current.checkpoint_reference_count > 0) {
        return Err(error(
            "engineering.custody.backend_checkpoint",
            "checkpoint_rooted",
        ));
    }
    Ok(())
}

/// Returns the one backend lifecycle state a custody state may claim.
const fn expected_lifecycle(state: SourceCustodyState) -> Option<RuntimeArtifactLifecycleState> {
    match state {
        SourceCustodyState::NotRetained => None,
        SourceCustodyState::Active => Some(RuntimeArtifactLifecycleState::Active),
        SourceCustodyState::Quarantined => Some(RuntimeArtifactLifecycleState::Quarantined),
        SourceCustodyState::Released => Some(RuntimeArtifactLifecycleState::Released),
        SourceCustodyState::Deleted => Some(RuntimeArtifactLifecycleState::Deleted),
    }
}

const fn error(code: &'static str, field: &'static str) -> CanonicalRecordError {
    CanonicalRecordError { code, field }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime_artifact::seal_runtime_artifact_manifest;
    use agentmage_kernel_contracts::{
        ContextSensitivity, PolicyId, RuntimeArtifactCleanupState, RuntimeArtifactId,
        RuntimeArtifactIntegrityState, RuntimeArtifactKind, RuntimeArtifactRef,
        RuntimeEventRetention, RuntimeRunId, SessionId, SourceCustodyBackend, SourceCustodyBinding,
        TaskId,
    };

    const SOURCE: &str = "source:1";
    const DIGEST: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const FORGED_SEAL: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
    const POLICY: &str = "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";
    const BYTES: u64 = 4_096;
    const EXPIRES: u64 = 1_800_000_000_000;
    const CREATED: u64 = 1_700_000_000_000;
    const SOURCE_CAPTURE_CEILING: u64 = 104_857_600;

    const VERSION: &str = "engineering.custody.version";
    const SOURCE_IDENTITY: &str = "engineering.custody.source_identity";
    const KIND: &str = "engineering.custody.kind_unsupported";
    const BINDING: &str = "engineering.custody.binding_state";
    const RETENTION: &str = "engineering.custody.retention_state";
    const EXPIRATION: &str = "engineering.custody.expiration";
    const REASON: &str = "engineering.custody.reason_code";
    const ROOTED: &str = "engineering.custody.checkpoint_root";
    const UNCAPTURED: &str = "engineering.custody.uncaptured_retention";
    const PAYLOAD: &str = "engineering.custody.payload_mismatch";
    const BOUNDS: &str = "engineering.custody.payload_bounds";
    const MANIFEST_STATE: &str = "engineering.custody.manifest_state";
    const MANIFEST_VERSION: &str = "engineering.custody.manifest_version";
    const MANIFEST_UNSEALED: &str = "engineering.custody.manifest_unsealed";
    const MANIFEST_IDENTITY: &str = "engineering.custody.manifest_identity";
    const MANIFEST_SEAL: &str = "engineering.custody.manifest_seal";
    const MANIFEST_KIND: &str = "engineering.custody.manifest_kind";
    const MANIFEST_PAYLOAD: &str = "engineering.custody.manifest_payload";
    const MANIFEST_OWNER: &str = "engineering.custody.manifest_owner";
    const MANIFEST_RETENTION: &str = "engineering.custody.manifest_retention";
    const BACKEND_IDENTITY: &str = "engineering.custody.backend_identity";
    const BACKEND_LIFECYCLE: &str = "engineering.custody.backend_lifecycle";
    const BACKEND_INTEGRITY: &str = "engineering.custody.backend_integrity";
    const BACKEND_CHECKPOINT: &str = "engineering.custody.backend_checkpoint";

    fn captured() -> SourceCaptureFacts<'static> {
        SourceCaptureFacts {
            source_artifact_id: SOURCE,
            capture_state: CanonicalCaptureState::Captured,
            sha256: Some(DIGEST),
            byte_length: Some(BYTES),
        }
    }

    fn missing() -> SourceCaptureFacts<'static> {
        SourceCaptureFacts {
            source_artifact_id: SOURCE,
            capture_state: CanonicalCaptureState::Unavailable,
            sha256: None,
            byte_length: None,
        }
    }

    fn binding(payload_sha256: &str, byte_size: u64) -> SourceCustodyBinding {
        SourceCustodyBinding {
            runtime_artifact_id: RuntimeArtifactId::from_raw("artifact:1"),
            manifest_sha256: FORGED_SEAL.to_owned(),
            payload_sha256: payload_sha256.to_owned(),
            byte_size,
        }
    }

    fn retention(kind: RuntimeEventRetentionKind, expires: Option<u64>) -> RuntimeEventRetention {
        RuntimeEventRetention {
            kind,
            expires_at_epoch_ms: expires,
        }
    }

    fn active() -> SourceArtifactCustody {
        SourceArtifactCustody {
            schema_version: SOURCE_ARTIFACT_CUSTODY_SCHEMA_VERSION,
            source_artifact_id: SOURCE.to_owned(),
            backend: SourceCustodyBackend::RuntimeArtifactStore,
            artifact_kind: SOURCE_CUSTODY_ARTIFACT_KIND,
            binding: Some(binding(DIGEST, BYTES)),
            owner_session_id: SessionId::from_raw("session:1"),
            owner_task_id: TaskId::from_raw("task:1"),
            owner_run_id: RuntimeRunId::from_raw("run:1"),
            retention: retention(RuntimeEventRetentionKind::Session, None),
            state: SourceCustodyState::Active,
            checkpoint_rooted: false,
            release_reason_code: None,
        }
    }

    fn unretained() -> SourceArtifactCustody {
        SourceArtifactCustody {
            binding: None,
            retention: retention(RuntimeEventRetentionKind::Ephemeral, None),
            state: SourceCustodyState::NotRetained,
            ..active()
        }
    }

    /// Returns one custody record in `state` carrying every reason code that state needs.
    fn closed(state: SourceCustodyState) -> SourceArtifactCustody {
        if state == SourceCustodyState::Active {
            return active();
        }
        SourceArtifactCustody {
            state,
            release_reason_code: Some("owner-release".to_owned()),
            ..active()
        }
    }

    /// Returns the manifest the artifact store would publish for one custody record.
    fn draft_manifest(custody: &SourceArtifactCustody) -> RuntimeArtifactManifest {
        let binding = custody
            .binding
            .clone()
            .expect("a retained fixture names one artifact");
        RuntimeArtifactManifest {
            schema_version: CONTRACT_SCHEMA_VERSION,
            artifact_id: binding.runtime_artifact_id,
            kind: custody.artifact_kind,
            payload_sha256: binding.payload_sha256,
            byte_size: binding.byte_size,
            media_type: "text/plain".to_owned(),
            sensitivity: ContextSensitivity::Internal,
            retention: custody.retention.clone(),
            session_id: custody.owner_session_id.clone(),
            task_id: custody.owner_task_id.clone(),
            producer_run_id: custody.owner_run_id.clone(),
            producer_turn_id: None,
            producer_operation_id: None,
            receipt_id: None,
            policy_id: PolicyId::from_raw("policy:1"),
            policy_sha256: POLICY.to_owned(),
            created_at_epoch_ms: CREATED,
            integrity: RuntimeArtifactIntegrityState::Verified,
            preview: None,
            manifest_sha256: FORGED_SEAL.to_owned(),
        }
    }

    /// Returns one custody record and the actually sealed manifest its binding repeats.
    ///
    /// A deliberately invalid fixture cannot seal; admission must then reject the record
    /// rather than the fixture panicking before the rule under test runs.
    fn sealed_pair(
        custody: &SourceArtifactCustody,
    ) -> (SourceArtifactCustody, RuntimeArtifactManifest) {
        let draft = draft_manifest(custody);
        let resealed = seal_runtime_artifact_manifest(draft.clone());
        let manifest = resealed.unwrap_or(draft);
        (rebind(custody, &manifest), manifest)
    }

    /// Reseals one deliberately drifted manifest so admission judges the drift itself.
    fn seal(manifest: RuntimeArtifactManifest) -> RuntimeArtifactManifest {
        seal_runtime_artifact_manifest(manifest)
            .expect("a drifted fixture must remain sealable")
    }

    /// Returns one custody record whose binding repeats the exact seal of `manifest`.
    fn rebind(
        custody: &SourceArtifactCustody,
        manifest: &RuntimeArtifactManifest,
    ) -> SourceArtifactCustody {
        let mut rebound = custody.clone();
        if let Some(binding) = rebound.binding.as_mut() {
            binding.manifest_sha256 = manifest.manifest_sha256.clone();
        }
        rebound
    }

    fn reference(manifest: &RuntimeArtifactManifest) -> RuntimeArtifactRef {
        RuntimeArtifactRef {
            schema_version: CONTRACT_SCHEMA_VERSION,
            artifact_id: manifest.artifact_id.clone(),
            manifest_sha256: manifest.manifest_sha256.clone(),
            payload_sha256: manifest.payload_sha256.clone(),
            byte_size: manifest.byte_size,
            media_type: manifest.media_type.clone(),
        }
    }

    const fn cleanup(lifecycle: RuntimeArtifactLifecycleState) -> RuntimeArtifactCleanupState {
        match lifecycle {
            RuntimeArtifactLifecycleState::Active => RuntimeArtifactCleanupState::Retained,
            RuntimeArtifactLifecycleState::Released => RuntimeArtifactCleanupState::Eligible,
            RuntimeArtifactLifecycleState::Quarantined => RuntimeArtifactCleanupState::Blocked,
            RuntimeArtifactLifecycleState::Deleted => RuntimeArtifactCleanupState::Completed,
        }
    }

    /// Returns the store projection of one artifact under an exact current disposition.
    fn operator_view(
        manifest: &RuntimeArtifactManifest,
        lifecycle: RuntimeArtifactLifecycleState,
        integrity: RuntimeArtifactIntegrityState,
        checkpoint_reference_count: u32,
    ) -> RuntimeArtifactOperatorView {
        RuntimeArtifactOperatorView {
            schema_version: CONTRACT_SCHEMA_VERSION,
            reference: reference(manifest),
            kind: manifest.kind,
            sensitivity: manifest.sensitivity,
            retention: manifest.retention.clone(),
            session_id: manifest.session_id.clone(),
            task_id: manifest.task_id.clone(),
            producer_run_id: manifest.producer_run_id.clone(),
            producer_turn_id: None,
            producer_operation_id: None,
            receipt_id: None,
            policy_id: manifest.policy_id.clone(),
            created_at_epoch_ms: manifest.created_at_epoch_ms,
            lifecycle,
            integrity,
            lifecycle_revision: 1,
            reason_code: "artifact.published".to_owned(),
            updated_at_epoch_ms: CREATED,
            checkpoint_reference_count,
            shared_active_reference_count: 1,
            cleanup: cleanup(lifecycle),
        }
    }

    /// Returns the current backend disposition one custody record legitimately claims.
    fn current_view(
        manifest: &RuntimeArtifactManifest,
        custody: &SourceArtifactCustody,
    ) -> RuntimeArtifactOperatorView {
        let (lifecycle, integrity) = match custody.state {
            SourceCustodyState::NotRetained | SourceCustodyState::Active => (
                RuntimeArtifactLifecycleState::Active,
                RuntimeArtifactIntegrityState::Verified,
            ),
            SourceCustodyState::Quarantined => (
                RuntimeArtifactLifecycleState::Quarantined,
                RuntimeArtifactIntegrityState::Quarantined,
            ),
            SourceCustodyState::Released => (
                RuntimeArtifactLifecycleState::Released,
                RuntimeArtifactIntegrityState::Verified,
            ),
            SourceCustodyState::Deleted => (
                RuntimeArtifactLifecycleState::Deleted,
                RuntimeArtifactIntegrityState::Deleted,
            ),
        };
        operator_view(
            manifest,
            lifecycle,
            integrity,
            u32::from(custody.checkpoint_rooted),
        )
    }

    /// Admits one custody record against its own sealed artifact and captured source.
    fn admit(custody: &SourceArtifactCustody) -> Result<(), CanonicalRecordError> {
        admit_capture(custody, captured())
    }

    fn admit_capture(
        custody: &SourceArtifactCustody,
        capture: SourceCaptureFacts<'_>,
    ) -> Result<(), CanonicalRecordError> {
        if custody.binding.is_none() {
            return validate_source_custody(custody, capture, None);
        }
        let (sealed, manifest) = sealed_pair(custody);
        let current = current_view(&manifest, &sealed);
        validate_source_custody(&sealed, capture, Some(facts(&manifest, &current)))
    }

    fn facts<'a>(
        manifest: &'a RuntimeArtifactManifest,
        current: &'a RuntimeArtifactOperatorView,
    ) -> SourceCustodyBackendFacts<'a> {
        SourceCustodyBackendFacts { manifest, current }
    }

    fn failure(custody: &SourceArtifactCustody, capture: SourceCaptureFacts<'_>) -> &'static str {
        admit_capture(custody, capture)
            .expect_err("custody must fail closed")
            .code
    }

    fn admit_backend(
        custody: &SourceArtifactCustody,
        manifest: &RuntimeArtifactManifest,
        current: &RuntimeArtifactOperatorView,
    ) -> Result<(), CanonicalRecordError> {
        validate_source_custody(custody, captured(), Some(facts(manifest, current)))
    }

    fn backend_failure(
        custody: &SourceArtifactCustody,
        manifest: &RuntimeArtifactManifest,
        current: &RuntimeArtifactOperatorView,
    ) -> &'static str {
        admit_backend(custody, manifest, current)
            .expect_err("custody must fail closed")
            .code
    }

    /// Fails one custody record against a drifted manifest and that manifest's own view.
    fn manifest_failure(
        custody: &SourceArtifactCustody,
        manifest: &RuntimeArtifactManifest,
    ) -> &'static str {
        let current = current_view(manifest, custody);
        backend_failure(custody, manifest, &current)
    }

    /// Fails one custody record against a sealed manifest that contradicts it.
    ///
    /// The binding repeats the drifted manifest's own seal, so admission must reject the
    /// contradiction itself rather than a seal the forger could trivially copy.
    fn drift_failure(
        custody: &SourceArtifactCustody,
        manifest: &RuntimeArtifactManifest,
    ) -> &'static str {
        manifest_failure(&rebind(custody, manifest), manifest)
    }

    #[test]
    fn active_custody_binds_the_exact_captured_content_address() {
        assert_eq!(admit(&active()), Ok(()));
    }

    #[test]
    fn custody_is_admitted_only_by_the_source_artifact_that_owns_it() {
        let custody = active();
        assert_eq!(admit(&custody), Ok(()));

        // A different source artifact with identical bytes must not admit this record.
        let twin = SourceCaptureFacts {
            source_artifact_id: "source:2",
            ..captured()
        };
        assert_eq!(failure(&custody, twin), SOURCE_IDENTITY);

        let owned_by_twin = SourceArtifactCustody {
            source_artifact_id: "source:2".to_owned(),
            ..active()
        };
        assert_eq!(admit_capture(&owned_by_twin, twin), Ok(()));
        assert_eq!(failure(&owned_by_twin, captured()), SOURCE_IDENTITY);
    }

    #[test]
    fn ephemeral_capture_stays_outside_the_durable_store() {
        assert_eq!(admit(&unretained()), Ok(()));
        assert_eq!(admit_capture(&unretained(), missing()), Ok(()));

        let claimed = SourceArtifactCustody {
            state: SourceCustodyState::Active,
            ..unretained()
        };
        assert_eq!(failure(&claimed, captured()), BINDING);

        let durable_empty = SourceArtifactCustody {
            retention: retention(RuntimeEventRetentionKind::Session, None),
            ..unretained()
        };
        assert_eq!(failure(&durable_empty, captured()), RETENTION);
    }

    #[test]
    fn an_uncaptured_source_cannot_retain_a_payload() {
        assert_eq!(failure(&active(), missing()), UNCAPTURED);
    }

    #[test]
    fn retained_bytes_must_equal_the_authoritative_source() {
        let wrong_digest = SourceArtifactCustody {
            binding: Some(binding(&"d".repeat(64), BYTES)),
            ..active()
        };
        assert_eq!(failure(&wrong_digest, captured()), PAYLOAD);

        let wrong_size = SourceArtifactCustody {
            binding: Some(binding(DIGEST, BYTES + 1)),
            ..active()
        };
        assert_eq!(failure(&wrong_size, captured()), PAYLOAD);
    }

    #[test]
    fn expiration_is_required_exactly_for_expiring_retention() {
        let expiring = SourceArtifactCustody {
            retention: retention(RuntimeEventRetentionKind::UntilExpiration, Some(EXPIRES)),
            ..active()
        };
        assert_eq!(admit(&expiring), Ok(()));

        let undated = SourceArtifactCustody {
            retention: retention(RuntimeEventRetentionKind::UntilExpiration, None),
            ..active()
        };
        assert_eq!(failure(&undated, captured()), EXPIRATION);

        let held = SourceArtifactCustody {
            retention: retention(RuntimeEventRetentionKind::UserHold, Some(EXPIRES)),
            ..active()
        };
        assert_eq!(failure(&held, captured()), EXPIRATION);
    }

    #[test]
    fn released_and_quarantined_references_require_a_stable_reason_code() {
        for state in [
            SourceCustodyState::Quarantined,
            SourceCustodyState::Released,
            SourceCustodyState::Deleted,
        ] {
            let silent = SourceArtifactCustody { state, ..active() };
            assert_eq!(failure(&silent, captured()), REASON);
            assert_eq!(admit(&closed(state)), Ok(()));
        }

        let unexplained = SourceArtifactCustody {
            release_reason_code: Some("owner-release".to_owned()),
            ..active()
        };
        assert_eq!(failure(&unexplained, captured()), REASON);
    }

    #[test]
    fn a_checkpoint_rooted_reference_cannot_be_released_or_deleted() {
        let rooted = SourceArtifactCustody {
            checkpoint_rooted: true,
            ..active()
        };
        assert_eq!(admit(&rooted), Ok(()));

        for state in [SourceCustodyState::Released, SourceCustodyState::Deleted] {
            let released = SourceArtifactCustody {
                checkpoint_rooted: true,
                ..closed(state)
            };
            assert_eq!(failure(&released, captured()), ROOTED);
        }
    }

    #[test]
    fn custody_reuses_the_frozen_runtime_artifact_kind_family() {
        let frozen = [
            RuntimeArtifactKind::Patch,
            RuntimeArtifactKind::StandardOutput,
            RuntimeArtifactKind::StandardError,
            RuntimeArtifactKind::TestLog,
            RuntimeArtifactKind::GeneratedFile,
            RuntimeArtifactKind::Report,
            RuntimeArtifactKind::ModelOutput,
        ];
        for kind in frozen {
            // This exhaustive match cannot compile if the closed family is ever widened.
            let wire = match kind {
                RuntimeArtifactKind::Patch => "patch",
                RuntimeArtifactKind::StandardOutput => "standard_output",
                RuntimeArtifactKind::StandardError => "standard_error",
                RuntimeArtifactKind::TestLog => "test_log",
                RuntimeArtifactKind::GeneratedFile => "generated_file",
                RuntimeArtifactKind::Report => "report",
                RuntimeArtifactKind::ModelOutput => "model_output",
            };
            let encoded = serde_json::to_string(&kind).expect("kind encodes");
            assert_eq!(encoded, format!("\"{wire}\""));

            let custody = SourceArtifactCustody {
                artifact_kind: kind,
                ..active()
            };
            if kind == SOURCE_CUSTODY_ARTIFACT_KIND {
                assert_eq!(admit(&custody), Ok(()));
            } else {
                assert_eq!(failure(&custody, captured()), KIND);
            }
        }
        assert_eq!(
            SOURCE_CUSTODY_ARTIFACT_KIND,
            RuntimeArtifactKind::GeneratedFile
        );
    }

    #[test]
    fn an_unsupported_custody_version_fails_closed() {
        for unsupported in [
            SOURCE_ARTIFACT_CUSTODY_SCHEMA_VERSION - 1,
            SOURCE_ARTIFACT_CUSTODY_SCHEMA_VERSION + 1,
        ] {
            let drifted = SourceArtifactCustody {
                schema_version: unsupported,
                ..active()
            };
            assert_eq!(failure(&drifted, captured()), VERSION);
        }
        assert_eq!(
            SOURCE_ARTIFACT_CUSTODY_SCHEMA_VERSION,
            CONTRACT_SCHEMA_VERSION
        );
    }

    #[test]
    fn custody_admission_requires_an_actually_sealed_manifest() {
        let (custody, sealed) = sealed_pair(&active());
        let current = current_view(&sealed, &custody);
        assert_eq!(admit_backend(&custody, &sealed, &current), Ok(()));

        // A manifest and binding that agree on an arbitrary seal are still not canonical.
        let forged = RuntimeArtifactManifest {
            manifest_sha256: "e".repeat(64),
            ..sealed.clone()
        };
        assert_eq!(drift_failure(&custody, &forged), MANIFEST_UNSEALED);

        // A mutated manifest carrying the previously canonical seal is equally forged.
        let mutated = RuntimeArtifactManifest {
            created_at_epoch_ms: CREATED + 1,
            ..sealed.clone()
        };
        assert_eq!(manifest_failure(&custody, &mutated), MANIFEST_UNSEALED);

        // A binding may not name a seal other than the canonical one it must repeat.
        let mismatched = SourceArtifactCustody {
            binding: Some(binding(DIGEST, BYTES)),
            ..custody
        };
        assert_eq!(manifest_failure(&mismatched, &sealed), MANIFEST_SEAL);
    }

    #[test]
    fn custody_admission_requires_the_exact_verified_runtime_manifest() {
        let (custody, sealed) = sealed_pair(&active());
        let current = current_view(&sealed, &custody);
        assert_eq!(admit_backend(&custody, &sealed, &current), Ok(()));

        assert_eq!(
            validate_source_custody(&custody, captured(), None)
                .expect_err("an unbacked retention must fail closed")
                .code,
            MANIFEST_STATE
        );
        let backend = facts(&sealed, &current);
        assert_eq!(
            validate_source_custody(&unretained(), captured(), Some(backend))
                .expect_err("an unretained record owns no backend object")
                .code,
            MANIFEST_STATE
        );

        let drifted = RuntimeArtifactManifest {
            schema_version: CONTRACT_SCHEMA_VERSION + 1,
            ..sealed.clone()
        };
        assert_eq!(drift_failure(&custody, &drifted), MANIFEST_VERSION);

        let other_id = seal(RuntimeArtifactManifest {
            artifact_id: RuntimeArtifactId::from_raw("artifact:2"),
            ..sealed.clone()
        });
        assert_eq!(drift_failure(&custody, &other_id), MANIFEST_IDENTITY);

        let other_kind = seal(RuntimeArtifactManifest {
            kind: RuntimeArtifactKind::Patch,
            ..sealed.clone()
        });
        assert_eq!(drift_failure(&custody, &other_kind), MANIFEST_KIND);

        let other_digest = seal(RuntimeArtifactManifest {
            payload_sha256: "f".repeat(64),
            ..sealed.clone()
        });
        assert_eq!(drift_failure(&custody, &other_digest), MANIFEST_PAYLOAD);

        let other_size = seal(RuntimeArtifactManifest {
            byte_size: BYTES + 1,
            ..sealed.clone()
        });
        assert_eq!(drift_failure(&custody, &other_size), MANIFEST_PAYLOAD);

        let other_session = seal(RuntimeArtifactManifest {
            session_id: SessionId::from_raw("session:2"),
            ..sealed.clone()
        });
        assert_eq!(drift_failure(&custody, &other_session), MANIFEST_OWNER);

        let other_task = seal(RuntimeArtifactManifest {
            task_id: TaskId::from_raw("task:2"),
            ..sealed.clone()
        });
        assert_eq!(drift_failure(&custody, &other_task), MANIFEST_OWNER);

        let other_run = seal(RuntimeArtifactManifest {
            producer_run_id: RuntimeRunId::from_raw("run:2"),
            ..sealed.clone()
        });
        assert_eq!(drift_failure(&custody, &other_run), MANIFEST_OWNER);

        let other_retention = seal(RuntimeArtifactManifest {
            retention: retention(RuntimeEventRetentionKind::UserHold, None),
            ..sealed
        });
        assert_eq!(
            drift_failure(&custody, &other_retention),
            MANIFEST_RETENTION
        );
    }

    #[test]
    fn custody_state_must_equal_the_authoritative_backend_lifecycle() {
        let backend_states = [
            (
                SourceCustodyState::Active,
                RuntimeArtifactLifecycleState::Active,
                RuntimeArtifactIntegrityState::Verified,
            ),
            (
                SourceCustodyState::Quarantined,
                RuntimeArtifactLifecycleState::Quarantined,
                RuntimeArtifactIntegrityState::Quarantined,
            ),
            (
                SourceCustodyState::Released,
                RuntimeArtifactLifecycleState::Released,
                RuntimeArtifactIntegrityState::Verified,
            ),
            (
                SourceCustodyState::Deleted,
                RuntimeArtifactLifecycleState::Deleted,
                RuntimeArtifactIntegrityState::Deleted,
            ),
        ];
        for (_, lifecycle, integrity) in backend_states {
            for (state, _, _) in backend_states {
                let (custody, manifest) = sealed_pair(&closed(state));
                let current = operator_view(&manifest, lifecycle, integrity, 0);
                let admitted = admit_backend(&custody, &manifest, &current);
                if expected_lifecycle(state) == Some(lifecycle) {
                    assert_eq!(admitted, Ok(()), "{state:?} must match {lifecycle:?}");
                } else {
                    let code = admitted.expect_err("drift must fail closed").code;
                    assert_eq!(code, BACKEND_LIFECYCLE, "{state:?} vs {lifecycle:?}");
                }
            }
        }
    }

    #[test]
    fn custody_cannot_claim_an_integrity_or_identity_the_backend_denies() {
        let (custody, manifest) = sealed_pair(&active());
        let unverified = operator_view(
            &manifest,
            RuntimeArtifactLifecycleState::Active,
            RuntimeArtifactIntegrityState::Missing,
            0,
        );
        assert_eq!(
            backend_failure(&custody, &manifest, &unverified),
            BACKEND_INTEGRITY
        );

        let (quarantined, quarantined_manifest) =
            sealed_pair(&closed(SourceCustodyState::Quarantined));
        let unquarantined = operator_view(
            &quarantined_manifest,
            RuntimeArtifactLifecycleState::Quarantined,
            RuntimeArtifactIntegrityState::Verified,
            0,
        );
        assert_eq!(
            backend_failure(&quarantined, &quarantined_manifest, &unquarantined),
            BACKEND_INTEGRITY
        );

        let other_artifact = seal(RuntimeArtifactManifest {
            artifact_id: RuntimeArtifactId::from_raw("artifact:2"),
            ..manifest.clone()
        });
        let other_view = operator_view(
            &other_artifact,
            RuntimeArtifactLifecycleState::Active,
            RuntimeArtifactIntegrityState::Verified,
            0,
        );
        assert_eq!(
            backend_failure(&custody, &manifest, &other_view),
            BACKEND_IDENTITY
        );
    }

    #[test]
    fn checkpoint_rooting_must_equal_the_canonical_checkpoint_reference_count() {
        let (custody, manifest) = sealed_pair(&active());
        let referenced = operator_view(
            &manifest,
            RuntimeArtifactLifecycleState::Active,
            RuntimeArtifactIntegrityState::Verified,
            1,
        );
        assert_eq!(
            backend_failure(&custody, &manifest, &referenced),
            BACKEND_CHECKPOINT
        );

        let rooted = SourceArtifactCustody {
            checkpoint_rooted: true,
            ..custody
        };
        assert_eq!(admit_backend(&rooted, &manifest, &referenced), Ok(()));
        let unreferenced = operator_view(
            &manifest,
            RuntimeArtifactLifecycleState::Active,
            RuntimeArtifactIntegrityState::Verified,
            0,
        );
        assert_eq!(
            backend_failure(&rooted, &manifest, &unreferenced),
            BACKEND_CHECKPOINT
        );
    }

    #[test]
    fn a_retained_binding_must_be_representable_by_the_existing_backend() {
        assert_eq!(MIN_SOURCE_CUSTODY_PAYLOAD_BYTES, 1);
        assert_eq!(MAX_SOURCE_CUSTODY_PAYLOAD_BYTES, 67_108_864);
        const { assert!(SOURCE_CAPTURE_CEILING > MAX_SOURCE_CUSTODY_PAYLOAD_BYTES) };

        let sized = |byte_size: u64| SourceArtifactCustody {
            binding: Some(binding(DIGEST, byte_size)),
            ..active()
        };
        let capture = |byte_length: u64| SourceCaptureFacts {
            source_artifact_id: SOURCE,
            capture_state: CanonicalCaptureState::Captured,
            sha256: Some(DIGEST),
            byte_length: Some(byte_length),
        };

        for byte_size in [
            0,
            MAX_SOURCE_CUSTODY_PAYLOAD_BYTES + 1,
            SOURCE_CAPTURE_CEILING,
        ] {
            assert_eq!(
                failure(&sized(byte_size), capture(byte_size)),
                BOUNDS,
                "{byte_size} bytes cannot be one backend object"
            );
        }
        for byte_size in [
            MIN_SOURCE_CUSTODY_PAYLOAD_BYTES,
            MAX_SOURCE_CUSTODY_PAYLOAD_BYTES,
        ] {
            assert_eq!(
                admit_capture(&sized(byte_size), capture(byte_size)),
                Ok(()),
                "{byte_size} bytes must stay sealable as one backend object"
            );
        }
    }
}
