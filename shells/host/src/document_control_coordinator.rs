//! Authority-free product composition for local document-control workflows.

use agentmage_capability_knowledge::built_in_document_control_skill_pack;
use agentmage_kernel_contracts::{
    DocumentActionApproval, DocumentActionPreview, DocumentActionReview, DocumentControlFinding,
    DocumentRegister, DocumentWorkflowKind, DocumentWorkflowReport,
};
use agentmage_kernel_engine::document_control::{
    build_document_workflow_report, review_document_action, review_document_register,
    verify_document_action_preview, verify_document_register,
};

/// Stable failure from the local document-control product composition.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DocumentControlCoordinatorError {
    /// The caller cancelled before any projection began.
    Cancelled,
    /// A required already-approved local dependency is unavailable.
    DependencyUnavailable,
    /// A sealed record, preview, approval, or workflow input failed closed.
    InvalidInput,
    /// The admitted built-in document-control skill pack is unavailable or incomplete.
    SkillPackInvalid,
    /// A supplied or derived record claimed an external effect or execution grant.
    AuthorityViolation,
}

impl DocumentControlCoordinatorError {
    /// Stable content-free failure code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::Cancelled => "document-control.coordinator.cancelled",
            Self::DependencyUnavailable => "document-control.coordinator.dependency-unavailable",
            Self::InvalidInput => "document-control.coordinator.input-invalid",
            Self::SkillPackInvalid => "document-control.coordinator.skill-pack-invalid",
            Self::AuthorityViolation => "document-control.coordinator.authority-violation",
        }
    }
}

impl std::fmt::Display for DocumentControlCoordinatorError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.code())
    }
}

impl std::error::Error for DocumentControlCoordinatorError {}

/// Caller-owned identity, workflow, and explicit local readiness state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DocumentControlCoordinatorRequest {
    /// True only after every declared local input dependency is available.
    pub dependencies_ready: bool,
    /// Sticky cancellation checked before dependency or record evaluation.
    pub cancellation_requested: bool,
    /// Stable identity for the local workflow report.
    pub report_id: String,
    /// Closed local-only workflow class.
    pub workflow_kind: DocumentWorkflowKind,
    /// Rendered routing slip, merge preview, calendar draft, or filing suggestion.
    pub rendered_preview: String,
}

/// One exact local document-control workspace with no effect authority.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DocumentControlCoordinatorOutcome {
    /// Exact verified source register.
    pub register: DocumentRegister,
    /// Exact verified action previews in canonical order.
    pub previews: Vec<DocumentActionPreview>,
    /// Deterministic content-minimized register findings.
    pub findings: Vec<DocumentControlFinding>,
    /// One review for every exact preview; complete review still creates no authority.
    pub reviews: Vec<DocumentActionReview>,
    /// Local-only workflow report over the exact register and previews.
    pub report: DocumentWorkflowReport,
    /// Exact number of admitted authority-free built-in document-control skills.
    pub admitted_skill_count: u32,
    /// Fixed false: composition cannot send, schedule, save, rename, move, file, or dispose.
    pub external_effect_allowed: bool,
}

