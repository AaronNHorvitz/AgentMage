//! Deterministic verifier registry and opaque completion proof.

use std::collections::BTreeSet;

use agentmage_kernel_contracts::{
    AgentStateKind, PostconditionId, ProposalId, RepositorySnapshotId, TaskId, VerifierCandidate,
    VerifierDisposition, VerifierId, VerifierSource,
};

const MAX_POSTCONDITIONS: usize = 128;
const MAX_EVIDENCE_PER_POSTCONDITION: usize = 32;

/// Stable reason a verifier candidate cannot establish completion.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VerifierError {
    /// Registry configuration or candidate identity is malformed.
    InvalidIdentity,
    /// Candidate names another task, proposal, snapshot, or state revision.
    ContextMismatch,
    /// Candidate names another or unregistered verifier.
    VerifierMismatch,
    /// Candidate came from a non-deterministic or advisory source.
    NonDeterministicSource,
    /// Candidate does not claim one of the two success dispositions.
    NonSuccessDisposition,
    /// Postcondition set is missing, extra, duplicate, oversized, or reordered.
    PostconditionSetInvalid,
    /// A required postcondition failed or lacks current exact evidence.
    PostconditionUnverified,
}

/// Exact immutable context required for one verifier registry.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerifierContext {
    /// Exact registered verifier identity.
    pub verifier_id: VerifierId,
    /// Exact task identity.
    pub task_id: TaskId,
    /// Exact admitted proposal identity.
    pub proposal_id: ProposalId,
    /// Exact repository snapshot being verified.
    pub repository_snapshot_id: RepositorySnapshotId,
    /// Exact current state revision.
    pub state_revision: u64,
    /// Ordered required postcondition identities.
    pub postconditions: Vec<PostconditionId>,
}

/// Opaque proof that all exact current deterministic postconditions passed.
///
/// Fields are private and this type has no public constructor or deserializer. Only
/// [`VerifierRegistry::verify`] can create it.
#[derive(Debug, PartialEq, Eq)]
pub struct VerifiedCompletion {
    state_revision: u64,
    target: AgentStateKind,
}

impl VerifiedCompletion {
    pub(crate) const fn state_revision(&self) -> u64 {
        self.state_revision
    }

    pub(crate) const fn target(&self) -> AgentStateKind {
        self.target
    }
}

/// Frozen deterministic registry for one exact completion evaluation.
pub struct VerifierRegistry {
    context: VerifierContext,
}

impl VerifierRegistry {
    /// Creates a registry from one complete unique positive context.
    pub fn new(context: VerifierContext) -> Result<Self, VerifierError> {
        if context.state_revision == 0
            || context.postconditions.is_empty()
            || context.postconditions.len() > MAX_POSTCONDITIONS
            || !valid_id(context.verifier_id.as_str())
            || !valid_id(context.task_id.as_str())
            || !valid_id(context.proposal_id.as_str())
            || !valid_id(context.repository_snapshot_id.as_str())
            || context
                .postconditions
                .iter()
                .any(|item| !valid_id(item.as_str()))
            || context.postconditions.iter().collect::<BTreeSet<_>>().len()
                != context.postconditions.len()
        {
            return Err(VerifierError::InvalidIdentity);
        }
        Ok(Self { context })
    }

