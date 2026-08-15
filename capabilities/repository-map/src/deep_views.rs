//! Branch-aware traces, bounded slices, and structured repository-learning exports.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fmt::Write;

use agentmage_kernel_contracts::WorkspacePath;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{
    DeepRepositoryIndex, RepositoryAnalysisFact, RepositoryBlindSpot, RepositoryFactCitation,
    RepositoryFactKind, RepositoryFactState, deep_analysis::verify_deep_index_integrity,
};

const VIEW_SCHEMA_VERSION: u16 = 1;
const MAX_HISTORY_OBSERVATIONS: usize = 10_000;
const MAX_HISTORY_PATHS: usize = 10_000;
const MAX_PORTFOLIO_REPOSITORIES: usize = 64;
const MAX_SLICE_TARGETS: usize = 128;
const MAX_SLICE_FACTS: usize = 50_000;
const MAX_NEIGHBORHOOD_DEPTH: u8 = 4;

/// Closed cited repository trace classes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RepositoryTraceKind {
    /// Application, package, service, library, module, and entry-point structure.
    Architecture,
    /// Candidate feature surface, always explicitly inferred without semantic evidence.
    Feature,
    /// Call and reference flow.
    DataFlow,
    /// Authentication behavior.
    Authentication,
    /// Authorization behavior.
    Authorization,
    /// Schema artifacts.
    Schema,
    /// Migration artifacts.
    Migration,
    /// Interfaces, traits, and protocols.
    Interface,
    /// Test surfaces.
    Test,
    /// Continuous-integration configuration.
    ContinuousIntegration,
    /// Package and import dependencies.
    Dependency,
}

impl RepositoryTraceKind {
    /// Complete stable trace inventory.
    pub const ALL: [Self; 11] = [
        Self::Architecture,
        Self::Feature,
        Self::DataFlow,
        Self::Authentication,
        Self::Authorization,
        Self::Schema,
        Self::Migration,
        Self::Interface,
        Self::Test,
        Self::ContinuousIntegration,
        Self::Dependency,
    ];
}

/// One exact cited trace or explicit non-claim.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RepositoryTrace {
    /// Stable trace identity.
    pub trace_id: String,
    /// Closed trace class.
    pub kind: RepositoryTraceKind,
    /// Explicit evidence strength.
    pub state: RepositoryFactState,
    /// Sorted fact identities supporting this trace.
    pub fact_ids: Vec<String>,
    /// Sorted blind-spot identities limiting this trace.
    pub blind_spot_ids: Vec<String>,
    /// SHA-256 over every preceding field.
    pub trace_sha256: String,
}

/// Complete trace set for one exact deep index.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RepositoryTraceSet {
    /// Schema version.
    pub schema_version: u16,
    /// Exact deep-index identity.
    pub index_sha256: String,
    /// Exactly one trace per closed trace class.
    pub traces: Vec<RepositoryTrace>,
    /// SHA-256 over every preceding field.
    pub trace_set_sha256: String,
}

/// Exact bounded Git history observation supplied by a read-only host collector.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RepositoryHistoryObservation {
    /// Exact 40- or 64-character commit object identity.
    pub commit_id: String,
    /// Stable sorted parent object identities.
    pub parent_ids: Vec<String>,
    /// Stable sorted changed paths; no file content is retained.
    pub changed_paths: Vec<WorkspacePath>,
    /// Monotonic collector sequence, independent of wall-clock time.
    pub sequence: u64,
    /// SHA-256 over every preceding field.
    pub observation_sha256: String,
}

impl RepositoryHistoryObservation {
    /// Seals one content-minimized history observation.
    pub fn seal(mut self) -> Result<Self, RepositoryDeepViewError> {
        self.observation_sha256.clear();
        validate_history_fields(&self)?;
        self.observation_sha256 = history_observation_digest(&self);
        Ok(self)
    }

    /// Verifies the observation independently of a workspace.
    #[must_use]
    pub fn verify(&self) -> bool {
        validate_history_fields(self).is_ok()
            && self.observation_sha256 == history_observation_digest(self)
    }
}

/// Branch-aware bounded Git history view.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RepositoryHistoryView {
    /// Schema version.
    pub schema_version: u16,
    /// Exact source index identity.
    pub index_sha256: String,
    /// Exact branch, or detached state.
    pub branch: Option<String>,
    /// Exact current map commit.
    pub current_commit_id: String,
    /// Stable sequence-ordered observations.
    pub observations: Vec<RepositoryHistoryObservation>,
    /// Whether the supplied history reaches the exact current commit.
    pub current_commit_observed: bool,
    /// Whether the history was truncated by the collector before this capability.
    pub truncated: bool,
    /// SHA-256 over every preceding field.
    pub history_sha256: String,
}

/// Deterministic relationship between one documentation source and implementation sources.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RepositoryDocumentationDriftState {
    /// Exact external checker found the documentation consistent with cited implementation.
    Current,
    /// Exact external checker found a contradiction.
    Drifted,
    /// No deterministic checker established consistency.
    UnknownBlocked,
}

