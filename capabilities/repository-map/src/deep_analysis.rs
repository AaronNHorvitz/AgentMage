//! Deterministic deep repository facts with exact citations and visible blind spots.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write;

use agentmage_kernel_contracts::WorkspacePath;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{
    RepositoryEntryDisposition, RepositoryGitIdentity, RepositoryLanguage, RepositoryMap,
    SourceRange, StructuralItemKind, StructuralSourceResolution, grammar_descriptor,
    resolve_structural_records, verify_repository_map,
};

const ANALYSIS_SCHEMA_VERSION: u16 = 1;
const RULE_SET_VERSION: &str = "repository-deep-analysis-rules-v1";
const MAX_ADAPTERS: usize = 32;
const MAX_ADAPTER_FACTS: usize = 100_000;
const MAX_FACT_LABEL_BYTES: usize = 2_048;
const MAX_CITATIONS_PER_FACT: usize = 256;

/// Evidence strength for one repository fact or trace.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RepositoryFactState {
    /// Direct parser or bounded-adapter observation.
    Observed,
    /// Deterministic rule output from one or more observed facts.
    Derived,
    /// Explicit interpretation that must not be presented as observed structure.
    Inferred,
    /// Required relationship could not be established from admitted evidence.
    UnknownBlocked,
}

/// Closed read-adapter implementation classes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RepositoryAdapterKind {
    /// Pinned syntax parser operating on admitted bytes.
    Parser,
    /// Separately confined language-native read tool.
    LanguageNative,
    /// Separately confined language-server read endpoint.
    LanguageServer,
}

/// Closed semantic capabilities that a read adapter may report.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RepositoryAdapterCapability {
    /// Definition locations.
    Definitions,
    /// Reference locations.
    References,
    /// Call relationships.
    Calls,
    /// Type relationships.
    Types,
    /// Diagnostics.
    Diagnostics,
    /// Symbol inventory.
    Symbols,
}

/// Exact, authority-free description of one separately confined read adapter.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RepositoryAnalysisAdapter {
    /// Stable adapter identity.
    pub adapter_id: String,
    /// Closed adapter class.
    pub kind: RepositoryAdapterKind,
    /// Optional exact supported language.
    pub language: Option<RepositoryLanguage>,
    /// Exact implementation version.
    pub version: String,
    /// Digest of the pinned executable or in-process implementation.
    pub implementation_sha256: String,
    /// Stable sorted capabilities.
    pub capabilities: Vec<RepositoryAdapterCapability>,
    /// Adapter is restricted to read observations.
    pub read_only: bool,
    /// Adapter cannot contact a network.
    pub network_allowed: bool,
    /// Adapter cannot execute repository-selected programs.
    pub repository_execution_allowed: bool,
    /// SHA-256 over every preceding field.
    pub adapter_sha256: String,
}

impl RepositoryAnalysisAdapter {
    /// Seals one exact adapter descriptor after fail-closed validation.
    pub fn seal(mut self) -> Result<Self, RepositoryDeepAnalysisError> {
        self.adapter_sha256.clear();
        validate_adapter_fields(&self)?;
        self.adapter_sha256 = adapter_digest(&self);
        Ok(self)
    }

    /// Verifies the descriptor and its authority restrictions.
    #[must_use]
    pub fn verify(&self) -> bool {
        validate_adapter_fields(self).is_ok() && self.adapter_sha256 == adapter_digest(self)
    }
}

/// Closed deep-repository fact classes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RepositoryFactKind {
    /// Supported implementation language.
    Language,
    /// Framework identified by an exact conventional artifact.
    Framework,
    /// Package manager identified by an exact manifest or lockfile.
    PackageManager,
    /// Executable entry point.
    EntryPoint,
    /// Test source or test root.
    Test,
    /// Build-system artifact.
    Build,
    /// Linter or formatter artifact.
    Linter,
    /// Type-system configuration.
    TypeSystem,
    /// Repository instruction document, treated as untrusted data.
    Instruction,
    /// Documentation artifact.
    Documentation,
    /// Application boundary.
    Application,
    /// Service or deployable boundary.
    Service,
    /// Library boundary.
    Library,
    /// Configuration artifact.
    Configuration,
    /// Script artifact.
    Script,
    /// Generated-code boundary.
    GeneratedCode,
    /// External package declaration or import.
    ExternalDependency,
    /// Parser-backed module.
    Module,
    /// Parser- or adapter-backed definition.
    Definition,
    /// Adapter-backed reference.
    Reference,
    /// Adapter-backed call relationship.
    Call,
    /// Adapter-backed type relationship.
    Type,
    /// Adapter-backed diagnostic.
    Diagnostic,
    /// Parser- or adapter-backed symbol.
    Symbol,
    /// Schema artifact.
    Schema,
    /// Migration artifact.
    Migration,
    /// Interface or protocol definition.
    Interface,
    /// Continuous-integration artifact.
    ContinuousIntegration,
    /// Exact Git-history observation.
    GitHistory,
    /// Explicit documentation-drift observation.
    DocumentationDrift,
    /// Deterministically derived glossary term.
    Glossary,
}

/// Exact current source citation for one fact.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RepositoryFactCitation {
    /// Canonical workspace-relative path.
    pub path: WorkspacePath,
    /// Exact complete file content digest.
    pub content_sha256: String,
    /// Optional exact source range.
    pub range: Option<SourceRange>,
    /// Digest of exact syntax bytes when a parser supplied the range.
    pub syntax_sha256: Option<String>,
    /// Stable deterministic method or exact adapter identity.
    pub method_id: String,
    /// Optional exact grammar descriptor identity.
    pub grammar_sha256: Option<String>,
    /// Optional exact parser or adapter version.
    pub parser_version: Option<String>,
    /// Exact repository, worktree, branch, and commit identity.
    pub git: RepositoryGitIdentity,
    /// SHA-256 over every preceding field.
    pub citation_sha256: String,
}

