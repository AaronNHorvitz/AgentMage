//! Source-bound change intent and minimal impact analysis without mutation authority.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write;

use agentmage_kernel_contracts::WorkspacePath;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{
    DeepRepositoryIndex, RepositoryAnalysisFact, RepositoryFactKind,
    deep_analysis::verify_deep_index_integrity,
};

const CHANGE_INTENT_SCHEMA_VERSION: u16 = 1;
const MAX_TEXT_BYTES: usize = 4_096;
const MAX_LIST_ITEMS: usize = 256;
const MAX_USERS: usize = 64;

/// Closed material-risk domains used to select required reviews.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChangeRiskDomain {
    /// Security boundary or attack-surface risk.
    Security,
    /// Privacy or personal-data risk.
    Privacy,
    /// Data integrity, schema, or retention risk.
    Data,
    /// Accessibility behavior risk.
    Accessibility,
    /// Performance or resource risk.
    Performance,
    /// Migration or compatibility risk.
    Migration,
    /// Reliability or recovery risk.
    Reliability,
    /// Dependency or supply-chain risk.
    Dependency,
}

/// Closed impact surfaces that every intent must assess.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChangeImpactSurfaceKind {
    /// Exact files cited by target facts.
    Files,
    /// Callers and references.
    Callers,
    /// Data and schema behavior.
    Data,
    /// Permission or authorization behavior.
    Permissions,
    /// Tests and fixtures.
    Tests,
    /// Documentation and repository instructions.
    Documentation,
    /// Configuration.
    Configuration,
    /// Migrations.
    Migrations,
    /// Interfaces, traits, and protocols.
    Interfaces,
    /// Package and external dependencies.
    Dependencies,
}

impl ChangeImpactSurfaceKind {
    /// Complete stable impact inventory.
    pub const ALL: [Self; 10] = [
        Self::Files,
        Self::Callers,
        Self::Data,
        Self::Permissions,
        Self::Tests,
        Self::Documentation,
        Self::Configuration,
        Self::Migrations,
        Self::Interfaces,
        Self::Dependencies,
    ];
}

/// Closed ambiguity dimensions that may require user clarification.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChangeUnknownDimension {
    /// Objective or requested behavior is unclear.
    Objective,
    /// Exact current behavior is unclear.
    CurrentBehavior,
    /// Base repository, branch, worktree, or commit is unclear.
    BaseIdentity,
    /// Affected scope is unclear.
    Scope,
    /// Data behavior is unclear.
    Data,
    /// Permission behavior is unclear.
    Permission,
    /// Dependency choice is unclear.
    Dependency,
    /// Migration behavior is unclear.
    Migration,
    /// Rollback or irreversibility is unclear.
    Rollback,
}

/// One material clarification stop with exact supporting facts where available.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChangeClarification {
    /// Stable question identity.
    pub question_id: String,
    /// Closed affected dimension.
    pub dimension: ChangeUnknownDimension,
    /// Bounded user-visible question.
    pub question: String,
    /// Stable exact fact identities supporting the ambiguity.
    pub evidence_fact_ids: Vec<String>,
    /// Material questions stop readiness.
    pub material: bool,
}

/// Explicit claim that one impact surface is outside this change.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChangeSurfaceExclusion {
    /// Exact excluded surface.
    pub surface: ChangeImpactSurfaceKind,
    /// Bounded rationale.
    pub rationale: String,
}

/// Unsealed exact change-intent input.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChangeIntentInput {
    /// Stable intent identity.
    pub intent_id: String,
    /// Requested behavior.
    pub requested_behavior: String,
    /// Current behavior established by cited facts.
    pub current_behavior: String,
    /// Exact current-behavior facts.
    pub current_behavior_fact_ids: Vec<String>,
    /// Minimal exact facts proposed as change targets.
    pub target_fact_ids: Vec<String>,
    /// Bounded affected user or operator classes.
    pub users: Vec<String>,
    /// Exact acceptance checks.
    pub acceptance_checks: Vec<String>,
    /// Explicit behavioral or scope exclusions.
    pub exclusions: Vec<String>,
    /// Typed material risk domains.
    pub risks: Vec<ChangeRiskDomain>,
    /// Bounded rollback or recovery procedure.
    pub rollback: String,
    /// Whether rollback is currently expected to be reversible.
    pub rollback_reversible: bool,
    /// Typed impact surfaces explicitly established as not applicable.
    pub surface_exclusions: Vec<ChangeSurfaceExclusion>,
    /// Material and nonmaterial unresolved questions.
    pub clarifications: Vec<ChangeClarification>,
}

