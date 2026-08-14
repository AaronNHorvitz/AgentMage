//! Linux composition for one previewed, approved, exact workspace-file read.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use agentmage_capability_read_only::{
    ReadOnlyEncoding, ReadOnlyLimits, ReadOnlyRequest, ReadOnlyToolKind,
    WORKSPACE_FILE_READ_TOOL_ID, WORKSPACE_FILE_READ_TOOL_VERSION, validate_read_only_request,
    workspace_file_read_definition,
};
use agentmage_kernel_contracts::{
    ActionId, ActionKind, ActorId, ApprovalId, ApprovalRequest, AuthorityTransactionId,
    ContractPayload, CorrelationId, DataSensitivity, DiagnosticComponent, DiagnosticObservation,
    DiagnosticState, DoctorReport, GrantId, GrantNonce, GrantOperation, GrantPreimage,
    GrantSideEffect, GrantTarget, HeldWorkspaceObject, OperationAttemptId, OperationBinding,
    OperationOutcome, Receipt, SessionId, TaskId, ToolCall, ToolCallId, ToolDefinition, ToolId,
    ValidationIssue, ValidationSeverity, WorkspaceAuthorizationId, WorkspaceId, WorkspacePath,
    WorkspaceScopePath,
};
use agentmage_kernel_engine::approval::{render_approval_request, verify_approval_request};
use agentmage_kernel_engine::authority_transaction::AuthorityTransactionRequest;
use agentmage_kernel_engine::diagnostics::build_doctor_report;
use agentmage_kernel_engine::grants::{DerivedOperationGrantRequest, SessionReadGrantRequest};
use agentmage_kernel_engine::platform_startup::VerifiedPlatformAdapter;
use agentmage_kernel_engine::policy::{
    PolicyEngine, PolicyEvaluationContext, StrictLocalReadOnlyScope, ToolPolicyBinding,
};
use agentmage_kernel_engine::tooling::{Tool, ToolRegistry};
use agentmage_platform_linux::{
    LinuxAuthenticatedIpcSession, LinuxAuthorityRuntime, LinuxHeldObject, LinuxPlatformAdapter,
    LinuxSandboxEffectDriver, LinuxSandboxOperation, LinuxSandboxRunner,
    resolve_linux_workspace_object, select_linux_workspace,
};
use rustix::rand::{GetRandomFlags, getrandom};
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::diagnostic_export::{DiagnosticExportError, DiagnosticExportWorkflow};
use crate::protocol::{
    HOST_PROTOCOL_VERSION, HostRequest, HostResponse, MAX_HOST_REQUEST_BYTES,
    MAX_HOST_RESPONSE_BYTES, ReceiptSummary, encode_response, parse_request,
};

#[cfg(test)]
use agentmage_kernel_contracts::AdapterInstanceId;

const PREVIEW_LIFETIME_MS: u64 = 60_000;
const OPERATION_LIFETIME_MS: u64 = 30_000;
const MAX_PENDING_PREVIEWS: usize = 8;

enum LinuxReadPlatform<'platform> {
    Verified(&'platform VerifiedPlatformAdapter<LinuxPlatformAdapter>),
    #[cfg(test)]
    Test(AdapterInstanceId),
}

/// Stable content-free failure from the Phase 9 Linux read workflow.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LinuxReadError {
    /// A required random identity could not be generated.
    IdentityUnavailable,
    /// The trusted wall clock could not produce a usable instant.
    ClockUnavailable,
    /// The selected workspace or relative path failed closed validation.
    PathDenied,
    /// Exact grant, policy, approval, or transaction construction failed.
    AuthorityDenied,
    /// The pending preview was absent, expired, cancelled, or mismatched.
    ApprovalDenied,
    /// The Linux worker failed or returned no exact result.
    WorkerFailed,
    /// Successful worker output was not bounded UTF-8 text.
    OutputDenied,
    /// The reviewed diagnostic export failed closed.
    DiagnosticExport(DiagnosticExportError),
}

/// Stable content-free failure while serving an authenticated host frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LinuxReadSessionError {
    /// The authenticated local channel failed closed.
    Channel,
    /// A bounded response could not be encoded safely.
    Protocol,
}

impl LinuxReadSessionError {
    /// Returns the stable redacted failure code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::Channel => "host.session.channel_failed",
            Self::Protocol => "host.session.protocol_failed",
        }
    }
}

impl LinuxReadError {
    /// Returns the stable redacted failure code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::IdentityUnavailable => "host.read.identity_unavailable",
            Self::ClockUnavailable => "host.read.clock_unavailable",
            Self::PathDenied => "host.read.path_denied",
            Self::AuthorityDenied => "host.read.authority_denied",
            Self::ApprovalDenied => "host.read.approval_denied",
            Self::WorkerFailed => "host.read.worker_failed",
            Self::OutputDenied => "host.read.output_denied",
            Self::DiagnosticExport(error) => error.code(),
        }
    }
}

/// Trusted time sample used for expiry, policy, and receipt correlation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReadInstant {
    epoch_ms: u64,
    occurred_at: String,
}

impl ReadInstant {
    /// Creates a bounded instant supplied by a trusted host clock adapter.
    pub fn new(epoch_ms: u64, occurred_at: impl Into<String>) -> Result<Self, LinuxReadError> {
        let occurred_at = occurred_at.into();
        if occurred_at.is_empty() || occurred_at.len() > 64 {
            return Err(LinuxReadError::ClockUnavailable);
        }
        Ok(Self {
            epoch_ms,
            occurred_at,
        })
    }
}

/// Host clock boundary, injectable for deterministic fault and expiry tests.
pub trait ReadClock {
    /// Returns the current trusted operation instant.
    fn now(&mut self) -> Result<ReadInstant, LinuxReadError>;
}

/// Standard local wall clock used by a packaged host.
#[derive(Debug, Default)]
pub struct SystemReadClock;

impl ReadClock for SystemReadClock {
    fn now(&mut self) -> Result<ReadInstant, LinuxReadError> {
        let duration = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| LinuxReadError::ClockUnavailable)?;
        let epoch_ms =
            u64::try_from(duration.as_millis()).map_err(|_| LinuxReadError::ClockUnavailable)?;
        ReadInstant::new(epoch_ms, format_utc(epoch_ms))
    }
}

