//! Exact approval and single-use authority composition for native coding operations.

use std::collections::BTreeSet;

use agentmage_kernel_contracts::{
    ActionId, ActionKind, ActorId, ApprovalId, ApprovalRequest, AuthorizedWorkspaceHandle,
    CapabilityGrant, GrantClass, GrantId, GrantNonce, GrantOperation, GrantPreimage,
    GrantSideEffect, GrantStatus, GrantTarget, OperationBinding, PolicyId,
    RuntimeApprovalChallenge, RuntimeApprovalDisposition, RuntimeApprovalResponse, RuntimeRunId,
    TaskId, ToolCall, WorkspaceScopePath, to_canonical_json,
};
use agentmage_kernel_engine::{
    approval::{render_approval_request, verify_approval_request},
    grants::DerivedOperationGrantRequest,
    operational_store::DurableAuthorityRuntime,
    policy::{PolicyDocument, PolicyEngine, ScopeRules, ToolPolicyBinding},
    runtime_coordinator::verify_runtime_approval_response,
    runtime_loop::runtime_action_id,
    tooling::ToolRegistry,
};
use sha2::{Digest, Sha256};

const MAX_OPERATION_TARGETS: usize = 128;
const OPERATION_GRANT_LIFETIME_MS: u64 = 30_000;

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
    /// The exact operation policy could not be constructed.
    PolicyDenied,
    /// The protected client decision did not match the displayed operation.
    DecisionDenied,
    /// Durable grant derivation or canonical authority binding failed.
    AuthorityDenied,
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
            Self::PolicyDenied => "runtime.coding-approval.policy-denied",
            Self::DecisionDenied => "runtime.coding-approval.decision-denied",
            Self::AuthorityDenied => "runtime.coding-approval.authority-denied",
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

/// Inputs for converting one exact protected decision into durable single-use authority.
pub struct ApprovedCodingGrantRequest<'request> {
    /// Durable kernel authority store that owns grant issuance.
    pub authority: &'request mut DurableAuthorityRuntime,
    /// Exact immutable native tool registry used to render the preview.
    pub registry: &'request ToolRegistry,
    /// Exact run-stable policy already bound into the parent and runtime request.
    pub policy: &'request PolicyEngine,
    /// Exact protected display object retained by the trusted boundary.
    pub approval: &'request ApprovalRequest,
    /// Exact coordinator challenge derived from that display.
    pub challenge: &'request RuntimeApprovalChallenge,
    /// Exact authenticated client response to the challenge.
    pub response: &'request RuntimeApprovalResponse,
    /// Fresh kernel-selected anti-replay nonce.
    pub nonce: GrantNonce,
    /// Trusted decision instant supplied by the coordinator clock.
    pub now_epoch_ms: u64,
}

/// One exact issued operation grant and the policy required to consume it.
#[derive(Debug)]
pub struct ApprovedCodingGrant {
    /// Durable, separately issued, single-use operation grant.
    pub grant: CapabilityGrant,
    /// Exact deny-by-default policy bound into the parent and child grants.
    pub policy: PolicyEngine,
    /// Digest of the exact authenticated client response.
    pub decision_sha256: String,
    /// Digest of the exact issued operation-grant revision.
    pub authority_sha256: String,
}

/// Inputs for one run-stable, deny-by-default coding policy envelope.
pub struct CodingRuntimePolicyRequest<'request, Workspace>
where
    Workspace: AuthorizedWorkspaceHandle,
{
    /// Exact authenticated local actor.
    pub actor_id: &'request ActorId,
    /// Exact task admitted for this run.
    pub task_id: &'request TaskId,
    /// Exact runtime run whose bounded action identities are precomputed.
    pub run_id: &'request RuntimeRunId,
    /// Continuously held authorized workspace root.
    pub workspace: &'request Workspace,
    /// Exact immutable native tool registry.
    pub registry: &'request ToolRegistry,
    /// Inclusive maximum number of coordinator tool calls.
    pub maximum_tool_calls: u32,
    /// Canonical authorization-bound subtrees denied throughout the run.
    pub excluded_scopes: Vec<WorkspaceScopePath>,
}

/// One immutable run policy and the parent-grant scope it protects.
#[derive(Debug)]
pub struct CodingRuntimePolicy {
    policy_id: PolicyId,
    engine: PolicyEngine,
    actor_id: ActorId,
    task_id: TaskId,
    run_id: RuntimeRunId,
    action_ids: BTreeSet<ActionId>,
    parent_targets: Vec<GrantTarget>,
    excluded_targets: Vec<GrantTarget>,
}