/// One source-resolvable documentation-drift record.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RepositoryDocumentationDrift {
    /// Stable record identity.
    pub drift_id: String,
    /// Exact documentation citation.
    pub documentation: RepositoryFactCitation,
    /// Stable exact implementation citations.
    pub implementation: Vec<RepositoryFactCitation>,
    /// Explicit drift disposition.
    pub state: RepositoryDocumentationDriftState,
    /// Exact deterministic checker identity, or `None` when unknown.
    pub checker_sha256: Option<String>,
    /// SHA-256 over every preceding field.
    pub drift_sha256: String,
}

/// One deterministic glossary entry derived from exact fact labels.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RepositoryGlossaryTerm {
    /// Exact bounded term.
    pub term: String,
    /// Stable supporting fact identities.
    pub fact_ids: Vec<String>,
    /// The term is derived from labels, not a semantic definition.
    pub state: RepositoryFactState,
    /// SHA-256 over every preceding field.
    pub term_sha256: String,
}

/// One repository entry in a cross-repository portfolio.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RepositoryPortfolioEntry {
    /// Stable repository identity digest.
    pub repository_sha256: String,
    /// Exact worktree identity digest.
    pub worktree_sha256: String,
    /// Exact branch, or detached state.
    pub branch: Option<String>,
    /// Exact commit identity.
    pub commit_id: String,
    /// Exact index identity.
    pub index_sha256: String,
}

/// Cross-repository view that never silently combines branches, worktrees, or commits.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RepositoryPortfolioView {
    /// Schema version.
    pub schema_version: u16,
    /// Stable separately identified repository entries.
    pub repositories: Vec<RepositoryPortfolioEntry>,
    /// Cross-repository relationships are absent until separately cited.
    pub cross_repository_relationships_observed: bool,
    /// SHA-256 over every preceding field.
    pub portfolio_sha256: String,
}

/// Closed large-repository slicing strategies.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RepositorySliceDimension {
    /// Package or component facts and their cited neighborhoods.
    Package,
    /// Entry points and facts sharing their cited files.
    EntryPoint,
    /// Dependency facts and cited neighborhoods.
    DependencyNeighborhood,
    /// Facts touching paths changed in a supplied history view.
    History,
    /// Exact user-selected facts and paths.
    UserSelected,
}

/// Exact bounded slice request.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RepositorySliceRequest {
    /// Slicing strategy.
    pub dimension: RepositorySliceDimension,
    /// Exact target fact identities.
    pub fact_ids: Vec<String>,
    /// Exact target paths.
    pub paths: Vec<WorkspacePath>,
    /// Closed target fact kinds.
    pub kinds: Vec<RepositoryFactKind>,
    /// Shared-citation neighborhood depth.
    pub neighborhood_depth: u8,
    /// Maximum facts returned.
    pub max_facts: u64,
}

/// Coverage for one bounded slice.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RepositorySliceCoverage {
    /// Total source-index fact count.
    pub available_facts: u64,
    /// Facts selected before the fixed output ceiling.
    pub matched_facts: u64,
    /// Facts returned.
    pub returned_facts: u64,
    /// Matching facts omitted by the output ceiling.
    pub omitted_facts: u64,
    /// Requested fact identities not present in the source index.
    pub unresolved_fact_ids: Vec<String>,
    /// Requested paths not cited by the source index.
    pub unresolved_paths: Vec<WorkspacePath>,
}

/// Deterministic bounded deep-repository slice.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RepositoryAnalysisSlice {
    /// Schema version.
    pub schema_version: u16,
    /// Exact source index identity.
    pub index_sha256: String,
    /// Exact request identity.
    pub request_sha256: String,
    /// Stable retained facts.
    pub facts: Vec<RepositoryAnalysisFact>,
    /// All source-index blind spots remain visible.
    pub blind_spots: Vec<RepositoryBlindSpot>,
    /// Complete slice coverage.
    pub coverage: RepositorySliceCoverage,
    /// Whether matching facts were omitted.
    pub truncated: bool,
    /// SHA-256 over every preceding field.
    pub slice_sha256: String,
}

/// Closed structured learning-export classes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RepositoryLearningGuideKind {
    /// Onboarding guide.
    Onboarding,
    /// Architecture guide.
    Architecture,
    /// Dependency guide.
    Dependency,
    /// Feature guide.
    Feature,
    /// Test guide.
    Test,
    /// Build guide.
    Build,
    /// Operations guide.
    Operations,
    /// Open-question guide.
    OpenQuestions,
}

impl RepositoryLearningGuideKind {
    /// Complete stable guide inventory.
    pub const ALL: [Self; 8] = [
        Self::Onboarding,
        Self::Architecture,
        Self::Dependency,
        Self::Feature,
        Self::Test,
        Self::Build,
        Self::Operations,
        Self::OpenQuestions,
    ];
}

/// One content-minimized guide containing only resolvable record identities.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RepositoryLearningGuide {
    /// Closed guide class.
    pub kind: RepositoryLearningGuideKind,
    /// Stable included fact identities.
    pub fact_ids: Vec<String>,
    /// Stable included trace identities.
    pub trace_ids: Vec<String>,
    /// Stable visible blind-spot identities.
    pub blind_spot_ids: Vec<String>,
    /// SHA-256 over every preceding field.
    pub guide_sha256: String,
}

