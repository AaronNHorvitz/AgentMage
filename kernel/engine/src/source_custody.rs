//! Deterministic validation of logical source-artifact ownership and retention.
//!
//! The closed public schema owns field syntax, bounds, and required members. This module
//! owns the cross-field rules that keep one logical source artifact, its owner, its
//! retention assignment, and the one existing content-addressed payload store consistent.

use agentmage_kernel_contracts::{
    CanonicalCaptureState, RuntimeEventRetentionKind, SOURCE_ARTIFACT_CUSTODY_SCHEMA_VERSION,
    SOURCE_CUSTODY_ARTIFACT_KIND, SourceArtifactCustody, SourceCustodyState,
};

use crate::engineering_records::CanonicalRecordError;

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

/// Validates one custody record against the source artifact that owns it.
///
/// The record cannot claim a payload the source never captured, cannot retain bytes under
/// an in-memory retention class, cannot release a checkpoint-rooted reference, and cannot
/// name payload bytes that differ from the authoritative source content address.
pub fn validate_source_custody(
    custody: &SourceArtifactCustody,
    capture: SourceCaptureFacts<'_>,
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
    if let Some(binding) = &custody.binding {
        let digest_matches = Some(binding.payload_sha256.as_str()) == capture.sha256;
        let size_matches = Some(binding.byte_size) == capture.byte_length;
        if !digest_matches || !size_matches {
            return Err(error("engineering.custody.payload_mismatch", "binding"));
        }
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
        RuntimeArtifactId, RuntimeArtifactKind, RuntimeEventRetention, RuntimeRunId, SessionId,
        SourceCustodyBackend, SourceCustodyBinding, TaskId,
    };

    const DIGEST: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const BYTES: u64 = 4_096;
    const EXPIRES: u64 = 1_800_000_000_000;

    const VERSION: &str = "engineering.custody.version";
    const KIND: &str = "engineering.custody.kind_unsupported";
    const BINDING: &str = "engineering.custody.binding_state";
    const RETENTION: &str = "engineering.custody.retention_state";
    const EXPIRATION: &str = "engineering.custody.expiration";
    const REASON: &str = "engineering.custody.reason_code";
    const ROOTED: &str = "engineering.custody.checkpoint_root";
    const UNCAPTURED: &str = "engineering.custody.uncaptured_retention";
    const PAYLOAD: &str = "engineering.custody.payload_mismatch";

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

    fn failure(custody: &SourceArtifactCustody, facts: SourceCaptureFacts<'_>) -> &'static str {
        validate_source_custody(custody, facts)
            .expect_err("custody must fail closed")
            .code
    }

    #[test]
    fn active_custody_binds_the_exact_captured_content_address() {
        assert_eq!(validate_source_custody(&active(), captured()), Ok(()));
    }

    #[test]
    fn ephemeral_capture_stays_outside_the_durable_store() {
        assert_eq!(validate_source_custody(&unretained(), captured()), Ok(()));
        assert_eq!(validate_source_custody(&unretained(), missing()), Ok(()));

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
            binding: Some(binding(&"b".repeat(64), BYTES)),
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
        assert_eq!(validate_source_custody(&expiring, captured()), Ok(()));

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
            assert_eq!(validate_source_custody(&explained, captured()), Ok(()));
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
        assert_eq!(validate_source_custody(&rooted, captured()), Ok(()));

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
                assert_eq!(validate_source_custody(&custody, captured()), Ok(()));
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
        let future = SourceArtifactCustody {
            schema_version: SOURCE_ARTIFACT_CUSTODY_SCHEMA_VERSION + 1,
            ..active()
        };
        assert_eq!(failure(&future, captured()), VERSION);
    }
}