impl CodingRuntimePolicy {
    /// Returns the stable policy identity exposed in the runtime request.
    #[must_use]
    pub const fn policy_id(&self) -> &PolicyId {
        &self.policy_id
    }

    /// Returns the immutable policy engine used for every operation in this run.
    #[must_use]
    pub const fn engine(&self) -> &PolicyEngine {
        &self.engine
    }

    /// Reports whether the immutable policy belongs to this exact bounded runtime run.
    #[must_use]
    pub fn binds_run(
        &self,
        actor_id: &ActorId,
        task_id: &TaskId,
        run_id: &RuntimeRunId,
        maximum_tool_calls: u32,
    ) -> bool {
        self.actor_id == *actor_id
            && self.task_id == *task_id
            && self.run_id == *run_id
            && maximum_tool_calls > 0
            && self.action_ids.len() == maximum_tool_calls as usize
            && (1..=maximum_tool_calls).all(|sequence| {
                self.action_ids
                    .contains(&runtime_action_id(run_id, sequence))
            })
    }

    /// Reports whether one coordinator-selected action is inside the fixed run budget.
    #[must_use]
    pub fn admits_action(&self, action_id: &ActionId) -> bool {
        self.action_ids.contains(action_id)
    }

    /// Returns the exact authorization-bound parent scopes.
    #[must_use]
    pub fn parent_targets(&self) -> &[GrantTarget] {
        &self.parent_targets
    }

    /// Returns canonical excluded scopes inherited by every child grant.
    #[must_use]
    pub fn excluded_targets(&self) -> &[GrantTarget] {
        &self.excluded_targets
    }
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

/// Builds one deny-by-default policy for exactly one native coding operation.
pub fn exact_coding_policy(
    actor_id: &ActorId,
    task_id: &TaskId,
    call: &ToolCall,
    operation: OperationBinding,
    targets: &[GrantTarget],
) -> Result<PolicyEngine, CodingApprovalError> {
    if !matches!(
        operation.operation(),
        GrantOperation::WorkspaceRead
            | GrantOperation::WorkspaceWrite
            | GrantOperation::CommandExecute
    ) || targets.is_empty()
        || targets.len() > MAX_OPERATION_TARGETS
        || targets.iter().any(|target| !target.is_operation_target())
        || targets.iter().collect::<BTreeSet<_>>().len() != targets.len()
    {
        return Err(CodingApprovalError::PolicyDenied);
    }
    let first = &targets[0];
    if targets.iter().any(|target| {
        target.workspace_id() != first.workspace_id()
            || target.authorization_id() != first.authorization_id()
            || target.adapter_instance_id() != first.adapter_instance_id()
            || target.platform() != first.platform()
    }) {
        return Err(CodingApprovalError::PolicyDenied);
    }

    PolicyEngine::new(PolicyDocument {
        schema_version: 1,
        revision: 1,
        actors: exact_rules(actor_id.clone()),
        tasks: exact_rules(task_id.clone()),
        actions: exact_rules(call.action_id.clone()),
        tools: exact_rules(ToolPolicyBinding {
            tool_id: call.tool_id.clone(),
            tool_version: call.tool_version.clone(),
        }),
        operations: exact_rules(operation),
        targets: ScopeRules {
            allowed: targets.iter().cloned().collect(),
            denied: BTreeSet::new(),
        },
        denied_argument_sha256s: BTreeSet::new(),
        denied_preimage_sha256s: BTreeSet::new(),
        network_scopes: ScopeRules::deny_all(),
        credential_scopes: ScopeRules::deny_all(),
        publication_scopes: ScopeRules::deny_all(),
    })
    .map_err(|_| CodingApprovalError::PolicyDenied)
}

/// Builds one stable policy envelope for every bounded action and native tool in a coding run.
pub fn build_coding_runtime_policy<Workspace>(
    request: CodingRuntimePolicyRequest<'_, Workspace>,
) -> Result<CodingRuntimePolicy, CodingApprovalError>
where
    Workspace: AuthorizedWorkspaceHandle,
{
    if request.maximum_tool_calls == 0 || request.registry.list_tools().is_empty() {
        return Err(CodingApprovalError::PolicyDenied);
    }
    let parent_scope = WorkspaceScopePath::new(
        request.workspace.workspace_id().clone(),
        std::iter::empty::<&str>(),
    )
    .map_err(|_| CodingApprovalError::PolicyDenied)?;
    let parent_target = GrantTarget::workspace_scope(request.workspace, parent_scope)
        .map_err(|_| CodingApprovalError::PolicyDenied)?;
    let mut excluded_targets = request
        .excluded_scopes
        .into_iter()
        .map(|scope| {
            GrantTarget::workspace_scope(request.workspace, scope)
                .map_err(|_| CodingApprovalError::PolicyDenied)
        })
        .collect::<Result<Vec<_>, _>>()?;
    excluded_targets.sort();
    if excluded_targets.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err(CodingApprovalError::PolicyDenied);
    }

