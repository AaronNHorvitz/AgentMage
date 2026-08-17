//! Profile-bound authority planning for validated native coding calls.

use std::fmt::Write;

use agentmage_kernel_contracts::{
    GrantOperation, OperationBinding, StateChange, ToolCall, WorkspacePath,
};
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::{
    coding_dispatch::{
        NativeCodingCallPreparer, NativeCodingDispatchError, PreparedNativeCodingCall,
    },
    coding_projection::{CodingProjectionObject, CodingRepositoryProjection},
    coding_session::CodingSessionProfile,
};

/// Exact trusted workspace material that must be resolved before authority can be requested.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "target_kind", rename_all = "snake_case")]
pub enum NativeCodingTargetPlan {
    /// One bounded path-ordered projection selected from the frozen repository map.
    ReadProjection {
        /// Exact selected objects and expected object kinds.
        objects: Vec<CodingProjectionObject>,
        /// Digest of the tool request and complete selected projection.
        projection_sha256: String,
    },
    /// The exact continuously held AgentMage-owned worktree root.
    OwnedWorktreeRoot,
    /// One existing regular file whose current preimage must still match.
    ExistingFile {
        /// Canonical path inside the owned worktree.
        path: WorkspacePath,
        /// Exact preimage digest observed by the proposing model.
        expected_preimage_sha256: String,
    },
    /// One exact parent directory and direct-child destination.
    DestinationParent {
        /// Canonical non-root parent, or `None` for the held worktree root.
        parent: Option<WorkspacePath>,
        /// Canonical absent destination path.
        destination: WorkspacePath,
        /// Digest of the trusted parent and sibling projection seen by the model.
        expected_parent_sha256: String,
    },
}

/// One validated native call bound to its operation, target plan, and expected effect class.
#[derive(Clone, Debug)]
pub struct PreparedNativeCodingOperation {
    operation: OperationBinding,
    target: NativeCodingTargetPlan,
    expected_state_change: StateChange,
    prepared: PreparedNativeCodingCall,
    plan_sha256: String,
}

impl PreparedNativeCodingOperation {
    /// Returns the exact canonical grant operation required by this call.
    #[must_use]
    pub const fn operation(&self) -> OperationBinding {
        self.operation
    }

    /// Returns the trusted workspace material that a platform binder must resolve.
    #[must_use]
    pub const fn target(&self) -> &NativeCodingTargetPlan {
        &self.target
    }

    /// Returns the state-change class a truthful terminal tool result must report.
    #[must_use]
    pub const fn expected_state_change(&self) -> StateChange {
        self.expected_state_change
    }

    /// Returns the provider-specific authority-free plan.
    #[must_use]
    pub const fn prepared(&self) -> &PreparedNativeCodingCall {
        &self.prepared
    }

    /// Returns the digest binding profile, call, operation, target, and effect class.
    #[must_use]
    pub fn plan_sha256(&self) -> &str {
        &self.plan_sha256
    }
}

/// Stable content-free refusal while binding one call to trusted workspace material.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NativeCodingOperationError {
    /// The call or provider-specific plan failed the immutable native catalog.
    DispatchDenied,
    /// The repository map could not select the requested exact object projection.
    ProjectionDenied,
    /// The definition, operation, target, or expected effect class disagreed.
    InvariantDenied,
}

impl NativeCodingOperationError {
    /// Returns one stable content-free diagnostic code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::DispatchDenied => "runtime.coding-operation.dispatch-denied",
            Self::ProjectionDenied => "runtime.coding-operation.projection-denied",
            Self::InvariantDenied => "runtime.coding-operation.invariant-denied",
        }
    }
}

impl From<NativeCodingDispatchError> for NativeCodingOperationError {
    fn from(_: NativeCodingDispatchError) -> Self {
        Self::DispatchDenied
    }
}

/// Plans native coding authority against one immutable profile and repository projection.
pub struct NativeCodingOperationPlanner<'session> {
    profile: &'session CodingSessionProfile,
    projection: &'session CodingRepositoryProjection,
}

impl<'session> NativeCodingOperationPlanner<'session> {
    /// Binds operation planning to the exact session profile and repository map.
    #[must_use]
    pub const fn new(
        profile: &'session CodingSessionProfile,
        projection: &'session CodingRepositoryProjection,
    ) -> Self {
        Self {
            profile,
            projection,
        }
    }

