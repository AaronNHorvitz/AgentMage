//! Authority-free composition for Markdown artifact and exact-edit workflows.

use agentmage_capability_knowledge::{
    GeneratedMarkdownArtifact, MarkdownArtifactRequest, MarkdownDocument, MarkdownQualityProfile,
    MarkdownQualityReport, MarkdownRoundTripResult, MarkdownUpdatePreview, MarkdownUpdateRequest,
    built_in_markdown_artifact_skill_pack, generate_markdown_artifact, preview_markdown_update,
    render_markdown_structure, review_markdown_quality, verify_markdown_round_trip,
    verify_markdown_update_preview,
};
use agentmage_kernel_contracts::WorkspacePath;
use agentmage_kernel_engine::filesystem_control::{
    FileClassification, FilesystemOperationDraft, NewDestinationDraft,
};
use sha2::{Digest, Sha256};

use crate::headless::{ClientCommand, MarkdownClientCommand, ThinClientRequest};
use crate::knowledge_write::{KnowledgeFilesystemContext, compose_knowledge_update};

/// Caller-held absent destination for one separately approved generated Markdown write.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MarkdownGeneratedWriteRequest {
    /// Stable operation identity shown by the controlled-filesystem preview.
    pub operation_id: String,
    /// Exact held parent, output path, and bounded sibling observation.
    pub destination: NewDestinationDraft,
    /// Exact POSIX-compatible destination mode.
    pub mode: u32,
}

/// Caller-held exact source context for one separately approved Markdown update.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MarkdownUpdateWriteRequest {
    /// Stable operation identity shown by the controlled-filesystem preview.
    pub operation_id: String,
    /// Exact held source, bytes, ownership disposition, and mode.
    pub context: KnowledgeFilesystemContext,
}

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
    /// Validated workspace-relative destination identity for the generated proposal.
    pub artifact_output_path: WorkspacePath,
    /// Optional exact structure-preserving edit request.
    pub update_request: Option<MarkdownUpdateRequest>,
    /// Exact bytes reopened from caller-owned local storage or an in-memory fixture.
    pub reopened_source: Vec<u8>,
    /// Optional conversion of the generated proposal into a controlled create draft.
    pub generated_write: Option<MarkdownGeneratedWriteRequest>,
    /// Optional conversion of the update preview into a controlled exact-patch draft.
    pub update_write: Option<MarkdownUpdateWriteRequest>,
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
    /// Authority-free controlled-filesystem drafts for the exact generated and edited proposals.
    pub controlled_write_drafts: Vec<FilesystemOperationDraft>,
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
    if request.artifact_output_path.workspace_id() != document.path().workspace_id()
        || request.artifact_output_path == *document.path()
    {
        return Err(MarkdownArtifactCoordinatorError::InvalidInput);
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
    let reopened =
        MarkdownDocument::parse(document.path().clone(), request.reopened_source.clone())
            .map_err(|_| MarkdownArtifactCoordinatorError::InvalidInput)?;
    let original_rendered = render_markdown_structure(document);
    let reopened_rendered = render_markdown_structure(&reopened);
    let round_trip = verify_markdown_round_trip(
        document,
        request.reopened_source,
        &original_rendered,
        &reopened_rendered,
    )
    .map_err(|_| MarkdownArtifactCoordinatorError::InvalidInput)?;

    let mut controlled_write_drafts = Vec::new();
    if let Some(write) = request.generated_write {
        controlled_write_drafts.push(prepare_generated_markdown_write(
            &artifact,
            &request.artifact_output_path,
            write,
        )?);
    }
    match (update_preview.as_ref(), request.update_write) {
        (Some(preview), Some(write)) => controlled_write_drafts.push(
            compose_knowledge_update(write.operation_id, preview, write.context)
                .map_err(|_| MarkdownArtifactCoordinatorError::InvalidInput)?,
        ),
        (None, Some(_)) => return Err(MarkdownArtifactCoordinatorError::InvalidInput),
        (_, None) => {}
    }

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
        controlled_write_drafts,
        admitted_skill_count: packages.len() as u32,
        external_effect_allowed: false,
    })
}

