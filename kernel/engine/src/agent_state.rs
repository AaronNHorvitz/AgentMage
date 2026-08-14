//! Deterministic legal transitions for the persisted agent-state contract.

use agentmage_kernel_contracts::{AgentStateKind, AgentStateTransition};

use crate::agent_restart::RestartPermit;
use crate::agent_verifier::VerifiedCompletion;

const MAX_STATE_REVISIONS: usize = 4_096;

/// Stable reason an agent-state transition is refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AgentStateError {
    /// The requested edge is absent from the closed transition graph.
    IllegalTransition,
    /// Ordinary transitions cannot produce verified-success states.
    VerifierRequired,
    /// The state history reached its fixed bound.
    HistoryLimitReached,
    /// A revision counter overflowed.
    RevisionOverflow,
    /// A verifier proof names another state revision.
    VerifierStateMismatch,
    /// A restored controller has not reconciled persisted restart state.
    RestartRequired,
    /// A restart permit names another state or revision.
    RestartPermitMismatch,
    /// A restored state or revision is not valid.
    InvalidRestoredState,
}

/// Append-only deterministic state controller for one task.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AgentStateController {
    current: AgentStateKind,
    revision: u64,
    transitions: Vec<AgentStateTransition>,
    restart_required: bool,
}

impl AgentStateController {
    /// Starts a new task in the observation state.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            current: AgentStateKind::Observation,
            revision: 1,
            transitions: Vec::new(),
            restart_required: false,
        }
    }

    /// Reconstructs one interrupted active state locked behind restart reconciliation.
    pub fn restore(current: AgentStateKind, revision: u64) -> Result<Self, AgentStateError> {
        if revision == 0 || current.is_terminal() {
            return Err(AgentStateError::InvalidRestoredState);
        }
        Ok(Self {
            current,
            revision,
            transitions: Vec::new(),
            restart_required: true,
        })
    }

    /// Consumes one exact reconciliation permit and unlocks future transitions.
    pub fn resume_after_restart(&mut self, permit: RestartPermit) -> Result<(), AgentStateError> {
        if !self.restart_required {
            return Err(AgentStateError::RestartPermitMismatch);
        }
        if permit.agent_state() != self.current || permit.agent_state_revision() != self.revision {
            return Err(AgentStateError::RestartPermitMismatch);
        }
        self.restart_required = false;
        Ok(())
    }

    /// Returns the exact current state.
    #[must_use]
    pub const fn current(&self) -> AgentStateKind {
        self.current
    }

    /// Returns the current monotonic state revision.
    #[must_use]
    pub const fn revision(&self) -> u64 {
        self.revision
    }

    /// Returns every admitted append-only transition.
    #[must_use]
    pub fn transitions(&self) -> &[AgentStateTransition] {
        &self.transitions
    }

    /// Applies one exact legal non-success transition.
    pub fn transition(
        &mut self,
        target: AgentStateKind,
    ) -> Result<&AgentStateTransition, AgentStateError> {
        if self.restart_required {
            return Err(AgentStateError::RestartRequired);
        }
        if target.is_success() {
            return Err(AgentStateError::VerifierRequired);
        }
        if !legal_transition(self.current, target) {
            return Err(AgentStateError::IllegalTransition);
        }
        self.append(target)
    }

    /// Completes the exact current verification state using one opaque verifier proof.
    pub fn complete(
        &mut self,
        completion: &VerifiedCompletion,
    ) -> Result<&AgentStateTransition, AgentStateError> {
        if self.restart_required {
            return Err(AgentStateError::RestartRequired);
        }
        if completion.state_revision() != self.revision {
            return Err(AgentStateError::VerifierStateMismatch);
        }
        let target = completion.target();
        if !legal_transition(self.current, target) {
            return Err(AgentStateError::IllegalTransition);
        }
        self.append(target)
    }

    fn append(&mut self, target: AgentStateKind) -> Result<&AgentStateTransition, AgentStateError> {
        if self.transitions.len() >= MAX_STATE_REVISIONS {
            return Err(AgentStateError::HistoryLimitReached);
        }
        let revision = self
            .revision
            .checked_add(1)
            .ok_or(AgentStateError::RevisionOverflow)?;
        self.transitions
            .push(AgentStateTransition::new(revision, self.current, target));
        self.current = target;
        self.revision = revision;
        Ok(self
            .transitions
            .last()
            .expect("one transition was appended"))
    }
}