/// Complete repository-learning export suite.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RepositoryLearningExport {
    /// Schema version.
    pub schema_version: u16,
    /// Exact source index identity.
    pub index_sha256: String,
    /// Exact source trace-set identity.
    pub trace_set_sha256: String,
    /// Exactly one guide per closed guide class.
    pub guides: Vec<RepositoryLearningGuide>,
    /// SHA-256 over every preceding field.
    pub export_sha256: String,
}

/// Content-free deep-view failure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RepositoryDeepViewError {
    /// Source index, trace, request, or collection is malformed or stale.
    InvalidInput,
    /// History evidence is malformed, stale, or unbounded.
    HistoryInvalid,
    /// Documentation citation or deterministic-checker evidence is invalid.
    DocumentationInvalid,
    /// Requested slice exceeds fixed limits.
    SliceInvalid,
}

impl RepositoryDeepViewError {
    /// Stable content-free error code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidInput => "repository.deep_view.input_invalid",
            Self::HistoryInvalid => "repository.deep_view.history_invalid",
            Self::DocumentationInvalid => "repository.deep_view.documentation_invalid",
            Self::SliceInvalid => "repository.deep_view.slice_invalid",
        }
    }
}

/// Builds all closed trace classes from one verified deep index.
pub fn build_repository_traces(
    index: &DeepRepositoryIndex,
) -> Result<RepositoryTraceSet, RepositoryDeepViewError> {
    validate_index_shape(index)?;
    let mut traces = RepositoryTraceKind::ALL
        .into_iter()
        .map(|kind| build_trace(index, kind))
        .collect::<Vec<_>>();
    traces.sort_by_key(|trace| trace.kind);
    let mut set = RepositoryTraceSet {
        schema_version: VIEW_SCHEMA_VERSION,
        index_sha256: index.index_sha256.clone(),
        traces,
        trace_set_sha256: String::new(),
    };
    set.trace_set_sha256 = trace_set_digest(&set);
    Ok(set)
}

/// Verifies a trace set by exact deterministic recomputation.
#[must_use]
pub fn verify_repository_traces(index: &DeepRepositoryIndex, traces: &RepositoryTraceSet) -> bool {
    build_repository_traces(index).is_ok_and(|expected| expected == *traces)
}

/// Builds a branch-aware history view from already bounded Git observations.
pub fn build_repository_history_view(
    index: &DeepRepositoryIndex,
    mut observations: Vec<RepositoryHistoryObservation>,
    truncated: bool,
) -> Result<RepositoryHistoryView, RepositoryDeepViewError> {
    validate_index_shape(index)?;
    if observations.is_empty() || observations.len() > MAX_HISTORY_OBSERVATIONS {
        return Err(RepositoryDeepViewError::HistoryInvalid);
    }
    observations.sort_by_key(|observation| observation.sequence);
    if observations.iter().any(|observation| {
        !observation.verify()
            || observation.changed_paths.len() > MAX_HISTORY_PATHS
            || observation
                .changed_paths
                .iter()
                .any(|path| path.workspace_id() != index.facts[0].citations[0].path.workspace_id())
    }) || observations
        .windows(2)
        .any(|pair| pair[0].sequence >= pair[1].sequence)
    {
        return Err(RepositoryDeepViewError::HistoryInvalid);
    }
    let current_commit_observed = observations
        .iter()
        .any(|observation| observation.commit_id == index.git.commit_id);
    let mut view = RepositoryHistoryView {
        schema_version: VIEW_SCHEMA_VERSION,
        index_sha256: index.index_sha256.clone(),
        branch: index.git.branch.clone(),
        current_commit_id: index.git.commit_id.clone(),
        observations,
        current_commit_observed,
        truncated,
        history_sha256: String::new(),
    };
    view.history_sha256 = history_view_digest(&view);
    Ok(view)
}

/// Seals one deterministic documentation-drift record against a current index.
pub fn seal_documentation_drift(
    index: &DeepRepositoryIndex,
    mut record: RepositoryDocumentationDrift,
) -> Result<RepositoryDocumentationDrift, RepositoryDeepViewError> {
    validate_index_shape(index)?;
    record.drift_id.clear();
    record.drift_sha256.clear();
    record
        .implementation
        .sort_by(|left, right| left.citation_sha256.cmp(&right.citation_sha256));
    record
        .implementation
        .dedup_by(|left, right| left.citation_sha256 == right.citation_sha256);
    let citations = index
        .facts
        .iter()
        .flat_map(|fact| fact.citations.iter())
        .map(|citation| citation.citation_sha256.as_str())
        .collect::<BTreeSet<_>>();
    if !citations.contains(record.documentation.citation_sha256.as_str())
        || record.implementation.is_empty()
        || record
            .implementation
            .iter()
            .any(|citation| !citations.contains(citation.citation_sha256.as_str()))
        || match record.state {
            RepositoryDocumentationDriftState::Current
            | RepositoryDocumentationDriftState::Drifted => record
                .checker_sha256
                .as_deref()
                .is_none_or(|value| !is_sha256(value)),
            RepositoryDocumentationDriftState::UnknownBlocked => record.checker_sha256.is_some(),
        }
    {
        return Err(RepositoryDeepViewError::DocumentationInvalid);
    }
    record.drift_id = sha256_json(&(
        &record.documentation,
        &record.implementation,
        record.state,
        &record.checker_sha256,
    ));
    record.drift_sha256 = documentation_drift_digest(&record);
    Ok(record)
}

