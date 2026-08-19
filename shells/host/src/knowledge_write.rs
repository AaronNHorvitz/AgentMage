//! Composition of verified knowledge previews into kernel filesystem drafts.

use std::fmt::Write as _;

use agentmage_capability_knowledge::{
    KnowledgeIndexPublication, KnowledgeIndexPublicationState, KnowledgeNoteCreatePreview,
    MarkdownDocument, MarkdownUpdatePreview, ObsidianPostWriteIndexResult, ObsidianVaultFreshness,
    ObsidianVaultIndex, ObsidianVaultSnapshot, obsidian_snapshot_sha256,
    verify_knowledge_note_create_preview, verify_markdown_update_preview,
};
use agentmage_kernel_contracts::GrantTarget;
use agentmage_kernel_engine::filesystem_control::{
    ExistingSourceDraft, ExistingWorkDisposition, FileClassification, FilesystemOperationDraft,
    NewDestinationDraft, StructuredPatch, StructuredPatchHunk,
};
use agentmage_kernel_engine::operational_store::DurableAuthorityRuntime;
use agentmage_kernel_engine::write_recovery::{
    WriteAwareCheckpoint, WriteAwareCheckpointInput, WriteBoundaryField, WriteCheckpointPhase,
    WriteFieldSensitivity, WritePrivacyBoundary, build_write_checkpoint, sanitize_write_boundary,
    verify_write_checkpoint,
};
use sha2::{Digest, Sha256};

/// Exact held-object context needed to compose one knowledge filesystem draft.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum KnowledgeFilesystemContext {
    /// Already observed absent destination and continuously held parent.
    Create {
        /// Exact held destination parent.
        destination_parent: GrantTarget,
        /// Complete bounded sibling-name snapshot.
        observed_sibling_names: Vec<String>,
        /// Digest of the complete canonical namespace observed for the preview.
        observed_namespace_sha256: String,
        /// Revision of the disposable derived index observed for the preview.
        observed_index_revision: u64,
        /// Exact proposed POSIX-compatible mode.
        mode: u32,
    },
    /// Already observed exact source file.
    Update {
        /// Exact held regular-file target.
        source_target: GrantTarget,
        /// Exact bytes read through the held target.
        observed_source_bytes: Vec<u8>,
        /// Exact observed POSIX-compatible mode.
        mode: u32,
        /// Current repository/work-packet ownership classification.
        work_disposition: ExistingWorkDisposition,
    },
}

/// Content-free knowledge-to-filesystem composition failure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KnowledgeFilesystemError {
    /// The knowledge preview was altered or malformed.
    PreviewInvalid,
    /// The held path, bytes, mode, or operation class did not match the preview.
    ContextMismatch,
    /// The exact patch could not be represented under fixed bounds.
    PatchUnavailable,
}

impl KnowledgeFilesystemError {
    /// Returns a stable redacted failure code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::PreviewInvalid => "host.knowledge-write.preview_invalid",
            Self::ContextMismatch => "host.knowledge-write.context_mismatch",
            Self::PatchUnavailable => "host.knowledge-write.patch_unavailable",
        }
    }
}

impl std::fmt::Display for KnowledgeFilesystemError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.code())
    }
}

impl std::error::Error for KnowledgeFilesystemError {}

/// Stable identities and clock values for one derived-index checkpoint pair.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KnowledgeIndexCheckpointRequest {
    /// Identity of the checkpoint persisted before index publication.
    pub updating_checkpoint_id: String,
    /// Identity of the checkpoint persisted after exact index verification.
    pub verified_checkpoint_id: String,
    /// Kernel-clock time for the pre-publication checkpoint.
    pub updating_at_epoch_ms: u64,
    /// Kernel-clock time for the verified checkpoint.
    pub verified_at_epoch_ms: u64,
}

/// Durable result of publishing one disposable index after canonical verification.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KnowledgeIndexCheckpointResult {
    /// Content-free digest of the exact declared index publication.
    pub index_update_sha256: String,
    /// Durable checkpoint committed before the index transaction.
    pub updating_checkpoint: WriteAwareCheckpoint,
    /// Durable checkpoint committed after the index matches canonical bytes.
    pub verified_checkpoint: WriteAwareCheckpoint,
    /// Exact non-canonical index publication result.
    pub publication: ObsidianPostWriteIndexResult,
}

/// Content-free failure from the derived-index checkpoint coordinator.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KnowledgeIndexCheckpointError {
    /// The canonical checkpoint, request, or publication intent was invalid.
    InvalidInput,
    /// The SQLCipher checkpoint journal could not commit safely.
    CheckpointPersistenceFailed,
    /// The disposable index did not publish or verify against canonical bytes.
    IndexPublicationFailed,
}

impl KnowledgeIndexCheckpointError {
    /// Returns a stable redacted failure code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidInput => "host.knowledge-index-checkpoint.invalid_input",
            Self::CheckpointPersistenceFailed => {
                "host.knowledge-index-checkpoint.persistence_failed"
            }
            Self::IndexPublicationFailed => "host.knowledge-index-checkpoint.publication_failed",
        }
    }
}

impl std::fmt::Display for KnowledgeIndexCheckpointError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.code())
    }
}

impl std::error::Error for KnowledgeIndexCheckpointError {}

