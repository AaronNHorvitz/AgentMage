// SPDX-License-Identifier: Apache-2.0
//! Descriptor-bound file reads for the original ephemeral endpoint coordinator.
//!
//! Trusted startup must obtain workspace selection/read consent and native
//! platform/authority/sandbox admission. Model disclosure remains separate.
//! No native model profile, source inventory or grant is inferred from wire data.

use std::{collections::BTreeSet, sync::Arc};

use agentmage_capability_read_only::{
    ReadOnlyResult, ReadOnlyToolKind, read_only_tool_definition, validate_read_only_request,
};
use agentmage_kernel_contracts::*;
use agentmage_kernel_engine::{
    authority_transaction::AuthorityTransactionRequest,
    grants::SessionReadGrantRequest,
    policy::PolicyEvaluationContext,
    propagation::{BorrowedEffectCancellation, EffectCancellationObservation},
    runtime_coordinator::verify_runtime_approval_response,
    runtime_loop::{
        RuntimePermissionEvaluation, RuntimePortFailure, RuntimeRunScope, RuntimeToolBoundary,
        RuntimeToolExecution,
    },
    tooling::ToolRegistry,
};
use agentmage_platform_linux::{
    LinuxAuthorityRuntime, LinuxAuthorizedWorkspace, LinuxHeldObject, LinuxPathAdapter,
    LinuxReadOnlyToolEffectDriver, LinuxReadOnlyToolInput, LinuxSandboxRunner,
};
use sha2::{Digest, Sha256};

use crate::{
    coding_authority::{
        ApprovedCodingGrant, ApprovedCodingGrantRequest, BoundEndpointCodingPolicy,
        CodingApprovalRequest, CodingRuntimePolicy, derive_approved_coding_grant,
        render_coding_approval_request,
    },
    linux_coding_runtime::{
        CodingIdentitySource, failed_read_projection, read_outcome, serialize_read_result_for_model,
    },
};

const PREVIEW_LIFETIME_MS: u64 = 30_000;
// Same native preimage bound, independent of a model's requested output range.
const MAX_FILE_BYTES: u64 = agentmage_platform_linux::DEFAULT_MAX_PREIMAGE_BYTES;

/// Native dependencies selected by trusted startup, never a wire request.
/// The continuously held workspace must have been explicitly selected for reads.
/// No path selection, disclosure, model or tool execution occurs in construction.
pub struct CodingEndpointFilesInput<I> {
    /// Exact sealed endpoint request and its original kernel coding policy.
    pub bound: BoundEndpointCodingPolicy,
    /// Already authorized, continuously held native workspace root.
    pub workspace: Arc<LinuxAuthorizedWorkspace>,
    /// Original durable, privately rooted kernel authority owner.
    pub authority: LinuxAuthorityRuntime,
    /// Admitted offline native read-only worker runner.
    pub sandbox: LinuxSandboxRunner,
    /// Exact original single-file text catalog.
    pub registry: ToolRegistry,
    /// Person-facing actor bound by the policy.
    pub actor_id: ActorId,
    /// Trusted fresh identities for approvals, grants and transactions.
    pub identities: I,
}

struct Pending {
    operation_id: RuntimeOperationId,
    approval: ApprovalRequest,
    held: LinuxHeldObject,
}

struct Issued {
    pending: Pending,
    approved: ApprovedCodingGrant,
    evaluation: RuntimePermissionEvaluation,
    resolved_at: u64,
}

/// Non-cloneable, one-run native file/grant owner. It accepts only the exact
/// sealed endpoint request and original read-file definition. Every call asks
/// for an exact protected decision, derives a single-use durable grant and
/// dispatches only through the existing offline Linux worker effect driver.
/// Pending and issued objects cannot be reconstructed from metadata. Used
/// action identities remain spent even on refusal or failed dispatch.
/// This boundary does not provide durable coordinator recovery or activation.
pub struct CodingEndpointFiles<I> {
    grants: EndpointFileGrants<I>,
    sandbox: Option<LinuxSandboxRunner>,
}

struct FileGrantsInput<I> {
    bound: BoundEndpointCodingPolicy,
    workspace: Arc<LinuxAuthorizedWorkspace>,
    authority: LinuxAuthorityRuntime,
    registry: ToolRegistry,
    actor_id: ActorId,
    identities: I,
}