impl RepositoryFactCitation {
    /// Seals one citation after validation against the exact current map.
    pub fn seal(
        map: &RepositoryMap,
        mut citation: Self,
    ) -> Result<Self, RepositoryDeepAnalysisError> {
        citation.citation_sha256.clear();
        if !citation_fields_valid(map, &citation) {
            return Err(RepositoryDeepAnalysisError::CitationInvalid);
        }
        citation.citation_sha256 = citation_digest(&citation);
        Ok(citation)
    }
}

/// One externally produced fact from an exact confined adapter.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RepositoryAdapterFactInput {
    /// Exact registered adapter identity.
    pub adapter_id: String,
    /// Capability that produced this fact.
    pub capability: RepositoryAdapterCapability,
    /// Closed resulting fact kind.
    pub kind: RepositoryFactKind,
    /// Bounded content-safe label; never executable instruction text.
    pub label: String,
    /// Evidence strength, limited to observed or derived.
    pub state: RepositoryFactState,
    /// One or more exact current citations.
    pub citations: Vec<RepositoryFactCitation>,
}

/// One immutable deep repository fact.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RepositoryAnalysisFact {
    /// Content-addressed fact identity.
    pub fact_id: String,
    /// Closed fact class.
    pub kind: RepositoryFactKind,
    /// Bounded exact label.
    pub label: String,
    /// Explicit evidence strength.
    pub state: RepositoryFactState,
    /// Sorted unique current citations.
    pub citations: Vec<RepositoryFactCitation>,
    /// Optional exact adapter descriptor identity.
    pub adapter_sha256: Option<String>,
    /// Workspace content can never grant authority.
    pub untrusted_repository_data: bool,
    /// SHA-256 over every preceding field.
    pub fact_sha256: String,
}

/// Closed reason that a relationship or region is not understood.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RepositoryBlindSpotCode {
    /// No pinned parser supports this file.
    UnsupportedLanguage,
    /// File content was not admitted.
    ContentNotRead,
    /// Binary content was inventoried only.
    BinaryInventoryOnly,
    /// File exceeded parser limits.
    ParseLimitExceeded,
    /// Parser failed.
    ParseFailed,
    /// Parser or traversal was incomplete.
    TruncatedOrSyntaxErrors,
    /// No admitted adapter covers this semantic capability.
    AdapterCapabilityUnavailable,
    /// Git history was not supplied.
    HistoryUnavailable,
    /// Cross-repository evidence was not supplied.
    CrossRepositoryUnavailable,
}

/// Explicit Unknown/Blocked entry retained in the index.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RepositoryBlindSpot {
    /// Stable content-free blind-spot identity.
    pub blind_spot_id: String,
    /// Closed limitation class.
    pub code: RepositoryBlindSpotCode,
    /// Optional exact affected path.
    pub path: Option<WorkspacePath>,
    /// Optional missing semantic capability.
    pub capability: Option<RepositoryAdapterCapability>,
    /// Missing evidence is always explicit.
    pub state: RepositoryFactState,
    /// SHA-256 over every preceding field.
    pub blind_spot_sha256: String,
}

/// Quantified deep-analysis coverage without a whole-repository overclaim.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RepositoryDeepCoverage {
    /// Base repository files discovered.
    pub discovered_files: u64,
    /// Files read by the base map.
    pub read_files: u64,
    /// Files parsed by pinned grammars.
    pub parsed_files: u64,
    /// Exact deep facts retained.
    pub indexed_facts: u64,
    /// Parser/path-rule facts retained.
    pub deterministic_facts: u64,
    /// Confined-adapter facts retained.
    pub adapter_facts: u64,
    /// Explicit blind spots retained.
    pub blind_spots: u64,
    /// Supported semantic adapter capabilities.
    pub supported_adapter_capabilities: Vec<RepositoryAdapterCapability>,
    /// Missing semantic adapter capabilities.
    pub missing_adapter_capabilities: Vec<RepositoryAdapterCapability>,
    /// True only when no file or semantic coverage is omitted.
    pub whole_repository_claim_permitted: bool,
}

/// Complete deterministic deep repository index for one exact map revision.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeepRepositoryIndex {
    /// Index schema version.
    pub schema_version: u16,
    /// Exact source repository-map identity.
    pub map_sha256: String,
    /// Exact branch-aware Git identity.
    pub git: RepositoryGitIdentity,
    /// Exact deterministic profile rule-set identity.
    pub rule_set_sha256: String,
    /// Stable verified adapters in identity order.
    pub adapters: Vec<RepositoryAnalysisAdapter>,
    /// Stable facts in fact-identity order.
    pub facts: Vec<RepositoryAnalysisFact>,
    /// Stable explicit blind spots.
    pub blind_spots: Vec<RepositoryBlindSpot>,
    /// Complete deep-analysis coverage ledger.
    pub coverage: RepositoryDeepCoverage,
    /// SHA-256 over every preceding field.
    pub index_sha256: String,
}

/// Content-free deep-analysis failure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RepositoryDeepAnalysisError {
    /// Source map or bounded collection is invalid.
    InvalidInput,
    /// Adapter authority, identity, or capability is invalid.
    AdapterInvalid,
    /// Citation is stale, forged, or malformed.
    CitationInvalid,
    /// Fact kind and adapter capability disagree.
    FactInvalid,
}

impl RepositoryDeepAnalysisError {
    /// Stable content-free error code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidInput => "repository.deep.input_invalid",
            Self::AdapterInvalid => "repository.deep.adapter_invalid",
            Self::CitationInvalid => "repository.deep.citation_invalid",
            Self::FactInvalid => "repository.deep.fact_invalid",
        }
    }
}

