//! Composition of verified structured file plans into one kernel shadow change set.

use std::collections::BTreeSet;
use std::fmt::Write;

use agentmage_capability_repository_map::{
    StructuredArtifactClass, StructuredFileChangePlan, StructuredReviewHook,
    verify_structured_file_change,
};
use agentmage_kernel_contracts::{CapabilityGrant, GrantTarget, WorkspaceObjectKind};
use agentmage_kernel_engine::write_approval::{
    ShadowChangeSet, ShadowChangeSetRequest, ShadowWriteDraft, WriteApprovalError,
    WriteArtifactClass, WriteChangeScope, WriteLineEndings, WriteReviewHook, WriteReviewNarrative,
    WriteSyntax, build_shadow_change_set,
};
use sha2::{Digest, Sha256};

const MAX_STRUCTURED_CHANGES: usize = 128;

/// One exact held target paired with one verified authority-free file plan.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BoundStructuredChange {
    /// Stable operation identity inside the shadow change set.
    pub operation_id: String,
    /// Continuously held exact regular-file target.
    pub target: GrantTarget,
    /// Exact syntax-aware or bounded-text proposal.
    pub plan: StructuredFileChangePlan,
}

/// Exact request to compose multiple file plans under one approved intent and change plan.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StructuredShadowChangeSetRequest {
    /// Stable change-set identity.
    pub change_set_id: String,
    /// Kernel-clock observation time.
    pub observed_at_epoch_ms: u64,
    /// Exact approved intent identity shared by every file plan.
    pub intent_sha256: String,
    /// Exact approved change-plan identity shared by every file plan.
    pub change_plan_sha256: String,
    /// Explicit breadth of the complete multi-file change.
    pub scope: WriteChangeScope,
    /// Separate approval for a nonminimal breadth.
    pub expanded_scope_approval_sha256: Option<String>,
    /// Stable path-sorted file plans and held targets.
    pub changes: Vec<BoundStructuredChange>,
    /// Complete bounded review narrative.
    pub review: WriteReviewNarrative,
    /// Separately grantable post-write checks.
    pub permitted_verification: Vec<String>,
}

/// Stable content-free composition failure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StructuredCodeChangeError {
    /// The request is malformed, mixed, duplicated, or not path ordered.
    InvalidInput,
    /// A structured plan does not verify exactly.
    PlanInvalid,
    /// A held target differs from the plan path or preimage.
    TargetMismatch,
    /// The kernel rejected the resulting shadow change request.
    ShadowRejected,
}

impl StructuredCodeChangeError {
    /// Stable content-free error code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidInput => "code-change.input-invalid",
            Self::PlanInvalid => "code-change.plan-invalid",
            Self::TargetMismatch => "code-change.target-mismatch",
            Self::ShadowRejected => "code-change.shadow-rejected",
        }
    }
}

impl From<WriteApprovalError> for StructuredCodeChangeError {
    fn from(_: WriteApprovalError) -> Self {
        Self::ShadowRejected
    }
}

/// Builds one ordered kernel shadow change set without granting write authority.
pub fn build_structured_shadow_change_set(
    parent: &CapabilityGrant,
    request: StructuredShadowChangeSetRequest,
) -> Result<ShadowChangeSet, StructuredCodeChangeError> {
    if request.changes.is_empty()
        || request.changes.len() > MAX_STRUCTURED_CHANGES
        || !is_sha256(&request.intent_sha256)
        || !is_sha256(&request.change_plan_sha256)
        || request.changes.windows(2).any(|pair| {
            pair[0].plan.summary().path >= pair[1].plan.summary().path
                || pair[0].operation_id >= pair[1].operation_id
        })
    {
        return Err(StructuredCodeChangeError::InvalidInput);
    }

    let mut operations = Vec::with_capacity(request.changes.len());
    let mut review_hooks = BTreeSet::new();
    for change in request.changes {
        if !verify_structured_file_change(&change.plan) {
            return Err(StructuredCodeChangeError::PlanInvalid);
        }
        let summary = change.plan.summary();
        if summary.intent_sha256 != request.intent_sha256
            || summary.change_plan_sha256 != request.change_plan_sha256
            || change.target.workspace_path() != Some(&summary.path)
            || change.target.object_kind() != Some(WorkspaceObjectKind::RegularFile)
            || change.target.preimage().is_none_or(|preimage| {
                preimage.byte_len()
                    != u64::try_from(change.plan.preimage().len()).unwrap_or(u64::MAX)
                    || hex_bytes(preimage.content_sha256()) != summary.preimage_sha256
            })
            || sha256_hex(change.plan.preimage()) != summary.preimage_sha256
            || sha256_hex(change.plan.postimage()) != summary.postimage_sha256
        {
            return Err(StructuredCodeChangeError::TargetMismatch);
        }
        review_hooks.extend(summary.review_hooks.iter().copied().map(map_review_hook));
        let generated = summary.artifact_class == StructuredArtifactClass::GeneratedOutput;
        operations.push(ShadowWriteDraft {
            operation_id: change.operation_id,
            target: change.target,
            observed_bytes: change.plan.preimage().to_vec(),
            proposed_bytes: change.plan.postimage().to_vec(),
            expected_postimage_sha256: summary.postimage_sha256.clone(),
            artifact_class: map_artifact_class(summary.artifact_class),
            syntax: syntax_for_path(&summary.path),
            line_endings: line_endings(change.plan.postimage())?,
            generated_file: generated,
            allow_generated_file: generated,
        });
    }

    build_shadow_change_set(
        parent,
        ShadowChangeSetRequest {
            change_set_id: request.change_set_id,
            observed_at_epoch_ms: request.observed_at_epoch_ms,
            intent_sha256: request.intent_sha256,
            plan_sha256: request.change_plan_sha256,
            scope: request.scope,
            expanded_scope_approval_sha256: request.expanded_scope_approval_sha256,
            review_hooks: review_hooks.into_iter().collect(),
            operations,
            review: request.review,
            permitted_verification: request.permitted_verification,
        },
    )
    .map_err(Into::into)
}