/// Verifies an identity-only thin-client request and routes it through the common coordinator.
pub fn coordinate_markdown_artifact_native(
    client: &ThinClientRequest,
    document: &MarkdownDocument,
    request: MarkdownArtifactCoordinatorRequest,
    now_epoch_ms: u64,
) -> Result<MarkdownArtifactCoordinatorOutcome, MarkdownArtifactCoordinatorError> {
    client
        .verify(now_epoch_ms)
        .map_err(|_| MarkdownArtifactCoordinatorError::InvalidInput)?;
    if client.status.workspace_id != document.path().workspace_id().as_str()
        || request.generated_write.is_some()
        || request.update_write.is_some()
        || !native_command_matches(&client.command, document, &request)
    {
        return Err(MarkdownArtifactCoordinatorError::InvalidInput);
    }
    coordinate_markdown_artifact_workspace(document, request)
}

fn native_command_matches(
    command: &ClientCommand,
    document: &MarkdownDocument,
    request: &MarkdownArtifactCoordinatorRequest,
) -> bool {
    let ClientCommand::Markdown {
        action:
            MarkdownClientCommand::Coordinate {
                source_sha256,
                quality_profile_id,
                artifact_id,
                artifact_output_path,
                reopened_sha256,
                update_requested,
            },
    } = command
    else {
        return false;
    };
    source_sha256 == document.source_sha256()
        && quality_profile_id == &request.quality_profile.profile_id
        && artifact_id == &request.artifact_request.artifact_id
        && reopened_sha256 == &hex_sha256(&request.reopened_source)
        && *update_requested == request.update_request.is_some()
        && request
            .artifact_output_path
            .components()
            .iter()
            .map(|component| component.as_str())
            .eq(artifact_output_path.iter().map(String::as_str))
}

/// Converts one exact generated Markdown proposal into an unapproved controlled create draft.
pub fn prepare_generated_markdown_write(
    artifact: &GeneratedMarkdownArtifact,
    output_path: &WorkspacePath,
    write: MarkdownGeneratedWriteRequest,
) -> Result<FilesystemOperationDraft, MarkdownArtifactCoordinatorError> {
    if !valid_identifier(&write.operation_id)
        || !matches!(write.mode, 0o600 | 0o640 | 0o644 | 0o700 | 0o740 | 0o755)
        || &write.destination.path != output_path
        || output_path
            .components()
            .last()
            .is_none_or(|component| !component.as_str().ends_with(".md"))
        || artifact.markdown.is_empty()
        || hex_sha256(&artifact.markdown) != artifact.markdown_sha256
        || !artifact.accessibility_structure_complete
        || !artifact.proposal_only
        || artifact.filesystem_effect_performed
        || artifact.network_access_performed
    {
        return Err(MarkdownArtifactCoordinatorError::InvalidInput);
    }
    Ok(FilesystemOperationDraft::Create {
        operation_id: write.operation_id,
        destination: write.destination,
        content: artifact.markdown.clone(),
        mode: write.mode,
        classification: FileClassification::Generated,
    })
}

fn valid_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value.bytes().enumerate().all(|(index, byte)| {
            byte.is_ascii_alphanumeric() || (index > 0 && matches!(byte, b'.' | b'_' | b':' | b'-'))
        })
}

