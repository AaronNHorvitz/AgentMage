//! Hash-bound evidence assignment for successful rendered runtime answers.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use agentmage_kernel_contracts::{
    CONTRACT_SCHEMA_VERSION, EvidenceReference, MaterialClaim, MaterialClaimEvidenceState,
    MaterialClaimKind, ModelManifestObservation, ModelRunId, RuntimeAnswerEvidence, RuntimeOutput,
    RuntimeRunRequest, to_canonical_json,
};
use sha2::{Digest, Sha256};

use crate::evidence_reconciliation::{
    AnswerClaimLedger, CitationResolution, build_answer_claim_ledger,
};
use crate::evidence_state::{DeterministicMethodRegistry, EvidenceStateAssigner};

const ZERO_SHA256: &str = "0000000000000000000000000000000000000000000000000000000000000000";
const RENDERED_CLAIM_ID: &str = "runtime.answer.content";
const ASSIGNMENT_ID: &str = "runtime.answer.assignment";
const CLAIM_SUBJECT_ID: &str = "runtime.answer";
const CLAIM_STATEMENT: &str = "Rendered model answer content";

/// Stable failure returned by runtime answer-evidence composition.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuntimeAnswerEvidenceError {
    /// A request, model, output, evidence, or assignment binding is malformed.
    InvalidBinding,
    /// Canonical serialization failed.
    Serialization,
}

/// Composes one inferred assignment for the complete exact rendered model answer.
///
/// The model supplies only inert output. The kernel creates the state label and binds it to the
/// exact admitted profile, response digest, output identity, and current evidence set.
pub fn compose_inferred_runtime_answer(
    request: &RuntimeRunRequest,
    model_run_id: ModelRunId,
    response_sha256: String,
    output_sha256: String,
    output_byte_size: u64,
    output_media_type: String,
    evidence: Vec<EvidenceReference>,
) -> Result<RuntimeAnswerEvidence, RuntimeAnswerEvidenceError> {
    if output_byte_size == 0
        || output_byte_size > request.limits.max_output_bytes
        || !valid_sha256(&output_sha256)
        || !valid_sha256(&response_sha256)
        || !valid_media_type(&output_media_type)
        || evidence.is_empty()
    {
        return Err(RuntimeAnswerEvidenceError::InvalidBinding);
    }
    let manifest = manifest_observation(request);
    let mut assigner = EvidenceStateAssigner::new(
        request.task.task_id.clone(),
        DeterministicMethodRegistry::empty(),
    )
    .map_err(|_| RuntimeAnswerEvidenceError::InvalidBinding)?;
    let assignment = assigner
        .assign_inferred(
            ASSIGNMENT_ID,
            MaterialClaim {
                schema_version: CONTRACT_SCHEMA_VERSION,
                claim_id: RENDERED_CLAIM_ID.to_owned(),
                task_id: request.task.task_id.clone(),
                kind: MaterialClaimKind::Read,
                statement: CLAIM_STATEMENT.to_owned(),
                subject_id: CLAIM_SUBJECT_ID.to_owned(),
                expected_revision: output_sha256.clone(),
                prerequisite_claim_ids: Vec::new(),
            },
            evidence,
            model_run_id.clone(),
            manifest,
            response_sha256.clone(),
        )
        .map_err(|_| RuntimeAnswerEvidenceError::InvalidBinding)?
        .clone();
    let validated = assigner.finalize();
    if validated.assignments() != [assignment.clone()] {
        return Err(RuntimeAnswerEvidenceError::InvalidBinding);
    }
    let mut answer = RuntimeAnswerEvidence {
        schema_version: CONTRACT_SCHEMA_VERSION,
        task_id: request.task.task_id.clone(),
        model_run_id,
        response_sha256,
        output_sha256,
        output_byte_size,
        output_media_type,
        rendered_claim_ids: vec![RENDERED_CLAIM_ID.to_owned()],
        assignments: vec![assignment],
        answer_evidence_sha256: ZERO_SHA256.to_owned(),
    };
    answer.answer_evidence_sha256 = answer_evidence_digest(&answer)?;
    Ok(answer)
}

