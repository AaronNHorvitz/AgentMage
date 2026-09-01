//! v0.5 manual-frontier release compositions and controlled local export.

/// Durable content-free recovery for the manual frontier release round trip.
#[path = "frontier_release_recovery.rs"]
pub mod recovery;

use agentmage_kernel_contracts::{CapabilityGrant, RenderedHandoff};

use crate::{
    filesystem_control::{
        FileClassification, FilesystemOperationDraft, FilesystemPlan, FilesystemPlanError,
        FilesystemPlanRequest, NewDestinationDraft, build_filesystem_plan,
    },
    handoff::verify_rendered_handoff,
    write_approval::WriteReviewNarrative,
};

/// Stable failure while preparing a user-initiated packet export plan.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FrontierPacketExportError {
    /// The request was not initiated by an exact current user action.
    UserInitiationRequired,
    /// The rendered packet or no-delivery receipt was not current and exact.
    InvalidRenderedPacket,
    /// The requested destination was not one visible Markdown file.
    InvalidDestination,
    /// The controlled filesystem pack rejected scope, state, or plan input.
    FilesystemPlanDenied,
}

impl FrontierPacketExportError {
    /// Returns a stable content-free error code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::UserInitiationRequired => "frontier.export.user-initiation-required",
            Self::InvalidRenderedPacket => "frontier.export.packet-invalid",
            Self::InvalidDestination => "frontier.export.destination-invalid",
            Self::FilesystemPlanDenied => "frontier.export.filesystem-plan-denied",
        }
    }
}

impl std::fmt::Display for FrontierPacketExportError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.code())
    }
}

impl std::error::Error for FrontierPacketExportError {}

impl From<FilesystemPlanError> for FrontierPacketExportError {
    fn from(_: FilesystemPlanError) -> Self {
        Self::FilesystemPlanDenied
    }
}

/// Exact local-only request to prepare one controlled packet-file creation plan.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FrontierPacketExportRequest {
    /// Stable controlled-filesystem plan identity.
    pub plan_id: String,
    /// Stable operation identity within that plan.
    pub operation_id: String,
    /// Current kernel clock used by the parent grant.
    pub observed_at_epoch_ms: u64,
    /// Exact absent local Markdown destination.
    pub destination: NewDestinationDraft,
    /// True only for an explicit current user export command.
    pub user_initiated: bool,
}

/// Builds an authority-free controlled-write plan for exact reviewed packet bytes.
pub fn build_frontier_packet_export_plan(
    parent: &CapabilityGrant,
    rendered: &RenderedHandoff,
    request: FrontierPacketExportRequest,
) -> Result<FilesystemPlan, FrontierPacketExportError> {
    if !request.user_initiated {
        return Err(FrontierPacketExportError::UserInitiationRequired);
    }
    verify_rendered_handoff(rendered)
        .map_err(|_| FrontierPacketExportError::InvalidRenderedPacket)?;
    let destination_name = request
        .destination
        .path
        .components()
        .last()
        .map(|component| component.as_str())
        .ok_or(FrontierPacketExportError::InvalidDestination)?;
    if !destination_name.ends_with(".md") || destination_name.starts_with('.') {
        return Err(FrontierPacketExportError::InvalidDestination);
    }
    let plan = build_filesystem_plan(
        parent,
        FilesystemPlanRequest {
            plan_id: request.plan_id,
            observed_at_epoch_ms: request.observed_at_epoch_ms,
            operations: vec![FilesystemOperationDraft::Create {
                operation_id: request.operation_id,
                destination: request.destination,
                content: rendered.packet_markdown.as_bytes().to_vec(),
                mode: 0o600,
                classification: FileClassification::Documentation,
            }],
            review: WriteReviewNarrative {
                rationale: "Export the exact reviewed local frontier packet".to_owned(),
                behavior_change:
                    "Create one user-selected local Markdown file; no external delivery".to_owned(),
                verification_plan: vec![
                    "Re-read the file and compare its SHA-256 to the reviewed packet".to_owned(),
                ],
                risks: vec![
                    "The user may later disclose the file through a separate product".to_owned(),
                ],
                rollback: "Use a fresh controlled trash-delete review for the exact file"
                    .to_owned(),
                unverified_assumptions: vec![
                    "No external handling or destination is represented by this local plan"
                        .to_owned(),
                ],
            },
            permitted_verification: vec!["frontier-packet-sha256".to_owned()],
        },
    )?;
    Ok(plan)
}

#[cfg(test)]
mod tests {
    use agentmage_kernel_contracts::{
        CapabilityGrant, GrantTarget, HandoffDestinationClass, HandoffDisclosureEntry,
        HandoffDraft, HandoffEntryDisposition, HandoffEntryKind, HandoffSensitivity, WorkspaceId,
        WorkspacePath,
    };
    use serde_json::json;
    use sha2::{Digest, Sha256};

    use super::{
        FrontierPacketExportError, FrontierPacketExportRequest, build_frontier_packet_export_plan,
    };
    use crate::{
        filesystem_control::NewDestinationDraft,
        handoff::{build_handoff_review, render_reviewed_handoff, seal_handoff_entry},
    };

