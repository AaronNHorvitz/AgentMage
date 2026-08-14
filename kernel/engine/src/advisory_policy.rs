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
    /// A complete result was incorrectly submitted to the failure mapper.
    FailureMappingNotApplicable,
}

/// Deterministic safe action for an incomplete or unavailable advisory classifier.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClassifierFailureAction {
    /// Narrow the existing deterministic boundary.
    Narrow,
    /// Isolate content or processing before another boundary.
    Isolate,
    /// Require an explicit user decision.
    UserDecision,
    /// Enter the named `BLOCKED` non-success state.
    Blocked,
}

/// Content-free failure decision bound to one exact deterministic fact set.
#[derive(Debug, PartialEq, Eq)]
pub struct ClassifierFailureDecision {
    status: AdvisoryClassifierStatus,
    action: ClassifierFailureAction,
    fact_set_sha256: String,
}

impl ClassifierFailureDecision {
    /// Returns the exact incomplete classifier state.
    #[must_use]
    pub const fn status(&self) -> AdvisoryClassifierStatus {
        self.status
    }

    /// Returns the deterministic narrower, user-decision, or blocked action.
    #[must_use]
    pub const fn action(&self) -> ClassifierFailureAction {
        self.action
    }

    /// Returns the exact deterministic fact-set digest that remains authoritative.
    #[must_use]
    pub fn fact_set_sha256(&self) -> &str {
        &self.fact_set_sha256
    }
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

