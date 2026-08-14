//! Separate deterministic and advisory classification contracts.

use crate::{
    ActionId, AuthorityClass, DataSensitivity, EvidenceReference, ModelRunId, OperationBinding,
    PolicyId, SessionId, TaskId,
};

/// Deterministic action-risk class, separate from data and model capability.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum ActionRisk {
    /// Observation or drafting with no requested state change.
    Minimal,
    /// Reversible local state change under exact authority.
    Controlled,
    /// Irreversible local change, remote write, or bounded execution.
    Elevated,
    /// Deployment, credential, or administrative authority.
    Critical,
}

/// Closed role for which one exact model profile may be measured.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum ModelCapabilityRole {
    /// Generative coding and planning.
    CodeGeneration,
    /// Structured tool-call selection without authority.
    ToolSelection,
    /// Advisory safety or content classification.
    AdvisoryClassification,
    /// Embedding or retrieval representation.
    Embedding,
    /// Image or other multimodal interpretation.
    Multimodal,
    /// Narrow specialist behavior.
    Specialist,
    /// Compatibility behavior retained for an older profile.
    LegacyCompatibility,
}

/// Measured disposition for one model capability role.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum ModelCapabilityStatus {
    /// Current evidence meets the declared role threshold.
    MeasuredSupported,
    /// Current evidence does not meet the declared role threshold.
    MeasuredUnsupported,
    /// Required artifact, runtime, resource, or review evidence is unavailable.
    Blocked,
    /// The role has not been measured for this exact profile.
    Unknown,
}

/// Versioned data-sensitivity assessment for one content-addressed subject.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DataSensitivityAssessment {
    /// Contract schema version.
    pub schema_version: u16,
    /// Stable assessment identity.
    pub assessment_id: String,
    /// Lowercase SHA-256 digest of the exact assessed subject.
    pub subject_sha256: String,
    /// Separate sensitivity label.
    pub sensitivity: DataSensitivity,
    /// Current evidence supporting the assessment.
    pub evidence: Vec<EvidenceReference>,
}

/// Versioned deterministic action-risk assessment.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActionRiskAssessment {
    /// Contract schema version.
    pub schema_version: u16,
    /// Stable assessment identity.
    pub assessment_id: String,
    /// Exact action identity.
    pub action_id: ActionId,
    /// Separate risk label.
    pub risk: ActionRisk,
    /// Canonical authority class used to derive risk.
    pub authority_class: AuthorityClass,
    /// Whether the represented action is mechanically reversible.
    pub reversible: bool,
    /// Current evidence supporting the assessment.
    pub evidence: Vec<EvidenceReference>,
}

/// Versioned measured capability of one exact model profile and artifact.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelCapabilityAssessment {
    /// Contract schema version.
    pub schema_version: u16,
    /// Stable assessment identity.
    pub assessment_id: String,
    /// Exact selected model-profile identity.
    pub model_profile_id: String,
    /// Lowercase SHA-256 digest of the exact model artifact or manifest.
    pub model_artifact_sha256: String,
    /// Separately measured model role.
    pub role: ModelCapabilityRole,
    /// Current measured role disposition.
    pub status: ModelCapabilityStatus,
    /// Current evidence supporting the disposition.
    pub evidence: Vec<EvidenceReference>,
}

/// Closed user-autonomy ceiling supplied to deterministic policy.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum AutonomyLevel {
    /// No agent operation is enabled.
    Disabled,
    /// Inspection and descriptive output only.
    Inspect,
    /// Exact operations confined to an approved workspace.
    WorkspaceAutonomous,
    /// Explicitly approved connected operations may be considered.
    ConnectedOperations,
    /// Bounded directly activated unrestricted-session authority ceiling.
    Owner,
}

