//! Stateful resource budgets and explicit sticky stop conditions.

use std::collections::{BTreeMap, BTreeSet};

use agentmage_kernel_contracts::{
    BudgetResource, StopConditionKind, TaskId, ValidationIssue, WorkPacket, WorkPacketId,
    WorkPacketState,
};

use crate::work_packet::{validate_completion, validate_packet};

/// Exact terminal reason retained by one bounded run controller.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RunStopReason {
    /// One declared stop condition was signaled.
    Condition(StopConditionKind),
    /// A resource attempt would exceed its inclusive declared limit.
    BudgetExceeded {
        /// Resource whose ceiling would be exceeded.
        resource: BudgetResource,
        /// Inclusive declared ceiling.
        limit: u64,
        /// Total that the rejected attempt would have produced.
        attempted_total: u64,
    },
    /// A resource was requested without a declared budget.
    BudgetNotDeclared {
        /// Undeclared resource.
        resource: BudgetResource,
    },
    /// Usage addition overflowed before it could be compared safely.
    CounterOverflow {
        /// Resource whose usage counter overflowed.
        resource: BudgetResource,
    },
}

/// Decision returned after a resource or stop-condition event.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RunDecision {
    /// The run remains eligible to propose another action.
    Continue {
        /// Resource updated by the event.
        resource: BudgetResource,
        /// Admitted cumulative usage.
        used: u64,
        /// Remaining declared capacity.
        remaining: u64,
    },
    /// The run is terminal and retains the first exact reason.
    Stop {
        /// Sticky terminal reason.
        reason: RunStopReason,
    },
}

/// Typed reason a run controller cannot be created or explicitly stopped.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RunControlError {
    /// The source packet failed closed validation.
    InvalidPacket {
        /// Exact ordered validation findings.
        issues: Vec<ValidationIssue>,
    },
    /// A run controller may begin only from active work.
    PacketNotActive,
    /// A caller signaled a condition not declared by the packet.
    ConditionNotDeclared {
        /// Undeclared condition.
        condition: StopConditionKind,
    },
    /// Budget exhaustion must be established by resource accounting.
    DirectBudgetSignalProhibited,
    /// Acceptance satisfaction must be established by completion evidence.
    DirectCompletionSignalProhibited,
    /// A completion candidate changed stable packet or task identity.
    CompletionIdentityChanged,
    /// A completion candidate did not advance the packet revision.
    CompletionRevisionNotNewer,
    /// A completion candidate failed packet or evidence validation.
    CompletionRejected {
        /// Exact ordered completion findings.
        issues: Vec<ValidationIssue>,
    },
}

/// Stateful, deterministic controller for one active work-packet run.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RunController {
    work_packet_id: WorkPacketId,
    task_id: TaskId,
    starting_revision: u32,
    limits: BTreeMap<BudgetResource, u64>,
    usage: BTreeMap<BudgetResource, u64>,
    stop_conditions: BTreeSet<StopConditionKind>,
    stop_reason: Option<RunStopReason>,
}

impl RunController {
    /// Creates a controller from one valid active packet.
    pub fn new(packet: &WorkPacket) -> Result<Self, RunControlError> {
        let issues = validate_packet(packet);
        if !issues.is_empty() {
            return Err(RunControlError::InvalidPacket { issues });
        }
        if packet.state != WorkPacketState::Active {
            return Err(RunControlError::PacketNotActive);
        }
        Ok(Self {
            work_packet_id: packet.work_packet_id.clone(),
            task_id: packet.task_id.clone(),
            starting_revision: packet.revision,
            limits: packet
                .budgets
                .iter()
                .map(|budget| (budget.resource, budget.limit))
                .collect(),
            usage: packet
                .budgets
                .iter()
                .map(|budget| (budget.resource, 0))
                .collect(),
            stop_conditions: packet
                .stop_conditions
                .iter()
                .map(|condition| condition.kind)
                .collect(),
            stop_reason: None,
        })
    }

