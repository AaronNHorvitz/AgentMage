//! Closed, length-bounded messages shared by the host and VS Code shell.

use serde::{Deserialize, Deserializer, Serialize};

use agentmage_capability_read_only::ReadOnlyResult;
use agentmage_kernel_contracts::{
    CancellationId, DoctorReport, HandoffProhibitedAction, HandoffReview, LocalHandoffReceipt,
    ModelPickerSnapshot, ModelSelectionRevalidation, RenderedHandoff, RuntimeApprovalChallenge,
    RuntimeApprovalResponse, RuntimeArtifactRef, RuntimeEvent, RuntimeEventCursor, RuntimeOutcome,
    RuntimeRunId, RuntimeRunRequest,
};
use agentmage_kernel_engine::runtime_coordinator::verify_runtime_run_request;

/// Version of the Phase 9 host protocol.
pub const HOST_PROTOCOL_VERSION: u16 = 1;

/// Maximum encoded client request accepted by the host.
pub const MAX_HOST_REQUEST_BYTES: usize = 64 * 1024;

/// Maximum encoded host response accepted by the shell.
pub const MAX_HOST_RESPONSE_BYTES: usize = 4 * 1024 * 1024;

const MAX_IDENTIFIER_BYTES: usize = 128;
const MAX_COMPONENTS: usize = 256;
const MAX_COMPONENT_BYTES: usize = 255;
const MAX_TOOL_ARGUMENT_BYTES: usize = 64 * 1024;
const MAX_RUNTIME_PROMPT_BYTES: usize = 4 * 1024;

/// Exact object kind requested for one bounded worker projection.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum HostProjectionKind {
    /// Resolve and continuously hold one regular file.
    RegularFile,
    /// Resolve and continuously hold one directory.
    Directory,
}

/// One exact workspace-relative object exposed to an isolated tool worker.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct HostProjectionPath {
    /// Canonical workspace-relative components.
    pub components: Vec<String>,
    /// Exact expected object kind.
    pub object_kind: HostProjectionKind,
}

/// Stable content-free failure while parsing or encoding a host message.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HostProtocolError {
    /// The encoded frame exceeded its directional bound.
    SizeExceeded,
    /// JSON or the closed message shape was malformed.
    Malformed,
    /// The message used an unsupported schema version.
    VersionMismatch,
    /// A bounded identifier or path component was invalid.
    InvalidValue,
}

impl HostProtocolError {
    /// Returns the stable redacted error code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::SizeExceeded => "host.protocol.size_exceeded",
            Self::Malformed => "host.protocol.malformed",
            Self::VersionMismatch => "host.protocol.version_mismatch",
            Self::InvalidValue => "host.protocol.value_invalid",
        }
    }
}