/// Closed origin class for content or an operation request.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum PolicySourceClass {
    /// Approved local workspace content.
    LocalWorkspace,
    /// User-supplied attachment.
    Attachment,
    /// Registered tool output.
    ToolOutput,
    /// Connector or provider response.
    Connector,
    /// Direct network content.
    Network,
    /// Model- or program-generated content.
    Generated,
    /// Origin cannot be established.
    Unknown,
}

/// Closed destination class evaluated before an effect or disclosure.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum PolicyDestinationClass {
    /// No destination is required.
    None,
    /// Approved local workspace.
    LocalWorkspace,
    /// Encrypted local operational store.
    LocalStore,
    /// Bounded local process.
    LocalProcess,
    /// User-visible local interface.
    UserDisplay,
    /// Approved remote provider or service.
    RemoteService,
    /// User-directed export artifact.
    Export,
    /// Destination cannot be established.
    Unknown,
}

/// Closed repository state supplied to policy.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum RepositoryState {
    /// Repository is clean at the observed head.
    Clean,
    /// Repository contains tracked or untracked changes.
    Dirty,
    /// Repository contains unresolved conflicts.
    Conflicted,
    /// Repository is at a detached head.
    Detached,
    /// Repository has no initial commit.
    Unborn,
    /// Target is not a repository.
    NotRepository,
    /// Repository state cannot be established.
    Unknown,
}

/// Closed credential-presence class supplied to policy.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum CredentialClass {
    /// No credential is required or present.
    None,
    /// Credential is available only through an approved broker.
    Brokered,
    /// Credential is exposed through process environment.
    Environment,
    /// Credential appears in file-backed content.
    File,
    /// Credential is held by an operating-system credential store.
    PlatformStore,
    /// Credential state cannot be established.
    Unknown,
}

/// Closed external-disclosure class supplied to policy.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum DisclosureClass {
    /// No disclosure occurs.
    None,
    /// Content remains local to the current user boundary.
    LocalOnly,
    /// Disclosure remains within the same authenticated tenant or account boundary.
    SameTenant,
    /// Disclosure reaches an external party or tenant.
    External,
    /// Disclosure is publicly accessible.
    Public,
    /// Disclosure boundary cannot be established.
    Unknown,
}

/// Closed network requirement supplied to policy.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum NetworkRequirement {
    /// No network access is required.
    None,
    /// Loopback-only communication is required.
    Loopback,
    /// Local-network communication is required.
    LocalNetwork,
    /// Internet communication is required.
    Internet,
    /// Network requirement cannot be established.
    Unknown,
}

/// Closed path-scope disposition supplied to policy.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum PathScopeState {
    /// Exact path is inside current approved scope.
    InScope,
    /// Exact path is outside current approved scope.
    OutOfScope,
    /// Path resolution is ambiguous or changed during evaluation.
    Ambiguous,
    /// Path does not exist at evaluation time.
    Missing,
    /// No path applies to the operation.
    NotApplicable,
}

/// Closed budget disposition supplied to policy.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum BudgetState {
    /// Current use remains below every applicable ceiling.
    Within,
    /// Current use is exactly at an applicable ceiling.
    AtLimit,
    /// At least one applicable ceiling is exceeded.
    Exceeded,
    /// Budget state cannot be established.
    Unknown,
}

/// Closed exact-authority disposition supplied to policy.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum ExactAuthorityState {
    /// No authority is present.
    None,
    /// User review is pending and grants no authority.
    Pending,
    /// One exact current grant is available for policy evaluation.
    CurrentExact,
    /// Named authority is stale or mismatched.
    Stale,
    /// Named authority was already consumed.
    Consumed,
    /// Authority or a possible effect cannot be reconciled.
    Uncertain,
}

/// Static deterministic check identity.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum StaticPolicyCheckKind {
    /// Secret or credential material.
    Secret,
    /// Path identity and scope.
    Path,
    /// Executable, macro, script, or active content.
    ExecutableContent,
    /// Destination identity and eligibility.
    Destination,
}