    let definitions = request.registry.list_tools();
    let tools = definitions
        .iter()
        .map(|definition| ToolPolicyBinding {
            tool_id: definition.tool_id.clone(),
            tool_version: definition.tool_version.clone(),
        })
        .collect::<BTreeSet<_>>();
    let operations = definitions
        .iter()
        .map(|definition| definition.required_grant.operation)
        .collect::<BTreeSet<_>>();
    if tools.len() != definitions.len()
        || operations.iter().any(|operation| {
            !matches!(
                operation.operation(),
                GrantOperation::WorkspaceRead
                    | GrantOperation::WorkspaceWrite
                    | GrantOperation::CommandExecute
            )
        })
    {
        return Err(CodingApprovalError::PolicyDenied);
    }
    let actions = (1..=request.maximum_tool_calls)
        .map(|sequence| runtime_action_id(request.run_id, sequence))
        .collect::<BTreeSet<_>>();
    if actions.len() != request.maximum_tool_calls as usize {
        return Err(CodingApprovalError::PolicyDenied);
    }
    let engine = PolicyEngine::new(PolicyDocument {
        schema_version: 1,
        revision: 1,
        actors: exact_rules(request.actor_id.clone()),
        tasks: exact_rules(request.task_id.clone()),
        actions: ScopeRules {
            allowed: actions.clone(),
            denied: BTreeSet::new(),
        },
        tools: ScopeRules {
            allowed: tools,
            denied: BTreeSet::new(),
        },
        operations: ScopeRules {
            allowed: operations,
            denied: BTreeSet::new(),
        },
        targets: ScopeRules {
            allowed: BTreeSet::from([parent_target.clone()]),
            denied: excluded_targets.iter().cloned().collect(),
        },
        denied_argument_sha256s: BTreeSet::new(),
        denied_preimage_sha256s: BTreeSet::new(),
        network_scopes: ScopeRules::deny_all(),
        credential_scopes: ScopeRules::deny_all(),
        publication_scopes: ScopeRules::deny_all(),
    })
    .map_err(|_| CodingApprovalError::PolicyDenied)?;
    let policy_id = PolicyId::from_raw(format!("policy:coding:{}", &engine.policy_sha256()[..32]));
    Ok(CodingRuntimePolicy {
        policy_id,
        engine,
        actor_id: request.actor_id.clone(),
        task_id: request.task_id.clone(),
        run_id: request.run_id.clone(),
        action_ids: actions,
        parent_targets: vec![parent_target],
        excluded_targets,
    })
}

