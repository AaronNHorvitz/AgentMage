//! Deterministic validation of logical source-artifact ownership and retention.
//!
//! The closed public schema owns field syntax, bounds, and required members. This module
//! owns the cross-field rules that keep one logical source artifact, its owner, its
//! retention assignment, and the one existing content-addressed payload store consistent.
//!
//! Custody never asserts backend facts on its own. A retained record is admitted only
//! against the authoritative [`RuntimeArtifactManifest`] of the artifact it names, so a
//! nonexistent, differently owned, differently retained, or differently sized backend
//! object cannot be described into existence by a well-formed custody record.

use agentmage_kernel_contracts::{
    CONTRACT_SCHEMA_VERSION, CanonicalCaptureState, RuntimeArtifactIntegrityState,
    RuntimeArtifactManifest, RuntimeEventRetentionKind, SOURCE_ARTIFACT_CUSTODY_SCHEMA_VERSION,
    SOURCE_CUSTODY_ARTIFACT_KIND, SourceArtifactCustody, SourceCustodyState,
};

use crate::engineering_records::CanonicalRecordError;
use crate::runtime_hardening::MAX_RUNTIME_ARTIFACT_BYTES;

/// Smallest payload the existing runtime artifact store can publish as one object.
///
/// The source-capture ceiling is deliberately wider than this backend range. A capture
/// that no single backend object can hold is representable as a source artifact and is
/// simply not retainable.
pub const MIN_SOURCE_CUSTODY_PAYLOAD_BYTES: u64 = 1;

/// Largest payload the existing runtime artifact store can publish as one object.
pub const MAX_SOURCE_CUSTODY_PAYLOAD_BYTES: u64 = MAX_RUNTIME_ARTIFACT_BYTES;

/// Exact capture facts of the source artifact that owns one custody record.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceCaptureFacts<'a> {
    /// Terminal capture state recorded for the source.
    pub capture_state: CanonicalCaptureState,
    /// Authoritative source digest, present only when bytes were captured.
    pub sha256: Option<&'a str>,
    /// Authoritative source byte length, present only when bytes were captured.
    pub byte_length: Option<u64>,
}

