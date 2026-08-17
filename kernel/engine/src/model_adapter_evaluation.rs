//! Exact, non-activating evaluation of alternate local model-runtime adapters.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const ZERO_SHA256: &str = "0000000000000000000000000000000000000000000000000000000000000000";
const MAX_REASONS: usize = 32;

/// Closed alternate local-runtime family under evaluation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AlternateRuntimeKind {
    /// One exact pinned Docker Model Runner topology.
    DockerModelRunner,
    /// One exact pinned Ollama topology, never an arbitrary daemon registration.
    Ollama,
    /// One exact pinned vLLM topology, never an arbitrary daemon registration.
    Vllm,
    /// One exact pinned OpenAI-compatible implementation and endpoint contract.
    OpenAiCompatible,
}

/// Complete evidence tuple required before an alternate runtime can enter profile admission.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AlternateRuntimeEvidence {
    /// Stable candidate profile identity.
    pub candidate_id: String,
    /// Closed runtime family.
    pub kind: AlternateRuntimeKind,
    /// Digest of the exact adapter/runtime artifact identity.
    pub runtime_identity_sha256: String,
    /// Digest of the exact authenticated endpoint identity and transport.
    pub endpoint_identity_sha256: String,
    /// Digest of executable, owner, launcher, and descendant-process evidence.
    pub process_identity_sha256: String,
    /// Digest of listener, namespace, route, caller, and zero-egress evidence.
    pub network_boundary_sha256: String,
    /// Digest of the exact tokenizer, template, parser, and family codec evidence.
    pub codec_sha256: String,
    /// Digest of publisher, lineage, license, origin, and artifact provenance.
    pub provenance_sha256: String,
    /// Digest of the measured operation and proposal capability closure.
    pub capability_sha256: String,
    /// Digest of context-window, token-count, truncation, and budget evidence.
    pub context_sha256: String,
    /// Digest of ordered stream parsing, terminal framing, and output bounds.
    pub streaming_sha256: String,
    /// Digest of cancellation propagation and owned-descendant cleanup evidence.
    pub cancellation_sha256: String,
    /// Digest of memory, accelerator, CPU, duration, and concurrency evidence.
    pub resources_sha256: String,
    /// Digest of native-reference semantic and failure-behavior parity evidence.
    pub parity_sha256: String,
    /// Digest of install, load, health, unload, restart, and rollback evidence.
    pub lifecycle_sha256: String,
    /// Digest proving configuration and package removal leave no active endpoint or authority.
    pub removal_sha256: String,
    /// Digest of exact-profile role quality and reliability evidence.
    pub quality_sha256: String,
    /// The adapter implements the shared `LocalModelRuntime` contract without a side path.
    pub local_model_runtime_implemented: bool,
    /// The endpoint is one immutable reviewed tuple, not arbitrary user-supplied compatibility.
    pub generic_endpoint_registration_absent: bool,
    /// Raw runtime access is unavailable to shells, tools, extensions, and unrelated processes.
    pub prohibited_callers_blocked: bool,
    /// The runtime receives no workspace, tool, grant, credential, or authority material.
    pub authority_inputs_absent: bool,
    /// Runtime execution has no acquisition, telemetry, tracking, or other egress path.
    pub zero_egress_verified: bool,
    /// Streaming and cancellation behavior matches the shared runtime contract.
    pub runtime_contract_parity_passed: bool,
    /// Install, recovery, unload, rollback, and removal campaigns passed.
    pub lifecycle_and_removal_passed: bool,
    /// The exact model/runtime tuple passed its declared role-quality thresholds.
    pub role_quality_passed: bool,
    /// Digest of the evidence generation and policy revision used by this evaluation.
    pub evidence_generation_sha256: String,
}

/// Non-activating disposition produced by alternate-runtime evaluation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AlternateRuntimeDisposition {
    /// The tuple may proceed to separate model-profile admission and benchmark review.
    EligibleForProfileAdmissionReview,
    /// The tuple is unavailable to configuration and routing.
    Rejected,
}

/// Reproducible content-minimized alternate-runtime evaluation record.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AlternateRuntimeEvaluation {
    /// Exact evaluated candidate identity.
    pub candidate_id: String,
    /// Closed evaluated runtime family.
    pub kind: AlternateRuntimeKind,
    /// Non-activating evaluation result.
    pub disposition: AlternateRuntimeDisposition,
    /// Stable ordered content-free rejection reasons.
    pub reason_codes: Vec<String>,
    /// Exact evidence-generation identity.
    pub evidence_generation_sha256: String,
    /// This record never enables configuration or routing by itself.
    pub activation_authority: bool,
    /// Canonical digest of this record with this field zeroed.
    pub evaluation_sha256: String,
}

/// Stable malformed-evidence failure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AlternateRuntimeEvaluationError {
    /// An identity, digest, bound, or canonical record was malformed.
    InvalidEvidence,
}

