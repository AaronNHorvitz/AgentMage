//! Verified, authority-free planning identity for one local coding session.

use std::fmt::Write;

use agentmage_capability_repository_map::{
    ChangeIntentRecord, ChangeIntentStatus, ChangePlanRecord, ChangePlanStatus,
    DeepRepositoryIndex, MinimalChangeImpactReport, RepositoryMap, verify_change_intent,
    verify_change_plan, verify_deep_repository_index, verify_minimal_change_impact,
    verify_repository_map,
};
use agentmage_kernel_contracts::{WorkspaceId, WorkspacePath};
use serde::Serialize;
use sha2::{Digest, Sha256};

/// Stable content-free refusal while binding verified planning evidence.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CodingPlanBindingError {
    /// A source record is stale, malformed, incomplete, or not review-ready.
    SourceDenied,
    /// The resulting repository, path, validation, or digest relationship is invalid.
    BindingDenied,
}

impl CodingPlanBindingError {
    /// Returns one stable content-free diagnostic code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::SourceDenied => "runtime.coding-plan.source-denied",
            Self::BindingDenied => "runtime.coding-plan.binding-denied",
        }
    }
}

/// Immutable descriptive plan identity admitted by one coding profile.
///
/// This record deliberately carries no capability, grant, executable callback, or effect handle.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CodingPlanBinding {
    schema_version: u16,
    plan_id: String,
    workspace_id: WorkspaceId,
    repository_map_sha256: String,
    repository_commit_id: String,
    worktree_sha256: String,
    deep_index_sha256: String,
    intent_sha256: String,
    impact_sha256: String,
    plan_sha256: String,
    proposed_paths: Vec<WorkspacePath>,
    planned_validation_ids: Vec<String>,
    binding_sha256: String,
}

impl CodingPlanBinding {
    /// Verifies the complete Sprint 44 record chain and seals its coding-session identity.
    pub fn from_verified_records(
        map: &RepositoryMap,
        index: &DeepRepositoryIndex,
        intent: &ChangeIntentRecord,
        impact: &MinimalChangeImpactReport,
        plan: &ChangePlanRecord,
    ) -> Result<Self, CodingPlanBindingError> {
        if !verify_repository_map(map)
            || !verify_deep_repository_index(map, index)
            || !verify_change_intent(index, intent)
            || !verify_minimal_change_impact(index, intent, impact)
            || !verify_change_plan(index, intent, impact, plan)
            || intent.status != ChangeIntentStatus::ReadyForPlanning
            || !impact.complete
            || plan.status != ChangePlanStatus::ReadyForImplementationReview
            || intent.mutation_authority
            || impact.mutation_authority
            || plan.mutation_authority
        {
            return Err(CodingPlanBindingError::SourceDenied);
        }

        let mut proposed_paths = impact.proposed_paths.clone();
        proposed_paths.sort();
        proposed_paths.dedup();
        let mut planned_validation_ids = plan
            .input
            .validations
            .iter()
            .map(|validation| validation.validation_id.clone())
            .collect::<Vec<_>>();
        planned_validation_ids.sort();
        planned_validation_ids.dedup();
        if proposed_paths.is_empty()
            || planned_validation_ids.is_empty()
            || proposed_paths
                .iter()
                .any(|path| path.workspace_id() != &map.workspace_id)
        {
            return Err(CodingPlanBindingError::BindingDenied);
        }

        let mut binding = Self {
            schema_version: 1,
            plan_id: plan.input.plan_id.clone(),
            workspace_id: map.workspace_id.clone(),
            repository_map_sha256: map.map_sha256.clone(),
            repository_commit_id: map.commit_id.clone(),
            worktree_sha256: map.worktree_sha256.clone(),
            deep_index_sha256: index.index_sha256.clone(),
            intent_sha256: intent.intent_sha256.clone(),
            impact_sha256: impact.impact_sha256.clone(),
            plan_sha256: plan.plan_sha256.clone(),
            proposed_paths,
            planned_validation_ids,
            binding_sha256: String::new(),
        };
        binding.binding_sha256 = binding_digest(&binding)?;
        binding
            .verify()
            .then_some(binding)
            .ok_or(CodingPlanBindingError::BindingDenied)
    }