    fn parent() -> CapabilityGrant {
        serde_json::from_value(json!({
            "schema_version": 2,
            "grant_id": "grant-frontier-export-parent",
            "revision": 0,
            "grant_class": "session_read",
            "actor_id": "actor-local",
            "approval_id": null,
            "session_id": "session-frontier-export",
            "task_id": "task-frontier-export",
            "action_id": null,
            "action_kind": null,
            "operation": {
                "taxonomy_version": 1,
                "operation": "workspace_read",
                "authority_class": "observe"
            },
            "tool_id": null,
            "tool_version": null,
            "targets": [{
                "target_kind": "workspace_scope",
                "path": {"workspace_id": "workspace-frontier-export", "components": []},
                "authorization_id": "authorization-frontier-export",
                "adapter_instance_id": "adapter-frontier-export",
                "platform": "deterministic_fake"
            }],
            "excluded_targets": [],
            "sensitivity": "restricted",
            "argument_sha256": "1".repeat(64),
            "preimages": [],
            "expected_side_effects": [],
            "rollback_description": "No read effect",
            "issued_at_epoch_ms": 1000,
            "expires_at_epoch_ms": 100000,
            "nonce": "nonce-frontier-export-parent",
            "use_limit": 10,
            "use_count": 0,
            "parent_grant_id": null,
            "parent_grant_sha256": null,
            "preview_sha256": "2".repeat(64),
            "policy_sha256": "3".repeat(64),
            "status": "issued"
        }))
        .expect("parent grant")
    }

    fn destination(name: &str) -> NewDestinationDraft {
        let identity: [u8; 32] = Sha256::digest(b"exports").into();
        let parent: GrantTarget = serde_json::from_value(json!({
            "target_kind": "held_object",
            "path": {"workspace_id": "workspace-frontier-export", "components": ["exports"]},
            "authorization_id": "authorization-frontier-export",
            "adapter_instance_id": "adapter-frontier-export",
            "platform": "deterministic_fake",
            "object_kind": "directory",
            "object_identity": {
                "platform": "deterministic_fake",
                "mount_identity_sha256": vec![1_u8; 32],
                "object_identity_sha256": identity
            },
            "preimage": null
        }))
        .expect("destination parent");
        NewDestinationDraft {
            parent,
            path: WorkspacePath::new(
                WorkspaceId::from_raw("workspace-frontier-export"),
                ["exports", name],
            )
            .expect("destination path"),
            observed_sibling_names: vec![],
        }
    }

    fn rendered() -> agentmage_kernel_contracts::RenderedHandoff {
        let entry = seal_handoff_entry(HandoffDisclosureEntry {
            entry_id: "entry-0001".to_owned(),
            kind: HandoffEntryKind::SourceExcerpt,
            source_id: "source-0001".to_owned(),
            display_source: "src/lib.rs".to_owned(),
            fragment: "lines-1-2".to_owned(),
            excerpt: "fn reviewed() {}".to_owned(),
            content_sha256: "4".repeat(64),
            sensitivity: HandoffSensitivity::Public,
            related: true,
            hidden: false,
            disposition: HandoffEntryDisposition::Include,
            redactions: vec![],
            entry_sha256: String::new(),
        })
        .expect("sealed entry");
        let draft = HandoffDraft {
            schema_version: 2,
            handoff_id: "handoff-frontier-export".to_owned(),
            workspace_state_sha256: "5".repeat(64),
            policy_sha256: "6".repeat(64),
            redaction_policy_sha256: "7".repeat(64),
            objective: "Review a local proposal".to_owned(),
            acceptance_criteria: vec!["Return an evidence-backed proposal".to_owned()],
            constraints: vec!["Do not claim authority".to_owned()],
            entries: vec![entry],
            exclusions: vec!["Credentials".to_owned()],
            unresolved_questions: vec!["Which local test should run?".to_owned()],
            destination: HandoffDestinationClass::ManualCodexInterface,
        };
        let review = build_handoff_review(&draft, "preview-frontier-export".to_owned(), 10_000)
            .expect("review");
        render_reviewed_handoff(
            &review,
            &draft,
            2_000,
            false,
            "attempt-frontier-export".to_owned(),
        )
        .expect("rendered packet")
    }

    fn request(name: &str, user_initiated: bool) -> FrontierPacketExportRequest {
        FrontierPacketExportRequest {
            plan_id: "frontier-export-plan-0001".to_owned(),
            operation_id: "frontier-export-create-0001".to_owned(),
            observed_at_epoch_ms: 2_000,
            destination: destination(name),
            user_initiated,
        }
    }

    #[test]
    fn exact_rendered_packet_becomes_one_authority_free_controlled_create_plan() {
        let rendered = rendered();
        let plan = build_frontier_packet_export_plan(
            &parent(),
            &rendered,
            request("frontier-packet.md", true),
        )
        .expect("controlled export plan");
        assert_eq!(plan.operations().len(), 1);
        assert_eq!(
            plan.operations()[0].postimage_bytes(),
            rendered.packet_markdown.as_bytes()
        );
        assert_eq!(plan.operations()[0].destination_mode(), Some(0o600));
        assert!(!plan.requires_high_risk_delete_grant());
    }

    #[test]
    fn automatic_non_markdown_and_mutated_packet_exports_fail_before_a_plan() {
        let rendered = rendered();
        assert_eq!(
            build_frontier_packet_export_plan(
                &parent(),
                &rendered,
                request("frontier-packet.md", false)
            ),
            Err(FrontierPacketExportError::UserInitiationRequired)
        );
        assert_eq!(
            build_frontier_packet_export_plan(
                &parent(),
                &rendered,
                request("frontier-packet.txt", true)
            ),
            Err(FrontierPacketExportError::InvalidDestination)
        );
        let mut changed = rendered;
        changed.packet_markdown.push_str("changed");
        assert_eq!(
            build_frontier_packet_export_plan(
                &parent(),
                &changed,
                request("frontier-packet.md", true)
            ),
            Err(FrontierPacketExportError::InvalidRenderedPacket)
        );
    }
}
