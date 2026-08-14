//! Bounded single-agent observe, plan, action-proposal, and review control loop.

use agentmage_kernel_contracts::{
    Action, ActionId, ActionKind, ActionState, BudgetResource, CONTRACT_SCHEMA_VERSION, Plan,
    PlanId, PlanStepId, StopConditionKind, TaskId, WorkPacket,
};

use crate::configuration::LoadedConfiguration;
use crate::run_control::{RunControlError, RunController, RunDecision, RunStopReason};
use crate::work_packet::adapt_packet_to_plan;

const MAX_ACTION_DESCRIPTION_BYTES: usize = 4_096;
const MAX_ACTION_EFFECTS: usize = 128;
const MAX_ACTION_ID_BYTES: usize = 128;

/// Current deterministic phase of one agent run.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AgentLoopPhase {
    /// The runtime is ready to account for one bounded observation.
    Observe,
    /// The runtime requires its one deterministic task plan.
    Plan,
    /// The runtime may admit one descriptive action proposal for later authority checks.
    Act,
    /// The runtime requires a result review for the exact pending action.
    Review,
    /// The first terminal stop reason is sticky and no further work is admitted.
    Stopped,
}

/// Non-authoritative next step returned by the deterministic loop.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AgentDirective {
    /// Build the task plan from the current work packet.
    Plan,
    /// Propose an action against the exact current plan.
    Act {
        /// Stable plan identity.
        plan_id: PlanId,
        /// Exact plan revision.
        plan_revision: u32,
    },
    /// Review the result of one exact descriptive action proposal.
    Review {
        /// Action whose result must be reviewed.
        action_id: ActionId,
    },
    /// Gather the next bounded observation before another action proposal.
    Observe {
        /// Number of completed review cycles.
        completed_cycles: u64,
    },
    /// Stop with the controller's first exact terminal reason.
    Stop {
        /// Sticky terminal reason.
        reason: RunStopReason,
    },
}

/// Typed review outcome for one exact pending action.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AgentReviewOutcome {
    /// The result was reviewed and the run may gather another observation.
    Progress,
    /// Deterministic policy denied further progress.
    PolicyDenied,
    /// A typed error prevents safe continuation.
    Error,
    /// The user or kernel cancelled the run.
    Cancelled,
    /// The action may have produced an effect whose terminal state is unknown.
    Uncertain,
    /// A fresh decision from the user is required.
    UserDecisionRequired,
}

/// Typed reason the agent runtime refuses a transition.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AgentRuntimeError {
    /// The source work packet cannot start a bounded run controller.
    RunControl(RunControlError),
    /// The requested operation is illegal in the current loop phase.
    PhaseMismatch {
        /// Phase required by the operation.
        expected: AgentLoopPhase,
        /// Current runtime phase.
        actual: AgentLoopPhase,
    },
    /// Deterministic plan construction rejected the current packet.
    PlanRejected,
    /// The action contract schema is unsupported.
    ActionSchemaUnsupported,
    /// The action belongs to another task.
    ActionTaskMismatch,
    /// The action is not a fresh descriptive proposal.
    ActionStateInvalid,
    /// The action identity is empty or exceeds its fixed bound.
    ActionIdentityInvalid,
    /// The action has no plan-step identity.
    ActionStepMissing,
    /// The action names a step outside the current plan.
    ActionStepUnknown,
    /// The action description is empty or exceeds its fixed bound.
    ActionDescriptionInvalid,
    /// The action effect list is empty, oversized, duplicated, or contains invalid text.
    ActionEffectsInvalid,
    /// Model inference was proposed while the exact loaded profile has no enabled model.
    ModelUnavailable,
    /// The review does not name the exact pending action.
    ReviewActionMismatch,
    /// A completed-cycle counter overflowed.
    CycleCounterOverflow,
}

/// One configuration-bound, single-agent deterministic run.
///
/// The runtime owns descriptive state and resource accounting only. It cannot issue a grant,
/// execute a tool, invoke a model, or treat an action proposal as authority.
pub struct AgentRuntime {
    configuration: LoadedConfiguration,
    packet: WorkPacket,
    controller: RunController,
    plan: Option<Plan>,
    pending_action: Option<ActionId>,
    phase: AgentLoopPhase,
    completed_cycles: u64,
}

impl AgentRuntime {
    /// Starts one runtime from an exact validated configuration and active work packet.
    pub fn new(
        configuration: LoadedConfiguration,
        packet: WorkPacket,
    ) -> Result<Self, AgentRuntimeError> {
        let controller = RunController::new(&packet).map_err(AgentRuntimeError::RunControl)?;
        Ok(Self {
            configuration,
            packet,
            controller,
            plan: None,
            pending_action: None,
            phase: AgentLoopPhase::Observe,
            completed_cycles: 0,
        })
    }