/// Composes a sealed register, exact previews, and matching approval reviews locally.
pub fn coordinate_document_control_workspace(
    register: DocumentRegister,
    previews: Vec<DocumentActionPreview>,
    approvals: Vec<Option<DocumentActionApproval>>,
    request: DocumentControlCoordinatorRequest,
) -> Result<DocumentControlCoordinatorOutcome, DocumentControlCoordinatorError> {
    if request.cancellation_requested {
        return Err(DocumentControlCoordinatorError::Cancelled);
    }
    if !request.dependencies_ready {
        return Err(DocumentControlCoordinatorError::DependencyUnavailable);
    }
    if previews.len() != approvals.len() {
        return Err(DocumentControlCoordinatorError::InvalidInput);
    }

    let packages = built_in_document_control_skill_pack()
        .map_err(|_| DocumentControlCoordinatorError::SkillPackInvalid)?;
    if packages.len() != 8 {
        return Err(DocumentControlCoordinatorError::SkillPackInvalid);
    }

    verify_document_register(&register)
        .map_err(|_| DocumentControlCoordinatorError::InvalidInput)?;
    for preview in &previews {
        verify_document_action_preview(preview)
            .map_err(|_| DocumentControlCoordinatorError::InvalidInput)?;
        if !register
            .entries
            .iter()
            .any(|entry| entry.record_id == preview.record_id)
        {
            return Err(DocumentControlCoordinatorError::InvalidInput);
        }
    }

    let findings = review_document_register(&register)
        .map_err(|_| DocumentControlCoordinatorError::InvalidInput)?;
    let reviews = previews
        .iter()
        .zip(&approvals)
        .map(|(preview, approval)| {
            review_document_action(preview, approval.as_ref())
                .map_err(|_| DocumentControlCoordinatorError::InvalidInput)
        })
        .collect::<Result<Vec<_>, _>>()?;
    let report = build_document_workflow_report(
        request.report_id,
        request.workflow_kind,
        &register,
        &previews,
        request.rendered_preview,
    )
    .map_err(|_| DocumentControlCoordinatorError::InvalidInput)?;

    if !register.proposal_only
        || register.external_effects_performed
        || register.records_disposition_performed
        || previews.iter().any(|preview| {
            !preview.approval_required || preview.approval_granted || preview.effect_performed
        })
        || reviews
            .iter()
            .any(|review| review.execution_authority_created || review.effect_performed)
        || !report.local_preview_only
        || report.communication_effect_performed
        || report.calendar_effect_performed
        || report.filesystem_effect_performed
        || report.records_disposition_performed
    {
        return Err(DocumentControlCoordinatorError::AuthorityViolation);
    }

    Ok(DocumentControlCoordinatorOutcome {
        register,
        previews,
        findings,
        reviews,
        report,
        admitted_skill_count: packages.len() as u32,
        external_effect_allowed: false,
    })
}

#[cfg(test)]
mod tests {
    use agentmage_kernel_contracts::{
        CONTRACT_SCHEMA_VERSION, DocumentActionKind, DocumentLifecycleState, DocumentRegisterEntry,
        DocumentRegisterKind, DocumentRegisterStatement, DocumentStatementClass,
        ExecutiveEvidenceState, ExecutiveSourceReference, ExecutiveSourceStore, MeetingFieldState,
    };
    use agentmage_kernel_engine::document_control::{
        seal_document_action_preview, seal_document_register,
    };

    use super::*;

    fn source() -> ExecutiveSourceReference {
        ExecutiveSourceReference {
            source_id: "source-1".to_owned(),
            object_id: "document-1".to_owned(),
            fragment: Some("approved local source".to_owned()),
            content_sha256: "a".repeat(64),
            observed_revision: Some("revision-1".to_owned()),
            store: ExecutiveSourceStore::PlainFolder,
        }
    }

    fn register() -> DocumentRegister {
        seal_document_register(DocumentRegister {
            schema_version: CONTRACT_SCHEMA_VERSION,
            register_id: "register-1".to_owned(),
            entries: vec![DocumentRegisterEntry {
                record_id: "record-1".to_owned(),
                kind: DocumentRegisterKind::Document,
                title: "Controlled Record".to_owned(),
                record_category: None,
                version: "1.0".to_owned(),
                lifecycle_state: DocumentLifecycleState::Draft,
                approval_id: None,
                named_parties: Vec::new(),
                quorum_or_status: None,
                quorum_or_status_evidence_state: ExecutiveEvidenceState::Unknown,
                attachments: Vec::new(),
                commitments: Vec::new(),
                deadlines: Vec::new(),
                statements: vec![DocumentRegisterStatement {
                    statement_id: "statement-1".to_owned(),
                    class: DocumentStatementClass::ObservedFact,
                    text: "Exact source-backed statement.".to_owned(),
                    evidence_state: ExecutiveEvidenceState::Confirmed,
                    owner: None,
                    owner_state: MeetingFieldState::Unknown,
                    due_date: None,
                    due_date_state: MeetingFieldState::Unknown,
                    source_ids: vec!["source-1".to_owned()],
                }],
                content_sha256: "b".repeat(64),
                source_path: "records/record-1.md".to_owned(),
                retention_schedule_id: None,
                accessibility_review_complete: false,
                supersedes_record_id: None,
                superseded_by_record_id: None,
                sources: vec![source()],
            }],
            register_sha256: String::new(),
            proposal_only: true,
            external_effects_performed: false,
            records_disposition_performed: false,
        })
        .expect("sealed register")
    }