/// Builds one deterministic deep index from a verified map and confined observations.
pub fn build_deep_repository_index(
    map: &RepositoryMap,
    mut adapters: Vec<RepositoryAnalysisAdapter>,
    adapter_facts: Vec<RepositoryAdapterFactInput>,
) -> Result<DeepRepositoryIndex, RepositoryDeepAnalysisError> {
    if !verify_repository_map(map)
        || adapters.len() > MAX_ADAPTERS
        || adapter_facts.len() > MAX_ADAPTER_FACTS
    {
        return Err(RepositoryDeepAnalysisError::InvalidInput);
    }
    adapters.sort_by(|left, right| left.adapter_id.cmp(&right.adapter_id));
    if adapters.iter().any(|adapter| !adapter.verify())
        || adapters
            .windows(2)
            .any(|pair| pair[0].adapter_id == pair[1].adapter_id)
    {
        return Err(RepositoryDeepAnalysisError::AdapterInvalid);
    }
    let adapter_index = adapters
        .iter()
        .map(|adapter| (adapter.adapter_id.as_str(), adapter))
        .collect::<BTreeMap<_, _>>();
    let git = map_git_identity(map);
    let mut grouped = BTreeMap::<
        (
            RepositoryFactKind,
            String,
            RepositoryFactState,
            Option<String>,
        ),
        Vec<RepositoryFactCitation>,
    >::new();
    collect_path_profile_facts(map, &git, &mut grouped)?;
    collect_parser_facts(map, &mut grouped)?;
    let deterministic_fact_count = grouped.len() as u64;

    for input in adapter_facts {
        let adapter = adapter_index
            .get(input.adapter_id.as_str())
            .ok_or(RepositoryDeepAnalysisError::AdapterInvalid)?;
        if !adapter.capabilities.contains(&input.capability)
            || !capability_matches_fact(input.capability, input.kind)
            || !matches!(
                input.state,
                RepositoryFactState::Observed | RepositoryFactState::Derived
            )
            || !valid_label(&input.label)
            || input.citations.is_empty()
            || input.citations.len() > MAX_CITATIONS_PER_FACT
            || input.citations.iter().any(|citation| {
                citation.method_id != adapter.adapter_id || !verify_fact_citation(map, citation)
            })
        {
            return Err(RepositoryDeepAnalysisError::FactInvalid);
        }
        grouped
            .entry((
                input.kind,
                input.label,
                input.state,
                Some(adapter.adapter_sha256.clone()),
            ))
            .or_default()
            .extend(input.citations);
    }
    let mut facts = grouped
        .into_iter()
        .map(|((kind, label, state, adapter_sha256), citations)| {
            seal_fact(kind, label, state, citations, adapter_sha256)
        })
        .collect::<Result<Vec<_>, _>>()?;
    facts.sort_by(|left, right| left.fact_id.cmp(&right.fact_id));

    let supported = adapters
        .iter()
        .flat_map(|adapter| adapter.capabilities.iter().copied())
        .collect::<BTreeSet<_>>();
    let missing = all_adapter_capabilities()
        .difference(&supported)
        .copied()
        .collect::<Vec<_>>();
    let mut blind_spots = collect_file_blind_spots(map);
    blind_spots.extend(missing.iter().copied().map(capability_blind_spot));
    blind_spots.push(general_blind_spot(
        RepositoryBlindSpotCode::HistoryUnavailable,
    ));
    blind_spots.push(general_blind_spot(
        RepositoryBlindSpotCode::CrossRepositoryUnavailable,
    ));
    blind_spots.sort_by(|left, right| left.blind_spot_id.cmp(&right.blind_spot_id));
    blind_spots.dedup_by(|left, right| left.blind_spot_id == right.blind_spot_id);

    let coverage = RepositoryDeepCoverage {
        discovered_files: map.coverage.discovered_files,
        read_files: map.coverage.read_files,
        parsed_files: map.coverage.parsed_files,
        indexed_facts: facts.len() as u64,
        deterministic_facts: deterministic_fact_count,
        adapter_facts: facts
            .len()
            .saturating_sub(deterministic_fact_count as usize) as u64,
        blind_spots: blind_spots.len() as u64,
        supported_adapter_capabilities: supported.iter().copied().collect(),
        missing_adapter_capabilities: missing,
        whole_repository_claim_permitted: false,
    };
    let mut index = DeepRepositoryIndex {
        schema_version: ANALYSIS_SCHEMA_VERSION,
        map_sha256: map.map_sha256.clone(),
        git,
        rule_set_sha256: sha256_hex(RULE_SET_VERSION.as_bytes()),
        adapters,
        facts,
        blind_spots,
        coverage,
        index_sha256: String::new(),
    };
    index.index_sha256 = index_digest(&index);
    Ok(index)
}

/// Verifies an index by deterministic recomputation against the exact current map.
#[must_use]
pub fn verify_deep_repository_index(map: &RepositoryMap, index: &DeepRepositoryIndex) -> bool {
    if !verify_deep_index_integrity(index)
        || index.schema_version != ANALYSIS_SCHEMA_VERSION
        || index.map_sha256 != map.map_sha256
        || index.git != map_git_identity(map)
        || index.rule_set_sha256 != sha256_hex(RULE_SET_VERSION.as_bytes())
        || index
            .facts
            .iter()
            .any(|fact| !verify_fact(map, index, fact))
        || index
            .blind_spots
            .iter()
            .any(|blind_spot| !verify_blind_spot(map, blind_spot))
    {
        return false;
    }
    let inputs = index
        .facts
        .iter()
        .filter_map(|fact| {
            let adapter_sha256 = fact.adapter_sha256.as_deref()?;
            let adapter = index
                .adapters
                .iter()
                .find(|candidate| candidate.adapter_sha256 == adapter_sha256)?;
            let capability = capability_for_fact(fact.kind)?;
            Some(RepositoryAdapterFactInput {
                adapter_id: adapter.adapter_id.clone(),
                capability,
                kind: fact.kind,
                label: fact.label.clone(),
                state: fact.state,
                citations: fact.citations.clone(),
            })
        })
        .collect::<Vec<_>>();
    build_deep_repository_index(map, index.adapters.clone(), inputs)
        .is_ok_and(|expected| expected == *index)
}

