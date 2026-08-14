//! Continuous content reclassification before successive trust boundaries.

use agentmage_kernel_contracts::{
    AdvisoryClassifierDisposition, ClassificationBoundary, DataSensitivity,
    DataSensitivityAssessment, ReclassificationContentKind, ReclassificationRequest,
};

use crate::{
    advisory_policy::AdvisoryBoundaryDecision, preclassification_policy::PreclassificationClearance,
};

const MAX_CLASSIFICATION_EVIDENCE: usize = 32;

/// Stable reason content cannot cross its requested next trust boundary.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReclassificationError {
    /// Request identity, revision, hash, or boundary shape is malformed.
    InvalidRequest,
    /// Sensitivity assessment is stale, mismatched, or lacks current evidence.
    SensitivityMismatch,
    /// Deterministic policy clearance names another task, action, or fact set.
    PolicyMismatch,
    /// Advisory output names another fact set or requires restriction or user review.
    AdvisoryRestriction,
    /// Restricted content cannot cross the requested external network boundary.
    SensitivityBoundary,
    /// Permit names another request or has already been consumed.
    PermitMismatch,
}

/// Opaque one-use proof that one exact content revision was reclassified.
#[derive(Debug, PartialEq, Eq)]
pub struct ReclassificationPermit {
    request_id: String,
    content_kind: ReclassificationContentKind,
    destination_boundary: ClassificationBoundary,
    subject_sha256: String,
    observed_revision: String,
    policy_fact_sha256: String,
}

/// Content-free record that one exact reclassified revision crossed one boundary.
#[derive(Debug, PartialEq, Eq)]
pub struct BoundaryCrossingReceipt {
    request_id: String,
    content_kind: ReclassificationContentKind,
    destination_boundary: ClassificationBoundary,
    subject_sha256: String,
    observed_revision: String,
    policy_fact_sha256: String,
}

impl BoundaryCrossingReceipt {
    /// Returns the exact request identity.
    #[must_use]
    pub fn request_id(&self) -> &str {
        &self.request_id
    }

    /// Returns the exact content class that crossed.
    #[must_use]
    pub const fn content_kind(&self) -> ReclassificationContentKind {
        self.content_kind
    }

    /// Returns the exact destination boundary.
    #[must_use]
    pub const fn destination_boundary(&self) -> ClassificationBoundary {
        self.destination_boundary
    }

    /// Returns the content digest, never raw content.
    #[must_use]
    pub fn subject_sha256(&self) -> &str {
        &self.subject_sha256
    }

    /// Returns the exact observed revision.
    #[must_use]
    pub fn observed_revision(&self) -> &str {
        &self.observed_revision
    }

    /// Returns the deterministic fact-set digest used for the crossing.
    #[must_use]
    pub fn policy_fact_sha256(&self) -> &str {
        &self.policy_fact_sha256
    }
}

/// One-request gate that requires fresh classification before one boundary crossing.
pub struct ReclassificationGate {
    request: ReclassificationRequest,
    crossed: bool,
}

impl ReclassificationGate {
    /// Creates a gate for one exact valid source-to-destination request.
    pub fn new(request: ReclassificationRequest) -> Result<Self, ReclassificationError> {
        if !valid_request(&request) {
            return Err(ReclassificationError::InvalidRequest);
        }
        Ok(Self {
            request,
            crossed: false,
        })
    }

