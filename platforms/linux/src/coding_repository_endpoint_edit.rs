// SPDX-License-Identifier: Apache-2.0
//! Exact endpoint approval handoff to the existing native worktree edit owner.

use super::*;
use agentmage_kernel_contracts::{
    AuthorizedWorkspaceHandle, ModelOperationControl, RuntimeApprovalChallenge,
    RuntimeApprovalDisposition, RuntimeApprovalPresentation, RuntimeApprovalResponse,
    RuntimeEndpointRunRequest, RuntimeOperationId, RuntimeSessionMode, RuntimeTurnId,
};
use agentmage_kernel_engine::{
    runtime_coordinator::{
        seal_runtime_approval_challenge, verify_runtime_approval_response,
        verify_runtime_endpoint_run_request,
    },
    runtime_loop::{runtime_action_id, runtime_tool_references},
};

#[path = "coding_repository_endpoint_boundary.rs"]
mod boundary;
pub use boundary::LinuxCodingEndpointEditBoundary;

const ENDPOINT_EDIT: &str = "agentmage.code.apply-prepared-edit";
const INPUT: &str = LINUX_CODING_PREPARED_INPUT_SCHEMA_JSON;
const OUTPUT: &str = r#"{"type":"object"}"#;

struct EndpointEditTool(ToolDefinition);
impl EndpointEditTool {
    fn new() -> Self {
        let schema = |id: &str, text: &str| SchemaReference {
            schema_id: SchemaId::from_raw(id),
            schema_version: 1,
            schema_sha256: hash(text.as_bytes()),
        };
        let operation = OperationBinding::new(GrantOperation::WorkspaceWrite);
        Self(ToolDefinition {
            schema_version: CONTRACT_SCHEMA_VERSION,
            tool_id: ToolId::from_raw(ENDPOINT_EDIT),
            tool_version: "1.0.0".into(),
            display_name: "Apply the complete prepared worktree diff".into(),
            description: "Propose only the original native preview; a separate exact protected decision is required".into(),
            input_schema: schema("agentmage.code.prepared-edit.input", INPUT),
            output_schema: schema("agentmage.code.prepared-edit.output", OUTPUT),
            risk_level: ToolRiskLevel::High,
            declared_effects: vec![operation],
            required_grant: RequiredGrantTemplate {
                operation,
                target_scope: "exact-service-created-worktree-diff".into(),
                single_use: true,
            },
            timeout_ms: LIFETIME_MS,
        })
    }
}
impl Tool for EndpointEditTool {
    fn definition(&self) -> &ToolDefinition {
        &self.0
    }
    fn validate_arguments(&self, bytes: &[u8]) -> Vec<ValidationIssue> {
        // Exact bytes are checked again against the privately held original
        // preview before a protected challenge can be constructed.
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
                code: "code.endpoint-edit.arguments".into(),
                severity: ValidationSeverity::Error,
                field_path: vec!["arguments".into()],
                message: "The complete original native preview is required".into(),
            }]
        }
    }
}

/// Exact single-operation catalog for trusted prepared-edit composition. This
/// grants no source read, model disclosure, write, command or replay authority.
pub fn linux_coding_endpoint_edit_registry() -> Result<ToolRegistry> {
    let mut registry = ToolRegistry::new();
    registry
        .register_tool(Box::new(EndpointEditTool::new()))
        .map_err(|_| err("code.endpoint-edit.catalog"))?;
    Ok(registry)
}

/// Non-cloneable handoff of an original prepared edit and its native owner.
/// Construction requires the live creation owner, not a root path or receipt.
/// Exact endpoint consent reaches the original kernel write transaction once.
/// This is a trusted library boundary, not an activated endpoint tool executor,
/// completion verifier, model admission, run journal or recovery mechanism.
pub struct LinuxCodingEndpointEdit {
    repository: LinuxCodingRepository,
    request: RuntimeEndpointRunRequest,
    challenge: RuntimeApprovalChallenge,
    call: ToolCall,
    preview: Value,
    spent: bool,
}