/// Readiness of one normalized intent.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChangeIntentStatus {
    /// Intent is internally complete enough for read-only planning.
    ReadyForPlanning,
    /// One or more material unknowns require clarification.
    ClarificationRequired,
}

/// Immutable normalized change intent bound to one deep index.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChangeIntentRecord {
    /// Schema version.
    pub schema_version: u16,
    /// Exact source deep-index identity.
    pub index_sha256: String,
    /// Exact input retained for deterministic recomputation.
    pub input: ChangeIntentInput,
    /// Current behavior citations resolved from the index.
    pub current_behavior_citation_ids: Vec<String>,
    /// Target citations resolved from the index.
    pub target_citation_ids: Vec<String>,
    /// Explicit readiness.
    pub status: ChangeIntentStatus,
    /// A descriptive record grants no mutation authority.
    pub mutation_authority: bool,
    /// SHA-256 over every preceding field.
    pub intent_sha256: String,
}

/// Evidence state for one assessed impact surface.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChangeImpactState {
    /// Applicable and backed by exact target facts.
    EvidenceBacked,
    /// Explicitly excluded with rationale.
    NotApplicable,
    /// Current evidence cannot establish applicability.
    UnknownBlocked,
}

/// One deterministic impact-surface assessment.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChangeImpactSurface {
    /// Closed surface class.
    pub kind: ChangeImpactSurfaceKind,
    /// Explicit state.
    pub state: ChangeImpactState,
    /// Stable supporting target facts.
    pub fact_ids: Vec<String>,
    /// Exact affected paths resolved from those facts.
    pub paths: Vec<WorkspacePath>,
    /// Optional explicit exclusion rationale.
    pub exclusion_rationale: Option<String>,
    /// SHA-256 over every preceding field.
    pub surface_sha256: String,
}

/// Minimal impact report derived only from declared target facts.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MinimalChangeImpactReport {
    /// Schema version.
    pub schema_version: u16,
    /// Exact source intent identity.
    pub intent_sha256: String,
    /// Exact source deep-index identity.
    pub index_sha256: String,
    /// Minimal declared target facts.
    pub target_fact_ids: Vec<String>,
    /// Exact union of paths cited by target facts.
    pub proposed_paths: Vec<WorkspacePath>,
    /// Exactly one assessment per closed surface.
    pub surfaces: Vec<ChangeImpactSurface>,
    /// Whether every surface is evidence-backed or explicitly not applicable.
    pub complete: bool,
    /// Scope is descriptive and cannot authorize writes.
    pub mutation_authority: bool,
    /// SHA-256 over every preceding field.
    pub impact_sha256: String,
}

/// Content-free change-intent failure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChangeIntentError {
    /// Deep index is malformed or stale internally.
    IndexInvalid,
    /// Intent syntax, bounds, ordering, or relationships are invalid.
    IntentInvalid,
    /// A cited fact is absent or inconsistent.
    EvidenceInvalid,
    /// Impact exclusions or surface mapping are contradictory.
    ImpactInvalid,
}

impl ChangeIntentError {
    /// Stable content-free error code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::IndexInvalid => "change.intent.index_invalid",
            Self::IntentInvalid => "change.intent.invalid",
            Self::EvidenceInvalid => "change.intent.evidence_invalid",
            Self::ImpactInvalid => "change.intent.impact_invalid",
        }
    }
}

/// Normalizes and seals one exact intent without granting mutation authority.
pub fn normalize_change_intent(
    index: &DeepRepositoryIndex,
    input: ChangeIntentInput,
) -> Result<ChangeIntentRecord, ChangeIntentError> {
    if !verify_deep_index_integrity(index) {
        return Err(ChangeIntentError::IndexInvalid);
    }
    validate_intent_input(&input)?;
    let facts = fact_index(index);
    for identity in input
        .current_behavior_fact_ids
        .iter()
        .chain(input.target_fact_ids.iter())
        .chain(
            input
                .clarifications
                .iter()
                .flat_map(|question| question.evidence_fact_ids.iter()),
        )
    {
        if !facts.contains_key(identity.as_str()) {
            return Err(ChangeIntentError::EvidenceInvalid);
        }
    }
    let current_behavior_citation_ids = citation_ids_for(&facts, &input.current_behavior_fact_ids)?;
    let target_citation_ids = citation_ids_for(&facts, &input.target_fact_ids)?;
    let status = if input
        .clarifications
        .iter()
        .any(|question| question.material)
    {
        ChangeIntentStatus::ClarificationRequired
    } else {
        ChangeIntentStatus::ReadyForPlanning
    };
    let mut record = ChangeIntentRecord {
        schema_version: CHANGE_INTENT_SCHEMA_VERSION,
        index_sha256: index.index_sha256.clone(),
        input,
        current_behavior_citation_ids,
        target_citation_ids,
        status,
        mutation_authority: false,
        intent_sha256: String::new(),
    };
    record.intent_sha256 = intent_digest(&record);
    Ok(record)
}

