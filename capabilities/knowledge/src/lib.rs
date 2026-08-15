#![deny(missing_docs)]
#![forbid(unsafe_code)]
//! Canonical user-owned Markdown knowledge-domain contracts without ambient authority.

mod authority;
mod coding_skills;
mod document_control_skills;
mod domain;
mod executive_skills;
mod index;
mod knowledge_write;
mod lifecycle;
mod markdown_artifacts;
mod markdown_write;
mod meeting_skills;
mod memory;
mod memory_lifecycle;
mod memory_working;
mod obsidian;
mod obsidian_index;
mod operations;
mod plain_folder;
mod retrieval;
mod schema;
mod semantic;
mod semantic_benchmark;
mod skills;
mod store;
mod tasks;
mod workflows;

pub use authority::{
    KnowledgeDataOwner, KnowledgeFieldPolicy, KnowledgeStorageRule, knowledge_data_dictionary,
    verify_data_dictionary,
};
pub use coding_skills::{
    CodingSkill, CodingSkillAdmission, CodingSkillAssessment, CodingSkillDefinition,
    CodingSkillFinding, assess_coding_skill_definition, built_in_coding_skill_definitions,
    built_in_coding_skill_pack,
};
pub use document_control_skills::{DocumentControlSkill, built_in_document_control_skill_pack};
pub use domain::{
    KnowledgeError, KnowledgeField, KnowledgeLink, KnowledgeLinkKind, KnowledgePrivacy,
    KnowledgeRecord, KnowledgeRecordId, KnowledgeRecordKind, KnowledgeRetention,
    KnowledgeRetentionKind, validate_record,
};
pub use executive_skills::{ExecutiveSkill, built_in_executive_skill_pack};
pub use index::{KnowledgeIndex, KnowledgeIndexError, KnowledgeIndexHit, KnowledgeIndexReport};
pub use knowledge_write::{
    CanonicalKnowledgeMutation, CanonicalMarkdownWriteOutcome, KnowledgeFrontmatterProperty,
    KnowledgeIndexPublication, KnowledgeIndexPublicationState, KnowledgeMemoryPromotionApproval,
    KnowledgeNamespaceSnapshot, KnowledgeNoteCreatePreview, KnowledgeNoteCreateRequest,
    KnowledgeSectionDraft, KnowledgeStructuralActionKind, KnowledgeStructuralActionPreview,
    KnowledgeWriteWorkflow, decide_index_publication, preview_knowledge_note_create,
    preview_knowledge_structural_action, verify_knowledge_note_create_preview,
};
pub use lifecycle::{
    KnowledgeBackup, KnowledgeBackupEntry, KnowledgeMigrationEntry, KnowledgeMigrationPlan,
    KnowledgeRestoreAction, KnowledgeRestoreActionKind, KnowledgeRestorePlan, build_backup,
    preview_migration, preview_restore, verify_backup,
};
pub use markdown_artifacts::{
    GeneratedMarkdownArtifact, MarkdownArtifactCitation, MarkdownArtifactError,
    MarkdownArtifactKind, MarkdownArtifactRequest, MarkdownArtifactSection,
    MarkdownArtifactStatement, MarkdownQualityFinding, MarkdownQualityFindingKind,
    MarkdownQualityProfile, MarkdownQualityReport, MarkdownRenderedBlock, MarkdownRoundTripResult,
    generate_markdown_artifact, map_markdown_parse_error, review_markdown_quality,
    verify_markdown_round_trip,
};
pub use markdown_write::{
    MarkdownDocument, MarkdownEdit, MarkdownElement, MarkdownElementKind, MarkdownFidelityWarning,
    MarkdownLineEnding, MarkdownSourceRange, MarkdownUpdatePreview, MarkdownUpdateRequest,
    MarkdownWriteError, preview_markdown_update, verify_markdown_update_preview,
};
pub use meeting_skills::{MeetingSkill, built_in_meeting_skill_pack};
pub use memory::{
    MemoryCandidate, MemoryCandidateClass, MemoryCandidateDecision, MemoryError, MemoryId,
    MemoryItem, MemoryItemStatus, MemoryScope, MemoryType, UserMemoryDecision,
    evaluate_memory_candidate, resolve_memory_candidate,
};
pub use memory_lifecycle::{
    MemoryCatalog, MemoryCatalogSummary, MemoryLifecycleReceipt, MemoryMarkdownBundle,
    MemoryMarkdownFile, MemoryTransitionKind,
};
pub use memory_working::{
    MemoryLoadHit, MemoryLoadQuery, MemoryLoadReason, MemoryLoadResult, WorkingCompactionPreview,
    WorkingCompactionProposal, WorkingMemory, WorkingMemoryPreview, select_memory,
};
pub use obsidian::{
    ObsidianAttachment, ObsidianBacklink, ObsidianBlockReference, ObsidianCallout,
    ObsidianCoverageItem, ObsidianEmbed, ObsidianEntryKind, ObsidianError,
    ObsidianFrontmatterValue, ObsidianHeading, ObsidianKnowledgeStore, ObsidianLinkIssue,
    ObsidianLinkIssueKind, ObsidianNoteInput, ObsidianParsedNote, ObsidianProperty,
    ObsidianResolvedLink, ObsidianSourceRange, ObsidianTag, ObsidianTask, ObsidianTimestamp,
    ObsidianVaultSelection, ObsidianVaultSnapshot,
};
pub use obsidian_index::{
    ObsidianAccessKind, ObsidianAccessReceipt, ObsidianFileChangePreview, ObsidianIndexConflict,
    ObsidianIndexConflictKind, ObsidianIndexElementKind, ObsidianIndexError, ObsidianIndexHit,
    ObsidianIndexReport, ObsidianIndexUpdate, ObsidianPostWriteIndexResult, ObsidianPreviewResult,
    ObsidianQueryResult, ObsidianTemporalClass, ObsidianTraversalResult, ObsidianVaultFreshness,
    ObsidianVaultIndex, ObsidianWatchEvent, ObsidianWatchEventKind,
};
pub use operations::{
    KnowledgeDashboard, KnowledgeDuplicate, KnowledgeDuplicateReason, KnowledgeExport,
    KnowledgeImportReport, KnowledgeRelationship, build_dashboard, build_json_lines_export,
    validate_import,
};
pub use plain_folder::{
    KnowledgeFilenameTemplate, KnowledgeKindPathTemplate, PlainFolderEntryKind,
    PlainFolderKnowledgeStore, PlainFolderLayout, PlainFolderNoteInput, parse_canonical_markdown,
    render_canonical_markdown,
};
pub use retrieval::{
    KnowledgeAnswerDraft, KnowledgeContextEntry, KnowledgeContextQuery, KnowledgeEvidenceState,
    KnowledgeFileType, KnowledgeFreshness, KnowledgeRenderedAnswer, KnowledgeRetrievalError,
    KnowledgeRetrievalHit, KnowledgeRetrievalResult, KnowledgeScoreFactor,
    KnowledgeSourceAuthority, KnowledgeSourceDocument, KnowledgeSourceFragment,
    KnowledgeSourceFragmentKind, KnowledgeSynthesisEnvelope, prepare_knowledge_synthesis,
    render_knowledge_answer, retrieve_knowledge,
};
pub use schema::{
    KnowledgeRecordSchema, knowledge_schema, knowledge_schemas, verify_schema_registry,
};
pub use semantic::{
    LocalSemanticIndex, SemanticActivation, SemanticAdmissionReceipt, SemanticChunkInput,
    SemanticError, SemanticField, SemanticIndexHit, SemanticIndexKey, SemanticIndexReport,
    SemanticIndexSummary, SemanticLifecycleReceipt, SemanticModelManifest, SemanticModelRole,
    SemanticOptIn, SemanticProfileState, SemanticRemoteOperation, SemanticRemoteRejectionReceipt,
    SemanticRuntime, SemanticScopeEntry, SemanticStorageProtection, SemanticVectorInput,
    reject_remote_semantic_attempt,
};
pub use semantic_benchmark::{
    RetrievalBenchmarkCase, RetrievalBenchmarkError, RetrievalBenchmarkMetrics,
    RetrievalBenchmarkMode, RetrievalBenchmarkReport, RetrievalReleaseBehavior, RetrievalTaskClass,
    SemanticBenefitThresholds, benchmark_retrieval,
};
pub use skills::{
    DeclarativeAssetKind, DeclarativeSkillAsset, DeclarativeSkillCompatibility,
    DeclarativeSkillError, DeclarativeSkillFile, DeclarativeSkillManifest, DeclarativeSkillPackage,
    DeclarativeSkillRegistry, DeclarativeSkillScope, DeclarativeSkillTrustState,
    SkillAuthorityCeiling, SkillContext, SkillContextEntry, SkillInfluenceReceipt,
    SkillInstructionConflict, compose_skill_context, seal_declarative_skill_manifest,
};
pub use store::{
    KnowledgeRecordSummary, KnowledgeStore, KnowledgeWriteKind, KnowledgeWritePreview,
};
pub use tasks::{
    KnowledgeTask, KnowledgeTaskDuplicate, KnowledgeTaskPriority, KnowledgeTaskStatus,
    KnowledgeTaskTransitionPreview, KnowledgeTaskView, KnowledgeTaskViewKind, build_task_view,
    preview_task_transition,
};
pub use workflows::{
    KnowledgeRetrievalMode, KnowledgeWorkflow, KnowledgeWorkflowEvidence, KnowledgeWorkflowResult,
    built_in_knowledge_skill_pack, built_in_skill_registry, run_read_only_knowledge_workflow,
};

/// Stable component identity used by diagnostics and build verification.
pub const COMPONENT_ID: &str = "capability-knowledge";

/// Immutable schema version for the first canonical knowledge-domain family.
pub const KNOWLEDGE_SCHEMA_VERSION: u16 = 1;

/// Exact Obsidian parser generation bound into every vault index element.
pub const OBSIDIAN_PARSER_VERSION: u16 = 2;

/// Returns the identity of the contracts consumed by this capability.
#[must_use]
pub const fn contract_component_id() -> &'static str {
    agentmage_kernel_contracts::COMPONENT_ID
}

#[cfg(test)]
mod tests {
    use super::{COMPONENT_ID, KNOWLEDGE_SCHEMA_VERSION, contract_component_id};

    #[test]
    fn capability_depends_only_on_contracts() {
        assert_eq!(COMPONENT_ID, "capability-knowledge");
        assert_eq!(KNOWLEDGE_SCHEMA_VERSION, 1);
        assert_eq!(contract_component_id(), "kernel-contracts");
    }
}
