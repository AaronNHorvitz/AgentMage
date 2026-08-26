//! Deterministic JSON serialization and closed parsing boundaries.

use serde::{Deserialize, Deserializer, Serialize, de::DeserializeOwned};

use crate::{CONTRACT_SCHEMA_VERSION, ContractError, ErrorCategory, ErrorId, RetryDisposition};

/// Maximum encoded size admitted by the shared contract JSON boundary.
pub const MAX_CONTRACT_JSON_BYTES: usize = 1_048_576;

/// Compact result returned by contract-boundary operations.
pub type ContractResult<T> = Result<T, Box<ContractError>>;

/// Contract that carries an independently checkable top-level schema version.
pub trait VersionedContract: Serialize + DeserializeOwned {
    /// Returns the candidate contract's schema version.
    fn schema_version(&self) -> u16;
}

pub(crate) fn deserialize_required_option<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(deserializer)
}

macro_rules! impl_versioned_contract {
    ($($contract:ty),+ $(,)?) => {
        $(
            impl VersionedContract for $contract {
                fn schema_version(&self) -> u16 {
                    self.schema_version
                }
            }
        )+
    };
}

impl_versioned_contract!(
    crate::Action,
    crate::ApprovalRequest,
    crate::AgentProposal,
    crate::AgentRestartSnapshot,
    crate::DataSensitivityAssessment,
    crate::ActionRiskAssessment,
    crate::ModelCapabilityAssessment,
    crate::DeterministicPolicyFacts,
    crate::AdvisoryClassifierResult,
    crate::ReclassificationRequest,
    crate::VerifierCandidate,
    crate::AuthorityTransactionRecord,
    crate::BoundaryFailure,
    crate::CapabilityGrant,
    crate::CancellationSignal,
    crate::ContractError,
    crate::EvidenceReference,
    crate::ArtifactUploadChunk,
    crate::ArtifactCaptureResult,
    crate::ArtifactRangeReceipt,
    crate::ContextDeliveryReceipt,
    crate::VerifiedModelTurnResult,
    crate::ModelEndpointProfile,
    crate::ModelRouteDecision,
    crate::ToolObservation,
    crate::CapabilityManifest,
    crate::AgentLease,
    crate::ReviewFinding,
    crate::IntegrationRecord,
    crate::TeamCampaign,
    crate::EngineeringEvent,
    crate::EngineeringSessionSnapshot,
    crate::CanonicalArtifactEnvelope,
    crate::CanonicalArtifactIngestionResult,
    crate::CanonicalArtifactTransformation,
    crate::CanonicalCapabilityManifest,
    crate::CanonicalContextDeliveryReceipt,
    crate::CanonicalContextManifest,
    crate::CanonicalModelEndpointProfile,
    crate::CanonicalModelRouteDecision,
    crate::CanonicalSourceLocator,
    crate::CanonicalSourceRetention,
    crate::CanonicalStepExecutionPolicy,
    crate::CanonicalTerminalResult,
    crate::CanonicalToolObservation,
    crate::CanonicalVerificationResult,
    crate::CanonicalWorkflowCheckpoint,
    crate::CanonicalWorkflowDefinition,
    crate::CanonicalWorkflowState,
    crate::HandoffDraft,
    crate::HandoffReview,
    crate::MaterialClaimEvidenceAssignment,
    crate::ComposedContextPacket,
    crate::CheckedContextSummary,
    crate::SessionCheckpoint,
    crate::ConversationRecord,
    crate::ConversationTurn,
    crate::ConversationCompactionRecord,
    crate::EncodedModelContext,
    crate::ExactModelProfile,
    crate::ModelPickerSnapshot,
    crate::ModelContextPacket,
    crate::ModelRunRequest,
    crate::ModelRunResult,
    crate::StreamedModelFragment,
    crate::ModelProposalWireCandidate,
    crate::ClosedModelProposal,
    crate::Plan,
    crate::Prompt,
    crate::Receipt,
    crate::RenderedHandoff,
    crate::RuntimeEvent,
    crate::RuntimeApprovalChallenge,
    crate::RuntimeApprovalResponse,
    crate::RuntimeArtifactManifest,
    crate::RuntimeArtifactRef,
    crate::RuntimeAnswerEvidence,
    crate::RuntimeContinuationState,
    crate::RuntimeOutcome,
    crate::RuntimeResumeBinding,
    crate::RuntimeToolAttemptState,
    crate::RuntimeRunRequest,
    crate::Task,
    crate::ToolCall,
    crate::ToolDefinition,
    crate::ToolResult,
    crate::WorkPacket,
);