    fn preview() -> DocumentActionPreview {
        seal_document_action_preview(DocumentActionPreview {
            schema_version: CONTRACT_SCHEMA_VERSION,
            preview_id: "preview-1".to_owned(),
            action: DocumentActionKind::SaveDraft,
            record_id: "record-1".to_owned(),
            source_path: None,
            destination_path: "drafts/record-1.md".to_owned(),
            source_sha256: None,
            proposed_sha256: "b".repeat(64),
            metadata_sha256: "c".repeat(64),
            content_preview: "Exact local draft preview.".to_owned(),
            metadata_preview: "Version 1.0; category unknown.".to_owned(),
            record_category: None,
            retention_schedule_id: None,
            preview_sha256: String::new(),
            approval_required: true,
            approval_granted: false,
            effect_performed: false,
        })
        .expect("sealed preview")
    }

    fn approval(preview: &DocumentActionPreview) -> DocumentActionApproval {
        DocumentActionApproval {
            approval_id: "approval-1".to_owned(),
            preview_sha256: preview.preview_sha256.clone(),
            destination_confirmed: true,
            content_confirmed: true,
            metadata_confirmed: true,
            record_category_confirmed: false,
            retention_confirmed: false,
            approved: true,
        }
    }

    fn request() -> DocumentControlCoordinatorRequest {
        DocumentControlCoordinatorRequest {
            dependencies_ready: true,
            cancellation_requested: false,
            report_id: "report-1".to_owned(),
            workflow_kind: DocumentWorkflowKind::Quality,
            rendered_preview: "Local records quality preview.".to_owned(),
        }
    }

    #[test]
    fn story_56_product_composition_binds_register_previews_reviews_and_eight_skills() {
        let exact_preview = preview();
        let outcome = coordinate_document_control_workspace(
            register(),
            vec![exact_preview.clone()],
            vec![Some(approval(&exact_preview))],
            request(),
        )
        .expect("document-control workspace");
        assert_eq!(outcome.admitted_skill_count, 8);
        assert!(outcome.reviews[0].approval_complete);
        assert!(!outcome.reviews[0].execution_authority_created);
        assert!(!outcome.report.filesystem_effect_performed);
        assert!(!outcome.report.records_disposition_performed);
        assert!(!outcome.external_effect_allowed);
    }

    #[test]
    fn story_56_incomplete_or_absent_approval_remains_visible_and_inert() {
        let exact_preview = preview();
        let outcome = coordinate_document_control_workspace(
            register(),
            vec![exact_preview],
            vec![None],
            request(),
        )
        .expect("incomplete review workspace");
        assert!(!outcome.reviews[0].approval_complete);
        assert_eq!(
            outcome.reviews[0].missing_confirmation_codes,
            ["approval.record.missing"]
        );
        assert!(!outcome.reviews[0].execution_authority_created);
        assert!(!outcome.reviews[0].effect_performed);
    }

    #[test]
    fn story_56_cross_record_stale_approval_and_precondition_fail_closed() {
        let exact_preview = preview();
        let mut stale = approval(&exact_preview);
        stale.preview_sha256 = "d".repeat(64);
        assert_eq!(
            coordinate_document_control_workspace(
                register(),
                vec![exact_preview.clone()],
                vec![Some(stale)],
                request(),
            ),
            Err(DocumentControlCoordinatorError::InvalidInput)
        );

        let mut cancelled = request();
        cancelled.cancellation_requested = true;
        cancelled.dependencies_ready = false;
        assert_eq!(
            coordinate_document_control_workspace(
                register(),
                vec![exact_preview.clone()],
                vec![None],
                cancelled,
            ),
            Err(DocumentControlCoordinatorError::Cancelled)
        );

        let mut unavailable = request();
        unavailable.dependencies_ready = false;
        assert_eq!(
            coordinate_document_control_workspace(
                register(),
                vec![exact_preview],
                vec![None],
                unavailable,
            ),
            Err(DocumentControlCoordinatorError::DependencyUnavailable)
        );
    }
}
