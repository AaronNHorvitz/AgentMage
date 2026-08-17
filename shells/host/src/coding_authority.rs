//! Exact non-authoritative approval composition for native coding operations.

use agentmage_kernel_contracts::{
    ActionKind, ApprovalId, ApprovalRequest, CapabilityGrant, GrantClass, GrantId, GrantOperation,
    GrantPreimage, GrantSideEffect, GrantStatus, GrantTarget, OperationBinding, ToolCall,
    to_canonical_json,
};
use agentmage_kernel_engine::{approval::render_approval_request, tooling::ToolRegistry};
use sha2::{Digest, Sha256};

const MAX_OPERATION_TARGETS: usize = 128;

/// Stable content-free refusal while composing one coding approval display.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CodingApprovalError {
    /// The current session parent is malformed, unavailable, or expired.
    ParentDenied,
    /// The proposed target set broadens or conflicts with the parent scope.
    TargetDenied,
    /// The proposed identity, lifetime, operation, or plan digest is malformed.
    ProposalDenied,
    /// The common approval renderer rejected the exact registered call.
    RenderingDenied,
}

impl CodingApprovalError {
    /// Returns one stable content-free diagnostic code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::ParentDenied => "runtime.coding-approval.parent-denied",
            Self::TargetDenied => "runtime.coding-approval.target-denied",
            Self::ProposalDenied => "runtime.coding-approval.proposal-denied",
            Self::RenderingDenied => "runtime.coding-approval.rendering-denied",
        }
    }
}

/// Inputs selected by trusted runtime state for one protected coding preview.
pub struct CodingApprovalRequest<'request> {
    /// Current exact session-scoped parent grant.
    pub parent: &'request CapabilityGrant,
    /// New stable approval identity.
    pub approval_id: ApprovalId,
    /// New operation-grant identity proposed for this approval only.
    pub proposed_grant_id: GrantId,
    /// Exact validated model-proposed call.
    pub call: &'request ToolCall,
    /// Exact operation frozen by the native tool definition and operation planner.
    pub operation: OperationBinding,
    /// Exact descriptor-backed operation targets in canonical order.
    pub targets: Vec<GrantTarget>,
    /// Digest binding the immutable coding profile, call, target plan, and effect class.
    pub operation_plan_sha256: &'request str,
    /// Trusted preview observation time.
    pub issued_at_epoch_ms: u64,
    /// Exclusive proposed operation-grant expiration.
    pub expires_at_epoch_ms: u64,
}

/// Renders one exact coding operation through the existing protected approval contract.
pub fn render_coding_approval_request(
    registry: &ToolRegistry,
    request: CodingApprovalRequest<'_>,
) -> Result<ApprovalRequest, CodingApprovalError> {
    validate_parent(request.parent, request.issued_at_epoch_ms)?;
    if request.targets.is_empty()
        || request.targets.len() > MAX_OPERATION_TARGETS
        || !is_sha256(request.operation_plan_sha256)
        || request.expires_at_epoch_ms <= request.issued_at_epoch_ms
        || request.expires_at_epoch_ms > request.parent.expires_at_epoch_ms
    {
        return Err(CodingApprovalError::ProposalDenied);
    }
    if request.targets.iter().any(|target| {
        !target.is_operation_target()
            || !request
                .parent
                .targets
                .iter()
                .any(|scope| scope.contains(target))
            || request
                .parent
                .excluded_targets
                .iter()
                .any(|excluded| excluded.overlaps_operation(target))
    }) {
        return Err(CodingApprovalError::TargetDenied);
    }

    let mut preimages = Vec::new();
    for (index, target) in request.targets.iter().enumerate() {
        if target.preimage().is_some() {
            let index = u32::try_from(index).map_err(|_| CodingApprovalError::TargetDenied)?;
            preimages.push(
                GrantPreimage::for_target(index, target)
                    .ok_or(CodingApprovalError::TargetDenied)?,
            );
        }
    }
    let target_indexes = (0..request.targets.len())
        .map(|index| u32::try_from(index).map_err(|_| CodingApprovalError::TargetDenied))
        .collect::<Result<Vec<_>, _>>()?;
    let parent_grant_sha256 = canonical_sha256(request.parent)?;
    let rollback_description = match request.operation.operation() {
        GrantOperation::WorkspaceWrite => {
            "Restore the exact prior bytes or remove the new file through a separate approved operation"
        }
        _ => "No workspace state change is permitted",
    }
    .to_owned();

    render_approval_request(
        registry,
        ApprovalRequest {
            schema_version: agentmage_kernel_contracts::CONTRACT_SCHEMA_VERSION,
            approval_id: request.approval_id,
            proposed_grant_id: request.proposed_grant_id,
            parent_grant_id: request.parent.grant_id.clone(),
            parent_grant_sha256,
            actor_id: request.parent.actor_id.clone(),
            session_id: request.parent.session_id.clone(),
            task_id: request.parent.task_id.clone(),
            action_kind: ActionKind::DeterministicTool,
            operation: request.operation,
            tool_call: request.call.clone(),
            targets: request.targets,
            excluded_targets: request.parent.excluded_targets.clone(),
            sensitivity: request.parent.sensitivity,
            preimages,
            expected_side_effects: vec![GrantSideEffect {
                operation: request.operation,
                target_indexes,
                details_sha256: request.operation_plan_sha256.to_owned(),
            }],
            rollback_description,
            issued_at_epoch_ms: request.issued_at_epoch_ms,
            expires_at_epoch_ms: request.expires_at_epoch_ms,
            policy_sha256: request.parent.policy_sha256.clone(),
            confirmation_sha256: "0".repeat(64),
        },
    )
    .map_err(|_| CodingApprovalError::RenderingDenied)
}

