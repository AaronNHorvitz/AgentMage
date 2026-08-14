//! Persistable deterministic agent-state contracts.

use crate::CONTRACT_SCHEMA_VERSION;

/// Exact active or terminal state of one deterministic agent task.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
pub enum AgentStateKind {
    /// Gather bounded observations before proposing work.
    #[serde(rename = "observation")]
    Observation,
    /// Hold one non-authoritative proposed next action.
    #[serde(rename = "proposal")]
    Proposal,
    /// Validate the exact proposal and its bound identities.
    #[serde(rename = "validation")]
    Validation,
    /// Await a material clarification from the user.
    #[serde(rename = "clarification")]
    Clarification,
    /// Await or evaluate exact approval authority.
    #[serde(rename = "approval")]
    Approval,
    /// Supervise one authorized bounded execution attempt.
    #[serde(rename = "execution")]
    Execution,
    /// Verify typed postconditions for the exact attempt.
    #[serde(rename = "verification")]
    Verification,
    /// Persist a reconciled checkpoint before continuation or completion.
    #[serde(rename = "checkpoint")]
    Checkpoint,
    /// Terminal verified success.
    #[serde(rename = "SUCCESS")]
    Success,
    /// Terminal verified no-op success.
    #[serde(rename = "NO_OP")]
    NoOp,
    /// Terminal safe stop because a prerequisite or policy boundary blocked work.
    #[serde(rename = "BLOCKED")]
    Blocked,
    /// Terminal user or policy decision declining the proposed work.
    #[serde(rename = "DECLINED")]
    Declined,
    /// Terminal stop after the declared no-progress ceiling.
    #[serde(rename = "STALLED")]
    Stalled,
    /// Terminal stop after a declared resource or retry ceiling.
    #[serde(rename = "EXHAUSTED")]
    Exhausted,
    /// Terminal stop because an effect or material fact cannot be established safely.
    #[serde(rename = "UNCERTAIN")]
    Uncertain,
    /// Terminal user or kernel cancellation.
    #[serde(rename = "CANCELLED")]
    Cancelled,
    /// Terminal typed failure.
    #[serde(rename = "FAILED")]
    Failed,
}

impl AgentStateKind {
    /// Returns whether this state ends the current task.
    #[must_use]
    pub const fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::Success
                | Self::NoOp
                | Self::Blocked
                | Self::Declined
                | Self::Stalled
                | Self::Exhausted
                | Self::Uncertain
                | Self::Cancelled
                | Self::Failed
        )
    }

    /// Returns whether this terminal state represents verified success.
    #[must_use]
    pub const fn is_success(self) -> bool {
        matches!(self, Self::Success | Self::NoOp)
    }
}

/// One append-only state transition suitable for durable serialization.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentStateTransition {
    /// Contract schema version.
    pub schema_version: u16,
    /// Monotonic state revision after this transition.
    pub revision: u64,
    /// Exact prior state.
    pub from: AgentStateKind,
    /// Exact resulting state.
    pub to: AgentStateKind,
}

impl AgentStateTransition {
    /// Builds a transition under the current contract version.
    #[must_use]
    pub const fn new(revision: u64, from: AgentStateKind, to: AgentStateKind) -> Self {
        Self {
            schema_version: CONTRACT_SCHEMA_VERSION,
            revision,
            from,
            to,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::AgentStateKind;

    #[test]
    fn terminal_and_success_sets_are_exact() {
        let states = [
            AgentStateKind::Observation,
            AgentStateKind::Proposal,
            AgentStateKind::Validation,
            AgentStateKind::Clarification,
            AgentStateKind::Approval,
            AgentStateKind::Execution,
            AgentStateKind::Verification,
            AgentStateKind::Checkpoint,
            AgentStateKind::Success,
            AgentStateKind::NoOp,
            AgentStateKind::Blocked,
            AgentStateKind::Declined,
            AgentStateKind::Stalled,
            AgentStateKind::Exhausted,
            AgentStateKind::Uncertain,
            AgentStateKind::Cancelled,
            AgentStateKind::Failed,
        ];
        assert_eq!(states.iter().filter(|state| state.is_terminal()).count(), 9);
        assert_eq!(states.iter().filter(|state| state.is_success()).count(), 2);
    }
}