/// Serializes one supported contract into stable compact JSON bytes.
///
/// Struct declaration order and sequence order define canonical field and item order.
/// AgentMage contract types intentionally expose no map-valued fields at this boundary.
pub fn to_canonical_json<T: VersionedContract>(value: &T) -> ContractResult<Vec<u8>> {
    validate_schema_version(value.schema_version())?;
    let encoded = serde_json::to_vec(value).map_err(|_| {
        boundary_error(
            "contract.serialization.failed",
            ErrorCategory::Internal,
            "Contract serialization failed",
            Vec::new(),
        )
    })?;
    if encoded.len() > MAX_CONTRACT_JSON_BYTES {
        return Err(boundary_error(
            "contract.size.exceeded",
            ErrorCategory::Resource,
            "Encoded contract exceeds the maximum size",
            Vec::new(),
        ));
    }
    Ok(encoded)
}

/// Parses one complete JSON value into a supported closed contract.
///
/// Input is rejected before parsing when it exceeds the shared byte limit. Derived
/// `deny_unknown_fields` contracts reject extra and duplicate fields, while Serde rejects
/// missing fields, malformed values, unsupported enum variants, and trailing content.
pub fn from_json<T: VersionedContract>(input: &[u8]) -> ContractResult<T> {
    if input.len() > MAX_CONTRACT_JSON_BYTES {
        return Err(boundary_error(
            "contract.size.exceeded",
            ErrorCategory::Resource,
            "Encoded contract exceeds the maximum size",
            Vec::new(),
        ));
    }
    let value: T = serde_json::from_slice(input).map_err(parse_error)?;
    validate_schema_version(value.schema_version())?;
    Ok(value)
}

fn validate_schema_version(schema_version: u16) -> ContractResult<()> {
    if schema_version != CONTRACT_SCHEMA_VERSION {
        return Err(boundary_error(
            "contract.version.unsupported",
            ErrorCategory::Validation,
            "Contract schema version is unsupported",
            vec!["schema_version".to_owned()],
        ));
    }
    Ok(())
}

fn parse_error(error: serde_json::Error) -> Box<ContractError> {
    let data_error = error.to_string();
    let (code, category, message) = match error.classify() {
        serde_json::error::Category::Io => (
            "contract.parse.io",
            ErrorCategory::Dependency,
            "Contract input could not be read",
        ),
        serde_json::error::Category::Syntax => (
            "contract.parse.syntax",
            ErrorCategory::Validation,
            "Contract JSON syntax is invalid",
        ),
        serde_json::error::Category::Data if data_error.starts_with("missing field") => (
            "contract.field.missing",
            ErrorCategory::Validation,
            "Contract JSON is missing a required field",
        ),
        serde_json::error::Category::Data if data_error.starts_with("unknown field") => (
            "contract.field.unknown",
            ErrorCategory::Validation,
            "Contract JSON contains an unknown field",
        ),
        serde_json::error::Category::Data if data_error.starts_with("duplicate field") => (
            "contract.field.duplicate",
            ErrorCategory::Validation,
            "Contract JSON repeats a field",
        ),
        serde_json::error::Category::Data if data_error.starts_with("unknown variant") => (
            "contract.value.unsupported",
            ErrorCategory::Validation,
            "Contract JSON contains an unsupported value",
        ),
        serde_json::error::Category::Data => (
            "contract.parse.data",
            ErrorCategory::Validation,
            "Contract JSON does not match the closed schema",
        ),
        serde_json::error::Category::Eof => (
            "contract.parse.eof",
            ErrorCategory::Validation,
            "Contract JSON ended before one complete value",
        ),
    };
    boundary_error(code, category, message, Vec::new())
}

fn boundary_error(
    code: &str,
    category: ErrorCategory,
    message: &str,
    field_path: Vec<String>,
) -> Box<ContractError> {
    Box::new(ContractError {
        schema_version: CONTRACT_SCHEMA_VERSION,
        error_id: ErrorId::from_raw(format!("error:{code}")),
        code: code.to_owned(),
        category,
        message: message.to_owned(),
        field_path,
        retry: RetryDisposition::AfterCorrection,
        caused_by: None,
    })
}

