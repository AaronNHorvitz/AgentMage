//! Linux approval, authority, and effect boundary for native coding runtime calls.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use agentmage_capability_read_only::{ReadOnlyOutcome, ReadOnlyResult};
use agentmage_kernel_contracts::{
    ActorId, ApprovalId, ApprovalRequest, AuthorityTransactionId, AuthorizedWorkspaceHandle,
    ContractPayload, DataSensitivity, EvidenceId, EvidenceKind, EvidenceReference, GrantId,
    GrantNonce, OperationAttemptId, OperationOutcome, RuntimeApprovalChallenge,
    RuntimeApprovalDisposition, RuntimeApprovalResponse, RuntimeOperationId, RuntimeRunRequest,
    RuntimeSessionMode, SessionId, StateChange, ToolCall, ToolDefinition, ToolResult,
    to_canonical_json,
};
use agentmage_kernel_engine::{
    authority_transaction::AuthorityTransactionRequest,
    grants::SessionReadGrantRequest,
    policy::PolicyEvaluationContext,
    runtime_coordinator::{verify_runtime_approval_response, verify_runtime_run_request},
    runtime_loop::{
        RuntimePermissionEvaluation, RuntimePortFailure, RuntimeToolBoundary, RuntimeToolExecution,
    },
};
use agentmage_platform_linux::{
    LinuxAuthorityRuntime, LinuxReadOnlyToolEffectDriver, LinuxReadOnlyToolInput,
    LinuxSandboxRunner,
};
use rustix::rand::{GetRandomFlags, getrandom};
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::{
    coding_authority::{
        ApprovedCodingGrant, ApprovedCodingGrantRequest, CodingApprovalRequest,
        CodingRuntimePolicy, derive_approved_coding_grant, render_coding_approval_request,
    },
    coding_dispatch::PreparedNativeCodingCall,
    linux_coding::{LinuxCodingTargetBinding, LinuxCodingWorkspace, PreparedLinuxCodingOperation},
};

const PREVIEW_LIFETIME_MS: u64 = 60_000;
const MAX_PENDING_OPERATIONS: usize = 8;

/// Stable content-free failure while composing the Linux coding boundary.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LinuxCodingRuntimeError {
    /// The immutable workspace, profile, or policy bindings disagree.
    BindingDenied,
    /// A required random identity could not be obtained.
    IdentityUnavailable,
}

impl LinuxCodingRuntimeError {
    /// Returns one stable content-free diagnostic code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::BindingDenied => "runtime.linux-coding.binding-denied",
            Self::IdentityUnavailable => "runtime.linux-coding.identity-unavailable",
        }
    }
}

/// CSPRNG identity boundary, injectable for deterministic runtime tests.
pub trait CodingIdentitySource {
    /// Returns one unique bounded identifier with the requested stable prefix.
    fn next(&mut self, prefix: &str) -> Result<String, LinuxCodingRuntimeError>;
}

/// Operating-system random identity source used by the packaged Linux host.
#[derive(Debug, Default)]
pub struct OsCodingIdentitySource;

impl CodingIdentitySource for OsCodingIdentitySource {
    fn next(&mut self, prefix: &str) -> Result<String, LinuxCodingRuntimeError> {
        let mut random = [0_u8; 16];
        let count = getrandom(&mut random, GetRandomFlags::empty())
            .map_err(|_| LinuxCodingRuntimeError::IdentityUnavailable)?;
        if count != random.len() {
            return Err(LinuxCodingRuntimeError::IdentityUnavailable);
        }
        Ok(format!("{prefix}-{}", hex(&random)))
    }
}

struct PendingCodingOperation<'workspace> {
    operation_id: RuntimeOperationId,
    approval: ApprovalRequest,
    prepared: PreparedLinuxCodingOperation<'workspace>,
}

struct IssuedCodingOperation<'workspace> {
    approval: ApprovalRequest,
    approved: ApprovedCodingGrant,
    prepared: PreparedLinuxCodingOperation<'workspace>,
    resolved_at_epoch_ms: u64,
}

/// Explicit verified dependencies required to construct one Linux coding boundary.
pub struct LinuxCodingRuntimeBoundaryInput<'workspace, 'session, 'platform, I>
where
    I: CodingIdentitySource,
{
    /// Descriptor-bound owned worktree and immutable coding profile.
    pub workspace: &'workspace LinuxCodingWorkspace<'session, 'platform>,
    /// Durable authority store under its continuously held private root.
    pub authority: LinuxAuthorityRuntime,
    /// Verified offline Linux worker sandbox.
    pub sandbox: LinuxSandboxRunner,
    /// Run-stable deny-by-default policy bound into the runtime request.
    pub policy: CodingRuntimePolicy,
    /// Exact authenticated local actor.
    pub actor_id: ActorId,
    /// Exact owning local session.
    pub session_id: SessionId,
    /// Data handling label inherited by session authority.
    pub sensitivity: DataSensitivity,
    /// CSPRNG or deterministic test identity source.
    pub identities: I,
}

/// Production Linux implementation of the reusable runtime's native coding boundary.
pub struct LinuxCodingRuntimeBoundary<'workspace, 'session, 'platform, I>
where
    I: CodingIdentitySource,
{
    workspace: &'workspace LinuxCodingWorkspace<'session, 'platform>,
    authority: LinuxAuthorityRuntime,
    sandbox: Option<LinuxSandboxRunner>,
    policy: CodingRuntimePolicy,
    actor_id: ActorId,
    session_id: SessionId,
    sensitivity: DataSensitivity,
    identities: I,
    pending: BTreeMap<String, PendingCodingOperation<'workspace>>,
    issued: BTreeMap<String, IssuedCodingOperation<'workspace>>,
}

impl<'workspace, 'session, 'platform, I>
    LinuxCodingRuntimeBoundary<'workspace, 'session, 'platform, I>
