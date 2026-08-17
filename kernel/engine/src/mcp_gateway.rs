//! Stateful read-only MCP request mediation, response validation, and receipts.

use std::collections::BTreeSet;

use agentmage_kernel_contracts::{
    ContextSensitivity, McpCancellation, McpConnection, McpDisconnect, McpManifest, McpReceipt,
    McpReceiptKind, McpRequest, McpRequestKind, McpResponse, McpResponseClass, McpTerminalState,
    ReceiptId, StateChange,
};
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::{
    mcp_registry::{
        McpProcessObservation, McpRegistryError, mcp_discovery, verify_mcp_connection,
        verify_mcp_manifest,
    },
    tooling::ToolRegistry,
};

const ZERO_SHA256: &str = "0000000000000000000000000000000000000000000000000000000000000000";
const MAX_ID_BYTES: usize = 128;

/// Stable fail-closed request-mediation error.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum McpGatewayError {
    /// Manifest or connection identity is stale, expired, or invalid.
    IdentityDenied,
    /// Request shape, schema, scope, limits, digest, or registration is invalid.
    RequestDenied,
    /// Response shape, schema, size, class, state, or digest is invalid.
    ResponseDenied,
    /// Cancellation or disconnect did not prove terminated descendants.
    CleanupDenied,
    /// Request identity is duplicated, absent, or already terminal.
    LifecycleDenied,
}

impl McpGatewayError {
    /// Returns one stable content-free diagnostic code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::IdentityDenied => "mcp.gateway.identity_denied",
            Self::RequestDenied => "mcp.gateway.request_denied",
            Self::ResponseDenied => "mcp.gateway.response_denied",
            Self::CleanupDenied => "mcp.gateway.cleanup_denied",
            Self::LifecycleDenied => "mcp.gateway.lifecycle_denied",
        }
    }
}

impl std::fmt::Display for McpGatewayError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.code())
    }
}

impl std::error::Error for McpGatewayError {}

/// Stateful mediator for one exact admitted server connection.
///
/// This type has no socket, process, filesystem, shell, secret, network, model, or approval API.
/// A platform-owned transport may carry only requests returned by [`Self::admit_request`] and must
/// return responses through [`Self::accept_response`].
pub struct McpGatewaySession {
    manifest: McpManifest,
    connection: McpConnection,
    observation: McpProcessObservation,
    active_request_ids: BTreeSet<String>,
    terminal_request_ids: BTreeSet<String>,
    receipts: Vec<McpReceipt>,
    next_receipt: u64,
    disconnected: bool,
}

impl McpGatewaySession {
    /// Opens one gateway session after current connection revalidation.
    pub fn open(
        manifest: McpManifest,
        connection: McpConnection,
        observation: McpProcessObservation,
        now_epoch_ms: u64,
    ) -> Result<Self, McpGatewayError> {
        verify_mcp_connection(&manifest, &connection, &observation, now_epoch_ms)
            .map_err(map_registry_error)?;
        let mut session = Self {
            manifest,
            connection,
            observation,
            active_request_ids: BTreeSet::new(),
            terminal_request_ids: BTreeSet::new(),
            receipts: Vec::new(),
            next_receipt: 1,
            disconnected: false,
        };
        session.push_receipt(McpReceiptKind::ManifestVerified, None, None)?;
        session.push_receipt(McpReceiptKind::Connected, None, None)?;
        Ok(session)
    }

    /// Returns immutable content-minimized lifecycle receipts.
    #[must_use]
    pub fn receipts(&self) -> &[McpReceipt] {
        &self.receipts
    }

    /// Returns deterministic manifest-backed discovery and records its attributable receipt.
    pub fn discover(
        &mut self,
        now_epoch_ms: u64,
    ) -> Result<agentmage_kernel_contracts::McpDiscovery, McpGatewayError> {
        self.verify_live(now_epoch_ms)?;
        let discovery =
            mcp_discovery(&self.manifest, &self.connection).map_err(map_registry_error)?;
        self.push_receipt(McpReceiptKind::Discovered, None, None)?;
        Ok(discovery)
    }

