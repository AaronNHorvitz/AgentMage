#![deny(missing_docs)]
#![forbid(unsafe_code)]
//! Canonical user-owned Markdown knowledge-domain contracts without ambient authority.

mod advanced_reconciliation;
mod authority;
mod coding_skills;
mod document_control_skills;
mod domain;
mod executive_skills;
mod index;
mod json_data;
mod knowledge_write;
mod lifecycle;
mod markdown_artifact_skills;
mod markdown_artifacts;
mod markdown_write;
mod meeting_skills;
mod memory;
mod memory_lifecycle;
mod memory_portable;
mod memory_working;
mod obsidian;
mod obsidian_index;
mod operations;
mod pdf_diagram;
mod pdf_extraction;
mod pdf_generation;
mod pdf_inspection;
mod pdf_redaction;
mod pdf_visual;
mod plain_folder;
mod presentation_generation;
mod presentation_ooxml;
mod reconciliation_workbook;
mod retrieval;
mod retrieval_integration;
mod schema;
mod semantic;
mod semantic_benchmark;
mod skills;
mod spreadsheet_ooxml;
mod spreadsheet_source;
mod store;
mod tabular;
mod tasks;
mod word_edit;
mod word_generation;
mod word_ooxml;
mod word_receipt;
mod word_rich_generation;
mod word_source;
mod word_visual;
mod workflows;