/// Validates one custody record against its source artifact and its backend manifest.
///
/// The record cannot claim a payload the source never captured, cannot retain bytes under
/// an in-memory retention class, cannot release a checkpoint-rooted reference, and cannot
/// name payload bytes that differ from the authoritative source content address.
///
/// `manifest` must be the verified canonical manifest of the artifact named by the
/// binding, and must be present exactly when the record retains a payload. Artifact
/// identity, manifest seal, artifact kind, payload identity, owner identities, and
/// retention assignment are reconciled exactly against it.
pub fn validate_source_custody(
    custody: &SourceArtifactCustody,
    capture: SourceCaptureFacts<'_>,
    manifest: Option<&RuntimeArtifactManifest>,
) -> Result<(), CanonicalRecordError> {
    if custody.schema_version != SOURCE_ARTIFACT_CUSTODY_SCHEMA_VERSION {
        return Err(error("engineering.custody.version", "schema_version"));
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
    if retained != manifest.is_some() {
        return Err(error("engineering.custody.manifest_state", "binding"));
    }

    let (Some(binding), Some(manifest)) = (&custody.binding, manifest) else {
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

    if manifest.schema_version != CONTRACT_SCHEMA_VERSION {
        return Err(error("engineering.custody.manifest_version", "binding"));
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
    if custody.state == SourceCustodyState::Active
        && manifest.integrity != RuntimeArtifactIntegrityState::Verified
    {
        return Err(error("engineering.custody.manifest_integrity", "state"));
    }
    Ok(())
}

const fn error(code: &'static str, field: &'static str) -> CanonicalRecordError {
    CanonicalRecordError { code, field }
}

#[cfg(test)]
mod tests {
    use super::*;
    use agentmage_kernel_contracts::{
        ContextSensitivity, PolicyId, RuntimeArtifactId, RuntimeArtifactKind,
        RuntimeEventRetention, RuntimeRunId, SessionId, SourceCustodyBackend, SourceCustodyBinding,
        TaskId,
    };

    const DIGEST: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const SEAL: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
    const POLICY: &str = "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";
    const BYTES: u64 = 4_096;
    const EXPIRES: u64 = 1_800_000_000_000;
    const CREATED: u64 = 1_700_000_000_000;
    const SOURCE_CAPTURE_CEILING: u64 = 104_857_600;

    const VERSION: &str = "engineering.custody.version";
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
    const MANIFEST_IDENTITY: &str = "engineering.custody.manifest_identity";
    const MANIFEST_SEAL: &str = "engineering.custody.manifest_seal";
    const MANIFEST_KIND: &str = "engineering.custody.manifest_kind";
    const MANIFEST_PAYLOAD: &str = "engineering.custody.manifest_payload";
    const MANIFEST_OWNER: &str = "engineering.custody.manifest_owner";
    const MANIFEST_RETENTION: &str = "engineering.custody.manifest_retention";
    const MANIFEST_INTEGRITY: &str = "engineering.custody.manifest_integrity";

    fn captured() -> SourceCaptureFacts<'static> {
        SourceCaptureFacts {
            capture_state: CanonicalCaptureState::Captured,
            sha256: Some(DIGEST),
            byte_length: Some(BYTES),
        }
    }

    fn missing() -> SourceCaptureFacts<'static> {
        SourceCaptureFacts {
            capture_state: CanonicalCaptureState::Unavailable,
            sha256: None,
            byte_length: None,
        }
    }

    fn binding(payload_sha256: &str, byte_size: u64) -> SourceCustodyBinding {
        SourceCustodyBinding {
            runtime_artifact_id: RuntimeArtifactId::from_raw("artifact:1"),
            manifest_sha256: SEAL.to_owned(),
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
            source_artifact_id: "source:1".to_owned(),
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

    /// Returns the exact canonical manifest of the artifact one custody record names.
    fn sealed_manifest(custody: &SourceArtifactCustody) -> RuntimeArtifactManifest {
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
            manifest_sha256: binding.manifest_sha256,
        }
    }

    /// Admits one custody record against its own exact manifest and captured source.
    fn admit(custody: &SourceArtifactCustody) -> Result<(), CanonicalRecordError> {
        admit_capture(custody, captured())
    }

    fn admit_capture(
        custody: &SourceArtifactCustody,
        capture: SourceCaptureFacts<'_>,
    ) -> Result<(), CanonicalRecordError> {
        let manifest = custody.binding.as_ref().map(|_| sealed_manifest(custody));
        validate_source_custody(custody, capture, manifest.as_ref())
    }

    fn failure(custody: &SourceArtifactCustody, capture: SourceCaptureFacts<'_>) -> &'static str {
        admit_capture(custody, capture)
            .expect_err("custody must fail closed")
            .code
    }

    fn manifest_failure(
        custody: &SourceArtifactCustody,
        manifest: &RuntimeArtifactManifest,
    ) -> &'static str {
        validate_source_custody(custody, captured(), Some(manifest))
            .expect_err("custody must fail closed")
            .code
    }

    #[test]
    fn active_custody_binds_the_exact_captured_content_address() {
        assert_eq!(admit(&active()), Ok(()));
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

            let explained = SourceArtifactCustody {
                state,
                release_reason_code: Some("owner-release".to_owned()),
                ..active()
            };
            assert_eq!(admit(&explained), Ok(()));
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
                state,
                release_reason_code: Some("owner-release".to_owned()),
                ..active()
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
    fn custody_admission_requires_the_exact_verified_runtime_manifest() {
        let custody = active();
        let sealed = sealed_manifest(&custody);
        assert_eq!(
            validate_source_custody(&custody, captured(), Some(&sealed)),
            Ok(())
        );

        assert_eq!(
            validate_source_custody(&custody, captured(), None)
                .expect_err("an unbacked retention must fail closed")
                .code,
            MANIFEST_STATE
        );
        assert_eq!(
            validate_source_custody(&unretained(), captured(), Some(&sealed))
                .expect_err("an unretained record owns no backend object")
                .code,
            MANIFEST_STATE
        );

        let drifted = RuntimeArtifactManifest {
            schema_version: CONTRACT_SCHEMA_VERSION + 1,
            ..sealed.clone()
        };
        assert_eq!(manifest_failure(&custody, &drifted), MANIFEST_VERSION);

        let other_id = RuntimeArtifactManifest {
            artifact_id: RuntimeArtifactId::from_raw("artifact:2"),
            ..sealed.clone()
        };
        assert_eq!(manifest_failure(&custody, &other_id), MANIFEST_IDENTITY);

        let unsealed = RuntimeArtifactManifest {
            manifest_sha256: "e".repeat(64),
            ..sealed.clone()
        };
        assert_eq!(manifest_failure(&custody, &unsealed), MANIFEST_SEAL);

        let other_kind = RuntimeArtifactManifest {
            kind: RuntimeArtifactKind::Patch,
            ..sealed.clone()
        };
        assert_eq!(manifest_failure(&custody, &other_kind), MANIFEST_KIND);

        let other_digest = RuntimeArtifactManifest {
            payload_sha256: "f".repeat(64),
            ..sealed.clone()
        };
        assert_eq!(manifest_failure(&custody, &other_digest), MANIFEST_PAYLOAD);

        let other_size = RuntimeArtifactManifest {
            byte_size: BYTES + 1,
            ..sealed.clone()
        };
        assert_eq!(manifest_failure(&custody, &other_size), MANIFEST_PAYLOAD);

        let other_session = RuntimeArtifactManifest {
            session_id: SessionId::from_raw("session:2"),
            ..sealed.clone()
        };
        assert_eq!(manifest_failure(&custody, &other_session), MANIFEST_OWNER);

        let other_task = RuntimeArtifactManifest {
            task_id: TaskId::from_raw("task:2"),
            ..sealed.clone()
        };
        assert_eq!(manifest_failure(&custody, &other_task), MANIFEST_OWNER);

        let other_run = RuntimeArtifactManifest {
            producer_run_id: RuntimeRunId::from_raw("run:2"),
            ..sealed.clone()
        };
        assert_eq!(manifest_failure(&custody, &other_run), MANIFEST_OWNER);

        let other_retention = RuntimeArtifactManifest {
            retention: retention(RuntimeEventRetentionKind::UserHold, None),
            ..sealed.clone()
        };
        assert_eq!(
            manifest_failure(&custody, &other_retention),
            MANIFEST_RETENTION
        );

        let unverified = RuntimeArtifactManifest {
            integrity: RuntimeArtifactIntegrityState::Missing,
            ..sealed
        };
        assert_eq!(manifest_failure(&custody, &unverified), MANIFEST_INTEGRITY);
    }

    #[test]
    fn a_retained_binding_must_be_representable_by_the_existing_backend() {
        assert_eq!(MIN_SOURCE_CUSTODY_PAYLOAD_BYTES, 1);
        assert_eq!(MAX_SOURCE_CUSTODY_PAYLOAD_BYTES, 67_108_864);
        assert!(SOURCE_CAPTURE_CEILING > MAX_SOURCE_CUSTODY_PAYLOAD_BYTES);

        let sized = |byte_size: u64| SourceArtifactCustody {
            binding: Some(binding(DIGEST, byte_size)),
            ..active()
        };
        let facts = |byte_length: u64| SourceCaptureFacts {
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
                failure(&sized(byte_size), facts(byte_size)),
                BOUNDS,
                "{byte_size} bytes cannot be one backend object"
            );
        }
        for byte_size in [
            MIN_SOURCE_CUSTODY_PAYLOAD_BYTES,
            MAX_SOURCE_CUSTODY_PAYLOAD_BYTES,
        ] {
            assert_eq!(
                admit_capture(&sized(byte_size), facts(byte_size)),
                Ok(()),
                "{byte_size} bytes must stay sealable as one backend object"
            );
        }
    }
}