    /// Revalidates and admits one sealed request through the common tool registry.
    pub fn admit_request(
        &mut self,
        common_registry: &ToolRegistry,
        request: McpRequest,
        now_epoch_ms: u64,
    ) -> Result<McpRequest, McpGatewayError> {
        self.verify_live(now_epoch_ms)?;
        verify_mcp_request(&self.manifest, &self.connection, &request, common_registry)?;
        if self.active_request_ids.contains(&request.request_id)
            || self.terminal_request_ids.contains(&request.request_id)
            || self.active_request_ids.len() >= self.manifest.limits.max_concurrency as usize
        {
            return Err(McpGatewayError::LifecycleDenied);
        }
        self.active_request_ids.insert(request.request_id.clone());
        self.push_receipt(McpReceiptKind::Request, Some(&request.request_id), None)?;
        Ok(request)
    }

    /// Accepts one bounded untrusted response and closes its request exactly once.
    pub fn accept_response(
        &mut self,
        request: &McpRequest,
        response: McpResponse,
        now_epoch_ms: u64,
    ) -> Result<McpResponse, McpGatewayError> {
        self.verify_live(now_epoch_ms)?;
        if !self.active_request_ids.contains(&request.request_id) {
            return Err(McpGatewayError::LifecycleDenied);
        }
        verify_mcp_response(&self.manifest, request, &response)?;
        self.close_request(&request.request_id)?;
        let kind = if response.state == McpTerminalState::Succeeded {
            McpReceiptKind::Response
        } else {
            McpReceiptKind::Failed
        };
        self.push_receipt(kind, Some(&request.request_id), Some(response.state))?;
        Ok(response)
    }

    /// Records observed cancellation only after owned work and descendants terminated.
    pub fn cancel(
        &mut self,
        cancellation: &McpCancellation,
        descendants_terminated: bool,
        now_epoch_ms: u64,
    ) -> Result<(), McpGatewayError> {
        self.verify_live(now_epoch_ms)?;
        if cancellation.connection_id != self.connection.connection_id
            || !self.active_request_ids.contains(&cancellation.request_id)
            || !descendants_terminated
        {
            return Err(McpGatewayError::CleanupDenied);
        }
        self.close_request(&cancellation.request_id)?;
        self.push_receipt(
            McpReceiptKind::Cancelled,
            Some(&cancellation.request_id),
            Some(McpTerminalState::Cancelled),
        )
    }

    /// Invalidates the entire connection after verified descendant cleanup.
    pub fn disconnect(&mut self, disconnect: &McpDisconnect) -> Result<(), McpGatewayError> {
        if self.disconnected
            || disconnect.connection_id != self.connection.connection_id
            || !disconnect.descendants_terminated
            || !valid_code(&disconnect.reason_code)
        {
            return Err(McpGatewayError::CleanupDenied);
        }
        let active = self.active_request_ids.iter().cloned().collect::<Vec<_>>();
        for request_id in active {
            self.close_request(&request_id)?;
            self.push_receipt(
                McpReceiptKind::Failed,
                Some(&request_id),
                Some(McpTerminalState::Disconnected),
            )?;
        }
        self.disconnected = true;
        self.push_receipt(McpReceiptKind::Disconnected, None, None)
    }

    fn verify_live(&self, now_epoch_ms: u64) -> Result<(), McpGatewayError> {
        if self.disconnected {
            return Err(McpGatewayError::LifecycleDenied);
        }
        verify_mcp_connection(
            &self.manifest,
            &self.connection,
            &self.observation,
            now_epoch_ms,
        )
        .map_err(map_registry_error)
    }

    fn close_request(&mut self, request_id: &str) -> Result<(), McpGatewayError> {
        if !self.active_request_ids.remove(request_id)
            || !self.terminal_request_ids.insert(request_id.to_owned())
        {
            return Err(McpGatewayError::LifecycleDenied);
        }
        Ok(())
    }