/// Result of one static deterministic check.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum StaticPolicyCheckState {
    /// Check established no represented hazard.
    Clear,
    /// Check detected the represented hazard.
    Detected,
    /// Check could not establish a result.
    Unknown,
    /// Check does not apply to the represented operation.
    NotApplicable,
}

/// One typed static check supplied to deterministic policy.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StaticPolicyCheck {
    /// Exact check identity.
    pub kind: StaticPolicyCheckKind,
    /// Deterministic check result.
    pub state: StaticPolicyCheckState,
    /// Lowercase SHA-256 digest of the checked subject or observation.
    pub observation_sha256: String,
}

/// Complete typed current facts consumed by deterministic deny-first policy.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeterministicPolicyFacts {
    /// Contract schema version.
    pub schema_version: u16,
    /// Stable fact-set identity.
    pub fact_set_id: String,
    /// Exact deterministic policy identity.
    pub policy_id: PolicyId,
    /// Lowercase SHA-256 digest of the exact policy revision.
    pub policy_sha256: String,
    /// Exact actor identity as a bounded pseudonymous wire value.
    pub actor_id: String,
    /// Exact session identity.
    pub session_id: SessionId,
    /// Exact task identity.
    pub task_id: TaskId,
    /// Exact action identity.
    pub action_id: ActionId,
    /// Current user-autonomy ceiling.
    pub autonomy: AutonomyLevel,
    /// Canonical requested operation and authority class.
    pub operation: OperationBinding,
    /// Origin class of the input or request.
    pub source: PolicySourceClass,
    /// Lowercase SHA-256 digest of the exact source identity.
    pub source_sha256: String,
    /// Destination class of the proposed output or effect.
    pub destination: PolicyDestinationClass,
    /// Lowercase SHA-256 digest of the exact destination identity.
    pub destination_sha256: String,
    /// Current path-scope state.
    pub path_state: PathScopeState,
    /// Lowercase SHA-256 digest of the resolved path identity, or a fixed no-path digest.
    pub path_sha256: String,
    /// Current repository state.
    pub repository_state: RepositoryState,
    /// Current credential class.
    pub credential_class: CredentialClass,
    /// Current data-sensitivity label.
    pub data_sensitivity: DataSensitivity,
    /// Current deterministic action-risk label.
    pub action_risk: ActionRisk,
    /// Measured model role relevant to the requested operation.
    pub model_capability_role: ModelCapabilityRole,
    /// Current measured model-role disposition.
    pub model_capability_status: ModelCapabilityStatus,
    /// Whether the requested action is mechanically reversible.
    pub reversible: bool,
    /// Current network requirement.
    pub network: NetworkRequirement,
    /// Current external-disclosure class.
    pub disclosure: DisclosureClass,
    /// Current budget state.
    pub budget: BudgetState,
    /// Current exact-authority state.
    pub exact_authority: ExactAuthorityState,
    /// Complete ordered static checks performed before semantic classification.
    pub static_checks: Vec<StaticPolicyCheck>,
}

/// Advisory classifier availability and quality state.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum AdvisoryClassifierStatus {
    /// One complete schema-valid advisory result is available.
    Complete,
    /// Result confidence is below its declared review threshold.
    LowConfidence,
    /// Output ended before a complete result was available.
    Truncated,
    /// Multiple advisory results disagree materially.
    Disagreement,
    /// Classifier is not available.
    Unavailable,
    /// Input is outside the classifier's measured distribution.
    OutOfDistribution,
    /// Classifier exceeded its deadline.
    TimedOut,
    /// Output does not match the closed advisory schema.
    Malformed,
}

/// Closed authority-reducing or user-escalating advisory disposition.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum AdvisoryClassifierDisposition {
    /// Deny the represented continuation.
    Deny,
    /// Narrow the represented scope or capability set.
    Narrow,
    /// Require typed redaction before another boundary.
    Redact,
    /// Require stronger process, storage, or context isolation.
    Isolate,
    /// Require an explicit user decision.
    Escalate,
}

