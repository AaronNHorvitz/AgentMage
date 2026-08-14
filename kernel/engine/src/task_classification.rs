//! Deterministic descriptive task intent, complexity, and risk classification.

use agentmage_kernel_contracts::{AuthorityClass, TaskId, ValidationIssue, WorkPacket};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::fmt::Write;

use crate::work_packet::validate_packet;

const FOCUSED_MAX_WORK_ITEMS: usize = 8;
const FOCUSED_MAX_BUDGETS: usize = 8;
const BOUNDED_MAX_ACCEPTANCE_CHECKS: usize = 16;
const BOUNDED_MAX_WORK_ITEMS: usize = 64;

/// Closed user-facing task-intent vocabulary.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskIntent {
    /// Produce a direct evidence-bounded answer.
    Answer,
    /// Explain observed or supplied material.
    Explain,
    /// Review an artifact or state without changing it.
    Review,
    /// Diagnose a bounded problem from evidence.
    Diagnose,
    /// Produce or revise a non-authoritative plan.
    Plan,
    /// Propose work that may require a state-changing authority class.
    Change,
    /// Observe a bounded condition over a declared run.
    Monitor,
    /// Wait for a declared event without creating background authority.
    Wait,
}

impl TaskIntent {
    /// Every intent in stable vocabulary order.
    pub const ALL: [Self; 8] = [
        Self::Answer,
        Self::Explain,
        Self::Review,
        Self::Diagnose,
        Self::Plan,
        Self::Change,
        Self::Monitor,
        Self::Wait,
    ];
}

/// Deterministic task-shape complexity class.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskComplexity {
    /// One acceptance check and a small bounded supporting shape.
    Focused,
    /// Multiple bounded checks or supporting records remain within the medium threshold.
    Bounded,
    /// The valid packet exceeds the bounded threshold and requires decomposition review.
    Extended,
}

/// Deterministic action-risk class derived without model judgment.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskRisk {
    /// Observation or drafting has no requested state-changing authority class.
    Minimal,
    /// A reversible local write is requested.
    Controlled,
    /// An irreversible local write, remote write, or execution is requested.
    Elevated,
    /// Deployment, credential access, or administration is requested.
    Critical,
}

/// Typed reason a packet cannot receive deterministic task classification.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TaskClassificationError {
    /// The source packet failed closed validation.
    InvalidPacket {
        /// Exact ordered validation findings.
        issues: Vec<ValidationIssue>,
    },
}

/// Content-free descriptive classification of one exact packet revision.
///
/// This record carries no capability, grant, operation, completion, or execution method. The
/// kernel authority module seals it as an always-denied descriptive authority candidate.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TaskClassification {
    task_id: TaskId,
    work_packet_revision: u32,
    intent: TaskIntent,
    complexity: TaskComplexity,
    risk: TaskRisk,
    required_authority_class: AuthorityClass,
    classification_sha256: String,
}

impl TaskClassification {
    /// Returns the exact classified task identity.
    #[must_use]
    pub const fn task_id(&self) -> &TaskId {
        &self.task_id
    }

    /// Returns the exact work-packet revision used for classification.
    #[must_use]
    pub const fn work_packet_revision(&self) -> u32 {
        self.work_packet_revision
    }

    /// Returns the explicit descriptive intent.
    #[must_use]
    pub const fn intent(&self) -> TaskIntent {
        self.intent
    }

    /// Returns deterministic task-shape complexity.
    #[must_use]
    pub const fn complexity(&self) -> TaskComplexity {
        self.complexity
    }

    /// Returns deterministic action risk.
    #[must_use]
    pub const fn risk(&self) -> TaskRisk {
        self.risk
    }

    /// Returns the packet's descriptive required authority class without granting it.
    #[must_use]
    pub const fn required_authority_class(&self) -> AuthorityClass {
        self.required_authority_class
    }

    /// Returns the deterministic classification identity.
    #[must_use]
    pub fn sha256(&self) -> &str {
        &self.classification_sha256
    }
}

#[derive(Serialize)]
struct ClassificationMaterial<'a> {
    schema_version: u16,
    record_type: &'static str,
    task_id: &'a str,
    work_packet_revision: u32,
    intent: TaskIntent,
    complexity: TaskComplexity,
    risk: TaskRisk,
    required_authority_class: AuthorityClass,
    acceptance_check_count: usize,
    evidence_class_count: usize,
    authoritative_evidence_count: usize,
    mutable_file_count: usize,
    protected_file_count: usize,
    budget_count: usize,
    rollback_reversible: bool,
}