/// Verifies a successful answer assignment against its request, output, and evidence set.
#[must_use]
pub fn verify_runtime_answer_evidence(
    answer: &RuntimeAnswerEvidence,
    request: &RuntimeRunRequest,
    output: &RuntimeOutput,
    outcome_evidence: &[EvidenceReference],
) -> bool {
    let Some((output_sha256, output_byte_size, output_media_type)) = output_binding(output) else {
        return false;
    };
    if answer.schema_version != CONTRACT_SCHEMA_VERSION
        || answer.task_id != request.task.task_id
        || answer.output_sha256 != output_sha256
        || answer.output_byte_size != output_byte_size
        || answer.output_media_type != output_media_type
        || answer.output_byte_size == 0
        || answer.output_byte_size > request.limits.max_output_bytes
        || !valid_identifier(answer.model_run_id.as_str())
        || !valid_sha256(&answer.response_sha256)
        || !valid_sha256(&answer.answer_evidence_sha256)
        || answer_evidence_digest(answer).ok().as_deref()
            != Some(answer.answer_evidence_sha256.as_str())
        || answer.rendered_claim_ids != [RENDERED_CLAIM_ID]
        || answer.assignments.len() != 1
    {
        return false;
    }
    let assignment = &answer.assignments[0];
    let claim = &assignment.claim;
    if assignment.schema_version != CONTRACT_SCHEMA_VERSION
        || assignment.assignment_id != ASSIGNMENT_ID
        || claim.schema_version != CONTRACT_SCHEMA_VERSION
        || claim.claim_id != RENDERED_CLAIM_ID
        || claim.task_id != answer.task_id
        || claim.kind != MaterialClaimKind::Read
        || claim.statement != CLAIM_STATEMENT
        || claim.subject_id != CLAIM_SUBJECT_ID
        || claim.expected_revision != answer.output_sha256
        || !claim.prerequisite_claim_ids.is_empty()
    {
        return false;
    }
    let MaterialClaimEvidenceState::Inferred(provenance) = &assignment.evidence_state else {
        return false;
    };
    provenance.citations == outcome_evidence
        && !provenance.citations.is_empty()
        && provenance.runtime.model_run_id == answer.model_run_id
        && provenance.runtime.response_sha256 == answer.response_sha256
        && provenance.runtime.manifest == manifest_observation(request)
}

/// Revalidates one successful runtime answer and binds it to exact current citations.
pub fn compose_runtime_answer_claim_ledger(
    answer: &RuntimeAnswerEvidence,
    request: &RuntimeRunRequest,
    output: &RuntimeOutput,
    citations: Vec<CitationResolution>,
) -> Result<AnswerClaimLedger, RuntimeAnswerEvidenceError> {
    let outcome_evidence = citations
        .iter()
        .map(|resolution| resolution.citation.evidence.clone())
        .collect::<Vec<_>>();
    if !verify_runtime_answer_evidence(answer, request, output, &outcome_evidence) {
        return Err(RuntimeAnswerEvidenceError::InvalidBinding);
    }
    let assignment = answer
        .assignments
        .first()
        .ok_or(RuntimeAnswerEvidenceError::InvalidBinding)?;
    let MaterialClaimEvidenceState::Inferred(provenance) = &assignment.evidence_state else {
        return Err(RuntimeAnswerEvidenceError::InvalidBinding);
    };
    let mut assigner = EvidenceStateAssigner::new(
        request.task.task_id.clone(),
        DeterministicMethodRegistry::empty(),
    )
    .map_err(|_| RuntimeAnswerEvidenceError::InvalidBinding)?;
    let rebuilt = assigner
        .assign_inferred(
            assignment.assignment_id.clone(),
            assignment.claim.clone(),
            provenance.citations.clone(),
            provenance.runtime.model_run_id.clone(),
            provenance.runtime.manifest.clone(),
            provenance.runtime.response_sha256.clone(),
        )
        .map_err(|_| RuntimeAnswerEvidenceError::InvalidBinding)?;
    if rebuilt != assignment {
        return Err(RuntimeAnswerEvidenceError::InvalidBinding);
    }
    build_answer_claim_ledger(
        assigner.finalize(),
        answer.rendered_claim_ids.clone(),
        BTreeMap::from([(
            assignment.assignment_id.clone(),
            (citations, vec!["model.inference".to_owned()]),
        )]),
    )
    .map_err(|_| RuntimeAnswerEvidenceError::InvalidBinding)
}

fn manifest_observation(request: &RuntimeRunRequest) -> ModelManifestObservation {
    ModelManifestObservation {
        profile_id: request.model_profile.profile_id.clone(),
        manifest_sha256: request.model_profile.manifest_sha256.clone(),
        artifact_sha256: request.model_profile.artifact.sha256.clone(),
        tokenizer_sha256: request.model_profile.codec.tokenizer_sha256.clone(),
        template_sha256: request.model_profile.codec.template_sha256.clone(),
        codec_sha256: request.model_profile.codec.codec_sha256.clone(),
        runtime: request.model_profile.runtime.clone(),
    }
}

fn output_binding(output: &RuntimeOutput) -> Option<(&str, u64, &str)> {
    match output {
        RuntimeOutput::Inline { payload } => Some((
            payload.sha256.as_str(),
            u64::try_from(payload.bytes.len()).ok()?,
            payload.media_type.as_str(),
        )),
        RuntimeOutput::Artifact { reference } => Some((
            reference.sha256.as_str(),
            reference.byte_size,
            reference.media_type.as_str(),
        )),
    }
}

fn answer_evidence_digest(
    answer: &RuntimeAnswerEvidence,
) -> Result<String, RuntimeAnswerEvidenceError> {
    let mut preimage = answer.clone();
    preimage.answer_evidence_sha256 = ZERO_SHA256.to_owned();
    let bytes =
        to_canonical_json(&preimage).map_err(|_| RuntimeAnswerEvidenceError::Serialization)?;
    Ok(sha256(&bytes))
}

fn valid_media_type(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'/' | b'+' | b'.' | b'-'))
}

fn valid_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 256
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b':' | b'-'))
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn sha256(value: &[u8]) -> String {
    let mut output = String::with_capacity(64);
    for byte in Sha256::digest(value) {
        write!(&mut output, "{byte:02x}").expect("writing to String cannot fail");
    }
    output
}