#[cfg(test)]
mod tests {
    use super::{MAX_CONTRACT_JSON_BYTES, from_json, to_canonical_json};
    use crate::{CONTRACT_SCHEMA_VERSION, SessionId, Task, TaskId, TaskStatus};

    fn task() -> Task {
        Task {
            schema_version: CONTRACT_SCHEMA_VERSION,
            task_id: TaskId::from_raw("task-0001"),
            session_id: SessionId::from_raw("session-0001"),
            objective: "Inspect one synthetic fixture".to_owned(),
            acceptance_criteria: vec!["Produce evidence".to_owned()],
            constraints: vec!["Read only".to_owned()],
            status: TaskStatus::Ready,
        }
    }

    #[test]
    fn canonical_bytes_are_stable_and_round_trip() {
        let expected = br#"{"schema_version":2,"task_id":"task-0001","session_id":"session-0001","objective":"Inspect one synthetic fixture","acceptance_criteria":["Produce evidence"],"constraints":["Read only"],"status":"ready"}"#;
        let first = to_canonical_json(&task()).expect("fixture must serialize");
        let second = to_canonical_json(&task()).expect("fixture must serialize again");
        assert_eq!(first, expected);
        assert_eq!(second, expected);
        assert_eq!(from_json::<Task>(&first), Ok(task()));
    }

    #[test]
    fn malformed_missing_extra_duplicate_and_trailing_inputs_fail_closed() {
        let cases: &[(&[u8], &str)] = &[
            (br#"{"schema_version":2"#, "contract.parse.eof"),
            (br#"{"schema_version":2}"#, "contract.field.missing"),
            (
                br#"{"schema_version":2,"task_id":"task-0001","session_id":"session-0001","objective":"fixture","acceptance_criteria":[],"constraints":[],"status":"ready","extra":true}"#,
                "contract.field.unknown",
            ),
            (
                br#"{"schema_version":2,"schema_version":2,"task_id":"task-0001","session_id":"session-0001","objective":"fixture","acceptance_criteria":[],"constraints":[],"status":"ready"}"#,
                "contract.field.duplicate",
            ),
            (
                br#"{"schema_version":2,"task_id":"task-0001","session_id":"session-0001","objective":"fixture","acceptance_criteria":[],"constraints":[],"status":"unsupported"}"#,
                "contract.value.unsupported",
            ),
            (
                br#"{"schema_version":2,"task_id":"task-0001","session_id":"session-0001","objective":"fixture","acceptance_criteria":[],"constraints":[],"status":"ready"}[]"#,
                "contract.parse.syntax",
            ),
        ];
        for (candidate, expected_code) in cases {
            let error = from_json::<Task>(candidate).expect_err("candidate must fail closed");
            assert_eq!(error.code, *expected_code);
            assert_eq!(error.category, crate::ErrorCategory::Validation);
            assert!(!error.message.contains("fixture"));
        }
    }

    #[test]
    fn oversized_input_and_unsupported_versions_have_exact_error_paths() {
        let oversized = vec![b' '; MAX_CONTRACT_JSON_BYTES + 1];
        let error = from_json::<Task>(&oversized).expect_err("oversized input must fail");
        assert_eq!(error.code, "contract.size.exceeded");
        assert_eq!(error.category, crate::ErrorCategory::Resource);

        let mut unsupported = task();
        unsupported.schema_version = CONTRACT_SCHEMA_VERSION + 1;
        let encoded = serde_json::to_vec(&unsupported).expect("raw fixture serialization");
        let error = from_json::<Task>(&encoded).expect_err("version must fail");
        assert_eq!(error.code, "contract.version.unsupported");
        assert_eq!(error.field_path, ["schema_version"]);

        let error = to_canonical_json(&unsupported).expect_err("version must not serialize");
        assert_eq!(error.code, "contract.version.unsupported");
        assert_eq!(error.field_path, ["schema_version"]);

        let mut historical = task();
        historical.schema_version = 1;
        let encoded = serde_json::to_vec(&historical).expect("historical fixture serialization");
        let error = from_json::<Task>(&encoded)
            .expect_err("version 1 must not be silently reinterpreted as version 2");
        assert_eq!(error.code, "contract.version.unsupported");
        assert_eq!(error.field_path, ["schema_version"]);
    }
}
