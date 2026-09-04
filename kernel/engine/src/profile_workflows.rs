//! Deterministic, authority-free local workflow definitions for specialist profiles.

use std::collections::{BTreeMap, BTreeSet};

/// Closed local workflow identity.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ProfileWorkflowKind {
    /// Idea to bounded work proposal.
    IdeaToWork,
    /// Bug to verified change proposal.
    BugToChange,
    /// Independent pull-request review fan-out.
    PullRequestReview,
    /// Roadmap and sprint reconciliation.
    RoadmapReconciliation,
}

/// Effect class requested by a workflow node.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProfileNodeEffect {
    /// Read-only synthesis may run in bounded parallel waves.
    ReadOnly,
    /// Local writes require exclusive sequential ownership.
    ControlledWrite,
}

/// One exact declarative workflow node.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProfileWorkflowNode {
    /// Stable node identity.
    pub node_id: String,
    /// Exact Decision 0041 profile.
    pub profile_id: String,
    /// Dependency nodes.
    pub dependencies: Vec<String>,
    /// Exact work-packet digest.
    pub work_packet_sha256: String,
    /// Exact intersected authority digest.
    pub authority_sha256: String,
    /// Effect class.
    pub effect: ProfileNodeEffect,
    /// Exclusive owned paths; empty for read-only work.
    pub owned_paths: BTreeSet<String>,
}

/// Complete local workflow definition.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProfileWorkflowDefinition {
    /// Workflow identity.
    pub kind: ProfileWorkflowKind,
    /// Exact ordered node list.
    pub nodes: Vec<ProfileWorkflowNode>,
    /// Maximum parallel read-only nodes.
    pub max_parallel_readers: u8,
    /// Every output requires explicit parent review.
    pub parent_review_required: bool,
    /// Provider effects are unavailable.
    pub provider_effect_count: u8,
}

/// One isolated review finding.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ProfileReviewFinding {
    /// Reviewer profile.
    pub reviewer_profile_id: String,
    /// Stable finding identity.
    pub finding_id: String,
    /// Severity.
    pub severity: String,
    /// Evidence digest.
    pub evidence_sha256: String,
    /// Conclusion text digest.
    pub conclusion_sha256: String,
}

/// Evidence-preserving review synthesis; never an approval.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProfileReviewSynthesis {
    /// Every distinct finding in stable order.
    pub findings: Vec<ProfileReviewFinding>,
    /// Finding identities with conflicting conclusions.
    pub dissent: BTreeSet<String>,
    /// Unresolved finding identities.
    pub unresolved: BTreeSet<String>,
    /// Human or separately authorized parent review remains required.
    pub parent_review_required: bool,
    /// Synthesis has no approval authority.
    pub approved: bool,
}

/// Stable graph validation failure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProfileWorkflowError {
    /// Shape, identity, digest, bound, or dependency invalid.
    InvalidDefinition,
    /// Writable ownership overlaps or permits parallel writes.
    OwnershipConflict,
    /// A workflow attempted a provider effect or self-approval.
    AuthorityViolation,
}

/// Returns the four canonical Decision 0041 local graphs.
#[must_use]
pub fn canonical_profile_workflows() -> Vec<ProfileWorkflowDefinition> {
    vec![
        workflow(
            ProfileWorkflowKind::IdeaToWork,
            &["AG-25", "AG-26", "AG-28", "AG-05", "AG-27", "AG-30"],
            &[],
        ),
        workflow(
            ProfileWorkflowKind::BugToChange,
            &[
                "AG-03", "AG-06", "AG-42", "AG-07", "AG-08", "AG-09", "AG-43", "AG-14",
            ],
            &[1, 3, 4, 7],
        ),
        workflow(
            ProfileWorkflowKind::PullRequestReview,
            &[
                "AG-35", "AG-36", "AG-37", "AG-38", "AG-39", "AG-40", "AG-41", "AG-11",
            ],
            &[],
        ),
        workflow(
            ProfileWorkflowKind::RoadmapReconciliation,
            &["AG-29", "AG-30", "AG-31", "AG-32", "AG-33", "AG-34"],
            &[],
        ),
    ]
}

/// Validates deterministic ordering, bounded reads, sequential writes, and zero provider effects.
pub fn validate_profile_workflow(
    definition: &ProfileWorkflowDefinition,
) -> Result<(), ProfileWorkflowError> {
    if definition.nodes.is_empty()
        || definition.max_parallel_readers == 0
        || definition.max_parallel_readers > 8
        || !definition.parent_review_required
        || definition.provider_effect_count != 0
    {
        return Err(ProfileWorkflowError::AuthorityViolation);
    }
    let ids = definition
        .nodes
        .iter()
        .map(|node| node.node_id.as_str())
        .collect::<BTreeSet<_>>();
    if ids.len() != definition.nodes.len() {
        return Err(ProfileWorkflowError::InvalidDefinition);
    }
    let mut prior = BTreeSet::new();
    let mut owned = BTreeSet::new();
    for node in &definition.nodes {
        if !valid_id(&node.node_id)
            || !valid_profile(&node.profile_id)
            || !valid_sha256(&node.work_packet_sha256)
            || !valid_sha256(&node.authority_sha256)
            || node
                .dependencies
                .iter()
                .any(|dependency| !prior.contains(dependency))
        {
            return Err(ProfileWorkflowError::InvalidDefinition);
        }
        match node.effect {
            ProfileNodeEffect::ReadOnly if !node.owned_paths.is_empty() => {
                return Err(ProfileWorkflowError::OwnershipConflict);
            }
            ProfileNodeEffect::ControlledWrite
                if node.owned_paths.is_empty() || !owned.is_disjoint(&node.owned_paths) =>
            {
                return Err(ProfileWorkflowError::OwnershipConflict);
            }
            ProfileNodeEffect::ControlledWrite => owned.extend(node.owned_paths.iter().cloned()),
            ProfileNodeEffect::ReadOnly => {}
        }
        prior.insert(node.node_id.clone());
    }
    Ok(())
}