fn hex_sha256(value: &[u8]) -> String {
    Sha256::digest(value)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[cfg(test)]
mod tests {
    use agentmage_capability_knowledge::{
        MarkdownArtifactCitation, MarkdownArtifactKind, MarkdownArtifactSection,
        MarkdownArtifactStatement, MarkdownEdit,
    };
    use agentmage_kernel_contracts::{
        ExecutiveEvidenceState, GrantTarget, WorkspaceId, WorkspacePath,
    };
    use agentmage_kernel_engine::filesystem_control::ExistingWorkDisposition;
    use serde_json::json;

    use crate::headless::{
        ClientAuthority, ClientStatusSnapshot, ClientSurface, PredeclaredClientGrant,
        kernel_operation_sha256,
    };

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

    fn directory_target() -> GrantTarget {
        serde_json::from_value(json!({
            "target_kind": "held_object",
            "path": WorkspacePath::new(
                WorkspaceId::from_raw("workspace-markdown-coordinator"),
                ["generated"],
            ).expect("directory path"),
            "authorization_id": "authorization-markdown-host",
            "adapter_instance_id": "adapter-markdown-host",
            "platform": "deterministic_fake",
            "object_kind": "directory",
            "object_identity": {
                "platform": "deterministic_fake",
                "mount_identity_sha256": vec![1_u8; 32],
                "object_identity_sha256": vec![2_u8; 32]
            },
            "preimage": null
        }))
        .expect("directory target")
    }

    fn file_target(bytes: &[u8]) -> GrantTarget {
        let content: [u8; 32] = Sha256::digest(bytes).into();
        serde_json::from_value(json!({
            "target_kind": "held_object",
            "path": path(),
            "authorization_id": "authorization-markdown-host",
            "adapter_instance_id": "adapter-markdown-host",
            "platform": "deterministic_fake",
            "object_kind": "regular_file",
            "object_identity": {
                "platform": "deterministic_fake",
                "mount_identity_sha256": vec![1_u8; 32],
                "object_identity_sha256": vec![3_u8; 32]
            },
            "preimage": {"byte_len": bytes.len(), "content_sha256": content}
        }))
        .expect("file target")
    }

    fn native_client(surface: ClientSurface, command: ClientCommand) -> ThinClientRequest {
        const NOW: u64 = 50_000;
        let operation = command.required_operation();
        let arguments_sha256 =
            kernel_operation_sha256("workspace-markdown-coordinator", &command, &"d".repeat(64))
                .expect("kernel operation");
        let authority = if surface.has_interactive_approval() {
            ClientAuthority::Interactive {
                approval_channel_sha256: "c".repeat(64),
            }
        } else {
            ClientAuthority::Predeclared {
                grant: PredeclaredClientGrant {
                    grant_id: format!("grant-{surface:?}").to_lowercase(),
                    operation,
                    policy_sha256: "d".repeat(64),
                    arguments_sha256,
                    nonce_sha256: "f".repeat(64),
                    issued_at_epoch_ms: NOW - 1,
                    expires_at_epoch_ms: NOW + 10_000,
                    single_use: true,
                },
            }
        };
        ThinClientRequest {
            schema_version: 0,
            request_id: format!("request-{surface:?}").to_lowercase(),
            surface,
            status: ClientStatusSnapshot {
                workspace_id: "workspace-markdown-coordinator".to_owned(),
                model_profile_id: None,
                permission_profile_id: "permission-markdown".to_owned(),
                conversation_id: None,
                plan_step_id: None,
                writable_roots: vec!["generated".to_owned()],
                offline: true,
                status_sha256: "0".repeat(64),
            }
            .seal()
            .expect("status"),
            command,
            authority,
            policy_sha256: "d".repeat(64),
            cancellation_id: format!("cancel-{surface:?}").to_lowercase(),
            max_event_bytes: 64 * 1024,
            max_output_bytes: 1024 * 1024,
            resume: None,
            kernel_request_sha256: "0".repeat(64),
            request_sha256: "0".repeat(64),
        }
        .seal(NOW)
        .expect("native client")
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
            artifact_output_path: WorkspacePath::new(
                WorkspaceId::from_raw("workspace-markdown-coordinator"),
                ["generated", "status.md"],
            )
            .expect("artifact output path"),
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
            generated_write: None,
            update_write: None,
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
        assert!(outcome.controlled_write_drafts.is_empty());
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

    #[test]
    fn generated_and_updated_markdown_enter_only_exact_unapproved_controlled_drafts() {
        let document = document();
        let mut detailed = request(&document);
        detailed.generated_write = Some(MarkdownGeneratedWriteRequest {
            operation_id: "write-generated-markdown".to_owned(),
            destination: NewDestinationDraft {
                parent: directory_target(),
                path: detailed.artifact_output_path.clone(),
                observed_sibling_names: vec!["existing.md".to_owned()],
            },
            mode: 0o600,
        });
        detailed.update_write = Some(MarkdownUpdateWriteRequest {
            operation_id: "patch-existing-markdown".to_owned(),
            context: KnowledgeFilesystemContext::Update {
                source_target: file_target(document.source_bytes()),
                observed_source_bytes: document.source_bytes().to_vec(),
                mode: 0o600,
                work_disposition: ExistingWorkDisposition::OwnedByCurrentTask,
            },
        });
        let outcome = coordinate_markdown_artifact_workspace(&document, detailed)
            .expect("controlled Markdown drafts");
        assert_eq!(outcome.controlled_write_drafts.len(), 2);
        assert!(matches!(
            &outcome.controlled_write_drafts[0],
            FilesystemOperationDraft::Create { content, .. } if content == &outcome.artifact.markdown
        ));
        assert!(matches!(
            &outcome.controlled_write_drafts[1],
            FilesystemOperationDraft::ExactPatch { expected_postimage_sha256, .. }
                if expected_postimage_sha256
                    == &outcome.update_preview.as_ref().expect("preview").proposed_source_sha256
        ));
        assert!(!outcome.external_effect_allowed);
    }

    #[test]
    fn every_native_surface_uses_one_identity_bound_markdown_coordinator() {
        const NOW: u64 = 50_000;
        let document = document();
        let detailed = request(&document);
        let command = ClientCommand::Markdown {
            action: MarkdownClientCommand::Coordinate {
                source_sha256: document.source_sha256().to_owned(),
                quality_profile_id: detailed.quality_profile.profile_id.clone(),
                artifact_id: detailed.artifact_request.artifact_id.clone(),
                artifact_output_path: detailed
                    .artifact_output_path
                    .components()
                    .iter()
                    .map(|component| component.as_str().to_owned())
                    .collect(),
                reopened_sha256: hex_sha256(&detailed.reopened_source),
                update_requested: true,
            },
        };
        let mut expected = None;
        for surface in [
            ClientSurface::NativeChat,
            ClientSurface::InteractiveCli,
            ClientSurface::Json,
            ClientSurface::Sdk,
            ClientSurface::Acp,
        ] {
            let outcome = coordinate_markdown_artifact_native(
                &native_client(surface, command.clone()),
                &document,
                detailed.clone(),
                NOW,
            )
            .unwrap_or_else(|error| panic!("{surface:?}: {error:?}"));
            let snapshot = (
                outcome.quality,
                outcome.artifact,
                outcome.update_preview,
                outcome.round_trip,
                outcome.external_effect_allowed,
            );
            if let Some(expected) = &expected {
                assert_eq!(&snapshot, expected, "{surface:?}");
            } else {
                expected = Some(snapshot);
            }
        }
    }

    #[test]
    fn native_markdown_binding_rejects_mismatch_and_write_context_smuggling() {
        const NOW: u64 = 50_000;
        let document = document();
        let detailed = request(&document);
        let command = ClientCommand::Markdown {
            action: MarkdownClientCommand::Coordinate {
                source_sha256: "b".repeat(64),
                quality_profile_id: detailed.quality_profile.profile_id.clone(),
                artifact_id: detailed.artifact_request.artifact_id.clone(),
                artifact_output_path: vec!["generated".to_owned(), "status.md".to_owned()],
                reopened_sha256: hex_sha256(&detailed.reopened_source),
                update_requested: true,
            },
        };
        assert_eq!(
            coordinate_markdown_artifact_native(
                &native_client(ClientSurface::Json, command),
                &document,
                detailed.clone(),
                NOW,
            ),
            Err(MarkdownArtifactCoordinatorError::InvalidInput)
        );

        let command = ClientCommand::Markdown {
            action: MarkdownClientCommand::Coordinate {
                source_sha256: document.source_sha256().to_owned(),
                quality_profile_id: detailed.quality_profile.profile_id.clone(),
                artifact_id: detailed.artifact_request.artifact_id.clone(),
                artifact_output_path: vec!["generated".to_owned(), "status.md".to_owned()],
                reopened_sha256: hex_sha256(&detailed.reopened_source),
                update_requested: true,
            },
        };
        let mut smuggled = detailed;
        smuggled.generated_write = Some(MarkdownGeneratedWriteRequest {
            operation_id: "smuggled-write".to_owned(),
            destination: NewDestinationDraft {
                parent: directory_target(),
                path: smuggled.artifact_output_path.clone(),
                observed_sibling_names: Vec::new(),
            },
            mode: 0o600,
        });
        assert_eq!(
            coordinate_markdown_artifact_native(
                &native_client(ClientSurface::Json, command),
                &document,
                smuggled,
                NOW,
            ),
            Err(MarkdownArtifactCoordinatorError::InvalidInput)
        );
    }
}
