//! Authority-free product composition for bounded Word artifact workflows.

use agentmage_capability_knowledge::{
    GeneratedWordPackage, MarkdownDocument, WordConversionProfile, WordExtractionResult,
    WordInspectionReport, WordSidecarCache, generate_docx_from_markdown, inspect_docx,
};
use agentmage_capability_read_only::ArtifactClassification;
use agentmage_kernel_contracts::{
    StructuredSourceExtractionError, StructuredSourceExtractionRequest, WorkspacePath,
};
use agentmage_kernel_engine::filesystem_control::{
    FileClassification, FilesystemOperationDraft, NewDestinationDraft,
};
use sha2::{Digest, Sha256};

use crate::headless::{ClientCommand, ThinClientRequest, WordClientCommand};
use crate::word_source_artifact::{WordSourceAdmissionOutcome, WordSourceArtifactService};

/// Optional caller-owned request for one unpersisted generated DOCX proposal.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WordGenerationCoordinatorRequest {
    /// Stable generated-artifact identity.
    pub artifact_id: String,
    /// Exact parsed Markdown source.
    pub document: MarkdownDocument,
    /// Validated workspace-relative output identity distinct from the Markdown source.
    pub output_path: WorkspacePath,
}

/// Caller-held absent destination for a separately approved controlled write.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WordControlledWriteRequest {
    /// Stable operation identity shown in the controlled-filesystem preview.
    pub operation_id: String,
    /// Exact held parent, output path, and sibling observation.
    pub destination: NewDestinationDraft,
    /// Exact POSIX-compatible destination mode.
    pub mode: u32,
}

/// Exact caller-owned inputs for one bounded Word product operation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WordArtifactCoordinatorRequest {
    /// True only after every declared local dependency is available.
    pub dependencies_ready: bool,
    /// Sticky cancellation checked before dependency or content evaluation.
    pub cancellation_requested: bool,
    /// Complete immutable source DOCX bytes obtained through caller-owned read authority.
    pub source: Vec<u8>,
    /// Exact bounded OOXML conversion profile.
    pub profile: WordConversionProfile,
    /// Shared structured-source request bound to the same source path and digest.
    pub structured_source: StructuredSourceExtractionRequest,
    /// Sensitivity applied to the common native artifact projection.
    pub classification: ArtifactClassification,
    /// Content-free source class such as attachment or generated.
    pub source_kind: String,
    /// Digest of protected origin metadata; never a raw path or URI.
    pub protected_origin_sha256: String,
    /// Optional deterministic Markdown-to-DOCX proposal.
    pub generation: Option<WordGenerationCoordinatorRequest>,
    /// Optional conversion of the generated proposal into a controlled-filesystem draft.
    pub controlled_write: Option<WordControlledWriteRequest>,
}

/// Stable content-free failure from Word artifact product composition.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WordArtifactCoordinatorError {
    /// The thin-client request was stale, malformed, mismatched, or authority-incompatible.
    NativeRequestDenied,
    /// The caller cancelled before publication.
    Cancelled,
    /// A required already-approved local dependency is unavailable.
    DependencyUnavailable,
    /// A source, profile, identity, extraction, or generation input failed closed.
    InvalidInput,
    /// A supplied or derived record claimed a write, network, or execution effect.
    AuthorityViolation,
}

impl WordArtifactCoordinatorError {
    /// Stable content-free error code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::NativeRequestDenied => "word-artifact.coordinator.native-request-denied",
            Self::Cancelled => "word-artifact.coordinator.cancelled",
            Self::DependencyUnavailable => "word-artifact.coordinator.dependency-unavailable",
            Self::InvalidInput => "word-artifact.coordinator.input-invalid",
            Self::AuthorityViolation => "word-artifact.coordinator.authority-violation",
        }
    }
}

impl std::fmt::Display for WordArtifactCoordinatorError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.code())
    }
}

impl std::error::Error for WordArtifactCoordinatorError {}

