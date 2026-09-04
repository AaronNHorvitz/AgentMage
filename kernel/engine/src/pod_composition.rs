//! Non-enabling composition of qualified roles and capabilities into bounded pods.

use std::collections::BTreeSet;

/// One exact qualified pod worker binding.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PodWorkerBinding {
    /// Exact worker identity.
    pub worker_id: String,
    /// Exact qualified `AG-NN` role.
    pub profile_id: String,
    /// Exact admitted capability identity.
    pub capability_id: String,
    /// Exact work packet.
    pub work_packet_sha256: String,
    /// Exact grant intersection.
    pub grant_sha256: String,
    /// Independent budget.
    pub budget_sha256: String,
    /// Isolated context.
    pub context_sha256: String,
    /// Dedicated worktree.
    pub worktree_sha256: String,
    /// Required evidence contract.
    pub evidence_sha256: String,
    /// Role qualification passed.
    pub profile_qualified: bool,
    /// Capability admission passed.
    pub capability_admitted: bool,
    /// Worker is independent from its reviewer.
    pub reviewer_id: String,
}

/// Complete non-enabling pod definition.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PodComposition {
    /// Stable pod identity.
    pub pod_id: String,
    /// Exact single-agent reliability evidence.
    pub single_agent_evidence_sha256: String,
    /// Exact resource evidence.
    pub resource_evidence_sha256: String,
    /// Exact verifier evidence.
    pub verifier_evidence_sha256: String,
    /// Deterministic DAG workers.
    pub workers: Vec<PodWorkerBinding>,
    /// A complete single-agent mode remains available.
    pub single_agent_mode_preserved: bool,
    /// Registration does not enable the pod.
    pub enabled: bool,
    /// Integration is serialized.
    pub serialized_integration: bool,
    /// Publication requires a separately owned exact authorization.
    pub publication_authority_present: bool,
}

/// Closed recovery disposition.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PodRecoveryDisposition {
    /// Safe accepted artifact remains immutable.
    PreserveAccepted,
    /// In-flight state is blocked pending reconciliation.
    BlockUncertain,
    /// Fully clean unstarted work may be rescheduled with a fresh identity.
    FreshReschedule,
}

/// Stable pod composition refusal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PodCompositionError {
    /// Identity, digest, qualification, or bound invalid.
    InvalidComposition,
    /// Worker/reviewer identity, worktree, or grant isolation failed.
    IsolationConflict,
    /// Composition attempted enablement, publication, or authority pooling.
    AuthorityViolation,
}

/// Validates one bounded pod without enabling or executing it.
pub fn validate_pod_composition(value: &PodComposition) -> Result<(), PodCompositionError> {
    if !valid_id(&value.pod_id)
        || !valid_sha256(&value.single_agent_evidence_sha256)
        || !valid_sha256(&value.resource_evidence_sha256)
        || !valid_sha256(&value.verifier_evidence_sha256)
        || value.workers.is_empty()
        || value.workers.len() > 5
        || !value.single_agent_mode_preserved
        || value.enabled
        || !value.serialized_integration
        || value.publication_authority_present
    {
        return Err(PodCompositionError::AuthorityViolation);
    }
    let mut worker_ids = BTreeSet::new();
    let mut worktrees = BTreeSet::new();
    let mut grants = BTreeSet::new();
    for worker in &value.workers {
        let hashes = [
            &worker.work_packet_sha256,
            &worker.grant_sha256,
            &worker.budget_sha256,
            &worker.context_sha256,
            &worker.worktree_sha256,
            &worker.evidence_sha256,
        ];
        if !valid_id(&worker.worker_id)
            || !valid_profile(&worker.profile_id)
            || !valid_id(&worker.capability_id)
            || !valid_id(&worker.reviewer_id)
            || worker.worker_id == worker.reviewer_id
            || !worker.profile_qualified
            || !worker.capability_admitted
            || !hashes.into_iter().all(|hash| valid_sha256(hash))
        {
            return Err(PodCompositionError::InvalidComposition);
        }
        if !worker_ids.insert(&worker.worker_id)
            || !worktrees.insert(&worker.worktree_sha256)
            || !grants.insert(&worker.grant_sha256)
        {
            return Err(PodCompositionError::IsolationConflict);
        }
    }
    Ok(())
}