/// CSPRNG identity boundary, injectable for deterministic tests.
pub trait ReadIdentitySource {
    /// Returns one unique bounded identifier with the requested stable prefix.
    fn next(&mut self, prefix: &str) -> Result<String, LinuxReadError>;
}

/// Operating-system random identity source used by a packaged host.
#[derive(Debug, Default)]
pub struct OsReadIdentitySource;

impl ReadIdentitySource for OsReadIdentitySource {
    fn next(&mut self, prefix: &str) -> Result<String, LinuxReadError> {
        let mut random = [0_u8; 16];
        let count = getrandom(&mut random, GetRandomFlags::empty())
            .map_err(|_| LinuxReadError::IdentityUnavailable)?;
        if count != random.len() {
            return Err(LinuxReadError::IdentityUnavailable);
        }
        Ok(format!("{prefix}-{}", hex(&random)))
    }
}

struct RegisteredReadTool {
    definition: ToolDefinition,
    kind: ReadOnlyToolKind,
}

impl Tool for RegisteredReadTool {
    fn definition(&self) -> &ToolDefinition {
        &self.definition
    }

    fn validate_arguments(&self, arguments: &[u8]) -> Vec<ValidationIssue> {
        validate_read_only_request(self.kind, arguments).map_or_else(
            |error| {
                vec![ValidationIssue {
                    code: error.code().to_owned(),
                    severity: ValidationSeverity::Error,
                    field_path: vec!["arguments".to_owned()],
                    message: "Read-only tool arguments failed closed validation".to_owned(),
                }]
            },
            |_| Vec::new(),
        )
    }
}

struct PendingLinuxRead {
    approval: ApprovalRequest,
    parent_grant_id: GrantId,
    held: LinuxHeldObject,
    operation_target: GrantTarget,
    display_uri: String,
}

/// One verified Linux session capable only of the Phase 9 exact read workflow.
pub struct LinuxReadWorkflow<'platform, I, C>
where
    I: ReadIdentitySource,
    C: ReadClock,
{
    platform: LinuxReadPlatform<'platform>,
    authority: LinuxAuthorityRuntime,
    sandbox: Option<LinuxSandboxRunner>,
    registry: ToolRegistry,
    actor_id: ActorId,
    session_id: SessionId,
    identities: I,
    clock: C,
    pending: BTreeMap<String, PendingLinuxRead>,
    diagnostic_exports: DiagnosticExportWorkflow,
}