/// Exact request admitted from an authenticated VS Code peer.
#[derive(Clone, PartialEq, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum HostRequest {
    /// Build one exact local-only handoff review from trusted current session state.
    PreviewHandoff {
        /// Protocol schema version.
        schema_version: u16,
        /// Correlation identity selected by the shell.
        request_id: String,
    },
    /// Revalidate and render one exact reviewed packet locally.
    RenderHandoff {
        /// Protocol schema version.
        schema_version: u16,
        /// Correlation identity selected by the shell.
        request_id: String,
        /// Exact pending preview identity.
        preview_id: String,
        /// Digest of the exact review confirmed by the user.
        confirmation_sha256: String,
        /// Explicit acknowledgment for permitted non-public content.
        non_public_acknowledged: bool,
    },
    /// Cancel one pending handoff review without rendering content.
    CancelHandoff {
        /// Protocol schema version.
        schema_version: u16,
        /// Correlation identity selected by the shell.
        request_id: String,
        /// Exact pending preview identity.
        preview_id: String,
    },
    /// Record denial of one prohibited delivery or interface-control attempt.
    DenyHandoffAction {
        /// Protocol schema version.
        schema_version: u16,
        /// Correlation identity selected by the shell.
        request_id: String,
        /// Optional exact handoff identity.
        #[serde(deserialize_with = "deserialize_required_option")]
        handoff_id: Option<String>,
        /// Exact prohibited action attempted.
        action: HandoffProhibitedAction,
    },
    /// Return the current content-free exact-profile picker projection.
    DiscoverModels {
        /// Protocol schema version.
        schema_version: u16,
        /// Correlation identity selected by the shell.
        request_id: String,
    },
    /// Revalidate one exact displayed profile immediately before any use.
    RevalidateModel {
        /// Protocol schema version.
        schema_version: u16,
        /// Correlation identity selected by the shell.
        request_id: String,
        /// Exact user-selected profile identity.
        profile_id: String,
        /// Digest of the exact picker entry shown to the user.
        expected_entry_sha256: String,
    },
    /// Return one redacted local doctor report without contacting the network.
    Doctor {
        /// Protocol schema version.
        schema_version: u16,
        /// Correlation identity selected by the shell.
        request_id: String,
    },
    /// Preview one explicit local diagnostic export destination.
    PreviewDiagnosticExport {
        /// Protocol schema version.
        schema_version: u16,
        /// Correlation identity selected by the shell.
        request_id: String,
        /// User-selected absolute local JSON destination.
        destination: String,
    },
    /// Confirm one exact diagnostic export preview.
    ApproveDiagnosticExport {
        /// Protocol schema version.
        schema_version: u16,
        /// Correlation identity selected by the shell.
        request_id: String,
        /// Exact pending preview identity.
        preview_id: String,
        /// Digest of the exact preview confirmed by the user.
        confirmation_sha256: String,
    },
    /// Cancel one pending diagnostic export without writing a file.
    CancelDiagnosticExport {
        /// Protocol schema version.
        schema_version: u16,
        /// Correlation identity selected by the shell.
        request_id: String,
        /// Exact pending preview identity.
        preview_id: String,
    },
    /// Resolve and hold one file, then return a non-authoritative preview.
    PreviewRead {
        /// Protocol schema version.
        schema_version: u16,
        /// Correlation identity selected by the shell.
        request_id: String,
        /// Stable workspace identity selected by the shell.
        workspace_id: String,
        /// Absolute root supplied only from the selected local VS Code workspace.
        workspace_root: String,
        /// Already separated workspace-relative path components.
        components: Vec<String>,
    },
    /// Confirm one exact previously rendered preview.
    ApproveRead {
        /// Protocol schema version.
        schema_version: u16,
        /// Correlation identity selected by the shell.
        request_id: String,
        /// Exact pending preview identity.
        preview_id: String,
        /// Digest of the exact approval display confirmed by the user.
        confirmation_sha256: String,
    },
    /// Cancel one pending preview without deriving or consuming authority.
    CancelRead {
        /// Protocol schema version.
        schema_version: u16,
        /// Correlation identity selected by the shell.
        request_id: String,
        /// Exact pending preview identity.
        preview_id: String,
    },
    /// Preview one exact closed-catalog tool call and its complete projection manifest.
    PreviewTool {
        /// Protocol schema version.
        schema_version: u16,
        /// Correlation identity selected by the shell.
        request_id: String,
        /// Stable workspace identity selected by the shell.
        workspace_id: String,
        /// Absolute root supplied only from the selected local VS Code workspace.
        workspace_root: String,
        /// Exact closed-catalog tool identity.
        tool_id: String,
        /// Exact closed-catalog semantic version.
        tool_version: String,
        /// Exact JSON bytes represented as a string to retain duplicate-key detection.
        arguments_json: String,
        /// Complete bounded set of exact objects projected into the worker.
        projection: Vec<HostProjectionPath>,
    },
    /// Confirm one exact generic tool preview.
    ApproveTool {
        /// Protocol schema version.
        schema_version: u16,
        /// Correlation identity selected by the shell.
        request_id: String,
        /// Exact pending preview identity.
        preview_id: String,
        /// Digest of the exact approval display confirmed by the user.
        confirmation_sha256: String,
    },
    /// Cancel one pending generic tool preview without deriving authority.
    CancelTool {
        /// Protocol schema version.
        schema_version: u16,
        /// Correlation identity selected by the shell.
        request_id: String,
        /// Exact pending preview identity.
        preview_id: String,
    },
    /// Ask the trusted host to frame one exact native Chat runtime request.
    PrepareRuntime {
        /// Protocol schema version.
        schema_version: u16,
        /// Correlation identity selected by the shell.
        request_id: String,
        /// Exact user-selected profile identity.
        profile_id: String,
        /// Digest of the exact picker entry shown to the user.
        expected_entry_sha256: String,
        /// Stable workspace identity selected by the shell.
        workspace_id: String,
        /// Absolute root supplied only from the selected local VS Code workspace.
        workspace_root: String,
        /// Bounded user request, retained as inert task input.
        prompt: String,
    },
    /// Submit one host-framed request to the shared runtime coordinator.
    StartRuntime {
        /// Protocol schema version.
        schema_version: u16,
        /// Correlation identity selected by the shell.
        request_id: String,
        /// Exact request previously framed by the trusted host.
        run_request: Box<RuntimeRunRequest>,
    },
    /// Advance or replay one exact shared-runtime run.
    AdvanceRuntime {
        /// Protocol schema version.
        schema_version: u16,
        /// Correlation identity selected by the shell.
        request_id: String,
        /// Exact active runtime run.
        run_id: RuntimeRunId,
        /// Digest of the exact admitted runtime request.
        request_sha256: String,
        /// Exact last event already accepted by the shell, or `null` for none.
        #[serde(deserialize_with = "deserialize_required_option")]
        after_event_cursor: Option<RuntimeEventCursor>,
        /// Exact protected user response, or `null` for a replay-only poll.
        #[serde(deserialize_with = "deserialize_required_option")]
        approval_response: Option<RuntimeApprovalResponse>,
    },
    /// Cancel one exact shared-runtime run from the shell boundary.
    CancelRuntime {
        /// Protocol schema version.
        schema_version: u16,
        /// Correlation identity selected by the shell.
        request_id: String,
        /// Exact active runtime run.
        run_id: RuntimeRunId,
        /// Digest of the exact admitted runtime request.
        request_sha256: String,
        /// Stable cancellation-tree identity selected for this request.
        cancellation_id: CancellationId,
        /// Exact last event already accepted by the shell, or `null` for none.
        #[serde(deserialize_with = "deserialize_required_option")]
        after_event_cursor: Option<RuntimeEventCursor>,
    },
    /// Discard one unstarted request or remove one terminal runtime session.
    ReleaseRuntime {
        /// Protocol schema version.
        schema_version: u16,
        /// Correlation identity selected by the shell.
        request_id: String,
        /// Exact prepared or terminal runtime run.
        run_id: RuntimeRunId,
        /// Digest of the exact admitted runtime request.
        request_sha256: String,
    },
}