/// One coherent Word product result with no persistence or external-effect authority.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WordArtifactCoordinatorOutcome {
    /// Complete bounded inspection of the immutable source package.
    pub inspection: WordInspectionReport,
    /// Deterministic text sidecar and exact part-range provenance.
    pub extraction: WordExtractionResult,
    /// True only when the exact sidecar result already existed in the bounded cache.
    pub sidecar_cache_hit: bool,
    /// Runtime-owned canonical source projection and content-free prepared manifest.
    pub source_admission: WordSourceAdmissionOutcome,
    /// Optional deterministic unpersisted DOCX proposal.
    pub generated: Option<GeneratedWordPackage>,
    /// Optional exact binary create draft; it carries no approval or effect authority.
    pub controlled_write_draft: Option<FilesystemOperationDraft>,
    /// Fixed false: this coordinator cannot write, fetch, render, or execute content.
    pub external_effect_allowed: bool,
}

/// Stateful bounded product coordinator sharing only in-memory projection caches.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WordArtifactCoordinator {
    sidecars: WordSidecarCache,
    sources: WordSourceArtifactService,
}

impl WordArtifactCoordinator {
    /// Creates one coordinator with a fixed nonzero sidecar-cache ceiling.
    pub fn new(maximum_sidecar_entries: usize) -> Result<Self, WordArtifactCoordinatorError> {
        Ok(Self {
            sidecars: WordSidecarCache::new(maximum_sidecar_entries)
                .map_err(|_| WordArtifactCoordinatorError::InvalidInput)?,
            sources: WordSourceArtifactService::default(),
        })
    }

    /// Coordinates inspection, extraction, shared-source preparation, and optional generation.
    pub fn coordinate(
        &mut self,
        request: WordArtifactCoordinatorRequest,
    ) -> Result<WordArtifactCoordinatorOutcome, WordArtifactCoordinatorError> {
        if request.cancellation_requested {
            return Err(WordArtifactCoordinatorError::Cancelled);
        }
        if !request.dependencies_ready {
            return Err(WordArtifactCoordinatorError::DependencyUnavailable);
        }

        let inspection = inspect_docx(
            &request.structured_source.source_path,
            &request.source,
            &request.profile,
        )
        .map_err(|_| WordArtifactCoordinatorError::InvalidInput)?;
        let sidecar = self
            .sidecars
            .get_or_extract(
                &request.structured_source.source_path,
                &request.source,
                &request.profile,
            )
            .map_err(|_| WordArtifactCoordinatorError::InvalidInput)?;
        let mut never_cancelled = || false;
        let source_admission = self
            .sources
            .admit(
                &request.structured_source,
                &request.source,
                request.classification,
                &request.source_kind,
                &request.protected_origin_sha256,
                &mut never_cancelled,
            )
            .map_err(map_source_error)?;
        let generated = request
            .generation
            .map(|generation| {
                generate_docx_from_markdown(
                    &generation.artifact_id,
                    generation.output_path,
                    &generation.document,
                    &request.profile,
                )
                .map_err(|_| WordArtifactCoordinatorError::InvalidInput)
            })
            .transpose()?;
        let controlled_write_draft = match (&generated, request.controlled_write) {
            (Some(proposal), Some(write)) => Some(prepare_generated_word_write(
                proposal,
                write.operation_id,
                write.destination,
                write.mode,
            )?),
            (None, Some(_)) => return Err(WordArtifactCoordinatorError::InvalidInput),
            (_, None) => None,
        };

        if inspection.filesystem_effect_performed
            || inspection.network_access_performed
            || inspection.execution_performed
            || sidecar.result.filesystem_effect_performed
            || sidecar.result.network_access_performed
            || sidecar.result.execution_performed
            || generated.as_ref().is_some_and(|proposal| {
                !proposal.proposal_only
                    || proposal.filesystem_effect_performed
                    || proposal.network_access_performed
                    || proposal.execution_performed
            })
        {
            return Err(WordArtifactCoordinatorError::AuthorityViolation);
        }

        Ok(WordArtifactCoordinatorOutcome {
            inspection,
            extraction: sidecar.result,
            sidecar_cache_hit: sidecar.cache_hit,
            source_admission,
            generated,
            controlled_write_draft,
            external_effect_allowed: false,
        })
    }

