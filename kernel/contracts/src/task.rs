//! Task, work-packet, plan, and action contracts.

use crate::{
    ActionId, EvidenceKind, PlanId, PlanStepId, SessionId, TaskId, ValidationIssue, WorkPacketId,
};

/// Lifecycle state of a user-directed task.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TaskStatus {
    /// The task is being assembled and is not runnable.
    Draft,
    /// The task passed its current validation boundary.
    Ready,
    /// At least one authorized action is in progress.
    Running,
    /// Progress is visibly blocked.
    Blocked,
    /// All declared acceptance criteria have verified evidence.
    Completed,
    /// The user or kernel cancelled the task.
    Cancelled,
    /// The task ended with a typed failure.
    Failed,
}

/// Versioned description of one user-directed task.
///
/// A task describes desired work but carries no operation authority.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Task {
    /// Contract schema version.
    pub schema_version: u16,
    /// Stable task identity.
    pub task_id: TaskId,
    /// Owning local session.
    pub session_id: SessionId,
    /// User-directed objective.
    pub objective: String,
    /// Conditions that must be evidenced before completion.
    pub acceptance_criteria: Vec<String>,
    /// Explicit task constraints and exclusions.
    pub constraints: Vec<String>,
    /// Current task state.
    pub status: TaskStatus,
}

/// Lifecycle state of a revision-controlled work packet.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorkPacketState {
    /// The packet is incomplete and cannot be scheduled.
    Draft,
    /// The packet passed closed validation.
    Validated,
    /// A plan has been attached.
    Planned,
    /// The packet has current work in progress.
    Active,
    /// The packet cannot progress without a visible decision or dependency.
    Blocked,
    /// The packet's acceptance criteria have verified evidence.
    Completed,
    /// The packet was cancelled.
    Cancelled,
}

/// Versioned snapshot of the exact work submitted to planning and execution.
///
/// Work packets are descriptive records. They cannot grant access or authorize effects.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkPacket {
    /// Contract schema version.
    pub schema_version: u16,
    /// Stable packet identity.
    pub work_packet_id: WorkPacketId,
    /// Task represented by this packet.
    pub task_id: TaskId,
    /// Monotonically increasing packet revision.
    pub revision: u32,
    /// Exact objective at this revision.
    pub objective: String,
    /// Exact acceptance criteria at this revision.
    pub acceptance_criteria: Vec<String>,
    /// Exact constraints at this revision.
    pub constraints: Vec<String>,
    /// Current packet-validation findings.
    pub validation_issues: Vec<ValidationIssue>,
    /// Attached plan identity, if planning has completed.
    pub plan_id: Option<PlanId>,
    /// Current packet state.
    pub state: WorkPacketState,
}

/// Lifecycle state of a plan.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlanState {
    /// The plan is a proposal and cannot authorize an action.
    Proposed,
    /// The plan is the current descriptive plan for its packet.
    Current,
    /// One or more steps are in progress.
    InProgress,
    /// All required steps have completed with evidence.
    Completed,
    /// A newer revision replaced the plan.
    Superseded,
    /// The plan was cancelled.
    Cancelled,
}

/// Lifecycle state of one plan step.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlanStepState {
    /// The step is proposed.
    Proposed,
    /// All declared dependencies are satisfied.
    Ready,
    /// The step is in progress.
    Running,
    /// The step is visibly blocked.
    Blocked,
    /// The step completed with required evidence.
    Completed,
    /// The step was explicitly skipped with a recorded reason.
    Skipped,
    /// The step was cancelled.
    Cancelled,
}

/// One ordered descriptive step in a task plan.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlanStep {
    /// Stable step identity.
    pub plan_step_id: PlanStepId,
    /// Stable zero-based order inside the plan.
    pub ordinal: u32,
    /// Human-readable proposed work.
    pub description: String,
    /// Step identities that must complete first.
    pub depends_on: Vec<PlanStepId>,
    /// Evidence classes required before this step may be complete.
    pub expected_evidence: Vec<EvidenceKind>,
    /// Current step state.
    pub state: PlanStepState,
}

/// Versioned ordered plan for one work-packet revision.
///
/// A plan describes intended actions but cannot authorize them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Plan {
    /// Contract schema version.
    pub schema_version: u16,
    /// Stable plan identity.
    pub plan_id: PlanId,
    /// Task the plan serves.
    pub task_id: TaskId,
    /// Exact work-packet revision used to produce the plan.
    pub work_packet_revision: u32,
    /// Plan revision.
    pub revision: u32,
    /// Ordered plan steps.
    pub steps: Vec<PlanStep>,
    /// Current plan state.
    pub state: PlanState,
}

/// Descriptive class of a proposed action.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActionKind {
    /// A deterministic registered tool may be proposed.
    DeterministicTool,
    /// A local model inference may be proposed.
    ModelInference,
    /// A decision from the user is required.
    UserDecision,
    /// A policy or validation decision from the kernel is required.
    KernelDecision,
}

/// Lifecycle state of an action record.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActionState {
    /// The action is descriptive only.
    Proposed,
    /// The action passed non-authority validation and awaits dispatch checks.
    Ready,
    /// The action attempt is in progress.
    Running,
    /// The action succeeded with a terminal result.
    Succeeded,
    /// Current policy denied the action.
    Denied,
    /// The action failed.
    Failed,
    /// The action was cancelled.
    Cancelled,
    /// The action timed out.
    TimedOut,
    /// The action may have produced an effect and requires recovery.
    Uncertain,
}

/// Versioned description and state of one bounded action.
///
/// An action does not contain authority. Dispatch must pair it with separately validated
/// authority at the kernel boundary introduced by the capability-grant story.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Action {
    /// Contract schema version.
    pub schema_version: u16,
    /// Stable action identity.
    pub action_id: ActionId,
    /// Owning task identity.
    pub task_id: TaskId,
    /// Optional plan step that proposed the action.
    pub plan_step_id: Option<PlanStepId>,
    /// Descriptive action class.
    pub kind: ActionKind,
    /// Bounded human-readable action description.
    pub description: String,
    /// Declared expected effects for review and postcondition checks.
    pub expected_effects: Vec<String>,
    /// Current action state.
    pub state: ActionState,
}

#[cfg(test)]
mod tests {
    use super::{Action, ActionKind, ActionState};
    use crate::{ActionId, TaskId};

    #[test]
    fn action_is_descriptive_and_has_no_embedded_authority() {
        let action = Action {
            schema_version: 1,
            action_id: ActionId::from_raw("action-0001"),
            task_id: TaskId::from_raw("task-0001"),
            plan_step_id: None,
            kind: ActionKind::DeterministicTool,
            description: "Read one synthetic fixture".to_owned(),
            expected_effects: vec!["read-only-observation".to_owned()],
            state: ActionState::Proposed,
        };
        assert_eq!(action.state, ActionState::Proposed);
        assert_eq!(action.expected_effects, ["read-only-observation"]);
    }
}