/// Builds a deterministic label glossary without inventing semantic definitions.
pub fn build_repository_glossary(
    index: &DeepRepositoryIndex,
) -> Result<Vec<RepositoryGlossaryTerm>, RepositoryDeepViewError> {
    validate_index_shape(index)?;
    let mut grouped = BTreeMap::<String, Vec<String>>::new();
    for fact in &index.facts {
        grouped
            .entry(fact.label.clone())
            .or_default()
            .push(fact.fact_id.clone());
    }
    Ok(grouped
        .into_iter()
        .map(|(term, mut fact_ids)| {
            fact_ids.sort();
            fact_ids.dedup();
            let mut entry = RepositoryGlossaryTerm {
                term,
                fact_ids,
                state: RepositoryFactState::Derived,
                term_sha256: String::new(),
            };
            entry.term_sha256 = glossary_digest(&entry);
            entry
        })
        .collect())
}

/// Builds a cross-repository view while preserving every exact branch and commit identity.
pub fn build_repository_portfolio(
    indexes: &[DeepRepositoryIndex],
) -> Result<RepositoryPortfolioView, RepositoryDeepViewError> {
    if indexes.is_empty() || indexes.len() > MAX_PORTFOLIO_REPOSITORIES {
        return Err(RepositoryDeepViewError::InvalidInput);
    }
    let mut repositories = indexes
        .iter()
        .map(|index| {
            validate_index_shape(index)?;
            Ok(RepositoryPortfolioEntry {
                repository_sha256: index.git.repository_sha256.clone(),
                worktree_sha256: index.git.worktree_sha256.clone(),
                branch: index.git.branch.clone(),
                commit_id: index.git.commit_id.clone(),
                index_sha256: index.index_sha256.clone(),
            })
        })
        .collect::<Result<Vec<_>, RepositoryDeepViewError>>()?;
    repositories.sort_by(|left, right| {
        left.repository_sha256
            .cmp(&right.repository_sha256)
            .then_with(|| left.worktree_sha256.cmp(&right.worktree_sha256))
            .then_with(|| left.commit_id.cmp(&right.commit_id))
    });
    if repositories.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err(RepositoryDeepViewError::InvalidInput);
    }
    let mut view = RepositoryPortfolioView {
        schema_version: VIEW_SCHEMA_VERSION,
        repositories,
        cross_repository_relationships_observed: false,
        portfolio_sha256: String::new(),
    };
    view.portfolio_sha256 = portfolio_digest(&view);
    Ok(view)
}

/// Builds one bounded deterministic repository slice.
pub fn slice_deep_repository_index(
    index: &DeepRepositoryIndex,
    request: &RepositorySliceRequest,
    history: Option<&RepositoryHistoryView>,
) -> Result<RepositoryAnalysisSlice, RepositoryDeepViewError> {
    validate_index_shape(index)?;
    validate_slice_request(index, request, history)?;
    let fact_index = index
        .facts
        .iter()
        .map(|fact| (fact.fact_id.as_str(), fact))
        .collect::<BTreeMap<_, _>>();
    let cited_paths = index
        .facts
        .iter()
        .flat_map(|fact| fact.citations.iter().map(|citation| &citation.path))
        .collect::<BTreeSet<_>>();
    let mut selected = seed_slice(index, request, history);
    let mut frontier = selected.iter().cloned().collect::<VecDeque<_>>();
    for _ in 0..request.neighborhood_depth {
        let level = frontier.len();
        for _ in 0..level {
            let Some(fact_id) = frontier.pop_front() else {
                break;
            };
            let Some(fact) = fact_index.get(fact_id.as_str()) else {
                continue;
            };
            let paths = fact
                .citations
                .iter()
                .map(|citation| &citation.path)
                .collect::<BTreeSet<_>>();
            for neighbor in &index.facts {
                if !selected.contains(&neighbor.fact_id)
                    && neighbor
                        .citations
                        .iter()
                        .any(|citation| paths.contains(&citation.path))
                {
                    selected.insert(neighbor.fact_id.clone());
                    frontier.push_back(neighbor.fact_id.clone());
                }
            }
        }
    }
    let mut matched = selected
        .iter()
        .filter_map(|identity| fact_index.get(identity.as_str()).copied())
        .cloned()
        .collect::<Vec<_>>();
    matched.sort_by(|left, right| left.fact_id.cmp(&right.fact_id));
    let matched_count = matched.len() as u64;
    matched.truncate(request.max_facts as usize);
    let unresolved_fact_ids = request
        .fact_ids
        .iter()
        .filter(|identity| !fact_index.contains_key(identity.as_str()))
        .cloned()
        .collect::<Vec<_>>();
    let unresolved_paths = request
        .paths
        .iter()
        .filter(|path| !cited_paths.contains(path))
        .cloned()
        .collect::<Vec<_>>();
    let returned = matched.len() as u64;
    let coverage = RepositorySliceCoverage {
        available_facts: index.facts.len() as u64,
        matched_facts: matched_count,
        returned_facts: returned,
        omitted_facts: matched_count.saturating_sub(returned),
        unresolved_fact_ids,
        unresolved_paths,
    };
    let mut slice = RepositoryAnalysisSlice {
        schema_version: VIEW_SCHEMA_VERSION,
        index_sha256: index.index_sha256.clone(),
        request_sha256: sha256_json(request),
        facts: matched,
        blind_spots: index.blind_spots.clone(),
        truncated: coverage.omitted_facts > 0,
        coverage,
        slice_sha256: String::new(),
    };
    slice.slice_sha256 = slice_digest(&slice);
    Ok(slice)
}