impl HostRequest {
    fn validate(&self) -> Result<(), HostProtocolError> {
        let (version, request_id) = match self {
            Self::PreviewHandoff {
                schema_version,
                request_id,
            } => (*schema_version, request_id),
            Self::RenderHandoff {
                schema_version,
                request_id,
                preview_id,
                confirmation_sha256,
                ..
            } => {
                if !valid_identifier(preview_id) || !valid_sha256(confirmation_sha256) {
                    return Err(HostProtocolError::InvalidValue);
                }
                (*schema_version, request_id)
            }
            Self::CancelHandoff {
                schema_version,
                request_id,
                preview_id,
            } => {
                if !valid_identifier(preview_id) {
                    return Err(HostProtocolError::InvalidValue);
                }
                (*schema_version, request_id)
            }
            Self::DenyHandoffAction {
                schema_version,
                request_id,
                handoff_id,
                ..
            } => {
                if handoff_id
                    .as_deref()
                    .is_some_and(|value| !valid_identifier(value))
                {
                    return Err(HostProtocolError::InvalidValue);
                }
                (*schema_version, request_id)
            }
            Self::DiscoverModels {
                schema_version,
                request_id,
            } => (*schema_version, request_id),
            Self::RevalidateModel {
                schema_version,
                request_id,
                profile_id,
                expected_entry_sha256,
            } => {
                if !valid_identifier(profile_id) || !valid_sha256(expected_entry_sha256) {
                    return Err(HostProtocolError::InvalidValue);
                }
                (*schema_version, request_id)
            }
            Self::Doctor {
                schema_version,
                request_id,
            } => (*schema_version, request_id),
            Self::PreviewDiagnosticExport {
                schema_version,
                request_id,
                destination,
            } => {
                if destination.is_empty() || destination.len() > 4_096 || destination.contains('\0')
                {
                    return Err(HostProtocolError::InvalidValue);
                }
                (*schema_version, request_id)
            }
            Self::ApproveDiagnosticExport {
                schema_version,
                request_id,
                preview_id,
                confirmation_sha256,
            } => {
                if !valid_identifier(preview_id) || !valid_sha256(confirmation_sha256) {
                    return Err(HostProtocolError::InvalidValue);
                }
                (*schema_version, request_id)
            }
            Self::CancelDiagnosticExport {
                schema_version,
                request_id,
                preview_id,
            } => {
                if !valid_identifier(preview_id) {
                    return Err(HostProtocolError::InvalidValue);
                }
                (*schema_version, request_id)
            }
            Self::PreviewRead {
                schema_version,
                request_id,
                workspace_id,
                workspace_root,
                components,
            } => {
                if !valid_identifier(workspace_id)
                    || workspace_root.is_empty()
                    || workspace_root.len() > 4_096
                    || components.is_empty()
                    || components.len() > MAX_COMPONENTS
                    || components.iter().any(|component| {
                        component.is_empty() || component.len() > MAX_COMPONENT_BYTES
                    })
                {
                    return Err(HostProtocolError::InvalidValue);
                }
                (*schema_version, request_id)
            }
            Self::ApproveRead {
                schema_version,
                request_id,
                preview_id,
                confirmation_sha256,
            } => {
                if !valid_identifier(preview_id) || !valid_sha256(confirmation_sha256) {
                    return Err(HostProtocolError::InvalidValue);
                }
                (*schema_version, request_id)
            }
            Self::CancelRead {
                schema_version,
                request_id,
                preview_id,
            } => {
                if !valid_identifier(preview_id) {
                    return Err(HostProtocolError::InvalidValue);
                }
                (*schema_version, request_id)
            }
            Self::PreviewTool {
                schema_version,
                request_id,
                workspace_id,
                workspace_root,
                tool_id,
                tool_version,
                arguments_json,
                projection,
            } => {
                if !valid_identifier(workspace_id)
                    || workspace_root.is_empty()
                    || workspace_root.len() > 4_096
                    || !valid_identifier(tool_id)
                    || !valid_semver(tool_version)
                    || arguments_json.is_empty()
                    || arguments_json.len() > MAX_TOOL_ARGUMENT_BYTES
                    || projection.is_empty()
                    || projection.len() > MAX_COMPONENTS
                    || projection
                        .iter()
                        .any(|item| !valid_components(&item.components))
                {
                    return Err(HostProtocolError::InvalidValue);
                }
                (*schema_version, request_id)
            }
            Self::ApproveTool {
                schema_version,
                request_id,
                preview_id,
                confirmation_sha256,
            } => {
                if !valid_identifier(preview_id) || !valid_sha256(confirmation_sha256) {
                    return Err(HostProtocolError::InvalidValue);
                }
                (*schema_version, request_id)
            }
            Self::CancelTool {
                schema_version,
                request_id,
                preview_id,
            } => {
                if !valid_identifier(preview_id) {
                    return Err(HostProtocolError::InvalidValue);
                }
                (*schema_version, request_id)
            }
            Self::PrepareRuntime {
                schema_version,
                request_id,
                profile_id,
                expected_entry_sha256,
                workspace_id,
                workspace_root,
                prompt,
            } => {
                if !valid_identifier(profile_id)
                    || !valid_sha256(expected_entry_sha256)
                    || !valid_identifier(workspace_id)
                    || workspace_root.is_empty()
                    || workspace_root.len() > 4_096
                    || workspace_root.contains('\0')
                    || prompt.trim().is_empty()
                    || prompt.len() > MAX_RUNTIME_PROMPT_BYTES
                    || prompt.contains('\0')
                {
                    return Err(HostProtocolError::InvalidValue);
                }
                (*schema_version, request_id)
            }
            Self::StartRuntime {
                schema_version,
                request_id,
                run_request,
            } => {
                verify_runtime_run_request(run_request)
                    .map_err(|_| HostProtocolError::InvalidValue)?;
                (*schema_version, request_id)
            }
            Self::AdvanceRuntime {
                schema_version,
                request_id,
                run_id,
                request_sha256,
                after_event_cursor,
                approval_response,
                ..
            } => {
                if !valid_identifier(run_id.as_str())
                    || !valid_sha256(request_sha256)
                    || after_event_cursor.as_ref().is_some_and(|cursor| {
                        cursor.run_id != *run_id
                            || !valid_identifier(cursor.event_id.as_str())
                            || !valid_sha256(&cursor.event_sha256)
                    })
                    || approval_response.as_ref().is_some_and(|response| {
                        response.run_id != *run_id
                            || !valid_identifier(response.approval_id.as_str())
                            || !valid_sha256(&response.challenge_sha256)
                            || response
                                .grant_id
                                .as_ref()
                                .is_some_and(|grant_id| !valid_identifier(grant_id.as_str()))
                    })
                {
                    return Err(HostProtocolError::InvalidValue);
                }
                (*schema_version, request_id)
            }
            Self::CancelRuntime {
                schema_version,
                request_id,
                run_id,
                request_sha256,
                cancellation_id,
                after_event_cursor,
            } => {
                if !valid_identifier(run_id.as_str())
                    || !valid_sha256(request_sha256)
                    || !valid_identifier(cancellation_id.as_str())
                    || after_event_cursor.as_ref().is_some_and(|cursor| {
                        cursor.run_id != *run_id
                            || !valid_identifier(cursor.event_id.as_str())
                            || !valid_sha256(&cursor.event_sha256)
                    })
                {
                    return Err(HostProtocolError::InvalidValue);
                }
                (*schema_version, request_id)
            }
            Self::ReleaseRuntime {
                schema_version,
                request_id,
                run_id,
                request_sha256,
            } => {
                if !valid_identifier(run_id.as_str()) || !valid_sha256(request_sha256) {
                    return Err(HostProtocolError::InvalidValue);
                }
                (*schema_version, request_id)
            }
        };
        if version != HOST_PROTOCOL_VERSION {
            return Err(HostProtocolError::VersionMismatch);
        }
        if !valid_identifier(request_id) {
            return Err(HostProtocolError::InvalidValue);
        }
        Ok(())
    }
}