pub(crate) fn verify_deep_index_integrity(index: &DeepRepositoryIndex) -> bool {
    let supported = index
        .adapters
        .iter()
        .flat_map(|adapter| adapter.capabilities.iter().copied())
        .collect::<BTreeSet<_>>();
    let missing = all_adapter_capabilities()
        .difference(&supported)
        .copied()
        .collect::<Vec<_>>();
    let adapter_facts = index
        .facts
        .iter()
        .filter(|fact| fact.adapter_sha256.is_some())
        .count() as u64;
    index.schema_version == ANALYSIS_SCHEMA_VERSION
        && index.rule_set_sha256 == sha256_hex(RULE_SET_VERSION.as_bytes())
        && index.index_sha256 == index_digest(index)
        && !index.coverage.whole_repository_claim_permitted
        && index.coverage.indexed_facts == index.facts.len() as u64
        && index.coverage.adapter_facts == adapter_facts
        && index.coverage.deterministic_facts + adapter_facts == index.facts.len() as u64
        && index.coverage.blind_spots == index.blind_spots.len() as u64
        && index.coverage.supported_adapter_capabilities
            == supported.iter().copied().collect::<Vec<_>>()
        && index.coverage.missing_adapter_capabilities == missing
        && index.adapters.iter().all(RepositoryAnalysisAdapter::verify)
        && index
            .adapters
            .windows(2)
            .all(|pair| pair[0].adapter_id < pair[1].adapter_id)
        && index
            .facts
            .windows(2)
            .all(|pair| pair[0].fact_id < pair[1].fact_id)
        && index
            .blind_spots
            .windows(2)
            .all(|pair| pair[0].blind_spot_id < pair[1].blind_spot_id)
        && index.facts.iter().all(|fact| {
            valid_label(&fact.label)
                && fact.untrusted_repository_data
                && !fact.citations.is_empty()
                && fact
                    .citations
                    .windows(2)
                    .all(|pair| pair[0].citation_sha256 < pair[1].citation_sha256)
                && fact.citations.iter().all(|citation| {
                    citation.git == index.git
                        && citation.citation_sha256 == citation_digest(citation)
                })
                && fact.adapter_sha256.as_ref().is_none_or(|identity| {
                    index
                        .adapters
                        .iter()
                        .any(|adapter| &adapter.adapter_sha256 == identity)
                })
                && fact.fact_id
                    == sha256_json(&(
                        fact.kind,
                        &fact.label,
                        fact.state,
                        &fact.citations,
                        &fact.adapter_sha256,
                    ))
                && fact.fact_sha256 == fact_digest(fact)
        })
        && index.blind_spots.iter().all(|spot| {
            spot.state == RepositoryFactState::UnknownBlocked
                && spot.blind_spot_id == sha256_json(&(spot.code, &spot.path, spot.capability))
                && spot.blind_spot_sha256 == blind_spot_digest(spot)
        })
}

/// Verifies one exact citation against the current map revision.
#[must_use]
pub fn verify_fact_citation(map: &RepositoryMap, citation: &RepositoryFactCitation) -> bool {
    citation_fields_valid(map, citation) && citation.citation_sha256 == citation_digest(citation)
}

fn collect_path_profile_facts(
    map: &RepositoryMap,
    git: &RepositoryGitIdentity,
    grouped: &mut BTreeMap<
        (
            RepositoryFactKind,
            String,
            RepositoryFactState,
            Option<String>,
        ),
        Vec<RepositoryFactCitation>,
    >,
) -> Result<(), RepositoryDeepAnalysisError> {
    for file in &map.files {
        let path = path_text(&file.path);
        let lower = path.to_ascii_lowercase();
        let base = lower.rsplit('/').next().unwrap_or(&lower);
        let citation = RepositoryFactCitation::seal(
            map,
            RepositoryFactCitation {
                path: file.path.clone(),
                content_sha256: file.content_sha256.clone(),
                range: None,
                syntax_sha256: None,
                method_id: RULE_SET_VERSION.to_owned(),
                grammar_sha256: None,
                parser_version: None,
                git: git.clone(),
                citation_sha256: String::new(),
            },
        )?;
        if let Some(language) = file.language {
            add_grouped(
                grouped,
                RepositoryFactKind::Language,
                language.id(),
                RepositoryFactState::Derived,
                citation.clone(),
            );
        }
        for (kind, label) in classify_path(&lower, base, file.disposition) {
            add_grouped(
                grouped,
                kind,
                label,
                RepositoryFactState::Derived,
                citation.clone(),
            );
        }
    }
    Ok(())
}

fn collect_parser_facts(
    map: &RepositoryMap,
    grouped: &mut BTreeMap<
        (
            RepositoryFactKind,
            String,
            RepositoryFactState,
            Option<String>,
        ),
        Vec<RepositoryFactCitation>,
    >,
) -> Result<(), RepositoryDeepAnalysisError> {
    for resolution in resolve_structural_records(map) {
        let kind = match resolution.kind {
            StructuralItemKind::Module => RepositoryFactKind::Module,
            StructuralItemKind::Import => RepositoryFactKind::ExternalDependency,
            StructuralItemKind::Interface | StructuralItemKind::Trait => {
                RepositoryFactKind::Interface
            }
            _ => RepositoryFactKind::Definition,
        };
        let citation = citation_from_resolution(map, &resolution)?;
        add_grouped(
            grouped,
            kind,
            &resolution.name,
            RepositoryFactState::Observed,
            citation.clone(),
        );
        if !matches!(
            resolution.kind,
            StructuralItemKind::Module | StructuralItemKind::Import
        ) {
            add_grouped(
                grouped,
                RepositoryFactKind::Symbol,
                &resolution.name,
                RepositoryFactState::Observed,
                citation,
            );
        }
    }
    Ok(())
}

