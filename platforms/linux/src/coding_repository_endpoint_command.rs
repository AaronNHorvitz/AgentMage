// SPDX-License-Identifier: Apache-2.0
//! One original command/test owner through the scoped endpoint tool port.

use super::*;
use agentmage_kernel_contracts::{
    AuthorizedWorkspaceHandle, CancellationSignal, EvidenceId, EvidenceKind, EvidenceReference,
    ModelCancellationProbe, ModelOperationControl, ModelOperationStop, Receipt,
    RuntimeApprovalChallenge, RuntimeApprovalDisposition, RuntimeApprovalPresentation,
    RuntimeApprovalResponse, RuntimeArtifactKind, RuntimeEndpointRunRequest, RuntimeOperationId,
    RuntimeRunRequest, RuntimeSessionMode, RuntimeTurnId, StateChange, ToolResult,
    to_canonical_json,
};
use agentmage_kernel_engine::{
    runtime_coordinator::{
        seal_runtime_approval_challenge, verify_runtime_approval_response,
        verify_runtime_endpoint_run_request,
    },
    runtime_loop::{
        RuntimeClock, RuntimePermissionEvaluation, RuntimePortFailure, RuntimeRunScope,
        RuntimeToolBoundary, RuntimeToolExecution, runtime_action_id, runtime_tool_references,
    },
};
use std::sync::Arc;

// Captured channels are serialized as JSON byte arrays: each byte can require
// three decimal digits and a separator. Reserve metadata room as well, before
// an approval or process effect. This is a minimum admission reservation, not
// an upper bound on arbitrary parsed validation metadata; project() must still
// check the actual complete result after dispatch.
const COMMAND_PROJECTION_ALLOWANCE: u64 = 16 * 1024;
fn output_reservation(stdout_bytes: u64, stderr_bytes: u64) -> Option<u64> {
    stdout_bytes
        .checked_add(stderr_bytes)?
        .checked_mul(4)?
        .checked_add(COMMAND_PROJECTION_ALLOWANCE)
}

struct EndpointCommandTool(ToolDefinition);
impl EndpointCommandTool {
    fn new() -> Self {
        let schema = |id: &str, text: &str| SchemaReference {
            schema_id: SchemaId::from_raw(id),
            schema_version: 1,
            schema_sha256: hash(text.as_bytes()),
        };
        let operation = OperationBinding::new(GrantOperation::CommandExecute);
        Self(ToolDefinition {
            schema_version: CONTRACT_SCHEMA_VERSION,
            tool_id: ToolId::from_raw("agentmage.code.run-prepared-command"),
            tool_version: "1.0.0".into(),
            display_name: "Run the exact prepared worktree command or tests".into(),
            description: "Only the original approved offline command; output is untrusted".into(),
            input_schema: schema(
                "agentmage.code.prepared-command.input",
                r#"{"type":"object","required":["preview"],"additionalProperties":false,"properties":{"preview":{"type":"object"}}}"#,
            ),
            output_schema: schema(
                "agentmage.code.prepared-command.output",
                r#"{"type":"object"}"#,
            ),
            risk_level: ToolRiskLevel::High,
            declared_effects: vec![operation],
            required_grant: RequiredGrantTemplate {
                operation,
                target_scope: "exact-service-created-worktree-command".into(),
                single_use: true,
            },
            timeout_ms: LIFETIME_MS,
        })
    }
}
impl Tool for EndpointCommandTool {
    fn definition(&self) -> &ToolDefinition {
        &self.0
    }
    fn validate_arguments(&self, bytes: &[u8]) -> Vec<ValidationIssue> {
        if serde_json::from_slice::<Value>(bytes)
            .ok()
            .is_some_and(|v| {
                v.as_object()
                    .is_some_and(|o| o.len() == 1 && o.get("preview").is_some_and(Value::is_object))
            })
        {
            vec![]
        } else {
            vec![ValidationIssue {
                code: "code.endpoint-command.arguments".into(),
                severity: ValidationSeverity::Error,
                field_path: vec!["arguments".into()],
                message: "The complete original preview is required".into(),
            }]
        }
    }
}

/// Single-operation endpoint catalog. No approval, command launch or model call.
pub fn linux_coding_endpoint_command_registry() -> Result<ToolRegistry> {
    let mut registry = ToolRegistry::new();
    registry
        .register_tool(Box::new(EndpointCommandTool::new()))
        .map_err(|_| err("code.endpoint-command.catalog"))?;
    Ok(registry)
}

