//! One-active-step plan history, progress, interruption, and response validation.

use agentmage_kernel_contracts::{
    AgentFinalResponse, AgentFinalState, AgentProgressEvent, AgentProgressKind, AgentStatusKind,
    AgentStatusResponse, CONTRACT_SCHEMA_VERSION, CancellationSignal, EvidenceReference, Plan,
    PlanState, PlanStepId, PlanStepState, TaskId, UserMessageDisposition, UserMessageIntent,
};

use crate::propagation::{CancellationError, CancellationToken};

const MAX_PLAN_REVISIONS: usize = 1_024;
const MAX_PLAN_STEPS: usize = 128;
const MAX_RESPONSE_ITEMS: usize = 128;
const MAX_RESPONSE_EVIDENCE: usize = 256;
const MAX_TEXT_BYTES: usize = 4_096;

/// Typed reason a plan, progress, interruption, or response transition is refused.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AgentProgressError {
    /// The initial or appended plan is structurally invalid.
    InvalidPlan,
    /// Plan or task identity changed.
    IdentityChanged,
    /// A plan revision was repeated, skipped, or exhausted.
    RevisionInvalid,
    /// The retained history reached its fixed bound.
    HistoryLimitReached,
    /// The requested step identity is absent.
    StepUnknown,
    /// The requested step transition is illegal.
    StepTransitionInvalid,
    /// A dependency has not completed.
    DependencyIncomplete,
    /// Another step is already running.
    ActiveStepExists,
    /// The cancellation token rejected the supplied signal.
    Cancellation(CancellationError),
    /// An interrupting message omitted cancellation material.
    CancellationRequired,
    /// A status-only message incorrectly supplied cancellation material.
    UnexpectedCancellation,
    /// The current plan is already terminal and cannot be interrupted.
    PlanNotInterruptible,
    /// The final-response contract is invalid.
    InvalidFinalResponse,
}

/// Append-only plan revisions with one-running-step enforcement.
pub struct PlanProgressController {
    revisions: Vec<Plan>,
    events: Vec<AgentProgressEvent>,
}

impl PlanProgressController {
    /// Starts progress tracking from one valid plan.
    pub fn new(plan: Plan) -> Result<Self, AgentProgressError> {
        validate_plan(&plan)?;
        let initial_event = AgentProgressEvent {
            schema_version: CONTRACT_SCHEMA_VERSION,
            sequence: 1,
            task_id: plan.task_id.clone(),
            plan_id: plan.plan_id.clone(),
            plan_revision: plan.revision,
            plan_step_id: None,
            kind: AgentProgressKind::PlanCurrent,
        };
        Ok(Self {
            revisions: vec![plan],
            events: vec![initial_event],
        })
    }

    /// Returns every immutable plan revision.
    #[must_use]
    pub fn revisions(&self) -> &[Plan] {
        &self.revisions
    }

    /// Returns every content-free progress event.
    #[must_use]
    pub fn events(&self) -> &[AgentProgressEvent] {
        &self.events
    }

    /// Returns the current plan revision.
    #[must_use]
    pub fn current(&self) -> &Plan {
        self.revisions
            .last()
            .expect("construction records one plan")
    }

    /// Applies one exact legal step transition as a new immutable plan revision.
    pub fn transition_step(
        &mut self,
        step_id: &PlanStepId,
        target: PlanStepState,
    ) -> Result<&Plan, AgentProgressError> {
        if self.revisions.len() >= MAX_PLAN_REVISIONS {
            return Err(AgentProgressError::HistoryLimitReached);
        }
        let mut candidate = self.current().clone();
        candidate.revision = candidate
            .revision
            .checked_add(1)
            .ok_or(AgentProgressError::RevisionInvalid)?;
        let index = candidate
            .steps
            .iter()
            .position(|step| &step.plan_step_id == step_id)
            .ok_or(AgentProgressError::StepUnknown)?;
        let from = candidate.steps[index].state;
        if !legal_step_transition(from, target) {
            return Err(AgentProgressError::StepTransitionInvalid);
        }
        if matches!(target, PlanStepState::Ready | PlanStepState::Running)
            && candidate.steps[index].depends_on.iter().any(|dependency| {
                candidate
                    .steps
                    .iter()
                    .find(|step| &step.plan_step_id == dependency)
                    .is_none_or(|step| step.state != PlanStepState::Completed)
            })
        {
            return Err(AgentProgressError::DependencyIncomplete);
        }
        if target == PlanStepState::Running
            && candidate
                .steps
                .iter()
                .enumerate()
                .any(|(other, step)| other != index && step.state == PlanStepState::Running)
        {
            return Err(AgentProgressError::ActiveStepExists);
        }
        candidate.steps[index].state = target;
        candidate.state = derived_plan_state(&candidate);
        validate_plan(&candidate)?;
        self.append(candidate, Some(step_id.clone()), event_kind(target))
    }

