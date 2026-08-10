//! Shared payload, validation, and error contracts.

use crate::{ErrorId, SchemaId};

/// Current schema version for the first kernel contract family.
pub const CONTRACT_SCHEMA_VERSION: u16 = 1;

/// Reference to a closed versioned schema without embedding parser authority.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SchemaReference {
    /// Stable schema identity.
    pub schema_id: SchemaId,
    /// Exact supported schema version.
    pub schema_version: u16,
    /// Lowercase SHA-256 digest of the schema bytes.
    pub schema_sha256: String,
}

/// Bounded opaque payload carried across a typed contract boundary.
///
/// The parser validates size, media type, canonical representation, and digest before a
/// payload can reach an implementation boundary.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContractPayload {
    /// Exact schema identity used to validate this payload.
    pub schema: SchemaReference,
    /// Declared media type of the payload bytes.
    pub media_type: String,
    /// Exact payload bytes.
    pub bytes: Vec<u8>,
    /// Lowercase SHA-256 digest of `bytes`.
    pub sha256: String,
}

/// Severity of a contract-validation issue.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ValidationSeverity {
    /// The candidate contract cannot be admitted.
    Error,
    /// The candidate remains representable but requires visible review.
    Warning,
}

/// One stable validation issue produced without partial admission.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ValidationIssue {
    /// Stable machine-readable diagnostic code.
    pub code: String,
    /// Diagnostic severity.
    pub severity: ValidationSeverity,
    /// Schema-relative field path; this is never an ambient filesystem path.
    pub field_path: Vec<String>,
    /// Bounded human-readable explanation that excludes raw sensitive values.
    pub message: String,
}

/// Broad category for a typed AgentMage error.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCategory {
    /// Input failed a closed contract.
    Validation,
    /// Current policy denied the requested operation.
    Policy,
    /// A required dependency was unavailable or failed.
    Dependency,
    /// A declared resource budget was exhausted.
    Resource,
    /// The operation was cancelled.
    Cancellation,
    /// The operation exceeded its deadline.
    Timeout,
    /// Completion cannot be established safely.
    Uncertain,
    /// An internal invariant failed without exposing private state.
    Internal,
}

/// Whether and under what condition an error may be retried.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RetryDisposition {
    /// Retrying the same request is prohibited.
    Never,
    /// Retry is possible only after correcting input.
    AfterCorrection,
    /// Retry is possible only after a dependency recovers.
    AfterDependencyRecovery,
    /// Retry requires an explicit new user decision.
    AfterUserDecision,
}

/// Versioned error returned at any kernel contract boundary.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContractError {
    /// Contract schema version.
    pub schema_version: u16,
    /// Stable error identity.
    pub error_id: ErrorId,
    /// Stable machine-readable error code.
    pub code: String,
    /// Error category.
    pub category: ErrorCategory,
    /// Bounded redacted explanation.
    pub message: String,
    /// Optional schema-relative field path.
    pub field_path: Vec<String>,
    /// Retry contract for the caller.
    pub retry: RetryDisposition,
    /// Optional causal error identity without embedding another raw error.
    #[serde(deserialize_with = "crate::serialization::deserialize_required_option")]
    pub caused_by: Option<ErrorId>,
}

#[cfg(test)]
mod tests {
    use super::{CONTRACT_SCHEMA_VERSION, ContractPayload, SchemaReference, ValidationSeverity};

    #[test]
    fn first_contract_family_is_explicitly_versioned() {
        assert_eq!(CONTRACT_SCHEMA_VERSION, 1);
    }

    #[test]
    fn payload_keeps_media_bytes_and_digest_separate() {
        let payload = ContractPayload {
            schema: SchemaReference {
                schema_id: crate::SchemaId::from_raw("fixture.schema"),
                schema_version: 1,
                schema_sha256: "1".repeat(64),
            },
            media_type: "application/json".to_owned(),
            bytes: br#"{"fixture":true}"#.to_vec(),
            sha256: "0".repeat(64),
        };
        assert_eq!(payload.media_type, "application/json");
        assert_eq!(payload.bytes, br#"{"fixture":true}"#);
        assert_eq!(ValidationSeverity::Error, ValidationSeverity::Error);
    }
}