    /// Validates one untrusted candidate and mints an opaque completion proof.
    pub fn verify(
        &self,
        candidate: &VerifierCandidate,
    ) -> Result<VerifiedCompletion, VerifierError> {
        if candidate.schema_version != agentmage_kernel_contracts::CONTRACT_SCHEMA_VERSION
            || !valid_id(candidate.verifier_record_id.as_str())
        {
            return Err(VerifierError::InvalidIdentity);
        }
        if candidate.verifier_id != self.context.verifier_id {
            return Err(VerifierError::VerifierMismatch);
        }
        if candidate.task_id != self.context.task_id
            || candidate.proposal_id != self.context.proposal_id
            || candidate.repository_snapshot_id != self.context.repository_snapshot_id
            || candidate.state_revision != self.context.state_revision
        {
            return Err(VerifierError::ContextMismatch);
        }
        if candidate.source != VerifierSource::DeterministicPostcondition {
            return Err(VerifierError::NonDeterministicSource);
        }
        let target = match candidate.disposition {
            VerifierDisposition::Success => AgentStateKind::Success,
            VerifierDisposition::NoOp => AgentStateKind::NoOp,
            VerifierDisposition::Failed => return Err(VerifierError::NonSuccessDisposition),
        };
        if candidate.postconditions.len() != self.context.postconditions.len()
            || candidate
                .postconditions
                .iter()
                .map(|item| &item.postcondition_id)
                .ne(self.context.postconditions.iter())
        {
            return Err(VerifierError::PostconditionSetInvalid);
        }
        for result in &candidate.postconditions {
            if !result.passed
                || result.evidence.is_empty()
                || result.evidence.len() > MAX_EVIDENCE_PER_POSTCONDITION
                || result.evidence.iter().any(|evidence| {
                    evidence.observed_revision.as_deref()
                        != Some(self.context.repository_snapshot_id.as_str())
                        || !valid_sha256(&evidence.content_sha256)
                })
            {
                return Err(VerifierError::PostconditionUnverified);
            }
        }
        Ok(VerifiedCompletion {
            state_revision: candidate.state_revision,
            target,
        })
    }
}

