//! Common artifact receipts and bounded local-audio transcription observations.

use std::collections::BTreeSet;

use agentmage_kernel_contracts::CONTRACT_SCHEMA_VERSION;
use serde::{Deserialize, Serialize};

use crate::word_generation::valid_identifier;
use crate::word_ooxml::word_sha256;

const MAX_REFERENCES: usize = 4_096;
const MAX_LIMITATIONS: usize = 1_024;
const MAX_SEGMENTS: usize = 100_000;
const MAX_TEXT_BYTES: usize = 64 * 1_024;

/// Closed artifact operation class.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactOperationKind {
    /// Format conversion.
    Converter,
    /// Read-only parsing or extraction.
    Parser,
    /// Deterministic or model-backed generation.
    Generator,
    /// Native or synthetic rendering observation.
    Renderer,
    /// Full-regeneration or decoded-content redaction.
    Redactor,
    /// Structural, semantic, visual, or policy verification.
    Verifier,
    /// Local audio transcription observation.
    Transcriber,
}

/// Closed fidelity disposition.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactFidelityState {
    /// Byte, value, or closed-structure exactness is demonstrated.
    Exact,
    /// A declared measured threshold passed.
    ThresholdPassed,
    /// Output is useful but carries explicit limitations.
    Limited,
    /// Required fidelity evidence is absent or failed.
    Blocked,
}

/// Closed source locator class used by common receipts.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactSourceLocationKind {
    /// Half-open source byte range.
    ByteRange,
    /// Inclusive one-based line range.
    LineRange,
    /// Inclusive one-based page range.
    PageRange,
    /// Inclusive one-based slide range.
    SlideRange,
    /// Zero-based notebook cell range.
    CellRange,
    /// Inclusive one-based row range.
    RowRange,
    /// Half-open audio millisecond range.
    TimeRangeMilliseconds,
    /// Whole source artifact.
    WholeArtifact,
}

/// Content-addressed input/output reference.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArtifactReceiptReference {
    /// Stable reference identity.
    pub reference_id: String,
    /// Exact content digest.
    pub sha256: String,
    /// Exact byte count when the artifact is materialized.
    pub byte_count: u64,
    /// Locator coordinate system.
    pub location_kind: ArtifactSourceLocationKind,
    /// Inclusive/half-open first coordinate according to `location_kind`.
    pub start: Option<u64>,
    /// Inclusive/half-open second coordinate according to `location_kind`.
    pub end: Option<u64>,
}

/// Request for one common artifact receipt.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CommonArtifactReceiptRequest {
    /// Stable operation identity.
    pub operation_id: String,
    /// Closed operation class.
    pub operation_kind: ArtifactOperationKind,
    /// Stable implementation identity.
    pub implementation_id: String,
    /// Exact implementation version.
    pub implementation_version: String,
    /// SHA-256 of the admitted implementation artifact or source identity.
    pub implementation_sha256: String,
    /// SHA-256 of the complete policy/profile.
    pub profile_sha256: String,
    /// Canonically ordered source references.
    pub inputs: Vec<ArtifactReceiptReference>,
    /// Canonically ordered output references.
    pub outputs: Vec<ArtifactReceiptReference>,
    /// Declared fidelity disposition.
    pub fidelity_state: ArtifactFidelityState,
    /// Canonically ordered content-free limitation codes.
    pub limitations: Vec<String>,
    /// Canonically ordered content-free unsupported-feature codes.
    pub unsupported_features: Vec<String>,
    /// True only when every source input remained invariant.
    pub source_invariant: bool,
    /// True only when declared temporary state was reconciled or no temporary state existed.
    pub cleanup_verified: bool,
    /// Whether a separately authorized file effect occurred outside the receipt builder.
    pub filesystem_effect_observed: bool,
    /// Whether a separately authorized network effect occurred outside the receipt builder.
    pub network_effect_observed: bool,
    /// Whether admitted content execution occurred outside the receipt builder.
    pub content_execution_observed: bool,
}