/// Content-free receipt fields returned to the UI after a terminal operation.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ReceiptSummary {
    /// Stable receipt identity.
    pub receipt_id: String,
    /// Monotonic sequence in the durable receipt chain.
    pub sequence: u64,
    /// Digest of the exact canonical receipt.
    pub receipt_sha256: String,
    /// Stable terminal outcome.
    pub outcome: String,
}

/// Exact bounded response returned by the host.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum HostResponse {
    /// Mandatory exact local-only handoff review.
    HandoffPreview {
        /// Protocol schema version.
        schema_version: u16,
        /// Correlation identity from the request.
        request_id: String,
        /// Kernel-built exact packet review and manifest.
        review: HandoffReview,
    },
    /// Exact locally rendered handoff and local-only receipt.
    HandoffRendered {
        /// Protocol schema version.
        schema_version: u16,
        /// Correlation identity from the request.
        request_id: String,
        /// Rendered packet, manifest, and no-delivery receipt.
        rendered: RenderedHandoff,
    },
    /// Local cancellation or prohibited-action denial receipt.
    HandoffReceipt {
        /// Protocol schema version.
        schema_version: u16,
        /// Correlation identity from the request.
        request_id: String,
        /// Content-free local-only receipt.
        receipt: LocalHandoffReceipt,
    },
    /// Current exact-profile discovery result from trusted host composition.
    ModelsDiscovered {
        /// Protocol schema version.
        schema_version: u16,
        /// Correlation identity from the request.
        request_id: String,
        /// Signed-catalog-bound content-free picker projection.
        snapshot: ModelPickerSnapshot,
    },
    /// Current exact-profile selection revalidation result.
    ModelRevalidated {
        /// Protocol schema version.
        schema_version: u16,
        /// Correlation identity from the request.
        request_id: String,
        /// Kernel-computed exact-entry result with no fallback.
        revalidation: ModelSelectionRevalidation,
    },
    /// One typed redacted local doctor report.
    DoctorCompleted {
        /// Protocol schema version.
        schema_version: u16,
        /// Correlation identity from the request.
        request_id: String,
        /// Kernel-owned content-free diagnostic report.
        report: DoctorReport,
    },
    /// One exact non-authoritative diagnostic export preview.
    DiagnosticExportPreview {
        /// Protocol schema version.
        schema_version: u16,
        /// Correlation identity from the request.
        request_id: String,
        /// Opaque pending-preview identity.
        preview_id: String,
        /// Digest of the selected canonical destination.
        destination_sha256: String,
        /// Digest of the exact export bytes.
        payload_sha256: String,
        /// Exact export byte count.
        payload_bytes: u64,
        /// Stable included field families.
        included_fields: Vec<String>,
        /// Stable redaction families.
        redactions: Vec<String>,
        /// Stable sensitivity label.
        sensitivity: String,
        /// Stable retention instruction.
        retention: String,
        /// Preview expiry.
        expires_at_epoch_ms: u64,
        /// Digest of the complete preview confirmation.
        confirmation_sha256: String,
    },
    /// One completed local diagnostic export.
    DiagnosticExportCompleted {
        /// Protocol schema version.
        schema_version: u16,
        /// Correlation identity from the request.
        request_id: String,
        /// Digest of the selected canonical destination.
        destination_sha256: String,
        /// Digest of the exact published bytes.
        payload_sha256: String,
        /// Exact published byte count.
        payload_bytes: u64,
        /// Stable terminal outcome.
        outcome: String,
    },
    /// One exact non-authoritative read preview.
    ReadPreview {
        /// Protocol schema version.
        schema_version: u16,
        /// Correlation identity from the request.
        request_id: String,
        /// Opaque pending-preview identity.
        preview_id: String,
        /// Canonical relative components rendered to the user.
        components: Vec<String>,
        /// Exact currently observed file byte count.
        byte_len: u64,
        /// Exact currently observed content digest.
        content_sha256: String,
        /// Expiration of this memory-only preview.
        expires_at_epoch_ms: u64,
        /// Digest of the complete approval display.
        confirmation_sha256: String,
    },
    /// One completed bounded UTF-8 read and its durable receipt.
    ReadCompleted {
        /// Protocol schema version.
        schema_version: u16,
        /// Correlation identity from the request.
        request_id: String,
        /// Bounded UTF-8 content from the isolated worker.
        content: String,
        /// Non-authoritative local file URI derived from the held object.
        file_uri: String,
        /// Durable content-free receipt fields.
        receipt: ReceiptSummary,
    },
    /// One exact non-authoritative generic tool preview.
    ToolPreview {
        /// Protocol schema version.
        schema_version: u16,
        /// Correlation identity from the request.
        request_id: String,
        /// Opaque pending-preview identity.
        preview_id: String,
        /// Exact closed-catalog tool identity.
        tool_id: String,
        /// Exact closed-catalog tool version.
        tool_version: String,
        /// Count of exact continuously held projected objects.
        projected_objects: u32,
        /// Digest of the ordered exact authority targets.
        target_set_sha256: String,
        /// Expiration of this memory-only preview.
        expires_at_epoch_ms: u64,
        /// Digest of the complete approval display.
        confirmation_sha256: String,
    },
    /// One verified typed tool result and its durable content-free receipt.
    ToolCompleted {
        /// Protocol schema version.
        schema_version: u16,
        /// Correlation identity from the request.
        request_id: String,
        /// Hash-verified result from the isolated worker.
        result: ReadOnlyResult,
        /// Durable content-free receipt fields.
        receipt: ReceiptSummary,
    },
    /// One exact reusable-runtime request framed by the trusted host.
    RuntimePrepared {
        /// Protocol schema version.
        schema_version: u16,
        /// Correlation identity from the request.
        request_id: String,
        /// Exact request that may be submitted unchanged to the shared coordinator.
        run_request: Box<RuntimeRunRequest>,
    },
    /// One ordered shared-runtime boundary projection.
    RuntimeStep {
        /// Protocol schema version.
        schema_version: u16,
        /// Correlation identity from the request.
        request_id: String,
        /// Exact active runtime run.
        run_id: RuntimeRunId,
        /// Digest of the exact admitted runtime request.
        request_sha256: String,
        /// Ordered verified events after the shell's supplied cursor.
        events: Vec<RuntimeEvent>,
        /// Complete verified artifact-reference set currently owned by the coordinator.
        artifacts: Vec<RuntimeArtifactRef>,
        /// Exact protected challenge only while the coordinator is waiting.
        #[serde(deserialize_with = "deserialize_required_option")]
        approval: Option<RuntimeApprovalChallenge>,
        /// Canonical outcome only after the coordinator reaches a terminal state.
        #[serde(deserialize_with = "deserialize_required_option")]
        outcome: Option<Box<RuntimeOutcome>>,
    },
    /// A pending preview was cancelled before execution.
    Cancelled {
        /// Protocol schema version.
        schema_version: u16,
        /// Correlation identity from the request.
        request_id: String,
    },
    /// A request failed closed with no partial content.
    Denied {
        /// Protocol schema version.
        schema_version: u16,
        /// Correlation identity from the request when valid.
        request_id: String,
        /// Stable content-free denial code.
        code: String,
        /// Retained receipt identity for a completed replay, when available.
        #[serde(skip_serializing_if = "Option::is_none")]
        receipt: Option<ReceiptSummary>,
    },
}