fn valid_id(value: &str) -> bool {
    !value.is_empty() && value.len() <= 128
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

#[cfg(test)]
mod tests {
    use super::{VerifierContext, VerifierError, VerifierRegistry};
    use crate::agent_state::{AgentStateController, AgentStateError};
    use agentmage_kernel_contracts::{
        AgentStateKind, EvidenceId, EvidenceKind, EvidenceReference, PostconditionId,
        PostconditionResult, ProposalId, RepositorySnapshotId, TaskId, VerifierCandidate,
        VerifierDisposition, VerifierId, VerifierRecordId, VerifierSource, from_json,
        to_canonical_json,
    };

    const SNAPSHOT: &str = "snapshot-0001";
    const SHA256: &str = "1111111111111111111111111111111111111111111111111111111111111111";

    fn context() -> VerifierContext {
        VerifierContext {
            verifier_id: VerifierId::from_raw("verifier-0001"),
            task_id: TaskId::from_raw("task-0001"),
            proposal_id: ProposalId::from_raw("proposal-0001"),
            repository_snapshot_id: RepositorySnapshotId::from_raw(SNAPSHOT),
            state_revision: 6,
            postconditions: vec![
                PostconditionId::from_raw("postcondition-build"),
                PostconditionId::from_raw("postcondition-test"),
            ],
        }
    }

    fn evidence(id: &str) -> EvidenceReference {
        EvidenceReference {
            schema_version: agentmage_kernel_contracts::CONTRACT_SCHEMA_VERSION,
            evidence_id: EvidenceId::from_raw(id),
            kind: EvidenceKind::Validation,
            source_id: "registered-verifier".to_owned(),
            object_id: id.to_owned(),
            fragment: Some("result".to_owned()),
            content_sha256: SHA256.to_owned(),
            observed_revision: Some(SNAPSHOT.to_owned()),
        }
    }

    fn candidate(disposition: VerifierDisposition) -> VerifierCandidate {
        VerifierCandidate {
            schema_version: agentmage_kernel_contracts::CONTRACT_SCHEMA_VERSION,
            verifier_record_id: VerifierRecordId::from_raw("verifier-record-0001"),
            verifier_id: VerifierId::from_raw("verifier-0001"),
            task_id: TaskId::from_raw("task-0001"),
            proposal_id: ProposalId::from_raw("proposal-0001"),
            repository_snapshot_id: RepositorySnapshotId::from_raw(SNAPSHOT),
            state_revision: 6,
            source: VerifierSource::DeterministicPostcondition,
            disposition,
            postconditions: vec![
                PostconditionResult {
                    postcondition_id: PostconditionId::from_raw("postcondition-build"),
                    passed: true,
                    evidence: vec![evidence("evidence-build")],
                },
                PostconditionResult {
                    postcondition_id: PostconditionId::from_raw("postcondition-test"),
                    passed: true,
                    evidence: vec![evidence("evidence-test")],
                },
            ],
        }
    }

    fn verification_controller() -> AgentStateController {
        let mut controller = AgentStateController::new();
        for target in [
            AgentStateKind::Proposal,
            AgentStateKind::Validation,
            AgentStateKind::Approval,
            AgentStateKind::Execution,
            AgentStateKind::Verification,
        ] {
            controller.transition(target).expect("legal transition");
        }
        assert_eq!(controller.revision(), 6);
        controller
    }

    #[test]
    fn task_12_2_1_4_exact_typed_success_is_the_only_success_route() {
        let registry = VerifierRegistry::new(context()).expect("valid registry");
        let candidate = candidate(VerifierDisposition::Success);
        let bytes = to_canonical_json(&candidate).expect("candidate serializes");
        assert_eq!(
            from_json::<VerifierCandidate>(&bytes),
            Ok(candidate.clone())
        );
        let completion = registry.verify(&candidate).expect("verified completion");
        let mut controller = verification_controller();
        controller
            .complete(&completion)
            .expect("success transition");
        assert_eq!(controller.current(), AgentStateKind::Success);
    }

    #[test]
    fn task_12_2_1_4_no_op_requires_the_same_deterministic_verification() {
        let registry = VerifierRegistry::new(context()).expect("valid registry");
        let completion = registry
            .verify(&candidate(VerifierDisposition::NoOp))
            .expect("verified no-op");
        let mut controller = verification_controller();
        controller.complete(&completion).expect("no-op transition");
        assert_eq!(controller.current(), AgentStateKind::NoOp);
    }

    #[test]
    fn task_12_2_1_4_all_advisory_sources_are_inert_completion_evidence() {
        let registry = VerifierRegistry::new(context()).expect("valid registry");
        for source in [
            VerifierSource::ModelProse,
            VerifierSource::Confidence,
            VerifierSource::SelfReview,
            VerifierSource::ModelJudge,
            VerifierSource::Classifier,
        ] {
            let mut untrusted = candidate(VerifierDisposition::Success);
            untrusted.source = source;
            assert_eq!(
                registry.verify(&untrusted),
                Err(VerifierError::NonDeterministicSource)
            );
        }
    }

    #[test]
    fn task_12_2_1_4_incomplete_failed_stale_or_reordered_evidence_fails_closed() {
        let registry = VerifierRegistry::new(context()).expect("valid registry");
        let mut candidates = Vec::new();

        let mut failed_disposition = candidate(VerifierDisposition::Failed);
        failed_disposition.postconditions[0].passed = false;
        candidates.push((failed_disposition, VerifierError::NonSuccessDisposition));

        let mut missing = candidate(VerifierDisposition::Success);
        missing.postconditions.pop();
        candidates.push((missing, VerifierError::PostconditionSetInvalid));

        let mut reordered = candidate(VerifierDisposition::Success);
        reordered.postconditions.reverse();
        candidates.push((reordered, VerifierError::PostconditionSetInvalid));

        let mut failed = candidate(VerifierDisposition::Success);
        failed.postconditions[0].passed = false;
        candidates.push((failed, VerifierError::PostconditionUnverified));

        let mut empty = candidate(VerifierDisposition::Success);
        empty.postconditions[0].evidence.clear();
        candidates.push((empty, VerifierError::PostconditionUnverified));

        let mut stale = candidate(VerifierDisposition::Success);
        stale.postconditions[0].evidence[0].observed_revision = Some("snapshot-old".to_owned());
        candidates.push((stale, VerifierError::PostconditionUnverified));

        let mut malformed_digest = candidate(VerifierDisposition::Success);
        malformed_digest.postconditions[0].evidence[0].content_sha256 = "A".repeat(64);
        candidates.push((malformed_digest, VerifierError::PostconditionUnverified));

        for (untrusted, expected) in candidates {
            assert_eq!(registry.verify(&untrusted), Err(expected));
        }
    }

    #[test]
    fn task_12_2_1_4_every_context_identity_drift_is_rejected() {
        let registry = VerifierRegistry::new(context()).expect("valid registry");
        let mut candidates = Vec::new();

        let mut verifier = candidate(VerifierDisposition::Success);
        verifier.verifier_id = VerifierId::from_raw("verifier-other");
        candidates.push((verifier, VerifierError::VerifierMismatch));

        let mut task = candidate(VerifierDisposition::Success);
        task.task_id = TaskId::from_raw("task-other");
        candidates.push((task, VerifierError::ContextMismatch));

        let mut proposal = candidate(VerifierDisposition::Success);
        proposal.proposal_id = ProposalId::from_raw("proposal-other");
        candidates.push((proposal, VerifierError::ContextMismatch));

        let mut snapshot = candidate(VerifierDisposition::Success);
        snapshot.repository_snapshot_id = RepositorySnapshotId::from_raw("snapshot-other");
        candidates.push((snapshot, VerifierError::ContextMismatch));

        let mut revision = candidate(VerifierDisposition::Success);
        revision.state_revision = 5;
        candidates.push((revision, VerifierError::ContextMismatch));

        for (untrusted, expected) in candidates {
            assert_eq!(registry.verify(&untrusted), Err(expected));
        }
    }

    #[test]
    fn task_12_2_1_4_stale_or_reused_completion_proofs_cannot_complete() {
        let registry = VerifierRegistry::new(context()).expect("valid registry");
        let completion = registry
            .verify(&candidate(VerifierDisposition::Success))
            .expect("verified completion");

        let mut stale = verification_controller();
        stale
            .transition(AgentStateKind::Checkpoint)
            .expect("checkpoint transition");
        assert_eq!(
            stale.complete(&completion),
            Err(AgentStateError::VerifierStateMismatch)
        );

        let mut used = verification_controller();
        used.complete(&completion).expect("first completion");
        assert_eq!(
            used.complete(&completion),
            Err(AgentStateError::VerifierStateMismatch)
        );
    }

    #[test]
    fn d027_s12_state_false_completion_sources_produce_no_verified_success() {
        let registry = VerifierRegistry::new(context()).expect("valid registry");
        for source in [
            VerifierSource::ModelProse,
            VerifierSource::Confidence,
            VerifierSource::SelfReview,
            VerifierSource::ModelJudge,
            VerifierSource::Classifier,
        ] {
            for disposition in [VerifierDisposition::Success, VerifierDisposition::NoOp] {
                let mut untrusted = candidate(disposition);
                untrusted.source = source;
                assert_eq!(
                    registry.verify(&untrusted),
                    Err(VerifierError::NonDeterministicSource)
                );
            }
        }

        let mut incomplete = candidate(VerifierDisposition::Success);
        incomplete.postconditions.pop();
        assert_eq!(
            registry.verify(&incomplete),
            Err(VerifierError::PostconditionSetInvalid)
        );
        let mut failed = candidate(VerifierDisposition::Success);
        failed.postconditions[0].passed = false;
        assert_eq!(
            registry.verify(&failed),
            Err(VerifierError::PostconditionUnverified)
        );
    }

    #[test]
    fn d027_s12_restart_verifier_proof_loss_never_creates_false_success() {
        let registry = VerifierRegistry::new(context()).expect("valid registry");
        for (disposition, terminal) in [
            (VerifierDisposition::Success, AgentStateKind::Success),
            (VerifierDisposition::NoOp, AgentStateKind::NoOp),
        ] {
            let mut controller = verification_controller();
            {
                let _proof_before_crash = registry
                    .verify(&candidate(disposition))
                    .expect("deterministic proof");
                assert_eq!(controller.current(), AgentStateKind::Verification);
            }

            let before = controller.clone();
            assert_eq!(
                controller.transition(terminal),
                Err(AgentStateError::VerifierRequired)
            );
            assert_eq!(controller, before);

            let proof_after_restart = registry
                .verify(&candidate(disposition))
                .expect("fresh deterministic proof");
            controller
                .complete(&proof_after_restart)
                .expect("verified completion");
            assert_eq!(controller.current(), terminal);
            let completed = controller.clone();
            assert_eq!(
                controller.transition(AgentStateKind::Proposal),
                Err(AgentStateError::IllegalTransition)
            );
            assert_eq!(controller, completed);
        }
    }
}