/// Publishes one canonical-bound disposable index with durable before/after checkpoints.
///
/// The canonical filesystem transaction, derived SQLite index, and SQLCipher journal remain
/// separate commits. A stop after the first checkpoint therefore reopens at `IndexUpdating` and
/// requires deterministic index verification or rebuild; it never reports the write complete.
pub fn publish_knowledge_index_with_checkpoints(
    authority: &mut DurableAuthorityRuntime,
    previous: &WriteAwareCheckpoint,
    index: &mut ObsidianVaultIndex,
    expected_index_revision: u64,
    publication: &KnowledgeIndexPublication,
    snapshot: &ObsidianVaultSnapshot,
    request: KnowledgeIndexCheckpointRequest,
) -> Result<KnowledgeIndexCheckpointResult, KnowledgeIndexCheckpointError> {
    if verify_write_checkpoint(previous).is_err()
        || previous.phase != WriteCheckpointPhase::CanonicalVerified
        || previous.consumed_grant_id.is_none()
        || previous.index_update_sha256.is_some()
        || previous.index_verified
        || publication.state != KnowledgeIndexPublicationState::ReadyAfterCommit
        || publication.observed_source_sha256.as_deref()
            != Some(publication.mutation.proposed_source_sha256())
        || request.verified_at_epoch_ms < request.updating_at_epoch_ms
    {
        return Err(KnowledgeIndexCheckpointError::InvalidInput);
    }
    let index_update_sha256 =
        knowledge_index_publication_digest(expected_index_revision, publication, snapshot)?;
    let scan = sanitize_write_boundary(
        WritePrivacyBoundary::Checkpoint,
        &[
            WriteBoundaryField {
                name: "transaction_id",
                value: previous.transaction_id.as_bytes(),
                sensitivity: WriteFieldSensitivity::PublicMetadata,
            },
            WriteBoundaryField {
                name: "action_id",
                value: previous.action_id.as_bytes(),
                sensitivity: WriteFieldSensitivity::PublicMetadata,
            },
            WriteBoundaryField {
                name: "updating_checkpoint_id",
                value: request.updating_checkpoint_id.as_bytes(),
                sensitivity: WriteFieldSensitivity::PublicMetadata,
            },
            WriteBoundaryField {
                name: "verified_checkpoint_id",
                value: request.verified_checkpoint_id.as_bytes(),
                sensitivity: WriteFieldSensitivity::PublicMetadata,
            },
            WriteBoundaryField {
                name: "index_update_sha256",
                value: index_update_sha256.as_bytes(),
                sensitivity: WriteFieldSensitivity::PublicMetadata,
            },
        ],
    )
    .map_err(|_| KnowledgeIndexCheckpointError::InvalidInput)?;
    let updating_checkpoint = build_index_checkpoint(
        previous,
        request.updating_checkpoint_id,
        WriteCheckpointPhase::IndexUpdating,
        &index_update_sha256,
        false,
        scan.receipt.receipt_sha256.clone(),
        request.updating_at_epoch_ms,
    )?;
    authority
        .checkpoint_write_transaction(std::slice::from_ref(&updating_checkpoint))
        .map_err(|_| KnowledgeIndexCheckpointError::CheckpointPersistenceFailed)?;

    let published = index
        .publish_after_canonical_write(expected_index_revision, publication, snapshot)
        .map_err(|_| KnowledgeIndexCheckpointError::IndexPublicationFailed)?;
    let update = published
        .update
        .as_ref()
        .ok_or(KnowledgeIndexCheckpointError::IndexPublicationFailed)?;
    let expected_revision = expected_index_revision
        .checked_add(1)
        .ok_or(KnowledgeIndexCheckpointError::IndexPublicationFailed)?;
    if published.state != KnowledgeIndexPublicationState::ReadyAfterCommit
        || published.report != update.report
        || published.report.revision != expected_revision
        || update.receipt.index_revision != expected_revision
        || update.receipt.snapshot_sha256 != published.report.snapshot_sha256
        || update.receipt.index_sha256 != published.report.index_sha256
        || update.receipt.source_files_mutated
        || update.receipt.external_process_started
        || update.receipt.network_accessed
        || index.freshness(snapshot) != Ok(ObsidianVaultFreshness::Current)
    {
        return Err(KnowledgeIndexCheckpointError::IndexPublicationFailed);
    }
    let verified_checkpoint = build_index_checkpoint(
        &updating_checkpoint,
        request.verified_checkpoint_id,
        WriteCheckpointPhase::IndexVerified,
        &index_update_sha256,
        true,
        scan.receipt.receipt_sha256,
        request.verified_at_epoch_ms,
    )?;
    authority
        .checkpoint_write_transaction(std::slice::from_ref(&verified_checkpoint))
        .map_err(|_| KnowledgeIndexCheckpointError::CheckpointPersistenceFailed)?;
    Ok(KnowledgeIndexCheckpointResult {
        index_update_sha256,
        updating_checkpoint,
        verified_checkpoint,
        publication: published,
    })
}

fn knowledge_index_publication_digest(
    expected_index_revision: u64,
    publication: &KnowledgeIndexPublication,
    snapshot: &ObsidianVaultSnapshot,
) -> Result<String, KnowledgeIndexCheckpointError> {
    let intent = serde_json::json!({
        "schema_version": 1,
        "expected_index_revision": expected_index_revision,
        "path_sha256": sha256(
            &serde_json::to_vec(publication.mutation.path())
                .map_err(|_| KnowledgeIndexCheckpointError::InvalidInput)?,
        ),
        "stable_id": publication.mutation.stable_id().as_str(),
        "expected_source_sha256": publication.mutation.expected_source_sha256(),
        "proposed_source_sha256": publication.mutation.proposed_source_sha256(),
        "preview_sha256": publication.mutation.preview_sha256(),
        "observed_source_sha256": publication.observed_source_sha256,
        "canonical_snapshot_sha256": obsidian_snapshot_sha256(snapshot)
            .map_err(|_| KnowledgeIndexCheckpointError::InvalidInput)?,
    });
    serde_json::to_vec(&intent)
        .map(|bytes| sha256(&bytes))
        .map_err(|_| KnowledgeIndexCheckpointError::InvalidInput)
}

fn build_index_checkpoint(
    previous: &WriteAwareCheckpoint,
    checkpoint_id: String,
    phase: WriteCheckpointPhase,
    index_update_sha256: &str,
    index_verified: bool,
    secret_scan_receipt_sha256: String,
    occurred_at_epoch_ms: u64,
) -> Result<WriteAwareCheckpoint, KnowledgeIndexCheckpointError> {
    build_write_checkpoint(
        WriteAwareCheckpointInput {
            checkpoint_id,
            transaction_id: previous.transaction_id.clone(),
            action_id: previous.action_id.clone(),
            phase,
            consumed_grant_id: previous.consumed_grant_id.clone(),
            file_receipt_head_sha256: previous.file_receipt_head_sha256.clone(),
            file_receipt_count: previous.file_receipt_count,
            evidence_set_sha256: previous.evidence_set_sha256.clone(),
            index_update_sha256: Some(index_update_sha256.to_owned()),
            next_session_checkpoint_sha256: previous.next_session_checkpoint_sha256.clone(),
            secret_scan_receipt_sha256,
            staging_inventory_sha256: previous.staging_inventory_sha256.clone(),
            staging_item_count: previous.staging_item_count,
            retention_expires_at_epoch_ms: previous.retention_expires_at_epoch_ms,
            canonical_postimages_verified: true,
            receipt_chain_verified: false,
            index_verified,
            rollback_verified: false,
            cleanup_state: previous.cleanup_state,
            failure_code: None,
            occurred_at_epoch_ms,
        },
        Some(previous),
    )
    .map_err(|_| KnowledgeIndexCheckpointError::InvalidInput)
}

/// Translates one verified create preview into the kernel's closed create draft.
pub fn compose_knowledge_create(
    operation_id: String,
    preview: &KnowledgeNoteCreatePreview,
    context: KnowledgeFilesystemContext,
) -> Result<FilesystemOperationDraft, KnowledgeFilesystemError> {
    verify_knowledge_note_create_preview(preview)
        .map_err(|_| KnowledgeFilesystemError::PreviewInvalid)?;
    let KnowledgeFilesystemContext::Create {
        destination_parent,
        observed_sibling_names,
        observed_namespace_sha256,
        observed_index_revision,
        mode,
    } = context
    else {
        return Err(KnowledgeFilesystemError::ContextMismatch);
    };
    if mode > 0o777
        || destination_parent.workspace_path().is_none()
        || destination_parent.object_kind()
            != Some(agentmage_kernel_contracts::WorkspaceObjectKind::Directory)
        || destination_parent.workspace_id() != preview.path.workspace_id()
        || observed_namespace_sha256 != preview.expected_namespace_sha256
        || observed_index_revision != preview.expected_index_revision
    {
        return Err(KnowledgeFilesystemError::ContextMismatch);
    }
    Ok(FilesystemOperationDraft::Create {
        operation_id,
        destination: NewDestinationDraft {
            parent: destination_parent,
            path: preview.path.clone(),
            observed_sibling_names,
        },
        content: preview.proposed_markdown().to_vec(),
        mode,
        classification: FileClassification::Documentation,
    })
}