struct EndpointFileGrants<I> {
    request: RuntimeEndpointRunRequest,
    workspace: Arc<LinuxAuthorizedWorkspace>,
    authority: LinuxAuthorityRuntime,
    registry: ToolRegistry,
    policy: CodingRuntimePolicy,
    actor_id: ActorId,
    identities: I,
    spent: BTreeSet<ActionId>,
    faulted: bool,
    pending: Option<Pending>,
    issued: Option<Issued>,
}

impl<I: CodingIdentitySource> CodingEndpointFiles<I> {
    /// Composes native file grants and the real admitted offline worker runner.
    pub fn new(input: CodingEndpointFilesInput<I>) -> Result<Self, RuntimePortFailure> {
        let CodingEndpointFilesInput {
            bound,
            workspace,
            authority,
            sandbox,
            registry,
            actor_id,
            identities,
        } = input;
        let grants = EndpointFileGrants::new(FileGrantsInput {
            bound,
            workspace,
            authority,
            registry,
            actor_id,
            identities,
        })?;
        Ok(Self {
            grants,
            sandbox: Some(sandbox),
        })
    }
}

impl<I: CodingIdentitySource> EndpointFileGrants<I> {
    fn new(input: FileGrantsInput<I>) -> Result<Self, RuntimePortFailure> {
        let FileGrantsInput {
            bound,
            workspace,
            authority,
            registry,
            actor_id,
            identities,
        } = input;
        let BoundEndpointCodingPolicy { request, policy } = bound;
        let scope = RuntimeRunScope::endpoint(&request)?;
        let definition = read_only_tool_definition(ReadOnlyToolKind::ReadText);
        let tools = agentmage_kernel_engine::runtime_loop::runtime_tool_references(&registry)
            .map_err(|_| RuntimePortFailure::Invalid)?;
        let root = GrantTarget::workspace_scope(
            &*workspace,
            WorkspaceScopePath::new(workspace.workspace_id().clone(), std::iter::empty::<&str>())
                .map_err(|_| RuntimePortFailure::Invalid)?,
        )
        .map_err(|_| RuntimePortFailure::Invalid)?;
        if scope.mode() != RuntimeSessionMode::EphemeralReadOnly
            || request.event_cursor.is_some()
            || request.workspace_id != *workspace.workspace_id()
            || registry.list_tools() != vec![&definition]
            || tools != request.visible_tools
            || request.policy_id != *policy.policy_id()
            || request.policy_sha256 != policy.engine().policy_sha256()
            || !policy.binds_run(
                &actor_id,
                &request.task.task_id,
                &request.run_id,
                request.limits.max_tool_calls,
            )
            || policy.parent_targets() != [root.clone()]
            || policy
                .excluded_targets()
                .iter()
                .any(|target| target.is_operation_target() || !root.contains(target))
        {
            return Err(RuntimePortFailure::Invalid);
        }
        workspace
            .revalidate()
            .map_err(|_| RuntimePortFailure::Invalid)?;
        authority
            .revalidate_root()
            .map_err(|_| RuntimePortFailure::Unavailable)?;
        Ok(Self {
            request,
            workspace,
            authority,
            registry,
            policy,
            actor_id,
            identities,
            spent: BTreeSet::new(),
            faulted: false,
            pending: None,
            issued: None,
        })
    }

    fn id(&mut self, prefix: &str) -> Result<String, RuntimePortFailure> {
        self.identities
            .next(prefix)
            .map_err(|_| RuntimePortFailure::Unavailable)
    }

    fn check(
        &self,
        scope: RuntimeRunScope<'_>,
        definition: &ToolDefinition,
        call: &ToolCall,
    ) -> Result<(), RuntimePortFailure> {
        if self.faulted
            || scope.endpoint_request() != Some(&self.request)
            || self.registry.get_tool(&call.tool_id, &call.tool_version) != Some(definition)
            || !self.policy.admits_action(&call.action_id)
            || self.registry.validate_arguments(call).is_err()
        {
            return Err(RuntimePortFailure::Invalid);
        }
        self.workspace
            .revalidate()
            .map_err(|_| RuntimePortFailure::Invalid)?;
        self.authority
            .revalidate_root()
            .map_err(|_| RuntimePortFailure::Unavailable)
    }

