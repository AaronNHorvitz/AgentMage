//! Authority-reducing application of learned or model-based classifier output.

use std::collections::BTreeSet;

use agentmage_kernel_contracts::{
    AdvisoryClassifierDisposition, AdvisoryClassifierResult, AdvisoryClassifierStatus,
};

use crate::preclassification_policy::PreclassificationClearance;

const MAX_ADVISORY_EVIDENCE: usize = 64;

/// Stable reason advisory classifier output cannot affect the deterministic boundary.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AdvisoryPolicyError {
    /// Result identity, digest, confidence, evidence, or dispositions are malformed.
    InvalidResult,
    /// Result names another deterministic fact set, task, or action.
    ClearanceMismatch,
    /// Result is incomplete or unavailable and requires failure mapping.
    ClassifierNotComplete,
}

/// Non-authoritative normalized restrictions layered over deterministic clearance.
///
/// An empty disposition list preserves the existing deterministic boundary; it does not create
/// an allow decision. This record has no grant, operation, destination selection, model switch,
/// execution, denial override, or completion method.
#[derive(Debug, PartialEq, Eq)]
pub struct AdvisoryBoundaryDecision {
    classifier_run_id: String,
    fact_set_sha256: String,
    dispositions: Vec<AdvisoryClassifierDisposition>,
}

impl AdvisoryBoundaryDecision {
    /// Returns the exact classifier-run identity.
    #[must_use]
    pub fn classifier_run_id(&self) -> &str {
        &self.classifier_run_id
    }

    /// Returns the exact deterministic fact-set digest this decision can only restrict.
    #[must_use]
    pub fn fact_set_sha256(&self) -> &str {
        &self.fact_set_sha256
    }

    /// Returns normalized authority-reducing or user-escalating dispositions.
    #[must_use]
    pub fn dispositions(&self) -> &[AdvisoryClassifierDisposition] {
        &self.dispositions
    }
}

/// Stateless gate applying advisory output only after deterministic clearance.
pub struct AdvisoryPolicyGate;

impl AdvisoryPolicyGate {
    /// Validates and normalizes one complete advisory result.
    pub fn apply(
        clearance: &PreclassificationClearance,
        result: &AdvisoryClassifierResult,
    ) -> Result<AdvisoryBoundaryDecision, AdvisoryPolicyError> {
        validate_result(result)?;
        if result.status != AdvisoryClassifierStatus::Complete {
            return Err(AdvisoryPolicyError::ClassifierNotComplete);
        }
        if !clearance.matches_advisory_identity(
            &result.task_id,
            result.action_id.as_ref(),
            &result.input_sha256,
        ) {
            return Err(AdvisoryPolicyError::ClearanceMismatch);
        }
        let dispositions = normalize_dispositions(&result.dispositions)?;
        Ok(AdvisoryBoundaryDecision {
            classifier_run_id: result.classifier_run_id.as_str().to_owned(),
            fact_set_sha256: clearance.fact_set_sha256().to_owned(),
            dispositions,
        })
    }
}

fn validate_result(result: &AdvisoryClassifierResult) -> Result<(), AdvisoryPolicyError> {
    if result.schema_version != agentmage_kernel_contracts::CONTRACT_SCHEMA_VERSION
        || !valid_id(result.classifier_run_id.as_str())
        || !valid_id(result.task_id.as_str())
        || result
            .action_id
            .as_ref()
            .is_none_or(|action_id| !valid_id(action_id.as_str()))
        || !valid_sha256(&result.input_sha256)
        || !valid_sha256(&result.rationale_sha256)
        || result
            .confidence_basis_points
            .is_some_and(|value| value > 10_000)
        || result.evidence.len() > MAX_ADVISORY_EVIDENCE
        || result.evidence.iter().any(|evidence| {
            evidence.schema_version != agentmage_kernel_contracts::CONTRACT_SCHEMA_VERSION
                || !valid_id(evidence.evidence_id.as_str())
                || !valid_sha256(&evidence.content_sha256)
        })
    {
        return Err(AdvisoryPolicyError::InvalidResult);
    }
    Ok(())
}