/// Verifies a slice by exact deterministic recomputation.
#[must_use]
pub fn verify_repository_slice(
    index: &DeepRepositoryIndex,
    request: &RepositorySliceRequest,
    history: Option<&RepositoryHistoryView>,
    slice: &RepositoryAnalysisSlice,
) -> bool {
    slice_deep_repository_index(index, request, history).is_ok_and(|expected| expected == *slice)
}

/// Builds all structured repository-learning guides from cited identities only.
pub fn build_repository_learning_export(
    index: &DeepRepositoryIndex,
    traces: &RepositoryTraceSet,
) -> Result<RepositoryLearningExport, RepositoryDeepViewError> {
    validate_index_shape(index)?;
    if !verify_repository_traces(index, traces) {
        return Err(RepositoryDeepViewError::InvalidInput);
    }
    let mut guides = RepositoryLearningGuideKind::ALL
        .into_iter()
        .map(|kind| build_guide(index, traces, kind))
        .collect::<Vec<_>>();
    guides.sort_by_key(|guide| guide.kind);
    let mut export = RepositoryLearningExport {
        schema_version: VIEW_SCHEMA_VERSION,
        index_sha256: index.index_sha256.clone(),
        trace_set_sha256: traces.trace_set_sha256.clone(),
        guides,
        export_sha256: String::new(),
    };
    export.export_sha256 = export_digest(&export);
    Ok(export)
}

fn build_trace(index: &DeepRepositoryIndex, kind: RepositoryTraceKind) -> RepositoryTrace {
    let accepted_kinds = trace_fact_kinds(kind);
    let mut fact_ids = index
        .facts
        .iter()
        .filter(|fact| accepted_kinds.contains(&fact.kind))
        .map(|fact| fact.fact_id.clone())
        .collect::<Vec<_>>();
    fact_ids.sort();
    let state = if fact_ids.is_empty() {
        RepositoryFactState::UnknownBlocked
    } else if kind == RepositoryTraceKind::Feature {
        RepositoryFactState::Inferred
    } else {
        RepositoryFactState::Derived
    };
    let mut blind_spot_ids = if state == RepositoryFactState::UnknownBlocked {
        index
            .blind_spots
            .iter()
            .map(|spot| spot.blind_spot_id.clone())
            .collect::<Vec<_>>()
    } else {
        index
            .blind_spots
            .iter()
            .filter(|spot| spot.path.is_some())
            .map(|spot| spot.blind_spot_id.clone())
            .collect::<Vec<_>>()
    };
    blind_spot_ids.sort();
    let trace_id = sha256_json(&(kind, state, &fact_ids, &blind_spot_ids));
    let mut trace = RepositoryTrace {
        trace_id,
        kind,
        state,
        fact_ids,
        blind_spot_ids,
        trace_sha256: String::new(),
    };
    trace.trace_sha256 = trace_digest(&trace);
    trace
}

fn trace_fact_kinds(kind: RepositoryTraceKind) -> BTreeSet<RepositoryFactKind> {
    let kinds: &[RepositoryFactKind] = match kind {
        RepositoryTraceKind::Architecture => &[
            RepositoryFactKind::Application,
            RepositoryFactKind::Service,
            RepositoryFactKind::Library,
            RepositoryFactKind::Module,
            RepositoryFactKind::EntryPoint,
            RepositoryFactKind::Build,
            RepositoryFactKind::Configuration,
        ],
        RepositoryTraceKind::Feature => &[
            RepositoryFactKind::EntryPoint,
            RepositoryFactKind::Definition,
            RepositoryFactKind::Interface,
        ],
        RepositoryTraceKind::DataFlow => &[RepositoryFactKind::Call, RepositoryFactKind::Reference],
        RepositoryTraceKind::Authentication | RepositoryTraceKind::Authorization => &[],
        RepositoryTraceKind::Schema => &[RepositoryFactKind::Schema],
        RepositoryTraceKind::Migration => &[RepositoryFactKind::Migration],
        RepositoryTraceKind::Interface => &[RepositoryFactKind::Interface],
        RepositoryTraceKind::Test => &[RepositoryFactKind::Test],
        RepositoryTraceKind::ContinuousIntegration => &[RepositoryFactKind::ContinuousIntegration],
        RepositoryTraceKind::Dependency => &[
            RepositoryFactKind::PackageManager,
            RepositoryFactKind::ExternalDependency,
        ],
    };
    kinds.iter().copied().collect()
}

