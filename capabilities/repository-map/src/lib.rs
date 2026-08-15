#![deny(missing_docs)]
#![forbid(unsafe_code)]
//! Deterministic bounded structural repository mapping without ambient authority.

mod cache;
mod deep_analysis;
mod grammar;
#[cfg(test)]
mod invariance_tests;
mod inventory;
mod parser;
mod renderer;
mod resolution;

pub use cache::{RepositoryMapCache, RepositoryMapCacheError, RepositoryMapCacheKey};
pub use deep_analysis::{
    DeepRepositoryIndex, RepositoryAdapterCapability, RepositoryAdapterFactInput,
    RepositoryAdapterKind, RepositoryAnalysisAdapter, RepositoryAnalysisFact, RepositoryBlindSpot,
    RepositoryBlindSpotCode, RepositoryDeepAnalysisError, RepositoryDeepCoverage,
    RepositoryFactCitation, RepositoryFactKind, RepositoryFactState, build_deep_repository_index,
    verify_deep_repository_index, verify_fact_citation,
};
pub use grammar::{
    GrammarDescriptor, RepositoryLanguage, grammar_descriptor, grammar_set_sha256,
    language_for_path, supported_grammars, verify_grammar_descriptor,
};
pub use inventory::{
    GitTrackedState, RepositoryCoverage, RepositoryEntryDisposition, RepositoryFileInput,
    RepositoryFileRecord, RepositoryMap, RepositoryMapError, RepositoryMapInput,
    build_repository_map, verify_repository_file_record, verify_repository_map,
};
pub use parser::{
    ParseDisposition, RepositoryParseError, SourceRange, StructuralItem, StructuralItemKind,
    StructuralParseResult, StructuralRelationship, StructuralRelationshipKind, parse_structure,
    verify_structural_parse_result,
};
pub use renderer::{
    LexicalSourceMatch, RenderedRepositoryContext, RenderedRepositoryFile,
    RepositoryContextRequest, RepositoryContextSource, RepositoryLimitation,
    RepositoryLimitationCode, RepositoryRenderCoverage, RepositoryRenderError,
    RepositoryRenderPriority, render_repository_context, verify_rendered_repository_context,
};
pub use resolution::{
    RepositoryGitIdentity, StructuralSourceResolution, resolve_structural_records,
    verify_structural_source_resolution,
};

/// Stable component identity used by diagnostics and build verification.
pub const COMPONENT_ID: &str = "capability-repository-map";

/// Returns the identity of the contracts consumed by this capability.
#[must_use]
pub const fn contract_component_id() -> &'static str {
    agentmage_kernel_contracts::COMPONENT_ID
}