/// Translates one verified update preview into the kernel's closed exact-patch draft.
pub fn compose_knowledge_update(
    operation_id: String,
    preview: &MarkdownUpdatePreview,
    context: KnowledgeFilesystemContext,
) -> Result<FilesystemOperationDraft, KnowledgeFilesystemError> {
    verify_markdown_update_preview(preview)
        .map_err(|_| KnowledgeFilesystemError::PreviewInvalid)?;
    let KnowledgeFilesystemContext::Update {
        source_target,
        observed_source_bytes,
        mode,
        work_disposition,
    } = context
    else {
        return Err(KnowledgeFilesystemError::ContextMismatch);
    };
    if mode > 0o777
        || !matches!(
            work_disposition,
            ExistingWorkDisposition::Clean | ExistingWorkDisposition::OwnedByCurrentTask
        )
        || source_target.workspace_path() != Some(&preview.path)
        || source_target.object_kind()
            != Some(agentmage_kernel_contracts::WorkspaceObjectKind::RegularFile)
        || sha256(&observed_source_bytes) != preview.expected_source_sha256
    {
        return Err(KnowledgeFilesystemError::ContextMismatch);
    }
    let document = MarkdownDocument::parse(preview.path.clone(), observed_source_bytes.clone())
        .map_err(|_| KnowledgeFilesystemError::ContextMismatch)?;
    if document.stable_id() != Some(&preview.stable_id) {
        return Err(KnowledgeFilesystemError::ContextMismatch);
    }
    let patch = StructuredPatch {
        schema_version: 1,
        hunks: vec![StructuredPatchHunk {
            old_start_line: 1,
            old_lines: exact_lines(&observed_source_bytes)?,
            new_lines: exact_lines(preview.proposed_markdown())?,
        }],
    };
    let patch_json =
        serde_json::to_vec(&patch).map_err(|_| KnowledgeFilesystemError::PatchUnavailable)?;
    Ok(FilesystemOperationDraft::ExactPatch {
        operation_id,
        source: ExistingSourceDraft {
            target: source_target,
            observed_bytes: observed_source_bytes,
            work_disposition,
            mode,
        },
        patch_json,
        expected_postimage_sha256: preview.proposed_source_sha256.clone(),
    })
}

fn exact_lines(bytes: &[u8]) -> Result<Vec<String>, KnowledgeFilesystemError> {
    let text =
        std::str::from_utf8(bytes).map_err(|_| KnowledgeFilesystemError::PatchUnavailable)?;
    let lines = text
        .split_inclusive('\n')
        .map(str::to_owned)
        .collect::<Vec<_>>();
    if lines.is_empty() {
        return Err(KnowledgeFilesystemError::PatchUnavailable);
    }
    Ok(lines)
}

fn sha256(value: &[u8]) -> String {
    let mut output = String::with_capacity(64);
    for byte in Sha256::digest(value) {
        write!(&mut output, "{byte:02x}").expect("writing to String cannot fail");
    }
    output
}

#[cfg(test)]
mod tests {
    use agentmage_capability_knowledge::{
        KnowledgeNamespaceSnapshot, KnowledgeNoteCreateRequest, KnowledgeRecordId,
        KnowledgeSectionDraft, KnowledgeWriteWorkflow, MarkdownEdit, MarkdownLineEnding,
        MarkdownUpdateRequest, preview_knowledge_note_create, preview_markdown_update,
    };
    use agentmage_kernel_contracts::{WorkspaceId, WorkspacePath};
    use agentmage_kernel_engine::filesystem_control::{
        FilesystemOperationDraft, apply_structured_patch, parse_structured_patch_json,
    };
    use serde_json::json;

    use super::*;

    fn path(name: &str) -> WorkspacePath {
        WorkspacePath::new(
            WorkspaceId::from_raw("workspace-knowledge-host"),
            ["notes", name],
        )
        .expect("path")
    }