/// Verifiable common receipt shared by all artifact-operation classes.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CommonArtifactReceipt {
    /// Kernel contract schema version.
    pub schema_version: u16,
    /// Stable operation identity.
    pub operation_id: String,
    /// Closed operation class.
    pub operation_kind: ArtifactOperationKind,
    /// Stable implementation identity.
    pub implementation_id: String,
    /// Exact implementation version.
    pub implementation_version: String,
    /// Exact implementation identity hash.
    pub implementation_sha256: String,
    /// Exact policy/profile digest.
    pub profile_sha256: String,
    /// Canonically ordered inputs.
    pub inputs: Vec<ArtifactReceiptReference>,
    /// Canonically ordered outputs.
    pub outputs: Vec<ArtifactReceiptReference>,
    /// Fidelity disposition.
    pub fidelity_state: ArtifactFidelityState,
    /// Canonically ordered limitation codes.
    pub limitations: Vec<String>,
    /// Canonically ordered unsupported-feature codes.
    pub unsupported_features: Vec<String>,
    /// Exact source-invariance observation.
    pub source_invariant: bool,
    /// Exact cleanup observation.
    pub cleanup_verified: bool,
    /// Externally observed file effect.
    pub filesystem_effect_observed: bool,
    /// Externally observed network effect.
    pub network_effect_observed: bool,
    /// Externally observed content execution.
    pub content_execution_observed: bool,
    /// SHA-256 of every preceding field in canonical JSON form.
    pub receipt_sha256: String,
}

/// Original-audio retention disposition.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OriginalAudioRetention {
    /// Keep the original under its existing owner and policy.
    RetainOriginal,
    /// Propose deletion only after separate explicit approval.
    DeleteAfterApproval,
    /// The caller owns retention outside AgentMage; no copy is created.
    CallerManaged,
}

/// One exact transcription segment supplied by an admitted local engine adapter.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AudioTranscriptionSegment {
    /// Zero-based segment order.
    pub segment_index: u32,
    /// Inclusive audio start in milliseconds.
    pub start_milliseconds: u64,
    /// Exclusive audio end in milliseconds.
    pub end_milliseconds: u64,
    /// Observed transcript text.
    pub text: String,
    /// Optional engine-supplied speaker label.
    pub speaker_label: Option<String>,
    /// True when the speaker label is inferred rather than observed metadata.
    pub speaker_inferred: bool,
    /// Optional integer speaker confidence in parts per million.
    pub speaker_confidence_ppm: Option<u32>,
    /// Integer text confidence in parts per million.
    pub text_confidence_ppm: u32,
    /// True when unclear language is explicitly retained.
    pub unclear_language: bool,
}

/// Caller-supplied output from one exact admitted local transcriber.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AudioTranscriptionObservation {
    /// Stable observation identity.
    pub observation_id: String,
    /// Exact source audio digest.
    pub source_audio_sha256: String,
    /// Source duration in milliseconds.
    pub source_duration_milliseconds: u64,
    /// Declared source codec.
    pub source_codec: String,
    /// Stable local engine identity.
    pub engine_id: String,
    /// Exact engine version.
    pub engine_version: String,
    /// SHA-256 of the admitted engine artifact.
    pub engine_sha256: String,
    /// SHA-256 of the complete transcription profile.
    pub profile_sha256: String,
    /// Ordered source-time-cited segments.
    pub segments: Vec<AudioTranscriptionSegment>,
    /// Original-audio retention disposition.
    pub original_audio_retention: OriginalAudioRetention,
    /// True only when the adapter observed a local engine route.
    pub local_engine_observed: bool,
    /// False because validation does not itself execute the transcriber.
    pub transcription_executed_by_validator: bool,
    /// False because validation uses no network.
    pub network_access_performed: bool,
    /// False because validation does not persist or delete audio.
    pub filesystem_effect_performed: bool,
}

/// Validated audio observation and its transcriber-class receipt.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ValidatedAudioTranscription {
    /// Exact validated observation.
    pub observation: AudioTranscriptionObservation,
    /// Common artifact receipt.
    pub receipt: CommonArtifactReceipt,
    /// True because labeled accuracy evidence remains separately required.
    pub accuracy_evidence_required: bool,
}

/// Common receipt or audio-observation failure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArtifactReceiptError {
    /// An identity, digest, version, locator, ordering, or observation is invalid.
    InvalidInput,
    /// A receipt or observation exceeds a compiled resource ceiling.
    ResourceLimit,
    /// Fidelity state contradicts its limitation or unsupported-feature inventory.
    FidelityMismatch,
    /// Receipt recomputation differs from the retained digest.
    ReceiptMismatch,
}