    /// Handles a status, replacement, or extension message deterministically.
    pub fn handle_user_message(
        &mut self,
        intent: UserMessageIntent,
        cancellation: Option<(&CancellationToken, CancellationSignal)>,
    ) -> Result<UserMessageDisposition, AgentProgressError> {
        match intent {
            UserMessageIntent::StatusQuery => {
                if cancellation.is_some() {
                    return Err(AgentProgressError::UnexpectedCancellation);
                }
                Ok(UserMessageDisposition::StatusReturned)
            }
            UserMessageIntent::ReplaceTask | UserMessageIntent::ExtendTask => {
                if !matches!(
                    self.current().state,
                    PlanState::Proposed | PlanState::Current | PlanState::InProgress
                ) {
                    return Err(AgentProgressError::PlanNotInterruptible);
                }
                if self.revisions.len() >= MAX_PLAN_REVISIONS {
                    return Err(AgentProgressError::HistoryLimitReached);
                }
                let (token, signal) =
                    cancellation.ok_or(AgentProgressError::CancellationRequired)?;
                token
                    .cancel(signal)
                    .map_err(AgentProgressError::Cancellation)?;
                self.cancel_plan()?;
                Ok(match intent {
                    UserMessageIntent::ReplaceTask => {
                        UserMessageDisposition::InterruptedForReplacement
                    }
                    UserMessageIntent::ExtendTask => {
                        UserMessageDisposition::InterruptedForExtension
                    }
                    UserMessageIntent::StatusQuery => unreachable!(),
                })
            }
        }
    }

    /// Returns a content-free status response for the current revision.
    #[must_use]
    pub fn status(&self) -> AgentStatusResponse {
        let plan = self.current();
        let active = plan
            .steps
            .iter()
            .find(|step| step.state == PlanStepState::Running);
        let status = match plan.state {
            PlanState::InProgress => AgentStatusKind::Running,
            PlanState::Completed => AgentStatusKind::Completed,
            PlanState::Cancelled => AgentStatusKind::Cancelled,
            PlanState::Proposed | PlanState::Current | PlanState::Superseded => {
                if plan
                    .steps
                    .iter()
                    .any(|step| step.state == PlanStepState::Blocked)
                {
                    AgentStatusKind::Blocked
                } else {
                    AgentStatusKind::Planned
                }
            }
        };
        AgentStatusResponse {
            schema_version: CONTRACT_SCHEMA_VERSION,
            task_id: plan.task_id.clone(),
            plan_id: plan.plan_id.clone(),
            plan_revision: plan.revision,
            status,
            active_step_id: active.map(|step| step.plan_step_id.clone()),
            completed_steps: u32::try_from(
                plan.steps
                    .iter()
                    .filter(|step| step.state == PlanStepState::Completed)
                    .count(),
            )
            .expect("plan step bound fits u32"),
            total_steps: u32::try_from(plan.steps.len()).expect("plan step bound fits u32"),
        }
    }