/// Process-held original pending command or test, never reconstructed from wire metadata.
pub struct LinuxCodingEndpointCommand {
    repository: LinuxCodingRepository,
    request: RuntimeEndpointRunRequest,
    challenge: RuntimeApprovalChallenge,
    call: ToolCall,
    preview: Value,
}
impl LinuxCodingRepository {
    /// Moves the live prepared command/test into one endpoint decision. Trusted
    /// startup retains admission, instruction narrowing and disclosure consent.
    /// Refusal returns the original owner without granting or dispatching.
    #[allow(clippy::result_large_err)]
    pub fn into_endpoint_command(
        self,
        request: RuntimeEndpointRunRequest,
        turn_id: RuntimeTurnId,
        operation_id: RuntimeOperationId,
        call: ToolCall,
    ) -> std::result::Result<LinuxCodingEndpointCommand, (Self, LinuxCodingRepositoryError)> {
        let prepare = || {
            verify_runtime_endpoint_run_request(&request)
                .map_err(|_| err("code.endpoint-command.binding"))?;
            let registry = linux_coding_endpoint_command_registry()?;
            let definition = EndpointCommandTool::new().0;
            let created = self.verify_created()?;
            let pending = self
                .command
                .as_ref()
                .ok_or(err("code.command.choice-required"))?;
            let bytes = serde_json::to_vec(&json!({"preview":pending.preview}))
                .map_err(|_| err("code.endpoint-command.binding"))?;
            if request.mode != RuntimeSessionMode::ControlledWrite
                || request.event_cursor.is_some()
                || request.workspace_id != *created.held.workspace_id()
                || request.visible_tools
                    != runtime_tool_references(&registry)
                        .map_err(|_| err("code.endpoint-command.catalog"))?
                || pending.preview["worktree_root_sha256"] != created.held.root_path_sha256()
                || pending.preview["creation_receipt_sha256"] != created.creation_receipt_sha256
                || pending.issued.is_some()
                || registry.validate_arguments(&call).is_err()
                || call.schema_version != CONTRACT_SCHEMA_VERSION
                || call.tool_id != definition.tool_id
                || call.tool_version != definition.tool_version
                || call.arguments.schema != definition.input_schema
                || call.arguments.media_type != "application/json"
                || call.arguments.bytes != bytes
                || call.arguments.sha256 != hash(&bytes)
                || !(1..=request.limits.max_tool_calls)
                    .any(|i| call.action_id == runtime_action_id(&request.run_id, i))
            {
                return Err(err("code.endpoint-command.binding"));
            }
            let bounds = pending.prepared.command().bounds;
            let budget = |resource| {
                request
                    .work_packet
                    .budgets
                    .iter()
                    .find(|b| b.resource == resource)
                    .map_or(0, |b| b.limit)
            };
            if bounds.timeout_ms > request.limits.max_elapsed_ms
                || bounds.memory_bytes
                    > budget(agentmage_kernel_contracts::BudgetResource::MemoryBytes)
                || u64::from(bounds.task_count)
                    > budget(agentmage_kernel_contracts::BudgetResource::ProcessCount)
                || output_reservation(bounds.stdout_bytes, bounds.stderr_bytes)
                    .is_none_or(|bytes| bytes > request.limits.max_output_bytes)
            {
                return Err(err("code.endpoint-command.bounds"));
            }
            let id = &pending.prepared.preview().command_attempt_id;
            let challenge = seal_runtime_approval_challenge(RuntimeApprovalChallenge {
                schema_version: CONTRACT_SCHEMA_VERSION,
                run_id: request.run_id.clone(),
                task_id: request.task.task_id.clone(),
                turn_id,
                operation_id,
                tool_call_id: call.tool_call_id.clone(),
                approval_id: ApprovalId::from_raw(format!("approval-{id}")),
                // Preserve the original issuer's identity, including its legacy prefix.
                proposed_grant_id: GrantId::from_raw(format!("create-{id}")),
                operation: GrantOperation::CommandExecute,
                presentation: RuntimeApprovalPresentation {
                    tool_id: definition.tool_id,
                    tool_version: definition.tool_version,
                    display_name: definition.display_name,
                    risk_level: definition.risk_level,
                    target_scope: definition.required_grant.target_scope,
                    arguments: call.arguments.clone(),
                    declared_effects: definition.declared_effects,
                    single_use: true,
                    timeout_ms: definition.timeout_ms,
                },
                preview_sha256: pending.hash.clone(),
                expires_at_epoch_ms: pending.expires_at,
                challenge_sha256: "0".repeat(64),
            })
            .map_err(|_| err("code.endpoint-command.binding"))?;
            Ok((challenge, pending.preview.clone()))
        };
        match prepare() {
            Ok((challenge, preview)) => Ok(LinuxCodingEndpointCommand {
                repository: self,
                request,
                challenge,
                call,
                preview,
            }),
            Err(error) => Err((self, error)),
        }
    }
}
impl LinuxCodingEndpointCommand {
    /// Abandons this original pending command or validation and returns the same
    /// live repository owner. No grant is issued, effect dispatched or preview
    /// restored. Existing command/validation results remain inspectable.
    #[must_use]
    pub fn into_repository(mut self) -> LinuxCodingRepository {
        self.repository.command = None;
        self.repository
    }