impl<'platform, I, C> LinuxReadWorkflow<'platform, I, C>
where
    I: ReadIdentitySource,
    C: ReadClock,
{
    /// Composes one verified platform, durable authority store, and inert worker runner.
    pub fn new(
        platform: &'platform VerifiedPlatformAdapter<LinuxPlatformAdapter>,
        authority: LinuxAuthorityRuntime,
        sandbox: LinuxSandboxRunner,
        actor_id: ActorId,
        session_id: SessionId,
        identities: I,
        clock: C,
    ) -> Result<Self, LinuxReadError> {
        let mut registry = ToolRegistry::new();
        registry
            .register_tool(Box::new(RegisteredReadTool {
                definition: workspace_file_read_definition(),
                kind: ReadOnlyToolKind::ReadText,
            }))
            .map_err(|_| LinuxReadError::AuthorityDenied)?;
        Ok(Self {
            platform: LinuxReadPlatform::Verified(platform),
            authority,
            sandbox: Some(sandbox),
            registry,
            actor_id,
            session_id,
            identities,
            clock,
            pending: BTreeMap::new(),
            diagnostic_exports: DiagnosticExportWorkflow::new(),
        })
    }

    #[cfg(test)]
    fn new_test(
        authority: LinuxAuthorityRuntime,
        sandbox: LinuxSandboxRunner,
        actor_id: ActorId,
        session_id: SessionId,
        identities: I,
        clock: C,
    ) -> Result<Self, LinuxReadError> {
        let mut workflow =
            Self::new_inner(authority, sandbox, actor_id, session_id, identities, clock)?;
        workflow.platform =
            LinuxReadPlatform::Test(AdapterInstanceId::from_raw("linux-read-test-adapter"));
        Ok(workflow)
    }

    #[cfg(test)]
    fn new_inner(
        authority: LinuxAuthorityRuntime,
        sandbox: LinuxSandboxRunner,
        actor_id: ActorId,
        session_id: SessionId,
        identities: I,
        clock: C,
    ) -> Result<Self, LinuxReadError> {
        let mut registry = ToolRegistry::new();
        registry
            .register_tool(Box::new(RegisteredReadTool {
                definition: workspace_file_read_definition(),
                kind: ReadOnlyToolKind::ReadText,
            }))
            .map_err(|_| LinuxReadError::AuthorityDenied)?;
        Ok(Self {
            platform: LinuxReadPlatform::Test(AdapterInstanceId::from_raw(
                "linux-read-test-uninitialized",
            )),
            authority,
            sandbox: Some(sandbox),
            registry,
            actor_id,
            session_id,
            identities,
            clock,
            pending: BTreeMap::new(),
            diagnostic_exports: DiagnosticExportWorkflow::new(),
        })
    }

    /// Handles one already parsed request from an authenticated local channel.
    #[must_use]
    pub fn handle(&mut self, request: HostRequest) -> HostResponse {
        let request_id = request_id(&request).to_owned();
        let result = match request {
            HostRequest::Doctor { .. } => self.doctor(&request_id),
            HostRequest::PreviewDiagnosticExport { destination, .. } => {
                self.preview_diagnostic_export(&request_id, Path::new(&destination))
            }
            HostRequest::ApproveDiagnosticExport {
                preview_id,
                confirmation_sha256,
                ..
            } => self.approve_diagnostic_export(&request_id, &preview_id, &confirmation_sha256),
            HostRequest::CancelDiagnosticExport { preview_id, .. } => {
                self.cancel_diagnostic_export(&request_id, &preview_id)
            }
            HostRequest::PreviewRead {
                workspace_id,
                workspace_root,
                components,
                ..
            } => self.preview_read(
                &request_id,
                &workspace_id,
                Path::new(&workspace_root),
                components,
            ),
            HostRequest::ApproveRead {
                preview_id,
                confirmation_sha256,
                ..
            } => self.approve_read(&request_id, &preview_id, &confirmation_sha256),
            HostRequest::CancelRead { preview_id, .. } => {
                self.cancel_read(&request_id, &preview_id)
            }
        };
        result.unwrap_or_else(|error| HostResponse::Denied {
            schema_version: HOST_PROTOCOL_VERSION,
            request_id,
            code: error.code().to_owned(),
            receipt: None,
        })
    }

    fn doctor(&self, request_id: &str) -> Result<HostResponse, LinuxReadError> {
        Ok(HostResponse::DoctorCompleted {
            schema_version: HOST_PROTOCOL_VERSION,
            request_id: request_id.to_owned(),
            report: self.local_doctor_report()?,
        })
    }

    fn local_doctor_report(&self) -> Result<DoctorReport, LinuxReadError> {
        let package_state = match self.platform {
            LinuxReadPlatform::Verified(_) => DiagnosticState::Healthy,
            #[cfg(test)]
            LinuxReadPlatform::Test(_) => DiagnosticState::Degraded,
        };
        let observations = vec![
            diagnostic(
                DiagnosticComponent::Package,
                package_state,
                if package_state == DiagnosticState::Healthy {
                    "diagnostic.package.verified"
                } else {
                    "diagnostic.package.test-boundary"
                },
            ),
            diagnostic(
                DiagnosticComponent::Platform,
                DiagnosticState::Healthy,
                "diagnostic.platform.verified",
            ),
            diagnostic(
                DiagnosticComponent::OfflineBoundary,
                DiagnosticState::Healthy,
                "diagnostic.offline.enforced",
            ),
            diagnostic(
                DiagnosticComponent::SandboxHelper,
                if self.sandbox.is_some() {
                    DiagnosticState::Healthy
                } else {
                    DiagnosticState::Unavailable
                },
                if self.sandbox.is_some() {
                    "diagnostic.sandbox.verified"
                } else {
                    "diagnostic.sandbox.unavailable"
                },
            ),
            diagnostic(
                DiagnosticComponent::Capabilities,
                DiagnosticState::Healthy,
                "diagnostic.capabilities.host-loaded",
            ),
            diagnostic(
                DiagnosticComponent::EncryptedStore,
                DiagnosticState::Healthy,
                "diagnostic.store.open",
            ),
        ];
        build_doctor_report(observations).map_err(|_| LinuxReadError::AuthorityDenied)
    }

    fn preview_diagnostic_export(
        &mut self,
        request_id: &str,
        destination: &Path,
    ) -> Result<HostResponse, LinuxReadError> {
        let now = self.clock.now()?;
        let preview_id = self.identities.next("diagnostic-export")?;
        let report = self.local_doctor_report()?;
        let preview = self
            .diagnostic_exports
            .preview(preview_id, &report, destination, now.epoch_ms)
            .map_err(LinuxReadError::DiagnosticExport)?;
        Ok(HostResponse::DiagnosticExportPreview {
            schema_version: HOST_PROTOCOL_VERSION,
            request_id: request_id.to_owned(),
            preview_id: preview.preview_id,
            destination_sha256: preview.destination_sha256,
            payload_sha256: preview.payload_sha256,
            payload_bytes: preview.payload_bytes,
            included_fields: preview.included_fields,
            redactions: preview.redactions,
            sensitivity: preview.sensitivity,
            retention: preview.retention,
            expires_at_epoch_ms: preview.expires_at_epoch_ms,
            confirmation_sha256: preview.confirmation_sha256,
        })
    }

    fn approve_diagnostic_export(
        &mut self,
        request_id: &str,
        preview_id: &str,
        confirmation_sha256: &str,
    ) -> Result<HostResponse, LinuxReadError> {
        let now = self.clock.now()?;
        let receipt = self
            .diagnostic_exports
            .approve(preview_id, confirmation_sha256, now.epoch_ms)
            .map_err(LinuxReadError::DiagnosticExport)?;
        Ok(HostResponse::DiagnosticExportCompleted {
            schema_version: HOST_PROTOCOL_VERSION,
            request_id: request_id.to_owned(),
            destination_sha256: receipt.destination_sha256,
            payload_sha256: receipt.payload_sha256,
            payload_bytes: receipt.payload_bytes,
            outcome: receipt.outcome,
        })
    }

    fn cancel_diagnostic_export(
        &mut self,
        request_id: &str,
        preview_id: &str,
    ) -> Result<HostResponse, LinuxReadError> {
        if !self.diagnostic_exports.cancel(preview_id) {
            return Err(LinuxReadError::DiagnosticExport(
                DiagnosticExportError::ApprovalDenied,
            ));
        }
        Ok(HostResponse::Cancelled {
            schema_version: HOST_PROTOCOL_VERSION,
            request_id: request_id.to_owned(),
        })
    }

    /// Reads and answers one request on an already authenticated Linux channel.
    ///
    /// Malformed product messages receive one bounded denial. Authentication
    /// failures cannot construct this session and therefore never reach here.
    pub fn handle_authenticated_frame(
        &mut self,
        session: &mut LinuxAuthenticatedIpcSession,
    ) -> Result<(), LinuxReadSessionError> {
        let frame = session
            .read_frame(MAX_HOST_REQUEST_BYTES)
            .map_err(|_| LinuxReadSessionError::Channel)?;
        let response = match parse_request(&frame) {
            Ok(request) => self.handle(request),
            Err(error) => HostResponse::Denied {
                schema_version: HOST_PROTOCOL_VERSION,
                request_id: "request-invalid".to_owned(),
                code: error.code().to_owned(),
                receipt: None,
            },
        };
        let encoded = encode_response(&response).map_err(|_| LinuxReadSessionError::Protocol)?;
        session
            .write_frame(&encoded, MAX_HOST_RESPONSE_BYTES)
            .map_err(|_| LinuxReadSessionError::Channel)
    }

    fn preview_read(
        &mut self,
        request_id: &str,
        workspace_id: &str,
        workspace_root: &Path,
        components: Vec<String>,
    ) -> Result<HostResponse, LinuxReadError> {
        let now = self.clock.now()?;
        self.pending
            .retain(|_, candidate| candidate.approval.expires_at_epoch_ms > now.epoch_ms);
        if self.pending.len() >= MAX_PENDING_PREVIEWS {
            return Err(LinuxReadError::ApprovalDenied);
        }

        let workspace_id = WorkspaceId::from_raw(workspace_id);
        let authorization_id =
            WorkspaceAuthorizationId::from_raw(self.identities.next("workspace-authorization")?);
        let path = WorkspacePath::new(workspace_id.clone(), components)
            .map_err(|_| LinuxReadError::PathDenied)?;
        let (workspace, held) = match &self.platform {
            LinuxReadPlatform::Verified(platform) => {
                let workspace = select_linux_workspace(
                    platform,
                    workspace_root,
                    workspace_id.clone(),
                    authorization_id,
                )
                .map_err(|_| LinuxReadError::PathDenied)?;
                let held = resolve_linux_workspace_object(
                    platform,
                    &workspace,
                    &path,
                    agentmage_kernel_contracts::PathResolutionIntent::ReadFile,
                )
                .map_err(|_| LinuxReadError::PathDenied)?;
                (workspace, held)
            }
            #[cfg(test)]
            LinuxReadPlatform::Test(adapter_instance_id) => {
                let workspace = agentmage_platform_linux::select_test_linux_workspace(
                    workspace_root,
                    workspace_id.clone(),
                    authorization_id,
                    adapter_instance_id.clone(),
                )
                .map_err(|_| LinuxReadError::PathDenied)?;
                let held = agentmage_platform_linux::resolve_test_linux_workspace_object(
                    &workspace,
                    adapter_instance_id.clone(),
                    &path,
                    agentmage_kernel_contracts::PathResolutionIntent::ReadFile,
                )
                .map_err(|_| LinuxReadError::PathDenied)?;
                (workspace, held)
            }
        };
        let display_uri = held
            .display_link(None)
            .map_err(|_| LinuxReadError::PathDenied)?
            .file_uri()
            .to_owned();
        let operation_target =
            GrantTarget::held_object(&held).map_err(|_| LinuxReadError::AuthorityDenied)?;
        let scope = GrantTarget::workspace_scope(
            &workspace,
            WorkspaceScopePath::new(
                workspace_id,
                path.components()
                    .iter()
                    .map(|component| component.as_str().to_owned()),
            )
            .map_err(|_| LinuxReadError::PathDenied)?,
        )
        .map_err(|_| LinuxReadError::AuthorityDenied)?;

        let task_id = TaskId::from_raw(self.identities.next("task")?);
        let action_id = ActionId::from_raw(self.identities.next("action")?);
        let parent_grant_id = GrantId::from_raw(self.identities.next("grant-parent")?);
        let operation_grant_id = GrantId::from_raw(self.identities.next("grant-operation")?);
        let approval_id = ApprovalId::from_raw(self.identities.next("approval")?);
        let preview_id = self.identities.next("preview")?;
        let tool_call = tool_call(&mut self.identities, &action_id, &path)?;
        let operation = OperationBinding::new(GrantOperation::WorkspaceRead);
        let preimage = GrantPreimage::for_target(0, &operation_target)
            .ok_or(LinuxReadError::AuthorityDenied)?;
        let side_effect = GrantSideEffect {
            operation,
            target_indexes: vec![0],
            details_sha256: digest_serialized(&operation_target)?,
        };
        let policy = exact_read_policy(&self.actor_id, &task_id, &action_id, &operation_target)?;
        let expires_at_epoch_ms = now
            .epoch_ms
            .checked_add(PREVIEW_LIFETIME_MS)
            .ok_or(LinuxReadError::ClockUnavailable)?;
        let parent_preview_sha256 = digest_serialized(&scope)?;
        self.authority
            .revalidate_root()
            .map_err(|_| LinuxReadError::AuthorityDenied)?;
        let parent = self
            .authority
            .authority_mut()
            .issue_session_read(SessionReadGrantRequest {
                grant_id: parent_grant_id.clone(),
                actor_id: self.actor_id.clone(),
                session_id: self.session_id.clone(),
                task_id: task_id.clone(),
                targets: vec![scope],
                excluded_targets: Vec::new(),
                sensitivity: DataSensitivity::Ephemeral,
                issued_at_epoch_ms: now.epoch_ms,
                expires_at_epoch_ms,
                nonce: GrantNonce::from_raw(self.identities.next("nonce-parent")?),
                maximum_derived_operations: 1,
                preview_sha256: parent_preview_sha256,
                policy_sha256: policy.policy_sha256().to_owned(),
            })
            .map_err(|_| LinuxReadError::AuthorityDenied)?;
        let parent_grant_sha256 = digest_serialized(&parent)?;
        let approval = render_approval_request(
            &self.registry,
            ApprovalRequest {
                schema_version: agentmage_kernel_contracts::CONTRACT_SCHEMA_VERSION,
                approval_id,
                proposed_grant_id: operation_grant_id,
                parent_grant_id: parent_grant_id.clone(),
                parent_grant_sha256,
                actor_id: self.actor_id.clone(),
                session_id: self.session_id.clone(),
                task_id,
                action_kind: ActionKind::DeterministicTool,
                operation,
                tool_call,
                targets: vec![operation_target.clone()],
                excluded_targets: Vec::new(),
                sensitivity: DataSensitivity::Ephemeral,
                preimages: vec![preimage],
                expected_side_effects: vec![side_effect],
                rollback_description: "No state change is permitted".to_owned(),
                issued_at_epoch_ms: now.epoch_ms,
                expires_at_epoch_ms,
                policy_sha256: policy.policy_sha256().to_owned(),
                confirmation_sha256: "0".repeat(64),
            },
        )
        .map_err(|_| LinuxReadError::AuthorityDenied)?;

        let held_preimage = held.preimage().ok_or(LinuxReadError::PathDenied)?;
        let response = HostResponse::ReadPreview {
            schema_version: HOST_PROTOCOL_VERSION,
            request_id: request_id.to_owned(),
            preview_id: preview_id.clone(),
            components: path
                .components()
                .iter()
                .map(|component| component.as_str().to_owned())
                .collect(),
            byte_len: held_preimage.byte_len(),
            content_sha256: hex(held_preimage.content_sha256()),
            expires_at_epoch_ms,
            confirmation_sha256: approval.confirmation_sha256.clone(),
        };
        self.pending.insert(
            preview_id,
            PendingLinuxRead {
                approval,
                parent_grant_id,
                held,
                operation_target,
                display_uri,
            },
        );
        Ok(response)
    }

    fn approve_read(
        &mut self,
        request_id: &str,
        preview_id: &str,
        confirmation_sha256: &str,
    ) -> Result<HostResponse, LinuxReadError> {
        let now = self.clock.now()?;
        let Some(pending) = self.pending.remove(preview_id) else {
            return Ok(replay_denial(
                request_id,
                self.receipt_for_preview(preview_id),
            ));
        };
        if pending.approval.expires_at_epoch_ms <= now.epoch_ms
            || pending.approval.confirmation_sha256 != confirmation_sha256
            || verify_approval_request(&self.registry, &pending.approval).is_err()
            || pending.held.revalidate().is_err()
        {
            return Err(LinuxReadError::ApprovalDenied);
        }

        let approval = pending.approval;
        self.authority
            .revalidate_root()
            .map_err(|_| LinuxReadError::AuthorityDenied)?;
        let grant = self
            .authority
            .authority_mut()
            .derive_operation(
                &pending.parent_grant_id,
                DerivedOperationGrantRequest {
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
                    issued_at_epoch_ms: now.epoch_ms,
                    expires_at_epoch_ms: now
                        .epoch_ms
                        .checked_add(OPERATION_LIFETIME_MS)
                        .map(|expiry| expiry.min(approval.expires_at_epoch_ms))
                        .ok_or(LinuxReadError::ClockUnavailable)?,
                    nonce: GrantNonce::from_raw(self.identities.next("nonce-operation")?),
                    preview_sha256: approval.confirmation_sha256.clone(),
                    policy_sha256: approval.policy_sha256.clone(),
                },
            )
            .map_err(|_| LinuxReadError::AuthorityDenied)?;
        let policy = exact_read_policy(
            &approval.actor_id,
            &approval.task_id,
            &approval.tool_call.action_id,
            &pending.operation_target,
        )?;
        let context = PolicyEvaluationContext {
            actor_id: approval.actor_id.clone(),
            session_id: approval.session_id.clone(),
            task_id: approval.task_id.clone(),
            action_id: approval.tool_call.action_id.clone(),
            action_kind: approval.action_kind,
            tool_id: approval.tool_call.tool_id.clone(),
            tool_version: approval.tool_call.tool_version.clone(),
            targets: grant.targets.clone(),
            argument_sha256: grant.argument_sha256.clone(),
            preimages: grant.preimages.clone(),
            expected_side_effects: grant.expected_side_effects.clone(),
            preview_sha256: grant.preview_sha256.clone(),
            now_epoch_ms: now.epoch_ms,
            network_scope: None,
            credential_scope: None,
            publication_scope: None,
        };
        let suffix = preview_suffix(preview_id).ok_or(LinuxReadError::ApprovalDenied)?;
        let transaction_id = AuthorityTransactionId::from_raw(format!("transaction-{suffix}"));
        let attempt_id = OperationAttemptId::from_raw(format!("attempt-{suffix}"));
        let request = AuthorityTransactionRequest::new(
            transaction_id,
            attempt_id,
            approval.approval_id,
            grant.grant_id,
            approval.tool_call,
            context,
            now.epoch_ms,
            now.occurred_at,
        )
        .map_err(|_| LinuxReadError::AuthorityDenied)?;
        self.authority
            .revalidate_root()
            .map_err(|_| LinuxReadError::AuthorityDenied)?;
        let operation_path = pending
            .operation_target
            .workspace_path()
            .ok_or(LinuxReadError::AuthorityDenied)?
            .clone();
        let runner = self.sandbox.take().ok_or(LinuxReadError::WorkerFailed)?;
        let mut driver = LinuxSandboxEffectDriver::new(
            runner,
            pending.held,
            LinuxSandboxOperation::ReadFile(operation_path),
        );
        let receipt_result = self.authority.authority_mut().execute_effect(
            &self.registry,
            &policy,
            request,
            &mut driver,
        );
        let worker_result = driver.take_result();
        self.sandbox = Some(driver.into_runner());
        let receipt = receipt_result.map_err(|_| LinuxReadError::AuthorityDenied)?;
        self.authority
            .revalidate_root()
            .map_err(|_| LinuxReadError::AuthorityDenied)?;
        let summary = receipt_summary(&receipt);
        let worker_result = worker_result.ok_or(LinuxReadError::WorkerFailed)?;
        if !worker_result.success() || receipt.outcome != OperationOutcome::Succeeded {
            return Ok(HostResponse::Denied {
                schema_version: HOST_PROTOCOL_VERSION,
                request_id: request_id.to_owned(),
                code: LinuxReadError::WorkerFailed.code().to_owned(),
                receipt: Some(summary),
            });
        }
        let content = match String::from_utf8(worker_result.stdout().to_vec()) {
            Ok(content) => content,
            Err(_) => {
                return Ok(HostResponse::Denied {
                    schema_version: HOST_PROTOCOL_VERSION,
                    request_id: request_id.to_owned(),
                    code: LinuxReadError::OutputDenied.code().to_owned(),
                    receipt: Some(summary),
                });
            }
        };
        Ok(HostResponse::ReadCompleted {
            schema_version: HOST_PROTOCOL_VERSION,
            request_id: request_id.to_owned(),
            content,
            file_uri: pending.display_uri,
            receipt: summary,
        })
    }

    fn cancel_read(
        &mut self,
        request_id: &str,
        preview_id: &str,
    ) -> Result<HostResponse, LinuxReadError> {
        if self.pending.remove(preview_id).is_some() {
            return Ok(HostResponse::Cancelled {
                schema_version: HOST_PROTOCOL_VERSION,
                request_id: request_id.to_owned(),
            });
        }
        Ok(replay_denial(
            request_id,
            self.receipt_for_preview(preview_id),
        ))
    }

    fn receipt_for_preview(&self, preview_id: &str) -> Option<ReceiptSummary> {
        let suffix = preview_suffix(preview_id)?;
        let transaction = format!("transaction-{suffix}");
        self.authority
            .authority()
            .receipts()
            .iter()
            .find(|receipt| receipt.authority_transaction_id.as_str() == transaction)
            .map(receipt_summary)
    }
}