fn seed_slice(
    index: &DeepRepositoryIndex,
    request: &RepositorySliceRequest,
    history: Option<&RepositoryHistoryView>,
) -> BTreeSet<String> {
    let requested_ids = request
        .fact_ids
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    let requested_paths = request.paths.iter().collect::<BTreeSet<_>>();
    let requested_kinds = request.kinds.iter().copied().collect::<BTreeSet<_>>();
    let history_paths = history
        .into_iter()
        .flat_map(|view| view.observations.iter())
        .flat_map(|observation| observation.changed_paths.iter())
        .collect::<BTreeSet<_>>();
    index
        .facts
        .iter()
        .filter(|fact| {
            requested_ids.contains(fact.fact_id.as_str())
                || requested_kinds.contains(&fact.kind)
                || fact
                    .citations
                    .iter()
                    .any(|citation| requested_paths.contains(&citation.path))
                || (request.dimension == RepositorySliceDimension::Package
                    && matches!(
                        fact.kind,
                        RepositoryFactKind::PackageManager
                            | RepositoryFactKind::Application
                            | RepositoryFactKind::Service
                            | RepositoryFactKind::Library
                    ))
                || (request.dimension == RepositorySliceDimension::EntryPoint
                    && fact.kind == RepositoryFactKind::EntryPoint)
                || (request.dimension == RepositorySliceDimension::DependencyNeighborhood
                    && matches!(
                        fact.kind,
                        RepositoryFactKind::ExternalDependency | RepositoryFactKind::PackageManager
                    ))
                || (request.dimension == RepositorySliceDimension::History
                    && fact
                        .citations
                        .iter()
                        .any(|citation| history_paths.contains(&citation.path)))
        })
        .map(|fact| fact.fact_id.clone())
        .collect()
}

fn validate_slice_request(
    index: &DeepRepositoryIndex,
    request: &RepositorySliceRequest,
    history: Option<&RepositoryHistoryView>,
) -> Result<(), RepositoryDeepViewError> {
    if request.fact_ids.len() > MAX_SLICE_TARGETS
        || request.paths.len() > MAX_SLICE_TARGETS
        || request.kinds.len() > MAX_SLICE_TARGETS
        || request.neighborhood_depth > MAX_NEIGHBORHOOD_DEPTH
        || request.max_facts == 0
        || request.max_facts > MAX_SLICE_FACTS as u64
        || request.fact_ids.windows(2).any(|pair| pair[0] >= pair[1])
        || request.paths.windows(2).any(|pair| pair[0] >= pair[1])
        || request.kinds.windows(2).any(|pair| pair[0] >= pair[1])
        || request
            .paths
            .iter()
            .any(|path| path.workspace_id() != index.facts[0].citations[0].path.workspace_id())
        || (request.dimension == RepositorySliceDimension::History
            && history.is_none_or(|view| {
                view.index_sha256 != index.index_sha256
                    || view.history_sha256 != history_view_digest(view)
            }))
    {
        return Err(RepositoryDeepViewError::SliceInvalid);
    }
    Ok(())
}

fn build_guide(
    index: &DeepRepositoryIndex,
    traces: &RepositoryTraceSet,
    kind: RepositoryLearningGuideKind,
) -> RepositoryLearningGuide {
    let trace_kinds: &[RepositoryTraceKind] = match kind {
        RepositoryLearningGuideKind::Onboarding => &[
            RepositoryTraceKind::Architecture,
            RepositoryTraceKind::Dependency,
            RepositoryTraceKind::Test,
            RepositoryTraceKind::ContinuousIntegration,
        ],
        RepositoryLearningGuideKind::Architecture => &[RepositoryTraceKind::Architecture],
        RepositoryLearningGuideKind::Dependency => &[RepositoryTraceKind::Dependency],
        RepositoryLearningGuideKind::Feature => &[RepositoryTraceKind::Feature],
        RepositoryLearningGuideKind::Test => &[RepositoryTraceKind::Test],
        RepositoryLearningGuideKind::Build => &[
            RepositoryTraceKind::Architecture,
            RepositoryTraceKind::ContinuousIntegration,
        ],
        RepositoryLearningGuideKind::Operations => &[
            RepositoryTraceKind::Architecture,
            RepositoryTraceKind::DataFlow,
            RepositoryTraceKind::Authentication,
            RepositoryTraceKind::Authorization,
        ],
        RepositoryLearningGuideKind::OpenQuestions => &RepositoryTraceKind::ALL,
    };
    let selected_traces = traces
        .traces
        .iter()
        .filter(|trace| trace_kinds.contains(&trace.kind))
        .collect::<Vec<_>>();
    let mut fact_ids = selected_traces
        .iter()
        .flat_map(|trace| trace.fact_ids.iter().cloned())
        .collect::<Vec<_>>();
    fact_ids.sort();
    fact_ids.dedup();
    let mut trace_ids = selected_traces
        .iter()
        .map(|trace| trace.trace_id.clone())
        .collect::<Vec<_>>();
    trace_ids.sort();
    let mut blind_spot_ids = if kind == RepositoryLearningGuideKind::OpenQuestions {
        index
            .blind_spots
            .iter()
            .map(|spot| spot.blind_spot_id.clone())
            .collect::<Vec<_>>()
    } else {
        selected_traces
            .iter()
            .flat_map(|trace| trace.blind_spot_ids.iter().cloned())
            .collect::<Vec<_>>()
    };
    blind_spot_ids.sort();
    blind_spot_ids.dedup();
    let mut guide = RepositoryLearningGuide {
        kind,
        fact_ids,
        trace_ids,
        blind_spot_ids,
        guide_sha256: String::new(),
    };
    guide.guide_sha256 = guide_digest(&guide);
    guide
}

