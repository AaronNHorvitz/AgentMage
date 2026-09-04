//! Optional MCP presentation of the existing native source-artifact tool family.

use agentmage_capability_read_only::{
    ArtifactToolKind, artifact_tool_definition, artifact_tool_kind,
};
use agentmage_kernel_contracts::{
    GrantOperation, McpLimits, McpManifest, McpPromptManifest, McpRequest, McpRequestKind,
    McpResourceManifest, McpResponseClass, McpToolManifest, McpTransport, RequiredGrantTemplate,
    StateChange, ToolCall, ToolId,
};
use agentmage_kernel_engine::{
    mcp_gateway::{McpGatewayError, McpGatewaySession},
    mcp_registry::{McpRegistryError, seal_mcp_manifest},
    tooling::{PreGrantDispatchDisposition, ProposalOrigin, ToolDispatcher, ToolRegistry},
};

const MCP_ARTIFACT_PREFIX: &str = "mcp.agentmage.";

/// Exact inert manifest input supplied by a platform-owned MCP launcher.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct McpArtifactManifestInput {
    /// Stable reviewed server identity.
    pub server_id: String,
    /// Exact package version.
    pub server_version: String,
    /// Digest of the installed server package.
    pub package_sha256: String,
    /// Digest of the executable and launch configuration.
    pub process_sha256: String,
    /// Exact transport and endpoint identity.
    pub transport: McpTransport,
    /// Sorted exact admitted workspace-root digests.
    pub workspace_root_sha256s: Vec<String>,
}

/// Verified projection entering the existing native dispatcher boundary.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct McpArtifactAdmission {
    /// Gateway-admitted request retained for the transport adapter.
    pub request: McpRequest,
    /// Exact existing native artifact operation.
    pub artifact_kind: ArtifactToolKind,
    /// Native tool call after identity-only transport translation.
    pub native_call: ToolCall,
    /// Unchanged single-use native grant requirement.
    pub required_grant: RequiredGrantTemplate,
}

/// Stable refusal before any MCP or artifact process can execute.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum McpArtifactAdapterError {
    /// Manifest fields or read-only invariants failed.
    ManifestDenied,
    /// Request is not one exact artifact tool call.
    RequestDenied,
    /// The native common registry did not admit the translated call.
    NativeDispatchDenied,
    /// The stateful MCP gateway rejected identity, lifecycle, or bounds.
    GatewayDenied,
}

impl McpArtifactAdapterError {
    /// Returns one content-free stable diagnostic.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::ManifestDenied => "mcp.artifact.manifest_denied",
            Self::RequestDenied => "mcp.artifact.request_denied",
            Self::NativeDispatchDenied => "mcp.artifact.native_dispatch_denied",
            Self::GatewayDenied => "mcp.artifact.gateway_denied",
        }
    }
}