impl ArtifactReceiptError {
    /// Stable content-free error code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidInput => "artifact_receipt.input.invalid",
            Self::ResourceLimit => "artifact_receipt.resource.limit",
            Self::FidelityMismatch => "artifact_receipt.fidelity.mismatch",
            Self::ReceiptMismatch => "artifact_receipt.digest.mismatch",
        }
    }
}

impl std::fmt::Display for ArtifactReceiptError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.code())
    }
}

impl std::error::Error for ArtifactReceiptError {}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}

fn valid_version(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value.is_ascii()
        && !value.bytes().any(|byte| byte.is_ascii_control())
}

fn valid_codes(values: &[String], maximum: usize) -> bool {
    values.len() <= maximum
        && values.iter().all(|value| valid_identifier(value))
        && values.windows(2).all(|pair| pair[0] < pair[1])
        && values.iter().collect::<BTreeSet<_>>().len() == values.len()
}

fn valid_reference(reference: &ArtifactReceiptReference) -> bool {
    let coordinates = match reference.location_kind {
        ArtifactSourceLocationKind::WholeArtifact => {
            reference.start.is_none() && reference.end.is_none()
        }
        ArtifactSourceLocationKind::ByteRange
        | ArtifactSourceLocationKind::TimeRangeMilliseconds => reference
            .start
            .zip(reference.end)
            .is_some_and(|(start, end)| start < end),
        _ => reference
            .start
            .zip(reference.end)
            .is_some_and(|(start, end)| start > 0 && start <= end),
    };
    valid_identifier(&reference.reference_id) && valid_sha256(&reference.sha256) && coordinates
}

fn validate_request(request: &CommonArtifactReceiptRequest) -> Result<(), ArtifactReceiptError> {
    if !valid_identifier(&request.operation_id)
        || !valid_identifier(&request.implementation_id)
        || !valid_version(&request.implementation_version)
        || !valid_sha256(&request.implementation_sha256)
        || !valid_sha256(&request.profile_sha256)
        || request.inputs.is_empty()
        || request.inputs.len() > MAX_REFERENCES
        || request.outputs.len() > MAX_REFERENCES
        || !request.inputs.iter().all(valid_reference)
        || !request.outputs.iter().all(valid_reference)
        || !request
            .inputs
            .windows(2)
            .all(|pair| pair[0].reference_id < pair[1].reference_id)
        || !request
            .outputs
            .windows(2)
            .all(|pair| pair[0].reference_id < pair[1].reference_id)
        || !valid_codes(&request.limitations, MAX_LIMITATIONS)
        || !valid_codes(&request.unsupported_features, MAX_LIMITATIONS)
    {
        return Err(ArtifactReceiptError::InvalidInput);
    }
    if matches!(request.fidelity_state, ArtifactFidelityState::Exact)
        && (!request.limitations.is_empty() || !request.unsupported_features.is_empty())
        || matches!(
            request.fidelity_state,
            ArtifactFidelityState::Limited | ArtifactFidelityState::Blocked
        ) && request.limitations.is_empty()
    {
        return Err(ArtifactReceiptError::FidelityMismatch);
    }
    Ok(())
}

fn receipt_digest(receipt: &CommonArtifactReceipt) -> Result<String, ArtifactReceiptError> {
    #[derive(Serialize)]
    struct Binding<'a> {
        schema_version: u16,
        operation_id: &'a str,
        operation_kind: ArtifactOperationKind,
        implementation_id: &'a str,
        implementation_version: &'a str,
        implementation_sha256: &'a str,
        profile_sha256: &'a str,
        inputs: &'a [ArtifactReceiptReference],
        outputs: &'a [ArtifactReceiptReference],
        fidelity_state: ArtifactFidelityState,
        limitations: &'a [String],
        unsupported_features: &'a [String],
        source_invariant: bool,
        cleanup_verified: bool,
        filesystem_effect_observed: bool,
        network_effect_observed: bool,
        content_execution_observed: bool,
    }
    serde_json::to_vec(&Binding {
        schema_version: receipt.schema_version,
        operation_id: &receipt.operation_id,
        operation_kind: receipt.operation_kind,
        implementation_id: &receipt.implementation_id,
        implementation_version: &receipt.implementation_version,
        implementation_sha256: &receipt.implementation_sha256,
        profile_sha256: &receipt.profile_sha256,
        inputs: &receipt.inputs,
        outputs: &receipt.outputs,
        fidelity_state: receipt.fidelity_state,
        limitations: &receipt.limitations,
        unsupported_features: &receipt.unsupported_features,
        source_invariant: receipt.source_invariant,
        cleanup_verified: receipt.cleanup_verified,
        filesystem_effect_observed: receipt.filesystem_effect_observed,
        network_effect_observed: receipt.network_effect_observed,
        content_execution_observed: receipt.content_execution_observed,
    })
    .map(|bytes| word_sha256(&bytes))
    .map_err(|_| ArtifactReceiptError::InvalidInput)
}