/// Evaluates one exact alternate runtime without admitting or activating it.
pub fn evaluate_alternate_runtime(
    evidence: &AlternateRuntimeEvidence,
) -> Result<AlternateRuntimeEvaluation, AlternateRuntimeEvaluationError> {
    if !valid_identifier(&evidence.candidate_id)
        || evidence_digests(evidence)
            .iter()
            .any(|digest| !valid_sha256(digest))
    {
        return Err(AlternateRuntimeEvaluationError::InvalidEvidence);
    }

    let checks = [
        (
            evidence.local_model_runtime_implemented,
            "model.adapter.local-runtime-contract-absent",
        ),
        (
            evidence.generic_endpoint_registration_absent,
            "model.adapter.generic-endpoint-prohibited",
        ),
        (
            evidence.prohibited_callers_blocked,
            "model.adapter.prohibited-caller-reachable",
        ),
        (
            evidence.authority_inputs_absent,
            "model.adapter.authority-input-present",
        ),
        (
            evidence.zero_egress_verified,
            "model.adapter.zero-egress-unproved",
        ),
        (
            evidence.runtime_contract_parity_passed,
            "model.adapter.runtime-parity-failed",
        ),
        (
            evidence.lifecycle_and_removal_passed,
            "model.adapter.lifecycle-removal-failed",
        ),
        (
            evidence.role_quality_passed,
            "model.adapter.role-quality-failed",
        ),
    ];
    let reason_codes = checks
        .into_iter()
        .filter_map(|(passed, code)| (!passed).then_some(code.to_owned()))
        .collect::<Vec<_>>();
    if reason_codes.len() > MAX_REASONS {
        return Err(AlternateRuntimeEvaluationError::InvalidEvidence);
    }
    let disposition = if reason_codes.is_empty() {
        AlternateRuntimeDisposition::EligibleForProfileAdmissionReview
    } else {
        AlternateRuntimeDisposition::Rejected
    };
    seal_evaluation(AlternateRuntimeEvaluation {
        candidate_id: evidence.candidate_id.clone(),
        kind: evidence.kind,
        disposition,
        reason_codes,
        evidence_generation_sha256: evidence.evidence_generation_sha256.clone(),
        activation_authority: false,
        evaluation_sha256: ZERO_SHA256.to_owned(),
    })
}

/// Verifies one retained alternate-runtime evaluation record.
pub fn verify_alternate_runtime_evaluation(
    evaluation: &AlternateRuntimeEvaluation,
) -> Result<(), AlternateRuntimeEvaluationError> {
    let sealed = seal_evaluation(evaluation.clone())?;
    if sealed != *evaluation {
        return Err(AlternateRuntimeEvaluationError::InvalidEvidence);
    }
    Ok(())
}

fn seal_evaluation(
    mut evaluation: AlternateRuntimeEvaluation,
) -> Result<AlternateRuntimeEvaluation, AlternateRuntimeEvaluationError> {
    evaluation.reason_codes.sort();
    evaluation.evaluation_sha256 = ZERO_SHA256.to_owned();
    if !valid_identifier(&evaluation.candidate_id)
        || !valid_sha256(&evaluation.evidence_generation_sha256)
        || evaluation.activation_authority
        || evaluation.reason_codes.len() > MAX_REASONS
        || evaluation
            .reason_codes
            .iter()
            .any(|code| !valid_reason(code))
        || evaluation
            .reason_codes
            .windows(2)
            .any(|pair| pair[0] == pair[1])
        || (evaluation.reason_codes.is_empty()
            != (evaluation.disposition
                == AlternateRuntimeDisposition::EligibleForProfileAdmissionReview))
    {
        return Err(AlternateRuntimeEvaluationError::InvalidEvidence);
    }
    evaluation.evaluation_sha256 = canonical_sha256(&evaluation)?;
    Ok(evaluation)
}

fn evidence_digests(evidence: &AlternateRuntimeEvidence) -> [&str; 16] {
    [
        &evidence.runtime_identity_sha256,
        &evidence.endpoint_identity_sha256,
        &evidence.process_identity_sha256,
        &evidence.network_boundary_sha256,
        &evidence.codec_sha256,
        &evidence.provenance_sha256,
        &evidence.capability_sha256,
        &evidence.context_sha256,
        &evidence.streaming_sha256,
        &evidence.cancellation_sha256,
        &evidence.resources_sha256,
        &evidence.parity_sha256,
        &evidence.lifecycle_sha256,
        &evidence.removal_sha256,
        &evidence.quality_sha256,
        &evidence.evidence_generation_sha256,
    ]
}

fn canonical_sha256(value: &impl Serialize) -> Result<String, AlternateRuntimeEvaluationError> {
    let bytes =
        serde_json::to_vec(value).map_err(|_| AlternateRuntimeEvaluationError::InvalidEvidence)?;
    Ok(Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect())
}

fn valid_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b':' | b'-'))
}