impl LinuxCodingRepository {
    /// Transfers the original prepared native edit into one exact endpoint
    /// decision. The trusted caller owns original run admission, instruction
    /// narrowing, source/disclosure consent and the continuously retained run
    /// control. No HTTP/model argument may construct these dependencies.
    /// All edited paths must be explicitly mutable and outside protected paths.
    /// Refusal returns the original owner intact; no effect or grant is issued.
    #[allow(clippy::result_large_err)]
    pub fn into_endpoint_edit(
        self,
        request: RuntimeEndpointRunRequest,
        turn_id: RuntimeTurnId,
        operation_id: RuntimeOperationId,
        call: ToolCall,
    ) -> std::result::Result<LinuxCodingEndpointEdit, (Self, LinuxCodingRepositoryError)> {
        let prepare = || {
            verify_runtime_endpoint_run_request(&request)
                .map_err(|_| err("code.endpoint-edit.binding"))?;
            let registry = linux_coding_endpoint_edit_registry()?;
            let definition = EndpointEditTool::new().0;
            let created = self.verify_created()?;
            let pending = self.edit.as_ref().ok_or(err("code.edit.choice-required"))?;
            self.verify_edit_scope(&pending.scope)?;
            // These are explicit canonical paths, never patterns whose scope
            // could be silently interpreted more narrowly than the request.
            for path in request
                .work_packet
                .mutable_files
                .iter()
                .chain(&request.work_packet.protected_files)
            {
                WorkspacePath::new(request.workspace_id.clone(), path.split('/'))
                    .map_err(|_| err("code.endpoint-edit.binding"))?;
            }
            let arguments = serde_json::to_vec(&json!({"preview":pending.preview}))
                .map_err(|_| err("code.endpoint-edit.binding"))?;
            if request.mode != RuntimeSessionMode::ControlledWrite
                || request.event_cursor.is_some()
                || request.workspace_id != *created.held.workspace_id()
                || request.visible_tools
                    != runtime_tool_references(&registry)
                        .map_err(|_| err("code.endpoint-edit.catalog"))?
                || pending.creation_receipt_sha256 != created.creation_receipt_sha256
                || registry.validate_arguments(&call).is_err()
                || call.schema_version != CONTRACT_SCHEMA_VERSION
                || call.tool_id != definition.tool_id
                || call.tool_version != definition.tool_version
                || call.arguments.schema != definition.input_schema
                || call.arguments.media_type != "application/json"
                || call.arguments.bytes != arguments
                || call.arguments.sha256 != hash(&arguments)
                || !(1..=request.limits.max_tool_calls)
                    .any(|i| call.action_id == runtime_action_id(&request.run_id, i))
                || pending.write.preview().files.iter().any(|path| {
                    !request.work_packet.mutable_files.contains(path)
                        || request.work_packet.protected_files.iter().any(|protected| {
                            path == protected || path.starts_with(&format!("{protected}/"))
                        })
                })
            {
                return Err(err("code.endpoint-edit.binding"));
            }
            let id = &pending.write.preview().change_set_id;
            let challenge = seal_runtime_approval_challenge(RuntimeApprovalChallenge {
                schema_version: CONTRACT_SCHEMA_VERSION,
                run_id: request.run_id.clone(),
                task_id: request.task.task_id.clone(),
                turn_id,
                operation_id,
                tool_call_id: call.tool_call_id.clone(),
                // These are the original native writer's eventual identities.
                // No grant exists until the original decide_edit dispatch.
                approval_id: ApprovalId::from_raw(format!("approval-{id}")),
                proposed_grant_id: GrantId::from_raw(format!("write-{id}")),
                operation: GrantOperation::WorkspaceWrite,
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
                expires_at_epoch_ms: pending.write.expires_at_epoch_ms(),
                challenge_sha256: "0".repeat(64),
            })
            .map_err(|_| err("code.endpoint-edit.binding"))?;
            Ok((challenge, pending.preview.clone()))
        };
        match prepare() {
            Ok((challenge, preview)) => Ok(LinuxCodingEndpointEdit {
                repository: self,
                request,
                challenge,
                call,
                preview,
                spent: false,
            }),
            Err(error) => Err((self, error)),
        }
    }
}

impl LinuxCodingEndpointEdit {
    /// Exact original sealed request. Endpoint selections never become profiles.
    #[must_use]
    pub const fn request(&self) -> &RuntimeEndpointRunRequest {
        &self.request
    }

    /// Original canonical call, including the complete native diff presentation.
    #[must_use]
    pub const fn call(&self) -> &ToolCall {
        &self.call
    }

    /// Complete protected challenge; every presentation field is sealed.
    #[must_use]
    pub const fn challenge(&self) -> &RuntimeApprovalChallenge {
        &self.challenge
    }

    /// Inspect the original native edit record after success, refusal or an
    /// uncertain reply. This cannot reconstruct an approval or creation owner.
    pub fn inspect(&self) -> Result<Value> {
        self.repository.inspect_edit()
    }

    /// Returns the same live owner for separately approved later work. Consumes
    /// this handle and abandons its pending edit, including after stop or denial;
    /// no old preview can be executed through the returned owner. Existing native
    /// results and successful edit lineage are preserved. This is process-local
    /// continuation, never reconstruction from disk, paths or old receipts.
    #[must_use]
    pub fn into_repository(mut self) -> LinuxCodingRepository {
        self.repository.edit = None;
        self.repository
    }

    /// Validates a response to the original challenge, then consumes that
    /// decision before observing original run control or dispatching. The native
    /// owner alone issues/consumes grants and reconciles writes. Stop, error and
    /// panic never reopen this handle. A rejected response has no effect.
    /// The bounded atomic transaction is non-interruptible after admission;
    /// stopping afterwards cannot establish no effect or replay eligibility.
    pub fn decide(
        &mut self,
        response: &RuntimeApprovalResponse,
        now: u64,
        control: &dyn ModelOperationControl,
    ) -> Result<Value> {
        if self.spent || response.disposition == RuntimeApprovalDisposition::Narrow {
            return Err(err("code.endpoint-edit.spent-or-invalid"));
        }
        verify_runtime_approval_response(&self.challenge, response, now)
            .map_err(|_| err("code.endpoint-edit.decision"))?;
        self.spent = true;
        control
            .remaining_ms()
            .map_err(|_| err("code.endpoint-edit.stopped"))?;
        if self.repository.inspect_edit()?["pending"] != self.preview {
            return Err(err("code.endpoint-edit.stale"));
        }
        control
            .remaining_ms()
            .map_err(|_| err("code.endpoint-edit.stopped"))?;
        self.repository.decide_edit(
            &self.challenge.preview_sha256,
            response.disposition == RuntimeApprovalDisposition::Allow,
            now,
        )
    }
}

#[cfg(test)]
#[path = "coding_repository_endpoint_edit_tests.rs"]
mod tests;