fn classify_path(
    path: &str,
    base: &str,
    disposition: RepositoryEntryDisposition,
) -> Vec<(RepositoryFactKind, &'static str)> {
    let mut facts = Vec::new();
    match base {
        "cargo.toml" | "cargo.lock" => {
            facts.push((RepositoryFactKind::PackageManager, "cargo"));
            facts.push((RepositoryFactKind::Build, "cargo"));
        }
        "package.json" | "package-lock.json" => {
            facts.push((RepositoryFactKind::PackageManager, "npm"));
            facts.push((RepositoryFactKind::Build, "npm"));
        }
        "pnpm-lock.yaml" => facts.push((RepositoryFactKind::PackageManager, "pnpm")),
        "yarn.lock" => facts.push((RepositoryFactKind::PackageManager, "yarn")),
        "pyproject.toml" | "poetry.lock" => {
            facts.push((RepositoryFactKind::PackageManager, "python-project"));
            facts.push((RepositoryFactKind::Build, "python-project"));
        }
        "requirements.txt" => facts.push((RepositoryFactKind::PackageManager, "pip")),
        "package.swift" => {
            facts.push((RepositoryFactKind::PackageManager, "swift-package-manager"));
            facts.push((RepositoryFactKind::Build, "swift-package-manager"));
        }
        "makefile" | "justfile" | "build.rs" | "dockerfile" => {
            facts.push((RepositoryFactKind::Build, "conventional-build-artifact"));
        }
        _ => {}
    }
    if matches!(
        base,
        "main.rs" | "main.py" | "app.py" | "main.swift" | "index.ts" | "index.js"
    ) {
        facts.push((RepositoryFactKind::EntryPoint, "conventional-entry-point"));
        facts.push((RepositoryFactKind::Application, "application-entry"));
    }
    if path.contains("/tests/")
        || path.starts_with("tests/")
        || base.starts_with("test_")
        || base.ends_with("_test.rs")
        || base.ends_with(".test.ts")
        || base.ends_with(".test.js")
    {
        facts.push((RepositoryFactKind::Test, "test-source"));
    }
    if path.starts_with(".github/workflows/")
        || base == ".gitlab-ci.yml"
        || base == "azure-pipelines.yml"
    {
        facts.push((
            RepositoryFactKind::ContinuousIntegration,
            "ci-configuration",
        ));
    }
    if path.contains("migration") {
        facts.push((RepositoryFactKind::Migration, "migration-artifact"));
    }
    if path.contains("schema") || base.ends_with(".sql") {
        facts.push((RepositoryFactKind::Schema, "schema-artifact"));
    }
    if matches!(base, "agents.md" | "claude.md" | "copilot-instructions.md")
        || path.contains("instructions")
    {
        facts.push((
            RepositoryFactKind::Instruction,
            "untrusted-repository-instructions",
        ));
    } else if base.ends_with(".md") || path.starts_with("docs/") || path.contains("/docs/") {
        facts.push((RepositoryFactKind::Documentation, "documentation-artifact"));
    }
    if matches!(
        base,
        "clippy.toml" | "rustfmt.toml" | ".eslintrc" | "eslint.config.js" | "ruff.toml"
    ) {
        facts.push((RepositoryFactKind::Linter, "lint-or-format-configuration"));
    }
    if matches!(base, "tsconfig.json" | "mypy.ini" | "pyrightconfig.json") {
        facts.push((RepositoryFactKind::TypeSystem, "type-system-configuration"));
    }
    if base.ends_with(".sh") || path.starts_with("scripts/") || path.contains("/scripts/") {
        facts.push((RepositoryFactKind::Script, "repository-script"));
    }
    if matches!(base, "lib.rs" | "__init__.py") || path.starts_with("sources/") {
        facts.push((RepositoryFactKind::Library, "library-boundary"));
    }
    if matches!(base, "dockerfile" | "compose.yaml" | "compose.yml")
        || path.starts_with("services/")
    {
        facts.push((RepositoryFactKind::Service, "service-boundary"));
    }
    if base.starts_with("next.config") {
        facts.push((RepositoryFactKind::Framework, "nextjs"));
    } else if base.starts_with("vite.config") {
        facts.push((RepositoryFactKind::Framework, "vite"));
    }
    if is_configuration(base) {
        facts.push((RepositoryFactKind::Configuration, "configuration-artifact"));
    }
    if disposition == RepositoryEntryDisposition::GeneratedExcluded {
        facts.push((RepositoryFactKind::GeneratedCode, "generated-code-boundary"));
    }
    facts
}

fn is_configuration(base: &str) -> bool {
    matches!(
        base.rsplit_once('.').map(|(_, extension)| extension),
        Some("json" | "toml" | "yaml" | "yml" | "ini")
    ) || base.starts_with('.')
}

fn citation_from_resolution(
    map: &RepositoryMap,
    resolution: &StructuralSourceResolution,
) -> Result<RepositoryFactCitation, RepositoryDeepAnalysisError> {
    RepositoryFactCitation::seal(
        map,
        RepositoryFactCitation {
            path: resolution.path.clone(),
            content_sha256: resolution.content_sha256.clone(),
            range: Some(resolution.range),
            syntax_sha256: Some(resolution.syntax_sha256.clone()),
            method_id: "pinned-tree-sitter-v1".to_owned(),
            grammar_sha256: Some(resolution.grammar_sha256.clone()),
            parser_version: Some(resolution.parser_version.clone()),
            git: resolution.git.clone(),
            citation_sha256: String::new(),
        },
    )
}

fn add_grouped(
    grouped: &mut BTreeMap<
        (
            RepositoryFactKind,
            String,
            RepositoryFactState,
            Option<String>,
        ),
        Vec<RepositoryFactCitation>,
    >,
    kind: RepositoryFactKind,
    label: &str,
    state: RepositoryFactState,
    citation: RepositoryFactCitation,
) {
    grouped
        .entry((kind, label.to_owned(), state, None))
        .or_default()
        .push(citation);
}