/// Synthesizes independent findings without hiding dissent or minting approval.
pub fn synthesize_profile_review(
    findings: Vec<ProfileReviewFinding>,
) -> Result<ProfileReviewSynthesis, ProfileWorkflowError> {
    if findings.is_empty()
        || findings.iter().any(|finding| {
            !valid_profile(&finding.reviewer_profile_id)
                || !valid_id(&finding.finding_id)
                || !valid_id(&finding.severity)
                || !valid_sha256(&finding.evidence_sha256)
                || !valid_sha256(&finding.conclusion_sha256)
        })
    {
        return Err(ProfileWorkflowError::InvalidDefinition);
    }
    let mut by_id: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for finding in &findings {
        by_id
            .entry(finding.finding_id.clone())
            .or_default()
            .insert(finding.conclusion_sha256.clone());
    }
    let dissent = by_id
        .iter()
        .filter(|(_, conclusions)| conclusions.len() > 1)
        .map(|(id, _)| id.clone())
        .collect();
    let unresolved = by_id.keys().cloned().collect();
    let mut findings = findings;
    findings.sort();
    findings.dedup();
    Ok(ProfileReviewSynthesis {
        findings,
        dissent,
        unresolved,
        parent_review_required: true,
        approved: false,
    })
}

fn workflow(
    kind: ProfileWorkflowKind,
    profiles: &[&str],
    writable_indices: &[usize],
) -> ProfileWorkflowDefinition {
    let nodes = profiles
        .iter()
        .enumerate()
        .map(|(index, profile)| ProfileWorkflowNode {
            node_id: format!("node-{index:02}"),
            profile_id: (*profile).to_owned(),
            dependencies: (index > 0)
                .then(|| format!("node-{:02}", index - 1))
                .into_iter()
                .collect(),
            work_packet_sha256: format!("{:064x}", index + 1),
            authority_sha256: format!("{:064x}", index + 101),
            effect: if writable_indices.contains(&index) {
                ProfileNodeEffect::ControlledWrite
            } else {
                ProfileNodeEffect::ReadOnly
            },
            owned_paths: writable_indices
                .contains(&index)
                .then(|| format!("worktree/node-{index:02}"))
                .into_iter()
                .collect(),
        })
        .collect();
    ProfileWorkflowDefinition {
        kind,
        nodes,
        max_parallel_readers: 5,
        parent_review_required: true,
        provider_effect_count: 0,
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
    #[test]
    fn four_canonical_graphs_are_valid_and_effect_free() {
        let graphs = canonical_profile_workflows();
        assert_eq!(graphs.len(), 4);
        assert!(
            graphs
                .iter()
                .all(|graph| validate_profile_workflow(graph).is_ok())
        );
        assert!(graphs.iter().all(|graph| graph.provider_effect_count == 0));
    }
    #[test]
    fn exact_profile_sequences_are_retained() {
        let graphs = canonical_profile_workflows();
        assert_eq!(
            graphs[0]
                .nodes
                .iter()
                .map(|node| node.profile_id.as_str())
                .collect::<Vec<_>>(),
            vec!["AG-25", "AG-26", "AG-28", "AG-05", "AG-27", "AG-30"]
        );
        assert_eq!(graphs[2].nodes.first().unwrap().profile_id, "AG-35");
        assert_eq!(graphs[2].nodes.last().unwrap().profile_id, "AG-11");
    }
    #[test]
    fn writable_collisions_and_provider_effects_fail_closed() {
        let mut graph = canonical_profile_workflows().remove(1);
        graph.nodes[4].owned_paths = graph.nodes[3].owned_paths.clone();
        assert_eq!(
            validate_profile_workflow(&graph),
            Err(ProfileWorkflowError::OwnershipConflict)
        );
        graph = canonical_profile_workflows().remove(0);
        graph.provider_effect_count = 1;
        assert_eq!(
            validate_profile_workflow(&graph),
            Err(ProfileWorkflowError::AuthorityViolation)
        );
    }
    #[test]
    fn conflicting_independent_findings_preserve_dissent_without_approval() {
        let findings = ["a", "b"]
            .into_iter()
            .enumerate()
            .map(|(index, conclusion)| ProfileReviewFinding {
                reviewer_profile_id: format!("AG-{:02}", index + 36),
                finding_id: "finding-one".to_owned(),
                severity: "high".to_owned(),
                evidence_sha256: "a".repeat(64),
                conclusion_sha256: conclusion.repeat(64),
            })
            .collect();
        let result = synthesize_profile_review(findings).unwrap();
        assert_eq!(result.findings.len(), 2);
        assert!(result.dissent.contains("finding-one"));
        assert!(result.unresolved.contains("finding-one"));
        assert!(result.parent_review_required);
        assert!(!result.approved);
    }
}