/// Parses one exact request after an authenticated channel enforces framing.
pub fn parse_request(bytes: &[u8]) -> Result<HostRequest, HostProtocolError> {
    if bytes.is_empty() || bytes.len() > MAX_HOST_REQUEST_BYTES {
        return Err(HostProtocolError::SizeExceeded);
    }
    let request: HostRequest =
        serde_json::from_slice(bytes).map_err(|_| HostProtocolError::Malformed)?;
    request.validate()?;
    Ok(request)
}

/// Encodes one bounded response for the authenticated VS Code peer.
pub fn encode_response(response: &HostResponse) -> Result<Vec<u8>, HostProtocolError> {
    let bytes = serde_json::to_vec(response).map_err(|_| HostProtocolError::Malformed)?;
    if bytes.len() > MAX_HOST_RESPONSE_BYTES {
        return Err(HostProtocolError::SizeExceeded);
    }
    Ok(bytes)
}

fn valid_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_IDENTIFIER_BYTES
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b':' | b'-'))
}

fn deserialize_required_option<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(deserializer)
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn valid_components(components: &[String]) -> bool {
    !components.is_empty()
        && components.len() <= MAX_COMPONENTS
        && components.iter().all(|component| {
            !component.is_empty()
                && component.len() <= MAX_COMPONENT_BYTES
                && component != "."
                && component != ".."
                && !component.contains('/')
                && !component.contains('\0')
        })
}