impl Default for AgentStateController {
    fn default() -> Self {
        Self::new()
    }
}

/// Returns whether one edge belongs to the closed deterministic state graph.
#[must_use]
pub const fn legal_transition(from: AgentStateKind, to: AgentStateKind) -> bool {
    use AgentStateKind as State;
    match from {
        State::Observation => matches!(
            to,
            State::Proposal | State::Blocked | State::Exhausted | State::Cancelled | State::Failed
        ),
        State::Proposal => matches!(
            to,
            State::Validation
                | State::Blocked
                | State::Exhausted
                | State::Cancelled
                | State::Failed
        ),
        State::Validation => matches!(
            to,
            State::Clarification
                | State::Approval
                | State::Blocked
                | State::Declined
                | State::Exhausted
                | State::Uncertain
                | State::Cancelled
                | State::Failed
        ),
        State::Clarification => matches!(
            to,
            State::Proposal
                | State::Blocked
                | State::Declined
                | State::Exhausted
                | State::Cancelled
                | State::Failed
        ),
        State::Approval => matches!(
            to,
            State::Execution
                | State::Blocked
                | State::Declined
                | State::Exhausted
                | State::Cancelled
                | State::Failed
        ),
        State::Execution => matches!(
            to,
            State::Verification
                | State::Exhausted
                | State::Uncertain
                | State::Cancelled
                | State::Failed
        ),
        State::Verification => matches!(
            to,
            State::Checkpoint
                | State::Success
                | State::NoOp
                | State::Blocked
                | State::Exhausted
                | State::Uncertain
                | State::Cancelled
                | State::Failed
        ),
        State::Checkpoint => matches!(
            to,
            State::Observation
                | State::Success
                | State::NoOp
                | State::Blocked
                | State::Stalled
                | State::Exhausted
                | State::Uncertain
                | State::Cancelled
                | State::Failed
        ),
        State::Success
        | State::NoOp
        | State::Blocked
        | State::Declined
        | State::Stalled
        | State::Exhausted
        | State::Uncertain
        | State::Cancelled
        | State::Failed => false,
    }
}

#[cfg(test)]
mod tests {
    use super::{AgentStateController, AgentStateError, legal_transition};
    use agentmage_kernel_contracts::AgentStateKind;