    /// Original sealed request, without endpoint credential material.
    pub const fn request(&self) -> &RuntimeEndpointRunRequest {
        &self.request
    }
    /// Exact original protected decision.
    pub const fn challenge(&self) -> &RuntimeApprovalChallenge {
        &self.challenge
    }
    /// Original canonical proposed call.
    pub const fn call(&self) -> &ToolCall {
        &self.call
    }
    /// Retains original continuous control and trusted epoch clock. No activation.
    pub fn into_boundary<C: RuntimeClock>(
        self,
        control: Arc<dyn ModelOperationControl + Send + Sync>,
        clock: C,
    ) -> LinuxCodingEndpointCommandBoundary<C> {
        LinuxCodingEndpointCommandBoundary {
            command: self,
            control,
            clock,
            stage: Stage::Ready,
            result: None,
        }
    }
}

enum Stage {
    Ready,
    Asked,
    Issued {
        evaluation: Box<RuntimePermissionEvaluation>,
        resolved_at: u64,
    },
    Spent,
}
/// One-shot coordinator adapter. Uses the original native command/test executor,
/// durable issuer and parser. Native-family calls refuse; no service or recovery.
pub struct LinuxCodingEndpointCommandBoundary<C> {
    command: LinuxCodingEndpointCommand,
    control: Arc<dyn ModelOperationControl + Send + Sync>,
    clock: C,
    stage: Stage,
    result: Option<Value>,
}
impl<C: RuntimeClock> LinuxCodingEndpointCommandBoundary<C> {
    /// Original command/test state and last result, including post-dispatch output refusal.
    pub fn inspect(&self) -> Result<Value> {
        Ok(json!({"command":self.command.repository.inspect_command()?,
            "validation":self.command.repository.inspect_validation()?,"last_result":self.result,
            "spent":matches!(self.stage,Stage::Spent),"automatic_replay":false}))
    }
    /// Returns the same live owner, abandoning any pending preview or issued grant
    /// handle. Durable authority remains recorded and expires normally; no replay.
    #[must_use]
    pub fn into_repository(self) -> LinuxCodingRepository {
        self.command.into_repository()
    }
    fn check(
        &self,
        scope: RuntimeRunScope<'_>,
        definition: &ToolDefinition,
        call: &ToolCall,
    ) -> std::result::Result<(), RuntimePortFailure> {
        if scope.endpoint_request() != Some(&self.command.request)
            || definition != &EndpointCommandTool::new().0
            || call != &self.command.call
        {
            return Err(RuntimePortFailure::Invalid);
        }
        Ok(())
    }
    fn running(&self) -> std::result::Result<(), RuntimePortFailure> {
        self.control
            .remaining_ms()
            .map(|_| ())
            .map_err(|stop| match stop {
                ModelOperationStop::Cancelled => RuntimePortFailure::Cancelled,
                ModelOperationStop::TimedOut => RuntimePortFailure::TimedOut,
                ModelOperationStop::Unavailable => RuntimePortFailure::Unavailable,
                ModelOperationStop::Invalid => RuntimePortFailure::Invalid,
            })
    }
    fn current(&self, now: u64) -> std::result::Result<(), RuntimePortFailure> {
        let owner = &self.command.repository;
        let pending = owner.command.as_ref().ok_or(RuntimePortFailure::Invalid)?;
        if pending.preview != self.command.preview
            || now < pending.created_at
            || now >= pending.expires_at
            || Instant::now() >= pending.deadline
        {
            return Err(RuntimePortFailure::Invalid);
        }
        owner
            .command_effect(pending, now)
            .map_err(|_| RuntimePortFailure::Invalid)?;
        Ok(())
    }
    // Only private component tests supply a synthetic preflight/executor. The
    // public scoped entry points always use the original native manifest checks.
    fn resolve_using(
        &mut self,
        scope: RuntimeRunScope<'_>,
        inputs: (
            &RuntimeApprovalChallenge,
            &RuntimeApprovalResponse,
            &ToolDefinition,
            &ToolCall,
        ),
        now: u64,
        preflight: impl FnOnce(&CommandRegistry) -> Result<()>,
    ) -> std::result::Result<RuntimePermissionEvaluation, RuntimePortFailure> {
        let (challenge, response, definition, call) = inputs;
        self.check(scope, definition, call)?;
        if !matches!(self.stage, Stage::Asked)
            || challenge != &self.command.challenge
            || response.disposition == RuntimeApprovalDisposition::Narrow
        {
            return Err(RuntimePortFailure::Invalid);
        }
        verify_runtime_approval_response(challenge, response, now)
            .map_err(|_| RuntimePortFailure::Invalid)?;
        self.stage = Stage::Spent;
        self.running()?;
        self.current(now)?;
        let decision_sha256 =
            hash(&to_canonical_json(response).map_err(|_| RuntimePortFailure::Invalid)?);
        if response.disposition == RuntimeApprovalDisposition::Deny {
            self.command
                .repository
                .take_command(&challenge.preview_sha256, false, now)
                .map_err(|_| RuntimePortFailure::Unavailable)?;
            return Ok(RuntimePermissionEvaluation::Deny {
                approval_id: challenge.approval_id.clone(),
                grant_id: challenge.proposed_grant_id.clone(),
                preview_sha256: challenge.preview_sha256.clone(),
                expires_at_epoch_ms: challenge.expires_at_epoch_ms,
                decision_sha256,
                reason_code: "code.endpoint-command.user-denied".into(),
            });
        }
        let pending = self
            .command
            .repository
            .command
            .as_ref()
            .ok_or(RuntimePortFailure::Invalid)?;
        let registry = command_registry(pending.prepared.command().clone())
            .map_err(|_| RuntimePortFailure::Invalid)?;
        preflight(&registry).map_err(|_| RuntimePortFailure::Unavailable)?;
        self.running()?;
        self.current(now)?;
        self.command
            .repository
            .issue_command(now)
            .map_err(|_| RuntimePortFailure::Unavailable)?;
        let issued = self
            .command
            .repository
            .command
            .as_ref()
            .and_then(|p| p.issued.as_ref())
            .ok_or(RuntimePortFailure::Invalid)?;
        if issued.grant.grant_id != challenge.proposed_grant_id {
            return Err(RuntimePortFailure::Invalid);
        }
        let evaluation = RuntimePermissionEvaluation::Allow {
            approval_id: challenge.approval_id.clone(),
            grant_id: issued.grant.grant_id.clone(),
            preview_sha256: challenge.preview_sha256.clone(),
            expires_at_epoch_ms: challenge.expires_at_epoch_ms,
            decision_sha256,
            authority_sha256: hash(
                &to_canonical_json(&issued.grant).map_err(|_| RuntimePortFailure::Invalid)?,
            ),
        };
        self.stage = Stage::Issued {
            evaluation: Box::new(evaluation.clone()),
            resolved_at: now,
        };
        Ok(evaluation)
    }
    fn execute_using<E: BoundedCommandExecutor<WorkingDirectory = LinuxAuthorizedWorkspace>>(
        &mut self,
        scope: RuntimeRunScope<'_>,
        inputs: (&RuntimePermissionEvaluation, &ToolDefinition, &ToolCall),
        cancellation: Option<&dyn ModelCancellationProbe>,
        executor: impl FnOnce(&CommandRegistry) -> Result<E>,
    ) -> std::result::Result<RuntimeToolExecution, RuntimePortFailure> {
        let (evaluation, definition, call) = inputs;
        self.check(scope, definition, call)?;
        let Stage::Issued {
            evaluation: issued,
            resolved_at,
        } = &self.stage
        else {
            return Err(RuntimePortFailure::Invalid);
        };
        if issued.as_ref() != evaluation {
            return Err(RuntimePortFailure::Invalid);
        }
        let resolved_at = *resolved_at;
        self.stage = Stage::Spent;
        self.running()?;
        let now = self.clock.now_epoch_ms()?;
        if now < resolved_at {
            return Err(RuntimePortFailure::Invalid);
        }
        self.current(now)?;
        let owner = &mut self.command.repository;
        let pending = owner
            .take_command(&self.command.challenge.preview_sha256, true, now)
            .map_err(|_| RuntimePortFailure::Invalid)?
            .ok_or(RuntimePortFailure::Invalid)?;
        let signal = LinuxCodingCommandCancellation::for_pending(&pending);
        let bridge = CommandControl {
            control: self.control.as_ref(),
            probe: cancellation,
            task: &self.command.request.task.task_id,
            correlation: &call.correlation_id,
            signal,
        };
        // Check before native preflight, then continuously through the original
        // supervisor. A foreign signal or unavailable observation fails closed.
        if bridge
            .observe_effect_cancellation()
            .map_err(|_| RuntimePortFailure::Unavailable)?
            .is_some()
        {
            return Err(RuntimePortFailure::Cancelled);
        }
        let registry = command_registry(pending.prepared.command().clone())
            .map_err(|_| RuntimePortFailure::Invalid)?;
        let executor = executor(&registry).map_err(|_| RuntimePortFailure::Unavailable)?;
        let started = Instant::now();
        let value = owner
            .run_command_observed(pending, executor, now, bridge)
            .map_err(|_| RuntimePortFailure::Uncertain)?;
        self.result = Some(value.clone());
        project(
            &self.command.request,
            definition,
            call,
            value,
            started.elapsed(),
        )
    }
}
impl<C: RuntimeClock> RuntimeToolBoundary for LinuxCodingEndpointCommandBoundary<C> {
    fn evaluate_scoped(
        &mut self,
        scope: RuntimeRunScope<'_>,
        operation: &RuntimeOperationId,
        definition: &ToolDefinition,
        call: &ToolCall,
        now: u64,
    ) -> std::result::Result<RuntimePermissionEvaluation, RuntimePortFailure> {
        self.check(scope, definition, call)?;
        if !matches!(self.stage, Stage::Ready) || operation != &self.command.challenge.operation_id
        {
            return Err(RuntimePortFailure::Invalid);
        }
        self.stage = Stage::Spent;
        self.running()?;
        self.current(now)?;
        self.stage = Stage::Asked;
        let c = &self.command.challenge;
        Ok(RuntimePermissionEvaluation::Ask {
            approval_id: c.approval_id.clone(),
            grant_id: c.proposed_grant_id.clone(),
            preview_sha256: c.preview_sha256.clone(),
            expires_at_epoch_ms: c.expires_at_epoch_ms,
        })
    }
    fn resolve_scoped(
        &mut self,
        scope: RuntimeRunScope<'_>,
        challenge: &RuntimeApprovalChallenge,
        response: &RuntimeApprovalResponse,
        definition: &ToolDefinition,
        call: &ToolCall,
        now: u64,
    ) -> std::result::Result<RuntimePermissionEvaluation, RuntimePortFailure> {
        self.resolve_using(
            scope,
            (challenge, response, definition, call),
            now,
            |registry| native_executor(registry).map(|_| ()),
        )
    }
    fn execute_scoped(
        &mut self,
        scope: RuntimeRunScope<'_>,
        evaluation: &RuntimePermissionEvaluation,
        definition: &ToolDefinition,
        call: &ToolCall,
        cancellation: Option<&dyn ModelCancellationProbe>,
    ) -> std::result::Result<RuntimeToolExecution, RuntimePortFailure> {
        self.execute_using(
            scope,
            (evaluation, definition, call),
            cancellation,
            native_executor,
        )
    }
    fn evaluate(
        &mut self,
        _: &RuntimeRunRequest,
        _: &RuntimeOperationId,
        _: &ToolDefinition,
        _: &ToolCall,
        _: u64,
    ) -> std::result::Result<RuntimePermissionEvaluation, RuntimePortFailure> {
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
    ) -> std::result::Result<RuntimePermissionEvaluation, RuntimePortFailure> {
        Err(RuntimePortFailure::Invalid)
    }
    fn execute(
        &mut self,
        _: &RuntimeRunRequest,
        _: &RuntimePermissionEvaluation,
        _: &ToolDefinition,
        _: &ToolCall,
        _: Option<&dyn ModelCancellationProbe>,
    ) -> std::result::Result<RuntimeToolExecution, RuntimePortFailure> {
        Err(RuntimePortFailure::Invalid)
    }
}
struct CommandControl<'a> {
    control: &'a (dyn ModelOperationControl + Sync),
    probe: Option<&'a dyn ModelCancellationProbe>,
    task: &'a TaskId,
    correlation: &'a CorrelationId,
    signal: LinuxCodingCommandCancellation,
}
impl EffectCancellationObservation for CommandControl<'_> {
    fn observe_effect_cancellation(
        &self,
    ) -> std::result::Result<Option<CancellationSignal>, CancellationObservationError> {
        match self.control.remaining_ms() {
            Ok(_) => (),
            Err(stop @ (ModelOperationStop::Cancelled | ModelOperationStop::TimedOut)) => {
                let reason = if stop == ModelOperationStop::TimedOut {
                    agentmage_kernel_contracts::CancellationReason::DeadlineReached
                } else {
                    agentmage_kernel_contracts::CancellationReason::UserRequested
                };
                self.signal
                    .request(reason)
                    .map_err(|_| CancellationObservationError::Unavailable)?;
            }
            Err(ModelOperationStop::Unavailable) => {
                return Err(CancellationObservationError::Unavailable);
            }
            Err(ModelOperationStop::Invalid) => {
                return Err(CancellationObservationError::InvalidSignal);
            }
        }
        if let Some(probe) = self.probe
            && let Some(signal) = probe
                .observe()
                .map_err(|_| CancellationObservationError::Unavailable)?
        {
            if signal.schema_version != CONTRACT_SCHEMA_VERSION
                || &signal.task_id != self.task
                || &signal.correlation_id != self.correlation
            {
                return Err(CancellationObservationError::ScopeMismatch);
            }
            self.signal
                .request(signal.reason)
                .map_err(|_| CancellationObservationError::Unavailable)?;
        }
        observe_command_cancellation(&self.signal)
    }
}
fn project(
    request: &RuntimeEndpointRunRequest,
    definition: &ToolDefinition,
    call: &ToolCall,
    value: Value,
    elapsed: Duration,
) -> std::result::Result<RuntimeToolExecution, RuntimePortFailure> {
    let receipt: Receipt = serde_json::from_value(value["receipt"].clone())
        .map_err(|_| RuntimePortFailure::Uncertain)?;
    let bytes = serde_json::to_vec(&value).map_err(|_| RuntimePortFailure::Uncertain)?;
    if bytes.len() as u64 > request.limits.max_output_bytes {
        return Err(RuntimePortFailure::Uncertain);
    }
    let digest = hash(&bytes);
    Ok(RuntimeToolExecution {
        receipt_id: receipt.receipt_id,
        receipt_sha256: receipt.receipt_sha256,
        result: ToolResult {
            schema_version: CONTRACT_SCHEMA_VERSION,
            tool_call_id: call.tool_call_id.clone(),
            correlation_id: call.correlation_id.clone(),
            outcome: receipt.outcome,
            output: Some(ContractPayload {
                schema: definition.output_schema.clone(),
                media_type: "application/json".into(),
                sha256: digest.clone(),
                bytes,
            }),
            validation_issues: vec![],
            evidence: vec![EvidenceReference {
                schema_version: CONTRACT_SCHEMA_VERSION,
                evidence_id: EvidenceId::from_raw(format!("native-command:{}", &digest[..32])),
                kind: EvidenceKind::ToolOutput,
                source_id: format!("native:{}@{}", call.tool_id.as_str(), call.tool_version),
                object_id: call.tool_call_id.as_str().into(),
                fragment: None,
                content_sha256: digest,
                observed_revision: Some(request.repository_snapshot_id.as_str().into()),
            }],
            error: None,
            elapsed_ms: u64::try_from(elapsed.as_millis()).unwrap_or(u64::MAX),
            // The original native command worker mounts the worktree read-only.
            // This is workspace-state reporting, never absence of a process effect.
            state_change: StateChange::NotChanged,
        },
        result_output_kind: Some(RuntimeArtifactKind::Report),
        artifact_candidates: vec![],
    })
}

#[cfg(test)]
#[path = "coding_repository_endpoint_command_tests.rs"]
mod tests;