    fn push_receipt(
        &mut self,
        kind: McpReceiptKind,
        request_id: Option<&str>,
        terminal_state: Option<McpTerminalState>,
    ) -> Result<(), McpGatewayError> {
        let mut receipt = McpReceipt {
            receipt_id: ReceiptId::from_raw(format!(
                "mcp-receipt-{}-{:08}",
                self.connection.connection_id, self.next_receipt
            )),
            kind,
            server_id: self.manifest.server_id.clone(),
            connection_id: Some(self.connection.connection_id.clone()),
            request_id: request_id.map(str::to_owned),
            manifest_sha256: self.manifest.manifest_sha256.clone(),
            terminal_state,
            side_effect: StateChange::NotChanged,
            receipt_sha256: ZERO_SHA256.to_owned(),
        };
        receipt.receipt_sha256 = canonical_sha256(&receipt)?;
        self.receipts.push(receipt);
        self.next_receipt = self
            .next_receipt
            .checked_add(1)
            .ok_or(McpGatewayError::LifecycleDenied)?;
        Ok(())
    }
}

/// Seals one statically valid request against an exact admitted manifest and connection.
pub fn seal_mcp_request(
    manifest: &McpManifest,
    connection: &McpConnection,
    common_registry: &ToolRegistry,
    mut request: McpRequest,
) -> Result<McpRequest, McpGatewayError> {
    request.request_sha256 = ZERO_SHA256.to_owned();
    validate_request(manifest, connection, &request, common_registry)?;
    request.request_sha256 = canonical_sha256(&request)?;
    Ok(request)
}

/// Verifies one retained request from canonical bytes.
pub fn verify_mcp_request(
    manifest: &McpManifest,
    connection: &McpConnection,
    request: &McpRequest,
    common_registry: &ToolRegistry,
) -> Result<(), McpGatewayError> {
    validate_request(manifest, connection, request, common_registry)?;
    let mut preimage = request.clone();
    preimage.request_sha256 = ZERO_SHA256.to_owned();
    if canonical_sha256(&preimage)? != request.request_sha256 {
        return Err(McpGatewayError::RequestDenied);
    }
    Ok(())
}

/// Verifies a server response without trusting its success claim or payload metadata.
pub fn verify_mcp_response(
    manifest: &McpManifest,
    request: &McpRequest,
    response: &McpResponse,
) -> Result<(), McpGatewayError> {
    verify_mcp_manifest(manifest).map_err(map_registry_error)?;
    let expected_class = response_class(manifest, request)?;
    let expected_schema = response_schema(manifest, request)?;
    let success = response.state == McpTerminalState::Succeeded;
    if response.request_id != request.request_id
        || response.response_class != expected_class
        || success != response.payload.is_some()
        || success != response.error_code.is_none()
        || (!success && response.item_count != 0)
        || response.item_count > request.max_response_items
        || response
            .error_code
            .as_deref()
            .is_some_and(|code| !valid_code(code))
        || response.payload.as_ref().is_some_and(|payload| {
            payload.schema != *expected_schema
                || payload.bytes.len() as u64 > request.max_response_bytes
                || payload.bytes.len() as u64 > manifest.limits.max_response_bytes
                || !valid_payload(payload)
        })
        || response.sensitivity == ContextSensitivity::Public
    {
        return Err(McpGatewayError::ResponseDenied);
    }
    let mut preimage = response.clone();
    preimage.response_sha256 = ZERO_SHA256.to_owned();
    if canonical_sha256(&preimage)? != response.response_sha256 {
        return Err(McpGatewayError::ResponseDenied);
    }
    Ok(())
}

/// Seals a candidate response for deterministic adapters and test fixtures.
pub fn seal_mcp_response(
    manifest: &McpManifest,
    request: &McpRequest,
    mut response: McpResponse,
) -> Result<McpResponse, McpGatewayError> {
    response.response_sha256 = ZERO_SHA256.to_owned();
    let expected_class = response_class(manifest, request)?;
    let expected_schema = response_schema(manifest, request)?;
    if response.response_class != expected_class
        || response
            .payload
            .as_ref()
            .is_some_and(|payload| payload.schema != *expected_schema)
    {
        return Err(McpGatewayError::ResponseDenied);
    }
    response.response_sha256 = canonical_sha256(&response)?;
    verify_mcp_response(manifest, request, &response)?;
    Ok(response)
}