/// Verifies one intent by exact recomputation.
#[must_use]
pub fn verify_change_intent(index: &DeepRepositoryIndex, record: &ChangeIntentRecord) -> bool {
    record.schema_version == CHANGE_INTENT_SCHEMA_VERSION
        && !record.mutation_authority
        && record.intent_sha256 == intent_digest(record)
        && normalize_change_intent(index, record.input.clone())
            .is_ok_and(|expected| expected == *record)
}

/// Derives one minimal impact report from only the intent's exact target facts.
pub fn build_minimal_change_impact(
    index: &DeepRepositoryIndex,
    intent: &ChangeIntentRecord,
) -> Result<MinimalChangeImpactReport, ChangeIntentError> {
    if !verify_change_intent(index, intent) {
        return Err(ChangeIntentError::IntentInvalid);
    }
    let facts = fact_index(index);
    let targets = intent
        .input
        .target_fact_ids
        .iter()
        .map(|identity| {
            facts
                .get(identity.as_str())
                .copied()
                .ok_or(ChangeIntentError::EvidenceInvalid)
        })
        .collect::<Result<Vec<_>, _>>()?;
    let exclusions = intent
        .input
        .surface_exclusions
        .iter()
        .map(|exclusion| (exclusion.surface, exclusion.rationale.as_str()))
        .collect::<BTreeMap<_, _>>();
    let mut proposed_paths = targets
        .iter()
        .flat_map(|fact| fact.citations.iter().map(|citation| citation.path.clone()))
        .collect::<Vec<_>>();
    proposed_paths.sort();
    proposed_paths.dedup();
    if proposed_paths.is_empty() {
        return Err(ChangeIntentError::ImpactInvalid);
    }
    let mut surfaces = ChangeImpactSurfaceKind::ALL
        .into_iter()
        .map(|kind| build_surface(kind, &targets, exclusions.get(&kind).copied()))
        .collect::<Result<Vec<_>, _>>()?;
    surfaces.sort_by_key(|surface| surface.kind);
    let complete = intent.status == ChangeIntentStatus::ReadyForPlanning
        && surfaces
            .iter()
            .all(|surface| surface.state != ChangeImpactState::UnknownBlocked);
    let mut report = MinimalChangeImpactReport {
        schema_version: CHANGE_INTENT_SCHEMA_VERSION,
        intent_sha256: intent.intent_sha256.clone(),
        index_sha256: index.index_sha256.clone(),
        target_fact_ids: intent.input.target_fact_ids.clone(),
        proposed_paths,
        surfaces,
        complete,
        mutation_authority: false,
        impact_sha256: String::new(),
    };
    report.impact_sha256 = impact_digest(&report);
    Ok(report)
}

/// Verifies one impact report by exact deterministic recomputation.
#[must_use]
pub fn verify_minimal_change_impact(
    index: &DeepRepositoryIndex,
    intent: &ChangeIntentRecord,
    report: &MinimalChangeImpactReport,
) -> bool {
    !report.mutation_authority
        && report.impact_sha256 == impact_digest(report)
        && build_minimal_change_impact(index, intent).is_ok_and(|expected| expected == *report)
}