/// Builds the optional seven-tool MCP view from the native definitions.
///
/// Names are transport-specific to prevent shadowing. Schemas, versions, effects, grants, and
/// limits remain byte-for-byte equal to the native definitions. This function launches nothing.
pub fn build_mcp_artifact_manifest(
    input: McpArtifactManifestInput,
) -> Result<McpManifest, McpArtifactAdapterError> {
    let mut tools = ArtifactToolKind::ALL
        .into_iter()
        .map(|kind| {
            let mut definition = artifact_tool_definition(kind);
            definition.tool_id = ToolId::from_raw(mcp_artifact_tool_id(kind));
            McpToolManifest {
                definition,
                response_class: McpResponseClass::UntrustedStructuredData,
                side_effect: StateChange::NotChanged,
            }
        })
        .collect::<Vec<_>>();
    tools.sort_by(|left, right| {
        left.definition.tool_id.cmp(&right.definition.tool_id).then(
            left.definition
                .tool_version
                .cmp(&right.definition.tool_version),
        )
    });
    let network_destinations = match input.transport.kind {
        agentmage_kernel_contracts::McpTransportKind::LoopbackTcp
        | agentmage_kernel_contracts::McpTransportKind::RemoteHttps => {
            vec![input.transport.destination.clone()]
        }
        _ => Vec::new(),
    };
    seal_mcp_manifest(McpManifest {
        schema_version: agentmage_kernel_contracts::CONTRACT_SCHEMA_VERSION,
        server_id: input.server_id,
        server_version: input.server_version,
        package_sha256: input.package_sha256,
        process_sha256: input.process_sha256,
        transport: input.transport,
        tools,
        resources: Vec::<McpResourceManifest>::new(),
        prompts: Vec::<McpPromptManifest>::new(),
        workspace_root_sha256s: input.workspace_root_sha256s,
        network_destinations,
        credential_ids: Vec::new(),
        limits: McpLimits {
            max_tools: ArtifactToolKind::ALL.len() as u32,
            max_resources: 0,
            max_prompts: 0,
            max_request_bytes: 1_048_576,
            max_response_bytes: 8 * 1_048_576,
            max_response_items: 16_384,
            timeout_ms: 300_000,
            max_concurrency: 8,
        },
        cancellation_supported: true,
        requested_operations: vec![GrantOperation::WorkspaceRead],
        manifest_sha256: "0".repeat(64),
    })
    .map_err(|_error: McpRegistryError| McpArtifactAdapterError::ManifestDenied)
}

/// Routes one MCP artifact call through gateway admission and the existing native dispatcher.
///
/// The returned admission is still non-executable: the common dispatcher requires its exact
/// single-use grant, and only the existing artifact runtime can later consume it and execute.
pub fn admit_mcp_artifact_request(
    gateway: &mut McpGatewaySession,
    mcp_registry: &ToolRegistry,
    native_registry: &ToolRegistry,
    request: McpRequest,
    now_epoch_ms: u64,
) -> Result<McpArtifactAdmission, McpArtifactAdapterError> {
    if request.kind != McpRequestKind::ToolCall || request.arguments.is_some() {
        return Err(McpArtifactAdapterError::RequestDenied);
    }
    let mcp_call = request
        .tool_call
        .as_ref()
        .ok_or(McpArtifactAdapterError::RequestDenied)?;
    let artifact_kind = mcp_artifact_kind(&mcp_call.tool_id, &mcp_call.tool_version)
        .ok_or(McpArtifactAdapterError::RequestDenied)?;
    let native_definition = artifact_tool_definition(artifact_kind);
    let mut native_call = mcp_call.clone();
    native_call.tool_id = native_definition.tool_id.clone();
    native_call.tool_version = native_definition.tool_version.clone();
    native_call.arguments.schema = native_definition.input_schema.clone();
    let definition = native_registry
        .validate_arguments(&native_call)
        .map_err(|_| McpArtifactAdapterError::NativeDispatchDenied)?;
    let dispatch =
        ToolDispatcher::new(native_registry).dispatch(ProposalOrigin::Tool, &native_call);
    if dispatch.disposition != PreGrantDispatchDisposition::GrantRequired
        || definition.required_grant.operation.operation() != GrantOperation::WorkspaceRead
        || !definition.required_grant.single_use
    {
        return Err(McpArtifactAdapterError::NativeDispatchDenied);
    }
    let request = gateway
        .admit_request(mcp_registry, request, now_epoch_ms)
        .map_err(|_error: McpGatewayError| McpArtifactAdapterError::GatewayDenied)?;
    Ok(McpArtifactAdmission {
        request,
        artifact_kind,
        native_call,
        required_grant: definition.required_grant.clone(),
    })
}

/// Returns the exact MCP presentation identity for one native artifact tool.
#[must_use]
pub fn mcp_artifact_tool_id(kind: ArtifactToolKind) -> String {
    format!("{MCP_ARTIFACT_PREFIX}{}", kind.id())
}