    /// Verifies the sealed planning identity without granting operation authority.
    #[must_use]
    pub fn verify(&self) -> bool {
        self.schema_version == 1
            && valid_identifier(&self.plan_id)
            && is_sha256(&self.repository_map_sha256)
            && matches!(self.repository_commit_id.len(), 40 | 64)
            && self
                .repository_commit_id
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit())
            && is_sha256(&self.worktree_sha256)
            && is_sha256(&self.deep_index_sha256)
            && is_sha256(&self.intent_sha256)
            && is_sha256(&self.impact_sha256)
            && is_sha256(&self.plan_sha256)
            && !self.proposed_paths.is_empty()
            && self.proposed_paths.windows(2).all(|pair| pair[0] < pair[1])
            && self
                .proposed_paths
                .iter()
                .all(|path| path.workspace_id() == &self.workspace_id)
            && !self.planned_validation_ids.is_empty()
            && self
                .planned_validation_ids
                .windows(2)
                .all(|pair| pair[0] < pair[1])
            && self
                .planned_validation_ids
                .iter()
                .all(|identity| valid_identifier(identity))
            && is_sha256(&self.binding_sha256)
            && binding_digest(self).is_ok_and(|digest| digest == self.binding_sha256)
    }

    /// Returns the exact descriptive plan identity attached to the work packet.
    #[must_use]
    pub fn plan_id(&self) -> &str {
        &self.plan_id
    }

    /// Returns the exact source repository-map digest.
    #[must_use]
    pub fn repository_map_sha256(&self) -> &str {
        &self.repository_map_sha256
    }

    /// Returns the exact workspace identity represented by the verified map.
    #[must_use]
    pub const fn workspace_id(&self) -> &WorkspaceId {
        &self.workspace_id
    }

    /// Returns the exact Git object represented by the verified map.
    #[must_use]
    pub fn repository_commit_id(&self) -> &str {
        &self.repository_commit_id
    }

    /// Returns the exact owned-worktree path identity represented by the verified map.
    #[must_use]
    pub fn worktree_sha256(&self) -> &str {
        &self.worktree_sha256
    }

    /// Returns the exact normalized change-intent digest.
    #[must_use]
    pub fn intent_sha256(&self) -> &str {
        &self.intent_sha256
    }

    /// Returns the exact review-ready change-plan digest.
    #[must_use]
    pub fn plan_sha256(&self) -> &str {
        &self.plan_sha256
    }

    /// Returns the canonical digest of the complete planning binding.
    #[must_use]
    pub fn binding_sha256(&self) -> &str {
        &self.binding_sha256
    }

    /// Reports whether one write proposal cites this exact verified plan chain.
    #[must_use]
    pub fn matches_write_proposal(&self, intent_sha256: &str, plan_sha256: &str) -> bool {
        self.verify() && self.intent_sha256 == intent_sha256 && self.plan_sha256 == plan_sha256
    }

    /// Reports whether one targeted validation was declared by the verified plan.
    #[must_use]
    pub fn admits_validation(&self, validation_id: &str) -> bool {
        self.verify()
            && self
                .planned_validation_ids
                .binary_search_by(|candidate| candidate.as_str().cmp(validation_id))
                .is_ok()
    }
}