/// Classifies recovery without replaying uncertain effects.
#[must_use]
pub const fn classify_pod_recovery(
    started: bool,
    accepted: bool,
    effect_uncertain: bool,
) -> PodRecoveryDisposition {
    if accepted {
        PodRecoveryDisposition::PreserveAccepted
    } else if started || effect_uncertain {
        PodRecoveryDisposition::BlockUncertain
    } else {
        PodRecoveryDisposition::FreshReschedule
    }
}

fn valid_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
}
fn valid_profile(value: &str) -> bool {
    value.len() == 5
        && value.starts_with("AG-")
        && value[3..]
            .parse::<u8>()
            .is_ok_and(|number| (1..=49).contains(&number))
}
fn valid_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn worker(id: &str, profile: &str, seed: char) -> PodWorkerBinding {
        PodWorkerBinding {
            worker_id: id.to_owned(),
            profile_id: profile.to_owned(),
            capability_id: "agentmage.repository-inspection".to_owned(),
            work_packet_sha256: seed.to_string().repeat(64),
            grant_sha256: ((seed as u8 + 1) as char).to_string().repeat(64),
            budget_sha256: "b".repeat(64),
            context_sha256: "c".repeat(64),
            worktree_sha256: ((seed as u8 + 2) as char).to_string().repeat(64),
            evidence_sha256: "e".repeat(64),
            profile_qualified: true,
            capability_admitted: true,
            reviewer_id: format!("reviewer-{id}"),
        }
    }
    fn pod() -> PodComposition {
        PodComposition {
            pod_id: "pod-one".to_owned(),
            single_agent_evidence_sha256: "1".repeat(64),
            resource_evidence_sha256: "2".repeat(64),
            verifier_evidence_sha256: "3".repeat(64),
            workers: vec![worker("one", "AG-07", '4'), worker("two", "AG-10", '7')],
            single_agent_mode_preserved: true,
            enabled: false,
            serialized_integration: true,
            publication_authority_present: false,
        }
    }
    #[test]
    fn qualified_pod_is_valid_but_disabled() {
        let value = pod();
        assert_eq!(validate_pod_composition(&value), Ok(()));
        assert!(!value.enabled);
    }
    #[test]
    fn collusion_collision_and_authority_attempts_fail_closed() {
        let mut value = pod();
        value.workers[1].reviewer_id = value.workers[1].worker_id.clone();
        assert_eq!(
            validate_pod_composition(&value),
            Err(PodCompositionError::InvalidComposition)
        );
        value = pod();
        value.workers[1].grant_sha256 = value.workers[0].grant_sha256.clone();
        assert_eq!(
            validate_pod_composition(&value),
            Err(PodCompositionError::IsolationConflict)
        );
        value = pod();
        value.enabled = true;
        assert_eq!(
            validate_pod_composition(&value),
            Err(PodCompositionError::AuthorityViolation)
        );
    }
    #[test]
    fn recovery_never_replays_uncertain_work() {
        assert_eq!(
            classify_pod_recovery(false, false, false),
            PodRecoveryDisposition::FreshReschedule
        );
        assert_eq!(
            classify_pod_recovery(true, false, false),
            PodRecoveryDisposition::BlockUncertain
        );
        assert_eq!(
            classify_pod_recovery(false, false, true),
            PodRecoveryDisposition::BlockUncertain
        );
        assert_eq!(
            classify_pod_recovery(true, true, true),
            PodRecoveryDisposition::PreserveAccepted
        );
    }
}