    fn directory_target() -> GrantTarget {
        serde_json::from_value(json!({
            "target_kind": "held_object",
            "path": {"workspace_id": "workspace-knowledge-host", "components": ["notes"]},
            "authorization_id": "authorization-knowledge-host",
            "adapter_instance_id": "adapter-knowledge-host",
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

    fn file_target(path: &WorkspacePath, bytes: &[u8]) -> GrantTarget {
        let content: [u8; 32] = Sha256::digest(bytes).into();
        serde_json::from_value(json!({
            "target_kind": "held_object",
            "path": path,
            "authorization_id": "authorization-knowledge-host",
            "adapter_instance_id": "adapter-knowledge-host",
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

    fn create_preview() -> KnowledgeNoteCreatePreview {
        preview_knowledge_note_create(
            KnowledgeNoteCreateRequest {
                path: path("decision.md"),
                stable_id: KnowledgeRecordId::parse("knowledge-decision-001").expect("identity"),
                workflow: KnowledgeWriteWorkflow::Decision,
                title: "Decision".to_owned(),
                properties: Vec::new(),
                sections: vec![
                    KnowledgeSectionDraft {
                        heading: "Decision".to_owned(),
                        body: "Original decision.".to_owned(),
                    },
                    KnowledgeSectionDraft {
                        heading: "Evidence".to_owned(),
                        body: "Synthetic evidence.".to_owned(),
                    },
                ],
                memory_promotion: None,
                line_ending: MarkdownLineEnding::CrLf,
            },
            &KnowledgeNamespaceSnapshot {
                paths: Vec::new(),
                stable_ids: Vec::new(),
                link_targets: Vec::new(),
                canonical_snapshot_sha256: "a".repeat(64),
                derived_index_revision: 1,
            },
        )
        .expect("create preview")
    }

    #[test]
    fn verified_create_becomes_only_a_closed_kernel_create_draft() {
        let preview = create_preview();
        let draft = compose_knowledge_create(
            "operation-knowledge-create".to_owned(),
            &preview,
            KnowledgeFilesystemContext::Create {
                destination_parent: directory_target(),
                observed_sibling_names: vec!["existing.md".to_owned()],
                observed_namespace_sha256: preview.expected_namespace_sha256.clone(),
                observed_index_revision: preview.expected_index_revision,
                mode: 0o600,
            },
        )
        .expect("draft");
        let FilesystemOperationDraft::Create {
            destination,
            content,
            mode,
            classification,
            ..
        } = draft
        else {
            panic!("expected create")
        };
        assert_eq!(destination.path, preview.path);
        assert_eq!(content, preview.proposed_markdown());
        assert_eq!(mode, 0o600);
        assert_eq!(classification, FileClassification::Documentation);
    }

    #[test]
    fn verified_update_becomes_an_exact_whole_document_structured_patch() {
        let create = create_preview();
        let document =
            MarkdownDocument::parse(create.path.clone(), create.proposed_markdown().to_vec())
                .expect("document");
        let update = preview_markdown_update(
            &document,
            MarkdownUpdateRequest {
                expected_source_sha256: document.source_sha256().to_owned(),
                expected_stable_id: document.stable_id().expect("identity").clone(),
                edit: MarkdownEdit::ReplaceHeadingBody {
                    heading: "Decision".to_owned(),
                    level: 2,
                    replacement: "Revised decision.".to_owned(),
                },
            },
        )
        .expect("update preview");
        let draft = compose_knowledge_update(
            "operation-knowledge-update".to_owned(),
            &update,
            KnowledgeFilesystemContext::Update {
                source_target: file_target(&update.path, document.source_bytes()),
                observed_source_bytes: document.source_bytes().to_vec(),
                mode: 0o640,
                work_disposition: ExistingWorkDisposition::OwnedByCurrentTask,
            },
        )
        .expect("draft");
        let FilesystemOperationDraft::ExactPatch {
            source,
            patch_json,
            expected_postimage_sha256,
            ..
        } = draft
        else {
            panic!("expected exact patch")
        };
        let patch = parse_structured_patch_json(&patch_json).expect("patch");
        assert_eq!(
            apply_structured_patch(&source.observed_bytes, &patch),
            Ok(update.proposed_markdown().to_vec())
        );
        assert_eq!(expected_postimage_sha256, update.proposed_source_sha256);
        assert_eq!(source.mode, 0o640);
    }

    #[test]
    fn preview_mutation_stale_bytes_wrong_context_and_unrelated_work_fail_closed() {
        let preview = create_preview();
        let mut changed = preview.clone();
        changed.proposed_source_sha256 = "b".repeat(64);
        assert_eq!(
            compose_knowledge_create(
                "operation-mutated".to_owned(),
                &changed,
                KnowledgeFilesystemContext::Create {
                    destination_parent: directory_target(),
                    observed_sibling_names: Vec::new(),
                    observed_namespace_sha256: changed.expected_namespace_sha256.clone(),
                    observed_index_revision: changed.expected_index_revision,
                    mode: 0o600,
                },
            ),
            Err(KnowledgeFilesystemError::PreviewInvalid)
        );

        assert_eq!(
            compose_knowledge_create(
                "operation-stale-namespace".to_owned(),
                &preview,
                KnowledgeFilesystemContext::Create {
                    destination_parent: directory_target(),
                    observed_sibling_names: Vec::new(),
                    observed_namespace_sha256: "b".repeat(64),
                    observed_index_revision: preview.expected_index_revision + 1,
                    mode: 0o600,
                },
            ),
            Err(KnowledgeFilesystemError::ContextMismatch)
        );

        let document =
            MarkdownDocument::parse(preview.path.clone(), preview.proposed_markdown().to_vec())
                .expect("document");
        let update = preview_markdown_update(
            &document,
            MarkdownUpdateRequest {
                expected_source_sha256: document.source_sha256().to_owned(),
                expected_stable_id: document.stable_id().expect("identity").clone(),
                edit: MarkdownEdit::ReplaceHeadingBody {
                    heading: "Evidence".to_owned(),
                    level: 2,
                    replacement: "Changed evidence.".to_owned(),
                },
            },
        )
        .expect("update");
        for disposition in [
            ExistingWorkDisposition::UnrelatedModified,
            ExistingWorkDisposition::Unknown,
        ] {
            assert_eq!(
                compose_knowledge_update(
                    "operation-denied".to_owned(),
                    &update,
                    KnowledgeFilesystemContext::Update {
                        source_target: file_target(&update.path, document.source_bytes()),
                        observed_source_bytes: document.source_bytes().to_vec(),
                        mode: 0o600,
                        work_disposition: disposition,
                    },
                ),
                Err(KnowledgeFilesystemError::ContextMismatch)
            );
        }
        assert_eq!(
            compose_knowledge_update(
                "operation-stale".to_owned(),
                &update,
                KnowledgeFilesystemContext::Update {
                    source_target: file_target(&update.path, document.source_bytes()),
                    observed_source_bytes: b"stale\n".to_vec(),
                    mode: 0o600,
                    work_disposition: ExistingWorkDisposition::Clean,
                },
            ),
            Err(KnowledgeFilesystemError::ContextMismatch)
        );
    }
}

#[cfg(all(test, target_os = "linux"))]
mod linux_tests {
    use std::collections::BTreeSet;
    use std::env;
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use std::path::{Path, PathBuf};
    use std::process::Command;
    use std::sync::atomic::{AtomicU64, Ordering};

    use agentmage_capability_knowledge::{
        CanonicalKnowledgeMutation, CanonicalMarkdownWriteOutcome, KnowledgeIndexPublicationState,
        MarkdownDocument, MarkdownEdit, MarkdownUpdateRequest, ObsidianEntryKind,
        ObsidianNoteInput, ObsidianVaultFreshness, ObsidianVaultIndex, ObsidianVaultSelection,
        ObsidianVaultSnapshot, decide_index_publication, preview_markdown_update,
    };
    use agentmage_kernel_contracts::{
        ActionId, ActionKind, ActorId, AdapterInstanceId, ApprovalId, DataSensitivity, GrantId,
        GrantNonce, GrantOperation, GrantTarget, OperationBinding, PathResolutionIntent,
        PlatformPathAdapter, SessionId, StorageFilesystemClass, StrictLocalStorageObservation,
        TaskId, ToolId, WorkspaceAuthorizationId, WorkspaceId, WorkspacePath, WorkspaceScopePath,
    };
    use agentmage_kernel_engine::filesystem_control::{
        FilesystemApprovalDecision, FilesystemGrantRequest, FilesystemOperationDraft,
        FilesystemPlan, FilesystemPlanRequest, FilesystemTransactionError,
        FilesystemTransactionOutcome, FilesystemTransactionRequest, build_filesystem_plan,
        execute_filesystem_transaction, issue_filesystem_grant, render_filesystem_preview,
    };
    use agentmage_kernel_engine::grants::{GrantIssuer, SessionReadGrantRequest};
    use agentmage_kernel_engine::operational_store::{
        DurableAuthorityRuntime, OperationalStoreKeyError, OperationalStoreKeyProvider,
    };
    use agentmage_kernel_engine::policy::{
        PolicyDocument, PolicyEngine, ScopeRules, ToolPolicyBinding,
    };
    use agentmage_kernel_engine::write_approval::WriteReviewNarrative;
    use agentmage_kernel_engine::write_recovery::{
        WriteAwareCheckpoint, WriteAwareCheckpointInput, WriteCheckpointPhase, WriteCleanupState,
        build_write_checkpoint,
    };
    use agentmage_platform_linux::{
        LinuxControlledFilesystemDriver, LinuxFilesystemDriverLimits, LinuxPathAdapter,
        select_test_linux_workspace,
    };

    use super::{
        KnowledgeFilesystemContext, KnowledgeIndexCheckpointError, KnowledgeIndexCheckpointRequest,
        compose_knowledge_update, publish_knowledge_index_with_checkpoints, sha256,
    };

    static TEMP_ID: AtomicU64 = AtomicU64::new(1);
    const KNOWLEDGE_CRASH_CHILD_EXIT: i32 = 88;

    struct TestDirectory(PathBuf);

    struct TestKey([u8; 32]);

    impl OperationalStoreKeyProvider for TestKey {
        fn with_key<T>(
            &mut self,
            operation: impl FnOnce(&[u8]) -> T,
        ) -> Result<T, OperationalStoreKeyError> {
            Ok(operation(&self.0))
        }
    }

    impl TestDirectory {
        fn new() -> Self {
            let id = TEMP_ID.fetch_add(1, Ordering::SeqCst);
            let path = std::env::temp_dir().join(format!(
                "agentmage-host-knowledge-native-{}-{id}",
                std::process::id()
            ));
            fs::create_dir_all(path.join("notes")).expect("native knowledge fixture");
            Self(path)
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.0).expect("native knowledge fixture removed");
        }
    }

    struct NativeKnowledgeFixture {
        _root: TestDirectory,
        workspace: agentmage_platform_linux::LinuxAuthorizedWorkspace,
        issuer: GrantIssuer,
        policy: PolicyEngine,
        plan: FilesystemPlan,
        approval: agentmage_kernel_engine::filesystem_control::FilesystemApprovalReceipt,
        source_path: PathBuf,
        source_bytes: Vec<u8>,
        postimage_bytes: Vec<u8>,
        workspace_path: WorkspacePath,
        mutation: CanonicalKnowledgeMutation,
    }

    fn rules<T: Ord>(values: impl IntoIterator<Item = T>) -> ScopeRules<T> {
        ScopeRules {
            allowed: values.into_iter().collect(),
            denied: BTreeSet::new(),
        }
    }

    fn native_fixture() -> NativeKnowledgeFixture {
        native_fixture_in(TestDirectory::new())
    }

    fn native_fixture_at(path: PathBuf) -> NativeKnowledgeFixture {
        native_fixture_in(TestDirectory(path))
    }

    fn native_fixture_in(root: TestDirectory) -> NativeKnowledgeFixture {
        let source_path = root.path().join("notes/decision.md");
        let source_bytes = concat!(
            "---\n",
            "agentmage_id: knowledge-decision-native-001\n",
            "type: decision\n",
            "---\n",
            "# Native Decision\n",
            "## Decision\n",
            "Original decision.\n",
            "## Evidence\n",
            "Synthetic evidence.\n",
        )
        .as_bytes()
        .to_vec();
        fs::write(&source_path, &source_bytes).expect("native Markdown source");
        fs::set_permissions(&source_path, fs::Permissions::from_mode(0o640))
            .expect("native Markdown mode");

        let workspace_id = WorkspaceId::from_raw("workspace-native-knowledge");
        let adapter_id = AdapterInstanceId::from_raw("adapter-native-knowledge");
        let workspace = select_test_linux_workspace(
            root.path(),
            workspace_id.clone(),
            WorkspaceAuthorizationId::from_raw("authorization-native-knowledge"),
            adapter_id.clone(),
        )
        .expect("native knowledge workspace");
        let workspace_path = WorkspacePath::new(workspace_id.clone(), ["notes", "decision.md"])
            .expect("native knowledge path");
        let document = MarkdownDocument::parse(workspace_path.clone(), source_bytes.clone())
            .expect("native Markdown document");
        let preview = preview_markdown_update(
            &document,
            MarkdownUpdateRequest {
                expected_source_sha256: document.source_sha256().to_owned(),
                expected_stable_id: document.stable_id().expect("stable identity").clone(),
                edit: MarkdownEdit::ReplaceHeadingBody {
                    heading: "Decision".to_owned(),
                    level: 2,
                    replacement: "Revised through the native boundary.".to_owned(),
                },
            },
        )
        .expect("native update preview");
        let mutation = CanonicalKnowledgeMutation::from_update(&preview);
        let adapter = LinuxPathAdapter::new(adapter_id, 1024 * 1024);
        let held = adapter
            .resolve(&workspace, &workspace_path, PathResolutionIntent::ReadFile)
            .expect("held native Markdown source");
        let source_target = GrantTarget::held_object(&held).expect("native source target");
        let draft = compose_knowledge_update(
            "operation-native-knowledge-update".to_owned(),
            &preview,
            KnowledgeFilesystemContext::Update {
                source_target: source_target.clone(),
                observed_source_bytes: source_bytes.clone(),
                mode: 0o640,
                work_disposition:
                    agentmage_kernel_engine::filesystem_control::ExistingWorkDisposition::Clean,
            },
        )
        .expect("native filesystem draft");
        assert!(matches!(draft, FilesystemOperationDraft::ExactPatch { .. }));

        let operation = OperationBinding::new(GrantOperation::WorkspaceWrite);
        let action_id = ActionId::from_raw("action-native-knowledge");
        let tool_id = ToolId::from_raw("workspace.native-knowledge");
        let policy = PolicyEngine::new(PolicyDocument {
            schema_version: 1,
            revision: 1,
            actors: rules([ActorId::from_raw("actor-local")]),
            tasks: rules([TaskId::from_raw("task-native-knowledge")]),
            actions: rules([action_id.clone()]),
            tools: rules([ToolPolicyBinding {
                tool_id: tool_id.clone(),
                tool_version: "1.0.0".to_owned(),
            }]),
            operations: rules([operation]),
            targets: rules([source_target]),
            denied_argument_sha256s: BTreeSet::new(),
            denied_preimage_sha256s: BTreeSet::new(),
            network_scopes: ScopeRules::deny_all(),
            credential_scopes: ScopeRules::deny_all(),
            publication_scopes: ScopeRules::deny_all(),
        })
        .expect("native knowledge policy");
        let mut issuer = GrantIssuer::new();
        let parent = issuer
            .issue_session_read(SessionReadGrantRequest {
                grant_id: GrantId::from_raw("grant-parent-native-knowledge"),
                actor_id: ActorId::from_raw("actor-local"),
                session_id: SessionId::from_raw("session-native-knowledge"),
                task_id: TaskId::from_raw("task-native-knowledge"),
                targets: vec![
                    GrantTarget::workspace_scope(
                        &workspace,
                        WorkspaceScopePath::new(workspace_id, Vec::<String>::new())
                            .expect("native root scope"),
                    )
                    .expect("native root target"),
                ],
                excluded_targets: Vec::new(),
                sensitivity: DataSensitivity::Restricted,
                issued_at_epoch_ms: 1_000,
                expires_at_epoch_ms: 100_000,
                nonce: GrantNonce::from_raw("nonce-parent-native-knowledge"),
                maximum_derived_operations: 2,
                preview_sha256: "a".repeat(64),
                policy_sha256: policy.policy_sha256().to_owned(),
            })
            .expect("native parent grant");
        let plan = build_filesystem_plan(
            &parent,
            FilesystemPlanRequest {
                plan_id: "plan-native-knowledge".to_owned(),
                observed_at_epoch_ms: 2_000,
                operations: vec![draft],
                review: WriteReviewNarrative {
                    rationale: "Apply one exact Markdown preview".to_owned(),
                    behavior_change: "Only the reviewed Decision body changes".to_owned(),
                    verification_plan: vec!["native-knowledge-test".to_owned()],
                    risks: vec!["The source may change before consumption".to_owned()],
                    rollback: "Restore the exact Markdown preimage".to_owned(),
                    unverified_assumptions: vec!["No external reader was inspected".to_owned()],
                },
                permitted_verification: vec!["native-knowledge-test".to_owned()],
            },
        )
        .expect("native filesystem plan");
        let rendered = render_filesystem_preview(&plan).expect("native filesystem preview");
        let approval = issue_filesystem_grant(
            &mut issuer,
            &plan,
            &rendered,
            &FilesystemApprovalDecision {
                approval_id: ApprovalId::from_raw("approval-native-knowledge"),
                approved_plan_sha256: rendered.plan_sha256.clone(),
                approved_preview_sha256: rendered.preview_sha256.clone(),
                approved_at_epoch_ms: 3_000,
                expires_at_epoch_ms: 30_000,
                permitted_verification: rendered.permitted_verification.clone(),
                user_confirmed: true,
                high_risk_delete_confirmed: false,
            },
            FilesystemGrantRequest {
                parent_grant_id: parent.grant_id,
                grant_id: GrantId::from_raw("grant-native-knowledge"),
                action_id,
                action_kind: ActionKind::DeterministicTool,
                tool_id,
                tool_version: "1.0.0".to_owned(),
                nonce: GrantNonce::from_raw("nonce-native-knowledge"),
                policy_sha256: policy.policy_sha256().to_owned(),
            },
        )
        .expect("native knowledge approval");

        NativeKnowledgeFixture {
            _root: root,
            workspace,
            issuer,
            policy,
            plan,
            approval,
            source_path,
            source_bytes,
            postimage_bytes: preview.proposed_markdown().to_vec(),
            workspace_path,
            mutation,
        }
    }

    fn vault_selection(workspace_id: &WorkspaceId) -> ObsidianVaultSelection {
        ObsidianVaultSelection::admit(
            WorkspaceScopePath::new(workspace_id.clone(), ["notes"]).expect("native vault scope"),
            StrictLocalStorageObservation {
                filesystem: StorageFilesystemClass::Local,
                synchronization_marker: None,
                root_identity_sha256: [7; 32],
                symlink_free: true,
            },
            Vec::new(),
        )
        .expect("native vault selection")
    }

    fn vault_snapshot(
        selection: &ObsidianVaultSelection,
        path: WorkspacePath,
        bytes: &[u8],
    ) -> ObsidianVaultSnapshot {
        ObsidianVaultSnapshot::from_snapshots(
            selection,
            vec![ObsidianNoteInput {
                path,
                entry_kind: ObsidianEntryKind::RegularFile,
                hidden: false,
                cloud_synchronized: false,
                content_sha256: sha256(bytes),
                content: bytes.to_vec(),
            }],
        )
        .expect("native vault snapshot")
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum KnowledgeCrashBoundary {
        BeforeSourceExecution,
        AfterSourceExecution,
        BeforeIndexPublication,
        AfterIndexPublication,
    }

    impl KnowledgeCrashBoundary {
        const ALL: [Self; 4] = [
            Self::BeforeSourceExecution,
            Self::AfterSourceExecution,
            Self::BeforeIndexPublication,
            Self::AfterIndexPublication,
        ];

        const fn code(self) -> &'static str {
            match self {
                Self::BeforeSourceExecution => "before-source-execution",
                Self::AfterSourceExecution => "after-source-execution",
                Self::BeforeIndexPublication => "before-index-publication",
                Self::AfterIndexPublication => "after-index-publication",
            }
        }

        fn from_code(code: &str) -> Self {
            Self::ALL
                .into_iter()
                .find(|boundary| boundary.code() == code)
                .unwrap_or_else(|| panic!("undeclared knowledge crash boundary: {code}"))
        }

        const fn source_committed(self) -> bool {
            !matches!(self, Self::BeforeSourceExecution)
        }
    }

    fn run_knowledge_crash_child() {
        let root = PathBuf::from(
            env::var_os("AGENTMAGE_KNOWLEDGE_CRASH_ROOT").expect("knowledge crash root"),
        );
        let boundary = KnowledgeCrashBoundary::from_code(
            &env::var("AGENTMAGE_KNOWLEDGE_CRASH_BOUNDARY").expect("knowledge crash boundary"),
        );
        let mut fixture = native_fixture_at(root);
        let selection = vault_selection(fixture.workspace_path.workspace_id());
        let initial_snapshot = vault_snapshot(
            &selection,
            fixture.workspace_path.clone(),
            &fixture.source_bytes,
        );
        let mut index = ObsidianVaultIndex::in_memory().expect("crash child index");
        let initial = index
            .rebuild(&initial_snapshot)
            .expect("crash child rebuild");
        if boundary == KnowledgeCrashBoundary::BeforeSourceExecution {
            std::process::exit(KNOWLEDGE_CRASH_CHILD_EXIT);
        }
        assert_eq!(
            execute_native(&mut fixture),
            Ok(FilesystemTransactionOutcome::Committed)
        );
        if boundary == KnowledgeCrashBoundary::AfterSourceExecution {
            std::process::exit(KNOWLEDGE_CRASH_CHILD_EXIT);
        }
        let post_snapshot = vault_snapshot(
            &selection,
            fixture.workspace_path.clone(),
            &fixture.postimage_bytes,
        );
        let publication = decide_index_publication(
            fixture.mutation,
            CanonicalMarkdownWriteOutcome::Committed,
            Some(sha256(&fixture.postimage_bytes)),
        );
        if boundary == KnowledgeCrashBoundary::BeforeIndexPublication {
            std::process::exit(KNOWLEDGE_CRASH_CHILD_EXIT);
        }
        index
            .publish_after_canonical_write(initial.report.revision, &publication, &post_snapshot)
            .expect("crash child publication");
        std::process::exit(KNOWLEDGE_CRASH_CHILD_EXIT);
    }

    fn launch_knowledge_crash_child(root: &Path, boundary: KnowledgeCrashBoundary) {
        let output = Command::new(env::current_exe().expect("current test executable"))
            .args([
                "--exact",
                "knowledge_write::linux_tests::native_knowledge_crash_boundary_child",
                "--nocapture",
            ])
            .env("AGENTMAGE_KNOWLEDGE_CRASH_CHILD", "1")
            .env("AGENTMAGE_KNOWLEDGE_CRASH_ROOT", root)
            .env("AGENTMAGE_KNOWLEDGE_CRASH_BOUNDARY", boundary.code())
            .output()
            .expect("knowledge crash child launches");
        assert_eq!(
            output.status.code(),
            Some(KNOWLEDGE_CRASH_CHILD_EXIT),
            "{boundary:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    fn execute_native(
        fixture: &mut NativeKnowledgeFixture,
    ) -> Result<FilesystemTransactionOutcome, FilesystemTransactionError> {
        let mut driver = LinuxControlledFilesystemDriver::new(
            &fixture.workspace,
            LinuxFilesystemDriverLimits {
                maximum_file_bytes: 1024 * 1024,
                maximum_transaction_bytes: 4 * 1024 * 1024,
                ..LinuxFilesystemDriverLimits::default()
            },
        );
        execute_filesystem_transaction(
            &mut fixture.issuer,
            &fixture.policy,
            &fixture.plan,
            &fixture.approval,
            FilesystemTransactionRequest {
                transaction_id: "transaction-native-knowledge".to_owned(),
                now_epoch_ms: 4_000,
                cancelled_before_consume: false,
            },
            &mut driver,
        )
        .map(|result| result.outcome)
    }

    fn checkpoint_input(
        checkpoint_id: &str,
        phase: WriteCheckpointPhase,
        previous: Option<&WriteAwareCheckpoint>,
    ) -> WriteAwareCheckpointInput {
        let consumed = phase != WriteCheckpointPhase::BeforeTransaction;
        let canonical_verified = phase == WriteCheckpointPhase::CanonicalVerified;
        WriteAwareCheckpointInput {
            checkpoint_id: checkpoint_id.to_owned(),
            transaction_id: "transaction-native-knowledge-checkpoint".to_owned(),
            action_id: "action-native-knowledge-checkpoint".to_owned(),
            phase,
            consumed_grant_id: consumed.then(|| "grant-native-knowledge-checkpoint".to_owned()),
            file_receipt_head_sha256: canonical_verified.then(|| sha256(b"file-receipt")),
            file_receipt_count: u32::from(canonical_verified),
            evidence_set_sha256: sha256(b"[]"),
            index_update_sha256: None,
            next_session_checkpoint_sha256: "0".repeat(64),
            secret_scan_receipt_sha256: sha256(b"checkpoint-scan"),
            staging_inventory_sha256: sha256(b"[]"),
            staging_item_count: 0,
            retention_expires_at_epoch_ms: 0,
            canonical_postimages_verified: canonical_verified,
            receipt_chain_verified: false,
            index_verified: false,
            rollback_verified: false,
            cleanup_state: WriteCleanupState::NotRequired,
            failure_code: None,
            occurred_at_epoch_ms: previous
                .map_or(1_000, |checkpoint| checkpoint.occurred_at_epoch_ms + 1),
        }
    }

    fn canonical_checkpoint_chain() -> Vec<WriteAwareCheckpoint> {
        let mut chain = Vec::new();
        for (checkpoint_id, phase) in [
            (
                "checkpoint-native-knowledge-before",
                WriteCheckpointPhase::BeforeTransaction,
            ),
            (
                "checkpoint-native-knowledge-consumed",
                WriteCheckpointPhase::GrantConsumed,
            ),
            (
                "checkpoint-native-knowledge-applying",
                WriteCheckpointPhase::Applying,
            ),
            (
                "checkpoint-native-knowledge-canonical",
                WriteCheckpointPhase::CanonicalVerified,
            ),
        ] {
            let checkpoint = build_write_checkpoint(
                checkpoint_input(checkpoint_id, phase, chain.last()),
                chain.last(),
            )
            .expect("canonical checkpoint");
            chain.push(checkpoint);
        }
        chain
    }

    fn open_checkpoint_authority(root: &Path) -> DurableAuthorityRuntime {
        let observation = StrictLocalStorageObservation {
            filesystem: StorageFilesystemClass::Local,
            synchronization_marker: None,
            root_identity_sha256: [9; 32],
            symlink_free: true,
        };
        DurableAuthorityRuntime::open(
            &root.join("knowledge-checkpoints.db"),
            &observation,
            &mut TestKey([41; 32]),
            10_000,
        )
        .expect("durable knowledge checkpoint authority")
    }

    #[test]
    fn native_markdown_preview_commits_and_external_edit_fails_closed() {
        let mut committed = native_fixture();
        let selection = vault_selection(committed.workspace_path.workspace_id());
        let initial_snapshot = vault_snapshot(
            &selection,
            committed.workspace_path.clone(),
            &committed.source_bytes,
        );
        let mut index = ObsidianVaultIndex::in_memory().expect("native derived index");
        let initial_index = index.rebuild(&initial_snapshot).expect("initial index");
        assert_eq!(
            execute_native(&mut committed),
            Ok(FilesystemTransactionOutcome::Committed)
        );
        assert_eq!(
            fs::read(&committed.source_path).expect("native Markdown postimage"),
            committed.postimage_bytes
        );
        assert_eq!(
            fs::metadata(&committed.source_path)
                .expect("native Markdown metadata")
                .permissions()
                .mode()
                & 0o777,
            0o640
        );
        let post_snapshot = vault_snapshot(
            &selection,
            committed.workspace_path.clone(),
            &committed.postimage_bytes,
        );
        let publication = decide_index_publication(
            committed.mutation.clone(),
            CanonicalMarkdownWriteOutcome::Committed,
            Some(sha256(&committed.postimage_bytes)),
        );
        let published = index
            .publish_after_canonical_write(
                initial_index.report.revision,
                &publication,
                &post_snapshot,
            )
            .expect("publish native postimage");
        assert_eq!(
            published.state,
            KnowledgeIndexPublicationState::ReadyAfterCommit
        );
        assert_eq!(
            index
                .freshness(&post_snapshot)
                .expect("postimage freshness"),
            ObsidianVaultFreshness::Current
        );

        let mut stale = native_fixture();
        let stale_selection = vault_selection(stale.workspace_path.workspace_id());
        let stale_initial_snapshot = vault_snapshot(
            &stale_selection,
            stale.workspace_path.clone(),
            &stale.source_bytes,
        );
        let mut stale_index = ObsidianVaultIndex::in_memory().expect("stale derived index");
        stale_index
            .rebuild(&stale_initial_snapshot)
            .expect("stale initial index");
        let external = concat!(
            "---\n",
            "agentmage_id: knowledge-decision-native-001\n",
            "type: decision\n",
            "---\n",
            "# Native Decision\n",
            "## Decision\n",
            "External user decision.\n",
            "## Evidence\n",
            "Synthetic evidence.\n",
        )
        .as_bytes();
        fs::write(&stale.source_path, external).expect("external Markdown edit");
        assert_eq!(
            execute_native(&mut stale),
            Err(FilesystemTransactionError::PreapplyDenied)
        );
        assert_eq!(
            fs::read(&stale.source_path).expect("external edit preserved"),
            external
        );
        assert_ne!(stale.source_bytes, external);
        let external_snapshot =
            vault_snapshot(&stale_selection, stale.workspace_path.clone(), external);
        assert_eq!(
            stale_index
                .freshness(&external_snapshot)
                .expect("external edit freshness"),
            ObsidianVaultFreshness::Stale
        );
        let blocked_publication = decide_index_publication(
            stale.mutation,
            CanonicalMarkdownWriteOutcome::FailedNoChange,
            Some(sha256(external)),
        );
        assert_eq!(
            blocked_publication.state,
            KnowledgeIndexPublicationState::RebuildRequired
        );
    }

    #[test]
    fn native_index_publication_persists_and_reopens_exact_checkpoint_phases() {
        let mut fixture = native_fixture();
        let selection = vault_selection(fixture.workspace_path.workspace_id());
        let initial_snapshot = vault_snapshot(
            &selection,
            fixture.workspace_path.clone(),
            &fixture.source_bytes,
        );
        let mut index = ObsidianVaultIndex::in_memory().expect("native durable index");
        let initial = index.rebuild(&initial_snapshot).expect("initial index");
        assert_eq!(
            execute_native(&mut fixture),
            Ok(FilesystemTransactionOutcome::Committed)
        );
        let post_snapshot = vault_snapshot(
            &selection,
            fixture.workspace_path.clone(),
            &fixture.postimage_bytes,
        );
        let publication = decide_index_publication(
            fixture.mutation.clone(),
            CanonicalMarkdownWriteOutcome::Committed,
            Some(sha256(&fixture.postimage_bytes)),
        );
        let chain = canonical_checkpoint_chain();
        let transaction_id = chain[0].transaction_id.clone();
        {
            let mut authority = open_checkpoint_authority(fixture._root.path());
            authority
                .checkpoint_write_transaction(&chain)
                .expect("canonical chain persists");
            let result = publish_knowledge_index_with_checkpoints(
                &mut authority,
                chain.last().expect("canonical head"),
                &mut index,
                initial.report.revision,
                &publication,
                &post_snapshot,
                KnowledgeIndexCheckpointRequest {
                    updating_checkpoint_id: "checkpoint-native-knowledge-index-updating".to_owned(),
                    verified_checkpoint_id: "checkpoint-native-knowledge-index-verified".to_owned(),
                    updating_at_epoch_ms: 2_000,
                    verified_at_epoch_ms: 2_001,
                },
            )
            .expect("checkpointed index publication");
            assert_eq!(
                result.updating_checkpoint.phase,
                WriteCheckpointPhase::IndexUpdating
            );
            assert_eq!(
                result.verified_checkpoint.phase,
                WriteCheckpointPhase::IndexVerified
            );
            assert!(result.verified_checkpoint.index_verified);
            assert_eq!(
                result.verified_checkpoint.index_update_sha256.as_deref(),
                Some(result.index_update_sha256.as_str())
            );
        }
        let reopened = open_checkpoint_authority(fixture._root.path());
        let retained = reopened
            .write_checkpoint_chain(&transaction_id)
            .expect("verified index chain reopens");
        assert_eq!(
            retained
                .iter()
                .map(|checkpoint| checkpoint.phase)
                .collect::<Vec<_>>(),
            vec![
                WriteCheckpointPhase::BeforeTransaction,
                WriteCheckpointPhase::GrantConsumed,
                WriteCheckpointPhase::Applying,
                WriteCheckpointPhase::CanonicalVerified,
                WriteCheckpointPhase::IndexUpdating,
                WriteCheckpointPhase::IndexVerified,
            ]
        );
        assert!(retained.last().expect("retained head").index_verified);
        assert_eq!(
            index.freshness(&post_snapshot).expect("index freshness"),
            ObsidianVaultFreshness::Current
        );
    }

    #[test]
    fn stale_index_publication_retains_updating_checkpoint_without_false_verification() {
        let fixture = native_fixture();
        let selection = vault_selection(fixture.workspace_path.workspace_id());
        let post_snapshot = vault_snapshot(
            &selection,
            fixture.workspace_path.clone(),
            &fixture.postimage_bytes,
        );
        let initial_snapshot = vault_snapshot(
            &selection,
            fixture.workspace_path.clone(),
            &fixture.source_bytes,
        );
        let mut index = ObsidianVaultIndex::in_memory().expect("stale durable index");
        let initial = index
            .rebuild(&initial_snapshot)
            .expect("initial stale index");
        let publication = decide_index_publication(
            fixture.mutation.clone(),
            CanonicalMarkdownWriteOutcome::Committed,
            Some(sha256(&fixture.postimage_bytes)),
        );
        let chain = canonical_checkpoint_chain();
        let mut authority = open_checkpoint_authority(fixture._root.path());
        authority
            .checkpoint_write_transaction(&chain)
            .expect("canonical chain persists");
        assert_eq!(
            publish_knowledge_index_with_checkpoints(
                &mut authority,
                chain.last().expect("canonical head"),
                &mut index,
                initial.report.revision + 1,
                &publication,
                &post_snapshot,
                KnowledgeIndexCheckpointRequest {
                    updating_checkpoint_id: "checkpoint-native-knowledge-stale-updating".to_owned(),
                    verified_checkpoint_id: "checkpoint-native-knowledge-stale-verified".to_owned(),
                    updating_at_epoch_ms: 2_000,
                    verified_at_epoch_ms: 2_001,
                },
            ),
            Err(KnowledgeIndexCheckpointError::IndexPublicationFailed)
        );
        let retained = authority
            .write_checkpoint_chain(&chain[0].transaction_id)
            .expect("stale checkpoint chain");
        assert_eq!(
            retained.last().expect("stale checkpoint head").phase,
            WriteCheckpointPhase::IndexUpdating
        );
        assert!(
            !retained
                .last()
                .expect("stale checkpoint head")
                .index_verified
        );
        assert_eq!(
            index
                .freshness(&initial_snapshot)
                .expect("original freshness"),
            ObsidianVaultFreshness::Current
        );
    }

    #[test]
    fn native_knowledge_crash_boundary_child() {
        if env::var_os("AGENTMAGE_KNOWLEDGE_CRASH_CHILD").is_some() {
            run_knowledge_crash_child();
        }
    }

    #[test]
    fn native_process_stops_rebuild_index_only_from_canonical_markdown() {
        let workspace_id = WorkspaceId::from_raw("workspace-native-knowledge");
        let workspace_path = WorkspacePath::new(workspace_id.clone(), ["notes", "decision.md"])
            .expect("recovery path");
        let selection = vault_selection(&workspace_id);
        for boundary in KnowledgeCrashBoundary::ALL {
            let root = TestDirectory::new();
            launch_knowledge_crash_child(root.path(), boundary);
            let source_path = root.path().join("notes/decision.md");
            let canonical = fs::read(&source_path).expect("canonical Markdown after stop");
            let document = MarkdownDocument::parse(workspace_path.clone(), canonical.clone())
                .expect("canonical Markdown remains parseable");
            let decision = document
                .elements()
                .iter()
                .find(|element| element.label == "Decision")
                .expect("Decision heading remains present");
            assert_eq!(decision.heading_level, Some(2));
            if boundary.source_committed() {
                assert!(
                    std::str::from_utf8(&canonical)
                        .expect("canonical UTF-8")
                        .contains("Revised through the native boundary.")
                );
            } else {
                assert!(
                    std::str::from_utf8(&canonical)
                        .expect("canonical UTF-8")
                        .contains("Original decision.")
                );
            }
            let snapshot = vault_snapshot(&selection, workspace_path.clone(), &canonical);
            let mut rebuilt = ObsidianVaultIndex::in_memory().expect("fresh recovery index");
            rebuilt.rebuild(&snapshot).expect("rebuild from canonical");
            assert_eq!(
                rebuilt.freshness(&snapshot).expect("recovery freshness"),
                ObsidianVaultFreshness::Current,
                "{boundary:?}"
            );
            assert!(
                fs::read_dir(root.path().join("notes"))
                    .expect("knowledge parent listing")
                    .all(|entry| !entry
                        .expect("knowledge entry")
                        .file_name()
                        .to_string_lossy()
                        .starts_with(".agentmage-write-")),
                "{boundary:?}"
            );
        }
    }
}