fn validate_request(
    manifest: &McpManifest,
    connection: &McpConnection,
    request: &McpRequest,
    common_registry: &ToolRegistry,
) -> Result<(), McpGatewayError> {
    verify_mcp_manifest(manifest).map_err(map_registry_error)?;
    if connection.server_id != manifest.server_id
        || connection.manifest_sha256 != manifest.manifest_sha256
        || request.connection_id != connection.connection_id
        || !valid_identifier(&request.request_id)
        || !valid_identifier(&request.capability_id)
        || request.max_response_bytes == 0
        || request.max_response_bytes > manifest.limits.max_response_bytes
        || request.max_response_items == 0
        || request.max_response_items > manifest.limits.max_response_items
        || request.timeout_ms == 0
        || request.timeout_ms > manifest.limits.timeout_ms
    {
        return Err(McpGatewayError::RequestDenied);
    }
    match request.kind {
        McpRequestKind::ToolCall => {
            let call = request
                .tool_call
                .as_ref()
                .ok_or(McpGatewayError::RequestDenied)?;
            if request.arguments.is_some()
                || request.capability_id != call.tool_id.as_str()
                || !manifest.tools.iter().any(|tool| {
                    tool.definition.tool_id == call.tool_id
                        && tool.definition.tool_version == call.tool_version
                })
                || common_registry.validate_arguments(call).is_err()
                || call.arguments.bytes.len() as u64 > manifest.limits.max_request_bytes
            {
                return Err(McpGatewayError::RequestDenied);
            }
        }
        McpRequestKind::ResourceRead => {
            if request.tool_call.is_some()
                || request.arguments.is_some()
                || !manifest
                    .resources
                    .iter()
                    .any(|resource| resource.resource_id == request.capability_id)
            {
                return Err(McpGatewayError::RequestDenied);
            }
        }
        McpRequestKind::PromptGet => {
            let arguments = request
                .arguments
                .as_ref()
                .ok_or(McpGatewayError::RequestDenied)?;
            let Some(prompt) = manifest
                .prompts
                .iter()
                .find(|prompt| prompt.prompt_id == request.capability_id)
            else {
                return Err(McpGatewayError::RequestDenied);
            };
            if request.tool_call.is_some()
                || arguments.schema != prompt.request_schema
                || arguments.bytes.len() as u64 > manifest.limits.max_request_bytes
                || !valid_payload(arguments)
            {
                return Err(McpGatewayError::RequestDenied);
            }
        }
    }
    Ok(())
}

fn response_class(
    manifest: &McpManifest,
    request: &McpRequest,
) -> Result<McpResponseClass, McpGatewayError> {
    match request.kind {
        McpRequestKind::ToolCall => manifest
            .tools
            .iter()
            .find(|tool| tool.definition.tool_id.as_str() == request.capability_id)
            .map(|tool| tool.response_class),
        McpRequestKind::ResourceRead => manifest
            .resources
            .iter()
            .find(|resource| resource.resource_id == request.capability_id)
            .map(|resource| resource.response_class),
        McpRequestKind::PromptGet => manifest
            .prompts
            .iter()
            .find(|prompt| prompt.prompt_id == request.capability_id)
            .map(|prompt| prompt.response_class),
    }
    .ok_or(McpGatewayError::RequestDenied)
}

fn response_schema<'a>(
    manifest: &'a McpManifest,
    request: &McpRequest,
) -> Result<&'a agentmage_kernel_contracts::SchemaReference, McpGatewayError> {
    match request.kind {
        McpRequestKind::ToolCall => manifest
            .tools
            .iter()
            .find(|tool| tool.definition.tool_id.as_str() == request.capability_id)
            .map(|tool| &tool.definition.output_schema),
        McpRequestKind::ResourceRead => manifest
            .resources
            .iter()
            .find(|resource| resource.resource_id == request.capability_id)
            .map(|resource| &resource.response_schema),
        McpRequestKind::PromptGet => manifest
            .prompts
            .iter()
            .find(|prompt| prompt.prompt_id == request.capability_id)
            .map(|prompt| &prompt.response_schema),
    }
    .ok_or(McpGatewayError::RequestDenied)
}

fn valid_payload(payload: &agentmage_kernel_contracts::ContractPayload) -> bool {
    !payload.bytes.is_empty()
        && payload.media_type == "application/json"
        && payload.sha256 == sha256_hex(&payload.bytes)
}

