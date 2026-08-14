//! Agent progress, status, interruption, and final-response contracts.

use crate::{EvidenceReference, PlanId, PlanStepId, TaskId};

/// Closed class of one user-visible progress event.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentProgressKind {
    /// A plan became current.
    PlanCurrent,
    /// A step became eligible to run.
    StepReady,
    /// One exact step began running.
    StepStarted,
    /// One exact step completed.
    StepCompleted,
    /// One exact step became visibly blocked.
    StepBlocked,
    /// One exact step was explicitly skipped.
    StepSkipped,
    /// Active work was cancelled.
    Cancelled,
}

/// Versioned content-free event for one plan transition.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentProgressEvent {
    /// Contract schema version.
    pub schema_version: u16,
    /// Monotonic one-based sequence within the controller.
    pub sequence: u64,
    /// Exact task identity.
    pub task_id: TaskId,
    /// Exact plan identity.
    pub plan_id: PlanId,
    /// Plan revision produced by the event.
    pub plan_revision: u32,
    /// Step identity when the event describes one step.
    #[serde(deserialize_with = "crate::serialization::deserialize_required_option")]
    pub plan_step_id: Option<PlanStepId>,
    /// Closed event class.
    pub kind: AgentProgressKind,
}

/// Closed user-visible task status class.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentStatusKind {
    /// A plan exists but no step is running.
    Planned,
    /// Exactly one step is running.
    Running,
    /// At least one unfinished step is blocked and none is running.
    Blocked,
    /// Every non-skipped step completed.
    Completed,
    /// The task was cancelled.
    Cancelled,
}

/// Versioned content-free status response for one exact plan revision.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentStatusResponse {
    /// Contract schema version.
    pub schema_version: u16,
    /// Exact task identity.
    pub task_id: TaskId,
    /// Exact plan identity.
    pub plan_id: PlanId,
    /// Current plan revision.
    pub plan_revision: u32,
    /// Closed status class.
    pub status: AgentStatusKind,
    /// Running step identity, if and only if status is `Running`.
    #[serde(deserialize_with = "crate::serialization::deserialize_required_option")]
    pub active_step_id: Option<PlanStepId>,
    /// Number of completed steps.
    pub completed_steps: u32,
    /// Total step count.
    pub total_steps: u32,
}

/// Closed intent for a new user message during active work.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UserMessageIntent {
    /// Ask for current status without interrupting work.
    StatusQuery,
    /// Interrupt current work so a newer packet can replace it.
    ReplaceTask,
    /// Interrupt current work so a newer packet can extend it.
    ExtendTask,
}

/// Deterministic handling disposition for a new user message.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UserMessageDisposition {
    /// Status was returned and current work was not interrupted.
    StatusReturned,
    /// Current work was cancelled before replacement planning.
    InterruptedForReplacement,
    /// Current work was cancelled before extension planning.
    InterruptedForExtension,
}

/// Closed final-response state before Story 12.2 adds persisted terminal-result detail.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentFinalState {
    /// Declared work completed with current evidence.
    Completed,
    /// Work ended with a typed failure.
    Failed,
    /// Work cannot continue without a decision or dependency.
    Blocked,
    /// Available evidence cannot establish a safe result.
    Unknown,
    /// Work was not attempted.
    NotRun,
    /// Work was cancelled.
    Cancelled,
}

/// Versioned final-response contract with explicit evidence and unresolved items.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentFinalResponse {
    /// Contract schema version.
    pub schema_version: u16,
    /// Exact task identity.
    pub task_id: TaskId,
    /// Closed final state.
    pub state: AgentFinalState,
    /// Bounded user-visible summary.
    pub summary: String,
    /// Current evidence supporting a completed response or explaining another state.
    pub evidence: Vec<EvidenceReference>,
    /// Bounded unresolved, blocked, failed, unknown, or not-run items.
    pub unresolved: Vec<String>,
}