    fn cancel_plan(&mut self) -> Result<&Plan, AgentProgressError> {
        if self.current().state == PlanState::Cancelled {
            return Ok(self.current());
        }
        if self.revisions.len() >= MAX_PLAN_REVISIONS {
            return Err(AgentProgressError::HistoryLimitReached);
        }
        let mut candidate = self.current().clone();
        candidate.revision = candidate
            .revision
            .checked_add(1)
            .ok_or(AgentProgressError::RevisionInvalid)?;
        for step in &mut candidate.steps {
            if matches!(
                step.state,
                PlanStepState::Proposed
                    | PlanStepState::Ready
                    | PlanStepState::Running
                    | PlanStepState::Blocked
            ) {
                step.state = PlanStepState::Cancelled;
            }
        }
        candidate.state = PlanState::Cancelled;
        validate_plan(&candidate)?;
        self.append(candidate, None, AgentProgressKind::Cancelled)
    }

    fn append(
        &mut self,
        candidate: Plan,
        step_id: Option<PlanStepId>,
        kind: AgentProgressKind,
    ) -> Result<&Plan, AgentProgressError> {
        let current = self.current();
        if candidate.plan_id != current.plan_id || candidate.task_id != current.task_id {
            return Err(AgentProgressError::IdentityChanged);
        }
        if candidate.revision != current.revision.saturating_add(1) {
            return Err(AgentProgressError::RevisionInvalid);
        }
        let sequence = u64::try_from(self.events.len())
            .ok()
            .and_then(|value| value.checked_add(1))
            .ok_or(AgentProgressError::RevisionInvalid)?;
        let event = AgentProgressEvent {
            schema_version: CONTRACT_SCHEMA_VERSION,
            sequence,
            task_id: candidate.task_id.clone(),
            plan_id: candidate.plan_id.clone(),
            plan_revision: candidate.revision,
            plan_step_id: step_id,
            kind,
        };
        self.revisions.push(candidate);
        self.events.push(event);
        Ok(self.current())
    }
}

/// Builds one bounded final response and rejects unsupported success or malformed text.
pub fn build_final_response(
    task_id: TaskId,
    state: AgentFinalState,
    summary: String,
    evidence: Vec<EvidenceReference>,
    unresolved: Vec<String>,
) -> Result<AgentFinalResponse, AgentProgressError> {
    if !valid_text(&summary)
        || evidence.len() > MAX_RESPONSE_EVIDENCE
        || unresolved.len() > MAX_RESPONSE_ITEMS
        || unresolved.iter().any(|item| !valid_text(item))
        || (state == AgentFinalState::Completed && (evidence.is_empty() || !unresolved.is_empty()))
        || (state == AgentFinalState::NotRun && !evidence.is_empty())
    {
        return Err(AgentProgressError::InvalidFinalResponse);
    }
    Ok(AgentFinalResponse {
        schema_version: CONTRACT_SCHEMA_VERSION,
        task_id,
        state,
        summary,
        evidence,
        unresolved,
    })
}

fn validate_plan(plan: &Plan) -> Result<(), AgentProgressError> {
    if plan.schema_version != CONTRACT_SCHEMA_VERSION
        || plan.revision == 0
        || plan.steps.is_empty()
        || plan.steps.len() > MAX_PLAN_STEPS
        || plan.plan_id.as_str().is_empty()
        || plan.task_id.as_str().is_empty()
    {
        return Err(AgentProgressError::InvalidPlan);
    }
    let mut running = 0;
    for (index, step) in plan.steps.iter().enumerate() {
        if usize::try_from(step.ordinal).ok() != Some(index)
            || step.plan_step_id.as_str().is_empty()
            || step.description.is_empty()
            || step.description.len() > MAX_TEXT_BYTES
            || plan.steps[..index]
                .iter()
                .any(|prior| prior.plan_step_id == step.plan_step_id)
            || step
                .depends_on
                .iter()
                .enumerate()
                .any(|(dependency_index, dependency)| {
                    step.depends_on[..dependency_index].contains(dependency)
                        || !plan.steps[..index]
                            .iter()
                            .any(|prior| &prior.plan_step_id == dependency)
                })
        {
            return Err(AgentProgressError::InvalidPlan);
        }
        running += usize::from(step.state == PlanStepState::Running);
    }
    let all_terminal = plan.steps.iter().all(|step| {
        matches!(
            step.state,
            PlanStepState::Completed | PlanStepState::Skipped
        )
    });
    let state_consistent = match plan.state {
        PlanState::Proposed => plan
            .steps
            .iter()
            .all(|step| step.state == PlanStepState::Proposed),
        PlanState::Current => running == 0 && !all_terminal,
        PlanState::InProgress => running == 1,
        PlanState::Completed => all_terminal,
        PlanState::Superseded | PlanState::Cancelled => running == 0,
    };
    if running > 1 || !state_consistent {
        return Err(AgentProgressError::InvalidPlan);
    }
    Ok(())
}