/// Verifies one protected allow decision and durably derives its exact operation grant.
pub fn derive_approved_coding_grant(
    request: ApprovedCodingGrantRequest<'_>,
) -> Result<ApprovedCodingGrant, CodingApprovalError> {
    verify_approval_request(request.registry, request.approval)
        .map_err(|_| CodingApprovalError::DecisionDenied)?;
    verify_coding_decision(
        request.approval,
        request.challenge,
        request.response,
        request.now_epoch_ms,
    )?;
    if request.policy.policy_sha256() != request.approval.policy_sha256 {
        return Err(CodingApprovalError::PolicyDenied);
    }
    let current_parent = request
        .authority
        .current_grant(&request.approval.parent_grant_id)
        .ok_or(CodingApprovalError::AuthorityDenied)?;
    if current_parent.actor_id != request.approval.actor_id
        || current_parent.session_id != request.approval.session_id
        || current_parent.task_id != request.approval.task_id
        || current_parent.policy_sha256 != request.approval.policy_sha256
        || canonical_contract_sha256(current_parent)
            .map_err(|_| CodingApprovalError::AuthorityDenied)?
            != request.approval.parent_grant_sha256
    {
        return Err(CodingApprovalError::AuthorityDenied);
    }
    let operation_expires_at_epoch_ms = request
        .now_epoch_ms
        .checked_add(OPERATION_GRANT_LIFETIME_MS)
        .map(|expires| expires.min(request.approval.expires_at_epoch_ms))
        .ok_or(CodingApprovalError::AuthorityDenied)?;
    let grant = request
        .authority
        .derive_operation(
            &request.approval.parent_grant_id,
            operation_grant_request(
                request.approval,
                request.nonce,
                request.now_epoch_ms,
                operation_expires_at_epoch_ms,
            )?,
        )
        .map_err(|_| CodingApprovalError::AuthorityDenied)?;
    let decision_sha256 = canonical_contract_sha256(request.response)
        .map_err(|_| CodingApprovalError::DecisionDenied)?;
    let authority_sha256 =
        canonical_contract_sha256(&grant).map_err(|_| CodingApprovalError::AuthorityDenied)?;
    Ok(ApprovedCodingGrant {
        grant,
        policy: request.policy.clone(),
        decision_sha256,
        authority_sha256,
    })
}

fn verify_coding_decision(
    approval: &ApprovalRequest,
    challenge: &RuntimeApprovalChallenge,
    response: &RuntimeApprovalResponse,
    now_epoch_ms: u64,
) -> Result<(), CodingApprovalError> {
    verify_runtime_approval_response(challenge, response, now_epoch_ms)
        .map_err(|_| CodingApprovalError::DecisionDenied)?;
    if response.disposition != RuntimeApprovalDisposition::Allow
        || challenge.task_id != approval.task_id
        || challenge.tool_call_id != approval.tool_call.tool_call_id
        || challenge.approval_id != approval.approval_id
        || challenge.proposed_grant_id != approval.proposed_grant_id
        || challenge.operation != approval.operation.operation()
        || challenge.preview_sha256 != approval.confirmation_sha256
        || challenge.expires_at_epoch_ms != approval.expires_at_epoch_ms
        || response.grant_id.as_ref() != Some(&approval.proposed_grant_id)
        || now_epoch_ms < approval.issued_at_epoch_ms
        || now_epoch_ms >= approval.expires_at_epoch_ms
    {
        return Err(CodingApprovalError::DecisionDenied);
    }
    Ok(())
}

fn operation_grant_request(
    approval: &ApprovalRequest,
    nonce: GrantNonce,
    issued_at_epoch_ms: u64,
    expires_at_epoch_ms: u64,
) -> Result<DerivedOperationGrantRequest, CodingApprovalError> {
    if issued_at_epoch_ms < approval.issued_at_epoch_ms
        || expires_at_epoch_ms <= issued_at_epoch_ms
        || expires_at_epoch_ms > approval.expires_at_epoch_ms
    {
        return Err(CodingApprovalError::AuthorityDenied);
    }
    Ok(DerivedOperationGrantRequest {
        grant_id: approval.proposed_grant_id.clone(),
        approval_id: approval.approval_id.clone(),
        action_id: approval.tool_call.action_id.clone(),
        action_kind: approval.action_kind,
        operation: approval.operation,
        tool_id: approval.tool_call.tool_id.clone(),
        tool_version: approval.tool_call.tool_version.clone(),
        targets: approval.targets.clone(),
        argument_sha256: approval.tool_call.arguments.sha256.clone(),
        preimages: approval.preimages.clone(),
        expected_side_effects: approval.expected_side_effects.clone(),
        rollback_description: approval.rollback_description.clone(),
        issued_at_epoch_ms,
        expires_at_epoch_ms,
        nonce,
        preview_sha256: approval.confirmation_sha256.clone(),
        policy_sha256: approval.policy_sha256.clone(),
    })
}