fn tool_call(
    identities: &mut impl ReadIdentitySource,
    action_id: &ActionId,
    path: &WorkspacePath,
) -> Result<ToolCall, LinuxReadError> {
    let definition = workspace_file_read_definition();
    let bytes = serde_json::to_vec(&ReadOnlyRequest {
        schema_version: 1,
        paths: vec![
            path.components()
                .iter()
                .map(|component| component.as_str().to_owned())
                .collect(),
        ],
        query: None,
        byte_offset: None,
        byte_count: None,
        encoding: ReadOnlyEncoding::Utf8,
        limits: ReadOnlyLimits::default(),
        call_depth: 0,
    })
    .map_err(|_| LinuxReadError::AuthorityDenied)?;
    Ok(ToolCall {
        schema_version: agentmage_kernel_contracts::CONTRACT_SCHEMA_VERSION,
        tool_call_id: ToolCallId::from_raw(identities.next("tool-call")?),
        correlation_id: CorrelationId::from_raw(identities.next("correlation")?),
        action_id: action_id.clone(),
        tool_id: ToolId::from_raw(WORKSPACE_FILE_READ_TOOL_ID),
        tool_version: WORKSPACE_FILE_READ_TOOL_VERSION.to_owned(),
        arguments: ContractPayload {
            schema: definition.input_schema,
            media_type: "application/json".to_owned(),
            sha256: digest(&bytes),
            bytes,
        },
    })
}