    /// Admits usage only when the exact resource has sufficient declared capacity.
    ///
    /// Rejected and overflowing attempts do not change admitted usage. Once any stop reason
    /// is set, all later calls return that first reason without changing state.
    pub fn consume(&mut self, resource: BudgetResource, amount: u64) -> RunDecision {
        if let Some(reason) = self.stop_reason {
            return RunDecision::Stop { reason };
        }
        let Some(limit) = self.limits.get(&resource).copied() else {
            return self.stop(RunStopReason::BudgetNotDeclared { resource });
        };
        let used = self.usage.get(&resource).copied().unwrap_or(0);
        let Some(attempted_total) = used.checked_add(amount) else {
            return self.stop(RunStopReason::CounterOverflow { resource });
        };
        if attempted_total > limit {
            return self.stop(RunStopReason::BudgetExceeded {
                resource,
                limit,
                attempted_total,
            });
        }
        self.usage.insert(resource, attempted_total);
        RunDecision::Continue {
            resource,
            used: attempted_total,
            remaining: limit - attempted_total,
        }
    }

    /// Stops on one explicitly declared non-budget condition.
    ///
    /// Repeated signals are idempotent and return the first sticky reason.
    pub fn signal(&mut self, condition: StopConditionKind) -> Result<RunDecision, RunControlError> {
        if let Some(reason) = self.stop_reason {
            return Ok(RunDecision::Stop { reason });
        }
        if condition == StopConditionKind::BudgetExhausted {
            return Err(RunControlError::DirectBudgetSignalProhibited);
        }
        if condition == StopConditionKind::AcceptanceSatisfied {
            return Err(RunControlError::DirectCompletionSignalProhibited);
        }
        if !self.stop_conditions.contains(&condition) {
            return Err(RunControlError::ConditionNotDeclared { condition });
        }
        Ok(self.stop(RunStopReason::Condition(condition)))
    }

    /// Validates and accepts a newer completed packet revision as terminal evidence.
    pub fn accept_completion(
        &mut self,
        packet: &WorkPacket,
    ) -> Result<RunDecision, RunControlError> {
        if let Some(reason) = self.stop_reason {
            return Ok(RunDecision::Stop { reason });
        }
        if packet.work_packet_id != self.work_packet_id || packet.task_id != self.task_id {
            return Err(RunControlError::CompletionIdentityChanged);
        }
        if packet.revision <= self.starting_revision {
            return Err(RunControlError::CompletionRevisionNotNewer);
        }
        let mut issues = validate_packet(packet);
        if issues.is_empty() {
            issues.extend(validate_completion(packet));
        }
        if !issues.is_empty() {
            return Err(RunControlError::CompletionRejected { issues });
        }
        Ok(self.stop(RunStopReason::Condition(
            StopConditionKind::AcceptanceSatisfied,
        )))
    }

    /// Returns admitted usage for one declared resource.
    #[must_use]
    pub fn usage(&self, resource: BudgetResource) -> Option<u64> {
        self.usage.get(&resource).copied()
    }

    /// Returns the declared limit for one resource.
    #[must_use]
    pub fn limit(&self, resource: BudgetResource) -> Option<u64> {
        self.limits.get(&resource).copied()
    }

    /// Returns the first terminal reason, if one exists.
    #[must_use]
    pub const fn stop_reason(&self) -> Option<RunStopReason> {
        self.stop_reason
    }

    /// Reports whether the controller has entered a terminal state.
    #[must_use]
    pub const fn is_stopped(&self) -> bool {
        self.stop_reason.is_some()
    }

    fn stop(&mut self, reason: RunStopReason) -> RunDecision {
        self.stop_reason = Some(reason);
        RunDecision::Stop { reason }
    }
}

#[cfg(test)]
mod tests {
    use super::{RunControlError, RunController, RunDecision, RunStopReason};
    use agentmage_kernel_contracts::{
        BudgetLimit, BudgetResource, CONTRACT_SCHEMA_VERSION, CompletionEvidence, DataSensitivity,
        EvidenceId, EvidenceKind, EvidenceReference, PlanId, RollbackPlan, StopCondition,
        StopConditionKind, TaskId, WorkPacket, WorkPacketId, WorkPacketState,
    };