where
    I: CodingIdentitySource,
{
    /// Composes already verified workspace, authority, sandbox, and policy objects.
    pub fn new(
        input: LinuxCodingRuntimeBoundaryInput<'workspace, 'session, 'platform, I>,
    ) -> Result<Self, LinuxCodingRuntimeError> {
        let LinuxCodingRuntimeBoundaryInput {
            workspace,
            authority,
            sandbox,
            policy,
            actor_id,
            session_id,
            sensitivity,
            identities,
        } = input;
        let root = workspace.workspace();
        if policy.parent_targets().len() != 1
            || policy.parent_targets().iter().any(|target| {
                target.workspace_id() != root.workspace_id()
                    || target.authorization_id() != root.authorization_id()
                    || target.adapter_instance_id() != root.adapter_instance_id()
                    || target.platform() != root.platform()
                    || !target.path_components().is_empty()
                    || target.is_operation_target()
            })
            || policy.excluded_targets().iter().any(|target| {
                target.workspace_id() != root.workspace_id()
                    || target.authorization_id() != root.authorization_id()
                    || target.adapter_instance_id() != root.adapter_instance_id()
                    || target.platform() != root.platform()
                    || target.is_operation_target()
            })
        {
            return Err(LinuxCodingRuntimeError::BindingDenied);
        }
        root.revalidate()
            .map_err(|_| LinuxCodingRuntimeError::BindingDenied)?;
        authority
            .revalidate_root()
            .map_err(|_| LinuxCodingRuntimeError::BindingDenied)?;
        Ok(Self {
            workspace,
            authority,
            sandbox: Some(sandbox),
            policy,
            actor_id,
            session_id,
            sensitivity,
            identities,
            pending: BTreeMap::new(),
            issued: BTreeMap::new(),
        })
    }

    fn evaluate_call(
        &mut self,
        request: &RuntimeRunRequest,
        operation_id: &RuntimeOperationId,
        definition: &ToolDefinition,
        call: &ToolCall,
        now_epoch_ms: u64,
    ) -> Result<RuntimePermissionEvaluation, RuntimePortFailure> {
        if self.pending.len() >= MAX_PENDING_OPERATIONS
            || !self.request_matches(request)
            || !self.definition_matches(definition, call)
            || !self.policy.admits_action(&call.action_id)
        {
            return Err(RuntimePortFailure::Invalid);
        }
        self.workspace
            .workspace()
            .revalidate()
            .map_err(|_| RuntimePortFailure::Invalid)?;
        self.authority
            .revalidate_root()
            .map_err(|_| RuntimePortFailure::Unavailable)?;
        let workspace: &'workspace LinuxCodingWorkspace<'session, 'platform> = self.workspace;
        let prepared = workspace
            .prepare(call)
            .map_err(|_| RuntimePortFailure::Invalid)?;
        let targets = operation_targets(prepared.binding())?;
        let expires_at_epoch_ms = now_epoch_ms
            .checked_add(PREVIEW_LIFETIME_MS)
            .ok_or(RuntimePortFailure::Invalid)?;
        let parent_preview_sha256 = parent_preview_sha256(
            request,
            operation_id,
            self.workspace.profile().profile_sha256(),
            prepared.operation().plan_sha256(),
            self.policy.parent_targets(),
            self.policy.excluded_targets(),
        )?;
        let parent_grant_id = GrantId::from_raw(self.next_id("grant-parent")?);
        let parent_nonce = GrantNonce::from_raw(self.next_id("nonce-parent")?);
        let parent = self
            .authority
            .authority_mut()
            .issue_session_read(SessionReadGrantRequest {
                grant_id: parent_grant_id,
                actor_id: self.actor_id.clone(),
                session_id: self.session_id.clone(),
                task_id: request.task.task_id.clone(),
                targets: self.policy.parent_targets().to_vec(),
                excluded_targets: self.policy.excluded_targets().to_vec(),
                sensitivity: self.sensitivity,
                issued_at_epoch_ms: now_epoch_ms,
                expires_at_epoch_ms,
                nonce: parent_nonce,
                maximum_derived_operations: 1,
                preview_sha256: parent_preview_sha256,
                policy_sha256: self.policy.engine().policy_sha256().to_owned(),
            })
            .map_err(|_| RuntimePortFailure::Invalid)?;
        let approval = render_coding_approval_request(
            self.workspace.profile().registry(),
            CodingApprovalRequest {
                parent: &parent,
                approval_id: ApprovalId::from_raw(self.next_id("approval")?),
                proposed_grant_id: GrantId::from_raw(self.next_id("grant-operation")?),
                call,
                operation: prepared.operation().operation(),
                targets,
                operation_plan_sha256: prepared.operation().plan_sha256(),
                issued_at_epoch_ms: now_epoch_ms,
                expires_at_epoch_ms,
            },
        )
        .map_err(|_| RuntimePortFailure::Invalid)?;
        let evaluation = RuntimePermissionEvaluation::Ask {
            approval_id: approval.approval_id.clone(),
            grant_id: approval.proposed_grant_id.clone(),
            preview_sha256: approval.confirmation_sha256.clone(),
            expires_at_epoch_ms: approval.expires_at_epoch_ms,
        };
        let key = approval.approval_id.as_str().to_owned();
        if self
            .pending
            .insert(
                key,
                PendingCodingOperation {
                    operation_id: operation_id.clone(),
                    approval,
                    prepared,
                },
            )
            .is_some()
        {
            return Err(RuntimePortFailure::Invalid);
        }
        self.authority
            .revalidate_root()
            .map_err(|_| RuntimePortFailure::Unavailable)?;
        Ok(evaluation)
    }

    fn resolve_call(
        &mut self,
        request: &RuntimeRunRequest,
        challenge: &RuntimeApprovalChallenge,
        response: &RuntimeApprovalResponse,
        definition: &ToolDefinition,
        call: &ToolCall,
        now_epoch_ms: u64,
    ) -> Result<RuntimePermissionEvaluation, RuntimePortFailure> {
        if !self.request_matches(request) || !self.definition_matches(definition, call) {
            return Err(RuntimePortFailure::Invalid);
        }
        let key = challenge.approval_id.as_str();
        let pending = self.pending.get(key).ok_or(RuntimePortFailure::Invalid)?;
        if !challenge_matches(request, challenge, pending, call)
            || verify_runtime_approval_response(challenge, response, now_epoch_ms).is_err()
        {
            return Err(RuntimePortFailure::Invalid);
        }
        let decision_sha256 = canonical_sha256(response)?;
        if response.disposition == RuntimeApprovalDisposition::Deny {
            let pending = self
                .pending
                .remove(key)
                .ok_or(RuntimePortFailure::Invalid)?;
            return Ok(RuntimePermissionEvaluation::Deny {
                approval_id: pending.approval.approval_id,
                grant_id: pending.approval.proposed_grant_id,
                preview_sha256: pending.approval.confirmation_sha256,
                expires_at_epoch_ms: pending.approval.expires_at_epoch_ms,
                decision_sha256,
                reason_code: "runtime.coding.user-denied".to_owned(),
            });
        }

        pending
            .prepared
            .revalidate()
            .map_err(|_| RuntimePortFailure::Invalid)?;
        self.authority
            .revalidate_root()
            .map_err(|_| RuntimePortFailure::Unavailable)?;
        let pending = self
            .pending
            .remove(key)
            .ok_or(RuntimePortFailure::Invalid)?;
        let operation_nonce = GrantNonce::from_raw(self.next_id("nonce-operation")?);
        let approved = derive_approved_coding_grant(ApprovedCodingGrantRequest {
            authority: self.authority.authority_mut(),
            registry: self.workspace.profile().registry(),
            policy: self.policy.engine(),
            approval: &pending.approval,
            challenge,
            response,
            nonce: operation_nonce,
            now_epoch_ms,
        })
        .map_err(|_| RuntimePortFailure::Invalid)?;
        let evaluation = RuntimePermissionEvaluation::Allow {
            approval_id: pending.approval.approval_id.clone(),
            preview_sha256: pending.approval.confirmation_sha256.clone(),
            expires_at_epoch_ms: pending.approval.expires_at_epoch_ms,
            grant_id: approved.grant.grant_id.clone(),
            decision_sha256: approved.decision_sha256.clone(),
            authority_sha256: approved.authority_sha256.clone(),
        };
        let grant_key = approved.grant.grant_id.as_str().to_owned();
        if self
            .issued
            .insert(
                grant_key,
                IssuedCodingOperation {
                    approval: pending.approval,
                    approved,
                    prepared: pending.prepared,
                    resolved_at_epoch_ms: now_epoch_ms,
                },
            )
            .is_some()
        {
            return Err(RuntimePortFailure::Invalid);
        }
        self.authority
            .revalidate_root()
            .map_err(|_| RuntimePortFailure::Unavailable)?;
        Ok(evaluation)
    }

    fn execute_call(
        &mut self,
        request: &RuntimeRunRequest,
        evaluation: &RuntimePermissionEvaluation,
        definition: &ToolDefinition,
        call: &ToolCall,
        cancellation: Option<&dyn agentmage_kernel_contracts::ModelCancellationProbe>,
    ) -> Result<RuntimeToolExecution, RuntimePortFailure> {
        let RuntimePermissionEvaluation::Allow {
            approval_id,
            preview_sha256,
            expires_at_epoch_ms,
            grant_id,
            decision_sha256,
            authority_sha256,
        } = evaluation
        else {
            return Err(RuntimePortFailure::Invalid);
        };
        if !self.request_matches(request) || !self.definition_matches(definition, call) {
            return Err(RuntimePortFailure::Invalid);
        }
        let key = grant_id.as_str();
        let issued = self.issued.get(key).ok_or(RuntimePortFailure::Invalid)?;
        if issued.approval.approval_id != *approval_id
            || issued.approval.confirmation_sha256 != *preview_sha256
            || issued.approval.expires_at_epoch_ms != *expires_at_epoch_ms
            || issued.approval.tool_call != *call
            || issued.approved.grant.grant_id != *grant_id
            || issued.approved.decision_sha256 != *decision_sha256
            || issued.approved.authority_sha256 != *authority_sha256
        {
            return Err(RuntimePortFailure::Invalid);
        }
        if cancellation
            .map(|probe| probe.observe())
            .transpose()
            .map_err(|_| RuntimePortFailure::Unavailable)?
            .flatten()
            .is_some()
        {
            return Err(RuntimePortFailure::Cancelled);
        }
        issued
            .prepared
            .revalidate()
            .map_err(|_| RuntimePortFailure::Invalid)?;
        self.authority
            .revalidate_root()
            .map_err(|_| RuntimePortFailure::Unavailable)?;
        let issued = self.issued.remove(key).ok_or(RuntimePortFailure::Invalid)?;
        self.execute_prepared_read(request, definition, call, issued)
    }

    fn execute_prepared_read(
        &mut self,
        request: &RuntimeRunRequest,
        definition: &ToolDefinition,
        call: &ToolCall,
        issued: IssuedCodingOperation<'workspace>,
    ) -> Result<RuntimeToolExecution, RuntimePortFailure> {
        let (operation, binding, write_draft, _) = issued.prepared.into_parts();
        let kind = match operation.prepared() {
            PreparedNativeCodingCall::ReadOnly { kind, .. } => *kind,
            _ => return Err(RuntimePortFailure::Unavailable),
        };
        let LinuxCodingTargetBinding::ReadProjection { held, .. } = binding else {
            return Err(RuntimePortFailure::Invalid);
        };
        if operation.expected_state_change() != StateChange::NotChanged || write_draft.is_some() {
            return Err(RuntimePortFailure::Invalid);
        }
        let worker_input = LinuxReadOnlyToolInput::seal(
            call.tool_id.as_str(),
            call.tool_version.as_str(),
            &call.arguments.bytes,
            held,
        )
        .map_err(|_| RuntimePortFailure::Invalid)?;
        let grant = &issued.approved.grant;
        let action_id = grant.action_id.clone().ok_or(RuntimePortFailure::Invalid)?;
        let action_kind = grant.action_kind.ok_or(RuntimePortFailure::Invalid)?;
        let tool_id = grant.tool_id.clone().ok_or(RuntimePortFailure::Invalid)?;
        let tool_version = grant
            .tool_version
            .clone()
            .ok_or(RuntimePortFailure::Invalid)?;
        let context = PolicyEvaluationContext {
            actor_id: grant.actor_id.clone(),
            session_id: grant.session_id.clone(),
            task_id: grant.task_id.clone(),
            action_id,
            action_kind,
            tool_id,
            tool_version,
            targets: grant.targets.clone(),
            argument_sha256: grant.argument_sha256.clone(),
            preimages: grant.preimages.clone(),
            expected_side_effects: grant.expected_side_effects.clone(),
            preview_sha256: grant.preview_sha256.clone(),
            now_epoch_ms: issued.resolved_at_epoch_ms,
            network_scope: None,
            credential_scope: None,
            publication_scope: None,
        };
        let transaction = AuthorityTransactionRequest::new(
            AuthorityTransactionId::from_raw(self.next_id("transaction")?),
            OperationAttemptId::from_raw(self.next_id("attempt")?),
            issued.approval.approval_id,
            grant.grant_id.clone(),
            call.clone(),
            context,
            issued.resolved_at_epoch_ms,
            format!("epoch-ms:{}", issued.resolved_at_epoch_ms),
        )
        .map_err(|_| RuntimePortFailure::Invalid)?;
        let runner = self.sandbox.take().ok_or(RuntimePortFailure::Unavailable)?;
        let mut driver = LinuxReadOnlyToolEffectDriver::new(runner, worker_input);
        let receipt_result = self.authority.authority_mut().execute_effect(
            self.workspace.profile().registry(),
            &issued.approved.policy,
            transaction,
            &mut driver,
        );
        let worker_result = driver.take_result();
        self.sandbox = Some(driver.into_runner());
        let receipt = receipt_result.map_err(|_| RuntimePortFailure::Uncertain)?;
        self.authority
            .revalidate_root()
            .map_err(|_| RuntimePortFailure::Uncertain)?;
        let worker_result = worker_result.ok_or(RuntimePortFailure::Uncertain)?;
        if !worker_result.success() {
            return Ok(RuntimeToolExecution {
                receipt_id: receipt.receipt_id,
                receipt_sha256: receipt.receipt_sha256,
                result: ToolResult {
                    schema_version: agentmage_kernel_contracts::CONTRACT_SCHEMA_VERSION,
                    tool_call_id: call.tool_call_id.clone(),
                    correlation_id: call.correlation_id.clone(),
                    outcome: OperationOutcome::Failed,
                    output: None,
                    validation_issues: Vec::new(),
                    evidence: Vec::new(),
                    error: None,
                    elapsed_ms: 0,
                    state_change: StateChange::NotChanged,
                },
            });
        }
        let result = serde_json::from_slice::<ReadOnlyResult>(worker_result.stdout())
            .ok()
            .filter(|result| result.verify(kind))
            .ok_or(RuntimePortFailure::Uncertain)?;
        let output_bytes = serde_json::to_vec(&result).map_err(|_| RuntimePortFailure::Invalid)?;
        if output_bytes.len() as u64 > request.limits.max_output_bytes {
            return Err(RuntimePortFailure::ResourceExhausted);
        }
        let output_sha256 = sha256(&output_bytes);
        let outcome = read_outcome(result.outcome);
        let evidence = if matches!(
            result.outcome,
            ReadOnlyOutcome::Succeeded
                | ReadOnlyOutcome::NoResult
                | ReadOnlyOutcome::Partial
                | ReadOnlyOutcome::Truncated
        ) {
            vec![EvidenceReference {
                schema_version: agentmage_kernel_contracts::CONTRACT_SCHEMA_VERSION,
                evidence_id: EvidenceId::from_raw(self.next_id("evidence")?),
                kind: EvidenceKind::ToolOutput,
                source_id: format!("native:{}@{}", call.tool_id.as_str(), call.tool_version),
                object_id: call.tool_call_id.as_str().to_owned(),
                fragment: None,
                content_sha256: result.result_sha256.clone(),
                observed_revision: Some(request.repository_snapshot_id.as_str().to_owned()),
            }]
        } else {
            Vec::new()
        };
        Ok(RuntimeToolExecution {
            receipt_id: receipt.receipt_id,
            receipt_sha256: receipt.receipt_sha256,
            result: ToolResult {
                schema_version: agentmage_kernel_contracts::CONTRACT_SCHEMA_VERSION,
                tool_call_id: call.tool_call_id.clone(),
                correlation_id: call.correlation_id.clone(),
                outcome,
                output: Some(ContractPayload {
                    schema: definition.output_schema.clone(),
                    media_type: "application/json".to_owned(),
                    bytes: output_bytes,
                    sha256: output_sha256,
                }),
                validation_issues: Vec::new(),
                evidence,
                error: None,
                elapsed_ms: 0,
                state_change: StateChange::NotChanged,
            },
        })
    }

    fn request_matches(&self, request: &RuntimeRunRequest) -> bool {
        let profile = self.workspace.profile();
        verify_runtime_run_request(request).is_ok()
            && request.mode == RuntimeSessionMode::ControlledWrite
            && request.event_cursor.is_none()
            && request.session_id == self.session_id
            && request.task.task_id.as_str() == profile.worktree().task_id
            && request.workspace_id == *profile.write_scope().workspace_id()
            && request.workspace_snapshot_sha256 == profile.worktree().record_sha256
            && request.repository_snapshot_id == *profile.repository_snapshot_id()
            && request.repository_snapshot_sha256 == profile.repository_snapshot_sha256()
            && request.model_profile == *profile.model_profile()
            && request.context_budget == profile.model_profile().context
            && request.tool_catalog_id == *profile.tool_catalog_id()
            && request.tool_catalog_sha256 == profile.tool_catalog_sha256()
            && request.visible_tools == profile.visible_tools()
            && request.policy_id == *self.policy.policy_id()
            && request.policy_sha256 == self.policy.engine().policy_sha256()
            && request.limits == *profile.limits()
            && self.policy.binds_run(
                &self.actor_id,
                &request.task.task_id,
                &request.run_id,
                request.limits.max_tool_calls,
            )
    }

    fn definition_matches(&self, definition: &ToolDefinition, call: &ToolCall) -> bool {
        self.workspace
            .profile()
            .registry()
            .get_tool(&call.tool_id, &call.tool_version)
            .is_some_and(|registered| registered == definition)
            && self
                .workspace
                .profile()
                .registry()
                .validate_arguments(call)
                .is_ok()
    }

    fn next_id(&mut self, prefix: &str) -> Result<String, RuntimePortFailure> {
        self.identities
            .next(prefix)
            .map_err(|_| RuntimePortFailure::Unavailable)
    }
}