fn validate_intent_input(input: &ChangeIntentInput) -> Result<(), ChangeIntentError> {
    if !valid_identifier(&input.intent_id)
        || !valid_text(&input.requested_behavior)
        || !valid_text(&input.current_behavior)
        || input.requested_behavior == input.current_behavior
        || !valid_sorted_ids(&input.current_behavior_fact_ids, false)
        || !valid_sorted_ids(&input.target_fact_ids, false)
        || !valid_sorted_text(&input.users, MAX_USERS, false)
        || !valid_sorted_text(&input.acceptance_checks, MAX_LIST_ITEMS, false)
        || !valid_sorted_text(&input.exclusions, MAX_LIST_ITEMS, false)
        || input.risks.is_empty()
        || input.risks.len() > ChangeRiskDomain::ALL.len()
        || input.risks.windows(2).any(|pair| pair[0] >= pair[1])
        || !valid_text(&input.rollback)
        || input.surface_exclusions.len() >= ChangeImpactSurfaceKind::ALL.len()
        || input
            .surface_exclusions
            .windows(2)
            .any(|pair| pair[0].surface >= pair[1].surface)
        || input.surface_exclusions.iter().any(|exclusion| {
            exclusion.surface == ChangeImpactSurfaceKind::Files || !valid_text(&exclusion.rationale)
        })
        || input.clarifications.len() > MAX_LIST_ITEMS
        || input
            .clarifications
            .windows(2)
            .any(|pair| pair[0].question_id >= pair[1].question_id)
        || input.clarifications.iter().any(|question| {
            !valid_identifier(&question.question_id)
                || !valid_text(&question.question)
                || !valid_sorted_ids(&question.evidence_fact_ids, true)
        })
    {
        return Err(ChangeIntentError::IntentInvalid);
    }
    let current = input
        .current_behavior_fact_ids
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    if !input
        .target_fact_ids
        .iter()
        .any(|identity| current.contains(identity.as_str()))
    {
        return Err(ChangeIntentError::IntentInvalid);
    }
    Ok(())
}

impl ChangeRiskDomain {
    const ALL: [Self; 8] = [
        Self::Security,
        Self::Privacy,
        Self::Data,
        Self::Accessibility,
        Self::Performance,
        Self::Migration,
        Self::Reliability,
        Self::Dependency,
    ];
}

fn build_surface(
    kind: ChangeImpactSurfaceKind,
    targets: &[&RepositoryAnalysisFact],
    exclusion: Option<&str>,
) -> Result<ChangeImpactSurface, ChangeIntentError> {
    let matching = targets
        .iter()
        .copied()
        .filter(|fact| surface_matches_fact(kind, fact.kind))
        .collect::<Vec<_>>();
    if !matching.is_empty() && exclusion.is_some() {
        return Err(ChangeIntentError::ImpactInvalid);
    }
    let state = if !matching.is_empty() {
        ChangeImpactState::EvidenceBacked
    } else if exclusion.is_some() {
        ChangeImpactState::NotApplicable
    } else {
        ChangeImpactState::UnknownBlocked
    };
    let mut fact_ids = matching
        .iter()
        .map(|fact| fact.fact_id.clone())
        .collect::<Vec<_>>();
    fact_ids.sort();
    let mut paths = matching
        .iter()
        .flat_map(|fact| fact.citations.iter().map(|citation| citation.path.clone()))
        .collect::<Vec<_>>();
    paths.sort();
    paths.dedup();
    let mut surface = ChangeImpactSurface {
        kind,
        state,
        fact_ids,
        paths,
        exclusion_rationale: exclusion.map(str::to_owned),
        surface_sha256: String::new(),
    };
    surface.surface_sha256 = surface_digest(&surface);
    Ok(surface)
}

fn surface_matches_fact(surface: ChangeImpactSurfaceKind, fact: RepositoryFactKind) -> bool {
    match surface {
        ChangeImpactSurfaceKind::Files => true,
        ChangeImpactSurfaceKind::Callers => {
            matches!(
                fact,
                RepositoryFactKind::Call | RepositoryFactKind::Reference
            )
        }
        ChangeImpactSurfaceKind::Data => {
            matches!(fact, RepositoryFactKind::Schema | RepositoryFactKind::Type)
        }
        ChangeImpactSurfaceKind::Permissions => false,
        ChangeImpactSurfaceKind::Tests => fact == RepositoryFactKind::Test,
        ChangeImpactSurfaceKind::Documentation => matches!(
            fact,
            RepositoryFactKind::Documentation | RepositoryFactKind::Instruction
        ),
        ChangeImpactSurfaceKind::Configuration => fact == RepositoryFactKind::Configuration,
        ChangeImpactSurfaceKind::Migrations => fact == RepositoryFactKind::Migration,
        ChangeImpactSurfaceKind::Interfaces => fact == RepositoryFactKind::Interface,
        ChangeImpactSurfaceKind::Dependencies => matches!(
            fact,
            RepositoryFactKind::PackageManager | RepositoryFactKind::ExternalDependency
        ),
    }
}