const fn map_artifact_class(value: StructuredArtifactClass) -> WriteArtifactClass {
    match value {
        StructuredArtifactClass::Code => WriteArtifactClass::Code,
        StructuredArtifactClass::Configuration => WriteArtifactClass::Configuration,
        StructuredArtifactClass::Test => WriteArtifactClass::Test,
        StructuredArtifactClass::Documentation => WriteArtifactClass::Documentation,
        StructuredArtifactClass::Migration => WriteArtifactClass::Migration,
        StructuredArtifactClass::GeneratedOutput => WriteArtifactClass::GeneratedOutput,
    }
}

const fn map_review_hook(value: StructuredReviewHook) -> WriteReviewHook {
    match value {
        StructuredReviewHook::Interface => WriteReviewHook::Interface,
        StructuredReviewHook::Dependency => WriteReviewHook::Dependency,
        StructuredReviewHook::Migration => WriteReviewHook::Migration,
        StructuredReviewHook::Security => WriteReviewHook::Security,
        StructuredReviewHook::Performance => WriteReviewHook::Performance,
        StructuredReviewHook::Accessibility => WriteReviewHook::Accessibility,
        StructuredReviewHook::Compatibility => WriteReviewHook::Compatibility,
    }
}

fn syntax_for_path(path: &agentmage_kernel_contracts::WorkspacePath) -> WriteSyntax {
    if path
        .components()
        .last()
        .is_some_and(|name| name.as_str().ends_with(".json"))
    {
        WriteSyntax::Json
    } else {
        WriteSyntax::Utf8
    }
}

fn line_endings(bytes: &[u8]) -> Result<WriteLineEndings, StructuredCodeChangeError> {
    if !bytes.contains(&b'\n') && !bytes.contains(&b'\r') {
        return Ok(WriteLineEndings::None);
    }
    if !bytes.contains(&b'\r') {
        return Ok(WriteLineEndings::Lf);
    }
    let mut index = 0;
    while index < bytes.len() {
        match bytes[index] {
            b'\r' if bytes.get(index + 1) == Some(&b'\n') => index += 2,
            b'\r' | b'\n' => return Err(StructuredCodeChangeError::InvalidInput),
            _ => index += 1,
        }
    }
    Ok(WriteLineEndings::CrLf)
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}

fn sha256_hex(bytes: &[u8]) -> String {
    hex_bytes(&Sha256::digest(bytes))
}

fn hex_bytes(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        write!(&mut output, "{byte:02x}").expect("writing to String cannot fail");
    }
    output
}

#[cfg(test)]
mod tests {
    use agentmage_capability_repository_map::{
        StructuredEdit, StructuredFileChangeRequest, StructuredLanguage, StructuredReviewHook,
        build_structured_file_change,
    };
    use agentmage_kernel_contracts::{
        ActorId, AdapterInstanceId, DataSensitivity, FilePreimage, GrantId, GrantNonce,
        PathPlatform, SessionId, TaskId, WorkspaceAuthorizationId, WorkspaceId,
        WorkspaceObjectIdentity, WorkspacePath,
    };
    use agentmage_kernel_engine::grants::{GrantIssuer, SessionReadGrantRequest};
    use agentmage_kernel_engine::write_approval::render_write_preview;