/// Classifies one exact valid work packet without granting authority or establishing completion.
pub fn classify_task(
    packet: &WorkPacket,
    intent: TaskIntent,
) -> Result<TaskClassification, TaskClassificationError> {
    let issues = validate_packet(packet);
    if !issues.is_empty() {
        return Err(TaskClassificationError::InvalidPacket { issues });
    }
    let work_items = packet
        .acceptance_checks
        .len()
        .saturating_add(packet.required_evidence.len())
        .saturating_add(packet.authoritative_evidence.len())
        .saturating_add(packet.mutable_files.len())
        .saturating_add(packet.protected_files.len());
    let complexity = if packet.acceptance_checks.len() == 1
        && work_items <= FOCUSED_MAX_WORK_ITEMS
        && packet.budgets.len() <= FOCUSED_MAX_BUDGETS
    {
        TaskComplexity::Focused
    } else if packet.acceptance_checks.len() <= BOUNDED_MAX_ACCEPTANCE_CHECKS
        && work_items <= BOUNDED_MAX_WORK_ITEMS
    {
        TaskComplexity::Bounded
    } else {
        TaskComplexity::Extended
    };
    let risk = risk_for(packet.required_capability_class, packet.rollback.reversible);
    let material = ClassificationMaterial {
        schema_version: agentmage_kernel_contracts::CONTRACT_SCHEMA_VERSION,
        record_type: "agentmage-task-classification-v1",
        task_id: packet.task_id.as_str(),
        work_packet_revision: packet.revision,
        intent,
        complexity,
        risk,
        required_authority_class: packet.required_capability_class,
        acceptance_check_count: packet.acceptance_checks.len(),
        evidence_class_count: packet.required_evidence.len(),
        authoritative_evidence_count: packet.authoritative_evidence.len(),
        mutable_file_count: packet.mutable_files.len(),
        protected_file_count: packet.protected_files.len(),
        budget_count: packet.budgets.len(),
        rollback_reversible: packet.rollback.reversible,
    };
    let bytes = serde_json::to_vec(&material).expect("fixed classification material serializes");
    Ok(TaskClassification {
        task_id: packet.task_id.clone(),
        work_packet_revision: packet.revision,
        intent,
        complexity,
        risk,
        required_authority_class: packet.required_capability_class,
        classification_sha256: sha256_hex(&bytes),
    })
}

fn sha256_hex(value: &[u8]) -> String {
    let mut encoded = String::with_capacity(64);
    for byte in Sha256::digest(value) {
        write!(&mut encoded, "{byte:02x}").expect("writing to String cannot fail");
    }
    encoded
}