fn fact_index(index: &DeepRepositoryIndex) -> BTreeMap<&str, &RepositoryAnalysisFact> {
    index
        .facts
        .iter()
        .map(|fact| (fact.fact_id.as_str(), fact))
        .collect()
}

fn citation_ids_for(
    facts: &BTreeMap<&str, &RepositoryAnalysisFact>,
    identities: &[String],
) -> Result<Vec<String>, ChangeIntentError> {
    let mut citations = identities
        .iter()
        .map(|identity| {
            facts
                .get(identity.as_str())
                .ok_or(ChangeIntentError::EvidenceInvalid)
        })
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .flat_map(|fact| {
            fact.citations
                .iter()
                .map(|citation| citation.citation_sha256.clone())
        })
        .collect::<Vec<_>>();
    citations.sort();
    citations.dedup();
    Ok(citations)
}

fn valid_sorted_ids(values: &[String], allow_empty: bool) -> bool {
    (allow_empty || !values.is_empty())
        && values.len() <= MAX_LIST_ITEMS
        && values.iter().all(|value| is_sha256(value))
        && values.windows(2).all(|pair| pair[0] < pair[1])
}

fn valid_sorted_text(values: &[String], maximum: usize, allow_empty: bool) -> bool {
    (allow_empty || !values.is_empty())
        && values.len() <= maximum
        && values.iter().all(|value| valid_text(value))
        && values.windows(2).all(|pair| pair[0] < pair[1])
}

fn valid_text(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_TEXT_BYTES
        && !value.chars().any(|character| character.is_control())
}

fn valid_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || "-._:".contains(character))
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn intent_digest(record: &ChangeIntentRecord) -> String {
    sha256_json(&(
        record.schema_version,
        &record.index_sha256,
        &record.input,
        &record.current_behavior_citation_ids,
        &record.target_citation_ids,
        record.status,
        record.mutation_authority,
    ))
}

fn surface_digest(surface: &ChangeImpactSurface) -> String {
    sha256_json(&(
        surface.kind,
        surface.state,
        &surface.fact_ids,
        &surface.paths,
        &surface.exclusion_rationale,
    ))
}

fn impact_digest(report: &MinimalChangeImpactReport) -> String {
    sha256_json(&(
        report.schema_version,
        &report.intent_sha256,
        &report.index_sha256,
        &report.target_fact_ids,
        &report.proposed_paths,
        &report.surfaces,
        report.complete,
        report.mutation_authority,
    ))
}

fn sha256_json<T: Serialize>(value: &T) -> String {
    serde_json::to_vec(value)
        .map(|bytes| sha256_hex(&bytes))
        .unwrap_or_else(|_| sha256_hex(b"change-intent-serialization-failed"))
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(64);
    for byte in Sha256::digest(bytes) {
        write!(&mut output, "{byte:02x}").expect("writing to String cannot fail");
    }
    output
}

#[cfg(test)]
mod tests {
    use agentmage_kernel_contracts::WorkspaceId;

    use super::*;
    use crate::{
        GitTrackedState, RepositoryFileInput, RepositoryMapInput, build_deep_repository_index,
        build_repository_map,
    };

    fn source(path: &[&str], content: &str) -> RepositoryFileInput {
        RepositoryFileInput {
            path: path.iter().map(|value| (*value).to_owned()).collect(),
            size_bytes: content.len() as u64,
            content_sha256: sha256_hex(content.as_bytes()),
            content: Some(content.as_bytes().to_vec()),
            git_state: GitTrackedState::TrackedClean,
            policy_excluded: false,
            generated: false,
            vendored: false,
        }
    }

    fn index() -> DeepRepositoryIndex {
        let map = build_repository_map(RepositoryMapInput {
            workspace_id: WorkspaceId::from_raw("workspace-change-intent"),
            repository_sha256: "a".repeat(64),
            worktree_sha256: "b".repeat(64),
            branch: Some("agentmage/tasks/change-intent".to_owned()),
            commit_id: "c".repeat(40),
            policy_sha256: "d".repeat(64),
            freshness_sha256: "e".repeat(64),
            files: vec![
                source(&["src", "lib.rs"], "pub fn calculate() -> u64 { 1 }\n"),
                source(
                    &["tests", "calculate_test.rs"],
                    "fn calculation_is_stable() {}\n",
                ),
                source(&["docs", "behavior.md"], "Current behavior.\n"),
            ],
        })
        .expect("map");
        build_deep_repository_index(&map, Vec::new(), Vec::new()).expect("index")
    }