/// Versioned non-authoritative learned or model-based classifier result.
///
/// This record has no grant, operation, destination, model-selection, execution, or
/// completion field. It can only propose one or more closed restrictive dispositions.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AdvisoryClassifierResult {
    /// Contract schema version.
    pub schema_version: u16,
    /// Exact model or learned-classifier run identity.
    pub classifier_run_id: ModelRunId,
    /// Exact task identity.
    pub task_id: TaskId,
    /// Exact action identity when one is being classified.
    #[serde(deserialize_with = "crate::serialization::deserialize_required_option")]
    pub action_id: Option<ActionId>,
    /// Lowercase SHA-256 digest of the exact classified input.
    pub input_sha256: String,
    /// Classifier availability and quality state.
    pub status: AdvisoryClassifierStatus,
    /// Ordered restrictive dispositions proposed by the classifier.
    pub dispositions: Vec<AdvisoryClassifierDisposition>,
    /// Optional integer confidence in basis points; never an authority decision.
    #[serde(deserialize_with = "crate::serialization::deserialize_required_option")]
    pub confidence_basis_points: Option<u16>,
    /// Lowercase SHA-256 digest of bounded rationale or result material.
    pub rationale_sha256: String,
    /// Current evidence supporting only the advisory result.
    pub evidence: Vec<EvidenceReference>,
}

#[cfg(test)]
mod tests {
    use super::{
        ActionRisk, ActionRiskAssessment, AdvisoryClassifierDisposition, AdvisoryClassifierResult,
        AdvisoryClassifierStatus, AutonomyLevel, BudgetState, CredentialClass,
        DataSensitivityAssessment, DeterministicPolicyFacts, DisclosureClass, ExactAuthorityState,
        ModelCapabilityAssessment, ModelCapabilityRole, ModelCapabilityStatus, NetworkRequirement,
        PathScopeState, PolicyDestinationClass, PolicySourceClass, RepositoryState,
        StaticPolicyCheck, StaticPolicyCheckKind, StaticPolicyCheckState,
    };
    use crate::{
        ActionId, AuthorityClass, DataSensitivity, GrantOperation, ModelRunId, OperationBinding,
        PolicyId, SessionId, TaskId, from_json, to_canonical_json,
    };

    const SHA256: &str = "1111111111111111111111111111111111111111111111111111111111111111";

    fn facts() -> DeterministicPolicyFacts {
        DeterministicPolicyFacts {
            schema_version: crate::CONTRACT_SCHEMA_VERSION,
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
            model_capability_role: ModelCapabilityRole::CodeGeneration,
            model_capability_status: ModelCapabilityStatus::MeasuredSupported,
            reversible: true,
            network: NetworkRequirement::None,
            disclosure: DisclosureClass::LocalOnly,
            budget: BudgetState::Within,
            exact_authority: ExactAuthorityState::None,
            static_checks: vec![
                StaticPolicyCheck {
                    kind: StaticPolicyCheckKind::Secret,
                    state: StaticPolicyCheckState::Clear,
                    observation_sha256: SHA256.to_owned(),
                },
                StaticPolicyCheck {
                    kind: StaticPolicyCheckKind::Path,
                    state: StaticPolicyCheckState::Clear,
                    observation_sha256: SHA256.to_owned(),
                },
                StaticPolicyCheck {
                    kind: StaticPolicyCheckKind::ExecutableContent,
                    state: StaticPolicyCheckState::NotApplicable,
                    observation_sha256: SHA256.to_owned(),
                },
                StaticPolicyCheck {
                    kind: StaticPolicyCheckKind::Destination,
                    state: StaticPolicyCheckState::Clear,
                    observation_sha256: SHA256.to_owned(),
                },
            ],
        }
    }