impl<'workspace, 'session, 'platform, I> RuntimeToolBoundary
    for LinuxCodingRuntimeBoundary<'workspace, 'session, 'platform, I>
where
    I: CodingIdentitySource,
{
    fn evaluate(
        &mut self,
        request: &RuntimeRunRequest,
        operation_id: &RuntimeOperationId,
        definition: &ToolDefinition,
        call: &ToolCall,
        now_epoch_ms: u64,
    ) -> Result<RuntimePermissionEvaluation, RuntimePortFailure> {
        self.evaluate_call(request, operation_id, definition, call, now_epoch_ms)
    }

    fn resolve(
        &mut self,
        request: &RuntimeRunRequest,
        challenge: &RuntimeApprovalChallenge,
        response: &RuntimeApprovalResponse,
        definition: &ToolDefinition,
        call: &ToolCall,
        now_epoch_ms: u64,
    ) -> Result<RuntimePermissionEvaluation, RuntimePortFailure> {
        self.resolve_call(request, challenge, response, definition, call, now_epoch_ms)
    }

    fn execute(
        &mut self,
        request: &RuntimeRunRequest,
        evaluation: &RuntimePermissionEvaluation,
        definition: &ToolDefinition,
        call: &ToolCall,
        cancellation: Option<&dyn agentmage_kernel_contracts::ModelCancellationProbe>,
    ) -> Result<RuntimeToolExecution, RuntimePortFailure> {
        self.execute_call(request, evaluation, definition, call, cancellation)
    }
}

