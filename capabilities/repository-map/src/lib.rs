#![deny(missing_docs)]
#![forbid(unsafe_code)]
//! Deterministic bounded structural repository mapping without ambient authority.

mod cache;
mod grammar;
mod inventory;
mod parser;

pub use cache::{RepositoryMapCache, RepositoryMapCacheError, RepositoryMapCacheKey};
pub use grammar::{
    GrammarDescriptor, RepositoryLanguage, grammar_descriptor, grammar_set_sha256,
    language_for_path, supported_grammars, verify_grammar_descriptor,
};
pub use inventory::{
    GitTrackedState, RepositoryEntryDisposition, RepositoryFileInput, RepositoryFileRecord,
    RepositoryMap, RepositoryMapError, RepositoryMapInput, build_repository_map,
    verify_repository_file_record, verify_repository_map,
};
pub use parser::{
    ParseDisposition, RepositoryParseError, SourceRange, StructuralItem, StructuralItemKind,
    StructuralParseResult, StructuralRelationship, StructuralRelationshipKind, parse_structure,
    verify_structural_parse_result,
};

/// Stable component identity used by diagnostics and build verification.
pub const COMPONENT_ID: &str = "capability-repository-map";

/// Returns the identity of the contracts consumed by this capability.
#[must_use]
pub const fn contract_component_id() -> &'static str {
    agentmage_kernel_contracts::COMPONENT_ID
}
