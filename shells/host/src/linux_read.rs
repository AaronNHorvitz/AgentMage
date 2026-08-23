//! Linux composition for one previewed, approved, exact workspace-file read.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use agentmage_capability_read_only::{
    MAX_READ_ONLY_CALL_DEPTH, READ_ONLY_TOOL_VERSION, ReadOnlyEncoding, ReadOnlyItem,
    ReadOnlyLimits, ReadOnlyOutcome, ReadOnlyRequest, ReadOnlyResult, ReadOnlyToolKind,
    read_only_tool_definition, read_only_tool_kind, validate_read_only_request,
};
use agentmage_kernel_contracts::{
    ActionId, ActionKind, ActorId, ApprovalId, ApprovalRequest, AuthorityTransactionId,
    ContractPayload, CorrelationId, DataSensitivity, DiagnosticComponent, DiagnosticObservation,
    DiagnosticState, DoctorReport, GrantId, GrantNonce, GrantOperation, GrantPreimage,
    GrantSideEffect, GrantTarget, HandoffDraft, HandoffProhibitedAction, HandoffReview,
    HeldWorkspaceObject, ModelPickerSnapshot, ModelProfileId, OperationAttemptId, OperationBinding,
    OperationOutcome, PathResolutionIntent, Receipt, SessionId, TaskId, ToolCall, ToolCallId,
    ToolDefinition, ToolId, ValidationIssue, ValidationSeverity, WorkspaceAuthorizationId,
    WorkspaceId, WorkspacePath, WorkspaceScopePath,
};
use agentmage_kernel_engine::approval::{render_approval_request, verify_approval_request};
use agentmage_kernel_engine::authority_transaction::AuthorityTransactionRequest;
use agentmage_kernel_engine::diagnostics::build_doctor_report;
use agentmage_kernel_engine::grants::{DerivedOperationGrantRequest, SessionReadGrantRequest};
use agentmage_kernel_engine::handoff::{
    SessionHandoffInput, build_handoff_review, cancel_handoff, compose_session_handoff_draft,
    deny_handoff_action, render_reviewed_handoff,
};
use agentmage_kernel_engine::model_discovery::{
    revalidate_model_selection, verify_model_picker_snapshot,
};
use agentmage_kernel_engine::platform_startup::VerifiedPlatformAdapter;
use agentmage_kernel_engine::policy::{
    PolicyEngine, PolicyEvaluationContext, StrictLocalReadOnlyScope, ToolPolicyBinding,
};
use agentmage_kernel_engine::tooling::{Tool, ToolAttemptGuard, ToolRegistry};
use agentmage_platform_linux::{
    LinuxAuthenticatedIpcSession, LinuxAuthorityRuntime, LinuxHeldObject, LinuxPlatformAdapter,
    LinuxReadOnlyToolEffectDriver, LinuxReadOnlyToolInput, LinuxSandboxCancellation,
    LinuxSandboxRunner, resolve_linux_workspace_object, select_linux_workspace,
};
use rustix::rand::{GetRandomFlags, getrandom};
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::diagnostic_export::{DiagnosticExportError, DiagnosticExportWorkflow};
use crate::engineering_runtime::{EngineeringRuntimePort, EngineeringRuntimeService};
use crate::protocol::{
    HOST_PROTOCOL_VERSION, HostProjectionKind, HostProjectionPath, HostRequest, HostResponse,
    MAX_HOST_REQUEST_BYTES, MAX_HOST_RESPONSE_BYTES, ReceiptSummary, encode_response,
    parse_request,
};
use crate::runtime_transport::{
    RuntimePrepareInput, RuntimeTransportError, RuntimeTransportPort, RuntimeTransportStep,
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
    /// No trusted current model-picker snapshot is installed.
    ModelDiscoveryUnavailable,
    /// A model-picker snapshot or selection revalidation was invalid.
    ModelDiscoveryInvalid,
    /// No trusted current handoff draft is installed.
    HandoffUnavailable,
    /// Handoff construction, review, revalidation, or rendering failed closed.
    HandoffInvalid,
    /// The optional shared-runtime transport dependency failed closed.
    RuntimeTransport(RuntimeTransportError),
    /// No durable Engineering Runtime service was installed by trusted composition.
    EngineeringRuntimeUnavailable,
    /// A durable Engineering Runtime operation failed closed.
    EngineeringRuntimeFailed,
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
            Self::ModelDiscoveryUnavailable => "host.model-discovery.unavailable",
            Self::ModelDiscoveryInvalid => "host.model-discovery.invalid",
            Self::HandoffUnavailable => "host.handoff.unavailable",
            Self::HandoffInvalid => "host.handoff.invalid",
            Self::RuntimeTransport(error) => error.code(),
            Self::EngineeringRuntimeUnavailable => "host.engineering-runtime.unavailable",
            Self::EngineeringRuntimeFailed => "host.engineering-runtime.failed",
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

struct PendingLinuxTool {
    approval: ApprovalRequest,
    parent_grant_id: GrantId,
    held: Vec<LinuxHeldObject>,
    operation_targets: Vec<GrantTarget>,
    kind: ReadOnlyToolKind,
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
    pending_tools: BTreeMap<String, PendingLinuxTool>,
    attempt_guard: ToolAttemptGuard,
    tool_cancellation: LinuxSandboxCancellation,
    diagnostic_exports: DiagnosticExportWorkflow,
    model_picker: Option<ModelPickerSnapshot>,
    handoff_draft: Option<HandoffDraft>,
    pending_handoffs: BTreeMap<String, HandoffReview>,
    runtime_transport: Option<Box<dyn RuntimeTransportPort>>,
    engineering_runtime: Option<Box<dyn EngineeringRuntimePort>>,
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
        let registry = read_only_registry()?;
        let engineering_runtime = EngineeringRuntimeService::new(
            authority
                .engineering_store()
                .map_err(|_| LinuxReadError::EngineeringRuntimeUnavailable)?,
        );
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
            pending_tools: BTreeMap::new(),
            attempt_guard: ToolAttemptGuard::new(3, MAX_READ_ONLY_CALL_DEPTH)
                .map_err(|_| LinuxReadError::AuthorityDenied)?,
            tool_cancellation: LinuxSandboxCancellation::default(),
            diagnostic_exports: DiagnosticExportWorkflow::new(),
            model_picker: None,
            handoff_draft: None,
            pending_handoffs: BTreeMap::new(),
            runtime_transport: None,
            engineering_runtime: Some(Box::new(engineering_runtime)),
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
        let registry = read_only_registry()?;
        let engineering_runtime = EngineeringRuntimeService::new(
            authority
                .engineering_store()
                .map_err(|_| LinuxReadError::EngineeringRuntimeUnavailable)?,
        );
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
            pending_tools: BTreeMap::new(),
            attempt_guard: ToolAttemptGuard::new(3, MAX_READ_ONLY_CALL_DEPTH)
                .map_err(|_| LinuxReadError::AuthorityDenied)?,
            tool_cancellation: LinuxSandboxCancellation::default(),
            diagnostic_exports: DiagnosticExportWorkflow::new(),
            model_picker: None,
            handoff_draft: None,
            pending_handoffs: BTreeMap::new(),
            runtime_transport: None,
            engineering_runtime: Some(Box::new(engineering_runtime)),
        })
    }

    /// Installs one explicitly composed caller-neutral shared-runtime transport.
    ///
    /// Model discovery alone never installs or selects this dependency. Until a trusted
    /// composition supplies it, every runtime request fails closed as unavailable.
    pub fn install_runtime_transport(&mut self, runtime: Box<dyn RuntimeTransportPort>) {
        self.runtime_transport = Some(runtime);
    }

    /// Installs one Rust-owned durable Engineering Runtime service.
    pub fn install_engineering_runtime(&mut self, runtime: Box<dyn EngineeringRuntimePort>) {
        self.engineering_runtime = Some(runtime);
    }

    /// Returns a signal bound to the next read-only worker launch in this session.
    #[must_use]
    pub fn next_tool_cancellation(&self) -> LinuxSandboxCancellation {
        self.tool_cancellation.clone()
    }

    /// Replaces the transport snapshot after trusted activation refreshes it.
    pub fn replace_model_picker_snapshot(
        &mut self,
        snapshot: ModelPickerSnapshot,
    ) -> Result<(), LinuxReadError> {
        verify_model_picker_snapshot(&snapshot)
            .map_err(|_| LinuxReadError::ModelDiscoveryInvalid)?;
        self.model_picker = Some(snapshot);
        Ok(())
    }

    /// Replaces the local-only handoff source from exact verified current-session material.
    pub fn replace_handoff_session(
        &mut self,
        input: SessionHandoffInput<'_>,
    ) -> Result<(), LinuxReadError> {
        let draft =
            compose_session_handoff_draft(input).map_err(|_| LinuxReadError::HandoffInvalid)?;
        self.install_handoff_draft(draft)
    }

    #[cfg(test)]
    fn replace_handoff_draft(&mut self, draft: HandoffDraft) -> Result<(), LinuxReadError> {
        self.install_handoff_draft(draft)
    }

    fn install_handoff_draft(&mut self, draft: HandoffDraft) -> Result<(), LinuxReadError> {
        build_handoff_review(&draft, "handoff-validation".to_owned(), 1)
            .map_err(|_| LinuxReadError::HandoffInvalid)?;
        self.pending_handoffs.clear();
        self.handoff_draft = Some(draft);
        Ok(())
    }

    /// Handles one already parsed request from an authenticated local channel.
    #[must_use]
    pub fn handle(&mut self, request: HostRequest) -> HostResponse {
        let request_id = request_id(&request).to_owned();
        let result = match request {
            HostRequest::Engineering { request, .. } => match self.engineering_runtime.as_mut() {
                Some(runtime) => match runtime.handle(request) {
                    Ok(response) => Ok(HostResponse::Engineering {
                        schema_version: HOST_PROTOCOL_VERSION,
                        request_id: request_id.clone(),
                        response,
                    }),
                    Err(error) => Ok(HostResponse::Denied {
                        schema_version: HOST_PROTOCOL_VERSION,
                        request_id: request_id.clone(),
                        code: error.code().to_owned(),
                        receipt: None,
                    }),
                },
                None => Err(LinuxReadError::EngineeringRuntimeUnavailable),
            },
            HostRequest::PreviewHandoff { .. } => self.preview_handoff(&request_id),
            HostRequest::RenderHandoff {
                preview_id,
                confirmation_sha256,
                non_public_acknowledged,
                ..
            } => self.render_handoff(
                &request_id,
                &preview_id,
                &confirmation_sha256,
                non_public_acknowledged,
            ),
            HostRequest::CancelHandoff { preview_id, .. } => {
                self.cancel_handoff_review(&request_id, &preview_id)
            }
            HostRequest::DenyHandoffAction {
                handoff_id, action, ..
            } => self.deny_handoff_delivery(&request_id, handoff_id, action),
            HostRequest::DiscoverModels { .. } => self.discover_models(&request_id),
            HostRequest::RevalidateModel {
                profile_id,
                expected_entry_sha256,
                ..
            } => self.revalidate_model(&request_id, profile_id, expected_entry_sha256),
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
            HostRequest::PreviewTool {
                workspace_id,
                workspace_root,
                tool_id,
                tool_version,
                arguments_json,
                projection,
                ..
            } => self.preview_tool(
                &request_id,
                &workspace_id,
                Path::new(&workspace_root),
                &tool_id,
                &tool_version,
                arguments_json.as_bytes(),
                projection,
            ),
            HostRequest::ApproveTool {
                preview_id,
                confirmation_sha256,
                ..
            } => self.approve_tool(&request_id, &preview_id, &confirmation_sha256),
            HostRequest::CancelTool { preview_id, .. } => {
                self.cancel_tool(&request_id, &preview_id)
            }
            HostRequest::PrepareRuntime {
                profile_id,
                expected_entry_sha256,
                workspace_id,
                workspace_root,
                prompt,
                ..
            } => self.prepare_runtime(
                &request_id,
                RuntimePrepareInput {
                    profile_id,
                    expected_entry_sha256,
                    workspace_id,
                    workspace_root,
                    prompt,
                },
            ),
            HostRequest::StartRuntime { run_request, .. } => {
                self.start_runtime(&request_id, *run_request)
            }
            HostRequest::AdvanceRuntime {
                run_id,
                request_sha256,
                after_event_cursor,
                approval_response,
                ..
            } => self.advance_runtime(
                &request_id,
                &run_id,
                &request_sha256,
                after_event_cursor.as_ref(),
                approval_response.as_ref(),
            ),
            HostRequest::CancelRuntime {
                run_id,
                request_sha256,
                cancellation_id,
                after_event_cursor,
                ..
            } => self.cancel_runtime(
                &request_id,
                &run_id,
                &request_sha256,
                cancellation_id,
                after_event_cursor.as_ref(),
            ),
            HostRequest::ReleaseRuntime {
                run_id,
                request_sha256,
                ..
            } => self.release_runtime(&request_id, &run_id, &request_sha256),
        };
        result.unwrap_or_else(|error| HostResponse::Denied {
            schema_version: HOST_PROTOCOL_VERSION,
            request_id,
            code: error.code().to_owned(),
            receipt: None,
        })
    }

    fn discover_models(&self, request_id: &str) -> Result<HostResponse, LinuxReadError> {
        let snapshot = self
            .model_picker
            .as_ref()
            .ok_or(LinuxReadError::ModelDiscoveryUnavailable)?;
        verify_model_picker_snapshot(snapshot)
            .map_err(|_| LinuxReadError::ModelDiscoveryInvalid)?;
        Ok(HostResponse::ModelsDiscovered {
            schema_version: HOST_PROTOCOL_VERSION,
            request_id: request_id.to_owned(),
            snapshot: snapshot.clone(),
        })
    }

    fn prepare_runtime(
        &mut self,
        request_id: &str,
        input: RuntimePrepareInput,
    ) -> Result<HostResponse, LinuxReadError> {
        let request = self
            .runtime_transport
            .as_mut()
            .ok_or(LinuxReadError::RuntimeTransport(
                RuntimeTransportError::RunUnavailable,
            ))?
            .prepare(input)
            .map_err(LinuxReadError::RuntimeTransport)?;
        Ok(HostResponse::RuntimePrepared {
            schema_version: HOST_PROTOCOL_VERSION,
            request_id: request_id.to_owned(),
            run_request: Box::new(request),
        })
    }

    fn start_runtime(
        &mut self,
        request_id: &str,
        request: agentmage_kernel_contracts::RuntimeRunRequest,
    ) -> Result<HostResponse, LinuxReadError> {
        let step = self
            .runtime_transport
            .as_mut()
            .ok_or(LinuxReadError::RuntimeTransport(
                RuntimeTransportError::RunUnavailable,
            ))?
            .start(request)
            .map_err(LinuxReadError::RuntimeTransport)?;
        Ok(runtime_response(request_id, step))
    }

    fn advance_runtime(
        &mut self,
        request_id: &str,
        run_id: &agentmage_kernel_contracts::RuntimeRunId,
        request_sha256: &str,
        after_event_cursor: Option<&agentmage_kernel_contracts::RuntimeEventCursor>,
        response: Option<&agentmage_kernel_contracts::RuntimeApprovalResponse>,
    ) -> Result<HostResponse, LinuxReadError> {
        let step = self
            .runtime_transport
            .as_mut()
            .ok_or(LinuxReadError::RuntimeTransport(
                RuntimeTransportError::RunUnavailable,
            ))?
            .advance(run_id, request_sha256, after_event_cursor, response)
            .map_err(LinuxReadError::RuntimeTransport)?;
        Ok(runtime_response(request_id, step))
    }

    fn cancel_runtime(
        &mut self,
        request_id: &str,
        run_id: &agentmage_kernel_contracts::RuntimeRunId,
        request_sha256: &str,
        cancellation_id: agentmage_kernel_contracts::CancellationId,
        after_event_cursor: Option<&agentmage_kernel_contracts::RuntimeEventCursor>,
    ) -> Result<HostResponse, LinuxReadError> {
        let step = self
            .runtime_transport
            .as_mut()
            .ok_or(LinuxReadError::RuntimeTransport(
                RuntimeTransportError::RunUnavailable,
            ))?
            .cancel(run_id, request_sha256, cancellation_id, after_event_cursor)
            .map_err(LinuxReadError::RuntimeTransport)?;
        Ok(runtime_response(request_id, step))
    }

    fn release_runtime(
        &mut self,
        request_id: &str,
        run_id: &agentmage_kernel_contracts::RuntimeRunId,
        request_sha256: &str,
    ) -> Result<HostResponse, LinuxReadError> {
        self.runtime_transport
            .as_mut()
            .ok_or(LinuxReadError::RuntimeTransport(
                RuntimeTransportError::RunUnavailable,
            ))?
            .release(run_id, request_sha256)
            .map_err(LinuxReadError::RuntimeTransport)?;
        Ok(HostResponse::Cancelled {
            schema_version: HOST_PROTOCOL_VERSION,
            request_id: request_id.to_owned(),
        })
    }

    fn preview_handoff(&mut self, request_id: &str) -> Result<HostResponse, LinuxReadError> {
        if self.pending.len() + self.pending_tools.len() + self.pending_handoffs.len()
            >= MAX_PENDING_PREVIEWS
        {
            return Err(LinuxReadError::ApprovalDenied);
        }
        let draft = self
            .handoff_draft
            .as_ref()
            .ok_or(LinuxReadError::HandoffUnavailable)?;
        let now = self.clock.now()?;
        let preview_id = self.identities.next("handoff-preview")?;
        let expires_at_ms = now
            .epoch_ms
            .checked_add(PREVIEW_LIFETIME_MS)
            .ok_or(LinuxReadError::ClockUnavailable)?;
        let review = build_handoff_review(draft, preview_id.clone(), expires_at_ms)
            .map_err(|_| LinuxReadError::HandoffInvalid)?;
        self.pending_handoffs.insert(preview_id, review.clone());
        Ok(HostResponse::HandoffPreview {
            schema_version: HOST_PROTOCOL_VERSION,
            request_id: request_id.to_owned(),
            review,
        })
    }

    fn render_handoff(
        &mut self,
        request_id: &str,
        preview_id: &str,
        confirmation_sha256: &str,
        non_public_acknowledged: bool,
    ) -> Result<HostResponse, LinuxReadError> {
        let review = self
            .pending_handoffs
            .remove(preview_id)
            .ok_or(LinuxReadError::ApprovalDenied)?;
        if review.confirmation_sha256 != confirmation_sha256 {
            return Err(LinuxReadError::ApprovalDenied);
        }
        let draft = self
            .handoff_draft
            .as_ref()
            .ok_or(LinuxReadError::HandoffUnavailable)?;
        let now = self.clock.now()?;
        let attempt_id = self.identities.next("handoff-render")?;
        let rendered = render_reviewed_handoff(
            &review,
            draft,
            now.epoch_ms,
            non_public_acknowledged,
            attempt_id,
        )
        .map_err(|_| LinuxReadError::HandoffInvalid)?;
        Ok(HostResponse::HandoffRendered {
            schema_version: HOST_PROTOCOL_VERSION,
            request_id: request_id.to_owned(),
            rendered,
        })
    }

    fn cancel_handoff_review(
        &mut self,
        request_id: &str,
        preview_id: &str,
    ) -> Result<HostResponse, LinuxReadError> {
        let review = self
            .pending_handoffs
            .remove(preview_id)
            .ok_or(LinuxReadError::ApprovalDenied)?;
        let attempt_id = self.identities.next("handoff-cancel")?;
        let receipt = cancel_handoff(attempt_id, review.manifest.handoff_id)
            .map_err(|_| LinuxReadError::HandoffInvalid)?;
        Ok(HostResponse::HandoffReceipt {
            schema_version: HOST_PROTOCOL_VERSION,
            request_id: request_id.to_owned(),
            receipt,
        })
    }

    fn deny_handoff_delivery(
        &mut self,
        request_id: &str,
        handoff_id: Option<String>,
        action: HandoffProhibitedAction,
    ) -> Result<HostResponse, LinuxReadError> {
        let attempt_id = self.identities.next("handoff-denial")?;
        let receipt = deny_handoff_action(attempt_id, handoff_id, action)
            .map_err(|_| LinuxReadError::HandoffInvalid)?;
        Ok(HostResponse::HandoffReceipt {
            schema_version: HOST_PROTOCOL_VERSION,
            request_id: request_id.to_owned(),
            receipt,
        })
    }

    fn revalidate_model(
        &self,
        request_id: &str,
        profile_id: String,
        expected_entry_sha256: String,
    ) -> Result<HostResponse, LinuxReadError> {
        let snapshot = self
            .model_picker
            .as_ref()
            .ok_or(LinuxReadError::ModelDiscoveryUnavailable)?;
        let revalidation = revalidate_model_selection(
            ModelProfileId::from_raw(profile_id),
            expected_entry_sha256,
            snapshot,
        )
        .map_err(|_| LinuxReadError::ModelDiscoveryInvalid)?;
        Ok(HostResponse::ModelRevalidated {
            schema_version: HOST_PROTOCOL_VERSION,
            request_id: request_id.to_owned(),
            revalidation,
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
        let Some(pending) = self.pending.remove(preview_id) else {
            return Ok(replay_denial(
                request_id,
                self.receipt_for_preview(preview_id),
            ));
        };
        let now = self.clock.now()?;
        if pending.approval.expires_at_epoch_ms <= now.epoch_ms
            || pending.approval.confirmation_sha256 != confirmation_sha256
            || verify_approval_request(&self.registry, &pending.approval).is_err()
            || pending.held.revalidate().is_err()
        {
            return Err(LinuxReadError::ApprovalDenied);
        }

        let approval = pending.approval;
        let tool_kind = read_only_tool_kind(
            &approval.tool_call.tool_id,
            &approval.tool_call.tool_version,
        )
        .ok_or(LinuxReadError::AuthorityDenied)?;
        let validated_request =
            validate_read_only_request(tool_kind, &approval.tool_call.arguments.bytes)
                .map_err(|_| LinuxReadError::AuthorityDenied)?;
        self.attempt_guard
            .record_attempt(&approval.tool_call, validated_request.call_depth)
            .map_err(|_| LinuxReadError::ApprovalDenied)?;
        let worker_input = LinuxReadOnlyToolInput::seal(
            approval.tool_call.tool_id.as_str(),
            approval.tool_call.tool_version.as_str(),
            &approval.tool_call.arguments.bytes,
            vec![pending.held],
        )
        .map_err(|_| LinuxReadError::WorkerFailed)?;
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
        let cancellation = std::mem::take(&mut self.tool_cancellation);
        let mut driver =
            LinuxReadOnlyToolEffectDriver::new_cancellable(runner, worker_input, cancellation);
        let receipt_result = self.authority.authority_mut().execute_effect(
            &self.registry,
            &policy,
            request,
            &mut driver,
        );
        let worker_result = driver.take_result();
        let worker_error = driver.take_error();
        self.sandbox = Some(driver.into_runner());
        let receipt = receipt_result.map_err(|_| LinuxReadError::AuthorityDenied)?;
        self.authority
            .revalidate_root()
            .map_err(|_| LinuxReadError::AuthorityDenied)?;
        let summary = receipt_summary(&receipt);
        let Some(worker_result) = worker_result else {
            debug_assert!(worker_error.is_some());
            return Ok(HostResponse::Denied {
                schema_version: HOST_PROTOCOL_VERSION,
                request_id: request_id.to_owned(),
                code: LinuxReadError::WorkerFailed.code().to_owned(),
                receipt: Some(summary),
            });
        };
        if worker_error.is_some()
            || !worker_result.success()
            || receipt.outcome != OperationOutcome::Succeeded
        {
            return Ok(HostResponse::Denied {
                schema_version: HOST_PROTOCOL_VERSION,
                request_id: request_id.to_owned(),
                code: LinuxReadError::WorkerFailed.code().to_owned(),
                receipt: Some(summary),
            });
        }
        let result = match serde_json::from_slice::<ReadOnlyResult>(worker_result.stdout()) {
            Ok(result) if result.verify(tool_kind) => result,
            _ => return Ok(output_denial(request_id, summary)),
        };
        if result.outcome != ReadOnlyOutcome::Succeeded || result.items.len() != 1 {
            return Ok(output_denial(request_id, summary));
        }
        let expected_path = operation_path
            .components()
            .iter()
            .map(|component| component.as_str().to_owned())
            .collect::<Vec<_>>();
        let content = match result.items.into_iter().next() {
            Some(ReadOnlyItem::Text { path, content, .. }) if path == expected_path => content,
            _ => return Ok(output_denial(request_id, summary)),
        };
        Ok(HostResponse::ReadCompleted {
            schema_version: HOST_PROTOCOL_VERSION,
            request_id: request_id.to_owned(),
            content,
            file_uri: pending.display_uri,
            receipt: summary,
        })
    }

    #[allow(clippy::too_many_arguments)]
    fn preview_tool(
        &mut self,
        request_id: &str,
        workspace_id: &str,
        workspace_root: &Path,
        tool_id: &str,
        tool_version: &str,
        arguments: &[u8],
        projection: Vec<HostProjectionPath>,
    ) -> Result<HostResponse, LinuxReadError> {
        let now = self.clock.now()?;
        self.pending_tools
            .retain(|_, candidate| candidate.approval.expires_at_epoch_ms > now.epoch_ms);
        if self.pending.len() + self.pending_tools.len() >= MAX_PENDING_PREVIEWS {
            return Err(LinuxReadError::ApprovalDenied);
        }
        let tool_id = ToolId::from_raw(tool_id);
        let kind =
            read_only_tool_kind(&tool_id, tool_version).ok_or(LinuxReadError::AuthorityDenied)?;
        let validated = validate_read_only_request(kind, arguments)
            .map_err(|_| LinuxReadError::AuthorityDenied)?;
        let mut unique_paths = BTreeSet::new();
        if projection.iter().any(|item| {
            !unique_paths.insert(item.components.clone())
                || !validated
                    .paths
                    .iter()
                    .any(|root| item.components.starts_with(root))
        }) || validated
            .paths
            .iter()
            .any(|path| !unique_paths.contains(path))
        {
            return Err(LinuxReadError::PathDenied);
        }

        let workspace_id = WorkspaceId::from_raw(workspace_id);
        let authorization_id =
            WorkspaceAuthorizationId::from_raw(self.identities.next("workspace-authorization")?);
        let (held, scopes) = match &self.platform {
            LinuxReadPlatform::Verified(platform) => {
                let workspace = select_linux_workspace(
                    platform,
                    workspace_root,
                    workspace_id.clone(),
                    authorization_id,
                )
                .map_err(|_| LinuxReadError::PathDenied)?;
                let mut held = Vec::with_capacity(projection.len());
                let mut scopes = Vec::with_capacity(projection.len());
                for item in &projection {
                    let path = WorkspacePath::new(workspace_id.clone(), item.components.clone())
                        .map_err(|_| LinuxReadError::PathDenied)?;
                    let intent = projection_intent(item.object_kind);
                    held.push(
                        resolve_linux_workspace_object(platform, &workspace, &path, intent)
                            .map_err(|_| LinuxReadError::PathDenied)?,
                    );
                    scopes.push(
                        GrantTarget::workspace_scope(
                            &workspace,
                            WorkspaceScopePath::new(workspace_id.clone(), item.components.clone())
                                .map_err(|_| LinuxReadError::PathDenied)?,
                        )
                        .map_err(|_| LinuxReadError::AuthorityDenied)?,
                    );
                }
                (held, scopes)
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
                let mut held = Vec::with_capacity(projection.len());
                let mut scopes = Vec::with_capacity(projection.len());
                for item in &projection {
                    let path = WorkspacePath::new(workspace_id.clone(), item.components.clone())
                        .map_err(|_| LinuxReadError::PathDenied)?;
                    let intent = projection_intent(item.object_kind);
                    held.push(
                        agentmage_platform_linux::resolve_test_linux_workspace_object(
                            &workspace,
                            adapter_instance_id.clone(),
                            &path,
                            intent,
                        )
                        .map_err(|_| LinuxReadError::PathDenied)?,
                    );
                    scopes.push(
                        GrantTarget::workspace_scope(
                            &workspace,
                            WorkspaceScopePath::new(workspace_id.clone(), item.components.clone())
                                .map_err(|_| LinuxReadError::PathDenied)?,
                        )
                        .map_err(|_| LinuxReadError::AuthorityDenied)?,
                    );
                }
                (held, scopes)
            }
        };
        let operation_targets = held
            .iter()
            .map(GrantTarget::held_object)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| LinuxReadError::AuthorityDenied)?;
        let task_id = TaskId::from_raw(self.identities.next("task")?);
        let action_id = ActionId::from_raw(self.identities.next("action")?);
        let parent_grant_id = GrantId::from_raw(self.identities.next("grant-parent")?);
        let operation_grant_id = GrantId::from_raw(self.identities.next("grant-operation")?);
        let approval_id = ApprovalId::from_raw(self.identities.next("approval")?);
        let preview_id = self.identities.next("preview")?;
        let tool_call =
            generic_tool_call(&mut self.identities, &action_id, kind, arguments.to_vec())?;
        let operation = OperationBinding::new(GrantOperation::WorkspaceRead);
        let preimages = operation_targets
            .iter()
            .enumerate()
            .filter_map(|(index, target)| {
                u32::try_from(index)
                    .ok()
                    .and_then(|index| GrantPreimage::for_target(index, target))
            })
            .collect::<Vec<_>>();
        let target_indexes = (0..operation_targets.len())
            .map(|index| u32::try_from(index).map_err(|_| LinuxReadError::AuthorityDenied))
            .collect::<Result<Vec<_>, _>>()?;
        let side_effect = GrantSideEffect {
            operation,
            target_indexes,
            details_sha256: digest_serialized(&operation_targets)?,
        };
        let policy = exact_tool_policy(
            &self.actor_id,
            &task_id,
            &action_id,
            kind,
            &operation_targets,
        )?;
        let expires_at_epoch_ms = now
            .epoch_ms
            .checked_add(PREVIEW_LIFETIME_MS)
            .ok_or(LinuxReadError::ClockUnavailable)?;
        let parent_preview_sha256 = digest_serialized(&scopes)?;
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
                targets: scopes,
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
        let approval = render_approval_request(
            &self.registry,
            ApprovalRequest {
                schema_version: agentmage_kernel_contracts::CONTRACT_SCHEMA_VERSION,
                approval_id,
                proposed_grant_id: operation_grant_id,
                parent_grant_id: parent_grant_id.clone(),
                parent_grant_sha256: digest_serialized(&parent)?,
                actor_id: self.actor_id.clone(),
                session_id: self.session_id.clone(),
                task_id,
                action_kind: ActionKind::DeterministicTool,
                operation,
                tool_call,
                targets: operation_targets.clone(),
                excluded_targets: Vec::new(),
                sensitivity: DataSensitivity::Ephemeral,
                preimages,
                expected_side_effects: vec![side_effect],
                rollback_description: "No state change is permitted".to_owned(),
                issued_at_epoch_ms: now.epoch_ms,
                expires_at_epoch_ms,
                policy_sha256: policy.policy_sha256().to_owned(),
                confirmation_sha256: "0".repeat(64),
            },
        )
        .map_err(|_| LinuxReadError::AuthorityDenied)?;
        let target_set_sha256 = digest_serialized(&operation_targets)?;
        let response = HostResponse::ToolPreview {
            schema_version: HOST_PROTOCOL_VERSION,
            request_id: request_id.to_owned(),
            preview_id: preview_id.clone(),
            tool_id: kind.id().to_owned(),
            tool_version: READ_ONLY_TOOL_VERSION.to_owned(),
            projected_objects: u32::try_from(operation_targets.len())
                .map_err(|_| LinuxReadError::AuthorityDenied)?,
            target_set_sha256,
            expires_at_epoch_ms,
            confirmation_sha256: approval.confirmation_sha256.clone(),
        };
        self.pending_tools.insert(
            preview_id,
            PendingLinuxTool {
                approval,
                parent_grant_id,
                held,
                operation_targets,
                kind,
            },
        );
        Ok(response)
    }

    fn approve_tool(
        &mut self,
        request_id: &str,
        preview_id: &str,
        confirmation_sha256: &str,
    ) -> Result<HostResponse, LinuxReadError> {
        let Some(pending) = self.pending_tools.remove(preview_id) else {
            return Ok(tool_replay_denial(
                request_id,
                self.receipt_for_preview(preview_id),
            ));
        };
        let now = self.clock.now()?;
        if pending.approval.expires_at_epoch_ms <= now.epoch_ms
            || pending.approval.confirmation_sha256 != confirmation_sha256
            || verify_approval_request(&self.registry, &pending.approval).is_err()
            || pending.held.iter().any(|held| held.revalidate().is_err())
        {
            return Err(LinuxReadError::ApprovalDenied);
        }
        let approval = pending.approval;
        let validated =
            validate_read_only_request(pending.kind, &approval.tool_call.arguments.bytes)
                .map_err(|_| LinuxReadError::AuthorityDenied)?;
        self.attempt_guard
            .record_attempt(&approval.tool_call, validated.call_depth)
            .map_err(|_| LinuxReadError::ApprovalDenied)?;
        let worker_input = LinuxReadOnlyToolInput::seal(
            approval.tool_call.tool_id.as_str(),
            approval.tool_call.tool_version.as_str(),
            &approval.tool_call.arguments.bytes,
            pending.held,
        )
        .map_err(|_| LinuxReadError::WorkerFailed)?;
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
        let policy = exact_tool_policy(
            &approval.actor_id,
            &approval.task_id,
            &approval.tool_call.action_id,
            pending.kind,
            &pending.operation_targets,
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
        let request = AuthorityTransactionRequest::new(
            AuthorityTransactionId::from_raw(format!("transaction-{suffix}")),
            OperationAttemptId::from_raw(format!("attempt-{suffix}")),
            approval.approval_id,
            grant.grant_id,
            approval.tool_call,
            context,
            now.epoch_ms,
            now.occurred_at,
        )
        .map_err(|_| LinuxReadError::AuthorityDenied)?;
        let runner = self.sandbox.take().ok_or(LinuxReadError::WorkerFailed)?;
        let cancellation = std::mem::take(&mut self.tool_cancellation);
        let mut driver =
            LinuxReadOnlyToolEffectDriver::new_cancellable(runner, worker_input, cancellation);
        let receipt_result = self.authority.authority_mut().execute_effect(
            &self.registry,
            &policy,
            request,
            &mut driver,
        );
        let worker_result = driver.take_result();
        let worker_error = driver.take_error();
        self.sandbox = Some(driver.into_runner());
        let receipt = receipt_result.map_err(|_| LinuxReadError::AuthorityDenied)?;
        self.authority
            .revalidate_root()
            .map_err(|_| LinuxReadError::AuthorityDenied)?;
        let summary = receipt_summary(&receipt);
        let Some(worker_result) = worker_result else {
            debug_assert!(worker_error.is_some());
            return Ok(HostResponse::Denied {
                schema_version: HOST_PROTOCOL_VERSION,
                request_id: request_id.to_owned(),
                code: LinuxReadError::WorkerFailed.code().to_owned(),
                receipt: Some(summary),
            });
        };
        if worker_error.is_some()
            || !worker_result.success()
            || receipt.outcome != OperationOutcome::Succeeded
        {
            return Ok(HostResponse::Denied {
                schema_version: HOST_PROTOCOL_VERSION,
                request_id: request_id.to_owned(),
                code: LinuxReadError::WorkerFailed.code().to_owned(),
                receipt: Some(summary),
            });
        }
        let result = match serde_json::from_slice::<ReadOnlyResult>(worker_result.stdout()) {
            Ok(result) if result.verify(pending.kind) => result,
            _ => return Ok(output_denial(request_id, summary)),
        };
        Ok(HostResponse::ToolCompleted {
            schema_version: HOST_PROTOCOL_VERSION,
            request_id: request_id.to_owned(),
            result,
            receipt: summary,
        })
    }

    fn cancel_tool(
        &mut self,
        request_id: &str,
        preview_id: &str,
    ) -> Result<HostResponse, LinuxReadError> {
        if self.pending_tools.remove(preview_id).is_some() {
            return Ok(HostResponse::Cancelled {
                schema_version: HOST_PROTOCOL_VERSION,
                request_id: request_id.to_owned(),
            });
        }
        Ok(tool_replay_denial(
            request_id,
            self.receipt_for_preview(preview_id),
        ))
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

fn read_only_registry() -> Result<ToolRegistry, LinuxReadError> {
    let mut registry = ToolRegistry::new();
    for kind in ReadOnlyToolKind::ALL {
        registry
            .register_tool(Box::new(RegisteredReadTool {
                definition: read_only_tool_definition(kind),
                kind,
            }))
            .map_err(|_| LinuxReadError::AuthorityDenied)?;
    }
    Ok(registry)
}

fn output_denial(request_id: &str, receipt: ReceiptSummary) -> HostResponse {
    HostResponse::Denied {
        schema_version: HOST_PROTOCOL_VERSION,
        request_id: request_id.to_owned(),
        code: LinuxReadError::OutputDenied.code().to_owned(),
        receipt: Some(receipt),
    }
}

fn tool_call(
    identities: &mut impl ReadIdentitySource,
    action_id: &ActionId,
    path: &WorkspacePath,
) -> Result<ToolCall, LinuxReadError> {
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
    generic_tool_call(identities, action_id, ReadOnlyToolKind::ReadText, bytes)
}

fn generic_tool_call(
    identities: &mut impl ReadIdentitySource,
    action_id: &ActionId,
    kind: ReadOnlyToolKind,
    bytes: Vec<u8>,
) -> Result<ToolCall, LinuxReadError> {
    let definition = read_only_tool_definition(kind);
    Ok(ToolCall {
        schema_version: agentmage_kernel_contracts::CONTRACT_SCHEMA_VERSION,
        tool_call_id: ToolCallId::from_raw(identities.next("tool-call")?),
        correlation_id: CorrelationId::from_raw(identities.next("correlation")?),
        action_id: action_id.clone(),
        tool_id: ToolId::from_raw(kind.id()),
        tool_version: READ_ONLY_TOOL_VERSION.to_owned(),
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
    exact_tool_policy(
        actor_id,
        task_id,
        action_id,
        ReadOnlyToolKind::ReadText,
        std::slice::from_ref(target),
    )
}

fn exact_tool_policy(
    actor_id: &ActorId,
    task_id: &TaskId,
    action_id: &ActionId,
    kind: ReadOnlyToolKind,
    targets: &[GrantTarget],
) -> Result<PolicyEngine, LinuxReadError> {
    PolicyEngine::strict_local_read_only(StrictLocalReadOnlyScope {
        revision: 1,
        actors: BTreeSet::from([actor_id.clone()]),
        tasks: BTreeSet::from([task_id.clone()]),
        actions: BTreeSet::from([action_id.clone()]),
        tools: BTreeSet::from([ToolPolicyBinding {
            tool_id: ToolId::from_raw(kind.id()),
            tool_version: READ_ONLY_TOOL_VERSION.to_owned(),
        }]),
        targets: targets.iter().cloned().collect(),
    })
    .map_err(|_| LinuxReadError::AuthorityDenied)
}

const fn projection_intent(kind: HostProjectionKind) -> PathResolutionIntent {
    match kind {
        HostProjectionKind::RegularFile => PathResolutionIntent::ReadFile,
        HostProjectionKind::Directory => PathResolutionIntent::ReadDirectory,
    }
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

fn tool_replay_denial(request_id: &str, receipt: Option<ReceiptSummary>) -> HostResponse {
    HostResponse::Denied {
        schema_version: HOST_PROTOCOL_VERSION,
        request_id: request_id.to_owned(),
        code: if receipt.is_some() {
            "host.tool.replay_denied"
        } else {
            "host.tool.preview_missing"
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

fn runtime_response(request_id: &str, step: RuntimeTransportStep) -> HostResponse {
    HostResponse::RuntimeStep {
        schema_version: HOST_PROTOCOL_VERSION,
        request_id: request_id.to_owned(),
        run_id: step.run_id,
        request_sha256: step.request_sha256,
        events: step.events,
        artifacts: step.artifacts,
        approval: step.approval,
        outcome: step.outcome.map(Box::new),
    }
}

fn request_id(request: &HostRequest) -> &str {
    match request {
        HostRequest::PreviewHandoff { request_id, .. }
        | HostRequest::Engineering { request_id, .. }
        | HostRequest::RenderHandoff { request_id, .. }
        | HostRequest::CancelHandoff { request_id, .. }
        | HostRequest::DenyHandoffAction { request_id, .. }
        | HostRequest::DiscoverModels { request_id, .. }
        | HostRequest::RevalidateModel { request_id, .. }
        | HostRequest::Doctor { request_id, .. }
        | HostRequest::PreviewDiagnosticExport { request_id, .. }
        | HostRequest::ApproveDiagnosticExport { request_id, .. }
        | HostRequest::CancelDiagnosticExport { request_id, .. }
        | HostRequest::PreviewRead { request_id, .. }
        | HostRequest::ApproveRead { request_id, .. }
        | HostRequest::CancelRead { request_id, .. }
        | HostRequest::PreviewTool { request_id, .. }
        | HostRequest::ApproveTool { request_id, .. }
        | HostRequest::CancelTool { request_id, .. }
        | HostRequest::PrepareRuntime { request_id, .. }
        | HostRequest::StartRuntime { request_id, .. }
        | HostRequest::AdvanceRuntime { request_id, .. }
        | HostRequest::CancelRuntime { request_id, .. }
        | HostRequest::ReleaseRuntime { request_id, .. } => request_id,
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
    use std::collections::{BTreeSet, VecDeque};
    use std::fs;
    use std::os::unix::fs::{MetadataExt, PermissionsExt};
    use std::path::{Path, PathBuf};
    use std::process::Command;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::thread;
    use std::time::{Duration, Instant};

    use agentmage_capability_read_only::{
        ReadOnlyEncoding, ReadOnlyItem, ReadOnlyLimits, ReadOnlyRequest, ReadOnlyToolKind,
    };
    use agentmage_kernel_contracts::{
        ActorId, CONTRACT_SCHEMA_VERSION, CheckedContextSummary, CheckedSummaryState,
        ComposedContextPacket, ContextAdmission, ContextItemCandidate, ContextItemKind,
        ContextPacketId, ContextSensitivity, ContextSummaryId, EvidenceId, HandoffDestinationClass,
        HandoffDisclosureEntry, HandoffDraft, HandoffEntryDisposition, HandoffEntryKind,
        HandoffProhibitedAction, HandoffSensitivity, LocalHandoffOutcome, ModelProfileId, PlanId,
        PlanStepId, PolicyId, RepositorySnapshotId, RuntimeApprovalResponse, RuntimeEventCursor,
        RuntimeRunId, RuntimeRunRequest, SessionCheckpoint, SessionCheckpointId, SessionId, Task,
        TaskId, TaskStatus, WorkspaceId,
    };
    use agentmage_kernel_engine::context_management::{
        ContextCompositionBudget, compose_context, finalize_checkpoint,
    };
    use agentmage_kernel_engine::handoff::{
        SessionHandoffInput, SessionHandoffSelection, build_handoff_review, seal_handoff_entry,
        validate_handoff_draft,
    };
    use agentmage_kernel_engine::model_discovery::build_model_picker_snapshot;
    use agentmage_kernel_engine::operational_store::{
        OperationalStoreKeyError, OperationalStoreKeyProvider,
    };
    use agentmage_platform_linux::{
        LinuxSandboxLimits, LinuxSandboxManifest, LinuxSandboxRunner, LinuxWorkerRuntimeFile,
        open_test_linux_authority,
    };

    use super::{
        HOST_PROTOCOL_VERSION, HostProjectionKind, HostProjectionPath, HostRequest, HostResponse,
        LinuxReadError, LinuxReadWorkflow, ReadClock, ReadIdentitySource, ReadInstant, digest,
        format_utc, preview_suffix, read_only_registry,
    };
    use crate::runtime_transport::{
        RuntimePrepareInput, RuntimeTransportError, RuntimeTransportPort, RuntimeTransportStep,
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

    struct RecordingRuntimeTransport {
        request: RuntimeRunRequest,
        step: RuntimeTransportStep,
    }

    impl RuntimeTransportPort for RecordingRuntimeTransport {
        fn prepare(
            &mut self,
            _input: RuntimePrepareInput,
        ) -> Result<RuntimeRunRequest, RuntimeTransportError> {
            Ok(self.request.clone())
        }

        fn start(
            &mut self,
            request: RuntimeRunRequest,
        ) -> Result<RuntimeTransportStep, RuntimeTransportError> {
            if request != self.request {
                return Err(RuntimeTransportError::RequestDenied);
            }
            Ok(self.step.clone())
        }

        fn advance(
            &mut self,
            _run_id: &RuntimeRunId,
            _request_sha256: &str,
            _after_event_cursor: Option<&RuntimeEventCursor>,
            _response: Option<&RuntimeApprovalResponse>,
        ) -> Result<RuntimeTransportStep, RuntimeTransportError> {
            Ok(self.step.clone())
        }

        fn cancel(
            &mut self,
            _run_id: &RuntimeRunId,
            _request_sha256: &str,
            _cancellation_id: agentmage_kernel_contracts::CancellationId,
            _after_event_cursor: Option<&RuntimeEventCursor>,
        ) -> Result<RuntimeTransportStep, RuntimeTransportError> {
            Ok(self.step.clone())
        }

        fn release(
            &mut self,
            _run_id: &RuntimeRunId,
            _request_sha256: &str,
        ) -> Result<(), RuntimeTransportError> {
            Ok(())
        }
    }

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
        sandbox_for("/usr/bin/bash")
    }

    fn sandbox_for(executable: impl AsRef<Path>) -> LinuxSandboxRunner {
        sandbox_for_limits(executable, LinuxSandboxLimits::default())
    }

    fn sandbox_for_limits(
        executable: impl AsRef<Path>,
        limits: LinuxSandboxLimits,
    ) -> LinuxSandboxRunner {
        let executable = fs::canonicalize(executable).expect("canonical worker executable");
        let systemd_run =
            fs::canonicalize("/usr/bin/systemd-run").expect("canonical systemd-run executable");
        let bubblewrap =
            fs::canonicalize("/usr/bin/bwrap").expect("canonical Bubblewrap executable");
        let runtime_files = runtime_files(&executable);
        let manifest =
            LinuxSandboxManifest::verify(systemd_run, bubblewrap, &executable, &runtime_files)
                .expect("verified worker manifest");
        LinuxSandboxRunner::new(manifest, limits).expect("sandbox runner")
    }

    fn lifecycle_fixture(case: &str) -> PathBuf {
        let root = std::env::var_os("AGENTMAGE_LIFECYCLE_FIXTURE_ROOT")
            .map(PathBuf::from)
            .expect("lifecycle fixture root");
        root.join(format!("agentmage-lifecycle-{case}"))
    }

    fn lifecycle_units() -> BTreeSet<String> {
        let output = Command::new("/usr/bin/systemctl")
            .args([
                "--user",
                "list-units",
                "--all",
                "--plain",
                "--no-legend",
                "agentmage-worker-*",
            ])
            .output()
            .expect("systemctl unit inventory");
        assert!(output.status.success());
        String::from_utf8(output.stdout)
            .expect("unit inventory UTF-8")
            .lines()
            .filter_map(|line| line.split_whitespace().next().map(str::to_owned))
            .collect()
    }

    fn lifecycle_processes(name: &str) -> Vec<u32> {
        let mut processes = fs::read_dir("/proc")
            .expect("proc inventory")
            .filter_map(Result::ok)
            .filter_map(|entry| entry.file_name().to_string_lossy().parse::<u32>().ok())
            .filter(|pid| {
                fs::read(format!("/proc/{pid}/cmdline")).is_ok_and(|command| {
                    command
                        .windows(name.len())
                        .any(|part| part == name.as_bytes())
                })
            })
            .collect::<Vec<_>>();
        processes.sort_unstable();
        processes
    }

    fn lifecycle_scratch_visible(name: &str) -> bool {
        lifecycle_processes(name).into_iter().any(|pid| {
            PathBuf::from(format!("/proc/{pid}/root/tmp/agentmage-lifecycle-scratch")).is_file()
        })
    }

    fn lifecycle_unit_tasks(unit: &str) -> u32 {
        let output = Command::new("/usr/bin/systemctl")
            .args(["--user", "show", "--property=TasksCurrent", "--value", unit])
            .output()
            .expect("systemctl task inventory");
        assert!(output.status.success());
        String::from_utf8(output.stdout)
            .expect("task inventory UTF-8")
            .trim()
            .parse()
            .unwrap_or(0)
    }

    fn wait_for_lifecycle_cleanup(name: &str, baseline_units: &BTreeSet<String>) {
        let deadline = Instant::now() + Duration::from_secs(5);
        while Instant::now() < deadline {
            if lifecycle_processes(name).is_empty() && lifecycle_units() == *baseline_units {
                return;
            }
            thread::sleep(Duration::from_millis(25));
        }
        panic!("lifecycle process or unit residue remains for {name}");
    }

    fn installed_read_only_worker_sandbox() -> LinuxSandboxRunner {
        sandbox_for("/usr/libexec/agentmage/agentmage-read-only-worker")
    }

    fn workflow(
        state_root: &Path,
        times: &[u64],
    ) -> LinuxReadWorkflow<'static, TestIdentities, TestClock> {
        workflow_with_sandbox(state_root, times, sandbox())
    }

    fn handoff_draft() -> HandoffDraft {
        let entry = seal_handoff_entry(HandoffDisclosureEntry {
            entry_id: "entry-host-0001".to_owned(),
            source_id: "source-host-0001".to_owned(),
            display_source: "src/lib.rs".to_owned(),
            fragment: "lines:1-10".to_owned(),
            excerpt: "pub fn bounded() {}".to_owned(),
            content_sha256: "a".repeat(64),
            kind: HandoffEntryKind::SourceExcerpt,
            sensitivity: HandoffSensitivity::UserProvided,
            disposition: HandoffEntryDisposition::Include,
            redactions: Vec::new(),
            hidden: false,
            related: true,
            entry_sha256: String::new(),
        })
        .expect("sealed handoff entry");
        HandoffDraft {
            schema_version: CONTRACT_SCHEMA_VERSION,
            handoff_id: "handoff-host-0001".to_owned(),
            workspace_state_sha256: "a".repeat(64),
            policy_sha256: "a".repeat(64),
            redaction_policy_sha256: "a".repeat(64),
            objective: "Review the local boundary".to_owned(),
            acceptance_criteria: vec!["Report findings".to_owned()],
            constraints: vec!["Do not modify files".to_owned()],
            entries: vec![entry],
            exclusions: vec!["Credentials".to_owned()],
            unresolved_questions: vec!["Is native evidence retained?".to_owned()],
            destination: HandoffDestinationClass::ManualCodexInterface,
        }
    }

    fn canonical_handoff_material() -> (
        Task,
        SessionCheckpoint,
        ComposedContextPacket,
        CheckedContextSummary,
    ) {
        let task = Task {
            schema_version: CONTRACT_SCHEMA_VERSION,
            task_id: TaskId::from_raw("task-host-handoff-0001"),
            session_id: SessionId::from_raw("session-host-handoff-0001"),
            objective: "Review the current local boundary".to_owned(),
            acceptance_criteria: vec!["Report exact findings".to_owned()],
            constraints: vec!["Do not modify files".to_owned()],
            status: TaskStatus::Ready,
        };
        let excerpt = "pub fn bounded() {}";
        let context = compose_context(
            ContextPacketId::from_raw("context-host-handoff-0001"),
            &ContextCompositionBudget {
                max_bytes: 1_024,
                max_tokens: 256,
                max_items: 8,
                token_counter_id: "counter-host-handoff-v1".to_owned(),
            },
            vec![ContextItemCandidate {
                item_id: "host-source".to_owned(),
                kind: ContextItemKind::NewestRequest,
                sensitivity: ContextSensitivity::Private,
                admission: ContextAdmission::Eligible,
                authoritative_evidence: false,
                essential: true,
                source_id: "src/lib.rs".to_owned(),
                source_revision: "lines:1-10".to_owned(),
                content_sha256: digest(excerpt.as_bytes()),
                bounded_excerpt: excerpt.to_owned(),
                token_count: 5,
            }],
        )
        .expect("host handoff context");
        let checkpoint = finalize_checkpoint(SessionCheckpoint {
            schema_version: CONTRACT_SCHEMA_VERSION,
            checkpoint_id: SessionCheckpointId::from_raw("checkpoint-host-handoff-0001"),
            session_id: task.session_id.clone(),
            task_id: task.task_id.clone(),
            objective_sha256: digest(task.objective.as_bytes()),
            plan_id: PlanId::from_raw("plan-host-handoff-0001"),
            plan_revision: 1,
            plan_step_id: PlanStepId::from_raw("step-host-handoff-0001"),
            next_action_sha256: "1".repeat(64),
            workspace_id: WorkspaceId::from_raw("workspace-host-handoff-0001"),
            workspace_state_sha256: "2".repeat(64),
            repository_snapshot_id: RepositorySnapshotId::from_raw("repository-host-handoff-0001"),
            repository_branch: "main".to_owned(),
            repository_map_sha256: "3".repeat(64),
            files: Vec::new(),
            instruction_sha256: "4".repeat(64),
            permission_profile_id: "permission-host-handoff-0001".to_owned(),
            permission_profile_sha256: "5".repeat(64),
            policy_id: PolicyId::from_raw("policy-host-handoff-0001"),
            policy_sha256: "6".repeat(64),
            model_profile_id: ModelProfileId::from_raw("model-host-handoff-0001"),
            model_manifest_sha256: "7".repeat(64),
            model_runtime_sha256: "8".repeat(64),
            evidence_ids: vec![EvidenceId::from_raw("evidence-host-handoff-0001")],
            citation_set_sha256: "9".repeat(64),
            blockers: Vec::new(),
            context_packet_sha256: context.packet_sha256.clone(),
            action_id: None,
            action_state: None,
            consumed_grant_id: None,
            receipt_id: None,
            receipt_sha256: None,
            ephemeral: true,
            checkpoint_sha256: "0".repeat(64),
        })
        .expect("host handoff checkpoint");
        let summary = CheckedContextSummary {
            schema_version: CONTRACT_SCHEMA_VERSION,
            summary_id: ContextSummaryId::from_raw("summary-host-handoff-0001"),
            state: CheckedSummaryState::Current,
            summary: "Current bounded host session".to_owned(),
            paths: vec!["src/lib.rs".to_owned()],
            errors: Vec::new(),
            identifiers: vec!["task-host-handoff-0001".to_owned()],
            commands: Vec::new(),
            decisions: vec!["Manual transfer only".to_owned()],
            unresolved_questions: vec!["Is native evidence retained?".to_owned()],
            evidence_ids: vec![EvidenceId::from_raw("evidence-host-handoff-0001")],
            citation_ids: vec!["citation-host-handoff-0001".to_owned()],
            receipt_ids: Vec::new(),
            source_set_sha256: "a".repeat(64),
        };
        (task, checkpoint, context, summary)
    }

    fn workflow_with_sandbox(
        state_root: &Path,
        times: &[u64],
        sandbox: LinuxSandboxRunner,
    ) -> LinuxReadWorkflow<'static, TestIdentities, TestClock> {
        let mut key = TestKey([91; 32]);
        let authority =
            open_test_linux_authority(state_root, &mut key, 1).expect("test authority opens");
        LinuxReadWorkflow::new_test(
            authority,
            sandbox,
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

    fn generic_search_request(workspace: &Path, request_id: &str) -> HostRequest {
        generic_tool_request(workspace, request_id, ReadOnlyToolKind::SearchText)
    }

    fn generic_tool_request(
        workspace: &Path,
        request_id: &str,
        kind: ReadOnlyToolKind,
    ) -> HostRequest {
        let src = vec!["src".to_owned()];
        let alpha = vec!["src".to_owned(), "alpha.txt".to_owned()];
        let zeta = vec!["src".to_owned(), "zeta.txt".to_owned()];
        let binary = vec!["src".to_owned(), "sample.bin".to_owned()];
        let (paths, query, encoding, projection) = match kind {
            ReadOnlyToolKind::ListDirectory
            | ReadOnlyToolKind::DirectoryTree
            | ReadOnlyToolKind::HashTree => (
                vec![src.clone()],
                None,
                ReadOnlyEncoding::Binary,
                vec![
                    (src, HostProjectionKind::Directory),
                    (alpha, HostProjectionKind::RegularFile),
                    (zeta, HostProjectionKind::RegularFile),
                    (binary, HostProjectionKind::RegularFile),
                ],
            ),
            ReadOnlyToolKind::SearchFilenames => (
                vec![src.clone()],
                Some("alpha".to_owned()),
                ReadOnlyEncoding::Binary,
                vec![
                    (src, HostProjectionKind::Directory),
                    (alpha, HostProjectionKind::RegularFile),
                    (zeta, HostProjectionKind::RegularFile),
                    (binary, HostProjectionKind::RegularFile),
                ],
            ),
            ReadOnlyToolKind::SearchText => (
                vec![src.clone()],
                Some("needle".to_owned()),
                ReadOnlyEncoding::Utf8,
                vec![
                    (src, HostProjectionKind::Directory),
                    (alpha, HostProjectionKind::RegularFile),
                    (zeta, HostProjectionKind::RegularFile),
                ],
            ),
            ReadOnlyToolKind::ReadText => (
                vec![alpha.clone()],
                None,
                ReadOnlyEncoding::Utf8,
                vec![(alpha, HostProjectionKind::RegularFile)],
            ),
            ReadOnlyToolKind::ReadMultiple => (
                vec![alpha.clone(), zeta.clone()],
                None,
                ReadOnlyEncoding::Utf8,
                vec![
                    (alpha, HostProjectionKind::RegularFile),
                    (zeta, HostProjectionKind::RegularFile),
                ],
            ),
            ReadOnlyToolKind::Metadata => (
                vec![alpha.clone(), zeta.clone()],
                None,
                ReadOnlyEncoding::Binary,
                vec![
                    (alpha, HostProjectionKind::RegularFile),
                    (zeta, HostProjectionKind::RegularFile),
                ],
            ),
            ReadOnlyToolKind::HashFile => (
                vec![alpha.clone()],
                None,
                ReadOnlyEncoding::Binary,
                vec![(alpha, HostProjectionKind::RegularFile)],
            ),
            ReadOnlyToolKind::BinaryMetadata => (
                vec![binary.clone()],
                None,
                ReadOnlyEncoding::Binary,
                vec![(binary, HostProjectionKind::RegularFile)],
            ),
        };
        let arguments = serde_json::to_string(&ReadOnlyRequest {
            schema_version: 1,
            paths,
            query,
            byte_offset: None,
            byte_count: None,
            encoding,
            limits: ReadOnlyLimits::default(),
            call_depth: 0,
        })
        .expect("request JSON");
        HostRequest::PreviewTool {
            schema_version: HOST_PROTOCOL_VERSION,
            request_id: request_id.to_owned(),
            workspace_id: "workspace-tool-test".to_owned(),
            workspace_root: workspace.to_str().expect("UTF-8 test root").to_owned(),
            tool_id: kind.id().to_owned(),
            tool_version: "1.0.0".to_owned(),
            arguments_json: arguments,
            projection: projection
                .into_iter()
                .map(|(components, object_kind)| HostProjectionPath {
                    components,
                    object_kind,
                })
                .collect(),
        }
    }

    fn workspace_observation(root: &Path) -> Vec<(PathBuf, u32, u32, u32, u64, i64, i64)> {
        fn visit(
            root: &Path,
            path: &Path,
            records: &mut Vec<(PathBuf, u32, u32, u32, u64, i64, i64)>,
        ) {
            let metadata = fs::symlink_metadata(path).expect("workspace metadata");
            records.push((
                path.strip_prefix(root)
                    .expect("relative workspace path")
                    .to_path_buf(),
                metadata.mode(),
                metadata.uid(),
                metadata.gid(),
                metadata.size(),
                metadata.mtime(),
                metadata.mtime_nsec(),
            ));
            if metadata.is_dir() {
                let mut children = fs::read_dir(path)
                    .expect("workspace directory")
                    .map(|entry| entry.expect("workspace entry").path())
                    .collect::<Vec<_>>();
                children.sort();
                for child in children {
                    visit(root, &child, records);
                }
            }
        }

        let mut records = Vec::new();
        visit(root, root, &mut records);
        records
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
    fn model_picker_transport_is_unavailable_until_verified_and_never_falls_back() {
        let state = temp_root("model-picker");
        fs::set_permissions(&state, fs::Permissions::from_mode(0o700)).expect("private state");
        let mut workflow = workflow(&state, &[]);
        let unavailable = workflow.handle(HostRequest::DiscoverModels {
            schema_version: HOST_PROTOCOL_VERSION,
            request_id: "request-models-0001".to_owned(),
        });
        assert!(matches!(
            unavailable,
            HostResponse::Denied { ref code, .. }
                if code == "host.model-discovery.unavailable"
        ));

        let snapshot = build_model_picker_snapshot("a".repeat(64), true, 10, Vec::new())
            .expect("empty verified projection");
        workflow
            .replace_model_picker_snapshot(snapshot.clone())
            .expect("valid snapshot");
        let discovered = workflow.handle(HostRequest::DiscoverModels {
            schema_version: HOST_PROTOCOL_VERSION,
            request_id: "request-models-0002".to_owned(),
        });
        assert!(matches!(
            discovered,
            HostResponse::ModelsDiscovered { snapshot: ref actual, .. }
                if actual == &snapshot
        ));

        let revalidated = workflow.handle(HostRequest::RevalidateModel {
            schema_version: HOST_PROTOCOL_VERSION,
            request_id: "request-models-0003".to_owned(),
            profile_id: "removed-profile".to_owned(),
            expected_entry_sha256: "b".repeat(64),
        });
        assert!(matches!(
            revalidated,
            HostResponse::ModelRevalidated { revalidation, .. }
                if !revalidation.admitted
                    && revalidation.profile_id.as_str() == "removed-profile"
                    && revalidation.result_code == "model.selection.profile-unavailable"
        ));

        let mut tampered = snapshot;
        tampered.snapshot_sha256 = "c".repeat(64);
        assert_eq!(
            workflow.replace_model_picker_snapshot(tampered),
            Err(LinuxReadError::ModelDiscoveryInvalid)
        );
        std::fs::remove_dir_all(state).expect("remove state");
    }

    #[test]
    fn shared_runtime_is_unavailable_until_explicitly_composed_and_then_host_framed() {
        let state = temp_root("native-chat-runtime");
        fs::set_permissions(&state, fs::Permissions::from_mode(0o700)).expect("private state");
        let mut workflow = workflow(&state, &[]);
        let prepare_request = HostRequest::PrepareRuntime {
            schema_version: HOST_PROTOCOL_VERSION,
            request_id: "request-runtime-prepare-0001".to_owned(),
            profile_id: "profile-native-chat-0001".to_owned(),
            expected_entry_sha256: "a".repeat(64),
            workspace_id: "workspace-native-chat-0001".to_owned(),
            workspace_root: "/tmp/native-chat-workspace".to_owned(),
            prompt: "Inspect the selected workspace".to_owned(),
        };
        assert!(matches!(
            workflow.handle(prepare_request.clone()),
            HostResponse::Denied { ref code, .. } if code == "host.runtime.run_unavailable"
        ));

        let (request, events, outcome, _) =
            crate::runtime_read_tests::completed_native_read_fixture();
        workflow.install_runtime_transport(Box::new(RecordingRuntimeTransport {
            request: request.clone(),
            step: RuntimeTransportStep {
                run_id: request.run_id.clone(),
                request_sha256: request.request_sha256.clone(),
                events: events.clone(),
                artifacts: Vec::new(),
                approval: None,
                outcome: Some(outcome.clone()),
            },
        }));
        assert!(matches!(
            workflow.handle(prepare_request),
            HostResponse::RuntimePrepared {
                run_request,
                ..
            } if *run_request == request
        ));
        assert!(matches!(
            workflow.handle(HostRequest::StartRuntime {
                schema_version: HOST_PROTOCOL_VERSION,
                request_id: "request-runtime-start-0001".to_owned(),
                run_request: Box::new(request.clone()),
            }),
            HostResponse::RuntimeStep {
                run_id,
                request_sha256,
                events: actual_events,
                artifacts,
                approval: None,
                outcome: Some(actual_outcome),
                ..
            } if run_id == request.run_id
                && request_sha256 == request.request_sha256
                && actual_events == events
                && artifacts.is_empty()
                && *actual_outcome == outcome
        ));
        fs::remove_dir_all(state).expect("remove state");
    }

    #[test]
    fn handoff_transport_requires_current_review_and_never_delivers() {
        let state_root = temp_root("handoff");
        fs::set_permissions(&state_root, fs::Permissions::from_mode(0o700)).expect("private state");
        let mut workflow = workflow(&state_root, &[10, 11, 12, 13]);
        assert!(matches!(
            workflow.handle(HostRequest::PreviewHandoff {
                schema_version: HOST_PROTOCOL_VERSION,
                request_id: "handoff-unavailable".to_owned(),
            }),
            HostResponse::Denied { ref code, .. } if code == "host.handoff.unavailable"
        ));

        let (task, checkpoint, context, summary) = canonical_handoff_material();
        workflow
            .replace_handoff_session(SessionHandoffInput {
                task: &task,
                checkpoint: &checkpoint,
                context: &context,
                summary: Some(&summary),
                redaction_policy_sha256: "b".repeat(64),
                selections: vec![SessionHandoffSelection {
                    context_item_id: "host-source".to_owned(),
                    redactions: Vec::new(),
                }],
                exclusions: vec!["Credentials".to_owned()],
            })
            .expect("canonical current-session handoff");
        let preview = workflow.handle(HostRequest::PreviewHandoff {
            schema_version: HOST_PROTOCOL_VERSION,
            request_id: "handoff-preview-request".to_owned(),
        });
        let (preview_id, confirmation_sha256, packet_sha256, handoff_id) = match preview {
            HostResponse::HandoffPreview { review, .. } => {
                assert!(review.manifest.acknowledgment_required);
                assert!(!review.manifest.delivered);
                (
                    review.preview_id,
                    review.confirmation_sha256,
                    review.manifest.packet_sha256,
                    review.manifest.handoff_id,
                )
            }
            _ => panic!("expected handoff preview"),
        };
        let rendered = workflow.handle(HostRequest::RenderHandoff {
            schema_version: HOST_PROTOCOL_VERSION,
            request_id: "handoff-render-request".to_owned(),
            preview_id,
            confirmation_sha256,
            non_public_acknowledged: true,
        });
        assert!(matches!(
            rendered,
            HostResponse::HandoffRendered { rendered, .. }
                if rendered.manifest.packet_sha256 == packet_sha256
                    && !rendered.receipt.external_delivery_attempted
                    && rendered.receipt.outcome == LocalHandoffOutcome::Rendered
        ));

        let denied = workflow.handle(HostRequest::DenyHandoffAction {
            schema_version: HOST_PROTOCOL_VERSION,
            request_id: "handoff-denial-request".to_owned(),
            handoff_id: Some(handoff_id),
            action: HandoffProhibitedAction::NetworkCall,
        });
        assert!(matches!(
            denied,
            HostResponse::HandoffReceipt { receipt, .. }
                if receipt.outcome == LocalHandoffOutcome::Denied
                    && receipt.prohibited_action == Some(HandoffProhibitedAction::NetworkCall)
                    && !receipt.external_delivery_attempted
        ));
        fs::remove_dir_all(state_root).expect("cleanup");
    }

    #[test]
    fn handoff_source_drift_and_missing_acknowledgment_fail_closed() {
        let state_root = temp_root("handoff-drift");
        fs::set_permissions(&state_root, fs::Permissions::from_mode(0o700)).expect("private state");
        let mut workflow = workflow(&state_root, &[10, 11, 12, 13]);
        let draft = handoff_draft();
        validate_handoff_draft(&draft).expect("valid host handoff draft");
        build_handoff_review(&draft, "host-validation".to_owned(), 1)
            .expect("valid host handoff fixture");
        workflow
            .replace_handoff_draft(draft.clone())
            .expect("trusted handoff draft");
        let preview = workflow.handle(HostRequest::PreviewHandoff {
            schema_version: HOST_PROTOCOL_VERSION,
            request_id: "handoff-preview-request".to_owned(),
        });
        let (preview_id, confirmation_sha256) = match preview {
            HostResponse::HandoffPreview { review, .. } => {
                (review.preview_id, review.confirmation_sha256)
            }
            _ => panic!("expected handoff preview"),
        };
        let unacknowledged = workflow.handle(HostRequest::RenderHandoff {
            schema_version: HOST_PROTOCOL_VERSION,
            request_id: "handoff-unacknowledged".to_owned(),
            preview_id,
            confirmation_sha256,
            non_public_acknowledged: false,
        });
        assert!(matches!(
            unacknowledged,
            HostResponse::Denied { ref code, .. } if code == "host.handoff.invalid"
        ));

        workflow
            .replace_handoff_draft(draft.clone())
            .expect("trusted handoff draft");
        let preview = workflow.handle(HostRequest::PreviewHandoff {
            schema_version: HOST_PROTOCOL_VERSION,
            request_id: "handoff-preview-drift".to_owned(),
        });
        let (preview_id, confirmation_sha256) = match preview {
            HostResponse::HandoffPreview { review, .. } => {
                (review.preview_id, review.confirmation_sha256)
            }
            _ => panic!("expected handoff preview"),
        };
        let mut changed = draft;
        changed.policy_sha256 = "b".repeat(64);
        workflow.handoff_draft = Some(changed);
        let stale = workflow.handle(HostRequest::RenderHandoff {
            schema_version: HOST_PROTOCOL_VERSION,
            request_id: "handoff-render-stale".to_owned(),
            preview_id,
            confirmation_sha256,
            non_public_acknowledged: true,
        });
        assert!(matches!(
            stale,
            HostResponse::Denied { ref code, .. } if code == "host.handoff.invalid"
        ));
        fs::remove_dir_all(state_root).expect("cleanup");
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
    fn production_registry_contains_only_the_complete_closed_read_only_catalog() {
        let registry = read_only_registry().expect("closed registry");
        let definitions = registry.list_tools();
        assert_eq!(definitions.len(), ReadOnlyToolKind::ALL.len());
        for kind in ReadOnlyToolKind::ALL {
            assert!(
                definitions
                    .iter()
                    .any(|definition| definition.tool_id.as_str() == kind.id())
            );
        }
        assert!(definitions.iter().all(|definition| {
            definition.declared_effects.len() == 1
                && definition.declared_effects[0].operation()
                    == agentmage_kernel_contracts::GrantOperation::WorkspaceRead
        }));
    }

    #[test]
    fn generic_tool_preview_binds_every_exact_object_and_cancels_without_receipt() {
        let root = temp_root("generic-preview");
        let workspace = root.join("workspace");
        let state = root.join("state");
        fs::create_dir_all(workspace.join("src")).expect("workspace tree");
        fs::create_dir(&state).expect("state");
        fs::set_permissions(&state, fs::Permissions::from_mode(0o700)).expect("private state");
        fs::write(workspace.join("src/alpha.txt"), b"needle first").expect("alpha");
        fs::write(workspace.join("src/zeta.txt"), b"needle last").expect("zeta");
        let mut workflow = workflow(&state, &[100]);

        let preview = workflow.handle(generic_search_request(&workspace, "request-tool-preview"));
        let (preview_id, confirmation_sha256) = match preview {
            HostResponse::ToolPreview {
                preview_id,
                confirmation_sha256,
                projected_objects,
                tool_id,
                target_set_sha256,
                ..
            } => {
                assert_eq!(projected_objects, 3);
                assert_eq!(tool_id, ReadOnlyToolKind::SearchText.id());
                assert_eq!(target_set_sha256.len(), 64);
                (preview_id, confirmation_sha256)
            }
            _ => panic!("expected generic tool preview"),
        };
        assert_eq!(confirmation_sha256.len(), 64);
        let cancelled = workflow.handle(HostRequest::CancelTool {
            schema_version: HOST_PROTOCOL_VERSION,
            request_id: "request-tool-cancel".to_owned(),
            preview_id,
        });
        assert!(matches!(cancelled, HostResponse::Cancelled { .. }));
        assert!(workflow.authority.authority().receipts().is_empty());
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn absent_read_and_tool_approvals_do_not_require_a_clock_sample() {
        let root = temp_root("approval-replay-clock");
        let state = root.join("state");
        fs::create_dir(&state).expect("state");
        fs::set_permissions(&state, fs::Permissions::from_mode(0o700)).expect("private state");
        let mut workflow = workflow(&state, &[]);
        let preview_id = "preview-00000000000000000000000000000001".to_owned();

        let read = workflow.handle(HostRequest::ApproveRead {
            schema_version: HOST_PROTOCOL_VERSION,
            request_id: "request-missing-read".to_owned(),
            preview_id: preview_id.clone(),
            confirmation_sha256: "a".repeat(64),
        });
        assert!(matches!(
            read,
            HostResponse::Denied {
                ref code,
                receipt: None,
                ..
            } if code == "host.read.preview_missing"
        ));

        let tool = workflow.handle(HostRequest::ApproveTool {
            schema_version: HOST_PROTOCOL_VERSION,
            request_id: "request-missing-tool".to_owned(),
            preview_id,
            confirmation_sha256: "b".repeat(64),
        });
        assert!(matches!(
            tool,
            HostResponse::Denied {
                ref code,
                receipt: None,
                ..
            } if code == "host.tool.preview_missing"
        ));
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn generic_tool_preview_rejects_duplicate_unscoped_and_wrong_kind_projections() {
        let root = temp_root("generic-denials");
        let workspace = root.join("workspace");
        let state = root.join("state");
        fs::create_dir_all(workspace.join("src")).expect("workspace tree");
        fs::create_dir(&state).expect("state");
        fs::set_permissions(&state, fs::Permissions::from_mode(0o700)).expect("private state");
        fs::write(workspace.join("src/alpha.txt"), b"needle").expect("alpha");
        fs::write(workspace.join("outside.txt"), b"outside").expect("outside");
        let mut workflow = workflow(&state, &[100, 200, 300]);

        let mut duplicate = generic_search_request(&workspace, "request-duplicate");
        if let HostRequest::PreviewTool { projection, .. } = &mut duplicate {
            projection.push(projection[1].clone());
        }
        assert!(matches!(
            workflow.handle(duplicate),
            HostResponse::Denied { ref code, .. } if code == LinuxReadError::PathDenied.code()
        ));

        let mut unscoped = generic_search_request(&workspace, "request-unscoped");
        if let HostRequest::PreviewTool { projection, .. } = &mut unscoped {
            projection.push(HostProjectionPath {
                components: vec!["outside.txt".to_owned()],
                object_kind: HostProjectionKind::RegularFile,
            });
        }
        assert!(matches!(
            workflow.handle(unscoped),
            HostResponse::Denied { ref code, .. } if code == LinuxReadError::PathDenied.code()
        ));

        let mut wrong_kind = generic_search_request(&workspace, "request-wrong-kind");
        if let HostRequest::PreviewTool { projection, .. } = &mut wrong_kind {
            projection[1].object_kind = HostProjectionKind::Directory;
        }
        assert!(matches!(
            workflow.handle(wrong_kind),
            HostResponse::Denied { ref code, .. } if code == LinuxReadError::PathDenied.code()
        ));
        assert!(workflow.authority.authority().receipts().is_empty());
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    #[ignore = "requires an installed root-owned package worker, systemd user session, and Bubblewrap"]
    fn every_generic_tool_worker_returns_verified_result_one_receipt_and_no_workspace_mutation() {
        let root = temp_root("generic-live");
        let workspace = root.join("workspace");
        let state = root.join("state");
        fs::create_dir_all(workspace.join("src")).expect("workspace tree");
        fs::create_dir(&state).expect("state");
        fs::set_permissions(&state, fs::Permissions::from_mode(0o700)).expect("private state");
        fs::write(workspace.join("src/alpha.txt"), b"needle first\n").expect("alpha");
        fs::write(workspace.join("src/zeta.txt"), b"needle last\n").expect("zeta");
        fs::write(
            workspace.join("src/sample.bin"),
            b"PK\x03\x04archive-like-bytes-are-never-expanded",
        )
        .expect("binary");
        let before = workspace_observation(&workspace);
        let times = (1..=ReadOnlyToolKind::ALL.len() * 2)
            .map(|index| index as u64 * 100)
            .collect::<Vec<_>>();
        let mut workflow =
            workflow_with_sandbox(&state, &times, installed_read_only_worker_sandbox());
        let mut final_preview = None;

        for (index, kind) in ReadOnlyToolKind::ALL.into_iter().enumerate() {
            let preview = workflow.handle(generic_tool_request(
                &workspace,
                &format!("request-live-preview-{index}"),
                kind,
            ));
            let (preview_id, confirmation_sha256) = match preview {
                HostResponse::ToolPreview {
                    preview_id,
                    confirmation_sha256,
                    tool_id,
                    ..
                } => {
                    assert_eq!(tool_id, kind.id());
                    (preview_id, confirmation_sha256)
                }
                _ => panic!("expected generic preview for {}", kind.id()),
            };
            let completed = workflow.handle(HostRequest::ApproveTool {
                schema_version: HOST_PROTOCOL_VERSION,
                request_id: format!("request-live-approve-{index}"),
                preview_id: preview_id.clone(),
                confirmation_sha256: confirmation_sha256.clone(),
            });
            let receipt_sha256 = match completed {
                HostResponse::ToolCompleted {
                    result, receipt, ..
                } => {
                    assert_eq!(
                        result.outcome,
                        agentmage_capability_read_only::ReadOnlyOutcome::Succeeded,
                        "{} outcome",
                        kind.id()
                    );
                    assert!(result.verify(kind), "{} result", kind.id());
                    assert!(!result.items.is_empty(), "{} items", kind.id());
                    if kind == ReadOnlyToolKind::BinaryMetadata {
                        assert!(matches!(
                            result.items.as_slice(),
                            [ReadOnlyItem::BinaryMetadata { format_hint, .. }]
                                if format_hint == "zip"
                        ));
                    }
                    assert_eq!(receipt.sequence, index as u64 + 1);
                    receipt.receipt_sha256
                }
                _ => panic!("expected completed generic tool for {}", kind.id()),
            };
            final_preview = Some((preview_id, confirmation_sha256, receipt_sha256));
        }

        let (preview_id, confirmation_sha256, receipt_sha256) =
            final_preview.expect("complete tool matrix");
        let replay = workflow.handle(HostRequest::ApproveTool {
            schema_version: HOST_PROTOCOL_VERSION,
            request_id: "request-live-replay".to_owned(),
            preview_id,
            confirmation_sha256,
        });
        assert!(matches!(
            replay,
            HostResponse::Denied {
                ref code,
                receipt: Some(ref retained),
                ..
            } if code == "host.tool.replay_denied"
                && retained.receipt_sha256 == receipt_sha256
        ));
        assert_eq!(workflow.authority.authority().receipts().len(), 10);
        assert_eq!(workspace_observation(&workspace), before);
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    #[ignore = "requires root-owned lifecycle fixtures, a systemd user session, and Bubblewrap"]
    fn lifecycle_matrix_retains_one_terminal_receipt_and_never_reports_false_completion() {
        for termination in ["cancel", "timeout", "kill", "crash"] {
            for phase in ["before", "during", "after"] {
                let case = format!("{termination}-{phase}");
                let root = temp_root(&format!("lifecycle-{case}"));
                let workspace = root.join("workspace");
                let state = root.join("state");
                fs::create_dir_all(workspace.join("src")).expect("workspace");
                fs::create_dir(&state).expect("state");
                fs::set_permissions(&state, fs::Permissions::from_mode(0o700))
                    .expect("private state");
                fs::write(
                    workspace.join("src/alpha.txt"),
                    b"bounded lifecycle input\n",
                )
                .expect("fixture");
                let before = workspace_observation(&workspace);
                let fixture = lifecycle_fixture(&case);
                let fixture_name = fixture
                    .file_name()
                    .and_then(|value| value.to_str())
                    .expect("fixture name")
                    .to_owned();
                let limits = LinuxSandboxLimits::new(
                    64 * 1024 * 1024,
                    16,
                    100,
                    if termination == "timeout" { 1 } else { 5 },
                    4 * 1024 * 1024,
                )
                .expect("lifecycle limits");
                let mut workflow = workflow_with_sandbox(
                    &state,
                    &[100, 200],
                    sandbox_for_limits(&fixture, limits),
                );
                let preview = workflow.handle(generic_tool_request(
                    &workspace,
                    &format!("request-lifecycle-preview-{case}"),
                    ReadOnlyToolKind::ReadText,
                ));
                let (preview_id, confirmation_sha256) = match preview {
                    HostResponse::ToolPreview {
                        preview_id,
                        confirmation_sha256,
                        ..
                    } => (preview_id, confirmation_sha256),
                    _ => panic!("expected lifecycle preview"),
                };
                let cancellation = workflow.next_tool_cancellation();
                let baseline_units = lifecycle_units();
                let control_baseline = baseline_units.clone();
                let control_name = fixture_name.clone();
                let control_termination = termination.to_owned();
                let controller = thread::spawn(move || {
                    let deadline = Instant::now() + Duration::from_secs(5);
                    let unit = loop {
                        let units = lifecycle_units()
                            .difference(&control_baseline)
                            .cloned()
                            .collect::<Vec<_>>();
                        if units.len() == 1
                            && !lifecycle_processes(&control_name).is_empty()
                            && lifecycle_unit_tasks(&units[0]) >= 2
                            && lifecycle_scratch_visible(&control_name)
                        {
                            break units[0].clone();
                        }
                        assert!(
                            Instant::now() < deadline,
                            "lifecycle fixture did not become ready"
                        );
                        thread::sleep(Duration::from_millis(5));
                    };
                    thread::sleep(Duration::from_millis(50));
                    match control_termination.as_str() {
                        "cancel" => cancellation.cancel(),
                        "kill" => {
                            let status = Command::new("/usr/bin/systemctl")
                                .args([
                                    "--user",
                                    "kill",
                                    "--kill-whom=all",
                                    "--signal=KILL",
                                    unit.as_str(),
                                ])
                                .status()
                                .expect("kill lifecycle unit");
                            assert!(status.success());
                        }
                        "timeout" | "crash" => {}
                        _ => panic!("unknown lifecycle termination"),
                    }
                });
                let response = workflow.handle(HostRequest::ApproveTool {
                    schema_version: HOST_PROTOCOL_VERSION,
                    request_id: format!("request-lifecycle-approve-{case}"),
                    preview_id: preview_id.clone(),
                    confirmation_sha256: confirmation_sha256.clone(),
                });
                controller.join().expect("lifecycle controller");
                let expected = match termination {
                    "cancel" => "cancelled",
                    "timeout" => "timed_out",
                    "kill" | "crash" => "failed",
                    _ => unreachable!(),
                };
                let receipt_sha256 = match response {
                    HostResponse::Denied {
                        code,
                        receipt: Some(receipt),
                        ..
                    } => {
                        assert_eq!(code, LinuxReadError::WorkerFailed.code(), "{case} code");
                        assert_eq!(receipt.outcome, expected, "{case} receipt outcome");
                        assert_eq!(receipt.sequence, 1, "{case} receipt sequence");
                        receipt.receipt_sha256
                    }
                    _ => panic!("{case} must not report completion"),
                };
                assert_eq!(
                    workflow.authority.authority().receipts().len(),
                    1,
                    "{case} receipt"
                );
                let replay = workflow.handle(HostRequest::ApproveTool {
                    schema_version: HOST_PROTOCOL_VERSION,
                    request_id: format!("request-lifecycle-replay-{case}"),
                    preview_id,
                    confirmation_sha256,
                });
                assert!(matches!(
                    replay,
                    HostResponse::Denied {
                        ref code,
                        receipt: Some(ref receipt),
                        ..
                    } if code == "host.tool.replay_denied"
                        && receipt.receipt_sha256 == receipt_sha256
                ));
                assert_eq!(
                    workflow.authority.authority().receipts().len(),
                    1,
                    "{case} replay"
                );
                assert_eq!(
                    workspace_observation(&workspace),
                    before,
                    "{case} workspace"
                );
                wait_for_lifecycle_cleanup(&fixture_name, &baseline_units);
                assert!(!Path::new("/tmp/agentmage-lifecycle-scratch").exists());
                fs::remove_dir_all(root).expect("cleanup");
            }
        }
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