const fn legal_step_transition(from: PlanStepState, to: PlanStepState) -> bool {
    matches!(
        (from, to),
        (PlanStepState::Proposed, PlanStepState::Ready)
            | (PlanStepState::Proposed, PlanStepState::Skipped)
            | (PlanStepState::Proposed, PlanStepState::Cancelled)
            | (PlanStepState::Ready, PlanStepState::Running)
            | (PlanStepState::Ready, PlanStepState::Blocked)
            | (PlanStepState::Ready, PlanStepState::Skipped)
            | (PlanStepState::Ready, PlanStepState::Cancelled)
            | (PlanStepState::Running, PlanStepState::Completed)
            | (PlanStepState::Running, PlanStepState::Blocked)
            | (PlanStepState::Running, PlanStepState::Cancelled)
            | (PlanStepState::Blocked, PlanStepState::Ready)
            | (PlanStepState::Blocked, PlanStepState::Skipped)
            | (PlanStepState::Blocked, PlanStepState::Cancelled)
    )
}

fn derived_plan_state(plan: &Plan) -> PlanState {
    if plan.steps.iter().all(|step| {
        matches!(
            step.state,
            PlanStepState::Completed | PlanStepState::Skipped
        )
    }) {
        PlanState::Completed
    } else if plan
        .steps
        .iter()
        .any(|step| step.state == PlanStepState::Running)
    {
        PlanState::InProgress
    } else {
        PlanState::Current
    }
}

const fn event_kind(state: PlanStepState) -> AgentProgressKind {
    match state {
        PlanStepState::Ready => AgentProgressKind::StepReady,
        PlanStepState::Running => AgentProgressKind::StepStarted,
        PlanStepState::Completed => AgentProgressKind::StepCompleted,
        PlanStepState::Blocked => AgentProgressKind::StepBlocked,
        PlanStepState::Skipped => AgentProgressKind::StepSkipped,
        PlanStepState::Cancelled => AgentProgressKind::Cancelled,
        PlanStepState::Proposed => AgentProgressKind::PlanCurrent,
    }
}

fn valid_text(value: &str) -> bool {
    !value.is_empty() && value.len() <= MAX_TEXT_BYTES
}

#[cfg(test)]
mod tests {
    use super::{AgentProgressError, PlanProgressController, build_final_response};
    use crate::propagation::CancellationToken;
    use agentmage_kernel_contracts::{
        AgentFinalState, AgentProgressKind, AgentStatusKind, BoundaryKind, CONTRACT_SCHEMA_VERSION,
        CancellationId, CancellationReason, CancellationSignal, CorrelationId, EvidenceId,
        EvidenceKind, EvidenceReference, Plan, PlanId, PlanState, PlanStep, PlanStepId,
        PlanStepState, TaskId, UserMessageDisposition, UserMessageIntent,
    };

    fn plan() -> Plan {
        let first = PlanStepId::from_raw("plan-progress:step:0");
        Plan {
            schema_version: CONTRACT_SCHEMA_VERSION,
            plan_id: PlanId::from_raw("plan-progress"),
            task_id: TaskId::from_raw("task-progress"),
            work_packet_revision: 1,
            revision: 1,
            steps: vec![
                PlanStep {
                    plan_step_id: first.clone(),
                    ordinal: 0,
                    description: "Observe fixture".to_owned(),
                    depends_on: Vec::new(),
                    expected_evidence: vec![EvidenceKind::Observation],
                    state: PlanStepState::Proposed,
                },
                PlanStep {
                    plan_step_id: PlanStepId::from_raw("plan-progress:step:1"),
                    ordinal: 1,
                    description: "Verify result".to_owned(),
                    depends_on: vec![first],
                    expected_evidence: vec![EvidenceKind::Validation],
                    state: PlanStepState::Proposed,
                },
            ],
            state: PlanState::Proposed,
        }
    }