fn valid_semver(value: &str) -> bool {
    let parts = value.split('.').collect::<Vec<_>>();
    value.len() <= 64
        && parts.len() == 3
        && parts
            .iter()
            .all(|part| !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit()))
}

#[cfg(test)]
mod tests {
    use agentmage_kernel_contracts::{
        CONTRACT_SCHEMA_VERSION, RuntimeApprovalDisposition, RuntimeApprovalResponse,
    };
    use serde_json::json;

    use super::{
        HOST_PROTOCOL_VERSION, HostProjectionKind, HostProjectionPath, HostProtocolError,
        HostRequest, HostResponse, MAX_HOST_REQUEST_BYTES, encode_response, parse_request,
    };

    fn preview_request() -> serde_json::Value {
        json!({
            "kind": "preview_read",
            "schema_version": HOST_PROTOCOL_VERSION,
            "request_id": "request-0001",
            "workspace_id": "workspace-0001",
            "workspace_root": "/tmp/workspace",
            "components": ["src", "lib.rs"]
        })
    }

    #[test]
    fn exact_preview_request_round_trips() {
        let bytes = serde_json::to_vec(&preview_request()).expect("request JSON");
        let parsed = parse_request(&bytes).expect("exact request");
        assert!(matches!(parsed, HostRequest::PreviewRead { .. }));
    }

    #[test]
    fn generic_tool_request_preserves_exact_argument_bytes_and_projection_kind() {
        let request = HostRequest::PreviewTool {
            schema_version: HOST_PROTOCOL_VERSION,
            request_id: "request-tool-0001".to_owned(),
            workspace_id: "workspace-0001".to_owned(),
            workspace_root: "/tmp/workspace".to_owned(),
            tool_id: "agentmage.workspace.search-text".to_owned(),
            tool_version: "1.0.0".to_owned(),
            arguments_json: r#"{"schema_version":1}"#.to_owned(),
            projection: vec![HostProjectionPath {
                components: vec!["src".to_owned(), "lib.rs".to_owned()],
                object_kind: HostProjectionKind::RegularFile,
            }],
        };
        let bytes = serde_json::to_vec(&request).expect("request JSON");
        let parsed = parse_request(&bytes).expect("generic request");
        assert!(matches!(
            parsed,
            HostRequest::PreviewTool {
                ref arguments_json,
                ref projection,
                ..
            } if arguments_json == r#"{"schema_version":1}"#
                && projection[0].object_kind == HostProjectionKind::RegularFile
        ));

        let mut invalid: serde_json::Value = serde_json::from_slice(&bytes).expect("value");
        invalid["projection"][0]["components"] = json!([".."]);
        assert!(matches!(
            parse_request(&serde_json::to_vec(&invalid).expect("invalid JSON")),
            Err(HostProtocolError::InvalidValue)
        ));
    }

    #[test]
    fn exact_doctor_request_round_trips_without_probe_fields() {
        let request = json!({
            "kind": "doctor",
            "schema_version": HOST_PROTOCOL_VERSION,
            "request_id": "request-doctor-0001"
        });
        let parsed = parse_request(&serde_json::to_vec(&request).expect("request JSON"))
            .expect("exact request");
        assert!(matches!(parsed, HostRequest::Doctor { .. }));

        let mut prohibited = request;
        prohibited["endpoint"] = json!("https://example.invalid/health");
        assert!(matches!(
            parse_request(&serde_json::to_vec(&prohibited).expect("request JSON")),
            Err(HostProtocolError::Malformed)
        ));
    }