    /// Validates one call and produces only an inert, profile-bound authority plan.
    pub fn prepare(
        &self,
        call: &ToolCall,
    ) -> Result<PreparedNativeCodingOperation, NativeCodingOperationError> {
        let definition = self
            .profile
            .registry()
            .get_tool(&call.tool_id, &call.tool_version)
            .ok_or(NativeCodingOperationError::DispatchDenied)?;
        let prepared = NativeCodingCallPreparer::new(self.profile).prepare(call)?;
        let (operation, target, expected_state_change) =
            target_for(self.profile.write_scope(), self.projection, &prepared)?;
        if definition.declared_effects.as_slice() != [operation]
            || definition.required_grant.operation != operation
            || !definition.required_grant.single_use
        {
            return Err(NativeCodingOperationError::InvariantDenied);
        }
        let plan_sha256 = plan_sha256(
            self.profile.profile_sha256(),
            call,
            operation,
            &target,
            expected_state_change,
        )?;
        Ok(PreparedNativeCodingOperation {
            operation,
            target,
            expected_state_change,
            prepared,
            plan_sha256,
        })
    }
}

fn target_for(
    write_scope: &crate::coding_changes::CodingWriteScope,
    projection: &CodingRepositoryProjection,
    prepared: &PreparedNativeCodingCall,
) -> Result<(OperationBinding, NativeCodingTargetPlan, StateChange), NativeCodingOperationError> {
    match prepared {
        PreparedNativeCodingCall::ReadOnly { kind, request } => {
            let selected = projection
                .select(*kind, request)
                .map_err(|_| NativeCodingOperationError::ProjectionDenied)?;
            Ok((
                OperationBinding::new(GrantOperation::WorkspaceRead),
                NativeCodingTargetPlan::ReadProjection {
                    objects: selected.objects,
                    projection_sha256: selected.projection_sha256,
                },
                StateChange::NotChanged,
            ))
        }
        _ => non_read_target(write_scope, prepared),
    }
}

fn non_read_target(
    write_scope: &crate::coding_changes::CodingWriteScope,
    prepared: &PreparedNativeCodingCall,
) -> Result<(OperationBinding, NativeCodingTargetPlan, StateChange), NativeCodingOperationError> {
    match prepared {
        PreparedNativeCodingCall::GitInspection { .. } => Ok((
            OperationBinding::new(GrantOperation::WorkspaceRead),
            NativeCodingTargetPlan::OwnedWorktreeRoot,
            StateChange::NotChanged,
        )),
        PreparedNativeCodingCall::StructuredPatch { proposal } => {
            let path = write_scope
                .resolve(&proposal.path)
                .map_err(|_| NativeCodingOperationError::InvariantDenied)?;
            Ok((
                OperationBinding::new(GrantOperation::WorkspaceWrite),
                NativeCodingTargetPlan::ExistingFile {
                    path,
                    expected_preimage_sha256: proposal.expected_preimage_sha256.clone(),
                },
                StateChange::Changed,
            ))
        }
        PreparedNativeCodingCall::ControlledCreate { proposal } => {
            let destination = write_scope
                .resolve(&proposal.path)
                .map_err(|_| NativeCodingOperationError::InvariantDenied)?;
            let parent = if proposal.path.len() == 1 {
                None
            } else {
                Some(
                    WorkspacePath::new(
                        destination.workspace_id().clone(),
                        proposal.path[..proposal.path.len() - 1].iter().cloned(),
                    )
                    .map_err(|_| NativeCodingOperationError::InvariantDenied)?,
                )
            };
            Ok((
                OperationBinding::new(GrantOperation::WorkspaceWrite),
                NativeCodingTargetPlan::DestinationParent {
                    parent,
                    destination,
                    expected_parent_sha256: proposal.expected_parent_sha256.clone(),
                },
                StateChange::Changed,
            ))
        }
        PreparedNativeCodingCall::Command { .. } | PreparedNativeCodingCall::Validation { .. } => {
            Ok((
                OperationBinding::new(GrantOperation::CommandExecute),
                NativeCodingTargetPlan::OwnedWorktreeRoot,
                StateChange::NotChanged,
            ))
        }
        PreparedNativeCodingCall::ReadOnly { .. } => {
            Err(NativeCodingOperationError::ProjectionDenied)
        }
    }
}

#[derive(Serialize)]
struct PlanMaterial<'plan> {
    profile_sha256: &'plan str,
    call: &'plan ToolCall,
    operation: OperationBinding,
    target: &'plan NativeCodingTargetPlan,
    expected_state_change: StateChange,
}