    /// Reclassifies current content and mints one opaque boundary permit.
    pub fn reclassify(
        &self,
        sensitivity: &DataSensitivityAssessment,
        clearance: &PreclassificationClearance,
        advisory: Option<&AdvisoryBoundaryDecision>,
    ) -> Result<ReclassificationPermit, ReclassificationError> {
        validate_sensitivity(&self.request, sensitivity)?;
        if !clearance.matches_reclassification_identity(
            &self.request.task_id,
            &self.request.action_id,
            &self.request.policy_fact_sha256,
        ) {
            return Err(ReclassificationError::PolicyMismatch);
        }
        if let Some(advisory) = advisory
            && (advisory.fact_set_sha256() != self.request.policy_fact_sha256
                || advisory.dispositions().iter().any(|disposition| {
                    matches!(
                        disposition,
                        AdvisoryClassifierDisposition::Deny
                            | AdvisoryClassifierDisposition::Narrow
                            | AdvisoryClassifierDisposition::Redact
                            | AdvisoryClassifierDisposition::Isolate
                            | AdvisoryClassifierDisposition::Escalate
                    )
                }))
        {
            return Err(ReclassificationError::AdvisoryRestriction);
        }
        if sensitivity.sensitivity == DataSensitivity::Restricted
            && self.request.destination_boundary == ClassificationBoundary::Network
        {
            return Err(ReclassificationError::SensitivityBoundary);
        }
        Ok(ReclassificationPermit {
            request_id: self.request.request_id.clone(),
            content_kind: self.request.content_kind,
            destination_boundary: self.request.destination_boundary,
            subject_sha256: self.request.subject_sha256.clone(),
            observed_revision: self.request.observed_revision.clone(),
            policy_fact_sha256: self.request.policy_fact_sha256.clone(),
        })
    }

    /// Consumes one exact permit and records the single boundary crossing.
    pub fn cross_boundary(
        &mut self,
        permit: ReclassificationPermit,
    ) -> Result<BoundaryCrossingReceipt, ReclassificationError> {
        if self.crossed
            || permit.request_id != self.request.request_id
            || permit.content_kind != self.request.content_kind
            || permit.destination_boundary != self.request.destination_boundary
            || permit.subject_sha256 != self.request.subject_sha256
            || permit.observed_revision != self.request.observed_revision
            || permit.policy_fact_sha256 != self.request.policy_fact_sha256
        {
            return Err(ReclassificationError::PermitMismatch);
        }
        self.crossed = true;
        Ok(BoundaryCrossingReceipt {
            request_id: permit.request_id,
            content_kind: permit.content_kind,
            destination_boundary: permit.destination_boundary,
            subject_sha256: permit.subject_sha256,
            observed_revision: permit.observed_revision,
            policy_fact_sha256: permit.policy_fact_sha256,
        })
    }
}

fn valid_request(request: &ReclassificationRequest) -> bool {
    request.schema_version == agentmage_kernel_contracts::CONTRACT_SCHEMA_VERSION
        && valid_id(&request.request_id)
        && valid_id(request.task_id.as_str())
        && valid_id(request.action_id.as_str())
        && request.source_boundary != request.destination_boundary
        && valid_sha256(&request.subject_sha256)
        && valid_id(&request.observed_revision)
        && valid_id(&request.sensitivity_assessment_id)
        && valid_sha256(&request.policy_fact_sha256)
}