fn valid_reason(value: &str) -> bool {
    valid_identifier(value) && value.starts_with("model.adapter.")
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn evidence() -> AlternateRuntimeEvidence {
        AlternateRuntimeEvidence {
            candidate_id: "exact-local-adapter-0001".to_owned(),
            kind: AlternateRuntimeKind::DockerModelRunner,
            runtime_identity_sha256: "1".repeat(64),
            endpoint_identity_sha256: "2".repeat(64),
            process_identity_sha256: "3".repeat(64),
            network_boundary_sha256: "4".repeat(64),
            codec_sha256: "5".repeat(64),
            provenance_sha256: "6".repeat(64),
            capability_sha256: "7".repeat(64),
            context_sha256: "8".repeat(64),
            streaming_sha256: "9".repeat(64),
            cancellation_sha256: "a".repeat(64),
            resources_sha256: "b".repeat(64),
            parity_sha256: "c".repeat(64),
            lifecycle_sha256: "d".repeat(64),
            removal_sha256: "e".repeat(64),
            quality_sha256: "f".repeat(64),
            local_model_runtime_implemented: true,
            generic_endpoint_registration_absent: true,
            prohibited_callers_blocked: true,
            authority_inputs_absent: true,
            zero_egress_verified: true,
            runtime_contract_parity_passed: true,
            lifecycle_and_removal_passed: true,
            role_quality_passed: true,
            evidence_generation_sha256: "0".repeat(64),
        }
    }

    #[test]
    fn exact_complete_tuple_only_proceeds_to_separate_profile_review() {
        let evaluation = evaluate_alternate_runtime(&evidence()).expect("evaluation");
        assert_eq!(
            evaluation.disposition,
            AlternateRuntimeDisposition::EligibleForProfileAdmissionReview
        );
        assert!(!evaluation.activation_authority);
        verify_alternate_runtime_evaluation(&evaluation).expect("verified record");
    }

    #[test]
    fn every_missing_safety_or_quality_gate_rejects_without_activation() {
        let mutations: [fn(&mut AlternateRuntimeEvidence); 8] = [
            |value| value.local_model_runtime_implemented = false,
            |value| value.generic_endpoint_registration_absent = false,
            |value| value.prohibited_callers_blocked = false,
            |value| value.authority_inputs_absent = false,
            |value| value.zero_egress_verified = false,
            |value| value.runtime_contract_parity_passed = false,
            |value| value.lifecycle_and_removal_passed = false,
            |value| value.role_quality_passed = false,
        ];
        for mutate in mutations {
            let mut candidate = evidence();
            mutate(&mut candidate);
            let evaluation = evaluate_alternate_runtime(&candidate).expect("rejected evaluation");
            assert_eq!(
                evaluation.disposition,
                AlternateRuntimeDisposition::Rejected
            );
            assert_eq!(evaluation.reason_codes.len(), 1);
            assert!(!evaluation.activation_authority);
        }
    }

    #[test]
    fn malformed_or_mutated_evidence_and_records_fail_closed() {
        for mutate in [
            |value: &mut AlternateRuntimeEvidence| value.candidate_id = "../adapter".to_owned(),
            |value: &mut AlternateRuntimeEvidence| {
                value.endpoint_identity_sha256 = "mutable-endpoint".to_owned();
            },
            |value: &mut AlternateRuntimeEvidence| {
                value.evidence_generation_sha256 = "short".to_owned();
            },
        ] {
            let mut candidate = evidence();
            mutate(&mut candidate);
            assert_eq!(
                evaluate_alternate_runtime(&candidate),
                Err(AlternateRuntimeEvaluationError::InvalidEvidence)
            );
        }
        let mut retained = evaluate_alternate_runtime(&evidence()).expect("evaluation");
        retained.activation_authority = true;
        assert_eq!(
            verify_alternate_runtime_evaluation(&retained),
            Err(AlternateRuntimeEvaluationError::InvalidEvidence)
        );
    }

    #[test]
    fn current_docker_candidate_remains_rejected_for_missing_contract_and_quality() {
        let mut docker = evidence();
        docker.candidate_id = "docker-model-runner-gemma4-e4b-quarantined".to_owned();
        docker.local_model_runtime_implemented = false;
        docker.runtime_contract_parity_passed = false;
        docker.lifecycle_and_removal_passed = false;
        docker.role_quality_passed = false;
        let evaluation = evaluate_alternate_runtime(&docker).expect("rejected Docker evaluation");
        assert_eq!(
            evaluation.disposition,
            AlternateRuntimeDisposition::Rejected
        );
        assert_eq!(evaluation.reason_codes.len(), 4);
        assert!(!evaluation.activation_authority);
    }

    #[test]
    fn retained_docker_evaluation_is_canonical_rejected_and_non_activating() {
        let retained = serde_json::from_str::<AlternateRuntimeEvaluation>(include_str!(
            "../../../model-profiles/routing/alternate-runtime-evaluation-v1.json"
        ))
        .expect("retained alternate-runtime evaluation");
        verify_alternate_runtime_evaluation(&retained).expect("canonical retained evaluation");
        assert_eq!(retained.disposition, AlternateRuntimeDisposition::Rejected);
        assert!(!retained.activation_authority);
    }
}