    fn evaluate_file(
        &mut self,
        scope: RuntimeRunScope<'_>,
        operation_id: &RuntimeOperationId,
        definition: &ToolDefinition,
        call: &ToolCall,
        now: u64,
    ) -> Result<RuntimePermissionEvaluation, RuntimePortFailure> {
        self.check(scope, definition, call)?;
        if self.pending.is_some() || self.issued.is_some() || self.spent.contains(&call.action_id) {
            return Err(RuntimePortFailure::Invalid);
        }
        let parsed = validate_read_only_request(ReadOnlyToolKind::ReadText, &call.arguments.bytes)
            .map_err(|_| RuntimePortFailure::Invalid)?;
        if parsed.paths.len() != 1 || parsed.call_depth > self.request.limits.max_tool_call_depth {
            return Err(RuntimePortFailure::Invalid);
        }
        let path = WorkspacePath::new(self.request.workspace_id.clone(), parsed.paths[0].clone())
            .map_err(|_| RuntimePortFailure::Invalid)?;
        // Refuse exclusions before resolving or hashing any object there.
        if self
            .policy
            .excluded_targets()
            .iter()
            .any(|target| path.components().starts_with(target.path_components()))
        {
            return Err(RuntimePortFailure::Invalid);
        }
        let adapter =
            LinuxPathAdapter::new(self.workspace.adapter_instance_id().clone(), MAX_FILE_BYTES);
        let held = adapter
            .resolve(&self.workspace, &path, PathResolutionIntent::ReadFile)
            .map_err(|_| RuntimePortFailure::Invalid)?;
        held.revalidate().map_err(|_| RuntimePortFailure::Invalid)?;
        let target = GrantTarget::held_object(&held).map_err(|_| RuntimePortFailure::Invalid)?;
        let expires = now
            .checked_add(PREVIEW_LIFETIME_MS)
            .ok_or(RuntimePortFailure::Invalid)?;
        let plan = hash(&to_canonical_json(call).map_err(|_| RuntimePortFailure::Invalid)?);
        let grant_id = GrantId::from_raw(self.id("endpoint-parent")?);
        let nonce = GrantNonce::from_raw(self.id("endpoint-parent-nonce")?);
        let approval_id = ApprovalId::from_raw(self.id("endpoint-approval")?);
        let proposed_grant_id = GrantId::from_raw(self.id("endpoint-operation")?);
        // Retire before the first durable mutation; a failure cannot renew this action.
        self.spent.insert(call.action_id.clone());
        let parent = self
            .authority
            .authority_mut()
            .issue_session_read(SessionReadGrantRequest {
                grant_id,
                actor_id: self.actor_id.clone(),
                session_id: self.request.session_id.clone(),
                task_id: self.request.task.task_id.clone(),
                targets: self.policy.parent_targets().to_vec(),
                excluded_targets: self.policy.excluded_targets().to_vec(),
                sensitivity: self.request.work_packet.sensitivity,
                issued_at_epoch_ms: now,
                expires_at_epoch_ms: expires,
                nonce,
                maximum_derived_operations: 1,
                preview_sha256: plan.clone(),
                policy_sha256: self.policy.engine().policy_sha256().to_owned(),
            })
            .map_err(|_| RuntimePortFailure::Unavailable)?;
        let approval = render_coding_approval_request(
            &self.registry,
            CodingApprovalRequest {
                parent: &parent,
                approval_id,
                proposed_grant_id,
                call,
                operation: OperationBinding::new(GrantOperation::WorkspaceRead),
                targets: vec![target],
                operation_plan_sha256: &plan,
                issued_at_epoch_ms: now,
                expires_at_epoch_ms: expires,
            },
        )
        .map_err(|_| RuntimePortFailure::Invalid)?;
        let result = RuntimePermissionEvaluation::Ask {
            approval_id: approval.approval_id.clone(),
            grant_id: approval.proposed_grant_id.clone(),
            preview_sha256: approval.confirmation_sha256.clone(),
            expires_at_epoch_ms: expires,
        };
        self.pending = Some(Pending {
            operation_id: operation_id.clone(),
            approval,
            held,
        });
        Ok(result)
    }