fn seal_fact(
    kind: RepositoryFactKind,
    label: String,
    state: RepositoryFactState,
    mut citations: Vec<RepositoryFactCitation>,
    adapter_sha256: Option<String>,
) -> Result<RepositoryAnalysisFact, RepositoryDeepAnalysisError> {
    if !valid_label(&label) || citations.is_empty() || citations.len() > MAX_CITATIONS_PER_FACT {
        return Err(RepositoryDeepAnalysisError::FactInvalid);
    }
    citations.sort_by(|left, right| left.citation_sha256.cmp(&right.citation_sha256));
    citations.dedup_by(|left, right| left.citation_sha256 == right.citation_sha256);
    let fact_id = sha256_json(&(kind, &label, state, &citations, &adapter_sha256));
    let mut fact = RepositoryAnalysisFact {
        fact_id,
        kind,
        label,
        state,
        citations,
        adapter_sha256,
        untrusted_repository_data: true,
        fact_sha256: String::new(),
    };
    fact.fact_sha256 = fact_digest(&fact);
    Ok(fact)
}

fn collect_file_blind_spots(map: &RepositoryMap) -> Vec<RepositoryBlindSpot> {
    map.files
        .iter()
        .filter_map(|file| {
            let code = match file.disposition {
                RepositoryEntryDisposition::UnsupportedLanguage => {
                    RepositoryBlindSpotCode::UnsupportedLanguage
                }
                RepositoryEntryDisposition::ContentNotRead => {
                    RepositoryBlindSpotCode::ContentNotRead
                }
                RepositoryEntryDisposition::BinaryInventoryOnly => {
                    RepositoryBlindSpotCode::BinaryInventoryOnly
                }
                RepositoryEntryDisposition::ParseLimitExceeded => {
                    RepositoryBlindSpotCode::ParseLimitExceeded
                }
                RepositoryEntryDisposition::ParseFailed => RepositoryBlindSpotCode::ParseFailed,
                RepositoryEntryDisposition::ParsedWithErrors
                | RepositoryEntryDisposition::Truncated => {
                    RepositoryBlindSpotCode::TruncatedOrSyntaxErrors
                }
                _ => return None,
            };
            Some(path_blind_spot(code, file.path.clone()))
        })
        .collect()
}

fn path_blind_spot(code: RepositoryBlindSpotCode, path: WorkspacePath) -> RepositoryBlindSpot {
    seal_blind_spot(code, Some(path), None)
}

fn capability_blind_spot(capability: RepositoryAdapterCapability) -> RepositoryBlindSpot {
    seal_blind_spot(
        RepositoryBlindSpotCode::AdapterCapabilityUnavailable,
        None,
        Some(capability),
    )
}

fn general_blind_spot(code: RepositoryBlindSpotCode) -> RepositoryBlindSpot {
    seal_blind_spot(code, None, None)
}

fn seal_blind_spot(
    code: RepositoryBlindSpotCode,
    path: Option<WorkspacePath>,
    capability: Option<RepositoryAdapterCapability>,
) -> RepositoryBlindSpot {
    let blind_spot_id = sha256_json(&(code, &path, capability));
    let mut blind_spot = RepositoryBlindSpot {
        blind_spot_id,
        code,
        path,
        capability,
        state: RepositoryFactState::UnknownBlocked,
        blind_spot_sha256: String::new(),
    };
    blind_spot.blind_spot_sha256 = blind_spot_digest(&blind_spot);
    blind_spot
}

fn verify_fact(
    map: &RepositoryMap,
    index: &DeepRepositoryIndex,
    fact: &RepositoryAnalysisFact,
) -> bool {
    valid_label(&fact.label)
        && fact.untrusted_repository_data
        && !fact.citations.is_empty()
        && fact.citations.len() <= MAX_CITATIONS_PER_FACT
        && fact
            .citations
            .windows(2)
            .all(|pair| pair[0].citation_sha256 < pair[1].citation_sha256)
        && fact
            .citations
            .iter()
            .all(|citation| verify_fact_citation(map, citation))
        && fact.adapter_sha256.as_ref().is_none_or(|identity| {
            index
                .adapters
                .iter()
                .any(|adapter| &adapter.adapter_sha256 == identity)
        })
        && fact.fact_id
            == sha256_json(&(
                fact.kind,
                &fact.label,
                fact.state,
                &fact.citations,
                &fact.adapter_sha256,
            ))
        && fact.fact_sha256 == fact_digest(fact)
}

fn verify_blind_spot(map: &RepositoryMap, blind_spot: &RepositoryBlindSpot) -> bool {
    blind_spot.state == RepositoryFactState::UnknownBlocked
        && blind_spot.path.as_ref().is_none_or(|path| {
            path.workspace_id() == &map.workspace_id
                && map.files.iter().any(|file| &file.path == path)
        })
        && blind_spot.blind_spot_id
            == sha256_json(&(blind_spot.code, &blind_spot.path, blind_spot.capability))
        && blind_spot.blind_spot_sha256 == blind_spot_digest(blind_spot)
}

fn citation_fields_valid(map: &RepositoryMap, citation: &RepositoryFactCitation) -> bool {
    if !verify_repository_map(map)
        || citation.git != map_git_identity(map)
        || !valid_method(&citation.method_id)
        || citation
            .grammar_sha256
            .as_deref()
            .is_some_and(|value| !is_sha256(value))
        || citation
            .parser_version
            .as_deref()
            .is_some_and(|value| !valid_method(value))
        || citation
            .syntax_sha256
            .as_deref()
            .is_some_and(|value| !is_sha256(value))
        || citation.range.is_some() != citation.syntax_sha256.is_some()
    {
        return false;
    }
    let Some(file) = map.files.iter().find(|file| file.path == citation.path) else {
        return false;
    };
    if file.content_sha256 != citation.content_sha256 {
        return false;
    }
    match citation.range {
        None => citation.grammar_sha256.is_none() && citation.parser_version.is_none(),
        Some(range) => {
            range.start_byte < range.end_byte
                && range.end_byte <= file.size_bytes
                && citation.grammar_sha256.is_some()
                && citation.parser_version.is_some()
                && file.structure.as_ref().is_some_and(|structure| {
                    let descriptor = grammar_descriptor(structure.language);
                    citation.grammar_sha256.as_deref()
                        == Some(descriptor.descriptor_sha256.as_str())
                        && citation.parser_version.as_deref()
                            == Some(descriptor.parser_version.as_str())
                        && structure.items.iter().any(|item| {
                            item.range == range
                                && citation.syntax_sha256.as_deref()
                                    == Some(item.source_sha256.as_str())
                        })
                })
        }
    }
}