fn validate_sensitivity(
    request: &ReclassificationRequest,
    sensitivity: &DataSensitivityAssessment,
) -> Result<(), ReclassificationError> {
    if sensitivity.schema_version != agentmage_kernel_contracts::CONTRACT_SCHEMA_VERSION
        || sensitivity.assessment_id != request.sensitivity_assessment_id
        || sensitivity.subject_sha256 != request.subject_sha256
        || sensitivity.evidence.is_empty()
        || sensitivity.evidence.len() > MAX_CLASSIFICATION_EVIDENCE
        || sensitivity.evidence.iter().any(|evidence| {
            evidence.schema_version != agentmage_kernel_contracts::CONTRACT_SCHEMA_VERSION
                || !valid_id(evidence.evidence_id.as_str())
                || evidence.content_sha256 != request.subject_sha256
                || evidence.observed_revision.as_deref() != Some(request.observed_revision.as_str())
        })
    {
        return Err(ReclassificationError::SensitivityMismatch);
    }
    Ok(())
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
    use super::{ReclassificationError, ReclassificationGate};
    use crate::{
        advisory_policy::AdvisoryPolicyGate,
        preclassification_policy::{PreclassificationClearance, PreclassificationPolicyGate},
    };
    use agentmage_kernel_contracts::{
        ActionId, ActionRisk, AdvisoryClassifierDisposition, AdvisoryClassifierResult,
        AdvisoryClassifierStatus, AutonomyLevel, BudgetState, ClassificationBoundary,
        CredentialClass, DataSensitivity, DataSensitivityAssessment, DeterministicPolicyFacts,
        DisclosureClass, EvidenceId, EvidenceKind, EvidenceReference, ExactAuthorityState,
        GrantOperation, ModelCapabilityRole, ModelCapabilityStatus, ModelRunId, NetworkRequirement,
        OperationBinding, PathScopeState, PolicyDestinationClass, PolicyId, PolicySourceClass,
        ReclassificationContentKind, ReclassificationRequest, RepositoryState, SessionId,
        StaticPolicyCheck, StaticPolicyCheckKind, StaticPolicyCheckState, TaskId, from_json,
        to_canonical_json,
    };

    const SHA256: &str = "1111111111111111111111111111111111111111111111111111111111111111";
    const REVISION: &str = "revision-0001";

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

    fn clearance() -> PreclassificationClearance {
        PreclassificationPolicyGate::evaluate(&facts()).expect("clearance")
    }

    fn request(
        clearance: &PreclassificationClearance,
        kind: ReclassificationContentKind,
        source: ClassificationBoundary,
        destination: ClassificationBoundary,
    ) -> ReclassificationRequest {
        ReclassificationRequest {
            schema_version: agentmage_kernel_contracts::CONTRACT_SCHEMA_VERSION,
            request_id: format!("reclassification-{kind:?}-{source:?}-{destination:?}"),
            task_id: TaskId::from_raw("task-0001"),
            action_id: ActionId::from_raw("action-0001"),
            content_kind: kind,
            source_boundary: source,
            destination_boundary: destination,
            subject_sha256: SHA256.to_owned(),
            observed_revision: REVISION.to_owned(),
            sensitivity_assessment_id: "sensitivity-0001".to_owned(),
            policy_fact_sha256: clearance.fact_set_sha256().to_owned(),
        }
    }

    fn sensitivity(label: DataSensitivity) -> DataSensitivityAssessment {
        DataSensitivityAssessment {
            schema_version: agentmage_kernel_contracts::CONTRACT_SCHEMA_VERSION,
            assessment_id: "sensitivity-0001".to_owned(),
            subject_sha256: SHA256.to_owned(),
            sensitivity: label,
            evidence: vec![EvidenceReference {
                schema_version: agentmage_kernel_contracts::CONTRACT_SCHEMA_VERSION,
                evidence_id: EvidenceId::from_raw("evidence-0001"),
                kind: EvidenceKind::Validation,
                source_id: "classifier".to_owned(),
                object_id: "subject".to_owned(),
                fragment: None,
                content_sha256: SHA256.to_owned(),
                observed_revision: Some(REVISION.to_owned()),
            }],
        }
    }

    fn advisory(
        clearance: &PreclassificationClearance,
        dispositions: Vec<AdvisoryClassifierDisposition>,
    ) -> crate::advisory_policy::AdvisoryBoundaryDecision {
        AdvisoryPolicyGate::apply(
            clearance,
            &AdvisoryClassifierResult {
                schema_version: agentmage_kernel_contracts::CONTRACT_SCHEMA_VERSION,
                classifier_run_id: ModelRunId::from_raw("classifier-0001"),
                task_id: TaskId::from_raw("task-0001"),
                action_id: Some(ActionId::from_raw("action-0001")),
                input_sha256: clearance.fact_set_sha256().to_owned(),
                status: AdvisoryClassifierStatus::Complete,
                dispositions,
                confidence_basis_points: None,
                rationale_sha256: SHA256.to_owned(),
                evidence: Vec::new(),
            },
        )
        .expect("advisory decision")
    }

    #[test]
    fn task_12_2_2_5_every_named_content_kind_requires_fresh_reclassification() {
        let clearance = clearance();
        let kinds = [
            ReclassificationContentKind::ReadContent,
            ReclassificationContentKind::ToolOutput,
            ReclassificationContentKind::Patch,
            ReclassificationContentKind::Diff,
            ReclassificationContentKind::Message,
            ReclassificationContentKind::Attachment,
            ReclassificationContentKind::ConnectorResult,
            ReclassificationContentKind::Summary,
            ReclassificationContentKind::Diagnostic,
            ReclassificationContentKind::ExportPayload,
        ];
        for kind in kinds {
            let candidate = request(
                &clearance,
                kind,
                ClassificationBoundary::Kernel,
                ClassificationBoundary::UserDisplay,
            );
            let encoded = to_canonical_json(&candidate).expect("request serializes");
            assert_eq!(
                from_json::<ReclassificationRequest>(&encoded),
                Ok(candidate.clone())
            );
            let mut gate = ReclassificationGate::new(candidate).expect("valid gate");
            let permit = gate
                .reclassify(&sensitivity(DataSensitivity::Ephemeral), &clearance, None)
                .expect("fresh classification");
            let receipt = gate.cross_boundary(permit).expect("crossing");
            assert_eq!(receipt.content_kind(), kind);
            assert_eq!(receipt.subject_sha256(), SHA256);
        }
    }

    #[test]
    fn task_12_2_2_5_all_source_and_destination_boundaries_use_the_same_gate() {
        let clearance = clearance();
        let boundaries = [
            ClassificationBoundary::Ingest,
            ClassificationBoundary::Kernel,
            ClassificationBoundary::ModelContext,
            ClassificationBoundary::ToolInput,
            ClassificationBoundary::Persistence,
            ClassificationBoundary::UserDisplay,
            ClassificationBoundary::Export,
            ClassificationBoundary::Network,
        ];
        let mut observed = 0;
        for source in boundaries {
            for destination in boundaries {
                if source == destination {
                    continue;
                }
                let mut gate = ReclassificationGate::new(request(
                    &clearance,
                    ReclassificationContentKind::ReadContent,
                    source,
                    destination,
                ))
                .expect("valid boundary pair");
                let permit = gate
                    .reclassify(&sensitivity(DataSensitivity::Ephemeral), &clearance, None)
                    .expect("fresh classification");
                assert_eq!(
                    gate.cross_boundary(permit)
                        .expect("crossing")
                        .destination_boundary(),
                    destination
                );
                observed += 1;
            }
        }
        assert_eq!(observed, 56);
    }

    #[test]
    fn task_12_2_2_5_stale_or_mismatched_sensitivity_evidence_is_inert() {
        let clearance = clearance();
        let gate = ReclassificationGate::new(request(
            &clearance,
            ReclassificationContentKind::Attachment,
            ClassificationBoundary::Ingest,
            ClassificationBoundary::Kernel,
        ))
        .expect("valid gate");
        let mut candidates = Vec::new();
        let mut stale = sensitivity(DataSensitivity::Ephemeral);
        stale.evidence[0].observed_revision = Some("revision-old".to_owned());
        candidates.push(stale);
        let mut changed = sensitivity(DataSensitivity::Ephemeral);
        changed.subject_sha256 = "2".repeat(64);
        candidates.push(changed);
        let mut missing = sensitivity(DataSensitivity::Ephemeral);
        missing.evidence.clear();
        candidates.push(missing);
        let mut wrong_id = sensitivity(DataSensitivity::Ephemeral);
        wrong_id.assessment_id = "sensitivity-other".to_owned();
        candidates.push(wrong_id);
        for candidate in candidates {
            assert_eq!(
                gate.reclassify(&candidate, &clearance, None),
                Err(ReclassificationError::SensitivityMismatch)
            );
        }
    }

    #[test]
    fn task_12_2_2_5_task_action_or_policy_fact_drift_cannot_cross() {
        let clearance = clearance();
        let mut candidates = Vec::new();
        let mut task = request(
            &clearance,
            ReclassificationContentKind::ToolOutput,
            ClassificationBoundary::ToolInput,
            ClassificationBoundary::Kernel,
        );
        task.task_id = TaskId::from_raw("task-other");
        candidates.push(task);
        let mut action = request(
            &clearance,
            ReclassificationContentKind::ToolOutput,
            ClassificationBoundary::ToolInput,
            ClassificationBoundary::Kernel,
        );
        action.action_id = ActionId::from_raw("action-other");
        candidates.push(action);
        let mut policy = request(
            &clearance,
            ReclassificationContentKind::ToolOutput,
            ClassificationBoundary::ToolInput,
            ClassificationBoundary::Kernel,
        );
        policy.policy_fact_sha256 = "2".repeat(64);
        candidates.push(policy);
        for candidate in candidates {
            let gate = ReclassificationGate::new(candidate).expect("shape is valid");
            assert_eq!(
                gate.reclassify(&sensitivity(DataSensitivity::Ephemeral), &clearance, None),
                Err(ReclassificationError::PolicyMismatch)
            );
        }
    }

    #[test]
    fn task_12_2_2_5_advisory_restrictions_and_restricted_network_content_stop() {
        let clearance = clearance();
        let gate = ReclassificationGate::new(request(
            &clearance,
            ReclassificationContentKind::Message,
            ClassificationBoundary::Kernel,
            ClassificationBoundary::Network,
        ))
        .expect("valid gate");
        for disposition in [
            AdvisoryClassifierDisposition::Deny,
            AdvisoryClassifierDisposition::Narrow,
            AdvisoryClassifierDisposition::Redact,
            AdvisoryClassifierDisposition::Isolate,
            AdvisoryClassifierDisposition::Escalate,
        ] {
            let decision = advisory(&clearance, vec![disposition]);
            assert_eq!(
                gate.reclassify(
                    &sensitivity(DataSensitivity::Ephemeral),
                    &clearance,
                    Some(&decision),
                ),
                Err(ReclassificationError::AdvisoryRestriction)
            );
        }
        assert_eq!(
            gate.reclassify(&sensitivity(DataSensitivity::Restricted), &clearance, None),
            Err(ReclassificationError::SensitivityBoundary)
        );
    }

    #[test]
    fn task_12_2_2_5_one_permit_crosses_once_and_new_revision_requires_new_work() {
        let clearance = clearance();
        let mut gate = ReclassificationGate::new(request(
            &clearance,
            ReclassificationContentKind::Patch,
            ClassificationBoundary::Kernel,
            ClassificationBoundary::UserDisplay,
        ))
        .expect("valid gate");
        let first = gate
            .reclassify(&sensitivity(DataSensitivity::Ephemeral), &clearance, None)
            .expect("first permit");
        let second = gate
            .reclassify(&sensitivity(DataSensitivity::Ephemeral), &clearance, None)
            .expect("second candidate permit");
        let receipt = gate.cross_boundary(first).expect("first crossing");
        assert_eq!(receipt.observed_revision(), REVISION);
        assert_eq!(receipt.policy_fact_sha256(), clearance.fact_set_sha256());
        assert_eq!(
            gate.cross_boundary(second),
            Err(ReclassificationError::PermitMismatch)
        );

        let mut next = request(
            &clearance,
            ReclassificationContentKind::Patch,
            ClassificationBoundary::Kernel,
            ClassificationBoundary::UserDisplay,
        );
        next.observed_revision = "revision-0002".to_owned();
        let next_gate = ReclassificationGate::new(next).expect("next gate");
        assert_eq!(
            next_gate.reclassify(&sensitivity(DataSensitivity::Ephemeral), &clearance, None,),
            Err(ReclassificationError::SensitivityMismatch)
        );
    }
}