    /// Maps one incomplete or unavailable classifier state to a safe deterministic action.
    pub fn map_failure(
        clearance: &PreclassificationClearance,
        result: &AdvisoryClassifierResult,
    ) -> Result<ClassifierFailureDecision, AdvisoryPolicyError> {
        validate_result(result)?;
        if !clearance.matches_advisory_identity(
            &result.task_id,
            result.action_id.as_ref(),
            &result.input_sha256,
        ) {
            return Err(AdvisoryPolicyError::ClearanceMismatch);
        }
        let action = match result.status {
            AdvisoryClassifierStatus::Complete => {
                return Err(AdvisoryPolicyError::FailureMappingNotApplicable);
            }
            AdvisoryClassifierStatus::LowConfidence => ClassifierFailureAction::Narrow,
            AdvisoryClassifierStatus::Disagreement => ClassifierFailureAction::UserDecision,
            AdvisoryClassifierStatus::OutOfDistribution => ClassifierFailureAction::Isolate,
            AdvisoryClassifierStatus::Truncated
            | AdvisoryClassifierStatus::Unavailable
            | AdvisoryClassifierStatus::TimedOut
            | AdvisoryClassifierStatus::Malformed => ClassifierFailureAction::Blocked,
        };
        Ok(ClassifierFailureDecision {
            status: result.status,
            action,
            fact_set_sha256: clearance.fact_set_sha256().to_owned(),
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
    use super::{AdvisoryPolicyError, AdvisoryPolicyGate, ClassifierFailureAction};
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

    #[test]
    fn task_12_2_2_4_every_incomplete_state_has_one_exact_safe_action() {
        let clearance = PreclassificationPolicyGate::evaluate(&facts()).expect("clearance");
        let cases = [
            (
                AdvisoryClassifierStatus::LowConfidence,
                ClassifierFailureAction::Narrow,
            ),
            (
                AdvisoryClassifierStatus::Truncated,
                ClassifierFailureAction::Blocked,
            ),
            (
                AdvisoryClassifierStatus::Disagreement,
                ClassifierFailureAction::UserDecision,
            ),
            (
                AdvisoryClassifierStatus::Unavailable,
                ClassifierFailureAction::Blocked,
            ),
            (
                AdvisoryClassifierStatus::OutOfDistribution,
                ClassifierFailureAction::Isolate,
            ),
            (
                AdvisoryClassifierStatus::TimedOut,
                ClassifierFailureAction::Blocked,
            ),
            (
                AdvisoryClassifierStatus::Malformed,
                ClassifierFailureAction::Blocked,
            ),
        ];
        for (status, expected) in cases {
            let decision = AdvisoryPolicyGate::map_failure(
                &clearance,
                &result(clearance.fact_set_sha256(), status, Vec::new()),
            )
            .expect("failure maps");
            assert_eq!(decision.status(), status);
            assert_eq!(decision.action(), expected);
            assert_eq!(decision.fact_set_sha256(), clearance.fact_set_sha256());
        }
    }

    #[test]
    fn task_12_2_2_4_failure_mapping_requires_exact_clearance_identity() {
        let clearance = PreclassificationPolicyGate::evaluate(&facts()).expect("clearance");
        let mut task = result(
            clearance.fact_set_sha256(),
            AdvisoryClassifierStatus::Unavailable,
            Vec::new(),
        );
        task.task_id = TaskId::from_raw("task-other");
        assert_eq!(
            AdvisoryPolicyGate::map_failure(&clearance, &task),
            Err(AdvisoryPolicyError::ClearanceMismatch)
        );
        let stale = result(SHA256, AdvisoryClassifierStatus::Unavailable, Vec::new());
        assert_eq!(
            AdvisoryPolicyGate::map_failure(&clearance, &stale),
            Err(AdvisoryPolicyError::ClearanceMismatch)
        );
    }

    #[test]
    fn task_12_2_2_4_complete_results_cannot_enter_failure_mapping() {
        let clearance = PreclassificationPolicyGate::evaluate(&facts()).expect("clearance");
        let complete = result(
            clearance.fact_set_sha256(),
            AdvisoryClassifierStatus::Complete,
            Vec::new(),
        );
        assert_eq!(
            AdvisoryPolicyGate::map_failure(&clearance, &complete),
            Err(AdvisoryPolicyError::FailureMappingNotApplicable)
        );
        assert!(AdvisoryPolicyGate::apply(&clearance, &complete).is_ok());
    }

    #[test]
    fn task_12_2_2_4_confidence_never_changes_failure_mapping() {
        let clearance = PreclassificationPolicyGate::evaluate(&facts()).expect("clearance");
        let mut low = result(
            clearance.fact_set_sha256(),
            AdvisoryClassifierStatus::LowConfidence,
            Vec::new(),
        );
        low.confidence_basis_points = Some(0);
        let mut high = low.clone();
        high.confidence_basis_points = Some(10_000);
        assert_eq!(
            AdvisoryPolicyGate::map_failure(&clearance, &low),
            AdvisoryPolicyGate::map_failure(&clearance, &high)
        );
        high.confidence_basis_points = Some(10_001);
        assert_eq!(
            AdvisoryPolicyGate::map_failure(&clearance, &high),
            Err(AdvisoryPolicyError::InvalidResult)
        );
    }

    #[test]
    fn task_12_2_2_4_classifier_dispositions_cannot_weaken_failure_action() {
        let clearance = PreclassificationPolicyGate::evaluate(&facts()).expect("clearance");
        for dispositions in [
            Vec::new(),
            vec![AdvisoryClassifierDisposition::Narrow],
            vec![
                AdvisoryClassifierDisposition::Deny,
                AdvisoryClassifierDisposition::Escalate,
            ],
        ] {
            let decision = AdvisoryPolicyGate::map_failure(
                &clearance,
                &result(
                    clearance.fact_set_sha256(),
                    AdvisoryClassifierStatus::TimedOut,
                    dispositions,
                ),
            )
            .expect("timeout maps");
            assert_eq!(decision.action(), ClassifierFailureAction::Blocked);
        }
    }

    #[test]
    fn task_12_2_2_4_failure_decisions_are_reproducible_and_non_authoritative() {
        let clearance = PreclassificationPolicyGate::evaluate(&facts()).expect("clearance");
        let unavailable = result(
            clearance.fact_set_sha256(),
            AdvisoryClassifierStatus::Unavailable,
            Vec::new(),
        );
        let first = AdvisoryPolicyGate::map_failure(&clearance, &unavailable).expect("maps");
        let repeated = AdvisoryPolicyGate::map_failure(&clearance, &unavailable).expect("maps");
        assert_eq!(first, repeated);
        assert_eq!(first.action(), ClassifierFailureAction::Blocked);
        assert_eq!(first.fact_set_sha256(), clearance.fact_set_sha256());
    }

    #[test]
    fn d027_s12_classifier_1280_outputs_never_broaden_authority_or_complete() {
        const OUTPUT_CLASS_COUNT: usize = 10;
        const OUTPUTS_PER_CLASS: usize = 128;
        const OUTPUT_COUNT: usize = OUTPUT_CLASS_COUNT * OUTPUTS_PER_CLASS;

        let clearance = PreclassificationPolicyGate::evaluate(&facts()).expect("clearance");
        let mut class_counts = [0_usize; OUTPUT_CLASS_COUNT];
        let mut parser_rejections = 0_usize;
        let mut failure_decisions = 0_usize;
        let mut restrictive_decisions = 0_usize;

        for index in 0..OUTPUT_COUNT {
            let output_class = index % OUTPUT_CLASS_COUNT;
            class_counts[output_class] += 1;
            let mut candidate = result(
                clearance.fact_set_sha256(),
                AdvisoryClassifierStatus::Complete,
                Vec::new(),
            );
            candidate.classifier_run_id = ModelRunId::from_raw(format!("classifier-{index:04}"));
            candidate.confidence_basis_points =
                Some(u16::try_from((index * 97) % 10_001).expect("bounded confidence fits u16"));

            match output_class {
                0 => {
                    let encoded = String::from_utf8(
                        to_canonical_json(&candidate).expect("candidate serializes"),
                    )
                    .expect("canonical JSON is UTF-8");
                    let field = ["allow", "approved", "safe", "authorized"][index % 4];
                    let hostile = encoded.replacen('{', &format!("{{\"{field}\":true,"), 1);
                    assert!(from_json::<AdvisoryClassifierResult>(hostile.as_bytes()).is_err());
                    parser_rejections += 1;
                }
                1..=7 => {
                    candidate.status = [
                        AdvisoryClassifierStatus::LowConfidence,
                        AdvisoryClassifierStatus::Disagreement,
                        AdvisoryClassifierStatus::Truncated,
                        AdvisoryClassifierStatus::TimedOut,
                        AdvisoryClassifierStatus::Malformed,
                        AdvisoryClassifierStatus::Unavailable,
                        AdvisoryClassifierStatus::OutOfDistribution,
                    ][output_class - 1];
                    candidate.dispositions = vec![AdvisoryClassifierDisposition::Escalate];
                    let decision = AdvisoryPolicyGate::map_failure(&clearance, &candidate)
                        .expect("failure has one deterministic safe action");
                    let expected = match candidate.status {
                        AdvisoryClassifierStatus::LowConfidence => ClassifierFailureAction::Narrow,
                        AdvisoryClassifierStatus::Disagreement => {
                            ClassifierFailureAction::UserDecision
                        }
                        AdvisoryClassifierStatus::OutOfDistribution => {
                            ClassifierFailureAction::Isolate
                        }
                        AdvisoryClassifierStatus::Truncated
                        | AdvisoryClassifierStatus::Unavailable
                        | AdvisoryClassifierStatus::TimedOut
                        | AdvisoryClassifierStatus::Malformed => ClassifierFailureAction::Blocked,
                        AdvisoryClassifierStatus::Complete => unreachable!("failure class only"),
                    };
                    assert_eq!(decision.action(), expected);
                    failure_decisions += 1;
                }
                8 => {
                    let encoded = String::from_utf8(
                        to_canonical_json(&candidate).expect("candidate serializes"),
                    )
                    .expect("canonical JSON is UTF-8");
                    let field = [
                        "grant",
                        "operation",
                        "destination",
                        "model_profile_id",
                        "execute",
                        "completion",
                    ][index % 6];
                    let hostile = encoded.replacen('{', &format!("{{\"{field}\":true,"), 1);
                    assert!(from_json::<AdvisoryClassifierResult>(hostile.as_bytes()).is_err());
                    parser_rejections += 1;
                }
                9 => {
                    candidate.dispositions = vec![
                        AdvisoryClassifierDisposition::Deny,
                        AdvisoryClassifierDisposition::Narrow,
                        AdvisoryClassifierDisposition::Redact,
                        AdvisoryClassifierDisposition::Isolate,
                        AdvisoryClassifierDisposition::Escalate,
                    ];
                    let decision = AdvisoryPolicyGate::apply(&clearance, &candidate)
                        .expect("complete restrictive result");
                    assert_eq!(decision.dispositions(), candidate.dispositions);
                    assert_eq!(decision.fact_set_sha256(), clearance.fact_set_sha256());
                    restrictive_decisions += 1;
                }
                _ => unreachable!("output class is reduced modulo 10"),
            }
        }

        assert_eq!(class_counts, [OUTPUTS_PER_CLASS; OUTPUT_CLASS_COUNT]);
        assert_eq!(parser_rejections, 256);
        assert_eq!(failure_decisions, 896);
        assert_eq!(restrictive_decisions, 128);
        assert_eq!(
            parser_rejections + failure_decisions + restrictive_decisions,
            OUTPUT_COUNT
        );
    }

    #[test]
    fn d027_s12_classification_context_remains_deterministic_and_bounded() {
        let baseline_facts = facts();
        let baseline_clearance =
            PreclassificationPolicyGate::evaluate(&baseline_facts).expect("baseline clearance");

        for risk in [
            ActionRisk::Minimal,
            ActionRisk::Controlled,
            ActionRisk::Elevated,
            ActionRisk::Critical,
        ] {
            let mut candidate = baseline_facts.clone();
            candidate.action_risk = risk;
            let first = PreclassificationPolicyGate::evaluate(&candidate);
            let repeated = PreclassificationPolicyGate::evaluate(&candidate);
            assert_eq!(first, repeated);
            assert_eq!(first.is_ok(), risk == ActionRisk::Minimal);
        }

        for role in [
            ModelCapabilityRole::CodeGeneration,
            ModelCapabilityRole::ToolSelection,
            ModelCapabilityRole::AdvisoryClassification,
            ModelCapabilityRole::Embedding,
            ModelCapabilityRole::Multimodal,
            ModelCapabilityRole::Specialist,
            ModelCapabilityRole::LegacyCompatibility,
        ] {
            for status in [
                ModelCapabilityStatus::MeasuredSupported,
                ModelCapabilityStatus::MeasuredUnsupported,
                ModelCapabilityStatus::Blocked,
                ModelCapabilityStatus::Unknown,
            ] {
                let mut candidate = baseline_facts.clone();
                candidate.model_capability_role = role;
                candidate.model_capability_status = status;
                let result = PreclassificationPolicyGate::evaluate(&candidate);
                assert_eq!(
                    result.is_ok(),
                    status == ModelCapabilityStatus::MeasuredSupported
                );
                assert_eq!(result, PreclassificationPolicyGate::evaluate(&candidate));
            }
        }

        let value: serde_json::Value =
            serde_json::from_slice(&to_canonical_json(&baseline_facts).expect("facts serialize"))
                .expect("facts JSON");
        let serde_json::Value::Object(fields) = value else {
            panic!("facts must serialize as an object");
        };
        let reordered = format!(
            "{{{}}}",
            fields
                .iter()
                .rev()
                .map(|(name, value)| format!(
                    "{}:{}",
                    serde_json::to_string(name).expect("field name"),
                    serde_json::to_string(value).expect("field value")
                ))
                .collect::<Vec<_>>()
                .join(",")
        );
        let reordered_facts =
            from_json::<DeterministicPolicyFacts>(reordered.as_bytes()).expect("reordered facts");
        assert_eq!(reordered_facts, baseline_facts);
        assert_eq!(
            PreclassificationPolicyGate::evaluate(&reordered_facts),
            Ok(baseline_clearance)
        );

        let mut baseline_result = result(
            PreclassificationPolicyGate::evaluate(&baseline_facts)
                .expect("clearance")
                .fact_set_sha256(),
            AdvisoryClassifierStatus::Complete,
            vec![AdvisoryClassifierDisposition::Narrow],
        );
        baseline_result.rationale_sha256 = "1".repeat(64);
        let baseline_decision = AdvisoryPolicyGate::apply(
            &PreclassificationPolicyGate::evaluate(&baseline_facts).expect("clearance"),
            &baseline_result,
        )
        .expect("baseline advisory");
        for marker in ['2', '3', '4', '5'] {
            let mut explained = baseline_result.clone();
            explained.rationale_sha256 = marker.to_string().repeat(64);
            let decision = AdvisoryPolicyGate::apply(
                &PreclassificationPolicyGate::evaluate(&baseline_facts).expect("clearance"),
                &explained,
            )
            .expect("changed explanation");
            assert_eq!(decision, baseline_decision);
        }
    }
}