    fn signal() -> CancellationSignal {
        CancellationSignal {
            schema_version: CONTRACT_SCHEMA_VERSION,
            cancellation_id: CancellationId::from_raw("cancel-progress"),
            task_id: TaskId::from_raw("task-progress"),
            correlation_id: CorrelationId::from_raw("correlation-progress"),
            requested_by: BoundaryKind::Shell,
            reason: CancellationReason::UserRequested,
        }
    }

    fn evidence() -> EvidenceReference {
        EvidenceReference {
            schema_version: CONTRACT_SCHEMA_VERSION,
            evidence_id: EvidenceId::from_raw("evidence-progress"),
            kind: EvidenceKind::Observation,
            source_id: "synthetic".to_owned(),
            object_id: "postcondition".to_owned(),
            fragment: None,
            content_sha256: "a".repeat(64),
            observed_revision: Some("fixture-v1".to_owned()),
        }
    }

    #[test]
    fn one_active_step_dependencies_history_events_and_status_are_exact() {
        let mut controller = PlanProgressController::new(plan()).expect("valid plan");
        let first = PlanStepId::from_raw("plan-progress:step:0");
        let second = PlanStepId::from_raw("plan-progress:step:1");
        assert_eq!(controller.status().status, AgentStatusKind::Planned);
        assert_eq!(
            controller.transition_step(&second, PlanStepState::Ready),
            Err(AgentProgressError::DependencyIncomplete)
        );
        controller
            .transition_step(&first, PlanStepState::Ready)
            .expect("first ready");
        controller
            .transition_step(&first, PlanStepState::Running)
            .expect("first running");
        assert_eq!(controller.status().status, AgentStatusKind::Running);
        assert_eq!(controller.status().active_step_id, Some(first.clone()));
        assert_eq!(
            controller.transition_step(&second, PlanStepState::Running),
            Err(AgentProgressError::StepTransitionInvalid)
        );
        controller
            .transition_step(&first, PlanStepState::Completed)
            .expect("first complete");
        controller
            .transition_step(&second, PlanStepState::Ready)
            .expect("second ready");
        controller
            .transition_step(&second, PlanStepState::Running)
            .expect("second running");
        controller
            .transition_step(&second, PlanStepState::Completed)
            .expect("second complete");
        assert_eq!(controller.status().status, AgentStatusKind::Completed);
        assert_eq!(controller.revisions().len(), 7);
        assert_eq!(controller.events().len(), 7);
        assert_eq!(controller.events()[0].kind, AgentProgressKind::PlanCurrent);
        assert_eq!(
            controller.events()[6].kind,
            AgentProgressKind::StepCompleted
        );
    }

    #[test]
    fn competing_running_step_and_illegal_transition_preserve_history() {
        let mut candidate = plan();
        candidate.steps[0].state = PlanStepState::Running;
        candidate.steps[1].state = PlanStepState::Running;
        candidate.state = PlanState::InProgress;
        assert!(matches!(
            PlanProgressController::new(candidate),
            Err(AgentProgressError::InvalidPlan)
        ));
        let mut inconsistent = plan();
        inconsistent.steps[0].state = PlanStepState::Running;
        assert!(matches!(
            PlanProgressController::new(inconsistent),
            Err(AgentProgressError::InvalidPlan)
        ));

        let mut controller = PlanProgressController::new(plan()).expect("valid plan");
        let first = PlanStepId::from_raw("plan-progress:step:0");
        assert_eq!(
            controller.transition_step(&first, PlanStepState::Completed),
            Err(AgentProgressError::StepTransitionInvalid)
        );
        assert_eq!(controller.revisions().len(), 1);
        assert_eq!(controller.events().len(), 1);
    }