    fn stop_conditions() -> Vec<StopCondition> {
        [
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
        .collect()
    }

    fn evidence() -> EvidenceReference {
        EvidenceReference {
            schema_version: CONTRACT_SCHEMA_VERSION,
            evidence_id: EvidenceId::from_raw("evidence-0001"),
            kind: EvidenceKind::Observation,
            source_id: "synthetic-corpus-v1".to_owned(),
            object_id: "case-0001".to_owned(),
            fragment: None,
            content_sha256: "a".repeat(64),
            observed_revision: Some("fixture-v1".to_owned()),
        }
    }

    fn packet(state: WorkPacketState) -> WorkPacket {
        WorkPacket {
            schema_version: CONTRACT_SCHEMA_VERSION,
            work_packet_id: WorkPacketId::from_raw("packet-0001"),
            task_id: TaskId::from_raw("task-0001"),
            revision: 1,
            objective: "Inspect one synthetic fixture".to_owned(),
            reason: "Exercise bounded run control".to_owned(),
            owner: "fixture-user".to_owned(),
            authoritative_evidence: Vec::new(),
            mutable_files: Vec::new(),
            protected_files: vec!["fixtures/input.txt".to_owned()],
            expected_output: "One observed fixture".to_owned(),
            acceptance_checks: vec!["Observe fixture".to_owned()],
            required_evidence: vec![EvidenceKind::Observation],
            required_capability_class: "read-only".to_owned(),
            budgets: vec![
                BudgetLimit {
                    resource: BudgetResource::PlanSteps,
                    limit: 2,
                },
                BudgetLimit {
                    resource: BudgetResource::ToolCalls,
                    limit: 1,
                },
            ],
            stop_conditions: stop_conditions(),
            rollback: RollbackPlan {
                reversible: true,
                description: "No state change is permitted".to_owned(),
            },
            sensitivity: DataSensitivity::Ephemeral,
            last_verification_date: "2026-08-10".to_owned(),
            next_action: None,
            next_review: None,
            status_reason: None,
            disposition: None,
            completion_evidence: Vec::new(),
            superseding_work: None,
            validation_issues: Vec::new(),
            plan_id: Some(PlanId::from_raw("plan-0001")),
            state,
        }
    }

    fn completed_packet() -> WorkPacket {
        let mut packet = packet(WorkPacketState::Completed);
        packet.revision = 2;
        packet.disposition = Some("All acceptance evidence verified".to_owned());
        packet.completion_evidence = vec![CompletionEvidence {
            acceptance_check: "Observe fixture".to_owned(),
            evidence: vec![evidence()],
        }];
        packet
    }

    #[test]
    fn exact_limit_is_admitted_and_over_limit_is_sticky_without_usage_change() {
        let mut controller = RunController::new(&packet(WorkPacketState::Active))
            .expect("active fixture must start");
        assert_eq!(
            controller.consume(BudgetResource::PlanSteps, 1),
            RunDecision::Continue {
                resource: BudgetResource::PlanSteps,
                used: 1,
                remaining: 1,
            }
        );
        assert_eq!(
            controller.consume(BudgetResource::PlanSteps, 1),
            RunDecision::Continue {
                resource: BudgetResource::PlanSteps,
                used: 2,
                remaining: 0,
            }
        );
        let reason = RunStopReason::BudgetExceeded {
            resource: BudgetResource::PlanSteps,
            limit: 2,
            attempted_total: 3,
        };
        assert_eq!(
            controller.consume(BudgetResource::PlanSteps, 1),
            RunDecision::Stop { reason }
        );
        assert_eq!(controller.usage(BudgetResource::PlanSteps), Some(2));
        assert_eq!(
            controller.consume(BudgetResource::ToolCalls, 1),
            RunDecision::Stop { reason }
        );
        assert_eq!(controller.usage(BudgetResource::ToolCalls), Some(0));
    }

    #[test]
    fn undeclared_resource_and_overflow_fail_closed_before_mutation() {
        let mut controller = RunController::new(&packet(WorkPacketState::Active))
            .expect("active fixture must start");
        let reason = RunStopReason::BudgetNotDeclared {
            resource: BudgetResource::ModelCalls,
        };
        assert_eq!(
            controller.consume(BudgetResource::ModelCalls, 1),
            RunDecision::Stop { reason }
        );
        assert_eq!(controller.usage(BudgetResource::ModelCalls), None);

        let mut overflow_packet = packet(WorkPacketState::Active);
        overflow_packet.budgets[0].limit = u64::MAX;
        let mut overflow = RunController::new(&overflow_packet).expect("large limit is valid");
        assert!(matches!(
            overflow.consume(BudgetResource::PlanSteps, u64::MAX),
            RunDecision::Continue { .. }
        ));
        assert_eq!(
            overflow.consume(BudgetResource::PlanSteps, 1),
            RunDecision::Stop {
                reason: RunStopReason::CounterOverflow {
                    resource: BudgetResource::PlanSteps
                }
            }
        );
        assert_eq!(overflow.usage(BudgetResource::PlanSteps), Some(u64::MAX));
    }

    #[test]
    fn explicit_stop_is_declared_idempotent_and_first_reason_wins() {
        let mut controller = RunController::new(&packet(WorkPacketState::Active))
            .expect("active fixture must start");
        let first = controller
            .signal(StopConditionKind::UserDecisionRequired)
            .expect("declared stop");
        assert_eq!(
            first,
            RunDecision::Stop {
                reason: RunStopReason::Condition(StopConditionKind::UserDecisionRequired)
            }
        );
        assert_eq!(
            controller
                .signal(StopConditionKind::Error)
                .expect("later signal is idempotent"),
            first
        );

        let mut deadline = RunController::new(&packet(WorkPacketState::Active))
            .expect("active fixture must start");
        assert_eq!(
            deadline.signal(StopConditionKind::DeadlineReached),
            Err(RunControlError::ConditionNotDeclared {
                condition: StopConditionKind::DeadlineReached
            })
        );
        assert!(!deadline.is_stopped());
        assert_eq!(
            deadline.signal(StopConditionKind::BudgetExhausted),
            Err(RunControlError::DirectBudgetSignalProhibited)
        );
        assert_eq!(
            deadline.signal(StopConditionKind::AcceptanceSatisfied),
            Err(RunControlError::DirectCompletionSignalProhibited)
        );
    }

    #[test]
    fn controller_rejects_non_active_and_incomplete_safety_contracts() {
        assert_eq!(
            RunController::new(&packet(WorkPacketState::Planned)),
            Err(RunControlError::PacketNotActive)
        );
        let mut incomplete = packet(WorkPacketState::Active);
        incomplete
            .stop_conditions
            .retain(|condition| condition.kind != StopConditionKind::Cancelled);
        let Err(RunControlError::InvalidPacket { issues }) = RunController::new(&incomplete) else {
            panic!("missing mandatory stop must fail");
        };
        assert!(
            issues
                .iter()
                .any(|finding| finding.code == "packet.stop_condition.required")
        );
    }

    #[test]
    fn completion_requires_newer_matching_evidence_bound_packet() {
        let mut controller = RunController::new(&packet(WorkPacketState::Active))
            .expect("active fixture must start");
        let mut stale = completed_packet();
        stale.revision = 1;
        assert_eq!(
            controller.accept_completion(&stale),
            Err(RunControlError::CompletionRevisionNotNewer)
        );
        let mut wrong_identity = completed_packet();
        wrong_identity.work_packet_id = WorkPacketId::from_raw("packet-elsewhere");
        assert_eq!(
            controller.accept_completion(&wrong_identity),
            Err(RunControlError::CompletionIdentityChanged)
        );
        let mut missing = completed_packet();
        missing.completion_evidence.clear();
        assert!(matches!(
            controller.accept_completion(&missing),
            Err(RunControlError::CompletionRejected { .. })
        ));
        assert_eq!(
            controller
                .accept_completion(&completed_packet())
                .expect("complete evidence must stop"),
            RunDecision::Stop {
                reason: RunStopReason::Condition(StopConditionKind::AcceptanceSatisfied)
            }
        );
    }
}