    const STATES: [AgentStateKind; 17] = [
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

    #[test]
    fn task_12_2_1_1_state_catalog_and_wire_names_are_exact() {
        let names: Vec<String> = STATES
            .iter()
            .map(|state| serde_json::to_string(state).expect("state serializes"))
            .collect();
        assert_eq!(
            names,
            [
                "\"observation\"",
                "\"proposal\"",
                "\"validation\"",
                "\"clarification\"",
                "\"approval\"",
                "\"execution\"",
                "\"verification\"",
                "\"checkpoint\"",
                "\"SUCCESS\"",
                "\"NO_OP\"",
                "\"BLOCKED\"",
                "\"DECLINED\"",
                "\"STALLED\"",
                "\"EXHAUSTED\"",
                "\"UNCERTAIN\"",
                "\"CANCELLED\"",
                "\"FAILED\"",
            ]
        );
    }

    #[test]
    fn task_12_2_1_1_every_state_pair_has_one_deterministic_disposition() {
        let admitted = STATES
            .iter()
            .flat_map(|from| STATES.iter().map(move |to| (*from, *to)))
            .filter(|(from, to)| legal_transition(*from, *to))
            .count();
        assert_eq!(STATES.len() * STATES.len(), 289);
        assert_eq!(admitted, 52);
    }

    #[test]
    fn task_12_2_1_1_ordinary_transition_cannot_claim_success() {
        for target in [AgentStateKind::Success, AgentStateKind::NoOp] {
            let mut controller = AgentStateController::new();
            controller
                .transition(AgentStateKind::Proposal)
                .expect("proposal");
            controller
                .transition(AgentStateKind::Validation)
                .expect("validation");
            controller
                .transition(AgentStateKind::Approval)
                .expect("approval");
            controller
                .transition(AgentStateKind::Execution)
                .expect("execution");
            controller
                .transition(AgentStateKind::Verification)
                .expect("verification");
            let before = controller.clone();
            assert_eq!(
                controller.transition(target),
                Err(AgentStateError::VerifierRequired)
            );
            assert_eq!(controller, before);
        }
    }

    #[test]
    fn task_12_2_1_1_terminal_states_are_sticky() {
        for terminal in STATES.into_iter().filter(|state| state.is_terminal()) {
            for target in STATES {
                assert!(!legal_transition(terminal, target));
            }
        }
        let mut controller = AgentStateController::new();
        controller
            .transition(AgentStateKind::Blocked)
            .expect("blocked");
        let before = controller.clone();
        assert_eq!(
            controller.transition(AgentStateKind::Proposal),
            Err(AgentStateError::IllegalTransition)
        );
        assert_eq!(controller, before);
    }

    #[test]
    fn task_12_2_1_1_revisions_and_history_are_append_only() {
        let mut controller = AgentStateController::new();
        for target in [
            AgentStateKind::Proposal,
            AgentStateKind::Validation,
            AgentStateKind::Approval,
            AgentStateKind::Execution,
            AgentStateKind::Verification,
            AgentStateKind::Checkpoint,
            AgentStateKind::Observation,
        ] {
            controller.transition(target).expect("legal transition");
        }
        assert_eq!(controller.revision(), 8);
        assert_eq!(controller.transitions().len(), 7);
        for (index, transition) in controller.transitions().iter().enumerate() {
            assert_eq!(
                transition.revision,
                u64::try_from(index).expect("index") + 2
            );
        }
    }

    #[test]
    fn task_12_2_1_1_success_edges_exist_but_have_no_ordinary_route() {
        for source in [AgentStateKind::Verification, AgentStateKind::Checkpoint] {
            assert!(legal_transition(source, AgentStateKind::Success));
            assert!(legal_transition(source, AgentStateKind::NoOp));
        }
        let mut controller = AgentStateController::new();
        assert_eq!(
            controller.transition(AgentStateKind::Success),
            Err(AgentStateError::VerifierRequired)
        );
        assert_eq!(controller.current(), AgentStateKind::Observation);
    }

    #[test]
    fn d027_s12_state_every_graph_pair_and_controller_disposition_is_exact() {
        let mut legal_edges = 0;
        let mut illegal_edges = 0;
        let mut ordinary_admissions = 0;
        let mut verifier_gated = 0;

        for from in STATES {
            for to in STATES {
                let graph_legal = legal_transition(from, to);
                if graph_legal {
                    legal_edges += 1;
                } else {
                    illegal_edges += 1;
                }

                let mut controller = AgentStateController {
                    current: from,
                    revision: 1,
                    transitions: Vec::new(),
                    restart_required: false,
                };
                let before = controller.clone();
                if to.is_success() {
                    assert_eq!(
                        controller.transition(to),
                        Err(AgentStateError::VerifierRequired)
                    );
                    assert_eq!(controller, before);
                    verifier_gated += 1;
                } else if graph_legal {
                    let transition = controller.transition(to).expect("legal ordinary edge");
                    assert_eq!(transition.from, from);
                    assert_eq!(transition.to, to);
                    assert_eq!(controller.current(), to);
                    ordinary_admissions += 1;
                } else {
                    assert_eq!(
                        controller.transition(to),
                        Err(AgentStateError::IllegalTransition)
                    );
                    assert_eq!(controller, before);
                }
            }
        }

        assert_eq!(legal_edges, 52);
        assert_eq!(illegal_edges, 237);
        assert_eq!(ordinary_admissions, 48);
        assert_eq!(verifier_gated, 34);
        assert_eq!(STATES.iter().filter(|state| state.is_terminal()).count(), 9);
    }
}