fn operation_targets(
    binding: &LinuxCodingTargetBinding,
) -> Result<Vec<agentmage_kernel_contracts::GrantTarget>, RuntimePortFailure> {
    (0..binding.target_count())
        .map(|index| {
            binding
                .target(index)
                .cloned()
                .ok_or(RuntimePortFailure::Invalid)
        })
        .collect()
}

fn challenge_matches(
    request: &RuntimeRunRequest,
    challenge: &RuntimeApprovalChallenge,
    pending: &PendingCodingOperation<'_>,
    call: &ToolCall,
) -> bool {
    challenge.run_id == request.run_id
        && challenge.task_id == request.task.task_id
        && challenge.operation_id == pending.operation_id
        && challenge.tool_call_id == call.tool_call_id
        && challenge.approval_id == pending.approval.approval_id
        && challenge.proposed_grant_id == pending.approval.proposed_grant_id
        && challenge.operation == pending.approval.operation.operation()
        && challenge.preview_sha256 == pending.approval.confirmation_sha256
        && challenge.expires_at_epoch_ms == pending.approval.expires_at_epoch_ms
        && pending.approval.tool_call == *call
}

#[derive(Serialize)]
struct ParentPreviewMaterial<'material> {
    run_id: &'material str,
    operation_id: &'material str,
    profile_sha256: &'material str,
    operation_plan_sha256: &'material str,
    parent_targets: &'material [agentmage_kernel_contracts::GrantTarget],
    excluded_targets: &'material [agentmage_kernel_contracts::GrantTarget],
}