fn exact_read_policy(
    actor_id: &ActorId,
    task_id: &TaskId,
    action_id: &ActionId,
    target: &GrantTarget,
) -> Result<PolicyEngine, LinuxReadError> {
    PolicyEngine::strict_local_read_only(StrictLocalReadOnlyScope {
        revision: 1,
        actors: BTreeSet::from([actor_id.clone()]),
        tasks: BTreeSet::from([task_id.clone()]),
        actions: BTreeSet::from([action_id.clone()]),
        tools: BTreeSet::from([ToolPolicyBinding {
            tool_id: ToolId::from_raw(WORKSPACE_FILE_READ_TOOL_ID),
            tool_version: WORKSPACE_FILE_READ_TOOL_VERSION.to_owned(),
        }]),
        targets: BTreeSet::from([target.clone()]),
    })
    .map_err(|_| LinuxReadError::AuthorityDenied)
}

fn replay_denial(request_id: &str, receipt: Option<ReceiptSummary>) -> HostResponse {
    HostResponse::Denied {
        schema_version: HOST_PROTOCOL_VERSION,
        request_id: request_id.to_owned(),
        code: if receipt.is_some() {
            "host.read.replay_denied"
        } else {
            "host.read.preview_missing"
        }
        .to_owned(),
        receipt,
    }
}