    /// Accounts for one bounded observation and selects planning or action proposal.
    pub fn observe(
        &mut self,
        input_bytes: u64,
        elapsed_milliseconds: u64,
    ) -> Result<AgentDirective, AgentRuntimeError> {
        self.require_phase(AgentLoopPhase::Observe)?;
        if let Some(stop) = self.consume(BudgetResource::InputBytes, input_bytes) {
            return Ok(stop);
        }
        if let Some(stop) = self.consume(BudgetResource::ElapsedMilliseconds, elapsed_milliseconds)
        {
            return Ok(stop);
        }
        if let Some(plan) = &self.plan {
            self.phase = AgentLoopPhase::Act;
            Ok(AgentDirective::Act {
                plan_id: plan.plan_id.clone(),
                plan_revision: plan.revision,
            })
        } else {
            self.phase = AgentLoopPhase::Plan;
            Ok(AgentDirective::Plan)
        }
    }

    /// Builds the one deterministic task plan from current acceptance checks.
    pub fn plan(&mut self) -> Result<AgentDirective, AgentRuntimeError> {
        self.require_phase(AgentLoopPhase::Plan)?;
        let plan_id = self
            .packet
            .plan_id
            .clone()
            .ok_or(AgentRuntimeError::PlanRejected)?;
        let plan = adapt_packet_to_plan(&self.packet, plan_id.clone(), None)
            .map_err(|_| AgentRuntimeError::PlanRejected)?;
        let plan_revision = plan.revision;
        self.plan = Some(plan);
        self.phase = AgentLoopPhase::Act;
        Ok(AgentDirective::Act {
            plan_id,
            plan_revision,
        })
    }

    /// Accounts for one descriptive action proposal and requires an exact later review.
    ///
    /// A successful return does not execute or authorize the action. Downstream kernel policy,
    /// grant consumption, platform mediation, and postcondition verification remain mandatory.
    pub fn act(&mut self, action: &Action) -> Result<AgentDirective, AgentRuntimeError> {
        self.require_phase(AgentLoopPhase::Act)?;
        self.validate_action(action)?;
        if action.kind == ActionKind::ModelInference && !self.configuration.model_enabled() {
            return Err(AgentRuntimeError::ModelUnavailable);
        }
        if let Some(stop) = self.consume(BudgetResource::PlanSteps, 1) {
            return Ok(stop);
        }
        let metered_resource = match action.kind {
            ActionKind::DeterministicTool => Some(BudgetResource::ToolCalls),
            ActionKind::ModelInference => Some(BudgetResource::ModelCalls),
            ActionKind::UserDecision => None,
            ActionKind::KernelDecision => None,
        };
        if let Some(resource) = metered_resource
            && let Some(stop) = self.consume(resource, 1)
        {
            return Ok(stop);
        }
        if action.kind == ActionKind::UserDecision {
            return self.signal(StopConditionKind::UserDecisionRequired);
        }
        self.pending_action = Some(action.action_id.clone());
        self.phase = AgentLoopPhase::Review;
        Ok(AgentDirective::Review {
            action_id: action.action_id.clone(),
        })
    }

    /// Reviews the exact pending action and either continues or enters a typed stop state.
    pub fn review(
        &mut self,
        action_id: &ActionId,
        outcome: AgentReviewOutcome,
        output_bytes: u64,
        elapsed_milliseconds: u64,
    ) -> Result<AgentDirective, AgentRuntimeError> {
        self.require_phase(AgentLoopPhase::Review)?;
        if self.pending_action.as_ref() != Some(action_id) {
            return Err(AgentRuntimeError::ReviewActionMismatch);
        }
        if let Some(stop) = self.consume(BudgetResource::OutputBytes, output_bytes) {
            return Ok(stop);
        }
        if let Some(stop) = self.consume(BudgetResource::ElapsedMilliseconds, elapsed_milliseconds)
        {
            return Ok(stop);
        }
        self.pending_action = None;
        match outcome {
            AgentReviewOutcome::Progress => {
                self.completed_cycles = self
                    .completed_cycles
                    .checked_add(1)
                    .ok_or(AgentRuntimeError::CycleCounterOverflow)?;
                self.phase = AgentLoopPhase::Observe;
                Ok(AgentDirective::Observe {
                    completed_cycles: self.completed_cycles,
                })
            }
            AgentReviewOutcome::PolicyDenied => self.signal(StopConditionKind::PolicyDenied),
            AgentReviewOutcome::Error => self.signal(StopConditionKind::Error),
            AgentReviewOutcome::Cancelled => self.signal(StopConditionKind::Cancelled),
            AgentReviewOutcome::Uncertain => self.signal(StopConditionKind::UncertainResult),
            AgentReviewOutcome::UserDecisionRequired => {
                self.signal(StopConditionKind::UserDecisionRequired)
            }
        }
    }