const fn risk_for(authority: AuthorityClass, reversible: bool) -> TaskRisk {
    match authority {
        AuthorityClass::Observe | AuthorityClass::Draft => TaskRisk::Minimal,
        AuthorityClass::LocalWrite if reversible => TaskRisk::Controlled,
        AuthorityClass::LocalWrite | AuthorityClass::RemoteWrite | AuthorityClass::Execute => {
            TaskRisk::Elevated
        }
        AuthorityClass::Deploy | AuthorityClass::Secrets | AuthorityClass::Admin => {
            TaskRisk::Critical
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{TaskClassificationError, TaskComplexity, TaskIntent, TaskRisk, classify_task};
    use crate::authority::{DescriptiveArtifactKind, reject_as_authority};
    use agentmage_kernel_contracts::{
        AuthorityClass, BudgetLimit, BudgetResource, CONTRACT_SCHEMA_VERSION, DataSensitivity,
        EvidenceKind, PlanId, RollbackPlan, StopCondition, StopConditionKind, TaskId, WorkPacket,
        WorkPacketId, WorkPacketState,
    };

    fn packet() -> WorkPacket {
        WorkPacket {
            schema_version: CONTRACT_SCHEMA_VERSION,
            work_packet_id: WorkPacketId::from_raw("packet-classification-0001"),
            task_id: TaskId::from_raw("task-classification-0001"),
            revision: 1,
            objective: "Classify one synthetic task".to_owned(),
            reason: "Exercise deterministic task classification".to_owned(),
            owner: "synthetic-user".to_owned(),
            authoritative_evidence: Vec::new(),
            mutable_files: Vec::new(),
            protected_files: vec!["fixtures/input.txt".to_owned()],
            expected_output: "One content-free classification".to_owned(),
            acceptance_checks: vec!["Record classification".to_owned()],
            required_evidence: vec![EvidenceKind::Validation],
            required_capability_class: AuthorityClass::Observe,
            budgets: vec![BudgetLimit {
                resource: BudgetResource::PlanSteps,
                limit: 1,
            }],
            stop_conditions: [
                StopConditionKind::AcceptanceSatisfied,
                StopConditionKind::UserDecisionRequired,
                StopConditionKind::PolicyDenied,
                StopConditionKind::Error,
                StopConditionKind::Cancelled,
                StopConditionKind::BudgetExhausted,
                StopConditionKind::UncertainResult,
            ]
            .into_iter()
            .map(|kind| StopCondition {
                kind,
                description: format!("Synthetic {kind:?} stop"),
            })
            .collect(),
            rollback: RollbackPlan {
                reversible: true,
                description: "No state change is requested".to_owned(),
            },
            sensitivity: DataSensitivity::Ephemeral,
            last_verification_date: "2026-08-13".to_owned(),
            next_action: None,
            next_review: None,
            status_reason: None,
            disposition: None,
            completion_evidence: Vec::new(),
            superseding_work: None,
            validation_issues: Vec::new(),
            plan_id: Some(PlanId::from_raw("plan-classification-0001")),
            state: WorkPacketState::Active,
        }
    }

    #[test]
    fn every_intent_is_stable_descriptive_and_never_authority() {
        let packet = packet();
        for intent in TaskIntent::ALL {
            let first = classify_task(&packet, intent).expect("valid classification");
            let repeated = classify_task(&packet, intent).expect("repeat classification");
            assert_eq!(first, repeated);
            assert_eq!(first.task_id(), &packet.task_id);
            assert_eq!(first.work_packet_revision(), 1);
            assert_eq!(first.intent(), intent);
            assert_eq!(first.complexity(), TaskComplexity::Focused);
            assert_eq!(first.risk(), TaskRisk::Minimal);
            assert_eq!(first.sha256().len(), 64);
            let denial = reject_as_authority(&first);
            assert_eq!(
                denial.artifact_kind,
                DescriptiveArtifactKind::TaskClassification
            );
            assert_eq!(denial.error.code, "authority.descriptive_artifact.denied");
        }
    }

    #[test]
    fn risk_is_derived_only_from_authority_class_and_reversibility() {
        let cases = [
            (AuthorityClass::Observe, true, TaskRisk::Minimal),
            (AuthorityClass::Draft, false, TaskRisk::Minimal),
            (AuthorityClass::LocalWrite, true, TaskRisk::Controlled),
            (AuthorityClass::LocalWrite, false, TaskRisk::Elevated),
            (AuthorityClass::RemoteWrite, true, TaskRisk::Elevated),
            (AuthorityClass::Execute, true, TaskRisk::Elevated),
            (AuthorityClass::Deploy, true, TaskRisk::Critical),
            (AuthorityClass::Secrets, true, TaskRisk::Critical),
            (AuthorityClass::Admin, true, TaskRisk::Critical),
        ];
        for (authority, reversible, expected) in cases {
            let mut candidate = packet();
            candidate.required_capability_class = authority;
            candidate.rollback.reversible = reversible;
            for intent in TaskIntent::ALL {
                let classification = classify_task(&candidate, intent).expect("valid risk fixture");
                assert_eq!(classification.risk(), expected);
                assert_eq!(classification.required_authority_class(), authority);
            }
        }
    }

    #[test]
    fn complexity_thresholds_are_exact_and_do_not_change_risk() {
        let focused = classify_task(&packet(), TaskIntent::Review).expect("focused");
        assert_eq!(focused.complexity(), TaskComplexity::Focused);

        let mut bounded_packet = packet();
        bounded_packet
            .acceptance_checks
            .push("Second check".to_owned());
        let bounded = classify_task(&bounded_packet, TaskIntent::Review).expect("bounded");
        assert_eq!(bounded.complexity(), TaskComplexity::Bounded);
        assert_eq!(bounded.risk(), TaskRisk::Minimal);

        let mut extended_packet = packet();
        extended_packet.acceptance_checks = (0..17)
            .map(|index| format!("Acceptance check {index}"))
            .collect();
        let extended = classify_task(&extended_packet, TaskIntent::Review).expect("extended");
        assert_eq!(extended.complexity(), TaskComplexity::Extended);
        assert_eq!(extended.risk(), TaskRisk::Minimal);
    }

    #[test]
    fn invalid_packet_fails_without_classification_or_authority() {
        let mut invalid = packet();
        invalid.objective.clear();
        let Err(TaskClassificationError::InvalidPacket { issues }) =
            classify_task(&invalid, TaskIntent::Change)
        else {
            panic!("invalid packet must fail")
        };
        assert!(issues.iter().any(|issue| issue.field_path == ["objective"]));
    }

    #[test]
    fn intent_changes_identity_but_never_risk_or_required_authority() {
        let mut packet = packet();
        packet.required_capability_class = AuthorityClass::LocalWrite;
        let review = classify_task(&packet, TaskIntent::Review).expect("review label");
        let change = classify_task(&packet, TaskIntent::Change).expect("change label");
        assert_ne!(review.sha256(), change.sha256());
        assert_eq!(review.risk(), change.risk());
        assert_eq!(
            review.required_authority_class(),
            change.required_authority_class()
        );
    }
}