    use super::*;

    fn path(parts: &[&str]) -> WorkspacePath {
        WorkspacePath::new(
            WorkspaceId::from_raw("workspace-code-change"),
            parts.iter().copied(),
        )
        .expect("path")
    }

    fn target(parts: &[&str], bytes: &[u8]) -> GrantTarget {
        #[derive(Debug)]
        struct Held {
            path: WorkspacePath,
            identity: WorkspaceObjectIdentity,
            preimage: FilePreimage,
        }
        impl agentmage_kernel_contracts::HeldWorkspaceObject for Held {
            fn workspace_path(&self) -> &WorkspacePath {
                &self.path
            }
            fn authorization_id(&self) -> &WorkspaceAuthorizationId {
                static ID: std::sync::OnceLock<WorkspaceAuthorizationId> =
                    std::sync::OnceLock::new();
                ID.get_or_init(|| WorkspaceAuthorizationId::from_raw("authorization-code-change"))
            }
            fn adapter_instance_id(&self) -> &AdapterInstanceId {
                static ID: std::sync::OnceLock<AdapterInstanceId> = std::sync::OnceLock::new();
                ID.get_or_init(|| AdapterInstanceId::from_raw("adapter-code-change"))
            }
            fn object_identity(&self) -> &WorkspaceObjectIdentity {
                &self.identity
            }
            fn intent(&self) -> agentmage_kernel_contracts::PathResolutionIntent {
                agentmage_kernel_contracts::PathResolutionIntent::ReadFile
            }
            fn object_kind(&self) -> WorkspaceObjectKind {
                WorkspaceObjectKind::RegularFile
            }
            fn preimage(&self) -> Option<&FilePreimage> {
                Some(&self.preimage)
            }
        }
        let digest: [u8; 32] = Sha256::digest(bytes).into();
        GrantTarget::held_object(&Held {
            path: path(parts),
            identity: WorkspaceObjectIdentity::new(
                PathPlatform::DeterministicFake,
                [1; 32],
                digest,
            ),
            preimage: FilePreimage::new(u64::try_from(bytes.len()).expect("length"), digest),
        })
        .expect("target")
    }

    fn parent() -> CapabilityGrant {
        #[derive(Debug)]
        struct Workspace;
        impl agentmage_kernel_contracts::AuthorizedWorkspaceHandle for Workspace {
            fn workspace_id(&self) -> &WorkspaceId {
                static ID: std::sync::OnceLock<WorkspaceId> = std::sync::OnceLock::new();
                ID.get_or_init(|| WorkspaceId::from_raw("workspace-code-change"))
            }
            fn authorization_id(&self) -> &WorkspaceAuthorizationId {
                static ID: std::sync::OnceLock<WorkspaceAuthorizationId> =
                    std::sync::OnceLock::new();
                ID.get_or_init(|| WorkspaceAuthorizationId::from_raw("authorization-code-change"))
            }
            fn adapter_instance_id(&self) -> &AdapterInstanceId {
                static ID: std::sync::OnceLock<AdapterInstanceId> = std::sync::OnceLock::new();
                ID.get_or_init(|| AdapterInstanceId::from_raw("adapter-code-change"))
            }
            fn platform(&self) -> PathPlatform {
                PathPlatform::DeterministicFake
            }
        }
        let scope = GrantTarget::workspace_scope(
            &Workspace,
            agentmage_kernel_contracts::WorkspaceScopePath::new(
                WorkspaceId::from_raw("workspace-code-change"),
                std::iter::empty::<&str>(),
            )
            .expect("scope"),
        )
        .expect("target");
        GrantIssuer::new()
            .issue_session_read(SessionReadGrantRequest {
                grant_id: GrantId::from_raw("grant-code-change-parent"),
                actor_id: ActorId::from_raw("actor-local"),
                session_id: SessionId::from_raw("session-code-change"),
                task_id: TaskId::from_raw("task-code-change"),
                targets: vec![scope],
                excluded_targets: Vec::new(),
                sensitivity: DataSensitivity::Restricted,
                issued_at_epoch_ms: 1_000,
                expires_at_epoch_ms: 20_000,
                nonce: GrantNonce::from_raw("nonce-code-change-parent"),
                maximum_derived_operations: 8,
                preview_sha256: "a".repeat(64),
                policy_sha256: "b".repeat(64),
            })
            .expect("parent")
    }