    #[test]
    fn task_12_2_2_1_data_sensitivity_schema_covers_its_separate_dimension() {
        for (index, sensitivity) in [
            DataSensitivity::Ephemeral,
            DataSensitivity::Operational,
            DataSensitivity::Durable,
            DataSensitivity::Restricted,
        ]
        .into_iter()
        .enumerate()
        {
            let assessment = DataSensitivityAssessment {
                schema_version: crate::CONTRACT_SCHEMA_VERSION,
                assessment_id: format!("sensitivity-{index}"),
                subject_sha256: SHA256.to_owned(),
                sensitivity,
                evidence: Vec::new(),
            };
            let bytes = to_canonical_json(&assessment).expect("assessment serializes");
            assert_eq!(
                from_json::<DataSensitivityAssessment>(&bytes),
                Ok(assessment)
            );
        }
    }

    #[test]
    fn task_12_2_2_1_action_risk_schema_is_not_a_data_or_capability_label() {
        for (index, risk) in [
            ActionRisk::Minimal,
            ActionRisk::Controlled,
            ActionRisk::Elevated,
            ActionRisk::Critical,
        ]
        .into_iter()
        .enumerate()
        {
            let assessment = ActionRiskAssessment {
                schema_version: crate::CONTRACT_SCHEMA_VERSION,
                assessment_id: format!("risk-{index}"),
                action_id: ActionId::from_raw("action-0001"),
                risk,
                authority_class: AuthorityClass::Observe,
                reversible: true,
                evidence: Vec::new(),
            };
            let encoded = to_canonical_json(&assessment).expect("assessment serializes");
            let value: serde_json::Value = serde_json::from_slice(&encoded).expect("valid JSON");
            assert!(value.get("risk").is_some());
            assert!(value.get("sensitivity").is_none());
            assert!(value.get("model_profile_id").is_none());
            assert_eq!(from_json::<ActionRiskAssessment>(&encoded), Ok(assessment));
        }
    }

    #[test]
    fn task_12_2_2_1_model_capability_schema_closes_every_role_and_status() {
        let roles = [
            ModelCapabilityRole::CodeGeneration,
            ModelCapabilityRole::ToolSelection,
            ModelCapabilityRole::AdvisoryClassification,
            ModelCapabilityRole::Embedding,
            ModelCapabilityRole::Multimodal,
            ModelCapabilityRole::Specialist,
            ModelCapabilityRole::LegacyCompatibility,
        ];
        let statuses = [
            ModelCapabilityStatus::MeasuredSupported,
            ModelCapabilityStatus::MeasuredUnsupported,
            ModelCapabilityStatus::Blocked,
            ModelCapabilityStatus::Unknown,
        ];
        for (role_index, role) in roles.into_iter().enumerate() {
            for (status_index, status) in statuses.into_iter().enumerate() {
                let assessment = ModelCapabilityAssessment {
                    schema_version: crate::CONTRACT_SCHEMA_VERSION,
                    assessment_id: format!("capability-{role_index}-{status_index}"),
                    model_profile_id: "model-profile-0001".to_owned(),
                    model_artifact_sha256: SHA256.to_owned(),
                    role,
                    status,
                    evidence: Vec::new(),
                };
                let bytes = to_canonical_json(&assessment).expect("assessment serializes");
                assert_eq!(
                    from_json::<ModelCapabilityAssessment>(&bytes),
                    Ok(assessment)
                );
            }
        }
    }

    #[test]
    fn task_12_2_2_1_deterministic_policy_facts_are_complete_and_advisory_free() {
        let facts = facts();
        let encoded = to_canonical_json(&facts).expect("facts serialize");
        let value: serde_json::Value = serde_json::from_slice(&encoded).expect("valid JSON");
        for required in [
            "policy_id",
            "actor_id",
            "session_id",
            "task_id",
            "action_id",
            "autonomy",
            "operation",
            "source",
            "destination",
            "path_state",
            "repository_state",
            "credential_class",
            "data_sensitivity",
            "action_risk",
            "model_capability_role",
            "reversible",
            "network",
            "disclosure",
            "budget",
            "exact_authority",
            "static_checks",
        ] {
            assert!(value.get(required).is_some(), "missing {required}");
        }
        for advisory in [
            "confidence",
            "rationale",
            "classifier_run_id",
            "dispositions",
        ] {
            assert!(value.get(advisory).is_none());
        }
        assert_eq!(from_json::<DeterministicPolicyFacts>(&encoded), Ok(facts));
    }