    /// Accepts a newer evidence-complete packet as the only direct successful stop path.
    pub fn accept_completion(
        &mut self,
        packet: &WorkPacket,
    ) -> Result<AgentDirective, AgentRuntimeError> {
        let decision = self
            .controller
            .accept_completion(packet)
            .map_err(AgentRuntimeError::RunControl)?;
        Ok(self.apply_decision(decision))
    }

    /// Signals one declared non-completion stop condition.
    pub fn signal(
        &mut self,
        condition: StopConditionKind,
    ) -> Result<AgentDirective, AgentRuntimeError> {
        let decision = self
            .controller
            .signal(condition)
            .map_err(AgentRuntimeError::RunControl)?;
        Ok(self.apply_decision(decision))
    }

    /// Returns the current loop phase.
    #[must_use]
    pub const fn phase(&self) -> AgentLoopPhase {
        self.phase
    }

    /// Returns the exact loaded configuration identity.
    #[must_use]
    pub fn configuration_sha256(&self) -> &str {
        self.configuration.sha256()
    }

    /// Returns the exact loaded profile identity.
    #[must_use]
    pub fn profile_id(&self) -> &str {
        self.configuration.profile_id()
    }

    /// Returns the stable task identity.
    #[must_use]
    pub fn task_id(&self) -> &TaskId {
        &self.packet.task_id
    }

    /// Returns the current deterministic plan, if planning has occurred.
    #[must_use]
    pub const fn current_plan(&self) -> Option<&Plan> {
        self.plan.as_ref()
    }

    /// Returns admitted usage for one declared resource.
    #[must_use]
    pub fn usage(&self, resource: BudgetResource) -> Option<u64> {
        self.controller.usage(resource)
    }

    /// Returns the first terminal stop reason, if one exists.
    #[must_use]
    pub const fn stop_reason(&self) -> Option<RunStopReason> {
        self.controller.stop_reason()
    }

    fn require_phase(&self, expected: AgentLoopPhase) -> Result<(), AgentRuntimeError> {
        if self.phase == expected {
            Ok(())
        } else {
            Err(AgentRuntimeError::PhaseMismatch {
                expected,
                actual: self.phase,
            })
        }
    }

    fn validate_action(&self, action: &Action) -> Result<(), AgentRuntimeError> {
        if action.schema_version != CONTRACT_SCHEMA_VERSION {
            return Err(AgentRuntimeError::ActionSchemaUnsupported);
        }
        if action.task_id != self.packet.task_id {
            return Err(AgentRuntimeError::ActionTaskMismatch);
        }
        if action.state != ActionState::Proposed {
            return Err(AgentRuntimeError::ActionStateInvalid);
        }
        if action.action_id.as_str().is_empty()
            || action.action_id.as_str().len() > MAX_ACTION_ID_BYTES
        {
            return Err(AgentRuntimeError::ActionIdentityInvalid);
        }
        let step_id = action
            .plan_step_id
            .as_ref()
            .ok_or(AgentRuntimeError::ActionStepMissing)?;
        if !self.plan_contains_step(step_id) {
            return Err(AgentRuntimeError::ActionStepUnknown);
        }
        if action.description.is_empty() || action.description.len() > MAX_ACTION_DESCRIPTION_BYTES
        {
            return Err(AgentRuntimeError::ActionDescriptionInvalid);
        }
        if action.expected_effects.is_empty()
            || action.expected_effects.len() > MAX_ACTION_EFFECTS
            || action
                .expected_effects
                .iter()
                .enumerate()
                .any(|(index, effect)| {
                    effect.is_empty()
                        || effect.len() > MAX_ACTION_DESCRIPTION_BYTES
                        || action.expected_effects[..index].contains(effect)
                })
        {
            return Err(AgentRuntimeError::ActionEffectsInvalid);
        }
        Ok(())
    }

    fn plan_contains_step(&self, step_id: &PlanStepId) -> bool {
        self.plan
            .as_ref()
            .is_some_and(|plan| plan.steps.iter().any(|step| &step.plan_step_id == step_id))
    }

    fn consume(&mut self, resource: BudgetResource, amount: u64) -> Option<AgentDirective> {
        match self.controller.consume(resource, amount) {
            RunDecision::Continue { .. } => None,
            stop @ RunDecision::Stop { .. } => Some(self.apply_decision(stop)),
        }
    }