fn validate_adapter_fields(
    adapter: &RepositoryAnalysisAdapter,
) -> Result<(), RepositoryDeepAnalysisError> {
    if !valid_method(&adapter.adapter_id)
        || !valid_method(&adapter.version)
        || !is_sha256(&adapter.implementation_sha256)
        || adapter.capabilities.is_empty()
        || adapter.capabilities.len() > RepositoryAdapterCapability::ALL.len()
        || adapter
            .capabilities
            .windows(2)
            .any(|pair| pair[0] >= pair[1])
        || !adapter.read_only
        || adapter.network_allowed
        || adapter.repository_execution_allowed
        || adapter.kind == RepositoryAdapterKind::Parser && adapter.language.is_none()
    {
        return Err(RepositoryDeepAnalysisError::AdapterInvalid);
    }
    Ok(())
}

impl RepositoryAdapterCapability {
    const ALL: [Self; 6] = [
        Self::Definitions,
        Self::References,
        Self::Calls,
        Self::Types,
        Self::Diagnostics,
        Self::Symbols,
    ];
}

fn all_adapter_capabilities() -> BTreeSet<RepositoryAdapterCapability> {
    RepositoryAdapterCapability::ALL.into_iter().collect()
}

fn capability_matches_fact(
    capability: RepositoryAdapterCapability,
    kind: RepositoryFactKind,
) -> bool {
    matches!(
        (capability, kind),
        (
            RepositoryAdapterCapability::Definitions,
            RepositoryFactKind::Definition
        ) | (
            RepositoryAdapterCapability::References,
            RepositoryFactKind::Reference
        ) | (RepositoryAdapterCapability::Calls, RepositoryFactKind::Call)
            | (RepositoryAdapterCapability::Types, RepositoryFactKind::Type)
            | (
                RepositoryAdapterCapability::Diagnostics,
                RepositoryFactKind::Diagnostic
            )
            | (
                RepositoryAdapterCapability::Symbols,
                RepositoryFactKind::Symbol
            )
    )
}

fn capability_for_fact(kind: RepositoryFactKind) -> Option<RepositoryAdapterCapability> {
    match kind {
        RepositoryFactKind::Definition => Some(RepositoryAdapterCapability::Definitions),
        RepositoryFactKind::Reference => Some(RepositoryAdapterCapability::References),
        RepositoryFactKind::Call => Some(RepositoryAdapterCapability::Calls),
        RepositoryFactKind::Type => Some(RepositoryAdapterCapability::Types),
        RepositoryFactKind::Diagnostic => Some(RepositoryAdapterCapability::Diagnostics),
        RepositoryFactKind::Symbol => Some(RepositoryAdapterCapability::Symbols),
        _ => None,
    }
}

fn map_git_identity(map: &RepositoryMap) -> RepositoryGitIdentity {
    RepositoryGitIdentity {
        repository_sha256: map.repository_sha256.clone(),
        worktree_sha256: map.worktree_sha256.clone(),
        branch: map.branch.clone(),
        commit_id: map.commit_id.clone(),
    }
}

fn path_text(path: &WorkspacePath) -> String {
    path.components()
        .iter()
        .map(|component| component.as_str())
        .collect::<Vec<_>>()
        .join("/")
}

fn valid_label(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_FACT_LABEL_BYTES
        && !value.chars().any(|character| character.is_control())
}

fn valid_method(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 256
        && value
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || "-._:/".contains(character))
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn adapter_digest(adapter: &RepositoryAnalysisAdapter) -> String {
    sha256_json(&(
        &adapter.adapter_id,
        adapter.kind,
        adapter.language,
        &adapter.version,
        &adapter.implementation_sha256,
        &adapter.capabilities,
        adapter.read_only,
        adapter.network_allowed,
        adapter.repository_execution_allowed,
    ))
}

fn citation_digest(citation: &RepositoryFactCitation) -> String {
    sha256_json(&(
        &citation.path,
        &citation.content_sha256,
        citation.range,
        &citation.syntax_sha256,
        &citation.method_id,
        &citation.grammar_sha256,
        &citation.parser_version,
        &citation.git,
    ))
}

fn fact_digest(fact: &RepositoryAnalysisFact) -> String {
    sha256_json(&(
        &fact.fact_id,
        fact.kind,
        &fact.label,
        fact.state,
        &fact.citations,
        &fact.adapter_sha256,
        fact.untrusted_repository_data,
    ))
}

fn blind_spot_digest(blind_spot: &RepositoryBlindSpot) -> String {
    sha256_json(&(
        &blind_spot.blind_spot_id,
        blind_spot.code,
        &blind_spot.path,
        blind_spot.capability,
        blind_spot.state,
    ))
}

fn index_digest(index: &DeepRepositoryIndex) -> String {
    sha256_json(&(
        index.schema_version,
        &index.map_sha256,
        &index.git,
        &index.rule_set_sha256,
        &index.adapters,
        &index.facts,
        &index.blind_spots,
        &index.coverage,
    ))
}