fn valid_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_ID_BYTES
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b':' | b'-'))
}

fn valid_code(value: &str) -> bool {
    valid_identifier(value)
}

fn map_registry_error(_error: McpRegistryError) -> McpGatewayError {
    McpGatewayError::IdentityDenied
}

fn canonical_sha256(value: &impl Serialize) -> Result<String, McpGatewayError> {
    let bytes = serde_json::to_vec(value).map_err(|_| McpGatewayError::ResponseDenied)?;
    Ok(sha256_hex(&bytes))
}

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[cfg(test)]
mod tests {
    use agentmage_kernel_contracts::{
        ActionId, CancellationId, ContractPayload, CorrelationId, McpRequestKind, ToolCall,
        ToolCallId,
    };

    use super::*;
    use crate::mcp_registry::{
        McpManifestRegistry, admit_mcp_connection, fixture_mcp_manifest, fixture_mcp_observation,
    };

    fn fixture() -> (
        McpManifest,
        McpConnection,
        McpProcessObservation,
        ToolRegistry,
    ) {
        let manifest = fixture_mcp_manifest();
        let observation = fixture_mcp_observation(&manifest);
        let connection = admit_mcp_connection(
            &manifest,
            &observation,
            "connection-gateway".to_owned(),
            1_000,
            10_000,
        )
        .expect("connection");
        let native = ToolRegistry::new();
        let mut manifests = McpManifestRegistry::new(&native);
        manifests.register(manifest.clone()).expect("manifest");
        let mut common = ToolRegistry::new();
        manifests
            .register_tools_into(&manifest.server_id, &mut common)
            .expect("common registry");
        (manifest, connection, observation, common)
    }

    fn payload(
        schema: agentmage_kernel_contracts::SchemaReference,
        bytes: &[u8],
    ) -> ContractPayload {
        ContractPayload {
            schema,
            media_type: "application/json".to_owned(),
            bytes: bytes.to_vec(),
            sha256: sha256_hex(bytes),
        }
    }

    fn request(
        manifest: &McpManifest,
        connection: &McpConnection,
        common: &ToolRegistry,
        request_id: &str,
    ) -> McpRequest {
        let definition = &manifest.tools[0].definition;
        seal_mcp_request(
            manifest,
            connection,
            common,
            McpRequest {
                request_id: request_id.to_owned(),
                connection_id: connection.connection_id.clone(),
                kind: McpRequestKind::ToolCall,
                capability_id: definition.tool_id.as_str().to_owned(),
                tool_call: Some(ToolCall {
                    schema_version: agentmage_kernel_contracts::CONTRACT_SCHEMA_VERSION,
                    tool_call_id: ToolCallId::from_raw(format!("call-{request_id}")),
                    correlation_id: CorrelationId::from_raw("correlation-mcp-gateway"),
                    action_id: ActionId::from_raw("action-mcp-gateway"),
                    tool_id: definition.tool_id.clone(),
                    tool_version: definition.tool_version.clone(),
                    arguments: payload(
                        definition.input_schema.clone(),
                        br#"{"path":"src/lib.rs"}"#,
                    ),
                }),
                arguments: None,
                max_response_bytes: 4_096,
                max_response_items: 4,
                timeout_ms: 1_000,
                request_sha256: ZERO_SHA256.to_owned(),
            },
        )
        .expect("sealed request")
    }

    fn response(manifest: &McpManifest, request: &McpRequest) -> McpResponse {
        seal_mcp_response(
            manifest,
            request,
            McpResponse {
                request_id: request.request_id.clone(),
                state: McpTerminalState::Succeeded,
                response_class: manifest.tools[0].response_class,
                payload: Some(payload(
                    manifest.tools[0].definition.output_schema.clone(),
                    br#"{"content":"fixture"}"#,
                )),
                item_count: 1,
                sensitivity: ContextSensitivity::Internal,
                error_code: None,
                response_sha256: ZERO_SHA256.to_owned(),
            },
        )
        .expect("sealed response")
    }