    fn revalidate_target(&self, pending: &Pending) -> Result<(), RuntimePortFailure> {
        pending
            .held
            .revalidate()
            .map_err(|_| RuntimePortFailure::Invalid)?;
        // A held inode can remain valid after its path is renamed/replaced.
        // Re-resolve that original path as well; never switch the approved object.
        let adapter =
            LinuxPathAdapter::new(self.workspace.adapter_instance_id().clone(), MAX_FILE_BYTES);
        let current = adapter
            .resolve(
                &self.workspace,
                pending.held.workspace_path(),
                PathResolutionIntent::ReadFile,
            )
            .map_err(|_| RuntimePortFailure::Invalid)?;
        current
            .revalidate()
            .map_err(|_| RuntimePortFailure::Invalid)?;
        let target = GrantTarget::held_object(&current).map_err(|_| RuntimePortFailure::Invalid)?;
        if pending.approval.targets != [target] {
            return Err(RuntimePortFailure::Invalid);
        }
        Ok(())
    }

    fn resolve_file(
        &mut self,
        scope: RuntimeRunScope<'_>,
        challenge: &RuntimeApprovalChallenge,
        response: &RuntimeApprovalResponse,
        definition: &ToolDefinition,
        call: &ToolCall,
        now: u64,
    ) -> Result<RuntimePermissionEvaluation, RuntimePortFailure> {
        self.check(scope, definition, call)?;
        let pending = self.pending.as_ref().ok_or(RuntimePortFailure::Invalid)?;
        let approval = &pending.approval;
        if challenge.run_id != self.request.run_id
            || challenge.task_id != self.request.task.task_id
            || challenge.operation_id != pending.operation_id
            || challenge.tool_call_id != call.tool_call_id
            || challenge.approval_id != approval.approval_id
            || challenge.proposed_grant_id != approval.proposed_grant_id
            || challenge.operation != GrantOperation::WorkspaceRead
            || challenge.preview_sha256 != approval.confirmation_sha256
            || challenge.expires_at_epoch_ms != approval.expires_at_epoch_ms
            || approval.tool_call != *call
            || response.disposition == RuntimeApprovalDisposition::Narrow
        {
            return Err(RuntimePortFailure::Invalid);
        }
        verify_runtime_approval_response(challenge, response, now)
            .map_err(|_| RuntimePortFailure::Invalid)?;
        let pending = self.pending.take().ok_or(RuntimePortFailure::Invalid)?;
        let approval = &pending.approval;
        let decision_sha256 =
            hash(&to_canonical_json(response).map_err(|_| RuntimePortFailure::Invalid)?);
        if response.disposition == RuntimeApprovalDisposition::Deny {
            return Ok(RuntimePermissionEvaluation::Deny {
                approval_id: approval.approval_id.clone(),
                grant_id: approval.proposed_grant_id.clone(),
                preview_sha256: approval.confirmation_sha256.clone(),
                expires_at_epoch_ms: approval.expires_at_epoch_ms,
                decision_sha256,
                reason_code: "runtime.coding.user-denied".into(),
            });
        }
        self.revalidate_target(&pending)?;
        let nonce = GrantNonce::from_raw(self.id("endpoint-operation-nonce")?);
        let approved = derive_approved_coding_grant(ApprovedCodingGrantRequest {
            authority: self.authority.authority_mut(),
            registry: &self.registry,
            policy: self.policy.engine(),
            approval,
            challenge,
            response,
            nonce,
            now_epoch_ms: now,
        })
        .map_err(|_| RuntimePortFailure::Invalid)?;
        let result = RuntimePermissionEvaluation::Allow {
            approval_id: approval.approval_id.clone(),
            grant_id: approved.grant.grant_id.clone(),
            preview_sha256: approval.confirmation_sha256.clone(),
            expires_at_epoch_ms: approval.expires_at_epoch_ms,
            decision_sha256: approved.decision_sha256.clone(),
            authority_sha256: approved.authority_sha256.clone(),
        };
        self.issued = Some(Issued {
            pending,
            approved,
            evaluation: result.clone(),
            resolved_at: now,
        });
        Ok(result)
    }