    fn plan(
        parts: &[&str],
        language: StructuredLanguage,
        source: &str,
        old: &str,
        new: &str,
    ) -> StructuredFileChangePlan {
        build_structured_file_change(StructuredFileChangeRequest {
            change_id: format!("change-{}", parts.last().expect("name").replace('.', "-")),
            path: path(parts),
            intent_sha256: "1".repeat(64),
            change_plan_sha256: "2".repeat(64),
            language,
            artifact_class: StructuredArtifactClass::Code,
            preimage: source.as_bytes().to_vec(),
            expected_preimage_sha256: sha256_hex(source.as_bytes()),
            edits: vec![if matches!(language, StructuredLanguage::Go) {
                StructuredEdit::ReplaceExactText {
                    edit_id: "edit-exact".to_owned(),
                    expected: old.to_owned(),
                    replacement: new.to_owned(),
                }
            } else {
                StructuredEdit::RenameIdentifier {
                    edit_id: "edit-rename".to_owned(),
                    old: old.to_owned(),
                    replacement: new.to_owned(),
                }
            }],
            additional_review_hooks: vec![StructuredReviewHook::Security],
            generated: false,
            allow_generated: false,
        })
        .expect("plan")
    }

    fn review() -> WriteReviewNarrative {
        WriteReviewNarrative {
            rationale: "Apply the exact reviewed multi-language change".to_owned(),
            behavior_change: "Both fixtures expose the approved symbol".to_owned(),
            verification_plan: vec!["Run focused fixture checks".to_owned()],
            risks: vec!["Callers may rely on the prior symbol".to_owned()],
            rollback: "Restore exact reviewed preimages through a new grant".to_owned(),
            unverified_assumptions: vec!["External callers were not inspected".to_owned()],
        }
    }

    fn request() -> StructuredShadowChangeSetRequest {
        let go_source = "package sample\nconst oldName = 1\n";
        let py_source = "def old_name(value):\n    return value\n";
        StructuredShadowChangeSetRequest {
            change_set_id: "change-set-structured".to_owned(),
            observed_at_epoch_ms: 2_000,
            intent_sha256: "1".repeat(64),
            change_plan_sha256: "2".repeat(64),
            scope: WriteChangeScope::Minimal,
            expanded_scope_approval_sha256: None,
            changes: vec![
                BoundStructuredChange {
                    operation_id: "operation-go".to_owned(),
                    target: target(&["src", "sample.go"], go_source.as_bytes()),
                    plan: plan(
                        &["src", "sample.go"],
                        StructuredLanguage::Go,
                        go_source,
                        "oldName",
                        "newName",
                    ),
                },
                BoundStructuredChange {
                    operation_id: "operation-python".to_owned(),
                    target: target(&["src", "sample.py"], py_source.as_bytes()),
                    plan: plan(
                        &["src", "sample.py"],
                        StructuredLanguage::Python,
                        py_source,
                        "old_name",
                        "new_name",
                    ),
                },
            ],
            review: review(),
            permitted_verification: vec!["focused-fixtures".to_owned()],
        }
    }

    #[test]
    fn mixed_language_plans_become_one_exact_authority_free_shadow_change_set() {
        let change_set =
            build_structured_shadow_change_set(&parent(), request()).expect("change set");
        let preview = render_write_preview(&change_set).expect("preview");
        assert_eq!(preview.files, ["src/sample.go", "src/sample.py"]);
        assert_eq!(preview.operations, ["operation-go", "operation-python"]);
        assert!(preview.review_hooks.contains(&WriteReviewHook::Security));
        assert!(preview.review_hooks.contains(&WriteReviewHook::Interface));
        assert!(
            preview
                .review_hooks
                .contains(&WriteReviewHook::Compatibility)
        );
        assert!(change_set.operations().iter().all(|operation| {
            operation.preimage_bytes() != operation.proposed_bytes()
                && operation.artifact_class() == WriteArtifactClass::Code
        }));
    }

    #[test]
    fn stale_targets_mixed_plans_and_reordering_fail_before_a_shadow_change_exists() {
        let mut stale = request();
        stale.changes[0].target = target(&["src", "sample.go"], b"changed concurrently\n");
        assert_eq!(
            build_structured_shadow_change_set(&parent(), stale),
            Err(StructuredCodeChangeError::TargetMismatch)
        );

        let mut mixed = request();
        mixed.change_plan_sha256 = "9".repeat(64);
        assert_eq!(
            build_structured_shadow_change_set(&parent(), mixed),
            Err(StructuredCodeChangeError::TargetMismatch)
        );

        let mut reordered = request();
        reordered.changes.reverse();
        assert_eq!(
            build_structured_shadow_change_set(&parent(), reordered),
            Err(StructuredCodeChangeError::InvalidInput)
        );
    }
}