    #[test]
    fn status_query_is_non_interrupting_and_replace_or_extend_cancels_descendants() {
        for (intent, expected) in [
            (
                UserMessageIntent::ReplaceTask,
                UserMessageDisposition::InterruptedForReplacement,
            ),
            (
                UserMessageIntent::ExtendTask,
                UserMessageDisposition::InterruptedForExtension,
            ),
        ] {
            let mut controller = PlanProgressController::new(plan()).expect("valid plan");
            let token = CancellationToken::root(
                BoundaryKind::Shell,
                TaskId::from_raw("task-progress"),
                CorrelationId::from_raw("correlation-progress"),
            );
            let child = token
                .derive_child(BoundaryKind::Kernel)
                .expect("kernel child");
            assert_eq!(
                controller
                    .handle_user_message(UserMessageIntent::StatusQuery, None)
                    .expect("status"),
                UserMessageDisposition::StatusReturned
            );
            assert!(!token.is_cancelled());
            assert_eq!(
                controller
                    .handle_user_message(intent, Some((&token, signal())))
                    .expect("interrupt"),
                expected
            );
            assert!(token.is_cancelled());
            assert!(child.is_cancelled());
            assert_eq!(controller.status().status, AgentStatusKind::Cancelled);
            assert_eq!(
                controller.events().last().unwrap().kind,
                AgentProgressKind::Cancelled
            );
        }
    }

    #[test]
    fn interruption_material_is_required_only_for_interrupting_messages() {
        let mut controller = PlanProgressController::new(plan()).expect("valid plan");
        assert_eq!(
            controller.handle_user_message(UserMessageIntent::ReplaceTask, None),
            Err(AgentProgressError::CancellationRequired)
        );
        let token = CancellationToken::root(
            BoundaryKind::Shell,
            TaskId::from_raw("task-progress"),
            CorrelationId::from_raw("correlation-progress"),
        );
        assert_eq!(
            controller
                .handle_user_message(UserMessageIntent::StatusQuery, Some((&token, signal()))),
            Err(AgentProgressError::UnexpectedCancellation)
        );
        assert!(!token.is_cancelled());
        assert_eq!(controller.revisions().len(), 1);

        let mut terminal = PlanProgressController::new(plan()).expect("valid plan");
        terminal.cancel_plan().expect("cancel plan");
        let terminal_token = CancellationToken::root(
            BoundaryKind::Shell,
            TaskId::from_raw("task-progress"),
            CorrelationId::from_raw("correlation-progress"),
        );
        assert_eq!(
            terminal.handle_user_message(
                UserMessageIntent::ReplaceTask,
                Some((&terminal_token, signal())),
            ),
            Err(AgentProgressError::PlanNotInterruptible)
        );
        assert!(!terminal_token.is_cancelled());
    }

    #[test]
    fn final_response_states_are_explicit_and_completion_requires_evidence() {
        for state in [
            AgentFinalState::Failed,
            AgentFinalState::Blocked,
            AgentFinalState::Unknown,
            AgentFinalState::Cancelled,
        ] {
            let response = build_final_response(
                TaskId::from_raw("task-progress"),
                state,
                "Work did not complete".to_owned(),
                Vec::new(),
                vec!["One unresolved item".to_owned()],
            )
            .expect("explicit non-success response");
            assert_eq!(response.state, state);
        }
        assert_eq!(
            build_final_response(
                TaskId::from_raw("task-progress"),
                AgentFinalState::Completed,
                "Unsupported completion".to_owned(),
                Vec::new(),
                Vec::new(),
            ),
            Err(AgentProgressError::InvalidFinalResponse)
        );
        assert!(
            build_final_response(
                TaskId::from_raw("task-progress"),
                AgentFinalState::Completed,
                "Evidence-backed completion".to_owned(),
                vec![evidence()],
                Vec::new(),
            )
            .is_ok()
        );
        assert_eq!(
            build_final_response(
                TaskId::from_raw("task-progress"),
                AgentFinalState::Completed,
                "Contradictory completion".to_owned(),
                vec![evidence()],
                vec!["Still unresolved".to_owned()],
            ),
            Err(AgentProgressError::InvalidFinalResponse)
        );
        assert_eq!(
            build_final_response(
                TaskId::from_raw("task-progress"),
                AgentFinalState::NotRun,
                "Not attempted".to_owned(),
                vec![evidence()],
                vec!["Not run".to_owned()],
            ),
            Err(AgentProgressError::InvalidFinalResponse)
        );
    }
}