fn validate_parent(parent: &CapabilityGrant, now_epoch_ms: u64) -> Result<(), CodingApprovalError> {
    if parent.grant_class != GrantClass::SessionRead
        || parent.status != GrantStatus::Issued
        || parent.operation.operation() != GrantOperation::WorkspaceRead
        || parent.use_count >= parent.use_limit
        || now_epoch_ms < parent.issued_at_epoch_ms
        || now_epoch_ms >= parent.expires_at_epoch_ms
        || !is_sha256(&parent.policy_sha256)
    {
        return Err(CodingApprovalError::ParentDenied);
    }
    Ok(())
}

fn canonical_sha256(value: &CapabilityGrant) -> Result<String, CodingApprovalError> {
    to_canonical_json(value)
        .map(|bytes| sha256(&bytes))
        .map_err(|_| CodingApprovalError::ParentDenied)
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}

fn sha256(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[cfg(test)]
mod tests {
    use agentmage_capability_read_only::{
        ReadOnlyEncoding, ReadOnlyLimits, ReadOnlyRequest, ReadOnlyToolKind,
    };
    use agentmage_kernel_contracts::{
        ActionId, ActorId, AdapterInstanceId, AuthorizedWorkspaceHandle, ContractPayload,
        CorrelationId, DataSensitivity, FilePreimage, GrantNonce, PathPlatform, SessionId, TaskId,
        ToolCallId, WorkspaceAuthorizationId, WorkspaceId, WorkspaceObjectIdentity, WorkspacePath,
        WorkspaceScopePath,
    };
    use agentmage_kernel_engine::{
        approval::verify_approval_request,
        grants::{GrantIssuer, SessionReadGrantRequest},
    };

    use super::*;
    use crate::runtime_tools::read_only_runtime_registry;

    #[derive(Debug)]
    struct Workspace;

    impl agentmage_kernel_contracts::AuthorizedWorkspaceHandle for Workspace {
        fn workspace_id(&self) -> &WorkspaceId {
            static ID: std::sync::OnceLock<WorkspaceId> = std::sync::OnceLock::new();
            ID.get_or_init(|| WorkspaceId::from_raw("workspace-coding-approval"))
        }

        fn authorization_id(&self) -> &WorkspaceAuthorizationId {
            static ID: std::sync::OnceLock<WorkspaceAuthorizationId> = std::sync::OnceLock::new();
            ID.get_or_init(|| WorkspaceAuthorizationId::from_raw("authorization-coding-approval"))
        }

        fn adapter_instance_id(&self) -> &AdapterInstanceId {
            static ID: std::sync::OnceLock<AdapterInstanceId> = std::sync::OnceLock::new();
            ID.get_or_init(|| AdapterInstanceId::from_raw("adapter-coding-approval"))
        }

        fn platform(&self) -> PathPlatform {
            PathPlatform::DeterministicFake
        }
    }

    #[derive(Debug)]
    struct Held {
        path: WorkspacePath,
        preimage: FilePreimage,
        identity: WorkspaceObjectIdentity,
    }

    impl agentmage_kernel_contracts::HeldWorkspaceObject for Held {
        fn workspace_path(&self) -> &WorkspacePath {
            &self.path
        }

        fn authorization_id(&self) -> &WorkspaceAuthorizationId {
            Workspace.authorization_id()
        }

        fn adapter_instance_id(&self) -> &AdapterInstanceId {
            Workspace.adapter_instance_id()
        }

        fn intent(&self) -> agentmage_kernel_contracts::PathResolutionIntent {
            agentmage_kernel_contracts::PathResolutionIntent::ReadFile
        }

        fn object_kind(&self) -> agentmage_kernel_contracts::WorkspaceObjectKind {
            agentmage_kernel_contracts::WorkspaceObjectKind::RegularFile
        }

        fn object_identity(&self) -> &WorkspaceObjectIdentity {
            &self.identity
        }

        fn preimage(&self) -> Option<&FilePreimage> {
            Some(&self.preimage)
        }
    }

    #[test]
    fn story_48_2_coding_approval_binds_parent_call_targets_preimages_and_plan() {
        use agentmage_kernel_contracts::AuthorizedWorkspaceHandle as _;

        let registry = read_only_runtime_registry().expect("registry");
        let definition = registry
            .get_tool(
                &agentmage_kernel_contracts::ToolId::from_raw(ReadOnlyToolKind::ReadText.id()),
                "1.0.0",
            )
            .expect("definition");
        let request = ReadOnlyRequest {
            schema_version: 1,
            paths: vec![vec!["src".to_owned(), "lib.rs".to_owned()]],
            query: None,
            byte_offset: Some(0),
            byte_count: Some(128),
            encoding: ReadOnlyEncoding::Utf8,
            limits: ReadOnlyLimits::default(),
            call_depth: 0,
        };
        let bytes = serde_json::to_vec(&request).expect("request bytes");
        let call = ToolCall {
            schema_version: agentmage_kernel_contracts::CONTRACT_SCHEMA_VERSION,
            tool_call_id: ToolCallId::from_raw("call-coding-approval"),
            correlation_id: CorrelationId::from_raw("correlation-coding-approval"),
            action_id: ActionId::from_raw("action-coding-approval"),
            tool_id: definition.tool_id.clone(),
            tool_version: definition.tool_version.clone(),
            arguments: ContractPayload {
                schema: definition.input_schema.clone(),
                media_type: "application/json".to_owned(),
                sha256: sha256(&bytes),
                bytes,
            },
        };
        let scope = GrantTarget::workspace_scope(
            &Workspace,
            WorkspaceScopePath::new(Workspace.workspace_id().clone(), ["src"]).expect("scope path"),
        )
        .expect("scope target");
        let mut issuer = GrantIssuer::new();
        let parent = issuer
            .issue_session_read(SessionReadGrantRequest {
                grant_id: GrantId::from_raw("grant-coding-parent"),
                actor_id: ActorId::from_raw("actor-coding"),
                session_id: SessionId::from_raw("session-coding"),
                task_id: TaskId::from_raw("task-coding"),
                targets: vec![scope],
                excluded_targets: Vec::new(),
                sensitivity: DataSensitivity::Restricted,
                issued_at_epoch_ms: 1_000,
                expires_at_epoch_ms: 20_000,
                nonce: GrantNonce::from_raw("nonce-coding-parent"),
                maximum_derived_operations: 4,
                preview_sha256: "a".repeat(64),
                policy_sha256: "b".repeat(64),
            })
            .expect("parent grant");
        let content = b"pub fn run() {}\n";
        let digest: [u8; 32] = Sha256::digest(content).into();
        let target = GrantTarget::held_object(&Held {
            path: WorkspacePath::new(Workspace.workspace_id().clone(), ["src", "lib.rs"])
                .expect("held path"),
            preimage: FilePreimage::new(content.len() as u64, digest),
            identity: WorkspaceObjectIdentity::new(
                PathPlatform::DeterministicFake,
                [1; 32],
                [2; 32],
            ),
        })
        .expect("held target");
        let approval = render_coding_approval_request(
            &registry,
            CodingApprovalRequest {
                parent: &parent,
                approval_id: ApprovalId::from_raw("approval-coding"),
                proposed_grant_id: GrantId::from_raw("grant-coding-operation"),
                call: &call,
                operation: OperationBinding::new(GrantOperation::WorkspaceRead),
                targets: vec![target.clone()],
                operation_plan_sha256: &"c".repeat(64),
                issued_at_epoch_ms: 2_000,
                expires_at_epoch_ms: 3_000,
            },
        )
        .expect("approval renders");

        verify_approval_request(&registry, &approval).expect("approval verifies");
        assert_eq!(approval.targets, [target]);
        assert_eq!(approval.preimages.len(), 1);
        assert_eq!(
            approval.expected_side_effects[0].details_sha256,
            "c".repeat(64)
        );
        assert_eq!(approval.parent_grant_id, parent.grant_id);
    }

    #[test]
    fn story_48_2_coding_approval_rejects_targets_outside_the_parent_scope() {
        use agentmage_kernel_contracts::AuthorizedWorkspaceHandle as _;

        let scope = GrantTarget::workspace_scope(
            &Workspace,
            WorkspaceScopePath::new(Workspace.workspace_id().clone(), ["src"]).expect("scope path"),
        )
        .expect("scope target");
        let mut issuer = GrantIssuer::new();
        let parent = issuer
            .issue_session_read(SessionReadGrantRequest {
                grant_id: GrantId::from_raw("grant-coding-parent"),
                actor_id: ActorId::from_raw("actor-coding"),
                session_id: SessionId::from_raw("session-coding"),
                task_id: TaskId::from_raw("task-coding"),
                targets: vec![scope],
                excluded_targets: Vec::new(),
                sensitivity: DataSensitivity::Restricted,
                issued_at_epoch_ms: 1_000,
                expires_at_epoch_ms: 20_000,
                nonce: GrantNonce::from_raw("nonce-coding-parent"),
                maximum_derived_operations: 4,
                preview_sha256: "a".repeat(64),
                policy_sha256: "b".repeat(64),
            })
            .expect("parent grant");
        let target = GrantTarget::held_object(&Held {
            path: WorkspacePath::new(Workspace.workspace_id().clone(), ["tests", "outside.rs"])
                .expect("held path"),
            preimage: FilePreimage::new(1, [3; 32]),
            identity: WorkspaceObjectIdentity::new(
                PathPlatform::DeterministicFake,
                [1; 32],
                [2; 32],
            ),
        })
        .expect("held target");
        let registry = read_only_runtime_registry().expect("registry");
        let definition = registry
            .get_tool(
                &agentmage_kernel_contracts::ToolId::from_raw(ReadOnlyToolKind::ReadText.id()),
                "1.0.0",
            )
            .expect("definition");
        let bytes = serde_json::to_vec(&ReadOnlyRequest {
            schema_version: 1,
            paths: vec![vec!["tests".to_owned(), "outside.rs".to_owned()]],
            query: None,
            byte_offset: Some(0),
            byte_count: Some(1),
            encoding: ReadOnlyEncoding::Utf8,
            limits: ReadOnlyLimits::default(),
            call_depth: 0,
        })
        .expect("request bytes");
        let call = ToolCall {
            schema_version: agentmage_kernel_contracts::CONTRACT_SCHEMA_VERSION,
            tool_call_id: ToolCallId::from_raw("call-coding-approval"),
            correlation_id: CorrelationId::from_raw("correlation-coding-approval"),
            action_id: ActionId::from_raw("action-coding-approval"),
            tool_id: definition.tool_id.clone(),
            tool_version: definition.tool_version.clone(),
            arguments: ContractPayload {
                schema: definition.input_schema.clone(),
                media_type: "application/json".to_owned(),
                sha256: sha256(&bytes),
                bytes,
            },
        };

        assert_eq!(
            render_coding_approval_request(
                &registry,
                CodingApprovalRequest {
                    parent: &parent,
                    approval_id: ApprovalId::from_raw("approval-coding"),
                    proposed_grant_id: GrantId::from_raw("grant-coding-operation"),
                    call: &call,
                    operation: OperationBinding::new(GrantOperation::WorkspaceRead),
                    targets: vec![target],
                    operation_plan_sha256: &"c".repeat(64),
                    issued_at_epoch_ms: 2_000,
                    expires_at_epoch_ms: 3_000,
                },
            ),
            Err(CodingApprovalError::TargetDenied)
        );
    }
}