/// Builds a common receipt without performing or inventing any artifact effect.
pub fn build_common_artifact_receipt(
    request: &CommonArtifactReceiptRequest,
) -> Result<CommonArtifactReceipt, ArtifactReceiptError> {
    validate_request(request)?;
    let mut receipt = CommonArtifactReceipt {
        schema_version: CONTRACT_SCHEMA_VERSION,
        operation_id: request.operation_id.clone(),
        operation_kind: request.operation_kind,
        implementation_id: request.implementation_id.clone(),
        implementation_version: request.implementation_version.clone(),
        implementation_sha256: request.implementation_sha256.clone(),
        profile_sha256: request.profile_sha256.clone(),
        inputs: request.inputs.clone(),
        outputs: request.outputs.clone(),
        fidelity_state: request.fidelity_state,
        limitations: request.limitations.clone(),
        unsupported_features: request.unsupported_features.clone(),
        source_invariant: request.source_invariant,
        cleanup_verified: request.cleanup_verified,
        filesystem_effect_observed: request.filesystem_effect_observed,
        network_effect_observed: request.network_effect_observed,
        content_execution_observed: request.content_execution_observed,
        receipt_sha256: String::new(),
    };
    receipt.receipt_sha256 = receipt_digest(&receipt)?;
    Ok(receipt)
}

/// Verifies every field and the self-digest of a retained common receipt.
pub fn verify_common_artifact_receipt(
    receipt: &CommonArtifactReceipt,
) -> Result<(), ArtifactReceiptError> {
    let request = CommonArtifactReceiptRequest {
        operation_id: receipt.operation_id.clone(),
        operation_kind: receipt.operation_kind,
        implementation_id: receipt.implementation_id.clone(),
        implementation_version: receipt.implementation_version.clone(),
        implementation_sha256: receipt.implementation_sha256.clone(),
        profile_sha256: receipt.profile_sha256.clone(),
        inputs: receipt.inputs.clone(),
        outputs: receipt.outputs.clone(),
        fidelity_state: receipt.fidelity_state,
        limitations: receipt.limitations.clone(),
        unsupported_features: receipt.unsupported_features.clone(),
        source_invariant: receipt.source_invariant,
        cleanup_verified: receipt.cleanup_verified,
        filesystem_effect_observed: receipt.filesystem_effect_observed,
        network_effect_observed: receipt.network_effect_observed,
        content_execution_observed: receipt.content_execution_observed,
    };
    validate_request(&request)?;
    if receipt.schema_version != CONTRACT_SCHEMA_VERSION
        || !valid_sha256(&receipt.receipt_sha256)
        || receipt.receipt_sha256 != receipt_digest(receipt)?
    {
        return Err(ArtifactReceiptError::ReceiptMismatch);
    }
    Ok(())
}