fn validate_index_shape(index: &DeepRepositoryIndex) -> Result<(), RepositoryDeepViewError> {
    if !verify_deep_index_integrity(index)
        || !is_sha256(&index.index_sha256)
        || !is_sha256(&index.map_sha256)
        || index.facts.is_empty()
        || index.facts.iter().any(|fact| {
            !is_sha256(&fact.fact_id) || !is_sha256(&fact.fact_sha256) || fact.citations.is_empty()
        })
        || index
            .blind_spots
            .iter()
            .any(|spot| !is_sha256(&spot.blind_spot_sha256))
    {
        return Err(RepositoryDeepViewError::InvalidInput);
    }
    Ok(())
}

fn validate_history_fields(
    observation: &RepositoryHistoryObservation,
) -> Result<(), RepositoryDeepViewError> {
    if !valid_commit(&observation.commit_id)
        || observation.parent_ids.len() > 64
        || observation
            .parent_ids
            .iter()
            .any(|parent| !valid_commit(parent))
        || observation
            .parent_ids
            .windows(2)
            .any(|pair| pair[0] >= pair[1])
        || observation.changed_paths.len() > MAX_HISTORY_PATHS
        || observation
            .changed_paths
            .windows(2)
            .any(|pair| pair[0] >= pair[1])
    {
        return Err(RepositoryDeepViewError::HistoryInvalid);
    }
    Ok(())
}