    #[test]
    fn sprint_81_common_registry_request_response_and_receipts_are_one_use() {
        let (manifest, connection, observation, common) = fixture();
        let mut gateway =
            McpGatewaySession::open(manifest.clone(), connection.clone(), observation, 1_100)
                .expect("gateway");
        gateway.discover(1_150).expect("discovery");
        let request = request(&manifest, &connection, &common, "request-0001");
        gateway
            .admit_request(&common, request.clone(), 1_200)
            .expect("admitted request");
        let response = response(&manifest, &request);
        let accepted = gateway
            .accept_response(&request, response.clone(), 1_300)
            .expect("accepted response");
        assert_eq!(accepted, response);
        assert_eq!(gateway.receipts().len(), 5);
        assert!(
            gateway
                .receipts()
                .iter()
                .all(|receipt| receipt.side_effect == StateChange::NotChanged)
        );
        assert_eq!(
            gateway.admit_request(&common, request, 1_400),
            Err(McpGatewayError::LifecycleDenied)
        );
    }

    #[test]
    fn sprint_81_malformed_oversized_reclassified_and_spoofed_output_fails_closed() {
        let (manifest, connection, observation, common) = fixture();
        let attacks = ["schema", "oversized", "public", "spoofed"];
        for attack in attacks {
            let mut gateway = McpGatewaySession::open(
                manifest.clone(),
                connection.clone(),
                observation.clone(),
                1_100,
            )
            .expect("gateway");
            let request = request(
                &manifest,
                &connection,
                &common,
                &format!("request-{attack}"),
            );
            gateway
                .admit_request(&common, request.clone(), 1_200)
                .expect("request");
            let mut candidate = response(&manifest, &request);
            match attack {
                "schema" => {
                    candidate
                        .payload
                        .as_mut()
                        .expect("payload")
                        .schema
                        .schema_sha256 = "f".repeat(64);
                }
                "oversized" => {
                    let payload = candidate.payload.as_mut().expect("payload");
                    payload.bytes = vec![b'x'; 4_097];
                    payload.sha256 = sha256_hex(&payload.bytes);
                }
                "public" => candidate.sensitivity = ContextSensitivity::Public,
                "spoofed" => candidate.request_id = "another-request".to_owned(),
                _ => unreachable!(),
            }
            candidate.response_sha256 = ZERO_SHA256.to_owned();
            candidate.response_sha256 = canonical_sha256(&candidate).expect("attack digest");
            assert_eq!(
                gateway.accept_response(&request, candidate, 1_300),
                Err(McpGatewayError::ResponseDenied),
                "{attack}"
            );
            assert_eq!(gateway.receipts().len(), 3, "{attack}");
        }
    }

    #[test]
    fn sprint_81_cancellation_and_disconnect_require_verified_cleanup() {
        let (manifest, connection, observation, common) = fixture();
        let mut gateway =
            McpGatewaySession::open(manifest.clone(), connection.clone(), observation, 1_100)
                .expect("gateway");
        let request = request(&manifest, &connection, &common, "request-cancel");
        gateway
            .admit_request(&common, request.clone(), 1_200)
            .expect("request");
        let cancellation = McpCancellation {
            cancellation_id: CancellationId::from_raw("cancellation-mcp"),
            request_id: request.request_id.clone(),
            connection_id: connection.connection_id.clone(),
        };
        assert_eq!(
            gateway.cancel(&cancellation, false, 1_300),
            Err(McpGatewayError::CleanupDenied)
        );
        gateway
            .cancel(&cancellation, true, 1_300)
            .expect("verified cancellation");
        assert_eq!(
            gateway.accept_response(&request, response(&manifest, &request), 1_400),
            Err(McpGatewayError::LifecycleDenied)
        );
        assert_eq!(
            gateway.disconnect(&McpDisconnect {
                connection_id: connection.connection_id.clone(),
                reason_code: "mcp.test.complete".to_owned(),
                descendants_terminated: false,
            }),
            Err(McpGatewayError::CleanupDenied)
        );
        gateway
            .disconnect(&McpDisconnect {
                connection_id: connection.connection_id,
                reason_code: "mcp.test.complete".to_owned(),
                descendants_terminated: true,
            })
            .expect("disconnect");
        assert_eq!(gateway.receipts().len(), 5);
    }
}