fn exact_rules<T>(value: T) -> ScopeRules<T>
where
    T: Ord,
{
    ScopeRules {
        allowed: BTreeSet::from([value]),
        denied: BTreeSet::new(),
    }
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

fn canonical_contract_sha256<T>(value: &T) -> Result<String, ()>
where
    T: agentmage_kernel_contracts::VersionedContract,
{
    to_canonical_json(value)
        .map(|bytes| sha256(&bytes))
        .map_err(|_| ())
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
        policy::PolicyEvaluationContext,
        runtime_coordinator::seal_runtime_approval_challenge,
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

    #[test]
    fn story_48_2_allow_decision_derives_only_the_exact_policy_bound_grant() {
        use agentmage_kernel_contracts::AuthorizedWorkspaceHandle as _;

        let registry = read_only_runtime_registry().expect("registry");
        let definition = registry
            .get_tool(
                &agentmage_kernel_contracts::ToolId::from_raw(ReadOnlyToolKind::ReadText.id()),
                "1.0.0",
            )
            .expect("definition");
        let bytes = serde_json::to_vec(&ReadOnlyRequest {
            schema_version: 1,
            paths: vec![vec!["src".to_owned(), "lib.rs".to_owned()]],
            query: None,
            byte_offset: Some(0),
            byte_count: Some(128),
            encoding: ReadOnlyEncoding::Utf8,
            limits: ReadOnlyLimits::default(),
            call_depth: 0,
        })
        .expect("request bytes");
        let call = ToolCall {
            schema_version: agentmage_kernel_contracts::CONTRACT_SCHEMA_VERSION,
            tool_call_id: ToolCallId::from_raw("call-coding-policy"),
            correlation_id: CorrelationId::from_raw("correlation-coding-policy"),
            action_id: ActionId::from_raw("action-coding-policy"),
            tool_id: definition.tool_id.clone(),
            tool_version: definition.tool_version.clone(),
            arguments: ContractPayload {
                schema: definition.input_schema.clone(),
                media_type: "application/json".to_owned(),
                sha256: sha256(&bytes),
                bytes,
            },
        };
        let content = b"pub fn run() {}\n";
        let digest: [u8; 32] = Sha256::digest(content).into();
        let target = GrantTarget::held_object(&Held {
            path: WorkspacePath::new(Workspace.workspace_id().clone(), ["src", "lib.rs"])
                .expect("held path"),
            preimage: FilePreimage::new(
                u64::try_from(content.len()).expect("content length"),
                digest,
            ),
            identity: WorkspaceObjectIdentity::new(
                PathPlatform::DeterministicFake,
                [1; 32],
                [2; 32],
            ),
        })
        .expect("held target");
        let actor_id = ActorId::from_raw("actor-coding-policy");
        let task_id = TaskId::from_raw("task-coding-policy");
        let operation = OperationBinding::new(GrantOperation::WorkspaceRead);
        let policy = exact_coding_policy(
            &actor_id,
            &task_id,
            &call,
            operation,
            std::slice::from_ref(&target),
        )
        .expect("exact policy");
        let scope = GrantTarget::workspace_scope(
            &Workspace,
            WorkspaceScopePath::new(Workspace.workspace_id().clone(), ["src"]).expect("scope path"),
        )
        .expect("scope target");
        let mut issuer = GrantIssuer::new();
        let parent = issuer
            .issue_session_read(SessionReadGrantRequest {
                grant_id: GrantId::from_raw("grant-coding-policy-parent"),
                actor_id: actor_id.clone(),
                session_id: SessionId::from_raw("session-coding-policy"),
                task_id: task_id.clone(),
                targets: vec![scope],
                excluded_targets: Vec::new(),
                sensitivity: DataSensitivity::Restricted,
                issued_at_epoch_ms: 1_000,
                expires_at_epoch_ms: 20_000,
                nonce: GrantNonce::from_raw("nonce-coding-policy-parent"),
                maximum_derived_operations: 1,
                preview_sha256: "a".repeat(64),
                policy_sha256: policy.policy_sha256().to_owned(),
            })
            .expect("parent grant");
        let approval = render_coding_approval_request(
            &registry,
            CodingApprovalRequest {
                parent: &parent,
                approval_id: ApprovalId::from_raw("approval-coding-policy"),
                proposed_grant_id: GrantId::from_raw("grant-coding-policy-operation"),
                call: &call,
                operation,
                targets: vec![target],
                operation_plan_sha256: &"c".repeat(64),
                issued_at_epoch_ms: 2_000,
                expires_at_epoch_ms: 10_000,
            },
        )
        .expect("approval");
        let challenge = seal_runtime_approval_challenge(RuntimeApprovalChallenge {
            schema_version: agentmage_kernel_contracts::CONTRACT_SCHEMA_VERSION,
            run_id: agentmage_kernel_contracts::RuntimeRunId::from_raw("run-coding-policy"),
            task_id: task_id.clone(),
            turn_id: agentmage_kernel_contracts::RuntimeTurnId::from_raw("turn-coding-policy"),
            operation_id: agentmage_kernel_contracts::RuntimeOperationId::from_raw(
                "operation-coding-policy",
            ),
            tool_call_id: call.tool_call_id.clone(),
            approval_id: approval.approval_id.clone(),
            proposed_grant_id: approval.proposed_grant_id.clone(),
            operation: GrantOperation::WorkspaceRead,
            preview_sha256: approval.confirmation_sha256.clone(),
            expires_at_epoch_ms: approval.expires_at_epoch_ms,
            challenge_sha256: "0".repeat(64),
        })
        .expect("challenge");
        let response = RuntimeApprovalResponse {
            schema_version: agentmage_kernel_contracts::CONTRACT_SCHEMA_VERSION,
            run_id: challenge.run_id.clone(),
            approval_id: challenge.approval_id.clone(),
            disposition: RuntimeApprovalDisposition::Allow,
            challenge_sha256: challenge.challenge_sha256.clone(),
            grant_id: Some(challenge.proposed_grant_id.clone()),
        };
        verify_coding_decision(&approval, &challenge, &response, 3_000).expect("exact response");
        let mut substituted = response.clone();
        substituted.grant_id = Some(GrantId::from_raw("grant-substituted"));
        assert_eq!(
            verify_coding_decision(&approval, &challenge, &substituted, 3_000),
            Err(CodingApprovalError::DecisionDenied)
        );

        let child = issuer
            .derive_operation(
                &parent.grant_id,
                operation_grant_request(
                    &approval,
                    GrantNonce::from_raw("nonce-coding-policy-operation"),
                    3_000,
                    9_000,
                )
                .expect("derived request"),
            )
            .expect("child grant");
        let context = PolicyEvaluationContext {
            actor_id,
            session_id: child.session_id.clone(),
            task_id,
            action_id: call.action_id,
            action_kind: ActionKind::DeterministicTool,
            tool_id: call.tool_id,
            tool_version: call.tool_version,
            targets: child.targets.clone(),
            argument_sha256: child.argument_sha256.clone(),
            preimages: child.preimages.clone(),
            expected_side_effects: child.expected_side_effects.clone(),
            preview_sha256: child.preview_sha256.clone(),
            now_epoch_ms: 4_000,
            network_scope: None,
            credential_scope: None,
            publication_scope: None,
        };
        assert!(policy.evaluate(&issuer, &child, &context).allowed);
        let mut broadened = context;
        broadened.targets.clear();
        assert!(!policy.evaluate(&issuer, &child, &broadened).allowed);
    }

    #[test]
    fn story_48_2_runtime_policy_is_stable_across_bounded_actions_and_denies_expansion() {
        use agentmage_kernel_contracts::AuthorizedWorkspaceHandle as _;

        let registry = read_only_runtime_registry().expect("registry");
        let run_id = RuntimeRunId::from_raw("run-coding-envelope");
        let actor_id = ActorId::from_raw("actor-coding-envelope");
        let task_id = TaskId::from_raw("task-coding-envelope");
        let runtime_policy = build_coding_runtime_policy(CodingRuntimePolicyRequest {
            actor_id: &actor_id,
            task_id: &task_id,
            run_id: &run_id,
            workspace: &Workspace,
            registry: &registry,
            maximum_tool_calls: 3,
            excluded_scopes: vec![
                WorkspaceScopePath::new(Workspace.workspace_id().clone(), ["private"])
                    .expect("excluded scope"),
            ],
        })
        .expect("runtime policy");
        assert!(
            runtime_policy
                .policy_id()
                .as_str()
                .starts_with("policy:coding:")
        );
        assert_eq!(runtime_policy.parent_targets().len(), 1);
        assert_eq!(runtime_policy.excluded_targets().len(), 1);

        let definition = registry
            .get_tool(
                &agentmage_kernel_contracts::ToolId::from_raw(ReadOnlyToolKind::ReadText.id()),
                "1.0.0",
            )
            .expect("definition");
        let bytes = br#"{"schema_version":1,"paths":[["src","lib.rs"]],"query":null,"byte_offset":0,"byte_count":1,"encoding":"utf8","limits":{"files":64,"depth":16,"total_bytes":8388608,"file_bytes":4194304,"matches":10000},"call_depth":0}"#.to_vec();
        let call = ToolCall {
            schema_version: agentmage_kernel_contracts::CONTRACT_SCHEMA_VERSION,
            tool_call_id: ToolCallId::from_raw("call-coding-envelope"),
            correlation_id: CorrelationId::from_raw("correlation-coding-envelope"),
            action_id: runtime_action_id(&run_id, 2),
            tool_id: definition.tool_id.clone(),
            tool_version: definition.tool_version.clone(),
            arguments: ContractPayload {
                schema: definition.input_schema.clone(),
                media_type: "application/json".to_owned(),
                sha256: sha256(&bytes),
                bytes,
            },
        };
        let target = GrantTarget::held_object(&Held {
            path: WorkspacePath::new(Workspace.workspace_id().clone(), ["src", "lib.rs"])
                .expect("held path"),
            preimage: FilePreimage::new(1, [7; 32]),
            identity: WorkspaceObjectIdentity::new(
                PathPlatform::DeterministicFake,
                [1; 32],
                [2; 32],
            ),
        })
        .expect("target");
        let operation = OperationBinding::new(GrantOperation::WorkspaceRead);
        let mut issuer = GrantIssuer::new();
        let parent = issuer
            .issue_session_read(SessionReadGrantRequest {
                grant_id: GrantId::from_raw("grant-coding-envelope-parent"),
                actor_id: actor_id.clone(),
                session_id: SessionId::from_raw("session-coding-envelope"),
                task_id: task_id.clone(),
                targets: runtime_policy.parent_targets().to_vec(),
                excluded_targets: runtime_policy.excluded_targets().to_vec(),
                sensitivity: DataSensitivity::Restricted,
                issued_at_epoch_ms: 1_000,
                expires_at_epoch_ms: 20_000,
                nonce: GrantNonce::from_raw("nonce-coding-envelope-parent"),
                maximum_derived_operations: 2,
                preview_sha256: "a".repeat(64),
                policy_sha256: runtime_policy.engine().policy_sha256().to_owned(),
            })
            .expect("parent");
        let preimage = GrantPreimage::for_target(0, &target).expect("preimage");
        let child = issuer
            .derive_operation(
                &parent.grant_id,
                DerivedOperationGrantRequest {
                    grant_id: GrantId::from_raw("grant-coding-envelope-child"),
                    approval_id: ApprovalId::from_raw("approval-coding-envelope"),
                    action_id: call.action_id.clone(),
                    action_kind: ActionKind::DeterministicTool,
                    operation,
                    tool_id: call.tool_id.clone(),
                    tool_version: call.tool_version.clone(),
                    targets: vec![target],
                    argument_sha256: call.arguments.sha256.clone(),
                    preimages: vec![preimage],
                    expected_side_effects: vec![GrantSideEffect {
                        operation,
                        target_indexes: vec![0],
                        details_sha256: "b".repeat(64),
                    }],
                    rollback_description: "No state change is permitted".to_owned(),
                    issued_at_epoch_ms: 2_000,
                    expires_at_epoch_ms: 10_000,
                    nonce: GrantNonce::from_raw("nonce-coding-envelope-child"),
                    preview_sha256: "c".repeat(64),
                    policy_sha256: runtime_policy.engine().policy_sha256().to_owned(),
                },
            )
            .expect("child");
        let context = PolicyEvaluationContext {
            actor_id,
            session_id: child.session_id.clone(),
            task_id,
            action_id: child.action_id.clone().expect("action"),
            action_kind: ActionKind::DeterministicTool,
            tool_id: child.tool_id.clone().expect("tool"),
            tool_version: child.tool_version.clone().expect("tool version"),
            targets: child.targets.clone(),
            argument_sha256: child.argument_sha256.clone(),
            preimages: child.preimages.clone(),
            expected_side_effects: child.expected_side_effects.clone(),
            preview_sha256: child.preview_sha256.clone(),
            now_epoch_ms: 3_000,
            network_scope: None,
            credential_scope: None,
            publication_scope: None,
        };
        assert!(
            runtime_policy
                .engine()
                .evaluate(&issuer, &child, &context)
                .allowed
        );
        let mut outside_budget = context;
        outside_budget.action_id = runtime_action_id(&run_id, 4);
        assert!(
            !runtime_policy
                .engine()
                .evaluate(&issuer, &child, &outside_budget)
                .allowed
        );
    }
}