    #[test]
    fn model_discovery_and_revalidation_requests_are_closed() {
        let discovery = json!({
            "kind": "discover_models",
            "schema_version": HOST_PROTOCOL_VERSION,
            "request_id": "request-models-0001"
        });
        assert!(matches!(
            parse_request(&serde_json::to_vec(&discovery).expect("request JSON")),
            Ok(HostRequest::DiscoverModels { .. })
        ));

        let revalidation = json!({
            "kind": "revalidate_model",
            "schema_version": HOST_PROTOCOL_VERSION,
            "request_id": "request-models-0002",
            "profile_id": "exact-profile-0001",
            "expected_entry_sha256": "a".repeat(64)
        });
        assert!(matches!(
            parse_request(&serde_json::to_vec(&revalidation).expect("request JSON")),
            Ok(HostRequest::RevalidateModel { .. })
        ));
        let mut hidden_fallback = revalidation.clone();
        hidden_fallback["fallback_profile_id"] = json!("other-profile");
        assert!(matches!(
            parse_request(&serde_json::to_vec(&hidden_fallback).expect("request JSON")),
            Err(HostProtocolError::Malformed)
        ));
        let mut invalid_digest = revalidation;
        invalid_digest["expected_entry_sha256"] = json!("mutable-tag");
        assert!(matches!(
            parse_request(&serde_json::to_vec(&invalid_digest).expect("request JSON")),
            Err(HostProtocolError::InvalidValue)
        ));
    }

    #[test]
    fn native_chat_runtime_requests_are_closed_exact_and_host_framed() {
        let prepare = json!({
            "kind": "prepare_runtime",
            "schema_version": HOST_PROTOCOL_VERSION,
            "request_id": "request-runtime-0001",
            "profile_id": "exact-profile-0001",
            "expected_entry_sha256": "a".repeat(64),
            "workspace_id": "workspace-0001",
            "workspace_root": "/tmp/workspace",
            "prompt": "Inspect the bounded repository"
        });
        assert!(matches!(
            parse_request(&serde_json::to_vec(&prepare).expect("request JSON")),
            Ok(HostRequest::PrepareRuntime { .. })
        ));
        let mut endpoint_injection = prepare.clone();
        endpoint_injection["model_endpoint"] = json!("https://example.invalid");
        assert!(matches!(
            parse_request(&serde_json::to_vec(&endpoint_injection).expect("request JSON")),
            Err(HostProtocolError::Malformed)
        ));

        let (_, run_request) = crate::coding_run::tests::fixture_profile_and_request();
        let start = HostRequest::StartRuntime {
            schema_version: HOST_PROTOCOL_VERSION,
            request_id: "request-runtime-0002".to_owned(),
            run_request: Box::new(run_request.clone()),
        };
        assert!(matches!(
            parse_request(&serde_json::to_vec(&start).expect("request JSON")),
            Ok(HostRequest::StartRuntime { .. })
        ));

        let advance = HostRequest::AdvanceRuntime {
            schema_version: HOST_PROTOCOL_VERSION,
            request_id: "request-runtime-0003".to_owned(),
            run_id: run_request.run_id.clone(),
            request_sha256: run_request.request_sha256.clone(),
            after_event_cursor: Some(agentmage_kernel_contracts::RuntimeEventCursor {
                run_id: run_request.run_id.clone(),
                event_id: agentmage_kernel_contracts::RuntimeEventId::from_raw(
                    "runtime-event-0007",
                ),
                sequence: 7,
                event_sha256: "c".repeat(64),
            }),
            approval_response: Some(RuntimeApprovalResponse {
                schema_version: CONTRACT_SCHEMA_VERSION,
                run_id: run_request.run_id.clone(),
                approval_id: agentmage_kernel_contracts::ApprovalId::from_raw(
                    "runtime-approval-0001",
                ),
                disposition: RuntimeApprovalDisposition::Deny,
                challenge_sha256: "b".repeat(64),
                grant_id: None,
            }),
        };
        let encoded = serde_json::to_vec(&advance).expect("advance JSON");
        assert!(matches!(
            parse_request(&encoded),
            Ok(HostRequest::AdvanceRuntime {
                after_event_cursor: Some(agentmage_kernel_contracts::RuntimeEventCursor {
                    sequence: 7,
                    ..
                }),
                approval_response: Some(_),
                ..
            })
        ));
        let mut hidden_authority: serde_json::Value =
            serde_json::from_slice(&encoded).expect("advance value");
        hidden_authority["approval_response"]["policy_override"] = json!(true);
        assert!(matches!(
            parse_request(&serde_json::to_vec(&hidden_authority).expect("request JSON")),
            Err(HostProtocolError::Malformed)
        ));

        let cancel = json!({
            "kind": "cancel_runtime",
            "schema_version": HOST_PROTOCOL_VERSION,
            "request_id": "request-runtime-0004",
            "run_id": run_request.run_id,
            "request_sha256": run_request.request_sha256,
            "cancellation_id": "runtime-cancellation-0001",
            "after_event_cursor": null
        });
        assert!(matches!(
            parse_request(&serde_json::to_vec(&cancel).expect("request JSON")),
            Ok(HostRequest::CancelRuntime {
                after_event_cursor: None,
                ..
            })
        ));

        let release = json!({
            "kind": "release_runtime",
            "schema_version": HOST_PROTOCOL_VERSION,
            "request_id": "request-runtime-0005",
            "run_id": run_request.run_id,
            "request_sha256": run_request.request_sha256
        });
        assert!(matches!(
            parse_request(&serde_json::to_vec(&release).expect("request JSON")),
            Ok(HostRequest::ReleaseRuntime { .. })
        ));
    }

