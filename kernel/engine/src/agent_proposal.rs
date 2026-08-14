//! Exact proposal identity admission without authority or execution.

use std::collections::BTreeSet;

use agentmage_kernel_contracts::{
    AgentProposal, ContextPacketId, CorrelationId, ModelRunId, PolicyId, ProposalId,
    RepositorySnapshotId, SessionId, TaskId, ToolCatalogId,
};

const MAX_IDENTIFIER_BYTES: usize = 128;

/// Stable reason an inert proposal candidate is refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProposalAdmissionError {
    /// One candidate identity or digest is malformed.
    InvalidIdentity,
    /// Session or task ownership changed.
    OwnershipMismatch,
    /// Model-run or context-packet identity changed.
    ModelContextMismatch,
    /// Repository snapshot, tool catalog, or policy changed.
    EvaluationContextMismatch,
    /// Correlation identity changed.
    CorrelationMismatch,
    /// Candidate turn is older than the exact expected turn.
    StaleTurn,
    /// Candidate turn is newer than the exact expected turn.
    FutureTurn,
    /// This exact proposal identity was already observed.
    Replayed,
    /// Another proposal was already admitted for this turn.
    CompetingProposal,
}

/// Exact expected proposal tuple for one turn.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExpectedProposalContext {
    /// Exact session identity.
    pub session_id: SessionId,
    /// Exact task identity.
    pub task_id: TaskId,
    /// Positive expected turn.
    pub turn: u64,
    /// Exact model-run identity.
    pub model_run_id: ModelRunId,
    /// Exact context-packet identity.
    pub context_packet_id: ContextPacketId,
    /// Exact repository-snapshot identity.
    pub repository_snapshot_id: RepositorySnapshotId,
    /// Exact tool-catalog identity.
    pub tool_catalog_id: ToolCatalogId,
    /// Exact policy identity.
    pub policy_id: PolicyId,
    /// Exact correlation identity.
    pub correlation_id: CorrelationId,
}

/// Content-free record that one exact candidate passed identity admission.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AdmittedProposal {
    proposal_id: ProposalId,
    turn: u64,
    proposal_sha256: String,
}

impl AdmittedProposal {
    /// Returns the admitted proposal identity.
    #[must_use]
    pub const fn proposal_id(&self) -> &ProposalId {
        &self.proposal_id
    }

    /// Returns the admitted turn.
    #[must_use]
    pub const fn turn(&self) -> u64 {
        self.turn
    }

    /// Returns the digest of separately validated closed proposal bytes.
    #[must_use]
    pub fn proposal_sha256(&self) -> &str {
        &self.proposal_sha256
    }
}

/// One-turn replay and ambiguity guard for proposal identity admission.
pub struct ProposalAdmissionRegistry {
    expected: ExpectedProposalContext,
    seen: BTreeSet<ProposalId>,
    admitted: Option<ProposalId>,
}

impl ProposalAdmissionRegistry {
    /// Creates a registry for one exact positive turn and context tuple.
    pub fn new(expected: ExpectedProposalContext) -> Result<Self, ProposalAdmissionError> {
        if expected.turn == 0 || !valid_context(&expected) {
            return Err(ProposalAdmissionError::InvalidIdentity);
        }
        Ok(Self {
            expected,
            seen: BTreeSet::new(),
            admitted: None,
        })
    }

    /// Admits at most one exact candidate for the expected turn.
    pub fn admit(
        &mut self,
        candidate: &AgentProposal,
    ) -> Result<AdmittedProposal, ProposalAdmissionError> {
        validate_candidate(candidate)?;
        if candidate.turn < self.expected.turn {
            return Err(ProposalAdmissionError::StaleTurn);
        }
        if candidate.turn > self.expected.turn {
            return Err(ProposalAdmissionError::FutureTurn);
        }
        if candidate.session_id != self.expected.session_id
            || candidate.task_id != self.expected.task_id
        {
            return Err(ProposalAdmissionError::OwnershipMismatch);
        }
        if candidate.model_run_id != self.expected.model_run_id
            || candidate.context_packet_id != self.expected.context_packet_id
        {
            return Err(ProposalAdmissionError::ModelContextMismatch);
        }
        if candidate.repository_snapshot_id != self.expected.repository_snapshot_id
            || candidate.tool_catalog_id != self.expected.tool_catalog_id
            || candidate.policy_id != self.expected.policy_id
        {
            return Err(ProposalAdmissionError::EvaluationContextMismatch);
        }
        if candidate.correlation_id != self.expected.correlation_id {
            return Err(ProposalAdmissionError::CorrelationMismatch);
        }
        if self.seen.contains(&candidate.proposal_id) {
            return Err(ProposalAdmissionError::Replayed);
        }
        if self.admitted.is_some() {
            self.seen.insert(candidate.proposal_id.clone());
            return Err(ProposalAdmissionError::CompetingProposal);
        }
        self.seen.insert(candidate.proposal_id.clone());
        self.admitted = Some(candidate.proposal_id.clone());
        Ok(AdmittedProposal {
            proposal_id: candidate.proposal_id.clone(),
            turn: candidate.turn,
            proposal_sha256: candidate.proposal_sha256.clone(),
        })
    }