    fn prepare_execution(
        &mut self,
        scope: RuntimeRunScope<'_>,
        evaluation: &RuntimePermissionEvaluation,
        definition: &ToolDefinition,
        call: &ToolCall,
    ) -> Result<
        (
            ApprovedCodingGrant,
            AuthorityTransactionRequest,
            LinuxReadOnlyToolInput,
        ),
        RuntimePortFailure,
    > {
        self.check(scope, definition, call)?;
        let issued = self.issued.as_ref().ok_or(RuntimePortFailure::Invalid)?;
        if issued.evaluation != *evaluation || issued.pending.approval.tool_call != *call {
            return Err(RuntimePortFailure::Invalid);
        }
        // Once a matching effect is attempted it cannot be reconstructed/retried,
        // including descriptor drift, cancellation and transaction uncertainty.
        let issued = self.issued.take().ok_or(RuntimePortFailure::Invalid)?;
        self.revalidate_target(&issued.pending)?;
        let grant = &issued.approved.grant;
        let context = PolicyEvaluationContext {
            actor_id: grant.actor_id.clone(),
            session_id: grant.session_id.clone(),
            task_id: grant.task_id.clone(),
            action_id: call.action_id.clone(),
            action_kind: ActionKind::DeterministicTool,
            tool_id: call.tool_id.clone(),
            tool_version: call.tool_version.clone(),
            targets: grant.targets.clone(),
            argument_sha256: grant.argument_sha256.clone(),
            preimages: grant.preimages.clone(),
            expected_side_effects: grant.expected_side_effects.clone(),
            preview_sha256: grant.preview_sha256.clone(),
            now_epoch_ms: issued.resolved_at,
            network_scope: None,
            credential_scope: None,
            publication_scope: None,
        };
        let transaction = AuthorityTransactionRequest::new(
            AuthorityTransactionId::from_raw(self.id("endpoint-transaction")?),
            OperationAttemptId::from_raw(self.id("endpoint-attempt")?),
            issued.pending.approval.approval_id,
            grant.grant_id.clone(),
            call.clone(),
            context,
            issued.resolved_at,
            format!("epoch-ms:{}", issued.resolved_at),
        )
        .map_err(|_| RuntimePortFailure::Invalid)?;
        let input = LinuxReadOnlyToolInput::seal(
            call.tool_id.as_str(),
            &call.tool_version,
            &call.arguments.bytes,
            vec![issued.pending.held],
        )
        .map_err(|_| RuntimePortFailure::Invalid)?;
        Ok((issued.approved, transaction, input))
    }
}

impl<I: CodingIdentitySource> CodingEndpointFiles<I> {
    fn execute_file(
        &mut self,
        scope: RuntimeRunScope<'_>,
        evaluation: &RuntimePermissionEvaluation,
        definition: &ToolDefinition,
        call: &ToolCall,
        cancellation: Option<&dyn ModelCancellationProbe>,
    ) -> Result<RuntimeToolExecution, RuntimePortFailure> {
        let (issued, transaction, input) = self
            .grants
            .prepare_execution(scope, evaluation, definition, call)?;
        let observed = cancellation.map(|source| {
            BorrowedEffectCancellation::new(
                source,
                self.grants.request.task.task_id.clone(),
                call.correlation_id.clone(),
            )
        });
        let runner = self.sandbox.take().ok_or(RuntimePortFailure::Unavailable)?;
        let mut driver = match &observed {
            Some(control) => LinuxReadOnlyToolEffectDriver::new_observed(
                runner,
                input,
                control as &dyn EffectCancellationObservation,
            ),
            None => LinuxReadOnlyToolEffectDriver::new(runner, input),
        };
        self.grants.faulted = true;
        let receipt = self.grants.authority.authority_mut().execute_effect(
            &self.grants.registry,
            &issued.policy,
            transaction,
            &mut driver,
        );
        let output = driver.take_result();
        let error = driver.take_error();
        self.sandbox = Some(driver.into_runner());
        let receipt = receipt.map_err(|_| RuntimePortFailure::Uncertain)?;
        self.grants
            .authority
            .revalidate_root()
            .map_err(|_| RuntimePortFailure::Uncertain)?;
        self.grants.faulted = receipt.outcome == OperationOutcome::Uncertain;
        if receipt.outcome != OperationOutcome::Succeeded
            || error.is_some()
            || !output.as_ref().is_some_and(|value| value.success())
        {
            return Ok(failed_read_projection(
                &receipt,
                call,
                "runtime.tool.worker-failed",
            ));
        }
        let result = output
            .and_then(|output| serde_json::from_slice::<ReadOnlyResult>(output.stdout()).ok())
            .filter(|result| result.verify(ReadOnlyToolKind::ReadText));
        let Some(result) = result else {
            return Ok(failed_read_projection(
                &receipt,
                call,
                "runtime.tool.output-invalid",
            ));
        };
        let bytes = match serialize_read_result_for_model(&result) {
            Ok(bytes) if bytes.len() as u64 <= self.grants.request.limits.max_output_bytes => bytes,
            Ok(_) => {
                return Ok(failed_read_projection(
                    &receipt,
                    call,
                    "runtime.tool.output-limit-exceeded",
                ));
            }
            Err(code) => return Ok(failed_read_projection(&receipt, call, code)),
        };
        let evidence = if read_outcome(result.outcome) == OperationOutcome::Succeeded {
            vec![EvidenceReference {
                schema_version: CONTRACT_SCHEMA_VERSION,
                evidence_id: EvidenceId::from_raw(self.grants.id("endpoint-evidence")?),
                kind: EvidenceKind::ToolOutput,
                source_id: format!("native:{}@{}", call.tool_id.as_str(), call.tool_version),
                object_id: call.tool_call_id.as_str().into(),
                fragment: None,
                content_sha256: result.result_sha256.clone(),
                observed_revision: Some(self.grants.request.repository_snapshot_id.as_str().into()),
            }]
        } else {
            vec![]
        };
        Ok(RuntimeToolExecution {
            receipt_id: receipt.receipt_id,
            receipt_sha256: receipt.receipt_sha256,
            result: ToolResult {
                schema_version: CONTRACT_SCHEMA_VERSION,
                tool_call_id: call.tool_call_id.clone(),
                correlation_id: call.correlation_id.clone(),
                outcome: read_outcome(result.outcome),
                output: Some(ContractPayload {
                    schema: definition.output_schema.clone(),
                    media_type: "application/json".into(),
                    sha256: hash(&bytes),
                    bytes,
                }),
                validation_issues: vec![],
                evidence,
                error: None,
                elapsed_ms: 0,
                state_change: StateChange::NotChanged,
            },
            result_output_kind: Some(RuntimeArtifactKind::Report),
            artifact_candidates: vec![],
        })
    }
}