fn binding_digest(binding: &CodingPlanBinding) -> Result<String, CodingPlanBindingError> {
    #[derive(Serialize)]
    struct Material<'a> {
        schema_version: u16,
        plan_id: &'a str,
        workspace_id: &'a WorkspaceId,
        repository_map_sha256: &'a str,
        repository_commit_id: &'a str,
        worktree_sha256: &'a str,
        deep_index_sha256: &'a str,
        intent_sha256: &'a str,
        impact_sha256: &'a str,
        plan_sha256: &'a str,
        proposed_paths: &'a [WorkspacePath],
        planned_validation_ids: &'a [String],
    }

    serde_json::to_vec(&Material {
        schema_version: binding.schema_version,
        plan_id: &binding.plan_id,
        workspace_id: &binding.workspace_id,
        repository_map_sha256: &binding.repository_map_sha256,
        repository_commit_id: &binding.repository_commit_id,
        worktree_sha256: &binding.worktree_sha256,
        deep_index_sha256: &binding.deep_index_sha256,
        intent_sha256: &binding.intent_sha256,
        impact_sha256: &binding.impact_sha256,
        plan_sha256: &binding.plan_sha256,
        proposed_paths: &binding.proposed_paths,
        planned_validation_ids: &binding.planned_validation_ids,
    })
    .map(|bytes| sha256(&bytes))
    .map_err(|_| CodingPlanBindingError::BindingDenied)
}

fn valid_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value.bytes().enumerate().all(|(index, byte)| {
            byte.is_ascii_alphanumeric() || (index > 0 && matches!(byte, b'.' | b'_' | b':' | b'-'))
        })
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}

fn sha256(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(64);
    for byte in Sha256::digest(bytes) {
        write!(&mut output, "{byte:02x}").expect("writing to String cannot fail");
    }
    output
}

#[cfg(test)]
pub(crate) fn fixture_coding_plan_binding(map: &RepositoryMap) -> CodingPlanBinding {
    use agentmage_capability_repository_map::{
        ChangeAlternativeDimension, ChangeAlternativeOption, ChangeAlternativeSet,
        ChangeImpactSurfaceKind, ChangeIntentInput, ChangePlanInput, ChangeRiskDomain,
        ChangeSurfaceExclusion, ChangeWorkKind, HypothesisRecord, PlannedValidation,
        PlannedValidationKind, RegressionTestDisposition, RegressionTestPlan, RepositoryFactKind,
        build_change_plan, build_deep_repository_index, build_minimal_change_impact,
        normalize_change_intent,
    };

    let index = build_deep_repository_index(map, Vec::new(), Vec::new()).expect("fixture index");
    let target = index
        .facts
        .iter()
        .find(|fact| fact.kind == RepositoryFactKind::Definition)
        .or_else(|| {
            index
                .facts
                .iter()
                .find(|fact| fact.kind == RepositoryFactKind::Language)
        })
        .expect("fixture source fact")
        .fact_id
        .clone();
    let intent = normalize_change_intent(
        &index,
        ChangeIntentInput {
            intent_id: "intent-coding-fixture".to_owned(),
            requested_behavior: "Produce the requested bounded fixture behavior".to_owned(),
            current_behavior: "The cited fixture retains its original behavior".to_owned(),
            current_behavior_fact_ids: vec![target.clone()],
            target_fact_ids: vec![target],
            users: vec!["fixture callers".to_owned()],
            acceptance_checks: vec!["The focused fixture passes".to_owned()],
            exclusions: vec!["No unrelated repository changes".to_owned()],
            risks: vec![ChangeRiskDomain::Reliability],
            rollback: "Restore the exact prior bytes through a fresh grant".to_owned(),
            rollback_reversible: true,
            surface_exclusions: ChangeImpactSurfaceKind::ALL
                .into_iter()
                .filter(|kind| *kind != ChangeImpactSurfaceKind::Files)
                .map(|surface| ChangeSurfaceExclusion {
                    surface,
                    rationale: "The bounded fixture provides no evidence for this surface"
                        .to_owned(),
                })
                .collect(),
            clarifications: Vec::new(),
        },
    )
    .expect("fixture intent");
    let impact = build_minimal_change_impact(&index, &intent).expect("fixture impact");
    let alternatives = ChangeAlternativeDimension::ALL
        .into_iter()
        .map(|dimension| ChangeAlternativeSet {
            dimension,
            options: vec![
                ChangeAlternativeOption {
                    option_id: "option-minimal".to_owned(),
                    description: "Change only the exact bounded fixture".to_owned(),
                    tradeoffs: vec!["Keeps the evidence scope bounded".to_owned()],
                    evidence_fact_ids: Vec::new(),
                    irreversible: false,
                },
                ChangeAlternativeOption {
                    option_id: "option-wide".to_owned(),
                    description: "Broaden the change beyond the cited fixture".to_owned(),
                    tradeoffs: vec!["Would require additional evidence".to_owned()],
                    evidence_fact_ids: Vec::new(),
                    irreversible: dimension == ChangeAlternativeDimension::Irreversibility,
                },
            ],
            selected_option_id: "option-minimal".to_owned(),
            decision_id: format!("decision-{dimension:?}").to_ascii_lowercase(),
            rationale: "The minimal option is the only evidence-backed selection".to_owned(),
            alternative_sha256: String::new(),
        })
        .collect();
    let plan = build_change_plan(
        &index,
        &intent,
        &impact,
        ChangePlanInput {
            plan_id: "coding-plan-0001".to_owned(),
            work_kind: ChangeWorkKind::Feature,
            reproduction: None,
            regression_test: RegressionTestPlan {
                disposition: RegressionTestDisposition::NotDefect,
                test_fact_ids: Vec::new(),
                rationale: Some("The fixture represents planned behavior".to_owned()),
                regression_sha256: String::new(),
            },
            hypotheses: HypothesisRecord {
                hypotheses: Vec::new(),
                selected_hypothesis_id: None,
                state_trace_fact_ids: Vec::new(),
                correlated_log_sha256s: Vec::new(),
                hypothesis_sha256: String::new(),
            },
            alternatives,
            validations: vec![PlannedValidation {
                validation_id: "validation-unit".to_owned(),
                kind: PlannedValidationKind::Unit,
                description: "Run the exact focused fixture".to_owned(),
                acceptance_checks: intent.input.acceptance_checks.clone(),
                separate_grant_required: true,
                write_authority: false,
            }],
        },
    )
    .expect("fixture plan");
    CodingPlanBinding::from_verified_records(map, &index, &intent, &impact, &plan)
        .expect("fixture coding plan binding")
}