pub use advanced_reconciliation::{
    AdvancedReconciliationError, AdvancedReconciliationRecord, AllocationRecipient,
    AllocationResult, ExactDecimal, FinancialRoundingMode, ReconciliationSourceFreshness,
    allocate_many_to_many, record_advanced_reconciliation, require_current_source, round_financial,
    within_tolerance,
};
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
pub use json_data::{
    StructuredJsonComparison, StructuredJsonDifference, StructuredJsonDifferenceReason,
    StructuredJsonDocument, StructuredJsonError, StructuredJsonIssue, StructuredJsonProfile,
    StructuredJsonRedaction, StructuredJsonRedactionEntry, StructuredJsonRedactionPolicy,
    StructuredJsonSchema, compare_structured_json, parse_structured_json, redact_structured_json,
};
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
pub use markdown_artifact_skills::{MarkdownArtifactSkill, built_in_markdown_artifact_skill_pack};
pub use markdown_artifacts::{
    GeneratedMarkdownArtifact, MarkdownArtifactCitation, MarkdownArtifactError,
    MarkdownArtifactKind, MarkdownArtifactRequest, MarkdownArtifactSection,
    MarkdownArtifactStatement, MarkdownQualityFinding, MarkdownQualityFindingKind,
    MarkdownQualityProfile, MarkdownQualityReport, MarkdownRenderedBlock, MarkdownRoundTripResult,
    generate_markdown_artifact, map_markdown_parse_error, render_markdown_structure,
    review_markdown_quality, verify_markdown_round_trip,
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
pub use memory_portable::{
    EncryptedMemoryExport, MemoryExportEntropy, MemoryPortableError, MemoryPortableKey,
    MemoryPortableReceipt, export_memory_catalog, import_memory_catalog,
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
    ObsidianQueryResult, ObsidianRetrievalDocuments, ObsidianTemporalClass,
    ObsidianTraversalResult, ObsidianVaultFreshness, ObsidianVaultIndex, ObsidianWatchEvent,
    ObsidianWatchEventKind, obsidian_snapshot_sha256, retrieval_documents_from_snapshot,
};
pub use operations::{
    KnowledgeDashboard, KnowledgeDuplicate, KnowledgeDuplicateReason, KnowledgeExport,
    KnowledgeImportReport, KnowledgeRelationship, build_dashboard, build_json_lines_export,
    validate_import,
};
pub use pdf_diagram::{
    PdfOfflineDiagramAdmission, PdfOfflineDiagramError, PdfOfflineDiagramObservation,
    PdfOfflineDiagramProjection, validate_pdf_offline_diagram,
};
pub use pdf_extraction::{
    PdfExtractionError, PdfExtractionLimitation, PdfExtractionMethod, PdfExtractionProfile,
    PdfExtractionResult, PdfOcrAdmission, PdfOcrEligibility, PdfOcrEngine, PdfOcrObservation,
    PdfOcrProjection, PdfOcrRegion, PdfPageCitation, PdfPageExtraction, PdfPageIdentity,
    PdfPageState, PdfReadingOrderObservation, PdfStructuredSourceExtractor, PdfTextSpan,
    extract_pdf_to_pages, pdf_extractor_identity_sha256, pdf_ocr_eligibility,
    validate_pdf_ocr_observation,
};
pub use pdf_generation::{
    GeneratedPdfReport, PdfGenerationError, PdfGenerationLimit, PdfPageSettings, PdfReportBlock,
    PdfReportFormField, PdfReportMetadata, PdfReportRequest, generate_pdf_report,
    pdf_generator_identity_sha256, pdf_report_request_from_markdown,
};
pub use pdf_inspection::{
    PdfArtifactFinding, PdfArtifactFindingKind, PdfArtifactInspection, PdfFormFieldObservation,
    PdfImageObservation, PdfLinkKind, PdfLinkObservation, PdfMetadataSummary, inspect_pdf_artifact,
};
pub use pdf_redaction::{
    PdfRedactionError, PdfRedactionLayer, PdfRedactionLayerCheck, PdfRedactionReceipt,
    PdfRedactionRequest, PdfRedactionTarget, RedactedPdfReport, pdf_redactor_identity_sha256,
    redact_generated_pdf_report,
};
pub use pdf_visual::{
    PdfPageImage, PdfRenderEvidenceKind, PdfRenderOutput, PdfRenderPlatform, PdfRenderProfile,
    PdfVisualComparisonError, PdfVisualComparisonReport, compare_pdf_page_images,
    pdf_render_profile_sha256,
};
pub use plain_folder::{
    KnowledgeFilenameTemplate, KnowledgeKindPathTemplate, PlainFolderEntryKind,
    PlainFolderKnowledgeStore, PlainFolderLayout, PlainFolderNoteInput, parse_canonical_markdown,
    render_canonical_markdown,
};
pub use presentation_generation::{
    EditedPresentation, GeneratedPresentation, PresentationBlock, PresentationChartSpec,
    PresentationDeckSpec, PresentationDiagramEdge, PresentationDiagramNode,
    PresentationDiagramSpec, PresentationEditRequest, PresentationGenerationError,
    PresentationObjectPreview, PresentationPlotPoint, PresentationPlotSpec,
    PresentationSlideChange, PresentationSlidePreview, PresentationSlideReplacement,
    PresentationSlideSpec, PresentationTableSpec, edit_generated_presentation,
    generate_presentation,
};
pub use presentation_ooxml::{
    PresentationError, PresentationFinding, PresentationFindingKind, PresentationImageReference,
    PresentationInspection, PresentationLink, PresentationObject, PresentationObjectKind,
    PresentationProfile, PresentationSlide, inspect_pptx,
};
pub use reconciliation_workbook::{
    GeneratedReconciliationWorkbook, ReconciliationFormula, ReconciliationWorkbookError,
    ReconciliationWorkbookProfile, ReconciliationWorkbookRequest, SpreadsheetFormulaObservation,
    SpreadsheetVerificationEvidenceKind, SpreadsheetVerificationObservation,
    SpreadsheetVerificationPlatform, SpreadsheetVerificationProfile, SpreadsheetVerificationReport,
    generate_reconciliation_workbook, verify_reconciliation_workbook,
};
pub use retrieval::{
    KnowledgeAnswerDraft, KnowledgeContextEntry, KnowledgeContextQuery, KnowledgeEvidenceState,
    KnowledgeFileType, KnowledgeFreshness, KnowledgeRenderedAnswer, KnowledgeRetrievalError,
    KnowledgeRetrievalHit, KnowledgeRetrievalResult, KnowledgeScoreFactor,
    KnowledgeSourceAuthority, KnowledgeSourceDocument, KnowledgeSourceFragment,
    KnowledgeSourceFragmentKind, KnowledgeSynthesisEnvelope, prepare_knowledge_synthesis,
    render_knowledge_answer, retrieve_knowledge,
};
pub use retrieval_integration::{
    ObsidianQuestionParityReport, ObsidianRetrievalIntegrationError,
    evaluate_obsidian_question_parity,
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
pub use spreadsheet_ooxml::{
    SpreadsheetCell, SpreadsheetCellKind, SpreadsheetDateSystem, SpreadsheetError,
    SpreadsheetFinding, SpreadsheetFindingKind, SpreadsheetHyperlink, SpreadsheetInspection,
    SpreadsheetProfile, SpreadsheetSheetState, SpreadsheetWorksheet, excel_serial_date,
    inspect_xlsx, inspect_xlsx_with_control,
};
pub use spreadsheet_source::SpreadsheetStructuredSourceExtractor;
pub use store::{
    KnowledgeRecordSummary, KnowledgeStore, KnowledgeWriteKind, KnowledgeWritePreview,
};
pub use tabular::{
    DelimitedDialect, SafeCsvProposal, TabularComparison, TabularDocument, TabularError,
    TabularFilter, TabularMatch, TabularMatchReason, TabularProfile, TabularRowProjection,
    TabularSort, TabularSortDirection, TabularSummary, build_safe_csv, clean_tabular_text,
    compare_tabular, filter_sort_tabular, normalized_tabular_filename, normalized_tabular_id,
    normalized_tabular_text, normalized_tabular_url, parse_delimited_table, summarize_tabular,
};
pub use tasks::{
    KnowledgeTask, KnowledgeTaskDuplicate, KnowledgeTaskPriority, KnowledgeTaskStatus,
    KnowledgeTaskTransitionPreview, KnowledgeTaskView, KnowledgeTaskViewKind, build_task_view,
    preview_task_transition,
};
pub use word_edit::{
    PreservedWordPart, WordCommentMetadata, WordEditChange, WordEditOperation,
    WordEditOperationKind, WordEditRequest, WordEditTarget, WordPackageEditPreview,
    WordPackageEditWarning, WordPackageEditWarningKind, WordPackageEditorError,
    WordRedlineMetadata, preview_word_package_edit, verify_word_package_edit_preview,
};
pub use word_generation::{
    GeneratedWordPackage, GeneratedWordPart, WordGenerationWarning, WordGenerationWarningKind,
    generate_docx_from_markdown, word_generator_identity_sha256,
};
pub use word_ooxml::{
    WordCompressionKind, WordConversionProfile, WordExtractionResult, WordFeatureCount,
    WordFeatureKind, WordFidelityWarning, WordInspectionReport, WordOoxmlError, WordPackageFinding,
    WordPackageFindingKind, WordPackagePart, WordPartKind, WordPartSourceRange, WordRevisionState,
    WordSidecarCache, WordSidecarCacheOutcome, WordTextFragment, extract_docx_to_sidecar,
    inspect_docx, word_conversion_identity_sha256,
};
pub use word_receipt::{
    WordArtifactChangeReference, WordArtifactCheck, WordArtifactCheckKind, WordArtifactCheckStatus,
    WordArtifactCompletionState, WordArtifactInput, WordArtifactReceipt, WordArtifactReceiptError,
    WordArtifactReceiptRequest, WordFidelityLimit, WordRenderEvidenceReference,
    build_word_artifact_receipt, verify_word_artifact_receipt,
};
pub use word_rich_generation::{
    RichWordPackageProposal, WordBorderStyle, WordDecisionCard, WordDocumentMetadata,
    WordHeaderFooterConfiguration, WordNumberingDefinition, WordPageConfiguration,
    WordRichDocumentBuilder, WordRichGenerationError, WordStyleConfiguration, WordStyleKind,
    WordTableLayout,
};
pub use word_source::WordStructuredSourceExtractor;
pub use word_visual::{
    WordPageComparison, WordPageDifferenceBounds, WordPageImage, WordRenderEvidenceKind,
    WordRenderOutput, WordRenderPlatform, WordRenderProfile, WordVisualComparisonError,
    WordVisualComparisonReport, compare_word_page_images, word_render_profile_sha256,
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