fn normalize_dispositions(
    values: &[AdvisoryClassifierDisposition],
) -> Result<Vec<AdvisoryClassifierDisposition>, AdvisoryPolicyError> {
    let unique: BTreeSet<_> = values.iter().copied().collect();
    if unique.len() != values.len() || unique.len() > 5 {
        return Err(AdvisoryPolicyError::InvalidResult);
    }
    let order = [
        AdvisoryClassifierDisposition::Deny,
        AdvisoryClassifierDisposition::Narrow,
        AdvisoryClassifierDisposition::Redact,
        AdvisoryClassifierDisposition::Isolate,
        AdvisoryClassifierDisposition::Escalate,
    ];
    Ok(order
        .into_iter()
        .filter(|value| unique.contains(value))
        .collect())
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

#[cfg(test)]
mod tests {
    use super::{AdvisoryPolicyError, AdvisoryPolicyGate};
    use crate::preclassification_policy::PreclassificationPolicyGate;
    use agentmage_kernel_contracts::{
        ActionId, ActionRisk, AdvisoryClassifierDisposition, AdvisoryClassifierResult,
        AdvisoryClassifierStatus, AutonomyLevel, BudgetState, CredentialClass, DataSensitivity,
        DeterministicPolicyFacts, DisclosureClass, ExactAuthorityState, GrantOperation,
        ModelCapabilityRole, ModelCapabilityStatus, ModelRunId, NetworkRequirement,
        OperationBinding, PathScopeState, PolicyDestinationClass, PolicyId, PolicySourceClass,
        RepositoryState, SessionId, StaticPolicyCheck, StaticPolicyCheckKind,
        StaticPolicyCheckState, TaskId, from_json, to_canonical_json,
    };

    const SHA256: &str = "1111111111111111111111111111111111111111111111111111111111111111";

    fn facts() -> DeterministicPolicyFacts {
        DeterministicPolicyFacts {
            schema_version: agentmage_kernel_contracts::CONTRACT_SCHEMA_VERSION,
            fact_set_id: "facts-0001".to_owned(),
            policy_id: PolicyId::from_raw("policy-0001"),
            policy_sha256: SHA256.to_owned(),
            actor_id: "actor-0001".to_owned(),
            session_id: SessionId::from_raw("session-0001"),
            task_id: TaskId::from_raw("task-0001"),
            action_id: ActionId::from_raw("action-0001"),
            autonomy: AutonomyLevel::Inspect,
            operation: OperationBinding::new(GrantOperation::WorkspaceRead),
            source: PolicySourceClass::LocalWorkspace,
            source_sha256: SHA256.to_owned(),
            destination: PolicyDestinationClass::UserDisplay,
            destination_sha256: SHA256.to_owned(),
            path_state: PathScopeState::InScope,
            path_sha256: SHA256.to_owned(),
            repository_state: RepositoryState::Clean,
            credential_class: CredentialClass::None,
            data_sensitivity: DataSensitivity::Ephemeral,
            action_risk: ActionRisk::Minimal,
            model_capability_role: ModelCapabilityRole::AdvisoryClassification,
            model_capability_status: ModelCapabilityStatus::MeasuredSupported,
            reversible: true,
            network: NetworkRequirement::None,
            disclosure: DisclosureClass::LocalOnly,
            budget: BudgetState::Within,
            exact_authority: ExactAuthorityState::CurrentExact,
            static_checks: [
                StaticPolicyCheckKind::Secret,
                StaticPolicyCheckKind::Path,
                StaticPolicyCheckKind::ExecutableContent,
                StaticPolicyCheckKind::Destination,
            ]
            .into_iter()
            .map(|kind| StaticPolicyCheck {
                kind,
                state: if kind == StaticPolicyCheckKind::ExecutableContent {
                    StaticPolicyCheckState::NotApplicable
                } else {
                    StaticPolicyCheckState::Clear
                },
                observation_sha256: SHA256.to_owned(),
            })
            .collect(),
        }
    }

    fn result(
        input_sha256: &str,
        status: AdvisoryClassifierStatus,
        dispositions: Vec<AdvisoryClassifierDisposition>,
    ) -> AdvisoryClassifierResult {
        AdvisoryClassifierResult {
            schema_version: agentmage_kernel_contracts::CONTRACT_SCHEMA_VERSION,
            classifier_run_id: ModelRunId::from_raw("classifier-0001"),
            task_id: TaskId::from_raw("task-0001"),
            action_id: Some(ActionId::from_raw("action-0001")),
            input_sha256: input_sha256.to_owned(),
            status,
            dispositions,
            confidence_basis_points: Some(8_000),
            rationale_sha256: SHA256.to_owned(),
            evidence: Vec::new(),
        }
    }

    #[test]
    fn task_12_2_2_3_each_advisory_disposition_only_restricts_or_escalates() {
        let clearance = PreclassificationPolicyGate::evaluate(&facts()).expect("clearance");
        for disposition in [
            AdvisoryClassifierDisposition::Deny,
            AdvisoryClassifierDisposition::Narrow,
            AdvisoryClassifierDisposition::Redact,
            AdvisoryClassifierDisposition::Isolate,
            AdvisoryClassifierDisposition::Escalate,
        ] {
            let decision = AdvisoryPolicyGate::apply(
                &clearance,
                &result(
                    clearance.fact_set_sha256(),
                    AdvisoryClassifierStatus::Complete,
                    vec![disposition],
                ),
            )
            .expect("restrictive decision");
            assert_eq!(decision.dispositions(), &[disposition]);
            assert_eq!(decision.fact_set_sha256(), clearance.fact_set_sha256());
        }
    }

    #[test]
    fn task_12_2_2_3_combinations_normalize_without_broadening() {
        let clearance = PreclassificationPolicyGate::evaluate(&facts()).expect("clearance");
        let candidate = result(
            clearance.fact_set_sha256(),
            AdvisoryClassifierStatus::Complete,
            vec![
                AdvisoryClassifierDisposition::Escalate,
                AdvisoryClassifierDisposition::Redact,
                AdvisoryClassifierDisposition::Deny,
            ],
        );
        let first = AdvisoryPolicyGate::apply(&clearance, &candidate).expect("decision");
        let repeated = AdvisoryPolicyGate::apply(&clearance, &candidate).expect("decision");
        assert_eq!(first, repeated);
        assert_eq!(
            first.dispositions(),
            [
                AdvisoryClassifierDisposition::Deny,
                AdvisoryClassifierDisposition::Redact,
                AdvisoryClassifierDisposition::Escalate,
            ]
        );

        let unchanged = AdvisoryPolicyGate::apply(
            &clearance,
            &result(
                clearance.fact_set_sha256(),
                AdvisoryClassifierStatus::Complete,
                Vec::new(),
            ),
        )
        .expect("no additional restriction");
        assert!(unchanged.dispositions().is_empty());
    }

    #[test]
    fn task_12_2_2_3_malformed_duplicate_or_oversized_results_are_inert() {
        let clearance = PreclassificationPolicyGate::evaluate(&facts()).expect("clearance");
        let mut candidates = Vec::new();
        let mut duplicate = result(
            clearance.fact_set_sha256(),
            AdvisoryClassifierStatus::Complete,
            vec![
                AdvisoryClassifierDisposition::Deny,
                AdvisoryClassifierDisposition::Deny,
            ],
        );
        candidates.push(duplicate.clone());
        duplicate.dispositions.clear();
        duplicate.confidence_basis_points = Some(10_001);
        candidates.push(duplicate);
        let mut malformed = result(
            clearance.fact_set_sha256(),
            AdvisoryClassifierStatus::Complete,
            Vec::new(),
        );
        malformed.rationale_sha256 = "invalid".to_owned();
        candidates.push(malformed);
        let mut missing_action = result(
            clearance.fact_set_sha256(),
            AdvisoryClassifierStatus::Complete,
            Vec::new(),
        );
        missing_action.action_id = None;
        candidates.push(missing_action);

        for candidate in candidates {
            assert_eq!(
                AdvisoryPolicyGate::apply(&clearance, &candidate),
                Err(AdvisoryPolicyError::InvalidResult)
            );
        }
    }

    #[test]
    fn task_12_2_2_3_task_action_and_fact_identity_must_match_clearance() {
        let clearance = PreclassificationPolicyGate::evaluate(&facts()).expect("clearance");
        let mut candidates = Vec::new();
        let mut task = result(
            clearance.fact_set_sha256(),
            AdvisoryClassifierStatus::Complete,
            Vec::new(),
        );
        task.task_id = TaskId::from_raw("task-other");
        candidates.push(task);
        let mut action = result(
            clearance.fact_set_sha256(),
            AdvisoryClassifierStatus::Complete,
            Vec::new(),
        );
        action.action_id = Some(ActionId::from_raw("action-other"));
        candidates.push(action);
        candidates.push(result(
            SHA256,
            AdvisoryClassifierStatus::Complete,
            Vec::new(),
        ));
        for candidate in candidates {
            assert_eq!(
                AdvisoryPolicyGate::apply(&clearance, &candidate),
                Err(AdvisoryPolicyError::ClearanceMismatch)
            );
        }
    }

    #[test]
    fn task_12_2_2_3_incomplete_classifier_states_cannot_change_boundary() {
        let clearance = PreclassificationPolicyGate::evaluate(&facts()).expect("clearance");
        for status in [
            AdvisoryClassifierStatus::LowConfidence,
            AdvisoryClassifierStatus::Truncated,
            AdvisoryClassifierStatus::Disagreement,
            AdvisoryClassifierStatus::Unavailable,
            AdvisoryClassifierStatus::OutOfDistribution,
            AdvisoryClassifierStatus::TimedOut,
            AdvisoryClassifierStatus::Malformed,
        ] {
            assert_eq!(
                AdvisoryPolicyGate::apply(
                    &clearance,
                    &result(
                        clearance.fact_set_sha256(),
                        status,
                        vec![AdvisoryClassifierDisposition::Narrow],
                    ),
                ),
                Err(AdvisoryPolicyError::ClassifierNotComplete)
            );
        }
    }

    #[test]
    fn task_12_2_2_3_prohibited_authority_fields_and_denial_override_are_impossible() {
        let clearance = PreclassificationPolicyGate::evaluate(&facts()).expect("clearance");
        let candidate = result(
            clearance.fact_set_sha256(),
            AdvisoryClassifierStatus::Complete,
            Vec::new(),
        );
        let encoded = to_canonical_json(&candidate).expect("result serializes");
        let value: serde_json::Value = serde_json::from_slice(&encoded).expect("valid JSON");
        for field in [
            "grant_id",
            "override_denial",
            "destination",
            "selected_model",
            "execute",
            "completion",
        ] {
            let mut attack = value.clone();
            attack[field] = serde_json::Value::Bool(true);
            assert!(
                from_json::<AdvisoryClassifierResult>(
                    &serde_json::to_vec(&attack).expect("attack JSON")
                )
                .is_err()
            );
        }

        let mut denied_facts = facts();
        denied_facts.static_checks[0].state = StaticPolicyCheckState::Detected;
        assert!(PreclassificationPolicyGate::evaluate(&denied_facts).is_err());
        assert_eq!(candidate.classifier_run_id.as_str(), "classifier-0001");
    }
}