fn mcp_artifact_kind(tool_id: &ToolId, version: &str) -> Option<ArtifactToolKind> {
    let native_id = tool_id.as_str().strip_prefix(MCP_ARTIFACT_PREFIX)?;
    artifact_tool_kind(&ToolId::from_raw(native_id), version)
}

#[cfg(test)]
mod tests {
    use agentmage_capability_read_only::{ARTIFACT_TOOL_VERSION, ArtifactLimits, ArtifactRequest};
    use agentmage_kernel_contracts::{
        ActionId, ContractPayload, CorrelationId, McpRequest, McpTransportKind, ToolCallId,
    };
    use agentmage_kernel_engine::{
        mcp_gateway::{McpGatewaySession, seal_mcp_request},
        mcp_registry::{McpManifestRegistry, McpProcessObservation, admit_mcp_connection},
    };
    use sha2::{Digest, Sha256};

    use super::*;
    use crate::runtime_tools::read_only_runtime_registry;

    fn hash(byte: char) -> String {
        byte.to_string().repeat(64)
    }

    fn manifest() -> McpManifest {
        build_mcp_artifact_manifest(McpArtifactManifestInput {
            server_id: "agentmage-artifact-provider".to_owned(),
            server_version: "1.0.0".to_owned(),
            package_sha256: hash('a'),
            process_sha256: hash('b'),
            transport: McpTransport {
                kind: McpTransportKind::InProcess,
                endpoint_identity_sha256: hash('c'),
                destination: String::new(),
            },
            workspace_root_sha256s: vec![hash('d')],
        })
        .expect("artifact manifest")
    }

    fn payload(definition: &agentmage_kernel_contracts::ToolDefinition) -> ContractPayload {
        let bytes = serde_json::to_vec(&ArtifactRequest {
            schema_version: 1,
            call_id: "artifact-list-call".to_owned(),
            source_id: None,
            section_id: None,
            range: None,
            query: None,
            freshness_sha256: None,
            output_identity: "artifact-list-output".to_owned(),
            limits: ArtifactLimits::default(),
            call_depth: 0,
        })
        .expect("request bytes");
        ContractPayload {
            schema: definition.input_schema.clone(),
            media_type: "application/json".to_owned(),
            sha256: Sha256::digest(&bytes)
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect(),
            bytes,
        }
    }

    #[test]
    fn sprint_81_all_artifact_definitions_preserve_native_contracts() {
        let manifest = manifest();
        assert_eq!(manifest.tools.len(), ArtifactToolKind::ALL.len());
        for kind in ArtifactToolKind::ALL {
            let native = artifact_tool_definition(kind);
            let exposed = manifest
                .tools
                .iter()
                .find(|tool| tool.definition.tool_id.as_str() == mcp_artifact_tool_id(kind))
                .expect("mapped tool");
            assert_eq!(exposed.definition.tool_version, native.tool_version);
            assert_eq!(exposed.definition.input_schema, native.input_schema);
            assert_eq!(exposed.definition.output_schema, native.output_schema);
            assert_eq!(exposed.definition.required_grant, native.required_grant);
            assert_eq!(exposed.definition.declared_effects, native.declared_effects);
            assert_eq!(exposed.definition.timeout_ms, native.timeout_ms);
        }
    }

