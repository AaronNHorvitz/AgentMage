//! Authority-free composition for Markdown artifact and exact-edit workflows.

use agentmage_capability_knowledge::{
    GeneratedMarkdownArtifact, MarkdownArtifactRequest, MarkdownDocument, MarkdownQualityProfile,
    MarkdownQualityReport, MarkdownRenderedBlock, MarkdownRoundTripResult, MarkdownUpdatePreview,
    MarkdownUpdateRequest, built_in_markdown_artifact_skill_pack, generate_markdown_artifact,
    preview_markdown_update, review_markdown_quality, verify_markdown_round_trip,
    verify_markdown_update_preview,
};

/// Stable failure from the local Markdown artifact composition.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MarkdownArtifactCoordinatorError {
    /// The caller cancelled before any projection began.
    Cancelled,
    /// A required already-approved local dependency is unavailable.
    DependencyUnavailable,
    /// Source, policy, edit, artifact, renderer, or reopen input failed closed.
    InvalidInput,
    /// The admitted built-in Markdown artifact skill pack is unavailable or incomplete.
    SkillPackInvalid,
    /// A supplied or derived record claimed a filesystem, network, or execution effect.
    AuthorityViolation,
}

impl MarkdownArtifactCoordinatorError {
    /// Stable content-free failure code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::Cancelled => "markdown-artifact.coordinator.cancelled",
            Self::DependencyUnavailable => "markdown-artifact.coordinator.dependency-unavailable",
            Self::InvalidInput => "markdown-artifact.coordinator.input-invalid",
            Self::SkillPackInvalid => "markdown-artifact.coordinator.skill-pack-invalid",
            Self::AuthorityViolation => "markdown-artifact.coordinator.authority-violation",
        }
    }
}

impl std::fmt::Display for MarkdownArtifactCoordinatorError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.code())
    }
}

impl std::error::Error for MarkdownArtifactCoordinatorError {}

/// Caller-owned policy, artifact, edit, and local-renderer inputs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MarkdownArtifactCoordinatorRequest {
    /// True only after every declared local dependency is available.
    pub dependencies_ready: bool,
    /// Sticky cancellation checked before dependency or content evaluation.
    pub cancellation_requested: bool,
    /// Exact deterministic quality policy.
    pub quality_profile: MarkdownQualityProfile,
    /// Evidence-state-aware local artifact request.
    pub artifact_request: MarkdownArtifactRequest,
    /// Optional exact structure-preserving edit request.
    pub update_request: Option<MarkdownUpdateRequest>,
    /// Exact bytes reopened from caller-owned local storage or an in-memory fixture.
    pub reopened_source: Vec<u8>,
    /// Content-free signatures supplied by an already-approved local renderer.
    pub original_rendered: Vec<MarkdownRenderedBlock>,
    /// Reopened content-free signatures from the same local renderer.
    pub reopened_rendered: Vec<MarkdownRenderedBlock>,
}

/// One coherent local Markdown workspace with no write, network, or execution authority.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MarkdownArtifactCoordinatorOutcome {
    /// Deterministic source-preserving quality review.
    pub quality: MarkdownQualityReport,
    /// Evidence-state-aware generated local artifact proposal.
    pub artifact: GeneratedMarkdownArtifact,
    /// Optional exact byte-preserving edit preview.
    pub update_preview: Option<MarkdownUpdatePreview>,
    /// Byte, semantic, and local-rendered round-trip result.
    pub round_trip: MarkdownRoundTripResult,
    /// Exact number of admitted authority-free built-in Markdown artifact skills.
    pub admitted_skill_count: u32,
    /// Fixed false: composition cannot write, fetch, render remotely, or execute content.
    pub external_effect_allowed: bool,
}

/// Coordinates quality, generation, optional exact edits, and local round-trip evidence.
pub fn coordinate_markdown_artifact_workspace(
    document: &MarkdownDocument,
    request: MarkdownArtifactCoordinatorRequest,
) -> Result<MarkdownArtifactCoordinatorOutcome, MarkdownArtifactCoordinatorError> {
    if request.cancellation_requested {
        return Err(MarkdownArtifactCoordinatorError::Cancelled);
    }
    if !request.dependencies_ready {
        return Err(MarkdownArtifactCoordinatorError::DependencyUnavailable);
    }

    let packages = built_in_markdown_artifact_skill_pack()
        .map_err(|_| MarkdownArtifactCoordinatorError::SkillPackInvalid)?;
    if packages.len() != 7 {
        return Err(MarkdownArtifactCoordinatorError::SkillPackInvalid);
    }

    let quality = review_markdown_quality(document, &request.quality_profile)
        .map_err(|_| MarkdownArtifactCoordinatorError::InvalidInput)?;
    let artifact = generate_markdown_artifact(request.artifact_request)
        .map_err(|_| MarkdownArtifactCoordinatorError::InvalidInput)?;
    let update_preview = request
        .update_request
        .map(|update| {
            let preview = preview_markdown_update(document, update)
                .map_err(|_| MarkdownArtifactCoordinatorError::InvalidInput)?;
            verify_markdown_update_preview(&preview)
                .map_err(|_| MarkdownArtifactCoordinatorError::InvalidInput)?;
            Ok(preview)
        })
        .transpose()?;
    let round_trip = verify_markdown_round_trip(
        document,
        request.reopened_source,
        &request.original_rendered,
        &request.reopened_rendered,
    )
    .map_err(|_| MarkdownArtifactCoordinatorError::InvalidInput)?;

    if quality.network_access_performed
        || quality.execution_performed
        || quality.source_mutation_performed
        || !artifact.proposal_only
        || artifact.filesystem_effect_performed
        || artifact.network_access_performed
        || round_trip.network_access_performed
        || round_trip.execution_performed
    {
        return Err(MarkdownArtifactCoordinatorError::AuthorityViolation);
    }

    Ok(MarkdownArtifactCoordinatorOutcome {
        quality,
        artifact,
        update_preview,
        round_trip,
        admitted_skill_count: packages.len() as u32,
        external_effect_allowed: false,
    })
}