/// Validates a caller-supplied local transcription observation and binds it to a common receipt.
pub fn validate_audio_transcription_observation(
    observation: AudioTranscriptionObservation,
) -> Result<ValidatedAudioTranscription, ArtifactReceiptError> {
    if !valid_identifier(&observation.observation_id)
        || !valid_sha256(&observation.source_audio_sha256)
        || observation.source_duration_milliseconds == 0
        || !valid_identifier(&observation.source_codec)
        || !valid_identifier(&observation.engine_id)
        || !valid_version(&observation.engine_version)
        || !valid_sha256(&observation.engine_sha256)
        || !valid_sha256(&observation.profile_sha256)
        || observation.segments.is_empty()
        || observation.segments.len() > MAX_SEGMENTS
        || !observation.local_engine_observed
        || observation.transcription_executed_by_validator
        || observation.network_access_performed
        || observation.filesystem_effect_performed
    {
        return Err(ArtifactReceiptError::InvalidInput);
    }
    let mut previous_end = 0_u64;
    for (index, segment) in observation.segments.iter().enumerate() {
        if segment.segment_index != u32::try_from(index).unwrap_or(u32::MAX)
            || segment.start_milliseconds < previous_end
            || segment.start_milliseconds >= segment.end_milliseconds
            || segment.end_milliseconds > observation.source_duration_milliseconds
            || segment.text.trim().is_empty()
            || segment.text.len() > MAX_TEXT_BYTES
            || segment.text_confidence_ppm > 1_000_000
            || segment
                .speaker_confidence_ppm
                .is_some_and(|value| value > 1_000_000)
            || segment
                .speaker_label
                .as_deref()
                .is_some_and(|value| !valid_identifier(value))
            || (segment.speaker_label.is_none()
                && (segment.speaker_inferred || segment.speaker_confidence_ppm.is_some()))
            || (segment.speaker_inferred && segment.speaker_confidence_ppm.is_none())
        {
            return Err(ArtifactReceiptError::InvalidInput);
        }
        previous_end = segment.end_milliseconds;
    }
    let output_bytes = serde_json::to_vec(&observation.segments)
        .map_err(|_| ArtifactReceiptError::InvalidInput)?;
    let receipt = build_common_artifact_receipt(&CommonArtifactReceiptRequest {
        operation_id: observation.observation_id.clone(),
        operation_kind: ArtifactOperationKind::Transcriber,
        implementation_id: observation.engine_id.clone(),
        implementation_version: observation.engine_version.clone(),
        implementation_sha256: observation.engine_sha256.clone(),
        profile_sha256: observation.profile_sha256.clone(),
        inputs: vec![ArtifactReceiptReference {
            reference_id: "source-audio".to_owned(),
            sha256: observation.source_audio_sha256.clone(),
            byte_count: 0,
            location_kind: ArtifactSourceLocationKind::TimeRangeMilliseconds,
            start: Some(0),
            end: Some(observation.source_duration_milliseconds),
        }],
        outputs: vec![ArtifactReceiptReference {
            reference_id: "transcript-segments".to_owned(),
            sha256: word_sha256(&output_bytes),
            byte_count: u64::try_from(output_bytes.len()).unwrap_or(u64::MAX),
            location_kind: ArtifactSourceLocationKind::WholeArtifact,
            start: None,
            end: None,
        }],
        fidelity_state: ArtifactFidelityState::Limited,
        limitations: vec!["audio.accuracy-labeled-evidence-required".to_owned()],
        unsupported_features: Vec::new(),
        source_invariant: true,
        cleanup_verified: true,
        filesystem_effect_observed: false,
        network_effect_observed: false,
        content_execution_observed: false,
    })?;
    Ok(ValidatedAudioTranscription {
        observation,
        receipt,
        accuracy_evidence_required: true,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reference(id: &str, hash: char) -> ArtifactReceiptReference {
        ArtifactReceiptReference {
            reference_id: id.to_owned(),
            sha256: hash.to_string().repeat(64),
            byte_count: 10,
            location_kind: ArtifactSourceLocationKind::WholeArtifact,
            start: None,
            end: None,
        }
    }

    fn request(kind: ArtifactOperationKind) -> CommonArtifactReceiptRequest {
        CommonArtifactReceiptRequest {
            operation_id: "operation-1".to_owned(),
            operation_kind: kind,
            implementation_id: "implementation-1".to_owned(),
            implementation_version: "1.0.0".to_owned(),
            implementation_sha256: "a".repeat(64),
            profile_sha256: "b".repeat(64),
            inputs: vec![reference("input-1", 'c')],
            outputs: vec![reference("output-1", 'd')],
            fidelity_state: ArtifactFidelityState::Exact,
            limitations: Vec::new(),
            unsupported_features: Vec::new(),
            source_invariant: true,
            cleanup_verified: true,
            filesystem_effect_observed: false,
            network_effect_observed: false,
            content_execution_observed: false,
        }
    }

    #[test]
    fn every_operation_class_uses_one_exact_self_verifying_receipt() {
        for kind in [
            ArtifactOperationKind::Converter,
            ArtifactOperationKind::Parser,
            ArtifactOperationKind::Generator,
            ArtifactOperationKind::Renderer,
            ArtifactOperationKind::Redactor,
            ArtifactOperationKind::Verifier,
            ArtifactOperationKind::Transcriber,
        ] {
            let receipt = build_common_artifact_receipt(&request(kind)).expect("receipt");
            verify_common_artifact_receipt(&receipt).expect("verify");
            assert!(receipt.source_invariant && receipt.cleanup_verified);
        }
    }

    #[test]
    fn receipt_rejects_stale_hash_order_locator_and_fidelity_drift() {
        let mut receipt = build_common_artifact_receipt(&request(ArtifactOperationKind::Parser))
            .expect("receipt");
        receipt.outputs[0].byte_count += 1;
        assert_eq!(
            verify_common_artifact_receipt(&receipt),
            Err(ArtifactReceiptError::ReceiptMismatch)
        );
        let mut invalid = request(ArtifactOperationKind::Parser);
        invalid.fidelity_state = ArtifactFidelityState::Limited;
        assert_eq!(
            build_common_artifact_receipt(&invalid),
            Err(ArtifactReceiptError::FidelityMismatch)
        );
        invalid.limitations = vec!["parser.limited".to_owned()];
        invalid.inputs[0].start = Some(0);
        assert_eq!(
            build_common_artifact_receipt(&invalid),
            Err(ArtifactReceiptError::InvalidInput)
        );
    }

    #[test]
    fn audio_observation_preserves_time_speaker_uncertainty_and_retention_truth() {
        let observation = AudioTranscriptionObservation {
            observation_id: "audio-1".to_owned(),
            source_audio_sha256: "a".repeat(64),
            source_duration_milliseconds: 2_000,
            source_codec: "wav-pcm16".to_owned(),
            engine_id: "local-engine".to_owned(),
            engine_version: "1.0.0".to_owned(),
            engine_sha256: "b".repeat(64),
            profile_sha256: "c".repeat(64),
            segments: vec![AudioTranscriptionSegment {
                segment_index: 0,
                start_milliseconds: 250,
                end_milliseconds: 1_500,
                text: "[unclear] status".to_owned(),
                speaker_label: Some("speaker-1".to_owned()),
                speaker_inferred: true,
                speaker_confidence_ppm: Some(550_000),
                text_confidence_ppm: 600_000,
                unclear_language: true,
            }],
            original_audio_retention: OriginalAudioRetention::RetainOriginal,
            local_engine_observed: true,
            transcription_executed_by_validator: false,
            network_access_performed: false,
            filesystem_effect_performed: false,
        };
        let validated = validate_audio_transcription_observation(observation).expect("validate");
        assert_eq!(
            validated.receipt.operation_kind,
            ArtifactOperationKind::Transcriber
        );
        assert!(validated.accuracy_evidence_required);
        assert_eq!(validated.observation.segments[0].start_milliseconds, 250);
        assert!(validated.observation.segments[0].speaker_inferred);
    }

    #[test]
    fn audio_observation_rejects_invented_order_time_speaker_and_effect_state() {
        let base = AudioTranscriptionObservation {
            observation_id: "audio-1".to_owned(),
            source_audio_sha256: "a".repeat(64),
            source_duration_milliseconds: 1_000,
            source_codec: "wav-pcm16".to_owned(),
            engine_id: "local-engine".to_owned(),
            engine_version: "1.0".to_owned(),
            engine_sha256: "b".repeat(64),
            profile_sha256: "c".repeat(64),
            segments: vec![AudioTranscriptionSegment {
                segment_index: 0,
                start_milliseconds: 0,
                end_milliseconds: 500,
                text: "observed".to_owned(),
                speaker_label: None,
                speaker_inferred: false,
                speaker_confidence_ppm: None,
                text_confidence_ppm: 500_000,
                unclear_language: false,
            }],
            original_audio_retention: OriginalAudioRetention::CallerManaged,
            local_engine_observed: true,
            transcription_executed_by_validator: false,
            network_access_performed: false,
            filesystem_effect_performed: false,
        };
        let mut changed = base.clone();
        changed.segments[0].end_milliseconds = 1_001;
        assert!(validate_audio_transcription_observation(changed).is_err());
        let mut changed = base.clone();
        changed.segments[0].speaker_inferred = true;
        assert!(validate_audio_transcription_observation(changed).is_err());
        let mut changed = base;
        changed.network_access_performed = true;
        assert!(validate_audio_transcription_observation(changed).is_err());
    }
}