fn plan_sha256(
    profile_sha256: &str,
    call: &ToolCall,
    operation: OperationBinding,
    target: &NativeCodingTargetPlan,
    expected_state_change: StateChange,
) -> Result<String, NativeCodingOperationError> {
    if !is_sha256(profile_sha256) {
        return Err(NativeCodingOperationError::InvariantDenied);
    }
    let bytes = serde_json::to_vec(&PlanMaterial {
        profile_sha256,
        call,
        operation,
        target,
        expected_state_change,
    })
    .map_err(|_| NativeCodingOperationError::InvariantDenied)?;
    let mut output = String::with_capacity(64);
    for byte in Sha256::digest(bytes) {
        write!(&mut output, "{byte:02x}").expect("writing to String cannot fail");
    }
    Ok(output)
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}

#[cfg(test)]
mod tests {
    use agentmage_capability_read_only::{
        GitInspectionOperation, GitInspectionRequest, plan_git_inspection,
    };
    use agentmage_capability_repository_map::{
        StructuredArtifactClass, StructuredEdit, StructuredLanguage,
    };
    use agentmage_kernel_contracts::{
        ActionId, CONTRACT_SCHEMA_VERSION, ContractPayload, CorrelationId, SchemaId,
        SchemaReference, ToolCallId, ToolId, WorkspaceId,
    };
    use agentmage_kernel_engine::command_runner::{
        CommandBounds, CommandRisk, CommandSpec, CommandWorkingDirectory, prepare_command,
    };

    use super::*;
    use crate::{
        coding_changes::{
            CodingWriteScope, ControlledFileClassification, ControlledFileCreationProposal,
            StructuredPatchProposal,
        },
        coding_tools::TargetedValidationRequest,
    };

    fn scope() -> CodingWriteScope {
        CodingWriteScope::new(
            WorkspaceId::from_raw("workspace-coding-operation"),
            vec![Vec::new()],
        )
        .expect("root write scope")
    }

    fn prepared_command() -> agentmage_kernel_engine::command_runner::PreparedCommand {
        let spec = CommandSpec::seal(
            "fixture.check",
            "1.0.0",
            "/usr/bin/true",
            "a".repeat(64),
            Vec::new(),
            CommandWorkingDirectory::OwnedWorktree,
            std::collections::BTreeMap::from([
                ("LANG".to_owned(), "C".to_owned()),
                ("TZ".to_owned(), "UTC".to_owned()),
            ]),
            CommandRisk::Low,
            CommandBounds::new(1_000, 1_024, 1_024, 32 * 1024 * 1024, 4, 100).expect("bounds"),
        )
        .expect("command spec");
        let registry =
            agentmage_kernel_engine::command_runner::CommandRegistry::build(vec![spec.clone()])
                .expect("command registry");
        prepare_command(
            &registry,
            agentmage_kernel_engine::command_runner::CommandRequest::new("attempt-1", &spec),
        )
        .expect("prepared command")
    }

    fn call() -> ToolCall {
        ToolCall {
            schema_version: CONTRACT_SCHEMA_VERSION,
            tool_call_id: ToolCallId::from_raw("call-coding-operation"),
            correlation_id: CorrelationId::from_raw("correlation-coding-operation"),
            action_id: ActionId::from_raw("action-coding-operation"),
            tool_id: ToolId::from_raw("fixture.tool"),
            tool_version: "1.0.0".to_owned(),
            arguments: ContractPayload {
                schema: SchemaReference {
                    schema_id: SchemaId::from_raw("fixture.input"),
                    schema_version: 1,
                    schema_sha256: "b".repeat(64),
                },
                media_type: "application/json".to_owned(),
                sha256: "c".repeat(64),
                bytes: b"{}".to_vec(),
            },
        }
    }

