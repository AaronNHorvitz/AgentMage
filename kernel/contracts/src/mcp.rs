//! Closed read-only Model Context Protocol boundary contracts.

use crate::{
    CancellationId, ContextSensitivity, ContractPayload, GrantOperation, ReceiptId,
    SchemaReference, StateChange, ToolCall, ToolDefinition,
};

/// Visibly distinct MCP transport family.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum McpTransportKind {
    /// Reviewed code loaded in the AgentMage process.
    InProcess,
    /// One owned local child process using standard input and output.
    LocalProcessStdio,
    /// One authenticated local operating-system socket.
    LocalSocket,
    /// One exact loopback TCP endpoint.
    LoopbackTcp,
    /// One exact remote HTTPS endpoint.
    RemoteHttps,
}

/// Exact transport identity declared by one MCP manifest.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct McpTransport {
    /// Closed transport family.
    pub kind: McpTransportKind,
    /// Digest of executable transport configuration and endpoint identity.
    pub endpoint_identity_sha256: String,
    /// Exact destination identity, empty only for in-process or standard-I/O transport.
    pub destination: String,
}

/// Inclusive limits for one MCP server registration and request.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct McpLimits {
    /// Maximum registered tools.
    pub max_tools: u32,
    /// Maximum registered resources.
    pub max_resources: u32,
    /// Maximum registered prompts.
    pub max_prompts: u32,
    /// Maximum request bytes.
    pub max_request_bytes: u64,
    /// Maximum response bytes.
    pub max_response_bytes: u64,
    /// Maximum response items.
    pub max_response_items: u32,
    /// Maximum elapsed request time in milliseconds.
    pub timeout_ms: u64,
    /// Maximum simultaneous requests to this server.
    pub max_concurrency: u16,
}

/// Closed response classification declared before an MCP call.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum McpResponseClass {
    /// Bounded untrusted text.
    UntrustedText,
    /// Bounded untrusted structured data.
    UntrustedStructuredData,
    /// Bounded read-only resource content.
    UntrustedResource,
    /// Bounded untrusted prompt template.
    UntrustedPrompt,
}

/// One read-only MCP tool declaration adapted into the common tool registry.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct McpToolManifest {
    /// Complete common tool definition.
    pub definition: ToolDefinition,
    /// Declared response classification.
    pub response_class: McpResponseClass,
    /// Declared side-effect state, required to be `NotChanged`.
    pub side_effect: StateChange,
}

/// One read-only MCP resource declaration.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct McpResourceManifest {
    /// Stable resource identity.
    pub resource_id: String,
    /// Digest of the exact allowed resource scope.
    pub scope_sha256: String,
    /// Schema for returned content.
    pub response_schema: SchemaReference,
    /// Declared response classification.
    pub response_class: McpResponseClass,
}

/// One authority-free MCP prompt declaration.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct McpPromptManifest {
    /// Stable prompt identity.
    pub prompt_id: String,
    /// Schema for bounded prompt arguments.
    pub request_schema: SchemaReference,
    /// Schema for the returned template.
    pub response_schema: SchemaReference,
    /// Declared response classification.
    pub response_class: McpResponseClass,
}

/// Complete immutable declaration for one optional MCP server.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct McpManifest {
    /// Contract schema version.
    pub schema_version: u16,
    /// Stable server identity.
    pub server_id: String,
    /// Exact server package version.
    pub server_version: String,
    /// Digest of the complete installed package artifact.
    pub package_sha256: String,
    /// Digest of the exact executable and launch identity.
    pub process_sha256: String,
    /// Exact transport declaration.
    pub transport: McpTransport,
    /// Sorted read-only tool declarations.
    pub tools: Vec<McpToolManifest>,
    /// Sorted read-only resource declarations.
    pub resources: Vec<McpResourceManifest>,
    /// Sorted authority-free prompt declarations.
    pub prompts: Vec<McpPromptManifest>,
    /// Sorted exact workspace-root scope digests.
    pub workspace_root_sha256s: Vec<String>,
    /// Sorted exact network destination identities.
    pub network_destinations: Vec<String>,
    /// Sorted brokered credential identities, never credential values.
    pub credential_ids: Vec<String>,
    /// Closed server and request ceilings.
    pub limits: McpLimits,
    /// Whether the server contract supports request cancellation.
    pub cancellation_supported: bool,
    /// Sorted exact operations requested by this manifest.
    pub requested_operations: Vec<GrantOperation>,
    /// Digest of this manifest with this field set to all zeroes.
    pub manifest_sha256: String,
}

/// Bound connection record created only after manifest and process verification.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct McpConnection {
    /// Stable connection identity.
    pub connection_id: String,
    /// Exact server identity.
    pub server_id: String,
    /// Exact admitted manifest digest.
    pub manifest_sha256: String,
    /// Package digest observed at this connection.
    pub observed_package_sha256: String,
    /// Process digest observed at this connection.
    pub observed_process_sha256: String,
    /// Connection time in Unix epoch milliseconds.
    pub connected_at_epoch_ms: u64,
    /// Exclusive connection expiry in Unix epoch milliseconds.
    pub expires_at_epoch_ms: u64,
    /// Digest of this record with this field set to all zeroes.
    pub connection_sha256: String,
}

