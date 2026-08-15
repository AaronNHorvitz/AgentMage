//! Composition of verified knowledge previews into kernel filesystem drafts.

use std::fmt::Write as _;

use agentmage_capability_knowledge::{
    KnowledgeNoteCreatePreview, MarkdownDocument, MarkdownUpdatePreview,
    verify_knowledge_note_create_preview, verify_markdown_update_preview,
};
use agentmage_kernel_contracts::GrantTarget;
use agentmage_kernel_engine::filesystem_control::{
    ExistingSourceDraft, ExistingWorkDisposition, FileClassification, FilesystemOperationDraft,
    NewDestinationDraft, StructuredPatch, StructuredPatchHunk,
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