    #[test]
    fn story_48_2_target_plans_cover_every_native_effect_family() {
        let scope = scope();
        let patch = PreparedNativeCodingCall::StructuredPatch {
            proposal: StructuredPatchProposal {
                schema_version: 1,
                change_id: "change-1".to_owned(),
                path: vec!["src".to_owned(), "lib.rs".to_owned()],
                expected_preimage_sha256: "d".repeat(64),
                intent_sha256: "e".repeat(64),
                change_plan_sha256: "f".repeat(64),
                language: StructuredLanguage::Rust,
                artifact_class: StructuredArtifactClass::Code,
                edits: vec![StructuredEdit::ReplaceExactText {
                    edit_id: "edit-1".to_owned(),
                    expected: "fn".to_owned(),
                    replacement: "pub fn".to_owned(),
                }],
                additional_review_hooks: Vec::new(),
                generated: false,
                allow_generated: false,
            },
        };
        let create = PreparedNativeCodingCall::ControlledCreate {
            proposal: ControlledFileCreationProposal {
                schema_version: 1,
                creation_id: "create-1".to_owned(),
                path: vec!["README.md".to_owned()],
                content: "bounded\n".to_owned(),
                mode: 0o644,
                classification: ControlledFileClassification::Documentation,
                intent_sha256: "1".repeat(64),
                change_plan_sha256: "2".repeat(64),
                expected_parent_sha256: "3".repeat(64),
            },
        };
        let command_call = PreparedNativeCodingCall::Command {
            prepared: Box::new(prepared_command()),
        };
        let validation_template =
            agentmage_kernel_engine::validation_template::seal_validation_template(
                agentmage_kernel_engine::validation_template::ValidationTemplateInput {
                    validation_id: "validation-unit".to_owned(),
                    kind: agentmage_kernel_engine::validation_template::ValidationKind::Unit,
                    command: prepared_command().command().clone(),
                    source: agentmage_kernel_engine::validation_template::ValidationTemplateSource::TrustedProjectConfiguration,
                    source_sha256: "5".repeat(64),
                    source_path: Some(
                        WorkspacePath::new(
                            WorkspaceId::from_raw("workspace-coding-operation"),
                            ["Cargo.toml"],
                        )
                        .expect("validation source path"),
                    ),
                    user_input_approval_sha256: None,
                    execution_scope_sha256: "6".repeat(64),
                    parser: agentmage_kernel_engine::validation_template::ValidationParserKind::AgentMageJsonV1,
                    parser_version: "1.0.0".to_owned(),
                    parser_sha256: "7".repeat(64),
                    minimum_test_count: 1,
                    focused: true,
                    fail_fast: true,
                    failed_test_rerun_allowed: true,
                    setup_idempotent: true,
                    expected_artifacts: Vec::new(),
                },
            )
            .expect("validation template");
        let validation = PreparedNativeCodingCall::Validation {
            request: TargetedValidationRequest {
                schema_version: 1,
                validation_attempt_id: "validation-attempt".to_owned(),
                validation_id: "validation-unit".to_owned(),
                template_sha256: validation_template.template_sha256.clone(),
            },
            template: Box::new(validation_template),
            prepared: Box::new(prepared_command()),
        };
        let git_request = GitInspectionRequest {
            schema_version: 1,
            operation: GitInspectionOperation::Status,
            revision: None,
            object_id: None,
            pathspecs: Vec::new(),
            max_records: 16,
            max_output_bytes: 4_096,
        };
        let git = PreparedNativeCodingCall::GitInspection {
            plan: plan_git_inspection(&git_request).expect("git plan"),
            request: git_request,
        };

        let existing = non_read_target(&scope, &patch).expect("patch target");
        assert!(matches!(
            existing,
            (
                _,
                NativeCodingTargetPlan::ExistingFile { .. },
                StateChange::Changed
            )
        ));
        let destination = non_read_target(&scope, &create).expect("create target");
        assert!(matches!(
            destination,
            (
                _,
                NativeCodingTargetPlan::DestinationParent { parent: None, .. },
                StateChange::Changed
            )
        ));
        for prepared in [&git, &command_call, &validation] {
            assert_eq!(
                non_read_target(&scope, prepared).expect("root target"),
                (
                    if matches!(prepared, PreparedNativeCodingCall::GitInspection { .. }) {
                        OperationBinding::new(GrantOperation::WorkspaceRead)
                    } else {
                        OperationBinding::new(GrantOperation::CommandExecute)
                    },
                    NativeCodingTargetPlan::OwnedWorktreeRoot,
                    StateChange::NotChanged,
                )
            );
        }
    }

    #[test]
    fn story_48_2_operation_digest_binds_profile_call_target_and_effect() {
        let call = call();
        let target = NativeCodingTargetPlan::OwnedWorktreeRoot;
        let baseline = plan_sha256(
            &"a".repeat(64),
            &call,
            OperationBinding::new(GrantOperation::CommandExecute),
            &target,
            StateChange::NotChanged,
        )
        .expect("baseline digest");
        assert_ne!(
            baseline,
            plan_sha256(
                &"b".repeat(64),
                &call,
                OperationBinding::new(GrantOperation::CommandExecute),
                &target,
                StateChange::NotChanged,
            )
            .expect("profile digest")
        );
        assert_ne!(
            baseline,
            plan_sha256(
                &"a".repeat(64),
                &call,
                OperationBinding::new(GrantOperation::WorkspaceRead),
                &target,
                StateChange::NotChanged,
            )
            .expect("operation digest")
        );
    }
}