    fn apply_decision(&mut self, decision: RunDecision) -> AgentDirective {
        match decision {
            RunDecision::Continue { .. } => {
                unreachable!("runtime applies only terminal run decisions")
            }
            RunDecision::Stop { reason } => {
                self.pending_action = None;
                self.phase = AgentLoopPhase::Stopped;
                AgentDirective::Stop { reason }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        AgentDirective, AgentLoopPhase, AgentReviewOutcome, AgentRuntime, AgentRuntimeError,
    };
    use crate::agent_progress::{AgentProgressError, PlanProgressController};
    use crate::configuration::ConfigurationManager;
    use crate::run_control::{RunControlError, RunStopReason};
    use agentmage_kernel_contracts::{
        Action, ActionId, ActionKind, ActionState, AuthorityClass, BudgetLimit, BudgetResource,
        CONTRACT_SCHEMA_VERSION, CompletionEvidence, DataSensitivity, EvidenceId, EvidenceKind,
        EvidenceReference, PlanId, PlanStepId, PlanStepState, RollbackPlan, StopCondition,
        StopConditionKind, TaskId, WorkPacket, WorkPacketId, WorkPacketState,
    };

    const PROFILE: &[u8] = include_bytes!("../../../configuration/profiles/synthetic-test.json");

    fn packet() -> WorkPacket {
        WorkPacket {
            schema_version: CONTRACT_SCHEMA_VERSION,
            work_packet_id: WorkPacketId::from_raw("packet-runtime-0001"),
            task_id: TaskId::from_raw("task-runtime-0001"),
            revision: 1,
            objective: "Inspect one synthetic fixture".to_owned(),
            reason: "Exercise the bounded agent runtime".to_owned(),
            owner: "synthetic-user".to_owned(),
            authoritative_evidence: Vec::new(),
            mutable_files: Vec::new(),
            protected_files: vec!["fixtures/input.txt".to_owned()],
            expected_output: "One reviewed synthetic observation".to_owned(),
            acceptance_checks: vec!["Record bounded evidence".to_owned()],
            required_evidence: vec![EvidenceKind::Observation],
            required_capability_class: AuthorityClass::Observe,
            budgets: vec![
                BudgetLimit {
                    resource: BudgetResource::PlanSteps,
                    limit: 2,
                },
                BudgetLimit {
                    resource: BudgetResource::ToolCalls,
                    limit: 2,
                },
                BudgetLimit {
                    resource: BudgetResource::ModelCalls,
                    limit: 1,
                },
                BudgetLimit {
                    resource: BudgetResource::InputBytes,
                    limit: 64,
                },
                BudgetLimit {
                    resource: BudgetResource::OutputBytes,
                    limit: 64,
                },
                BudgetLimit {
                    resource: BudgetResource::ElapsedMilliseconds,
                    limit: 100,
                },
            ],
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
                description: "No state change is authorized".to_owned(),
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
            plan_id: Some(PlanId::from_raw("plan-runtime-0001")),
            state: WorkPacketState::Active,
        }
    }

    fn runtime_with(packet: WorkPacket) -> AgentRuntime {
        let configuration = ConfigurationManager::default()
            .load_bytes(PROFILE)
            .expect("valid configuration fixture");
        AgentRuntime::new(configuration, packet).expect("active packet starts")
    }

    fn planned_runtime() -> AgentRuntime {
        let mut runtime = runtime_with(packet());
        assert_eq!(runtime.observe(8, 5), Ok(AgentDirective::Plan));
        assert!(matches!(runtime.plan(), Ok(AgentDirective::Act { .. })));
        runtime
    }

    fn action(runtime: &AgentRuntime, kind: ActionKind) -> Action {
        Action {
            schema_version: CONTRACT_SCHEMA_VERSION,
            action_id: ActionId::from_raw("action-runtime-0001"),
            task_id: runtime.task_id().clone(),
            plan_step_id: Some(
                runtime.current_plan().expect("plan exists").steps[0]
                    .plan_step_id
                    .clone(),
            ),
            kind,
            description: "Inspect one synthetic fixture".to_owned(),
            expected_effects: vec!["descriptive-proposal-only".to_owned()],
            state: ActionState::Proposed,
        }
    }

    fn completion(mut packet: WorkPacket) -> WorkPacket {
        packet.state = WorkPacketState::Completed;
        packet.revision += 1;
        packet.disposition = Some("Current evidence verifies completion".to_owned());
        packet.completion_evidence = packet
            .acceptance_checks
            .iter()
            .map(|check| CompletionEvidence {
                acceptance_check: check.clone(),
                evidence: vec![EvidenceReference {
                    schema_version: CONTRACT_SCHEMA_VERSION,
                    evidence_id: EvidenceId::from_raw("evidence-runtime-0001"),
                    kind: EvidenceKind::Observation,
                    source_id: "synthetic-runtime".to_owned(),
                    object_id: "postcondition".to_owned(),
                    fragment: None,
                    content_sha256: "a".repeat(64),
                    observed_revision: Some("fixture-v1".to_owned()),
                }],
            })
            .collect();
        packet
    }

    #[test]
    fn observe_plan_act_review_is_bounded_configuration_bound_and_non_authoritative() {
        let mut runtime = planned_runtime();
        let configuration_sha256 = runtime.configuration_sha256().to_owned();
        assert_eq!(runtime.profile_id(), "synthetic-test");
        assert_eq!(configuration_sha256.len(), 64);

        let proposal = action(&runtime, ActionKind::DeterministicTool);
        assert_eq!(
            runtime.act(&proposal),
            Ok(AgentDirective::Review {
                action_id: proposal.action_id.clone()
            })
        );
        assert_eq!(runtime.usage(BudgetResource::PlanSteps), Some(1));
        assert_eq!(runtime.usage(BudgetResource::ToolCalls), Some(1));
        assert_eq!(
            runtime.review(&proposal.action_id, AgentReviewOutcome::Progress, 12, 7),
            Ok(AgentDirective::Observe {
                completed_cycles: 1
            })
        );
        assert_eq!(runtime.phase(), AgentLoopPhase::Observe);
        assert!(matches!(
            runtime.observe(4, 3),
            Ok(AgentDirective::Act {
                plan_revision: 1,
                ..
            })
        ));
    }

    #[test]
    fn every_phase_rejects_out_of_order_calls_without_budget_or_state_change() {
        let mut runtime = runtime_with(packet());
        assert!(matches!(
            runtime.plan(),
            Err(AgentRuntimeError::PhaseMismatch {
                expected: AgentLoopPhase::Plan,
                actual: AgentLoopPhase::Observe
            })
        ));
        assert_eq!(runtime.usage(BudgetResource::InputBytes), Some(0));
        assert_eq!(runtime.observe(1, 1), Ok(AgentDirective::Plan));
        let unplanned = Action {
            schema_version: CONTRACT_SCHEMA_VERSION,
            action_id: ActionId::from_raw("action-out-of-order"),
            task_id: runtime.task_id().clone(),
            plan_step_id: None,
            kind: ActionKind::KernelDecision,
            description: "No authority".to_owned(),
            expected_effects: Vec::new(),
            state: ActionState::Proposed,
        };
        assert!(matches!(
            runtime.act(&unplanned),
            Err(AgentRuntimeError::PhaseMismatch {
                expected: AgentLoopPhase::Act,
                actual: AgentLoopPhase::Plan
            })
        ));
        assert_eq!(runtime.usage(BudgetResource::PlanSteps), Some(0));
    }

    #[test]
    fn action_schema_task_state_step_and_model_availability_fail_before_accounting() {
        let mut mutations = Vec::new();
        let runtime = planned_runtime();
        let base = action(&runtime, ActionKind::DeterministicTool);
        let mut unsupported = base.clone();
        unsupported.schema_version += 1;
        mutations.push((unsupported, AgentRuntimeError::ActionSchemaUnsupported));
        let mut wrong_task = base.clone();
        wrong_task.task_id = agentmage_kernel_contracts::TaskId::from_raw("task-elsewhere");
        mutations.push((wrong_task, AgentRuntimeError::ActionTaskMismatch));
        let mut wrong_state = base.clone();
        wrong_state.state = ActionState::Ready;
        mutations.push((wrong_state, AgentRuntimeError::ActionStateInvalid));
        let mut missing_identity = base.clone();
        missing_identity.action_id = ActionId::from_raw("");
        mutations.push((missing_identity, AgentRuntimeError::ActionIdentityInvalid));
        let mut missing_step = base.clone();
        missing_step.plan_step_id = None;
        mutations.push((missing_step, AgentRuntimeError::ActionStepMissing));
        let mut unknown_step = base;
        unknown_step.plan_step_id = Some(agentmage_kernel_contracts::PlanStepId::from_raw(
            "plan-elsewhere:step:0",
        ));
        mutations.push((unknown_step, AgentRuntimeError::ActionStepUnknown));
        let mut missing_effect = action(&runtime, ActionKind::DeterministicTool);
        missing_effect.expected_effects.clear();
        mutations.push((missing_effect, AgentRuntimeError::ActionEffectsInvalid));
        let mut duplicate_effect = action(&runtime, ActionKind::DeterministicTool);
        duplicate_effect
            .expected_effects
            .push("descriptive-proposal-only".to_owned());
        mutations.push((duplicate_effect, AgentRuntimeError::ActionEffectsInvalid));
        let mut empty_description = action(&runtime, ActionKind::DeterministicTool);
        empty_description.description.clear();
        mutations.push((
            empty_description,
            AgentRuntimeError::ActionDescriptionInvalid,
        ));

        for (candidate, expected) in mutations {
            let mut runtime = planned_runtime();
            assert_eq!(runtime.act(&candidate), Err(expected));
            assert_eq!(runtime.phase(), AgentLoopPhase::Act);
            assert_eq!(runtime.usage(BudgetResource::PlanSteps), Some(0));
            assert_eq!(runtime.usage(BudgetResource::ToolCalls), Some(0));
        }

        let mut runtime = planned_runtime();
        let model = action(&runtime, ActionKind::ModelInference);
        assert_eq!(
            runtime.act(&model),
            Err(AgentRuntimeError::ModelUnavailable)
        );
        assert_eq!(runtime.usage(BudgetResource::PlanSteps), Some(0));
        assert_eq!(runtime.usage(BudgetResource::ModelCalls), Some(0));
    }

    #[test]
    fn budget_exhaustion_is_sticky_and_stops_before_later_phases() {
        let mut bounded = packet();
        bounded
            .budgets
            .iter_mut()
            .find(|budget| budget.resource == BudgetResource::InputBytes)
            .expect("input budget")
            .limit = 4;
        let mut runtime = runtime_with(bounded);
        let stopped = runtime.observe(5, 1).expect("budget stop is typed");
        assert!(matches!(stopped, AgentDirective::Stop { .. }));
        assert_eq!(runtime.phase(), AgentLoopPhase::Stopped);
        assert_eq!(runtime.usage(BudgetResource::InputBytes), Some(0));
        assert!(matches!(
            runtime.observe(1, 1),
            Err(AgentRuntimeError::PhaseMismatch {
                actual: AgentLoopPhase::Stopped,
                ..
            })
        ));
        assert!(runtime.stop_reason().is_some());
    }

    #[test]
    fn review_identity_and_typed_outcomes_cannot_become_progress() {
        let outcomes = [
            (
                AgentReviewOutcome::PolicyDenied,
                StopConditionKind::PolicyDenied,
            ),
            (AgentReviewOutcome::Error, StopConditionKind::Error),
            (AgentReviewOutcome::Cancelled, StopConditionKind::Cancelled),
            (
                AgentReviewOutcome::Uncertain,
                StopConditionKind::UncertainResult,
            ),
            (
                AgentReviewOutcome::UserDecisionRequired,
                StopConditionKind::UserDecisionRequired,
            ),
        ];
        for (outcome, condition) in outcomes {
            let mut runtime = planned_runtime();
            let proposal = action(&runtime, ActionKind::DeterministicTool);
            runtime.act(&proposal).expect("proposal reaches review");
            assert_eq!(
                runtime.review(&ActionId::from_raw("action-elsewhere"), outcome, 0, 0),
                Err(AgentRuntimeError::ReviewActionMismatch)
            );
            assert_eq!(runtime.phase(), AgentLoopPhase::Review);
            let stopped = runtime
                .review(&proposal.action_id, outcome, 0, 1)
                .expect("typed review stop");
            assert!(matches!(
                stopped,
                AgentDirective::Stop {
                    reason: crate::run_control::RunStopReason::Condition(actual)
                } if actual == condition
            ));
        }
    }

    #[test]
    fn user_decision_action_stops_without_pending_review_or_tool_execution() {
        let mut runtime = planned_runtime();
        let decision = action(&runtime, ActionKind::UserDecision);
        let stopped = runtime.act(&decision).expect("user decision is typed");
        assert!(matches!(
            stopped,
            AgentDirective::Stop {
                reason: crate::run_control::RunStopReason::Condition(
                    StopConditionKind::UserDecisionRequired
                )
            }
        ));
        assert_eq!(runtime.usage(BudgetResource::PlanSteps), Some(1));
        assert_eq!(runtime.usage(BudgetResource::ToolCalls), Some(0));
    }

    #[test]
    fn only_newer_evidence_complete_packet_can_stop_as_acceptance_satisfied() {
        let source = packet();
        let mut runtime = runtime_with(source.clone());
        let mut invalid = completion(source.clone());
        invalid.completion_evidence.clear();
        assert!(matches!(
            runtime.accept_completion(&invalid),
            Err(AgentRuntimeError::RunControl(
                crate::run_control::RunControlError::CompletionRejected { .. }
            ))
        ));
        let stopped = runtime
            .accept_completion(&completion(source))
            .expect("verified completion stops");
        assert!(matches!(
            stopped,
            AgentDirective::Stop {
                reason: crate::run_control::RunStopReason::Condition(
                    StopConditionKind::AcceptanceSatisfied
                )
            }
        ));
    }

    #[test]
    fn missing_mandatory_stop_or_budget_contract_fails_at_construction() {
        let mut missing_stop = packet();
        missing_stop
            .stop_conditions
            .retain(|condition: &StopCondition| condition.kind != StopConditionKind::Cancelled);
        let configuration = ConfigurationManager::default()
            .load_bytes(PROFILE)
            .expect("configuration");
        assert!(matches!(
            AgentRuntime::new(configuration, missing_stop),
            Err(AgentRuntimeError::RunControl(
                crate::run_control::RunControlError::InvalidPacket { .. }
            ))
        ));

        let mut missing_budget = packet();
        missing_budget.budgets.clear();
        let configuration = ConfigurationManager::default()
            .load_bytes(PROFILE)
            .expect("configuration");
        assert!(matches!(
            AgentRuntime::new(configuration, missing_budget),
            Err(AgentRuntimeError::RunControl(
                crate::run_control::RunControlError::InvalidPacket { .. }
            ))
        ));
    }

    #[derive(Clone, Copy)]
    enum S012Operation {
        Observe,
        Plan,
        Act,
        Review,
    }

    impl S012Operation {
        const ALL: [Self; 4] = [Self::Observe, Self::Plan, Self::Act, Self::Review];

        const fn required_phase(self) -> AgentLoopPhase {
            match self {
                Self::Observe => AgentLoopPhase::Observe,
                Self::Plan => AgentLoopPhase::Plan,
                Self::Act => AgentLoopPhase::Act,
                Self::Review => AgentLoopPhase::Review,
            }
        }
    }

    fn s012_runtime_at(phase: AgentLoopPhase) -> AgentRuntime {
        let mut runtime = runtime_with(packet());
        if phase == AgentLoopPhase::Observe {
            return runtime;
        }
        runtime.observe(0, 0).expect("observe reaches plan");
        if phase == AgentLoopPhase::Plan {
            return runtime;
        }
        runtime.plan().expect("plan reaches act");
        if phase == AgentLoopPhase::Act {
            return runtime;
        }
        let proposal = action(&runtime, ActionKind::DeterministicTool);
        runtime.act(&proposal).expect("act reaches review");
        if phase == AgentLoopPhase::Review {
            return runtime;
        }
        runtime
            .signal(StopConditionKind::Cancelled)
            .expect("review may stop");
        runtime
    }

    fn s012_exercise(
        runtime: &mut AgentRuntime,
        operation: S012Operation,
    ) -> Result<AgentDirective, AgentRuntimeError> {
        match operation {
            S012Operation::Observe => runtime.observe(0, 0),
            S012Operation::Plan => runtime.plan(),
            S012Operation::Act => {
                let proposal = if runtime.current_plan().is_some() {
                    action(runtime, ActionKind::DeterministicTool)
                } else {
                    Action {
                        schema_version: CONTRACT_SCHEMA_VERSION,
                        action_id: ActionId::from_raw("action-runtime-0001"),
                        task_id: runtime.task_id().clone(),
                        plan_step_id: Some(PlanStepId::from_raw("unavailable-step")),
                        kind: ActionKind::DeterministicTool,
                        description: "Synthetic phase probe".to_owned(),
                        expected_effects: vec!["descriptive-only".to_owned()],
                        state: ActionState::Proposed,
                    }
                };
                runtime.act(&proposal)
            }
            S012Operation::Review => runtime.review(
                &ActionId::from_raw("action-runtime-0001"),
                AgentReviewOutcome::Progress,
                0,
                0,
            ),
        }
    }

    #[test]
    fn s_012_ut01_covers_every_legal_and_illegal_loop_transition() {
        let phases = [
            AgentLoopPhase::Observe,
            AgentLoopPhase::Plan,
            AgentLoopPhase::Act,
            AgentLoopPhase::Review,
            AgentLoopPhase::Stopped,
        ];
        for phase in phases {
            for operation in S012Operation::ALL {
                let mut runtime = s012_runtime_at(phase);
                let result = s012_exercise(&mut runtime, operation);
                if phase == operation.required_phase() {
                    let expected = match operation {
                        S012Operation::Observe => AgentLoopPhase::Plan,
                        S012Operation::Plan => AgentLoopPhase::Act,
                        S012Operation::Act => AgentLoopPhase::Review,
                        S012Operation::Review => AgentLoopPhase::Observe,
                    };
                    assert!(result.is_ok(), "legal phase transition must succeed");
                    assert_eq!(runtime.phase(), expected);
                } else {
                    assert!(matches!(
                        result,
                        Err(AgentRuntimeError::PhaseMismatch {
                            expected,
                            actual
                        }) if expected == operation.required_phase() && actual == phase
                    ));
                    assert_eq!(runtime.phase(), phase);
                }
            }
        }

        for phase in phases[..4].iter().copied() {
            let mut runtime = s012_runtime_at(phase);
            assert!(matches!(
                runtime.signal(StopConditionKind::Cancelled),
                Ok(AgentDirective::Stop {
                    reason: RunStopReason::Condition(StopConditionKind::Cancelled)
                })
            ));
            assert_eq!(runtime.phase(), AgentLoopPhase::Stopped);
        }
        let mut stopped = s012_runtime_at(AgentLoopPhase::Stopped);
        assert!(matches!(
            stopped.signal(StopConditionKind::PolicyDenied),
            Ok(AgentDirective::Stop {
                reason: RunStopReason::Condition(StopConditionKind::Cancelled)
            })
        ));
        assert_eq!(stopped.phase(), AgentLoopPhase::Stopped);
    }

    #[test]
    fn s_012_ut01_rejects_empty_and_oversized_objectives_deterministically() {
        for objective in [String::new(), "x".repeat(4_097)] {
            let mut candidate = packet();
            candidate.objective = objective;
            let make_error = || {
                let configuration = ConfigurationManager::default()
                    .load_bytes(PROFILE)
                    .expect("configuration fixture");
                AgentRuntime::new(configuration, candidate.clone())
                    .err()
                    .expect("invalid objective fails")
            };
            let first = make_error();
            let second = make_error();
            assert_eq!(first, second);
            assert!(matches!(
                first,
                AgentRuntimeError::RunControl(RunControlError::InvalidPacket { ref issues })
                    if issues.iter().any(|issue|
                        issue.code == "packet.text.invalid"
                            && issue.field_path == ["objective".to_owned()]
                    )
            ));
        }
    }

    #[test]
    fn s_012_ut01_plan_revisions_advance_once_and_fail_without_mutation() {
        let runtime = planned_runtime();
        let plan = runtime.current_plan().expect("plan exists").clone();
        assert_eq!(plan.revision, 1);
        let step_id = plan.steps[0].plan_step_id.clone();
        let mut progress = PlanProgressController::new(plan).expect("plan is valid");
        assert_eq!(
            progress
                .transition_step(&step_id, PlanStepState::Ready)
                .expect("proposed step becomes ready")
                .revision,
            2
        );
        assert_eq!(
            progress
                .transition_step(&step_id, PlanStepState::Running)
                .expect("ready step begins")
                .revision,
            3
        );
        assert_eq!(
            progress.transition_step(&step_id, PlanStepState::Ready),
            Err(AgentProgressError::StepTransitionInvalid)
        );
        assert_eq!(progress.current().revision, 3);
        assert_eq!(progress.revisions().len(), 3);
    }

    #[test]
    fn s_012_ut01_budget_boundary_is_inclusive_and_overage_is_sticky() {
        let mut bounded = packet();
        bounded
            .budgets
            .iter_mut()
            .find(|budget| budget.resource == BudgetResource::InputBytes)
            .expect("input budget")
            .limit = 4;
        let mut exact = runtime_with(bounded.clone());
        assert_eq!(exact.observe(4, 0), Ok(AgentDirective::Plan));
        assert_eq!(exact.usage(BudgetResource::InputBytes), Some(4));

        let mut exceeded = runtime_with(bounded);
        assert!(matches!(
            exceeded.observe(5, 0),
            Ok(AgentDirective::Stop {
                reason: RunStopReason::BudgetExceeded {
                    resource: BudgetResource::InputBytes,
                    limit: 4,
                    attempted_total: 5
                }
            })
        ));
        assert_eq!(exceeded.usage(BudgetResource::InputBytes), Some(0));
        assert_eq!(exceeded.phase(), AgentLoopPhase::Stopped);
        assert!(matches!(
            exceeded.signal(StopConditionKind::Error),
            Ok(AgentDirective::Stop {
                reason: RunStopReason::BudgetExceeded {
                    resource: BudgetResource::InputBytes,
                    limit: 4,
                    attempted_total: 5
                }
            })
        ));
    }

    #[test]
    fn s_012_ut01_stop_conditions_and_completion_claims_are_typed() {
        for condition in [
            StopConditionKind::UserDecisionRequired,
            StopConditionKind::PolicyDenied,
            StopConditionKind::Error,
            StopConditionKind::Cancelled,
            StopConditionKind::UncertainResult,
        ] {
            let mut runtime = runtime_with(packet());
            assert_eq!(
                runtime.signal(condition),
                Ok(AgentDirective::Stop {
                    reason: RunStopReason::Condition(condition)
                })
            );
        }
        let prohibited = [
            (
                StopConditionKind::AcceptanceSatisfied,
                RunControlError::DirectCompletionSignalProhibited,
            ),
            (
                StopConditionKind::BudgetExhausted,
                RunControlError::DirectBudgetSignalProhibited,
            ),
            (
                StopConditionKind::DeadlineReached,
                RunControlError::ConditionNotDeclared {
                    condition: StopConditionKind::DeadlineReached,
                },
            ),
        ];
        for (condition, expected) in prohibited {
            let mut runtime = runtime_with(packet());
            assert_eq!(
                runtime.signal(condition),
                Err(AgentRuntimeError::RunControl(expected))
            );
            assert_eq!(runtime.phase(), AgentLoopPhase::Observe);
        }

        let source = packet();
        let mut runtime = runtime_with(source.clone());
        let mut stale = completion(source.clone());
        stale.revision = source.revision;
        assert_eq!(
            runtime.accept_completion(&stale),
            Err(AgentRuntimeError::RunControl(
                RunControlError::CompletionRevisionNotNewer
            ))
        );
        let mut unsupported = completion(source.clone());
        unsupported.completion_evidence.clear();
        assert!(matches!(
            runtime.accept_completion(&unsupported),
            Err(AgentRuntimeError::RunControl(
                RunControlError::CompletionRejected { .. }
            ))
        ));
        assert_eq!(
            runtime.accept_completion(&completion(source)),
            Ok(AgentDirective::Stop {
                reason: RunStopReason::Condition(StopConditionKind::AcceptanceSatisfied)
            })
        );
        assert_eq!(runtime.phase(), AgentLoopPhase::Stopped);
    }
}
