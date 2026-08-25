//! Logical source-artifact ownership and retention over the existing artifact store.
//!
//! These contracts add no physical store. Every retained source payload is one object in
//! the existing encrypted content-addressed runtime artifact store, published under one
//! already declared [`RuntimeArtifactKind`] member and one already declared
//! [`RuntimeEventRetention`] assignment.
//!
//! Custody is its own top-level record keyed by `source_artifact_id`. It is deliberately
//! not a member of the frozen version-1 source-artifact record: adding a required member
//! to a published record without changing its schema identity would invalidate records
//! that were exactly valid under that identity.

use crate::{
    CONTRACT_SCHEMA_VERSION, RuntimeArtifactId, RuntimeArtifactKind, RuntimeEventRetention,
    RuntimeRunId, SessionId, TaskId,
};

/// Contract schema version for the source-artifact custody family.
///
/// Custody is a new record and therefore enters the shared bounded contract boundary at
/// the live kernel version. `to_canonical_json` and `from_json` are its only supported
/// encode and parse paths.
pub const SOURCE_ARTIFACT_CUSTODY_SCHEMA_VERSION: u16 = CONTRACT_SCHEMA_VERSION;

/// Closed artifact family reused for every retained source payload.
///
/// Source custody reuses one published `RuntimeArtifactKind` member rather than widening
/// that closed family, so a retained source payload is indistinguishable from any other
/// object in the same store.
pub const SOURCE_CUSTODY_ARTIFACT_KIND: RuntimeArtifactKind = RuntimeArtifactKind::GeneratedFile;

/// Physical backend permitted to retain source-artifact payload bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceCustodyBackend {
    /// The one existing encrypted content-addressed runtime artifact store.
    RuntimeArtifactStore,
}

/// Current logical custody state of one source artifact.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceCustodyState {
    /// No durable payload exists; no bytes were admitted to the store.
    NotRetained,
    /// The logical reference is current and its payload may be opened under policy.
    Active,
    /// The reference is retained for inspection but returns no payload bytes.
    Quarantined,
    /// The owner released the logical reference.
    Released,
    /// Canonical metadata records a completed payload deletion.
    Deleted,
}

/// Exact logical binding between one source artifact and its retained payload.
///
/// A binding names one existing runtime artifact. It is admitted only against that
/// artifact's own immutable manifest, so its members repeat manifest facts rather than
/// asserting them.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceCustodyBinding {
    /// Logical artifact identity in the existing store; this is not path authority.
    pub runtime_artifact_id: RuntimeArtifactId,
    /// Digest of the immutable manifest that grants meaning to this binding.
    pub manifest_sha256: String,
    /// Lowercase SHA-256 payload content address held by that store.
    pub payload_sha256: String,
    /// Exact retained payload size.
    pub byte_size: u64,
}