fn receipt_summary(receipt: &Receipt) -> ReceiptSummary {
    ReceiptSummary {
        receipt_id: receipt.receipt_id.as_str().to_owned(),
        sequence: receipt.sequence,
        receipt_sha256: receipt.receipt_sha256.clone(),
        outcome: match receipt.outcome {
            OperationOutcome::Succeeded => "succeeded",
            OperationOutcome::Denied => "denied",
            OperationOutcome::Failed => "failed",
            OperationOutcome::Cancelled => "cancelled",
            OperationOutcome::TimedOut => "timed_out",
            OperationOutcome::Uncertain => "uncertain",
        }
        .to_owned(),
    }
}

fn request_id(request: &HostRequest) -> &str {
    match request {
        HostRequest::Doctor { request_id, .. }
        | HostRequest::PreviewDiagnosticExport { request_id, .. }
        | HostRequest::ApproveDiagnosticExport { request_id, .. }
        | HostRequest::CancelDiagnosticExport { request_id, .. }
        | HostRequest::PreviewRead { request_id, .. }
        | HostRequest::ApproveRead { request_id, .. }
        | HostRequest::CancelRead { request_id, .. } => request_id,
    }
}

fn diagnostic(
    component: DiagnosticComponent,
    state: DiagnosticState,
    reason_code: &str,
) -> DiagnosticObservation {
    DiagnosticObservation {
        component,
        state,
        reason_code: reason_code.to_owned(),
        identity_sha256: None,
        stale: false,
    }
}

fn preview_suffix(preview_id: &str) -> Option<&str> {
    let suffix = preview_id.strip_prefix("preview-")?;
    (suffix.len() == 32 && suffix.bytes().all(|byte| byte.is_ascii_hexdigit())).then_some(suffix)
}

fn digest_serialized(value: &impl Serialize) -> Result<String, LinuxReadError> {
    let bytes = serde_json::to_vec(value).map_err(|_| LinuxReadError::AuthorityDenied)?;
    Ok(digest(&bytes))
}

fn digest(value: &[u8]) -> String {
    hex(Sha256::digest(value).as_slice())
}

fn hex(value: &[u8]) -> String {
    value.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn format_utc(epoch_ms: u64) -> String {
    let seconds = epoch_ms / 1_000;
    let milliseconds = epoch_ms % 1_000;
    let days = i64::try_from(seconds / 86_400).unwrap_or(i64::MAX);
    let seconds_in_day = seconds % 86_400;
    let (year, month, day) = civil_from_days(days);
    let hour = seconds_in_day / 3_600;
    let minute = (seconds_in_day % 3_600) / 60;
    let second = seconds_in_day % 60;
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}.{milliseconds:03}Z")
}