    /// Verifies an identity-only thin-client request and routes it through this same coordinator.
    ///
    /// Document bytes and parsed Markdown remain trusted host inputs and never cross the client
    /// protocol. Native Chat, terminal, JSON, SDK, and ACP therefore share one request binding and
    /// one product implementation without owning parser or filesystem authority.
    pub fn coordinate_native(
        &mut self,
        client: &ThinClientRequest,
        request: WordArtifactCoordinatorRequest,
        now_epoch_ms: u64,
    ) -> Result<WordArtifactCoordinatorOutcome, WordArtifactCoordinatorError> {
        client
            .verify(now_epoch_ms)
            .map_err(|_| WordArtifactCoordinatorError::NativeRequestDenied)?;
        if client.status.workspace_id
            != request
                .structured_source
                .source_path
                .workspace_id()
                .as_str()
            || !native_command_matches(&client.command, &request)
        {
            return Err(WordArtifactCoordinatorError::NativeRequestDenied);
        }
        self.coordinate(request)
    }
}

fn native_command_matches(
    command: &ClientCommand,
    request: &WordArtifactCoordinatorRequest,
) -> bool {
    let common_matches = |source_id: &str, source_sha256: &str, profile_id: &str| {
        source_id == request.structured_source.source_id
            && source_sha256 == request.structured_source.source_sha256
            && profile_id == request.profile.profile_id
            && request.controlled_write.is_none()
    };
    match command {
        ClientCommand::Word {
            action:
                WordClientCommand::Inspect {
                    source_id,
                    source_sha256,
                    profile_id,
                },
        } => common_matches(source_id, source_sha256, profile_id) && request.generation.is_none(),
        ClientCommand::Word {
            action:
                WordClientCommand::Generate {
                    source_id,
                    source_sha256,
                    profile_id,
                    artifact_id,
                    markdown_source_sha256,
                    output_path,
                },
        } => {
            common_matches(source_id, source_sha256, profile_id)
                && request.generation.as_ref().is_some_and(|generation| {
                    artifact_id == &generation.artifact_id
                        && markdown_source_sha256 == generation.document.source_sha256()
                        && generation
                            .output_path
                            .components()
                            .iter()
                            .map(|component| component.as_str())
                            .eq(output_path.iter().map(String::as_str))
                })
        }
        _ => false,
    }
}