/// Discovery response bound to one admitted manifest and connection.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct McpDiscovery {
    /// Exact connection identity.
    pub connection_id: String,
    /// Exact admitted manifest digest.
    pub manifest_sha256: String,
    /// Sorted tool identities and immutable versions.
    pub tool_ids: Vec<String>,
    /// Sorted resource identities.
    pub resource_ids: Vec<String>,
    /// Sorted prompt identities.
    pub prompt_ids: Vec<String>,
    /// Digest of this discovery result with this field set to all zeroes.
    pub discovery_sha256: String,
}

/// Closed MCP request family.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum McpRequestKind {
    /// Invoke one common-registry read-only tool call.
    ToolCall,
    /// Read one declared resource.
    ResourceRead,
    /// Retrieve one declared authority-free prompt template.
    PromptGet,
}

/// One bounded request sent only through the MCP gateway.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct McpRequest {
    /// Stable request identity.
    pub request_id: String,
    /// Exact admitted connection.
    pub connection_id: String,
    /// Closed request family.
    pub kind: McpRequestKind,
    /// Exact declared capability identity.
    pub capability_id: String,
    /// Tool call for `ToolCall`, absent for every other request family.
    #[serde(deserialize_with = "crate::serialization::deserialize_required_option")]
    pub tool_call: Option<ToolCall>,
    /// Schema-bound arguments for resource and prompt requests.
    #[serde(deserialize_with = "crate::serialization::deserialize_required_option")]
    pub arguments: Option<ContractPayload>,
    /// Caller-selected response byte ceiling no broader than the manifest.
    pub max_response_bytes: u64,
    /// Caller-selected item ceiling no broader than the manifest.
    pub max_response_items: u32,
    /// Caller-selected timeout no broader than the manifest.
    pub timeout_ms: u64,
    /// Digest of this request with this field set to all zeroes.
    pub request_sha256: String,
}

/// Closed terminal disposition for an MCP request.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum McpTerminalState {
    /// A bounded response passed gateway validation.
    Succeeded,
    /// Policy or exact scope denied the request before dispatch.
    Denied,
    /// The server or gateway failed with no claimed success.
    Failed,
    /// Cancellation was observed and server work terminated.
    Cancelled,
    /// The exact timeout elapsed and server work terminated.
    TimedOut,
    /// The connection ended before a valid terminal response.
    Disconnected,
}

/// One bounded, untrusted MCP response.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct McpResponse {
    /// Exact request identity.
    pub request_id: String,
    /// Terminal request disposition.
    pub state: McpTerminalState,
    /// Declared response class.
    pub response_class: McpResponseClass,
    /// Bounded payload only for a successful response.
    #[serde(deserialize_with = "crate::serialization::deserialize_required_option")]
    pub payload: Option<ContractPayload>,
    /// Number of logical response items.
    pub item_count: u32,
    /// Classification assigned before any downstream use.
    pub sensitivity: ContextSensitivity,
    /// Stable content-free error code for non-success outcomes.
    #[serde(deserialize_with = "crate::serialization::deserialize_required_option")]
    pub error_code: Option<String>,
    /// Digest of this response with this field set to all zeroes.
    pub response_sha256: String,
}

/// Content-free MCP protocol or gateway error.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct McpError {
    /// Exact request identity when one exists.
    pub request_id: Option<String>,
    /// Stable bounded error code.
    pub code: String,
    /// Whether retry is permitted only by a separately owned workflow policy.
    pub retryable: bool,
}

/// Exact cancellation request for one pending MCP operation.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct McpCancellation {
    /// Stable cancellation identity.
    pub cancellation_id: CancellationId,
    /// Exact request being cancelled.
    pub request_id: String,
    /// Exact admitted connection.
    pub connection_id: String,
}

/// Terminal disconnect record that invalidates the connection identity.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct McpDisconnect {
    /// Exact connection identity.
    pub connection_id: String,
    /// Stable content-free disconnect reason.
    pub reason_code: String,
    /// Whether owned descendant processes were terminated.
    pub descendants_terminated: bool,
}

/// Closed attributable MCP lifecycle receipt family.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum McpReceiptKind {
    /// Manifest verification completed.
    ManifestVerified,
    /// Connection identity was admitted.
    Connected,
    /// Discovery metadata was verified.
    Discovered,
    /// A request was admitted or denied.
    Request,
    /// A response was classified or rejected.
    Response,
    /// Cancellation was observed.
    Cancelled,
    /// A failure terminated the operation.
    Failed,
    /// The server disconnected and authority was invalidated.
    Disconnected,
}

/// One content-minimized MCP lifecycle receipt.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct McpReceipt {
    /// Stable receipt identity.
    pub receipt_id: ReceiptId,
    /// Closed lifecycle event.
    pub kind: McpReceiptKind,
    /// Exact server identity.
    pub server_id: String,
    /// Optional connection identity.
    pub connection_id: Option<String>,
    /// Optional request identity.
    pub request_id: Option<String>,
    /// Exact manifest digest.
    pub manifest_sha256: String,
    /// Terminal disposition where applicable.
    pub terminal_state: Option<McpTerminalState>,
    /// Whether any state change was observed, required to remain `NotChanged`.
    pub side_effect: StateChange,
    /// Digest of this receipt with this field set to all zeroes.
    pub receipt_sha256: String,
}