    #[test]
    fn handoff_requests_are_local_closed_and_confirmation_bound() {
        let preview = json!({
            "kind": "preview_handoff",
            "schema_version": HOST_PROTOCOL_VERSION,
            "request_id": "request-handoff-0001"
        });
        assert!(matches!(
            parse_request(&serde_json::to_vec(&preview).expect("request JSON")),
            Ok(HostRequest::PreviewHandoff { .. })
        ));
        let render = json!({
            "kind": "render_handoff",
            "schema_version": HOST_PROTOCOL_VERSION,
            "request_id": "request-handoff-0002",
            "preview_id": "preview-handoff-0001",
            "confirmation_sha256": "a".repeat(64),
            "non_public_acknowledged": true
        });
        assert!(matches!(
            parse_request(&serde_json::to_vec(&render).expect("request JSON")),
            Ok(HostRequest::RenderHandoff { .. })
        ));
        let denial = json!({
            "kind": "deny_handoff_action",
            "schema_version": HOST_PROTOCOL_VERSION,
            "request_id": "request-handoff-0003",
            "handoff_id": null,
            "action": "clipboard_write"
        });
        assert!(matches!(
            parse_request(&serde_json::to_vec(&denial).expect("request JSON")),
            Ok(HostRequest::DenyHandoffAction { .. })
        ));

        let mut prohibited_transfer = render.clone();
        prohibited_transfer["endpoint"] = json!("https://example.invalid/submit");
        assert!(matches!(
            parse_request(&serde_json::to_vec(&prohibited_transfer).expect("request JSON")),
            Err(HostProtocolError::Malformed)
        ));
        let mut omitted_required_null = denial;
        omitted_required_null
            .as_object_mut()
            .expect("object")
            .remove("handoff_id");
        assert!(matches!(
            parse_request(&serde_json::to_vec(&omitted_required_null).expect("request JSON")),
            Err(HostProtocolError::Malformed)
        ));
    }

    #[test]
    fn diagnostic_export_protocol_is_closed_and_confirmation_bound() {
        let preview = json!({
            "kind": "preview_diagnostic_export",
            "schema_version": HOST_PROTOCOL_VERSION,
            "request_id": "request-export-0001",
            "destination": "/var/home/user/private/doctor.json"
        });
        assert!(matches!(
            parse_request(&serde_json::to_vec(&preview).expect("request JSON")),
            Ok(HostRequest::PreviewDiagnosticExport { .. })
        ));
        let approval = json!({
            "kind": "approve_diagnostic_export",
            "schema_version": HOST_PROTOCOL_VERSION,
            "request_id": "request-export-0002",
            "preview_id": "diagnostic-export-0001",
            "confirmation_sha256": "a".repeat(64)
        });
        assert!(matches!(
            parse_request(&serde_json::to_vec(&approval).expect("request JSON")),
            Ok(HostRequest::ApproveDiagnosticExport { .. })
        ));
        let mut malformed = approval;
        malformed["grant"] = json!(true);
        assert!(matches!(
            parse_request(&serde_json::to_vec(&malformed).expect("request JSON")),
            Err(HostProtocolError::Malformed)
        ));
    }

    #[test]
    fn unknown_version_field_and_oversize_fail_closed() {
        let mut version = preview_request();
        version["schema_version"] = json!(HOST_PROTOCOL_VERSION + 1);
        assert!(matches!(
            parse_request(&serde_json::to_vec(&version).expect("JSON")),
            Err(HostProtocolError::VersionMismatch)
        ));

        let mut unknown = preview_request();
        unknown["ambient_path"] = json!("/etc/passwd");
        assert!(matches!(
            parse_request(&serde_json::to_vec(&unknown).expect("JSON")),
            Err(HostProtocolError::Malformed)
        ));

        assert!(matches!(
            parse_request(&vec![b'x'; MAX_HOST_REQUEST_BYTES + 1]),
            Err(HostProtocolError::SizeExceeded)
        ));
    }

    #[test]
    fn malformed_values_and_digest_are_rejected() {
        let mut traversal = preview_request();
        traversal["workspace_id"] = json!("workspace/*");
        assert!(matches!(
            parse_request(&serde_json::to_vec(&traversal).expect("JSON")),
            Err(HostProtocolError::InvalidValue)
        ));

        let approval = json!({
            "kind": "approve_read",
            "schema_version": HOST_PROTOCOL_VERSION,
            "request_id": "request-0002",
            "preview_id": "preview-0001",
            "confirmation_sha256": "not-a-digest"
        });
        assert!(matches!(
            parse_request(&serde_json::to_vec(&approval).expect("JSON")),
            Err(HostProtocolError::InvalidValue)
        ));
    }

    #[test]
    fn denial_response_has_no_unbounded_error_detail() {
        let bytes = encode_response(&HostResponse::Denied {
            schema_version: HOST_PROTOCOL_VERSION,
            request_id: "request-0001".to_owned(),
            code: "host.read.approval_required".to_owned(),
            receipt: None,
        })
        .expect("bounded response");
        let value: serde_json::Value = serde_json::from_slice(&bytes).expect("response JSON");
        assert_eq!(value["code"], "host.read.approval_required");
        assert!(value.get("detail").is_none());
    }
}