fn parent_preview_sha256(
    request: &RuntimeRunRequest,
    operation_id: &RuntimeOperationId,
    profile_sha256: &str,
    operation_plan_sha256: &str,
    parent_targets: &[agentmage_kernel_contracts::GrantTarget],
    excluded_targets: &[agentmage_kernel_contracts::GrantTarget],
) -> Result<String, RuntimePortFailure> {
    serde_json::to_vec(&ParentPreviewMaterial {
        run_id: request.run_id.as_str(),
        operation_id: operation_id.as_str(),
        profile_sha256,
        operation_plan_sha256,
        parent_targets,
        excluded_targets,
    })
    .map(|bytes| sha256(&bytes))
    .map_err(|_| RuntimePortFailure::Invalid)
}

fn canonical_sha256<T>(value: &T) -> Result<String, RuntimePortFailure>
where
    T: agentmage_kernel_contracts::VersionedContract,
{
    to_canonical_json(value)
        .map(|bytes| sha256(&bytes))
        .map_err(|_| RuntimePortFailure::Invalid)
}

const fn read_outcome(outcome: ReadOnlyOutcome) -> OperationOutcome {
    match outcome {
        ReadOnlyOutcome::Succeeded
        | ReadOnlyOutcome::NoResult
        | ReadOnlyOutcome::Partial
        | ReadOnlyOutcome::Truncated => OperationOutcome::Succeeded,
        ReadOnlyOutcome::Denied => OperationOutcome::Denied,
        ReadOnlyOutcome::Cancelled => OperationOutcome::Cancelled,
        ReadOnlyOutcome::Malformed | ReadOnlyOutcome::Failed => OperationOutcome::Failed,
    }
}

fn sha256(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(64);
    for byte in Sha256::digest(bytes) {
        write!(&mut output, "{byte:02x}").expect("writing to String cannot fail");
    }
    output
}

