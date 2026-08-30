//! Deterministic evaluation of exact workflow completion evidence.

use std::collections::BTreeSet;
use std::fmt::Write as _;

use agentmage_kernel_contracts::{
    CanonicalStateChange, CanonicalToolObservation, CanonicalToolOutcome,
    CanonicalVerificationOutcome, CanonicalVerificationResult,
};
use sha2::{Digest, Sha256};

use crate::engineering_records::ValidateCanonicalRecord;

const ZERO_SHA256: &str = "0000000000000000000000000000000000000000000000000000000000000000";
const MAX_ITEMS: usize = 128;

/// Exact observation and receipt expected from one workflow attempt.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct RequiredWorkflowObservation {
    /// Stable observation identity.
    pub observation_id: String,
    /// Exact tool-call identity.
    pub tool_call_id: String,
    /// Exact attempt identity.
    pub attempt_id: String,
    /// Integrity digest of the terminal observation and its receipt.
    pub receipt_sha256: String,
}

/// Immutable verifier policy for one exact workflow step and current state.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct WorkflowVerifierPolicy {
    /// Owning workflow identity.
    pub workflow_id: String,
    /// Owning task identity used by every observation.
    pub task_id: String,
    /// Exact step identity.
    pub step_id: String,
    /// Digest of the expected output being verified.
    pub expected_output_sha256: String,
    /// Digest of the current state against which evidence was gathered.
    pub expected_state_sha256: String,
    /// Ordered deterministic verifier identities, one per required postcondition.
    pub required_postcondition_verifier_ids: Vec<String>,
    /// Ordered invariant identities every verifier must report as preserved.
    pub required_invariant_ids: Vec<String>,
    /// Ordered effect identities whose absence every verifier must establish.
    pub prohibited_effect_ids: Vec<String>,
    /// Exact terminal observations and receipts required by this evaluation.
    pub required_observations: Vec<RequiredWorkflowObservation>,
    /// Ordered complete evidence digest set for the current state.
    pub required_current_evidence_sha256s: Vec<String>,
    /// Digest of this policy with this field zeroed.
    pub policy_sha256: String,
}

/// Complete untrusted inputs to one deterministic workflow evaluation.
#[derive(Clone, Copy, Debug)]
pub struct WorkflowVerifierInput<'a> {
    /// Current state digest supplied by the runtime state authority.
    pub current_state_sha256: &'a str,
    /// Exact evidence digests currently available to the verifier.
    pub current_evidence_sha256s: &'a [String],
    /// Complete terminal tool observations for the step.
    pub observations: &'a [CanonicalToolObservation],
    /// Complete results from the registered deterministic postcondition verifiers.
    pub verification_results: &'a [CanonicalVerificationResult],
}

/// Stable content-free reason completion evidence was refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorkflowVerifierError {
    /// Policy identities, lists, or digests are malformed.
    InvalidPolicy,
    /// The policy digest does not bind its exact canonical preimage.
    PolicyIntegrity,
    /// Current state does not match the state named by the policy.
    StateMismatch,
    /// Current evidence is missing, stale, duplicated, reordered, or substituted.
    EvidenceMismatch,
    /// Observation set or an observation identity does not match policy.
    ObservationMismatch,
    /// A terminal observation or its receipt failed canonical integrity validation.
    ObservationIntegrity,
    /// An observation did not establish a known successful attempt.
    ObservationNonSuccess,
    /// Deterministic postcondition result set is incomplete, duplicated, or substituted.
    PostconditionSetMismatch,
    /// A verification result failed canonical integrity validation.
    VerificationIntegrity,
    /// A verification result names another workflow, step, output, or evidence set.
    VerificationBindingMismatch,
    /// A verifier did not pass on current evidence.
    VerificationNonPass,
    /// A required invariant was not preserved or a prohibited effect was observed.
    SafetyVerificationFailed,
}

impl WorkflowVerifierError {
    /// Returns a stable content-free diagnostic code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidPolicy => "workflow.verifier.policy_invalid",
            Self::PolicyIntegrity => "workflow.verifier.policy_integrity",
            Self::StateMismatch => "workflow.verifier.state_mismatch",
            Self::EvidenceMismatch => "workflow.verifier.evidence_mismatch",
            Self::ObservationMismatch => "workflow.verifier.observation_mismatch",
            Self::ObservationIntegrity => "workflow.verifier.observation_integrity",
            Self::ObservationNonSuccess => "workflow.verifier.observation_non_success",
            Self::PostconditionSetMismatch => "workflow.verifier.postcondition_set",
            Self::VerificationIntegrity => "workflow.verifier.result_integrity",
            Self::VerificationBindingMismatch => "workflow.verifier.result_binding",
            Self::VerificationNonPass => "workflow.verifier.non_pass",
            Self::SafetyVerificationFailed => "workflow.verifier.safety_failed",
        }
    }
}

impl std::fmt::Display for WorkflowVerifierError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.code())
    }
}

impl std::error::Error for WorkflowVerifierError {}