/// Logical ownership and retention assignment for one source artifact.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceArtifactCustody {
    /// Contract schema version.
    pub schema_version: u16,
    /// Owning source-artifact identity.
    pub source_artifact_id: String,
    /// Backend that retains payload bytes; exactly one store is representable.
    pub backend: SourceCustodyBackend,
    /// Reused closed artifact family.
    pub artifact_kind: RuntimeArtifactKind,
    /// Exact payload binding; absent while nothing durable is retained.
    #[serde(deserialize_with = "crate::serialization::deserialize_required_option")]
    pub binding: Option<SourceCustodyBinding>,
    /// Owning local session; knowledge of this identity grants no access by itself.
    pub owner_session_id: SessionId,
    /// Owning task.
    pub owner_task_id: TaskId,
    /// Owning runtime run.
    pub owner_run_id: RuntimeRunId,
    /// Exact retention assignment reusing the canonical runtime retention family.
    pub retention: RuntimeEventRetention,
    /// Current logical custody state.
    pub state: SourceCustodyState,
    /// Whether a current durable checkpoint roots this reference against release.
    pub checkpoint_rooted: bool,
    /// Stable content-free reason for a quarantined, released, or deleted reference.
    #[serde(deserialize_with = "crate::serialization::deserialize_required_option")]
    pub release_reason_code: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{MAX_CONTRACT_JSON_BYTES, RuntimeEventRetentionKind, from_json, to_canonical_json};

    const DIGEST: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const SEAL: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

    fn custody() -> SourceArtifactCustody {
        SourceArtifactCustody {
            schema_version: SOURCE_ARTIFACT_CUSTODY_SCHEMA_VERSION,
            source_artifact_id: "source-1".to_owned(),
            backend: SourceCustodyBackend::RuntimeArtifactStore,
            artifact_kind: SOURCE_CUSTODY_ARTIFACT_KIND,
            binding: Some(SourceCustodyBinding {
                runtime_artifact_id: RuntimeArtifactId::from_raw("artifact-1"),
                manifest_sha256: SEAL.to_owned(),
                payload_sha256: DIGEST.to_owned(),
                byte_size: 4_096,
            }),
            owner_session_id: SessionId::from_raw("session-1"),
            owner_task_id: TaskId::from_raw("task-1"),
            owner_run_id: RuntimeRunId::from_raw("run-1"),
            retention: RuntimeEventRetention {
                kind: RuntimeEventRetentionKind::Session,
                expires_at_epoch_ms: None,
            },
            state: SourceCustodyState::Active,
            checkpoint_rooted: false,
            release_reason_code: None,
        }
    }

    #[test]
    fn custody_bytes_are_stable_and_round_trip_through_the_shared_boundary() {
        let expected = br#"{"schema_version":2,"source_artifact_id":"source-1","backend":"runtime_artifact_store","artifact_kind":"generated_file","binding":{"runtime_artifact_id":"artifact-1","manifest_sha256":"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb","payload_sha256":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","byte_size":4096},"owner_session_id":"session-1","owner_task_id":"task-1","owner_run_id":"run-1","retention":{"kind":"session","expires_at_epoch_ms":null},"state":"active","checkpoint_rooted":false,"release_reason_code":null}"#;
        let first = to_canonical_json(&custody()).expect("custody must serialize");
        let second = to_canonical_json(&custody()).expect("custody must serialize again");
        assert_eq!(first, expected);
        assert_eq!(second, expected);
        assert_eq!(from_json::<SourceArtifactCustody>(&first), Ok(custody()));
    }

    #[test]
    fn malformed_missing_unknown_and_duplicate_custody_inputs_fail_closed() {
        let complete = br#"{"schema_version":2,"source_artifact_id":"source-1","backend":"runtime_artifact_store","artifact_kind":"generated_file","binding":null,"owner_session_id":"session-1","owner_task_id":"task-1","owner_run_id":"run-1","retention":{"kind":"ephemeral","expires_at_epoch_ms":null},"state":"not_retained","checkpoint_rooted":false,"release_reason_code":null}"#;
        assert!(from_json::<SourceArtifactCustody>(complete).is_ok());

        let cases: &[(&[u8], &str)] = &[
            (br#"{"schema_version":2"#, "contract.parse.eof"),
            (br#"{"schema_version":2}"#, "contract.field.missing"),
            (
                br#"{"schema_version":2,"source_artifact_id":"source-1","backend":"runtime_artifact_store","artifact_kind":"generated_file","binding":null,"owner_session_id":"session-1","owner_task_id":"task-1","owner_run_id":"run-1","retention":{"kind":"ephemeral","expires_at_epoch_ms":null},"state":"not_retained","checkpoint_rooted":false,"release_reason_code":null,"extra":true}"#,
                "contract.field.unknown",
            ),
            (
                br#"{"schema_version":2,"schema_version":2,"source_artifact_id":"source-1","backend":"runtime_artifact_store","artifact_kind":"generated_file","binding":null,"owner_session_id":"session-1","owner_task_id":"task-1","owner_run_id":"run-1","retention":{"kind":"ephemeral","expires_at_epoch_ms":null},"state":"not_retained","checkpoint_rooted":false,"release_reason_code":null}"#,
                "contract.field.duplicate",
            ),
            (
                br#"{"schema_version":2,"source_artifact_id":"source-1","backend":"source_artifact_store","artifact_kind":"generated_file","binding":null,"owner_session_id":"session-1","owner_task_id":"task-1","owner_run_id":"run-1","retention":{"kind":"ephemeral","expires_at_epoch_ms":null},"state":"not_retained","checkpoint_rooted":false,"release_reason_code":null}"#,
                "contract.value.unsupported",
            ),
            (
                br#"{"schema_version":2,"source_artifact_id":"source-1","backend":"runtime_artifact_store","artifact_kind":"generated_file","binding":null,"owner_session_id":"session-1","owner_task_id":"task-1","owner_run_id":"run-1","retention":{"kind":"ephemeral","expires_at_epoch_ms":null},"state":"not_retained","checkpoint_rooted":"no","release_reason_code":null}"#,
                "contract.parse.data",
            ),
        ];
        for (candidate, expected_code) in cases {
            let error =
                from_json::<SourceArtifactCustody>(candidate).expect_err("input must fail closed");
            assert_eq!(error.code, *expected_code);
            assert_eq!(error.category, crate::ErrorCategory::Validation);
            assert!(!error.message.contains("source-1"));
        }
    }

    #[test]
    fn oversized_and_unsupported_custody_versions_have_exact_error_paths() {
        let oversized = vec![b' '; MAX_CONTRACT_JSON_BYTES + 1];
        let error =
            from_json::<SourceArtifactCustody>(&oversized).expect_err("oversized input must fail");
        assert_eq!(error.code, "contract.size.exceeded");
        assert_eq!(error.category, crate::ErrorCategory::Resource);

        for unsupported_version in [
            SOURCE_ARTIFACT_CUSTODY_SCHEMA_VERSION - 1,
            SOURCE_ARTIFACT_CUSTODY_SCHEMA_VERSION + 1,
        ] {
            let unsupported = SourceArtifactCustody {
                schema_version: unsupported_version,
                ..custody()
            };
            let encoded = serde_json::to_vec(&unsupported).expect("raw fixture serialization");
            let error =
                from_json::<SourceArtifactCustody>(&encoded).expect_err("version must fail");
            assert_eq!(error.code, "contract.version.unsupported");
            assert_eq!(error.field_path, ["schema_version"]);

            let error = to_canonical_json(&unsupported).expect_err("version must not serialize");
            assert_eq!(error.code, "contract.version.unsupported");
            assert_eq!(error.field_path, ["schema_version"]);
        }
    }
}