fn hex(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        write!(&mut output, "{byte:02x}").expect("writing to String cannot fail");
    }
    output
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::os::unix::fs::PermissionsExt as _;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    use agentmage_capability_read_only::{
        ReadOnlyEncoding, ReadOnlyLimits, ReadOnlyRequest, ReadOnlyToolKind,
    };
    use agentmage_capability_repository_map::{
        GitTrackedState, RepositoryFileInput, RepositoryMapInput, build_repository_map,
    };
    use agentmage_kernel_contracts::{
        ActionId, AuthorityClass, BudgetLimit, BudgetResource, CONTRACT_SCHEMA_VERSION,
        CorrelationId, DataSensitivity, PlanId, RollbackPlan, RuntimeApprovalChallenge,
        RuntimeApprovalDisposition, RuntimeApprovalResponse, RuntimeOperationId, RuntimeRunId,
        RuntimeRunRequest, RuntimeSessionMode, RuntimeTurnId, SessionId, StopCondition,
        StopConditionKind, Task, TaskId, TaskStatus, ToolCall, ToolCallId, ToolId, WorkPacket,
        WorkPacketId, WorkPacketState, WorkspaceAuthorizationId, WorkspaceScopePath,
    };
    use agentmage_kernel_engine::{
        operational_store::{OperationalStoreKeyError, OperationalStoreKeyProvider},
        runtime_coordinator::{seal_runtime_approval_challenge, seal_runtime_run_request},
        runtime_loop::{RuntimePermissionEvaluation, RuntimeToolBoundary, runtime_action_id},
    };
    use agentmage_platform_linux::{
        LinuxSandboxLimits, LinuxSandboxManifest, LinuxSandboxRunner, linux_repository_path_sha256,
        open_test_linux_authority,
    };

    use super::*;
    use crate::{
        coding_authority::{CodingRuntimePolicyRequest, build_coding_runtime_policy},
        coding_session::{CodingSessionProfile, tests::input_with_worktree_path_sha256},
        linux_coding::LinuxCodingWorkspace,
    };

    static TEMP_ID: AtomicU64 = AtomicU64::new(1);

    struct TestKey([u8; 32]);

    impl OperationalStoreKeyProvider for TestKey {
        fn with_key<T>(
            &mut self,
            operation: impl FnOnce(&[u8]) -> T,
        ) -> Result<T, OperationalStoreKeyError> {
            Ok(operation(&self.0))
        }
    }

    struct TestIdentities(u64);

    impl CodingIdentitySource for TestIdentities {
        fn next(&mut self, prefix: &str) -> Result<String, LinuxCodingRuntimeError> {
            self.0 += 1;
            Ok(format!("{prefix}-{:032x}", self.0))
        }
    }

    struct Fixture {
        root: PathBuf,
        request: RuntimeRunRequest,
        definition: ToolDefinition,
        call: ToolCall,
        operation_id: RuntimeOperationId,
        boundary: LinuxCodingRuntimeBoundary<'static, 'static, 'static, TestIdentities>,
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    fn fixture() -> Fixture {
        let root = temp_root("boundary");
        let worktree_root = root.join("worktree");
        let state_root = root.join("state");
        fs::create_dir_all(worktree_root.join("src")).expect("worktree tree");
        fs::create_dir(&state_root).expect("state root");
        fs::set_permissions(&state_root, fs::Permissions::from_mode(0o700))
            .expect("private state root");
        let content = b"pub fn runtime_fixture() {}\n";
        fs::write(worktree_root.join("src/lib.rs"), content).expect("fixture source");
        let worktree_sha256 =
            linux_repository_path_sha256(&worktree_root).expect("worktree identity");
        let mut profile_input = input_with_worktree_path_sha256(worktree_sha256.clone());
        let repository_map = build_repository_map(RepositoryMapInput {
            workspace_id: profile_input.write_scope.workspace_id().clone(),
            repository_sha256: "a".repeat(64),
            worktree_sha256,
            branch: Some(profile_input.worktree.branch_ref.clone()),
            commit_id: profile_input.immutable_base_commit.clone(),
            policy_sha256: "d".repeat(64),
            freshness_sha256: "e".repeat(64),
            files: vec![RepositoryFileInput {
                path: vec!["src".to_owned(), "lib.rs".to_owned()],
                size_bytes: content.len() as u64,
                content_sha256: sha256(content),
                content: Some(content.to_vec()),
                git_state: GitTrackedState::TrackedClean,
                policy_excluded: false,
                generated: false,
                vendored: false,
            }],
        })
        .expect("repository map");
        profile_input.repository_snapshot_sha256 = repository_map.map_sha256.clone();
        let profile = Box::leak(Box::new(
            CodingSessionProfile::build(profile_input).expect("coding profile"),
        ));
        let workspace = Box::leak(Box::new(
            LinuxCodingWorkspace::bind_test(
                profile,
                repository_map,
                &worktree_root,
                WorkspaceAuthorizationId::from_raw("authorization-coding-runtime"),
                agentmage_kernel_contracts::AdapterInstanceId::from_raw("adapter-coding-runtime"),
            )
            .expect("Linux coding workspace"),
        ));
        let actor_id = ActorId::from_raw("actor-coding-runtime");
        let session_id = SessionId::from_raw("session-coding-runtime");
        let task_id = TaskId::from_raw(profile.worktree().task_id.clone());
        let run_id = RuntimeRunId::from_raw("run-coding-runtime");
        let policy = build_coding_runtime_policy(CodingRuntimePolicyRequest {
            actor_id: &actor_id,
            task_id: &task_id,
            run_id: &run_id,
            workspace: workspace.workspace(),
            registry: profile.registry(),
            maximum_tool_calls: profile.limits().max_tool_calls,
            excluded_scopes: vec![
                WorkspaceScopePath::new(profile.write_scope().workspace_id().clone(), ["private"])
                    .expect("excluded scope"),
            ],
        })
        .expect("coding runtime policy");
        let request = runtime_request(profile, &policy, run_id, session_id.clone(), task_id);
        let definition = profile
            .registry()
            .get_tool(&ToolId::from_raw(ReadOnlyToolKind::ReadText.id()), "1.0.0")
            .expect("read tool")
            .clone();
        let arguments = serde_json::to_vec(&ReadOnlyRequest {
            schema_version: 1,
            paths: vec![vec!["src".to_owned(), "lib.rs".to_owned()]],
            query: None,
            byte_offset: Some(0),
            byte_count: Some(128),
            encoding: ReadOnlyEncoding::Utf8,
            limits: ReadOnlyLimits::default(),
            call_depth: 0,
        })
        .expect("read request");
        let call = ToolCall {
            schema_version: CONTRACT_SCHEMA_VERSION,
            tool_call_id: ToolCallId::from_raw("call-coding-runtime"),
            correlation_id: CorrelationId::from_raw("correlation-coding-runtime"),
            action_id: runtime_action_id(&request.run_id, 1),
            tool_id: definition.tool_id.clone(),
            tool_version: definition.tool_version.clone(),
            arguments: ContractPayload {
                schema: definition.input_schema.clone(),
                media_type: "application/json".to_owned(),
                sha256: sha256(&arguments),
                bytes: arguments,
            },
        };
        let mut key = TestKey([51; 32]);
        let authority =
            open_test_linux_authority(&state_root, &mut key, 1).expect("test authority");
        let sandbox = sandbox();
        let boundary = LinuxCodingRuntimeBoundary::new(LinuxCodingRuntimeBoundaryInput {
            workspace,
            authority,
            sandbox,
            policy,
            actor_id,
            session_id,
            sensitivity: DataSensitivity::Operational,
            identities: TestIdentities(0),
        })
        .expect("coding runtime boundary");
        Fixture {
            root,
            request,
            definition,
            call,
            operation_id: RuntimeOperationId::from_raw("operation-coding-runtime"),
            boundary,
        }
    }

    fn runtime_request(
        profile: &CodingSessionProfile,
        policy: &CodingRuntimePolicy,
        run_id: RuntimeRunId,
        session_id: SessionId,
        task_id: TaskId,
    ) -> RuntimeRunRequest {
        seal_runtime_run_request(RuntimeRunRequest {
            schema_version: CONTRACT_SCHEMA_VERSION,
            run_id,
            session_id: session_id.clone(),
            mode: RuntimeSessionMode::ControlledWrite,
            task: Task {
                schema_version: CONTRACT_SCHEMA_VERSION,
                task_id: task_id.clone(),
                session_id,
                objective: "Inspect the exact current source".to_owned(),
                acceptance_criteria: vec!["Return grounded read evidence".to_owned()],
                constraints: vec!["No network".to_owned()],
                status: TaskStatus::Ready,
            },
            work_packet: work_packet(task_id),
            workspace_id: profile.write_scope().workspace_id().clone(),
            workspace_snapshot_sha256: profile.worktree().record_sha256.clone(),
            repository_snapshot_id: profile.repository_snapshot_id().clone(),
            repository_snapshot_sha256: profile.repository_snapshot_sha256().to_owned(),
            model_profile: profile.model_profile().clone(),
            context_budget: profile.model_profile().context.clone(),
            tool_catalog_id: profile.tool_catalog_id().clone(),
            tool_catalog_sha256: profile.tool_catalog_sha256().to_owned(),
            visible_tools: profile.visible_tools().to_vec(),
            policy_id: policy.policy_id().clone(),
            policy_sha256: policy.engine().policy_sha256().to_owned(),
            limits: profile.limits().clone(),
            event_cursor: None,
            request_sha256: "0".repeat(64),
        })
        .expect("runtime request")
    }

    fn work_packet(task_id: TaskId) -> WorkPacket {
        WorkPacket {
            schema_version: CONTRACT_SCHEMA_VERSION,
            work_packet_id: WorkPacketId::from_raw("packet-coding-runtime"),
            task_id,
            revision: 1,
            objective: "Inspect the exact current source".to_owned(),
            reason: "Prove the production coding authority boundary".to_owned(),
            owner: "fixture-user".to_owned(),
            authoritative_evidence: Vec::new(),
            mutable_files: Vec::new(),
            protected_files: vec!["src/lib.rs".to_owned()],
            expected_output: "Grounded read evidence".to_owned(),
            acceptance_checks: vec!["Return grounded read evidence".to_owned()],
            required_evidence: vec![EvidenceKind::ToolOutput],
            required_capability_class: AuthorityClass::Observe,
            budgets: vec![
                BudgetLimit {
                    resource: BudgetResource::PlanSteps,
                    limit: 3,
                },
                BudgetLimit {
                    resource: BudgetResource::ModelCalls,
                    limit: 3,
                },
                BudgetLimit {
                    resource: BudgetResource::ToolCalls,
                    limit: 2,
                },
            ],
            stop_conditions: [
                StopConditionKind::AcceptanceSatisfied,
                StopConditionKind::UserDecisionRequired,
                StopConditionKind::PolicyDenied,
                StopConditionKind::Error,
                StopConditionKind::Cancelled,
                StopConditionKind::BudgetExhausted,
                StopConditionKind::UncertainResult,
            ]
            .into_iter()
            .map(|kind| StopCondition {
                kind,
                description: format!("Stop for {kind:?}"),
            })
            .collect(),
            rollback: RollbackPlan {
                reversible: true,
                description: "No state change is permitted".to_owned(),
            },
            sensitivity: DataSensitivity::Operational,
            last_verification_date: "2026-08-17".to_owned(),
            next_action: None,
            next_review: None,
            status_reason: None,
            disposition: None,
            completion_evidence: Vec::new(),
            superseding_work: None,
            validation_issues: Vec::new(),
            plan_id: Some(PlanId::from_raw("plan-coding-runtime")),
            state: WorkPacketState::Active,
        }
    }

    fn challenge(
        fixture: &Fixture,
        evaluation: &RuntimePermissionEvaluation,
    ) -> RuntimeApprovalChallenge {
        let RuntimePermissionEvaluation::Ask {
            approval_id,
            grant_id,
            preview_sha256,
            expires_at_epoch_ms,
        } = evaluation
        else {
            panic!("expected ask")
        };
        seal_runtime_approval_challenge(RuntimeApprovalChallenge {
            schema_version: CONTRACT_SCHEMA_VERSION,
            run_id: fixture.request.run_id.clone(),
            task_id: fixture.request.task.task_id.clone(),
            turn_id: RuntimeTurnId::from_raw("turn-coding-runtime"),
            operation_id: fixture.operation_id.clone(),
            tool_call_id: fixture.call.tool_call_id.clone(),
            approval_id: approval_id.clone(),
            proposed_grant_id: grant_id.clone(),
            operation: fixture.definition.required_grant.operation.operation(),
            preview_sha256: preview_sha256.clone(),
            expires_at_epoch_ms: *expires_at_epoch_ms,
            challenge_sha256: "0".repeat(64),
        })
        .expect("approval challenge")
    }

    fn response(
        challenge: &RuntimeApprovalChallenge,
        disposition: RuntimeApprovalDisposition,
    ) -> RuntimeApprovalResponse {
        RuntimeApprovalResponse {
            schema_version: CONTRACT_SCHEMA_VERSION,
            run_id: challenge.run_id.clone(),
            approval_id: challenge.approval_id.clone(),
            disposition,
            challenge_sha256: challenge.challenge_sha256.clone(),
            grant_id: (disposition == RuntimeApprovalDisposition::Allow)
                .then(|| challenge.proposed_grant_id.clone()),
        }
    }

    #[test]
    fn story_48_2_linux_runtime_asks_then_derives_one_exact_operation_grant() {
        let mut fixture = fixture();
        let evaluation = fixture
            .boundary
            .evaluate(
                &fixture.request,
                &fixture.operation_id,
                &fixture.definition,
                &fixture.call,
                1_000,
            )
            .expect("approval preview");
        let challenge = challenge(&fixture, &evaluation);
        let allowed = fixture
            .boundary
            .resolve(
                &fixture.request,
                &challenge,
                &response(&challenge, RuntimeApprovalDisposition::Allow),
                &fixture.definition,
                &fixture.call,
                1_001,
            )
            .expect("exact allow");

        assert!(matches!(allowed, RuntimePermissionEvaluation::Allow { .. }));
        assert!(fixture.boundary.pending.is_empty());
        assert_eq!(fixture.boundary.issued.len(), 1);
        assert!(fixture.boundary.authority.authority().receipts().is_empty());
    }

    #[test]
    fn story_48_2_linux_runtime_user_denial_is_inert_and_non_replayable() {
        let mut fixture = fixture();
        let evaluation = fixture
            .boundary
            .evaluate(
                &fixture.request,
                &fixture.operation_id,
                &fixture.definition,
                &fixture.call,
                2_000,
            )
            .expect("approval preview");
        let challenge = challenge(&fixture, &evaluation);
        let denied = fixture
            .boundary
            .resolve(
                &fixture.request,
                &challenge,
                &response(&challenge, RuntimeApprovalDisposition::Deny),
                &fixture.definition,
                &fixture.call,
                2_001,
            )
            .expect("exact deny");

        assert!(matches!(denied, RuntimePermissionEvaluation::Deny { .. }));
        assert!(fixture.boundary.pending.is_empty());
        assert!(fixture.boundary.issued.is_empty());
        assert!(fixture.boundary.authority.authority().receipts().is_empty());
        assert_eq!(
            fixture.boundary.resolve(
                &fixture.request,
                &challenge,
                &response(&challenge, RuntimeApprovalDisposition::Deny),
                &fixture.definition,
                &fixture.call,
                2_002,
            ),
            Err(RuntimePortFailure::Invalid)
        );
    }

    #[test]
    fn story_48_2_linux_runtime_rejects_substitution_staleness_and_unbudgeted_actions() {
        let mut fixture = fixture();
        let evaluation = fixture
            .boundary
            .evaluate(
                &fixture.request,
                &fixture.operation_id,
                &fixture.definition,
                &fixture.call,
                3_000,
            )
            .expect("approval preview");
        let challenge = challenge(&fixture, &evaluation);
        let mut substituted = fixture.call.clone();
        substituted.tool_call_id = ToolCallId::from_raw("call-substituted");
        assert_eq!(
            fixture.boundary.resolve(
                &fixture.request,
                &challenge,
                &response(&challenge, RuntimeApprovalDisposition::Allow),
                &fixture.definition,
                &substituted,
                3_001,
            ),
            Err(RuntimePortFailure::Invalid)
        );

        fs::write(
            fixture.root.join("worktree/src/lib.rs"),
            b"pub fn changed_after_preview() {}\n",
        )
        .expect("change fixture after preview");
        assert_eq!(
            fixture.boundary.resolve(
                &fixture.request,
                &challenge,
                &response(&challenge, RuntimeApprovalDisposition::Allow),
                &fixture.definition,
                &fixture.call,
                3_002,
            ),
            Err(RuntimePortFailure::Invalid)
        );

        let mut out_of_budget = fixture.call.clone();
        out_of_budget.tool_call_id = ToolCallId::from_raw("call-out-of-budget");
        out_of_budget.action_id = ActionId::from_raw("action-out-of-budget");
        assert_eq!(
            fixture.boundary.evaluate(
                &fixture.request,
                &RuntimeOperationId::from_raw("operation-out-of-budget"),
                &fixture.definition,
                &out_of_budget,
                3_003,
            ),
            Err(RuntimePortFailure::Invalid)
        );
    }

    fn sandbox() -> LinuxSandboxRunner {
        let manifest = LinuxSandboxManifest::verify(
            "/usr/bin/systemd-run",
            "/usr/bin/bwrap",
            "/usr/bin/cat",
            &[],
        )
        .expect("test sandbox manifest");
        LinuxSandboxRunner::new(manifest, LinuxSandboxLimits::default())
            .expect("test sandbox runner")
    }

    fn temp_root(label: &str) -> PathBuf {
        let id = TEMP_ID.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "agentmage-linux-coding-runtime-{}-{label}-{id}",
            std::process::id()
        ));
        if path.exists() {
            fs::remove_dir_all(&path).expect("remove stale fixture");
        }
        fs::create_dir(&path).expect("create fixture root");
        path
    }
}