/// Opaque proof that the exact current workflow evidence passed deterministic evaluation.
///
/// This type grants no execution authority and does not itself establish a terminal outcome.
/// Only [`evaluate_workflow_evidence`] can construct it.
#[derive(Debug, PartialEq, Eq)]
pub struct VerifiedWorkflowEvidence {
    workflow_id: String,
    step_id: String,
    expected_output_sha256: String,
    verified_state_sha256: String,
    verification_result_ids: Vec<String>,
    observation_receipt_sha256s: Vec<String>,
}

impl VerifiedWorkflowEvidence {
    /// Returns the workflow identity whose evidence was verified.
    #[must_use]
    pub fn workflow_id(&self) -> &str {
        &self.workflow_id
    }

    /// Returns the step identity whose evidence was verified.
    #[must_use]
    pub fn step_id(&self) -> &str {
        &self.step_id
    }

    /// Returns the exact expected output digest established by every verifier.
    #[must_use]
    pub fn expected_output_sha256(&self) -> &str {
        &self.expected_output_sha256
    }

    /// Returns the current state digest established by the evaluation.
    #[must_use]
    pub fn verified_state_sha256(&self) -> &str {
        &self.verified_state_sha256
    }

    /// Returns the complete ordered deterministic verification result identities.
    #[must_use]
    pub fn verification_result_ids(&self) -> &[String] {
        &self.verification_result_ids
    }

    /// Returns the complete ordered receipt digests checked against observations.
    #[must_use]
    pub fn observation_receipt_sha256s(&self) -> &[String] {
        &self.observation_receipt_sha256s
    }
}

/// Computes the canonical integrity digest for a workflow verifier policy.
pub fn workflow_verifier_policy_sha256(
    policy: &WorkflowVerifierPolicy,
) -> Result<String, WorkflowVerifierError> {
    let mut unsigned = policy.clone();
    unsigned.policy_sha256 = ZERO_SHA256.to_owned();
    canonical_sha256(&unsigned).map_err(|_| WorkflowVerifierError::InvalidPolicy)
}

/// Evaluates all expected outputs, postconditions, invariants, prohibited effects,
/// observations, receipts, and current evidence through the exact registered results.
///
/// Exit status, tool output excerpts, and model assertions are deliberately ignored. They
/// cannot compensate for a missing or non-passing deterministic result.
pub fn evaluate_workflow_evidence(
    policy: &WorkflowVerifierPolicy,
    input: WorkflowVerifierInput<'_>,
) -> Result<VerifiedWorkflowEvidence, WorkflowVerifierError> {
    validate_policy(policy)?;
    if workflow_verifier_policy_sha256(policy)? != policy.policy_sha256 {
        return Err(WorkflowVerifierError::PolicyIntegrity);
    }
    if input.current_state_sha256 != policy.expected_state_sha256 {
        return Err(WorkflowVerifierError::StateMismatch);
    }
    if input.current_evidence_sha256s != policy.required_current_evidence_sha256s
        || !valid_unique_sha_list(input.current_evidence_sha256s, false)
    {
        return Err(WorkflowVerifierError::EvidenceMismatch);
    }

    verify_observations(policy, input.observations)?;
    verify_results(policy, input.verification_results)?;

    Ok(VerifiedWorkflowEvidence {
        workflow_id: policy.workflow_id.clone(),
        step_id: policy.step_id.clone(),
        expected_output_sha256: policy.expected_output_sha256.clone(),
        verified_state_sha256: policy.expected_state_sha256.clone(),
        verification_result_ids: input
            .verification_results
            .iter()
            .map(|result| result.verification_result_id.clone())
            .collect(),
        observation_receipt_sha256s: input
            .observations
            .iter()
            .map(|observation| observation.receipt_sha256.clone())
            .collect(),
    })
}

fn validate_policy(policy: &WorkflowVerifierPolicy) -> Result<(), WorkflowVerifierError> {
    if !valid_id(&policy.workflow_id)
        || !valid_id(&policy.task_id)
        || !valid_id(&policy.step_id)
        || !valid_sha256(&policy.expected_output_sha256)
        || !valid_sha256(&policy.expected_state_sha256)
        || !valid_sha256(&policy.policy_sha256)
        || !valid_unique_id_list(&policy.required_postcondition_verifier_ids, false)
        || !valid_unique_id_list(&policy.required_invariant_ids, true)
        || !valid_unique_id_list(&policy.prohibited_effect_ids, true)
        || !valid_unique_sha_list(&policy.required_current_evidence_sha256s, false)
        || policy.required_observations.len() > MAX_ITEMS
    {
        return Err(WorkflowVerifierError::InvalidPolicy);
    }
    let mut observations = BTreeSet::new();
    let mut calls = BTreeSet::new();
    let mut attempts = BTreeSet::new();
    let mut receipts = BTreeSet::new();
    for expected in &policy.required_observations {
        if !valid_id(&expected.observation_id)
            || !valid_id(&expected.tool_call_id)
            || !valid_id(&expected.attempt_id)
            || !valid_sha256(&expected.receipt_sha256)
            || !observations.insert(expected.observation_id.as_str())
            || !calls.insert(expected.tool_call_id.as_str())
            || !attempts.insert(expected.attempt_id.as_str())
            || !receipts.insert(expected.receipt_sha256.as_str())
            || !policy
                .required_current_evidence_sha256s
                .contains(&expected.receipt_sha256)
        {
            return Err(WorkflowVerifierError::InvalidPolicy);
        }
    }
    if !policy
        .required_current_evidence_sha256s
        .contains(&policy.expected_output_sha256)
    {
        return Err(WorkflowVerifierError::InvalidPolicy);
    }
    Ok(())
}