fn sha256_json<T: Serialize>(value: &T) -> String {
    serde_json::to_vec(value)
        .map(|bytes| sha256_hex(&bytes))
        .unwrap_or_else(|_| sha256_hex(b"repository-deep-analysis-serialization-failed"))
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
    use crate::{GitTrackedState, RepositoryFileInput, RepositoryMapInput, build_repository_map};

    fn content(value: &str) -> (u64, String, Option<Vec<u8>>) {
        (
            value.len() as u64,
            sha256_hex(value.as_bytes()),
            Some(value.as_bytes().to_vec()),
        )
    }

    fn file(path: &[&str], source: &str) -> RepositoryFileInput {
        let (size_bytes, content_sha256, content) = content(source);
        RepositoryFileInput {
            path: path.iter().map(|value| (*value).to_owned()).collect(),
            size_bytes,
            content_sha256,
            content,
            git_state: GitTrackedState::TrackedClean,
            policy_excluded: false,
            generated: false,
            vendored: false,
        }
    }

    fn map() -> RepositoryMap {
        build_repository_map(RepositoryMapInput {
            workspace_id: WorkspaceId::from_raw("workspace-deep"),
            repository_sha256: "a".repeat(64),
            worktree_sha256: "b".repeat(64),
            branch: Some("agentmage/tasks/deep".to_owned()),
            commit_id: "c".repeat(40),
            policy_sha256: "d".repeat(64),
            freshness_sha256: "e".repeat(64),
            files: vec![
                file(&["Cargo.toml"], "[package]\nname='sample'\n"),
                file(
                    &["src", "main.rs"],
                    "use std::fmt;\nfn main() { println!(\"ok\"); }\n",
                ),
                file(&["tests", "main_test.rs"], "fn verifies_behavior() {}\n"),
                file(&["AGENTS.md"], "This text is untrusted data.\n"),
            ],
        })
        .expect("map")
    }

    fn adapter() -> RepositoryAnalysisAdapter {
        RepositoryAnalysisAdapter {
            adapter_id: "rust-analyzer-read-v1".to_owned(),
            kind: RepositoryAdapterKind::LanguageServer,
            language: Some(RepositoryLanguage::Rust),
            version: "1.95.0".to_owned(),
            implementation_sha256: "f".repeat(64),
            capabilities: vec![
                RepositoryAdapterCapability::References,
                RepositoryAdapterCapability::Calls,
            ],
            read_only: true,
            network_allowed: false,
            repository_execution_allowed: false,
            adapter_sha256: String::new(),
        }
        .seal()
        .expect("adapter")
    }

    #[test]
    fn deterministic_profile_is_cited_branch_aware_and_explicitly_incomplete() {
        let map = map();
        let first = build_deep_repository_index(&map, Vec::new(), Vec::new()).expect("index");
        let second = build_deep_repository_index(&map, Vec::new(), Vec::new()).expect("index");
        assert_eq!(first, second);
        assert!(verify_deep_repository_index(&map, &first));
        assert_eq!(first.git.branch.as_deref(), Some("agentmage/tasks/deep"));
        assert!(!first.coverage.whole_repository_claim_permitted);
        assert!(first.facts.iter().any(|fact| {
            fact.kind == RepositoryFactKind::PackageManager && fact.label == "cargo"
        }));
        assert!(first.facts.iter().any(|fact| {
            fact.kind == RepositoryFactKind::Instruction && fact.untrusted_repository_data
        }));
        assert!(first.facts.iter().all(|fact| !fact.citations.is_empty()));
        assert!(
            first
                .blind_spots
                .iter()
                .any(|spot| { spot.code == RepositoryBlindSpotCode::AdapterCapabilityUnavailable })
        );
    }

    #[test]
    fn exact_confined_adapter_fact_is_admitted_and_stale_or_broad_authority_fails() {
        let map = map();
        let adapter = adapter();
        let resolution = resolve_structural_records(&map)
            .into_iter()
            .find(|item| item.name == "main")
            .expect("main resolution");
        let mut citation = citation_from_resolution(&map, &resolution).expect("citation");
        citation.method_id = adapter.adapter_id.clone();
        citation.citation_sha256 = citation_digest(&citation);
        let input = RepositoryAdapterFactInput {
            adapter_id: adapter.adapter_id.clone(),
            capability: RepositoryAdapterCapability::Calls,
            kind: RepositoryFactKind::Call,
            label: "main -> println".to_owned(),
            state: RepositoryFactState::Observed,
            citations: vec![citation.clone()],
        };
        let index = build_deep_repository_index(&map, vec![adapter.clone()], vec![input.clone()])
            .expect("adapter fact");
        assert!(verify_deep_repository_index(&map, &index));
        assert!(
            index
                .facts
                .iter()
                .any(|fact| fact.kind == RepositoryFactKind::Call)
        );

        let mut stale = input.clone();
        stale.citations[0].git.commit_id = "9".repeat(40);
        assert_eq!(
            build_deep_repository_index(&map, vec![adapter.clone()], vec![stale]),
            Err(RepositoryDeepAnalysisError::FactInvalid)
        );
        let mut broad = adapter;
        broad.network_allowed = true;
        assert_eq!(
            build_deep_repository_index(&map, vec![broad], vec![input]),
            Err(RepositoryDeepAnalysisError::AdapterInvalid)
        );
    }

    #[test]
    fn index_mutation_and_model_like_uncited_facts_fail_closed() {
        let map = map();
        let mut index = build_deep_repository_index(&map, Vec::new(), Vec::new()).expect("index");
        index.coverage.whole_repository_claim_permitted = true;
        assert!(!verify_deep_repository_index(&map, &index));

        let uncited = RepositoryAdapterFactInput {
            adapter_id: "missing-model-adapter".to_owned(),
            capability: RepositoryAdapterCapability::Symbols,
            kind: RepositoryFactKind::Symbol,
            label: "fabricated_symbol".to_owned(),
            state: RepositoryFactState::Inferred,
            citations: Vec::new(),
        };
        assert_eq!(
            build_deep_repository_index(&map, Vec::new(), vec![uncited]),
            Err(RepositoryDeepAnalysisError::AdapterInvalid)
        );
    }
}