impl<I: CodingIdentitySource> RuntimeToolBoundary for CodingEndpointFiles<I> {
    fn evaluate_scoped(
        &mut self,
        scope: RuntimeRunScope<'_>,
        operation_id: &RuntimeOperationId,
        definition: &ToolDefinition,
        call: &ToolCall,
        now: u64,
    ) -> Result<RuntimePermissionEvaluation, RuntimePortFailure> {
        self.grants
            .evaluate_file(scope, operation_id, definition, call, now)
    }

    fn resolve_scoped(
        &mut self,
        scope: RuntimeRunScope<'_>,
        challenge: &RuntimeApprovalChallenge,
        response: &RuntimeApprovalResponse,
        definition: &ToolDefinition,
        call: &ToolCall,
        now: u64,
    ) -> Result<RuntimePermissionEvaluation, RuntimePortFailure> {
        self.grants
            .resolve_file(scope, challenge, response, definition, call, now)
    }

    fn execute_scoped(
        &mut self,
        scope: RuntimeRunScope<'_>,
        evaluation: &RuntimePermissionEvaluation,
        definition: &ToolDefinition,
        call: &ToolCall,
        cancellation: Option<&dyn ModelCancellationProbe>,
    ) -> Result<RuntimeToolExecution, RuntimePortFailure> {
        self.execute_file(scope, evaluation, definition, call, cancellation)
    }

    fn evaluate(
        &mut self,
        _: &RuntimeRunRequest,
        _: &RuntimeOperationId,
        _: &ToolDefinition,
        _: &ToolCall,
        _: u64,
    ) -> Result<RuntimePermissionEvaluation, RuntimePortFailure> {
        Err(RuntimePortFailure::Invalid)
    }

    fn resolve(
        &mut self,
        _: &RuntimeRunRequest,
        _: &RuntimeApprovalChallenge,
        _: &RuntimeApprovalResponse,
        _: &ToolDefinition,
        _: &ToolCall,
        _: u64,
    ) -> Result<RuntimePermissionEvaluation, RuntimePortFailure> {
        Err(RuntimePortFailure::Invalid)
    }

    fn execute(
        &mut self,
        _: &RuntimeRunRequest,
        _: &RuntimePermissionEvaluation,
        _: &ToolDefinition,
        _: &ToolCall,
        _: Option<&dyn ModelCancellationProbe>,
    ) -> Result<RuntimeToolExecution, RuntimePortFailure> {
        Err(RuntimePortFailure::Invalid)
    }
}

fn hash(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[cfg(test)]
#[path = "coding_endpoint_files_tests.rs"]
mod tests;