    /// Returns the number of candidate identities observed after full tuple validation.
    #[must_use]
    pub fn seen_count(&self) -> usize {
        self.seen.len()
    }

    /// Returns the one admitted proposal identity, if present.
    #[must_use]
    pub const fn admitted(&self) -> Option<&ProposalId> {
        self.admitted.as_ref()
    }
}

fn valid_context(context: &ExpectedProposalContext) -> bool {
    [
        context.session_id.as_str(),
        context.task_id.as_str(),
        context.model_run_id.as_str(),
        context.context_packet_id.as_str(),
        context.repository_snapshot_id.as_str(),
        context.tool_catalog_id.as_str(),
        context.policy_id.as_str(),
        context.correlation_id.as_str(),
    ]
    .into_iter()
    .all(valid_identifier)
}

fn validate_candidate(candidate: &AgentProposal) -> Result<(), ProposalAdmissionError> {
    if candidate.schema_version != agentmage_kernel_contracts::CONTRACT_SCHEMA_VERSION
        || candidate.turn == 0
        || !valid_identifier(candidate.proposal_id.as_str())
        || !valid_sha256(&candidate.proposal_sha256)
    {
        return Err(ProposalAdmissionError::InvalidIdentity);
    }
    Ok(())
}

fn valid_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_IDENTIFIER_BYTES
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':'))
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

#[cfg(test)]
mod tests {
    use super::{ExpectedProposalContext, ProposalAdmissionError, ProposalAdmissionRegistry};
    use agentmage_kernel_contracts::{
        AgentProposal, CONTRACT_SCHEMA_VERSION, ContextPacketId, CorrelationId, ModelRunId,
        PolicyId, ProposalId, RepositorySnapshotId, SessionId, TaskId, ToolCatalogId, from_json,
        to_canonical_json,
    };

    fn context() -> ExpectedProposalContext {
        ExpectedProposalContext {
            session_id: SessionId::from_raw("session-0001"),
            task_id: TaskId::from_raw("task-0001"),
            turn: 7,
            model_run_id: ModelRunId::from_raw("model-run-0001"),
            context_packet_id: ContextPacketId::from_raw("context-0001"),
            repository_snapshot_id: RepositorySnapshotId::from_raw("snapshot-0001"),
            tool_catalog_id: ToolCatalogId::from_raw("catalog-0001"),
            policy_id: PolicyId::from_raw("policy-0001"),
            correlation_id: CorrelationId::from_raw("correlation-0001"),
        }
    }

    fn proposal() -> AgentProposal {
        let expected = context();
        AgentProposal {
            schema_version: CONTRACT_SCHEMA_VERSION,
            proposal_id: ProposalId::from_raw("proposal-0001"),
            session_id: expected.session_id,
            task_id: expected.task_id,
            turn: expected.turn,
            model_run_id: expected.model_run_id,
            context_packet_id: expected.context_packet_id,
            repository_snapshot_id: expected.repository_snapshot_id,
            tool_catalog_id: expected.tool_catalog_id,
            policy_id: expected.policy_id,
            correlation_id: expected.correlation_id,
            proposal_sha256: "a".repeat(64),
        }
    }

    #[test]
    fn task_12_2_1_2_exact_proposal_round_trips_and_binds_every_identity() {
        let candidate = proposal();
        let bytes = to_canonical_json(&candidate).expect("proposal serializes");
        assert_eq!(from_json::<AgentProposal>(&bytes), Ok(candidate.clone()));
        let admitted = ProposalAdmissionRegistry::new(context())
            .expect("context")
            .admit(&candidate)
            .expect("exact proposal");
        assert_eq!(admitted.proposal_id(), &candidate.proposal_id);
        assert_eq!(admitted.turn(), 7);
        assert_eq!(admitted.proposal_sha256(), "a".repeat(64));
    }

    #[test]
    fn task_12_2_1_2_malformed_partial_duplicate_unknown_and_oversized_are_inert() {
        let valid = to_canonical_json(&proposal()).expect("proposal serializes");
        let text = String::from_utf8(valid).expect("JSON is UTF-8");
        let cases = [
            "{".to_owned(),
            "{\"schema_version\":2}".to_owned(),
            text.replacen(
                "\"schema_version\":2",
                "\"schema_version\":2,\"schema_version\":2",
                1,
            ),
            text.replacen("{", "{\"unknown\":true,", 1),
        ];
        for input in cases {
            assert!(from_json::<AgentProposal>(input.as_bytes()).is_err());
        }
        let oversized = vec![b' '; agentmage_kernel_contracts::MAX_CONTRACT_JSON_BYTES + 1];
        assert!(from_json::<AgentProposal>(&oversized).is_err());
    }