    fn input(index: &DeepRepositoryIndex) -> ChangeIntentInput {
        let definition = index
            .facts
            .iter()
            .find(|fact| fact.label == "calculate")
            .expect("definition")
            .fact_id
            .clone();
        let mut surface_exclusions = ChangeImpactSurfaceKind::ALL
            .into_iter()
            .filter(|surface| *surface != ChangeImpactSurfaceKind::Files)
            .map(|surface| ChangeSurfaceExclusion {
                surface,
                rationale: "Fixture evidence establishes no impact in this surface".to_owned(),
            })
            .collect::<Vec<_>>();
        surface_exclusions.sort_by_key(|exclusion| exclusion.surface);
        ChangeIntentInput {
            intent_id: "intent-calculate-1".to_owned(),
            requested_behavior: "Return the accepted fixture value".to_owned(),
            current_behavior: "The cited function returns the old fixture value".to_owned(),
            current_behavior_fact_ids: vec![definition.clone()],
            target_fact_ids: vec![definition],
            users: vec!["library callers".to_owned()],
            acceptance_checks: vec!["The exact regression fixture passes".to_owned()],
            exclusions: vec!["No public interface change".to_owned()],
            risks: vec![ChangeRiskDomain::Reliability],
            rollback: "Restore the exact preimage after a failed validation".to_owned(),
            rollback_reversible: true,
            surface_exclusions,
            clarifications: Vec::new(),
        }
    }

    #[test]
    fn exact_intent_and_minimal_impact_are_deterministic_and_authority_free() {
        let index = index();
        let intent = normalize_change_intent(&index, input(&index)).expect("intent");
        assert!(verify_change_intent(&index, &intent));
        assert_eq!(intent.status, ChangeIntentStatus::ReadyForPlanning);
        assert!(!intent.mutation_authority);
        let impact = build_minimal_change_impact(&index, &intent).expect("impact");
        assert!(verify_minimal_change_impact(&index, &intent, &impact));
        assert!(impact.complete);
        assert!(!impact.mutation_authority);
        assert_eq!(impact.proposed_paths.len(), 1);
        assert_eq!(
            impact
                .surfaces
                .iter()
                .find(|surface| surface.kind == ChangeImpactSurfaceKind::Files)
                .expect("files")
                .state,
            ChangeImpactState::EvidenceBacked
        );
    }

    #[test]
    fn material_unknown_and_unassessed_surface_stop_planning() {
        let index = index();
        let mut candidate = input(&index);
        candidate.clarifications.push(ChangeClarification {
            question_id: "question-scope".to_owned(),
            dimension: ChangeUnknownDimension::Scope,
            question: "Should the sibling implementation change too?".to_owned(),
            evidence_fact_ids: Vec::new(),
            material: true,
        });
        candidate.surface_exclusions.pop();
        let intent = normalize_change_intent(&index, candidate).expect("blocked intent");
        assert_eq!(intent.status, ChangeIntentStatus::ClarificationRequired);
        let impact = build_minimal_change_impact(&index, &intent).expect("impact");
        assert!(!impact.complete);
        assert!(
            impact
                .surfaces
                .iter()
                .any(|surface| { surface.state == ChangeImpactState::UnknownBlocked })
        );
    }

    #[test]
    fn stale_facts_contradictions_and_scope_broadening_fail_closed() {
        let index = index();
        let mut missing = input(&index);
        missing.target_fact_ids = vec!["9".repeat(64)];
        assert_eq!(
            normalize_change_intent(&index, missing),
            Err(ChangeIntentError::IntentInvalid)
        );

        let mut contradiction = input(&index);
        contradiction.requested_behavior = contradiction.current_behavior.clone();
        assert_eq!(
            normalize_change_intent(&index, contradiction),
            Err(ChangeIntentError::IntentInvalid)
        );

        let intent = normalize_change_intent(&index, input(&index)).expect("intent");
        let mut impact = build_minimal_change_impact(&index, &intent).expect("impact");
        impact.proposed_paths.push(
            WorkspacePath::new(
                intent.input.target_fact_ids.first().map_or_else(
                    || WorkspaceId::from_raw("workspace-change-intent"),
                    |_| WorkspaceId::from_raw("workspace-change-intent"),
                ),
                ["unrelated.rs"],
            )
            .expect("path"),
        );
        assert!(!verify_minimal_change_impact(&index, &intent, &impact));
    }
}