#[cfg(test)]
mod tests {
    use agentmage_capability_knowledge::{
        MarkdownArtifactCitation, MarkdownArtifactKind, MarkdownArtifactSection,
        MarkdownArtifactStatement, MarkdownEdit,
    };
    use agentmage_kernel_contracts::{ExecutiveEvidenceState, WorkspaceId, WorkspacePath};

    use super::*;

    fn path() -> WorkspacePath {
        WorkspacePath::new(
            WorkspaceId::from_raw("workspace-markdown-coordinator"),
            ["notes", "report.md"],
        )
        .expect("workspace path")
    }

    fn source() -> Vec<u8> {
        b"---\nagentmage_id: knowledge-report-001\nstatus: current\n---\n# Report\n\n## Summary\nOriginal summary.\n".to_vec()
    }

    fn document() -> MarkdownDocument {
        MarkdownDocument::parse(path(), source()).expect("document")
    }

    fn rendered() -> Vec<MarkdownRenderedBlock> {
        vec![MarkdownRenderedBlock {
            ordinal: 1,
            kind: "document".to_owned(),
            heading_level: None,
            text_sha256: "a".repeat(64),
        }]
    }

    fn request(document: &MarkdownDocument) -> MarkdownArtifactCoordinatorRequest {
        let citation = MarkdownArtifactCitation {
            citation_id: "citation-1".to_owned(),
            source_path: path(),
            start_line: 7,
            end_line: 8,
            source_sha256: document.source_sha256().to_owned(),
        };
        MarkdownArtifactCoordinatorRequest {
            dependencies_ready: true,
            cancellation_requested: false,
            quality_profile: MarkdownQualityProfile {
                profile_id: "quality-1".to_owned(),
                maximum_sentence_words: 20,
                maximum_paragraph_bytes: 256,
                approved_acronyms: Vec::new(),
                approved_local_link_targets: Vec::new(),
            },
            artifact_request: MarkdownArtifactRequest {
                artifact_id: "artifact-1".to_owned(),
                kind: MarkdownArtifactKind::StatusReport,
                title: "Local Status".to_owned(),
                sections: vec![MarkdownArtifactSection {
                    section_id: "section-1".to_owned(),
                    heading: "Evidence".to_owned(),
                    statements: vec![MarkdownArtifactStatement {
                        statement_id: "statement-1".to_owned(),
                        text: "Source-grounded local statement.".to_owned(),
                        evidence_state: ExecutiveEvidenceState::Confirmed,
                        citation_ids: vec!["citation-1".to_owned()],
                    }],
                }],
                citations: vec![citation],
            },
            update_request: Some(MarkdownUpdateRequest {
                expected_source_sha256: document.source_sha256().to_owned(),
                expected_stable_id: document.stable_id().expect("stable id").clone(),
                edit: MarkdownEdit::ReplaceHeadingBody {
                    heading: "Summary".to_owned(),
                    level: 2,
                    replacement: "Revised summary.".to_owned(),
                },
            }),
            reopened_source: source(),
            original_rendered: rendered(),
            reopened_rendered: rendered(),
        }
    }

    #[test]
    fn story_57_product_composition_binds_quality_artifact_edit_round_trip_and_skills() {
        let document = document();
        let outcome = coordinate_markdown_artifact_workspace(&document, request(&document))
            .expect("Markdown workspace");
        assert_eq!(outcome.admitted_skill_count, 7);
        assert!(outcome.artifact.accessibility_structure_complete);
        assert!(outcome.update_preview.is_some());
        assert!(outcome.round_trip.locally_complete);
        assert!(!outcome.external_effect_allowed);
        assert!(!outcome.artifact.filesystem_effect_performed);
    }

    #[test]
    fn story_57_stale_edit_and_changed_round_trip_fail_or_remain_visible() {
        let document = document();
        let mut stale = request(&document);
        stale
            .update_request
            .as_mut()
            .expect("update")
            .expected_source_sha256 = "b".repeat(64);
        assert_eq!(
            coordinate_markdown_artifact_workspace(&document, stale),
            Err(MarkdownArtifactCoordinatorError::InvalidInput)
        );

        let mut changed = request(&document);
        changed.update_request = None;
        changed.reopened_source = b"# Changed\n".to_vec();
        changed.reopened_rendered[0].text_sha256 = "b".repeat(64);
        let outcome = coordinate_markdown_artifact_workspace(&document, changed)
            .expect("visible incomplete round trip");
        assert!(!outcome.round_trip.locally_complete);
        assert!(!outcome.round_trip.limitations.is_empty());
    }

    #[test]
    fn story_57_sticky_cancellation_and_dependency_failure_precede_content_evaluation() {
        let document = document();
        let mut cancelled = request(&document);
        cancelled.cancellation_requested = true;
        cancelled.dependencies_ready = false;
        assert_eq!(
            coordinate_markdown_artifact_workspace(&document, cancelled),
            Err(MarkdownArtifactCoordinatorError::Cancelled)
        );

        let mut unavailable = request(&document);
        unavailable.dependencies_ready = false;
        assert_eq!(
            coordinate_markdown_artifact_workspace(&document, unavailable),
            Err(MarkdownArtifactCoordinatorError::DependencyUnavailable)
        );
    }
}