    #[test]
    fn sprint_81_request_reaches_gateway_and_native_pregrant_dispatcher() {
        let native = read_only_runtime_registry().expect("native registry");
        let manifest = manifest();
        let observation = McpProcessObservation {
            package_sha256: manifest.package_sha256.clone(),
            process_sha256: manifest.process_sha256.clone(),
            endpoint_identity_sha256: manifest.transport.endpoint_identity_sha256.clone(),
            transport: manifest.transport.kind,
            descendants_contained: true,
        };
        let connection = admit_mcp_connection(
            &manifest,
            &observation,
            "artifact-connection".to_owned(),
            1_000,
            9_000,
        )
        .expect("connection");
        let mut manifests = McpManifestRegistry::new(&native);
        manifests.register(manifest.clone()).expect("registration");
        let mut mcp_registry = ToolRegistry::new();
        manifests
            .register_tools_into(&manifest.server_id, &mut mcp_registry)
            .expect("MCP common registry");
        let definition = &manifest
            .tools
            .iter()
            .find(|tool| {
                tool.definition.tool_id.as_str() == mcp_artifact_tool_id(ArtifactToolKind::List)
            })
            .expect("list mapping")
            .definition;
        let call = ToolCall {
            schema_version: agentmage_kernel_contracts::CONTRACT_SCHEMA_VERSION,
            tool_call_id: ToolCallId::from_raw("mcp-artifact-tool-call"),
            correlation_id: CorrelationId::from_raw("mcp-artifact-correlation"),
            action_id: ActionId::from_raw("mcp-artifact-action"),
            tool_id: definition.tool_id.clone(),
            tool_version: definition.tool_version.clone(),
            arguments: payload(definition),
        };
        let request = seal_mcp_request(
            &manifest,
            &connection,
            &mcp_registry,
            McpRequest {
                request_id: "mcp-artifact-request".to_owned(),
                connection_id: connection.connection_id.clone(),
                kind: McpRequestKind::ToolCall,
                capability_id: call.tool_id.as_str().to_owned(),
                tool_call: Some(call),
                arguments: None,
                max_response_bytes: 4_096,
                max_response_items: 8,
                timeout_ms: 1_000,
                request_sha256: hash('0'),
            },
        )
        .expect("request");
        let mut invalid = request.clone();
        let invalid_payload = &mut invalid.tool_call.as_mut().expect("tool call").arguments;
        invalid_payload.bytes = br#"{}"#.to_vec();
        invalid_payload.sha256 = Sha256::digest(&invalid_payload.bytes)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect();
        invalid.request_sha256 = hash('0');
        let invalid = seal_mcp_request(&manifest, &connection, &mcp_registry, invalid)
            .expect("transport-valid malformed native request");
        let mut gateway = McpGatewaySession::open(manifest.clone(), connection, observation, 1_100)
            .expect("gateway");
        assert_eq!(
            admit_mcp_artifact_request(&mut gateway, &mcp_registry, &native, invalid, 1_150),
            Err(McpArtifactAdapterError::NativeDispatchDenied)
        );
        assert_eq!(gateway.receipts().len(), 2);
        let admitted = admit_mcp_artifact_request(
            &mut gateway,
            &mcp_registry,
            &native,
            request.clone(),
            1_200,
        )
        .expect("admission");
        assert_eq!(admitted.artifact_kind, ArtifactToolKind::List);
        assert_eq!(
            admitted.native_call.tool_id.as_str(),
            ArtifactToolKind::List.id()
        );
        assert!(admitted.required_grant.single_use);
        assert_eq!(gateway.receipts().len(), 3);
        assert_eq!(
            admit_mcp_artifact_request(&mut gateway, &mcp_registry, &native, request, 1_300),
            Err(McpArtifactAdapterError::GatewayDenied)
        );
    }

    #[test]
    fn sprint_81_disablement_leaves_native_artifacts_and_no_mcp_registration() {
        let native = read_only_runtime_registry().expect("native registry");
        let manifest = manifest();
        let mut manifests = McpManifestRegistry::new(&native);
        manifests.register(manifest.clone()).expect("registration");
        manifests.disable(&manifest.server_id).expect("disable");
        let mut rebuilt = ToolRegistry::new();
        assert!(
            manifests
                .register_tools_into(&manifest.server_id, &mut rebuilt)
                .is_err()
        );
        assert!(ArtifactToolKind::ALL.into_iter().all(|kind| {
            native
                .get_tool(&ToolId::from_raw(kind.id()), ARTIFACT_TOOL_VERSION)
                .is_some()
        }));
        assert!(rebuilt.list_tools().is_empty());
    }
}