    #[test]
    fn task_12_2_1_2_every_bound_identity_mismatch_fails_without_mutation() {
        let mut mutations = Vec::new();
        let mut candidate = proposal();
        candidate.session_id = SessionId::from_raw("session-other");
        mutations.push((candidate, ProposalAdmissionError::OwnershipMismatch));
        let mut candidate = proposal();
        candidate.task_id = TaskId::from_raw("task-other");
        mutations.push((candidate, ProposalAdmissionError::OwnershipMismatch));
        let mut candidate = proposal();
        candidate.model_run_id = ModelRunId::from_raw("model-run-other");
        mutations.push((candidate, ProposalAdmissionError::ModelContextMismatch));
        let mut candidate = proposal();
        candidate.context_packet_id = ContextPacketId::from_raw("context-other");
        mutations.push((candidate, ProposalAdmissionError::ModelContextMismatch));
        let mut candidate = proposal();
        candidate.repository_snapshot_id = RepositorySnapshotId::from_raw("snapshot-other");
        mutations.push((candidate, ProposalAdmissionError::EvaluationContextMismatch));
        let mut candidate = proposal();
        candidate.tool_catalog_id = ToolCatalogId::from_raw("catalog-other");
        mutations.push((candidate, ProposalAdmissionError::EvaluationContextMismatch));
        let mut candidate = proposal();
        candidate.policy_id = PolicyId::from_raw("policy-other");
        mutations.push((candidate, ProposalAdmissionError::EvaluationContextMismatch));
        let mut candidate = proposal();
        candidate.correlation_id = CorrelationId::from_raw("correlation-other");
        mutations.push((candidate, ProposalAdmissionError::CorrelationMismatch));
        for (candidate, expected) in mutations {
            let mut registry = ProposalAdmissionRegistry::new(context()).expect("context");
            assert_eq!(registry.admit(&candidate), Err(expected));
            assert_eq!(registry.seen_count(), 0);
            assert_eq!(registry.admitted(), None);
        }
    }

    #[test]
    fn task_12_2_1_2_stale_and_future_turns_are_inert() {
        for (turn, expected) in [
            (6, ProposalAdmissionError::StaleTurn),
            (8, ProposalAdmissionError::FutureTurn),
        ] {
            let mut candidate = proposal();
            candidate.turn = turn;
            let mut registry = ProposalAdmissionRegistry::new(context()).expect("context");
            assert_eq!(registry.admit(&candidate), Err(expected));
            assert_eq!(registry.seen_count(), 0);
        }
    }

    #[test]
    fn task_12_2_1_2_replay_and_competing_proposals_have_exact_results() {
        let first = proposal();
        let mut registry = ProposalAdmissionRegistry::new(context()).expect("context");
        registry.admit(&first).expect("first proposal");
        assert_eq!(
            registry.admit(&first),
            Err(ProposalAdmissionError::Replayed)
        );
        let mut competing = proposal();
        competing.proposal_id = ProposalId::from_raw("proposal-0002");
        competing.proposal_sha256 = "b".repeat(64);
        assert_eq!(
            registry.admit(&competing),
            Err(ProposalAdmissionError::CompetingProposal)
        );
        assert_eq!(registry.seen_count(), 2);
        assert_eq!(registry.admitted(), Some(&first.proposal_id));
    }

    #[test]
    fn task_12_2_1_2_invalid_context_and_candidate_never_enter_registry() {
        let mut invalid = context();
        invalid.turn = 0;
        assert!(matches!(
            ProposalAdmissionRegistry::new(invalid),
            Err(ProposalAdmissionError::InvalidIdentity)
        ));
        for mutation in [
            AgentProposal {
                proposal_id: ProposalId::from_raw(""),
                ..proposal()
            },
            AgentProposal {
                proposal_sha256: "A".repeat(64),
                ..proposal()
            },
            AgentProposal {
                turn: 0,
                ..proposal()
            },
        ] {
            let mut registry = ProposalAdmissionRegistry::new(context()).expect("context");
            assert_eq!(
                registry.admit(&mutation),
                Err(ProposalAdmissionError::InvalidIdentity)
            );
            assert_eq!(registry.seen_count(), 0);
        }
    }

    #[test]
    fn d027_s12_state_duplicate_and_competing_proposals_never_replace_admission() {
        let first = proposal();
        let mut registry = ProposalAdmissionRegistry::new(context()).expect("context");
        let admitted = registry.admit(&first).expect("first proposal");
        assert_eq!(admitted.proposal_id(), &first.proposal_id);

        assert_eq!(
            registry.admit(&first),
            Err(ProposalAdmissionError::Replayed)
        );
        let mut competing = proposal();
        competing.proposal_id = ProposalId::from_raw("proposal-d027-competing");
        competing.proposal_sha256 = "c".repeat(64);
        assert_eq!(
            registry.admit(&competing),
            Err(ProposalAdmissionError::CompetingProposal)
        );
        assert_eq!(registry.admitted(), Some(&first.proposal_id));
        assert_eq!(registry.seen_count(), 2);
    }
}