fn verify_observations(
    policy: &WorkflowVerifierPolicy,
    observations: &[CanonicalToolObservation],
) -> Result<(), WorkflowVerifierError> {
    if observations.len() != policy.required_observations.len() {
        return Err(WorkflowVerifierError::ObservationMismatch);
    }
    for (observation, expected) in observations.iter().zip(&policy.required_observations) {
        if observation.observation_id != expected.observation_id
            || observation.tool_call_id != expected.tool_call_id
            || observation.attempt_id != expected.attempt_id
            || observation.task_id != policy.task_id
            || observation.step_id != policy.step_id
            || observation.receipt_sha256 != expected.receipt_sha256
        {
            return Err(WorkflowVerifierError::ObservationMismatch);
        }
        observation
            .validate_canonical()
            .map_err(|_| WorkflowVerifierError::ObservationIntegrity)?;
        if !observation_integrity(observation) {
            return Err(WorkflowVerifierError::ObservationIntegrity);
        }
        if observation.outcome != CanonicalToolOutcome::Succeeded
            || observation.state_change == CanonicalStateChange::Uncertain
        {
            return Err(WorkflowVerifierError::ObservationNonSuccess);
        }
    }
    Ok(())
}

fn verify_results(
    policy: &WorkflowVerifierPolicy,
    results: &[CanonicalVerificationResult],
) -> Result<(), WorkflowVerifierError> {
    if results.len() != policy.required_postcondition_verifier_ids.len()
        || results
            .iter()
            .map(|result| result.verifier_id.as_str())
            .ne(policy
                .required_postcondition_verifier_ids
                .iter()
                .map(String::as_str))
        || results
            .iter()
            .map(|result| result.verification_result_id.as_str())
            .collect::<BTreeSet<_>>()
            .len()
            != results.len()
    {
        return Err(WorkflowVerifierError::PostconditionSetMismatch);
    }
    for result in results {
        if result.outcome != CanonicalVerificationOutcome::Passed || !result.current {
            return Err(WorkflowVerifierError::VerificationNonPass);
        }
        result
            .validate_canonical()
            .map_err(|_| WorkflowVerifierError::VerificationIntegrity)?;
        if !verification_integrity(result) {
            return Err(WorkflowVerifierError::VerificationIntegrity);
        }
        if result.workflow_id != policy.workflow_id
            || result.step_id.as_deref() != Some(policy.step_id.as_str())
            || result.subject_sha256 != policy.expected_output_sha256
            || result.observed_evidence_sha256s != policy.required_current_evidence_sha256s
        {
            return Err(WorkflowVerifierError::VerificationBindingMismatch);
        }
        if result.preserved_invariants != policy.required_invariant_ids
            || !result.prohibited_effects_observed.is_empty()
        {
            return Err(WorkflowVerifierError::SafetyVerificationFailed);
        }
    }
    Ok(())
}

fn observation_integrity(value: &CanonicalToolObservation) -> bool {
    let mut unsigned = value.clone();
    unsigned.receipt_sha256 = ZERO_SHA256.to_owned();
    canonical_sha256(&unsigned).is_ok_and(|expected| expected == value.receipt_sha256)
}

fn verification_integrity(value: &CanonicalVerificationResult) -> bool {
    let mut unsigned = value.clone();
    unsigned.result_sha256 = ZERO_SHA256.to_owned();
    canonical_sha256(&unsigned).is_ok_and(|expected| expected == value.result_sha256)
}

fn canonical_sha256(value: &impl serde::Serialize) -> Result<String, serde_json::Error> {
    let bytes = serde_json::to_vec(value)?;
    let digest = Sha256::digest(bytes);
    let mut encoded = String::with_capacity(64);
    for byte in digest {
        let _ = write!(&mut encoded, "{byte:02x}");
    }
    Ok(encoded)
}

fn valid_unique_id_list(values: &[String], empty_allowed: bool) -> bool {
    values.len() <= MAX_ITEMS
        && (empty_allowed || !values.is_empty())
        && values.iter().all(|value| valid_id(value))
        && values.iter().collect::<BTreeSet<_>>().len() == values.len()
}

fn valid_unique_sha_list(values: &[String], empty_allowed: bool) -> bool {
    values.len() <= MAX_ITEMS
        && (empty_allowed || !values.is_empty())
        && values.iter().all(|value| valid_sha256(value))
        && values.iter().collect::<BTreeSet<_>>().len() == values.len()
}

fn valid_id(value: &str) -> bool {
    !value.is_empty() && value.len() <= 128
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}