#[cfg(test)]
mod tests {
    use agentmage_capability_repository_map::{
        GitTrackedState, RepositoryFileInput, RepositoryMapInput, build_repository_map,
    };

    use super::*;

    fn map() -> RepositoryMap {
        let content = b"pub fn runtime_fixture() {}\n";
        build_repository_map(RepositoryMapInput {
            workspace_id: WorkspaceId::from_raw("workspace-coding"),
            repository_sha256: "a".repeat(64),
            worktree_sha256: "b".repeat(64),
            branch: Some("refs/heads/agentmage/tasks/task-coding-0001".to_owned()),
            commit_id: "1".repeat(40),
            policy_sha256: "d".repeat(64),
            freshness_sha256: "e".repeat(64),
            files: vec![RepositoryFileInput {
                path: vec!["src".to_owned(), "lib.rs".to_owned()],
                size_bytes: content.len() as u64,
                content_sha256: sha256(content),
                content: Some(content.to_vec()),
                git_state: GitTrackedState::TrackedClean,
                policy_excluded: false,
                generated: false,
                vendored: false,
            }],
        })
        .expect("fixture map")
    }

    #[test]
    fn story_48_2_plan_binding_accepts_only_the_verified_record_chain() {
        let binding = fixture_coding_plan_binding(&map());
        assert!(binding.verify());
        assert!(binding.matches_write_proposal(binding.intent_sha256(), binding.plan_sha256()));
        assert!(!binding.matches_write_proposal(&"f".repeat(64), binding.plan_sha256()));
        assert!(binding.admits_validation("validation-unit"));
        assert!(!binding.admits_validation("validation-unplanned"));
    }
}