    #[test]
    fn task_12_2_2_1_advisory_schema_has_only_restrictive_dispositions() {
        let statuses = [
            AdvisoryClassifierStatus::Complete,
            AdvisoryClassifierStatus::LowConfidence,
            AdvisoryClassifierStatus::Truncated,
            AdvisoryClassifierStatus::Disagreement,
            AdvisoryClassifierStatus::Unavailable,
            AdvisoryClassifierStatus::OutOfDistribution,
            AdvisoryClassifierStatus::TimedOut,
            AdvisoryClassifierStatus::Malformed,
        ];
        let dispositions = [
            AdvisoryClassifierDisposition::Deny,
            AdvisoryClassifierDisposition::Narrow,
            AdvisoryClassifierDisposition::Redact,
            AdvisoryClassifierDisposition::Isolate,
            AdvisoryClassifierDisposition::Escalate,
        ];
        for (index, status) in statuses.into_iter().enumerate() {
            let advisory = AdvisoryClassifierResult {
                schema_version: crate::CONTRACT_SCHEMA_VERSION,
                classifier_run_id: ModelRunId::from_raw(format!("classifier-{index}")),
                task_id: TaskId::from_raw("task-0001"),
                action_id: Some(ActionId::from_raw("action-0001")),
                input_sha256: SHA256.to_owned(),
                status,
                dispositions: dispositions.to_vec(),
                confidence_basis_points: Some(5_000),
                rationale_sha256: SHA256.to_owned(),
                evidence: Vec::new(),
            };
            let encoded = to_canonical_json(&advisory).expect("advisory serializes");
            let value: serde_json::Value = serde_json::from_slice(&encoded).expect("valid JSON");
            for prohibited in [
                "grant_id",
                "operation",
                "destination",
                "selected_model",
                "execute",
                "completion",
            ] {
                assert!(value.get(prohibited).is_none());
            }
            assert_eq!(
                from_json::<AdvisoryClassifierResult>(&encoded),
                Ok(advisory)
            );
        }
    }

    #[test]
    fn task_12_2_2_1_unknown_cross_schema_and_authority_fields_fail_closed() {
        let encoded = to_canonical_json(&facts()).expect("facts serialize");
        assert!(from_json::<AdvisoryClassifierResult>(&encoded).is_err());

        let advisory = AdvisoryClassifierResult {
            schema_version: crate::CONTRACT_SCHEMA_VERSION,
            classifier_run_id: ModelRunId::from_raw("classifier-0001"),
            task_id: TaskId::from_raw("task-0001"),
            action_id: None,
            input_sha256: SHA256.to_owned(),
            status: AdvisoryClassifierStatus::Complete,
            dispositions: vec![AdvisoryClassifierDisposition::Deny],
            confidence_basis_points: None,
            rationale_sha256: SHA256.to_owned(),
            evidence: Vec::new(),
        };
        let mut value = serde_json::to_value(advisory).expect("advisory JSON");
        value["grant_id"] = serde_json::Value::String("grant-forged".to_owned());
        assert!(
            from_json::<AdvisoryClassifierResult>(
                &serde_json::to_vec(&value).expect("candidate JSON")
            )
            .is_err()
        );
        value.as_object_mut().expect("object").remove("grant_id");
        value["status"] = serde_json::Value::String("allow".to_owned());
        assert!(
            from_json::<AdvisoryClassifierResult>(
                &serde_json::to_vec(&value).expect("candidate JSON")
            )
            .is_err()
        );
    }
}