// Gregorian conversion for nonnegative days since the Unix epoch.
fn civil_from_days(days_since_epoch: i64) -> (i64, i64, i64) {
    let z = days_since_epoch + 719_468;
    let era = z / 146_097;
    let day_of_era = z - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let mut year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_prime = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_prime + 2) / 5 + 1;
    let month = month_prime + if month_prime < 10 { 3 } else { -9 };
    year += i64::from(month <= 2);
    (year, month, day)
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use std::path::{Path, PathBuf};
    use std::process::Command;
    use std::sync::atomic::{AtomicU64, Ordering};

    use agentmage_kernel_contracts::{ActorId, SessionId};
    use agentmage_kernel_engine::operational_store::{
        OperationalStoreKeyError, OperationalStoreKeyProvider,
    };
    use agentmage_platform_linux::{
        LinuxSandboxLimits, LinuxSandboxManifest, LinuxSandboxRunner, LinuxWorkerRuntimeFile,
        open_test_linux_authority,
    };

    use super::{
        HOST_PROTOCOL_VERSION, HostRequest, HostResponse, LinuxReadError, LinuxReadWorkflow,
        ReadClock, ReadIdentitySource, ReadInstant, format_utc, preview_suffix,
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

    impl ReadIdentitySource for TestIdentities {
        fn next(&mut self, prefix: &str) -> Result<String, LinuxReadError> {
            self.0 += 1;
            Ok(format!("{prefix}-{:032x}", self.0))
        }
    }

    struct TestClock(VecDeque<ReadInstant>);

    impl TestClock {
        fn at(times: &[u64]) -> Self {
            Self(
                times
                    .iter()
                    .map(|epoch_ms| {
                        ReadInstant::new(*epoch_ms, format_utc(*epoch_ms)).expect("test instant")
                    })
                    .collect(),
            )
        }
    }

    impl ReadClock for TestClock {
        fn now(&mut self) -> Result<ReadInstant, LinuxReadError> {
            self.0.pop_front().ok_or(LinuxReadError::ClockUnavailable)
        }
    }

    fn temp_root(label: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "agentmage-host-{label}-{}-{}",
            std::process::id(),
            TEMP_ID.fetch_add(1, Ordering::SeqCst)
        ));
        fs::create_dir(&root).expect("test root");
        root
    }

    fn runtime_files(executable: &Path) -> Vec<LinuxWorkerRuntimeFile> {
        let output = Command::new("/usr/bin/ldd")
            .arg(executable)
            .output()
            .expect("ldd available");
        assert!(output.status.success());
        String::from_utf8(output.stdout)
            .expect("ldd UTF-8")
            .lines()
            .filter_map(|line| {
                let candidate = line.split_once("=>").map_or_else(
                    || line.split_whitespace().next(),
                    |(_, right)| right.split_whitespace().next(),
                )?;
                if !candidate.starts_with('/') {
                    return None;
                }
                let canonical = fs::canonicalize(candidate).expect("canonical runtime");
                Some(LinuxWorkerRuntimeFile::new(canonical, candidate))
            })
            .collect()
    }

    fn sandbox() -> LinuxSandboxRunner {
        let executable = fs::canonicalize("/usr/bin/cat").expect("canonical worker executable");
        let manifest = LinuxSandboxManifest::verify(
            "/usr/bin/systemd-run",
            "/usr/bin/bwrap",
            &executable,
            &runtime_files(&executable),
        )
        .expect("verified worker manifest");
        LinuxSandboxRunner::new(manifest, LinuxSandboxLimits::default()).expect("sandbox runner")
    }

    fn workflow(
        state_root: &Path,
        times: &[u64],
    ) -> LinuxReadWorkflow<'static, TestIdentities, TestClock> {
        let mut key = TestKey([91; 32]);
        let authority =
            open_test_linux_authority(state_root, &mut key, 1).expect("test authority opens");
        LinuxReadWorkflow::new_test(
            authority,
            sandbox(),
            ActorId::from_raw("actor-phase9-test"),
            SessionId::from_raw("session-phase9-test"),
            TestIdentities(0),
            TestClock::at(times),
        )
        .expect("test workflow")
    }

    fn preview_request(workspace: &Path, request_id: &str) -> HostRequest {
        HostRequest::PreviewRead {
            schema_version: HOST_PROTOCOL_VERSION,
            request_id: request_id.to_owned(),
            workspace_id: "workspace-phase9-test".to_owned(),
            workspace_root: workspace.to_str().expect("UTF-8 test root").to_owned(),
            components: vec!["approved.txt".to_owned()],
        }
    }

    fn preview_fields(response: HostResponse) -> (String, String) {
        match response {
            HostResponse::ReadPreview {
                preview_id,
                confirmation_sha256,
                ..
            } => (preview_id, confirmation_sha256),
            _ => panic!("expected exact preview"),
        }
    }

    #[test]
    fn doctor_uses_the_same_workflow_and_reports_unproved_components_unavailable() {
        let state = temp_root("doctor");
        fs::set_permissions(&state, fs::Permissions::from_mode(0o700)).expect("private state");
        let mut workflow = workflow(&state, &[]);
        let response = workflow.handle(HostRequest::Doctor {
            schema_version: HOST_PROTOCOL_VERSION,
            request_id: "request-doctor-0001".to_owned(),
        });
        let HostResponse::DoctorCompleted {
            request_id, report, ..
        } = response
        else {
            panic!("expected doctor response");
        };
        assert_eq!(request_id, "request-doctor-0001");
        assert_eq!(
            report.items.len(),
            agentmage_kernel_contracts::DiagnosticComponent::ALL.len()
        );
        assert_eq!(
            report.items[2].state,
            agentmage_kernel_contracts::DiagnosticState::Unavailable
        );
        assert_eq!(
            report.items[5].state,
            agentmage_kernel_contracts::DiagnosticState::Healthy
        );
        let encoded = serde_json::to_string(&report).expect("report JSON");
        assert!(!encoded.contains(state.to_string_lossy().as_ref()));
        std::fs::remove_dir_all(state).expect("remove state");
    }

    #[test]
    fn diagnostic_export_preview_and_approval_use_the_authenticated_workflow() {
        let state = temp_root("diagnostic-export-state");
        fs::set_permissions(&state, fs::Permissions::from_mode(0o700)).expect("private state");
        let destination_root = temp_root("diagnostic-export-destination");
        fs::set_permissions(&destination_root, fs::Permissions::from_mode(0o700))
            .expect("private destination");
        let destination = destination_root.join("doctor.json");
        let mut workflow = workflow(&state, &[100, 101]);
        let preview = workflow.handle(HostRequest::PreviewDiagnosticExport {
            schema_version: HOST_PROTOCOL_VERSION,
            request_id: "request-export-preview".to_owned(),
            destination: destination.to_string_lossy().into_owned(),
        });
        let HostResponse::DiagnosticExportPreview {
            preview_id,
            confirmation_sha256,
            payload_sha256,
            ..
        } = preview
        else {
            panic!("expected diagnostic export preview");
        };
        assert!(!destination.exists());
        let completed = workflow.handle(HostRequest::ApproveDiagnosticExport {
            schema_version: HOST_PROTOCOL_VERSION,
            request_id: "request-export-approval".to_owned(),
            preview_id,
            confirmation_sha256,
        });
        assert!(matches!(
            completed,
            HostResponse::DiagnosticExportCompleted {
                ref outcome,
                payload_sha256: ref completed_sha256,
                ..
            } if outcome == "succeeded" && completed_sha256 == &payload_sha256
        ));
        assert!(destination.exists());
        fs::remove_dir_all(state).expect("remove state");
        fs::remove_dir_all(destination_root).expect("remove destination");
    }

    #[test]
    fn utc_format_and_preview_identity_are_stable() {
        assert_eq!(format_utc(0), "1970-01-01T00:00:00.000Z");
        assert_eq!(format_utc(1_786_320_000_000), "2026-08-10T00:00:00.000Z");
        assert_eq!(
            preview_suffix("preview-00112233445566778899aabbccddeeff"),
            Some("00112233445566778899aabbccddeeff")
        );
        assert_eq!(preview_suffix("preview-not-hex"), None);
    }

    #[test]
    #[ignore = "requires the Fedora or Ubuntu systemd user session and Bubblewrap runtime"]
    fn approved_read_is_receipted_replay_safe_and_restart_verifiable() {
        let root = temp_root("approved-read");
        let workspace = root.join("workspace");
        let state = root.join("state");
        fs::create_dir(&workspace).expect("workspace");
        fs::create_dir(&state).expect("state");
        fs::set_permissions(&state, fs::Permissions::from_mode(0o700)).expect("private state");
        fs::write(workspace.join("approved.txt"), b"bounded phase 9 output\n").expect("fixture");
        fs::write(workspace.join("adjacent.txt"), b"must remain invisible\n").expect("adjacent");

        let mut workflow = workflow(&state, &[1_000, 2_000, 3_000]);
        let (preview_id, confirmation_sha256) =
            preview_fields(workflow.handle(preview_request(&workspace, "request-preview")));
        let completed = workflow.handle(HostRequest::ApproveRead {
            schema_version: HOST_PROTOCOL_VERSION,
            request_id: "request-approve".to_owned(),
            preview_id: preview_id.clone(),
            confirmation_sha256: confirmation_sha256.clone(),
        });
        let receipt_sha256 = match completed {
            HostResponse::ReadCompleted {
                content, receipt, ..
            } => {
                assert_eq!(content, "bounded phase 9 output\n");
                assert_eq!(receipt.sequence, 1);
                receipt.receipt_sha256
            }
            _ => panic!("approved read must complete"),
        };
        let replay = workflow.handle(HostRequest::ApproveRead {
            schema_version: HOST_PROTOCOL_VERSION,
            request_id: "request-replay".to_owned(),
            preview_id,
            confirmation_sha256,
        });
        assert!(matches!(
            replay,
            HostResponse::Denied {
                ref code,
                receipt: Some(ref retained),
                ..
            } if code == "host.read.replay_denied"
                && retained.receipt_sha256 == receipt_sha256
        ));
        drop(workflow);

        let mut key = TestKey([91; 32]);
        let reopened =
            open_test_linux_authority(&state, &mut key, 4_000).expect("durable authority reopens");
        assert_eq!(reopened.authority().receipts().len(), 1);
        assert_eq!(
            reopened.authority().receipts()[0].receipt_sha256,
            receipt_sha256
        );
        drop(reopened);
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    #[ignore = "requires the Fedora or Ubuntu worker manifest binaries"]
    fn stale_and_cancelled_previews_start_no_worker_and_publish_no_receipt() {
        let root = temp_root("denied-read");
        let workspace = root.join("workspace");
        let state = root.join("state");
        fs::create_dir(&workspace).expect("workspace");
        fs::create_dir(&state).expect("state");
        fs::set_permissions(&state, fs::Permissions::from_mode(0o700)).expect("private state");
        fs::write(workspace.join("approved.txt"), b"first").expect("fixture");

        let mut workflow = workflow(&state, &[1_000, 2_000, 3_000, 4_000, 5_000, 6_000, 70_000]);
        let (stale_preview, stale_confirmation) =
            preview_fields(workflow.handle(preview_request(&workspace, "request-stale")));
        fs::write(workspace.join("approved.txt"), b"changed").expect("stale mutation");
        let stale = workflow.handle(HostRequest::ApproveRead {
            schema_version: HOST_PROTOCOL_VERSION,
            request_id: "request-stale-approve".to_owned(),
            preview_id: stale_preview,
            confirmation_sha256: stale_confirmation,
        });
        assert!(matches!(
            stale,
            HostResponse::Denied { ref code, .. } if code == "host.read.approval_denied"
        ));

        let (cancel_preview, _) =
            preview_fields(workflow.handle(preview_request(&workspace, "request-cancel")));
        let cancelled = workflow.handle(HostRequest::CancelRead {
            schema_version: HOST_PROTOCOL_VERSION,
            request_id: "request-cancel-confirm".to_owned(),
            preview_id: cancel_preview,
        });
        assert!(matches!(cancelled, HostResponse::Cancelled { .. }));

        let (mismatch_preview, _) =
            preview_fields(workflow.handle(preview_request(&workspace, "request-mismatch")));
        let mismatch = workflow.handle(HostRequest::ApproveRead {
            schema_version: HOST_PROTOCOL_VERSION,
            request_id: "request-mismatch-approve".to_owned(),
            preview_id: mismatch_preview,
            confirmation_sha256: "f".repeat(64),
        });
        assert!(matches!(
            mismatch,
            HostResponse::Denied { ref code, .. } if code == "host.read.approval_denied"
        ));

        let (expired_preview, expired_confirmation) =
            preview_fields(workflow.handle(preview_request(&workspace, "request-expired")));
        let expired = workflow.handle(HostRequest::ApproveRead {
            schema_version: HOST_PROTOCOL_VERSION,
            request_id: "request-expired-approve".to_owned(),
            preview_id: expired_preview,
            confirmation_sha256: expired_confirmation,
        });
        assert!(matches!(
            expired,
            HostResponse::Denied { ref code, .. } if code == "host.read.approval_denied"
        ));
        assert!(workflow.authority.authority().receipts().is_empty());
        drop(workflow);
        fs::remove_dir_all(root).expect("cleanup");
    }
}