/// Converts one exact verified DOCX proposal into the existing controlled-filesystem draft type.
///
/// The returned draft is still authority-free. A separate current session grant, exact plan and
/// preview, explicit decision, single-use write grant, native driver, and post-write verification
/// remain mandatory before bytes can be persisted.
pub fn prepare_generated_word_write(
    proposal: &GeneratedWordPackage,
    operation_id: String,
    destination: NewDestinationDraft,
    mode: u32,
) -> Result<FilesystemOperationDraft, WordArtifactCoordinatorError> {
    if !valid_identifier(&operation_id)
        || !matches!(mode, 0o600 | 0o640 | 0o644 | 0o700 | 0o740 | 0o755)
        || destination.path != proposal.output_path
        || proposal.package.is_empty()
        || hex_sha256(&proposal.package) != proposal.package_sha256
        || proposal.inspection.source_sha256 != proposal.package_sha256
        || proposal.inspection.source_path != proposal.output_path
        || proposal.inspection.quarantined
        || !proposal.inspection.inspection_complete
        || !proposal.accessibility_structure_complete
        || !proposal.proposal_only
        || proposal.filesystem_effect_performed
        || proposal.network_access_performed
        || proposal.execution_performed
    {
        return Err(WordArtifactCoordinatorError::InvalidInput);
    }
    Ok(FilesystemOperationDraft::Create {
        operation_id,
        destination,
        content: proposal.package.clone(),
        mode,
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

const fn map_source_error(error: StructuredSourceExtractionError) -> WordArtifactCoordinatorError {
    match error {
        StructuredSourceExtractionError::Cancelled => WordArtifactCoordinatorError::Cancelled,
        StructuredSourceExtractionError::InvalidInput
        | StructuredSourceExtractionError::Unsupported
        | StructuredSourceExtractionError::ResourceLimit
        | StructuredSourceExtractionError::Quarantined
        | StructuredSourceExtractionError::Malformed => WordArtifactCoordinatorError::InvalidInput,
    }
}

#[cfg(test)]
mod tests {
    use agentmage_kernel_contracts::{
        AdapterInstanceId, CONTRACT_SCHEMA_VERSION, FilePreimage, GrantOperation, GrantTarget,
        HeldWorkspaceObject, PathPlatform, PathResolutionIntent, WorkspaceAuthorizationId,
        WorkspaceId, WorkspaceObjectIdentity, WorkspaceObjectKind,
    };
    use sha2::{Digest, Sha256};

    use super::*;
    use crate::headless::{
        ClientAuthority, ClientStatusSnapshot, ClientSurface, PredeclaredClientGrant,
        kernel_operation_sha256,
    };

    const NOW: u64 = 50_000;

    #[derive(Debug)]
    struct HeldDirectory {
        path: WorkspacePath,
        authorization_id: WorkspaceAuthorizationId,
        adapter_instance_id: AdapterInstanceId,
        identity: WorkspaceObjectIdentity,
    }

    impl HeldWorkspaceObject for HeldDirectory {
        fn workspace_path(&self) -> &WorkspacePath {
            &self.path
        }

        fn authorization_id(&self) -> &WorkspaceAuthorizationId {
            &self.authorization_id
        }

        fn adapter_instance_id(&self) -> &AdapterInstanceId {
            &self.adapter_instance_id
        }

        fn intent(&self) -> PathResolutionIntent {
            PathResolutionIntent::Metadata
        }

        fn object_kind(&self) -> WorkspaceObjectKind {
            WorkspaceObjectKind::Directory
        }

        fn object_identity(&self) -> &WorkspaceObjectIdentity {
            &self.identity
        }

        fn preimage(&self) -> Option<&FilePreimage> {
            None
        }
    }

    fn path(name: &str) -> WorkspacePath {
        WorkspacePath::new(
            WorkspaceId::from_raw("workspace-word-coordinator"),
            ["documents", name],
        )
        .expect("path")
    }

    fn digest(bytes: &[u8]) -> String {
        Sha256::digest(bytes)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect()
    }

    fn markdown() -> MarkdownDocument {
        MarkdownDocument::parse(
            path("source.md"),
            b"# Status\n\n- Complete\n\n| Item | State |\n| --- | --- |\n| Word | Ready |\n"
                .to_vec(),
        )
        .expect("Markdown")
    }

    fn request() -> WordArtifactCoordinatorRequest {
        let profile = WordConversionProfile::strict_default();
        let source =
            generate_docx_from_markdown("source-docx", path("source.docx"), &markdown(), &profile)
                .expect("source package")
                .package;
        WordArtifactCoordinatorRequest {
            dependencies_ready: true,
            cancellation_requested: false,
            structured_source: StructuredSourceExtractionRequest {
                schema_version: CONTRACT_SCHEMA_VERSION,
                source_id: "word-source-1".to_owned(),
                media_type:
                    "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
                        .to_owned(),
                source_sha256: digest(&source),
                source_path: path("source.docx"),
                maximum_sections: 128,
                maximum_output_bytes: 1_048_576,
            },
            source,
            profile,
            classification: ArtifactClassification::Internal,
            source_kind: "attachment".to_owned(),
            protected_origin_sha256: "a".repeat(64),
            generation: Some(WordGenerationCoordinatorRequest {
                artifact_id: "generated-docx".to_owned(),
                document: markdown(),
                output_path: path("generated.docx"),
            }),
            controlled_write: None,
        }
    }

    fn generated_destination() -> NewDestinationDraft {
        let held = HeldDirectory {
            path: WorkspacePath::new(
                WorkspaceId::from_raw("workspace-word-coordinator"),
                ["documents"],
            )
            .expect("parent path"),
            authorization_id: WorkspaceAuthorizationId::from_raw("authorization-word-write"),
            adapter_instance_id: AdapterInstanceId::from_raw("adapter-word-write"),
            identity: WorkspaceObjectIdentity::new(PathPlatform::Linux, [1; 32], [2; 32]),
        };
        NewDestinationDraft {
            parent: GrantTarget::held_object(&held).expect("held destination parent"),
            path: path("generated.docx"),
            observed_sibling_names: vec!["source.docx".to_owned()],
        }
    }

    fn native_client(surface: ClientSurface, command: ClientCommand) -> ThinClientRequest {
        let operation = command.required_operation();
        let arguments_sha256 =
            kernel_operation_sha256("workspace-word-coordinator", &command, &"d".repeat(64))
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
                workspace_id: "workspace-word-coordinator".to_owned(),
                model_profile_id: None,
                permission_profile_id: "permission-word".to_owned(),
                conversation_id: None,
                plan_step_id: None,
                writable_roots: vec!["documents".to_owned()],
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
        .expect("native client request")
    }

    #[test]
    fn product_coordinator_binds_inspection_sidecar_source_projection_and_generation() {
        let mut coordinator = WordArtifactCoordinator::new(4).expect("coordinator");
        let outcome = coordinator.coordinate(request()).expect("Word workspace");
        assert!(outcome.inspection.inspection_complete);
        assert!(!outcome.inspection.quarantined);
        assert!(!outcome.extraction.sidecar.is_empty());
        assert_eq!(
            outcome.extraction.source_sha256,
            outcome.source_admission.prepared_manifest.source_sha256
        );
        let generated = outcome.generated.expect("generated package");
        assert!(generated.accessibility_structure_complete);
        assert!(generated.proposal_only);
        assert!(outcome.controlled_write_draft.is_none());
        assert!(!outcome.external_effect_allowed);
    }

    #[test]
    fn exact_replay_reuses_both_projection_caches_without_storing_source_authority() {
        let mut coordinator = WordArtifactCoordinator::new(4).expect("coordinator");
        let first = coordinator.coordinate(request()).expect("first");
        let second = coordinator.coordinate(request()).expect("second");
        assert!(!first.sidecar_cache_hit);
        assert!(!first.source_admission.cache_hit);
        assert!(second.sidecar_cache_hit);
        assert!(second.source_admission.cache_hit);
        assert_eq!(first.extraction, second.extraction);
        assert_eq!(
            first.source_admission.manifest,
            second.source_admission.manifest
        );
        assert_eq!(
            first.source_admission.prepared_manifest,
            second.source_admission.prepared_manifest
        );
    }

    #[test]
    fn cancellation_dependency_source_and_generation_fail_closed() {
        let mut coordinator = WordArtifactCoordinator::new(4).expect("coordinator");
        let mut cancelled = request();
        cancelled.cancellation_requested = true;
        cancelled.dependencies_ready = false;
        assert_eq!(
            coordinator.coordinate(cancelled),
            Err(WordArtifactCoordinatorError::Cancelled)
        );

        let mut unavailable = request();
        unavailable.dependencies_ready = false;
        assert_eq!(
            coordinator.coordinate(unavailable),
            Err(WordArtifactCoordinatorError::DependencyUnavailable)
        );

        let mut stale = request();
        stale.structured_source.source_sha256 = "b".repeat(64);
        assert_eq!(
            coordinator.coordinate(stale),
            Err(WordArtifactCoordinatorError::InvalidInput)
        );

        let mut overwrite = request();
        overwrite
            .generation
            .as_mut()
            .expect("generation")
            .output_path = path("source.md");
        assert_eq!(
            coordinator.coordinate(overwrite),
            Err(WordArtifactCoordinatorError::InvalidInput)
        );
    }

    #[test]
    fn generated_binary_enters_controlled_writer_only_as_an_unapproved_exact_draft() {
        let mut coordinator = WordArtifactCoordinator::new(4).expect("coordinator");
        let mut request = request();
        request.controlled_write = Some(WordControlledWriteRequest {
            operation_id: "write-generated-docx".to_owned(),
            destination: generated_destination(),
            mode: 0o600,
        });
        let outcome = coordinator.coordinate(request).expect("controlled draft");
        let generated = outcome.generated.expect("generated package");
        match outcome.controlled_write_draft.expect("write draft") {
            FilesystemOperationDraft::Create {
                operation_id,
                destination,
                content,
                mode,
                classification,
            } => {
                assert_eq!(operation_id, "write-generated-docx");
                assert_eq!(destination.path, generated.output_path);
                assert_eq!(content, generated.package);
                assert_eq!(mode, 0o600);
                assert_eq!(classification, FileClassification::Generated);
            }
            _ => panic!("unexpected controlled writer draft"),
        }
        assert!(!outcome.external_effect_allowed);
    }

    #[test]
    fn every_native_surface_routes_identity_only_generation_through_one_coordinator() {
        let detailed = request();
        let generation = detailed.generation.as_ref().expect("generation");
        let command = ClientCommand::Word {
            action: WordClientCommand::Generate {
                source_id: detailed.structured_source.source_id.clone(),
                source_sha256: detailed.structured_source.source_sha256.clone(),
                profile_id: detailed.profile.profile_id.clone(),
                artifact_id: generation.artifact_id.clone(),
                markdown_source_sha256: generation.document.source_sha256().to_owned(),
                output_path: generation
                    .output_path
                    .components()
                    .iter()
                    .map(|component| component.as_str().to_owned())
                    .collect(),
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
            let mut coordinator = WordArtifactCoordinator::new(4).expect("coordinator");
            let outcome = coordinator
                .coordinate_native(
                    &native_client(surface, command.clone()),
                    detailed.clone(),
                    NOW,
                )
                .unwrap_or_else(|error| panic!("{surface:?}: {error:?}"));
            let snapshot = (
                outcome.inspection,
                outcome.extraction,
                outcome.source_admission.manifest,
                outcome.generated,
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
    fn native_word_binding_rejects_command_mismatch_and_write_authority_smuggling() {
        let mut detailed = request();
        let client = native_client(
            ClientSurface::Json,
            ClientCommand::Word {
                action: WordClientCommand::Inspect {
                    source_id: detailed.structured_source.source_id.clone(),
                    source_sha256: detailed.structured_source.source_sha256.clone(),
                    profile_id: detailed.profile.profile_id.clone(),
                },
            },
        );
        let mut coordinator = WordArtifactCoordinator::new(4).expect("coordinator");
        assert_eq!(
            coordinator.coordinate_native(&client, detailed.clone(), NOW),
            Err(WordArtifactCoordinatorError::NativeRequestDenied)
        );
        detailed.generation = None;
        detailed.controlled_write = Some(WordControlledWriteRequest {
            operation_id: "smuggled-write".to_owned(),
            destination: generated_destination(),
            mode: 0o600,
        });
        assert_eq!(
            coordinator.coordinate_native(&client, detailed, NOW),
            Err(WordArtifactCoordinatorError::NativeRequestDenied)
        );
        assert_eq!(
            client.command.required_operation(),
            GrantOperation::WorkspaceRead
        );
    }
}