fn valid_commit(value: &str) -> bool {
    matches!(value.len(), 40 | 64)
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn trace_digest(trace: &RepositoryTrace) -> String {
    sha256_json(&(
        &trace.trace_id,
        trace.kind,
        trace.state,
        &trace.fact_ids,
        &trace.blind_spot_ids,
    ))
}

fn trace_set_digest(set: &RepositoryTraceSet) -> String {
    sha256_json(&(set.schema_version, &set.index_sha256, &set.traces))
}

fn history_observation_digest(observation: &RepositoryHistoryObservation) -> String {
    sha256_json(&(
        &observation.commit_id,
        &observation.parent_ids,
        &observation.changed_paths,
        observation.sequence,
    ))
}

fn history_view_digest(view: &RepositoryHistoryView) -> String {
    sha256_json(&(
        view.schema_version,
        &view.index_sha256,
        &view.branch,
        &view.current_commit_id,
        &view.observations,
        view.current_commit_observed,
        view.truncated,
    ))
}

fn documentation_drift_digest(record: &RepositoryDocumentationDrift) -> String {
    sha256_json(&(
        &record.drift_id,
        &record.documentation,
        &record.implementation,
        record.state,
        &record.checker_sha256,
    ))
}

fn glossary_digest(term: &RepositoryGlossaryTerm) -> String {
    sha256_json(&(&term.term, &term.fact_ids, term.state))
}

fn portfolio_digest(view: &RepositoryPortfolioView) -> String {
    sha256_json(&(
        view.schema_version,
        &view.repositories,
        view.cross_repository_relationships_observed,
    ))
}

fn slice_digest(slice: &RepositoryAnalysisSlice) -> String {
    sha256_json(&(
        slice.schema_version,
        &slice.index_sha256,
        &slice.request_sha256,
        &slice.facts,
        &slice.blind_spots,
        &slice.coverage,
        slice.truncated,
    ))
}

fn guide_digest(guide: &RepositoryLearningGuide) -> String {
    sha256_json(&(
        guide.kind,
        &guide.fact_ids,
        &guide.trace_ids,
        &guide.blind_spot_ids,
    ))
}

fn export_digest(export: &RepositoryLearningExport) -> String {
    sha256_json(&(
        export.schema_version,
        &export.index_sha256,
        &export.trace_set_sha256,
        &export.guides,
    ))
}

fn sha256_json<T: Serialize>(value: &T) -> String {
    serde_json::to_vec(value)
        .map(|bytes| sha256_hex(&bytes))
        .unwrap_or_else(|_| sha256_hex(b"repository-deep-view-serialization-failed"))
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

    fn file(path: &[&str], source: &str) -> RepositoryFileInput {
        RepositoryFileInput {
            path: path.iter().map(|value| (*value).to_owned()).collect(),
            size_bytes: source.len() as u64,
            content_sha256: sha256_hex(source.as_bytes()),
            content: Some(source.as_bytes().to_vec()),
            git_state: GitTrackedState::TrackedClean,
            policy_excluded: false,
            generated: false,
            vendored: false,
        }
    }

    fn index() -> DeepRepositoryIndex {
        let map = build_repository_map(RepositoryMapInput {
            workspace_id: WorkspaceId::from_raw("workspace-views"),
            repository_sha256: "a".repeat(64),
            worktree_sha256: "b".repeat(64),
            branch: Some("agentmage/tasks/views".to_owned()),
            commit_id: "c".repeat(40),
            policy_sha256: "d".repeat(64),
            freshness_sha256: "e".repeat(64),
            files: vec![
                file(&["Cargo.toml"], "[package]\nname='views'\n"),
                file(&["src", "main.rs"], "use std::fmt;\nfn main() {}\n"),
                file(&["tests", "view_test.rs"], "fn view_works() {}\n"),
            ],
        })
        .expect("map");
        build_deep_repository_index(&map, Vec::new(), Vec::new()).expect("index")
    }

    #[test]
    fn traces_exports_and_glossary_are_deterministic_and_unknowns_remain_visible() {
        let index = index();
        let traces = build_repository_traces(&index).expect("traces");
        assert!(verify_repository_traces(&index, &traces));
        assert_eq!(traces.traces.len(), RepositoryTraceKind::ALL.len());
        assert!(traces.traces.iter().any(|trace| {
            trace.kind == RepositoryTraceKind::Authentication
                && trace.state == RepositoryFactState::UnknownBlocked
                && trace.fact_ids.is_empty()
        }));
        assert!(traces.traces.iter().any(|trace| {
            trace.kind == RepositoryTraceKind::Feature
                && trace.state == RepositoryFactState::Inferred
        }));
        let glossary = build_repository_glossary(&index).expect("glossary");
        assert!(!glossary.is_empty());
        assert!(
            glossary
                .iter()
                .all(|term| term.state == RepositoryFactState::Derived)
        );
        let export = build_repository_learning_export(&index, &traces).expect("export");
        assert_eq!(export.guides.len(), RepositoryLearningGuideKind::ALL.len());
        assert!(export.guides.iter().any(|guide| {
            guide.kind == RepositoryLearningGuideKind::OpenQuestions
                && !guide.blind_spot_ids.is_empty()
        }));
    }

    #[test]
    fn history_and_portfolio_preserve_exact_branch_commit_and_repository_boundaries() {
        let index = index();
        let path = index.facts[0].citations[0].path.clone();
        let observation = RepositoryHistoryObservation {
            commit_id: index.git.commit_id.clone(),
            parent_ids: vec!["1".repeat(40)],
            changed_paths: vec![path],
            sequence: 1,
            observation_sha256: String::new(),
        }
        .seal()
        .expect("history observation");
        let history =
            build_repository_history_view(&index, vec![observation], false).expect("history view");
        assert!(history.current_commit_observed);
        assert_eq!(history.branch, index.git.branch);
        let portfolio =
            build_repository_portfolio(std::slice::from_ref(&index)).expect("portfolio");
        assert_eq!(portfolio.repositories.len(), 1);
        assert!(!portfolio.cross_repository_relationships_observed);
        assert_eq!(
            build_repository_portfolio(&[index.clone(), index]),
            Err(RepositoryDeepViewError::InvalidInput)
        );
    }

    #[test]
    fn bounded_slices_report_omissions_and_stale_history_or_requests_fail_closed() {
        let index = index();
        let request = RepositorySliceRequest {
            dimension: RepositorySliceDimension::EntryPoint,
            fact_ids: Vec::new(),
            paths: Vec::new(),
            kinds: Vec::new(),
            neighborhood_depth: 1,
            max_facts: 1,
        };
        let slice = slice_deep_repository_index(&index, &request, None).expect("slice");
        assert!(verify_repository_slice(&index, &request, None, &slice));
        assert_eq!(slice.coverage.returned_facts, 1);
        assert_eq!(slice.truncated, slice.coverage.omitted_facts > 0);
        assert_eq!(slice.blind_spots, index.blind_spots);

        let mut invalid = request;
        invalid.neighborhood_depth = MAX_NEIGHBORHOOD_DEPTH + 1;
        assert_eq!(
            slice_deep_repository_index(&index, &invalid, None),
            Err(RepositoryDeepViewError::SliceInvalid)
        );
    }

    #[test]
    fn documentation_drift_requires_current_citations_and_exact_checker_evidence() {
        let index = index();
        let citations = index
            .facts
            .iter()
            .flat_map(|fact| fact.citations.iter())
            .cloned()
            .collect::<Vec<_>>();
        let record = RepositoryDocumentationDrift {
            drift_id: String::new(),
            documentation: citations[0].clone(),
            implementation: vec![citations[1].clone()],
            state: RepositoryDocumentationDriftState::Current,
            checker_sha256: Some("9".repeat(64)),
            drift_sha256: String::new(),
        };
        let sealed = seal_documentation_drift(&index, record.clone()).expect("drift record");
        assert_eq!(sealed.state, RepositoryDocumentationDriftState::Current);
        let mut unsupported = record;
        unsupported.checker_sha256 = None;
        assert_eq!(
            seal_documentation_drift(&index, unsupported),
            Err(RepositoryDeepViewError::DocumentationInvalid)
        );
    }
}
