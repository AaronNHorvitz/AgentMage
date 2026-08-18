#![deny(missing_docs)]
#![forbid(unsafe_code)]
//! Deterministic bounded structural repository mapping without ambient authority.

mod cache;
mod change_intent;
mod change_plan;
mod deep_analysis;
mod deep_views;
mod grammar;
#[cfg(test)]
mod invariance_tests;
mod inventory;
mod language_service;
mod package_scaffold;
mod parser;
mod renderer;
mod resolution;
mod structured_edit;
mod test_generation;

pub use cache::{RepositoryMapCache, RepositoryMapCacheError, RepositoryMapCacheKey};
pub use change_intent::{
    ChangeClarification, ChangeImpactState, ChangeImpactSurface, ChangeImpactSurfaceKind,
    ChangeIntentError, ChangeIntentInput, ChangeIntentRecord, ChangeIntentStatus, ChangeRiskDomain,
    ChangeSurfaceExclusion, ChangeUnknownDimension, MinimalChangeImpactReport,
    build_minimal_change_impact, normalize_change_intent, verify_change_intent,
    verify_minimal_change_impact,
};
pub use change_plan::{
    ChangeAlternativeDimension, ChangeAlternativeOption, ChangeAlternativeSet, ChangeHypothesis,
    ChangePlanError, ChangePlanInput, ChangePlanRecord, ChangePlanStatus, ChangeReviewKind,
    ChangeWorkKind, HypothesisCheck, HypothesisCheckResult, HypothesisRecord, HypothesisStatus,
    PlannedValidation, PlannedValidationKind, RegressionTestDisposition, RegressionTestPlan,
    ReproductionExecutionStatus, ReproductionInput, ReproductionOutcome, ReproductionRecord,
    ReproductionStep, ReproductionStepKind, build_change_plan, seal_hypotheses,
    seal_regression_test_plan, seal_reproduction, verify_change_plan, verify_reproduction,
};
pub use deep_analysis::{
    DeepRepositoryIndex, RepositoryAdapterCapability, RepositoryAdapterFactInput,
    RepositoryAdapterKind, RepositoryAnalysisAdapter, RepositoryAnalysisFact, RepositoryBlindSpot,
    RepositoryBlindSpotCode, RepositoryDeepAnalysisError, RepositoryDeepCoverage,
    RepositoryFactCitation, RepositoryFactKind, RepositoryFactState, build_deep_repository_index,
    verify_deep_repository_index, verify_fact_citation,
};
pub use deep_views::{
    RepositoryAnalysisSlice, RepositoryDeepViewError, RepositoryDocumentationDrift,
    RepositoryDocumentationDriftState, RepositoryGlossaryTerm, RepositoryHistoryObservation,
    RepositoryHistoryView, RepositoryLearningExport, RepositoryLearningGuide,
    RepositoryLearningGuideKind, RepositoryPortfolioEntry, RepositoryPortfolioView,
    RepositorySliceCoverage, RepositorySliceDimension, RepositorySliceRequest, RepositoryTrace,
    RepositoryTraceKind, RepositoryTraceSet, build_repository_glossary,
    build_repository_history_view, build_repository_learning_export, build_repository_portfolio,
    build_repository_traces, seal_documentation_drift, slice_deep_repository_index,
    verify_repository_slice, verify_repository_traces,
};
pub use grammar::{
    GrammarDescriptor, RepositoryLanguage, grammar_descriptor, grammar_set_sha256,
    language_for_path, supported_grammars, verify_grammar_descriptor,
};
pub use inventory::{
    GitTrackedState, RepositoryCoverage, RepositoryEntryDisposition, RepositoryFileInput,
    RepositoryFileRecord, RepositoryMap, RepositoryMapError, RepositoryMapInput,
    RepositoryObjectKind, build_repository_map, verify_repository_file_record,
    verify_repository_map,
};
pub use language_service::{
    LanguageServiceCapability, LanguageServiceDescriptor, LanguageServiceError,
    LanguageServiceItem, LanguageServiceItemKind, LanguageServiceObservation,
    LanguageServiceObservationStatus, LanguageServiceRequest, LanguageServiceRequestKind,
    LanguageServiceVisibleFile, seal_language_service_descriptor,
    seal_language_service_observation, verify_language_service_observation,
};
pub use package_scaffold::{
    PackageLanguage, PackageScaffoldError, PackageScaffoldPlan, PackageScaffoldRequest,
    ScaffoldCommand, ScaffoldCommandPurpose, ScaffoldFile, build_package_scaffold,
    package_convention_sha256, verify_package_scaffold,
};
pub use parser::{
    ParseDisposition, RepositoryParseError, SourceRange, StructuralItem, StructuralItemKind,
    StructuralParseResult, StructuralRelationship, StructuralRelationshipKind, parse_structure,
    parse_structure_with_cancellation, verify_structural_parse_result,
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
pub use structured_edit::{
    StructuredArtifactClass, StructuredChangedRange, StructuredEdit, StructuredEditError,
    StructuredEditMethod, StructuredFileChangePlan, StructuredFileChangeRequest,
    StructuredFileChangeSummary, StructuredLanguage, StructuredReviewHook, StructuredUnchangedSpan,
    build_structured_file_change, validate_structured_edit_proposal, verify_structured_file_change,
};
pub use test_generation::{
    GeneratedTestCase, GeneratedTestChangeBinding, GeneratedTestExpectation,
    RepositoryTestFramework, RepositoryTestStyle, TestConcern, TestConcernApplicability,
    TestGenerationError, TestGenerationPlan, TestGenerationRequest, build_test_generation_plan,
    verify_test_generation_plan,
};

/// Stable component identity used by diagnostics and build verification.
pub const COMPONENT_ID: &str = "capability-repository-map";

/// Returns the identity of the contracts consumed by this capability.
#[must_use]
pub const fn contract_component_id() -> &'static str {
    agentmage_kernel_contracts::COMPONENT_ID
}
