//! Read-only MCP manifest admission, identity binding, and optional tool adaptation.

use std::collections::{BTreeMap, BTreeSet};

use agentmage_kernel_contracts::{
    CONTRACT_SCHEMA_VERSION, GrantOperation, McpConnection, McpDiscovery, McpManifest,
    McpResponseClass, McpToolManifest, McpTransportKind, StateChange, ToolDefinition, ToolId,
};
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::tooling::{Tool, ToolRegistry};

const ZERO_SHA256: &str = "0000000000000000000000000000000000000000000000000000000000000000";
const MAX_ID_BYTES: usize = 128;
const MAX_VERSION_BYTES: usize = 64;
const MAX_CAPABILITIES: usize = 256;
const MAX_REQUEST_BYTES: u64 = 1024 * 1024;
const MAX_RESPONSE_BYTES: u64 = 8 * 1024 * 1024;
const MAX_RESPONSE_ITEMS: u32 = 16_384;
const MAX_TIMEOUT_MS: u64 = 300_000;
const MAX_CONCURRENCY: u16 = 32;
const MAX_CONNECTION_LIFETIME_MS: u64 = 86_400_000;

/// Exact package, process, and endpoint observation made by a trusted platform adapter.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct McpProcessObservation {
    /// Observed package digest.
    pub package_sha256: String,
    /// Observed executable and launch digest.
    pub process_sha256: String,
    /// Observed endpoint configuration digest.
    pub endpoint_identity_sha256: String,
    /// Observed transport family.
    pub transport: McpTransportKind,
    /// Whether process descendants remain inside the declared owned boundary.
    pub descendants_contained: bool,
}

/// Stable fail-closed MCP registration and identity error.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum McpRegistryError {
    /// Manifest fields, bounds, schemas, transport, or digest are invalid.
    InvalidManifest,
    /// A capability is write-capable, privileged, or not read-only.
    WriteCapabilityDenied,
    /// The manifest shadows an existing native or MCP tool identity.
    ToolShadowDenied,
    /// Package, process, transport, endpoint, containment, or expiry evidence disagrees.
    IdentityDenied,
    /// The server or connection is absent or stale.
    NotAdmitted,
    /// Common tool registration refused the adapted definition.
    ToolRegistrationDenied,
}

impl McpRegistryError {
    /// Returns a stable content-free diagnostic code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidManifest => "mcp.registry.manifest_invalid",
            Self::WriteCapabilityDenied => "mcp.registry.write_capability_denied",
            Self::ToolShadowDenied => "mcp.registry.tool_shadow_denied",
            Self::IdentityDenied => "mcp.registry.identity_denied",
            Self::NotAdmitted => "mcp.registry.not_admitted",
            Self::ToolRegistrationDenied => "mcp.registry.tool_registration_denied",
        }
    }
}

impl std::fmt::Display for McpRegistryError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.code())
    }
}

impl std::error::Error for McpRegistryError {}

/// Immutable registry of reviewed optional read-only MCP manifests.
pub struct McpManifestRegistry {
    native_tool_ids: BTreeSet<ToolId>,
    manifests: BTreeMap<String, McpManifest>,
}

impl McpManifestRegistry {
    /// Creates an empty MCP registry bound to the current native tool identities.
    #[must_use]
    pub fn new(native_registry: &ToolRegistry) -> Self {
        Self {
            native_tool_ids: native_registry
                .list_tools()
                .into_iter()
                .map(|definition| definition.tool_id.clone())
                .collect(),
            manifests: BTreeMap::new(),
        }
    }

    /// Registers one already sealed read-only manifest without launching a server.
    pub fn register(&mut self, manifest: McpManifest) -> Result<(), McpRegistryError> {
        verify_mcp_manifest(&manifest)?;
        if self.manifests.contains_key(&manifest.server_id) {
            return Err(McpRegistryError::InvalidManifest);
        }
        let existing_mcp_ids = self
            .manifests
            .values()
            .flat_map(|registered| registered.tools.iter())
            .map(|tool| tool.definition.tool_id.clone())
            .collect::<BTreeSet<_>>();
        if manifest.tools.iter().any(|tool| {
            self.native_tool_ids.contains(&tool.definition.tool_id)
                || existing_mcp_ids.contains(&tool.definition.tool_id)
        }) {
            return Err(McpRegistryError::ToolShadowDenied);
        }
        self.manifests.insert(manifest.server_id.clone(), manifest);
        Ok(())
    }

    /// Returns one exact registered manifest.
    #[must_use]
    pub fn get(&self, server_id: &str) -> Option<&McpManifest> {
        self.manifests.get(server_id)
    }

    /// Removes one optional manifest registration without touching native tools.
    pub fn disable(&mut self, server_id: &str) -> Result<McpManifest, McpRegistryError> {
        self.manifests
            .remove(server_id)
            .ok_or(McpRegistryError::NotAdmitted)
    }

    /// Adds one server's read-only definitions to a fresh common registry.
    ///
    /// Callers rebuild the common registry from native tools when MCP is disabled. The MCP
    /// registry never owns, removes, proxies, or replaces native definitions.
    pub fn register_tools_into(
        &self,
        server_id: &str,
        common_registry: &mut ToolRegistry,
    ) -> Result<(), McpRegistryError> {
        let manifest = self.get(server_id).ok_or(McpRegistryError::NotAdmitted)?;
        for tool in &manifest.tools {
            if common_registry
                .list_tools()
                .iter()
                .any(|definition| definition.tool_id == tool.definition.tool_id)
            {
                return Err(McpRegistryError::ToolShadowDenied);
            }
            common_registry
                .register_tool(Box::new(ManifestTool(tool.definition.clone())))
                .map_err(|_| McpRegistryError::ToolRegistrationDenied)?;
        }
        Ok(())
    }
}

/// Seals one statically valid read-only MCP manifest.
pub fn seal_mcp_manifest(mut manifest: McpManifest) -> Result<McpManifest, McpRegistryError> {
    manifest.schema_version = CONTRACT_SCHEMA_VERSION;
    manifest.manifest_sha256 = ZERO_SHA256.to_owned();
    validate_manifest(&manifest)?;
    manifest.manifest_sha256 = canonical_sha256(&manifest)?;
    Ok(manifest)
}

/// Verifies one retained MCP manifest from its canonical bytes.
pub fn verify_mcp_manifest(manifest: &McpManifest) -> Result<(), McpRegistryError> {
    validate_manifest(manifest)?;
    let mut preimage = manifest.clone();
    preimage.manifest_sha256 = ZERO_SHA256.to_owned();
    if canonical_sha256(&preimage)? != manifest.manifest_sha256 {
        return Err(McpRegistryError::InvalidManifest);
    }
    Ok(())
}

/// Binds an expiring connection to exact launch and connection observations.
pub fn admit_mcp_connection(
    manifest: &McpManifest,
    observation: &McpProcessObservation,
    connection_id: String,
    now_epoch_ms: u64,
    expires_at_epoch_ms: u64,
) -> Result<McpConnection, McpRegistryError> {
    verify_mcp_manifest(manifest)?;
    verify_observation(manifest, observation)?;
    if !valid_identifier(&connection_id)
        || now_epoch_ms == 0
        || expires_at_epoch_ms <= now_epoch_ms
        || expires_at_epoch_ms.saturating_sub(now_epoch_ms) > MAX_CONNECTION_LIFETIME_MS
    {
        return Err(McpRegistryError::IdentityDenied);
    }
    let mut connection = McpConnection {
        connection_id,
        server_id: manifest.server_id.clone(),
        manifest_sha256: manifest.manifest_sha256.clone(),
        observed_package_sha256: observation.package_sha256.clone(),
        observed_process_sha256: observation.process_sha256.clone(),
        connected_at_epoch_ms: now_epoch_ms,
        expires_at_epoch_ms,
        connection_sha256: ZERO_SHA256.to_owned(),
    };
    connection.connection_sha256 = canonical_sha256(&connection)?;
    Ok(connection)
}

/// Revalidates a connection against current manifest, identity, and time evidence.
pub fn verify_mcp_connection(
    manifest: &McpManifest,
    connection: &McpConnection,
    observation: &McpProcessObservation,
    now_epoch_ms: u64,
) -> Result<(), McpRegistryError> {
    verify_mcp_manifest(manifest)?;
    verify_observation(manifest, observation)?;
    let mut preimage = connection.clone();
    preimage.connection_sha256 = ZERO_SHA256.to_owned();
    if connection.server_id != manifest.server_id
        || connection.manifest_sha256 != manifest.manifest_sha256
        || connection.observed_package_sha256 != observation.package_sha256
        || connection.observed_process_sha256 != observation.process_sha256
        || now_epoch_ms < connection.connected_at_epoch_ms
        || now_epoch_ms >= connection.expires_at_epoch_ms
        || canonical_sha256(&preimage)? != connection.connection_sha256
    {
        return Err(McpRegistryError::IdentityDenied);
    }
    Ok(())
}

/// Seals deterministic discovery from the admitted manifest, never server-supplied substitutions.
pub fn mcp_discovery(
    manifest: &McpManifest,
    connection: &McpConnection,
) -> Result<McpDiscovery, McpRegistryError> {
    verify_mcp_manifest(manifest)?;
    if connection.server_id != manifest.server_id
        || connection.manifest_sha256 != manifest.manifest_sha256
    {
        return Err(McpRegistryError::IdentityDenied);
    }
    let mut discovery = McpDiscovery {
        connection_id: connection.connection_id.clone(),
        manifest_sha256: manifest.manifest_sha256.clone(),
        tool_ids: manifest
            .tools
            .iter()
            .map(|tool| {
                format!(
                    "{}@{}",
                    tool.definition.tool_id.as_str(),
                    tool.definition.tool_version
                )
            })
            .collect(),
        resource_ids: manifest
            .resources
            .iter()
            .map(|resource| resource.resource_id.clone())
            .collect(),
        prompt_ids: manifest
            .prompts
            .iter()
            .map(|prompt| prompt.prompt_id.clone())
            .collect(),
        discovery_sha256: ZERO_SHA256.to_owned(),
    };
    discovery.discovery_sha256 = canonical_sha256(&discovery)?;
    Ok(discovery)
}

fn validate_manifest(manifest: &McpManifest) -> Result<(), McpRegistryError> {
    if manifest.schema_version != CONTRACT_SCHEMA_VERSION
        || !valid_identifier(&manifest.server_id)
        || manifest.server_version.is_empty()
        || manifest.server_version.len() > MAX_VERSION_BYTES
        || !valid_sha256(&manifest.package_sha256)
        || !valid_sha256(&manifest.process_sha256)
        || !valid_sha256(&manifest.transport.endpoint_identity_sha256)
        || !manifest.cancellation_supported
        || manifest.tools.len() > MAX_CAPABILITIES
        || manifest.resources.len() > MAX_CAPABILITIES
        || manifest.prompts.len() > MAX_CAPABILITIES
        || manifest.tools.len() > manifest.limits.max_tools as usize
        || manifest.resources.len() > manifest.limits.max_resources as usize
        || manifest.prompts.len() > manifest.limits.max_prompts as usize
        || manifest.limits.max_tools as usize > MAX_CAPABILITIES
        || manifest.limits.max_resources as usize > MAX_CAPABILITIES
        || manifest.limits.max_prompts as usize > MAX_CAPABILITIES
        || manifest.limits.max_request_bytes == 0
        || manifest.limits.max_request_bytes > MAX_REQUEST_BYTES
        || manifest.limits.max_response_bytes == 0
        || manifest.limits.max_response_bytes > MAX_RESPONSE_BYTES
        || manifest.limits.max_response_items == 0
        || manifest.limits.max_response_items > MAX_RESPONSE_ITEMS
        || manifest.limits.timeout_ms == 0
        || manifest.limits.timeout_ms > MAX_TIMEOUT_MS
        || manifest.limits.max_concurrency == 0
        || manifest.limits.max_concurrency > MAX_CONCURRENCY
        || manifest.credential_ids.len() > MAX_CAPABILITIES
        || !manifest.credential_ids.is_empty()
        || manifest.requested_operations != [GrantOperation::WorkspaceRead]
        || !sorted_unique(&manifest.workspace_root_sha256s)
        || !manifest
            .workspace_root_sha256s
            .iter()
            .all(|value| valid_sha256(value))
        || !sorted_unique(&manifest.network_destinations)
        || !transport_valid(manifest)
        || !capability_order_valid(manifest)
    {
        return Err(McpRegistryError::InvalidManifest);
    }
    for tool in &manifest.tools {
        validate_read_only_tool(tool)?;
    }
    if manifest.resources.iter().any(|resource| {
        !valid_identifier(&resource.resource_id)
            || !valid_sha256(&resource.scope_sha256)
            || !valid_schema(&resource.response_schema)
            || resource.response_class != McpResponseClass::UntrustedResource
    }) || manifest.prompts.iter().any(|prompt| {
        !valid_identifier(&prompt.prompt_id)
            || !valid_schema(&prompt.request_schema)
            || !valid_schema(&prompt.response_schema)
            || prompt.response_class != McpResponseClass::UntrustedPrompt
    }) {
        return Err(McpRegistryError::InvalidManifest);
    }
    Ok(())
}

fn validate_read_only_tool(tool: &McpToolManifest) -> Result<(), McpRegistryError> {
    let definition = &tool.definition;
    if tool.side_effect != StateChange::NotChanged
        || !matches!(
            tool.response_class,
            McpResponseClass::UntrustedText | McpResponseClass::UntrustedStructuredData
        )
        || definition.declared_effects.len() != 1
        || definition.declared_effects[0].operation() != GrantOperation::WorkspaceRead
        || definition.required_grant.operation.operation() != GrantOperation::WorkspaceRead
        || !definition.required_grant.single_use
    {
        return Err(McpRegistryError::WriteCapabilityDenied);
    }
    let mut registry = ToolRegistry::new();
    registry
        .register_tool(Box::new(ManifestTool(definition.clone())))
        .map_err(|_| McpRegistryError::InvalidManifest)
}

fn transport_valid(manifest: &McpManifest) -> bool {
    let destination = &manifest.transport.destination;
    match manifest.transport.kind {
        McpTransportKind::InProcess | McpTransportKind::LocalProcessStdio => {
            destination.is_empty() && manifest.network_destinations.is_empty()
        }
        McpTransportKind::LocalSocket => {
            valid_destination(destination) && manifest.network_destinations.is_empty()
        }
        McpTransportKind::LoopbackTcp | McpTransportKind::RemoteHttps => {
            valid_destination(destination) && manifest.network_destinations == [destination.clone()]
        }
    }
}

fn capability_order_valid(manifest: &McpManifest) -> bool {
    manifest.tools.windows(2).all(|pair| {
        (
            &pair[0].definition.tool_id,
            &pair[0].definition.tool_version,
        ) < (
            &pair[1].definition.tool_id,
            &pair[1].definition.tool_version,
        )
    }) && manifest
        .resources
        .windows(2)
        .all(|pair| pair[0].resource_id < pair[1].resource_id)
        && manifest
            .prompts
            .windows(2)
            .all(|pair| pair[0].prompt_id < pair[1].prompt_id)
}

fn verify_observation(
    manifest: &McpManifest,
    observation: &McpProcessObservation,
) -> Result<(), McpRegistryError> {
    if observation.package_sha256 != manifest.package_sha256
        || observation.process_sha256 != manifest.process_sha256
        || observation.endpoint_identity_sha256 != manifest.transport.endpoint_identity_sha256
        || observation.transport != manifest.transport.kind
        || !observation.descendants_contained
    {
        return Err(McpRegistryError::IdentityDenied);
    }
    Ok(())
}

struct ManifestTool(ToolDefinition);

impl Tool for ManifestTool {
    fn definition(&self) -> &ToolDefinition {
        &self.0
    }
}

fn valid_schema(schema: &agentmage_kernel_contracts::SchemaReference) -> bool {
    valid_identifier(schema.schema_id.as_str())
        && schema.schema_version > 0
        && valid_sha256(&schema.schema_sha256)
}

fn sorted_unique<T: Ord>(values: &[T]) -> bool {
    values.windows(2).all(|pair| pair[0] < pair[1])
}

fn valid_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_ID_BYTES
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b':' | b'-'))
}

fn valid_destination(value: &str) -> bool {
    !value.is_empty() && value.len() <= 512 && !value.bytes().any(|byte| byte.is_ascii_control())
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn canonical_sha256(value: &impl Serialize) -> Result<String, McpRegistryError> {
    let bytes = serde_json::to_vec(value).map_err(|_| McpRegistryError::InvalidManifest)?;
    Ok(Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect())
}

#[cfg(test)]
pub(crate) fn fixture_mcp_manifest() -> McpManifest {
    use agentmage_kernel_contracts::{
        McpLimits, McpResponseClass, McpToolManifest, McpTransport, OperationBinding,
        RequiredGrantTemplate, SchemaId, SchemaReference, ToolRiskLevel,
    };

    let schema = |id: &str, byte: char| SchemaReference {
        schema_id: SchemaId::from_raw(id),
        schema_version: 1,
        schema_sha256: byte.to_string().repeat(64),
    };
    seal_mcp_manifest(McpManifest {
        schema_version: CONTRACT_SCHEMA_VERSION,
        server_id: "fixture.mcp".to_owned(),
        server_version: "1.0.0".to_owned(),
        package_sha256: "3".repeat(64),
        process_sha256: "4".repeat(64),
        transport: McpTransport {
            kind: McpTransportKind::LocalProcessStdio,
            endpoint_identity_sha256: "5".repeat(64),
            destination: String::new(),
        },
        tools: vec![McpToolManifest {
            definition: ToolDefinition {
                schema_version: CONTRACT_SCHEMA_VERSION,
                tool_id: ToolId::from_raw("mcp.fixture.read"),
                tool_version: "1.0.0".to_owned(),
                display_name: "Fixture MCP read".to_owned(),
                description: "Reads one exact reviewed fixture scope".to_owned(),
                input_schema: schema("mcp.fixture.input", '1'),
                output_schema: schema("mcp.fixture.output", '2'),
                risk_level: ToolRiskLevel::Low,
                declared_effects: vec![OperationBinding::new(GrantOperation::WorkspaceRead)],
                required_grant: RequiredGrantTemplate {
                    operation: OperationBinding::new(GrantOperation::WorkspaceRead),
                    target_scope: "mcp.fixture.scope".to_owned(),
                    single_use: true,
                },
                timeout_ms: 1_000,
            },
            response_class: McpResponseClass::UntrustedStructuredData,
            side_effect: StateChange::NotChanged,
        }],
        resources: Vec::new(),
        prompts: Vec::new(),
        workspace_root_sha256s: vec!["6".repeat(64)],
        network_destinations: Vec::new(),
        credential_ids: Vec::new(),
        limits: McpLimits {
            max_tools: 1,
            max_resources: 1,
            max_prompts: 1,
            max_request_bytes: 4_096,
            max_response_bytes: 8_192,
            max_response_items: 16,
            timeout_ms: 5_000,
            max_concurrency: 1,
        },
        cancellation_supported: true,
        requested_operations: vec![GrantOperation::WorkspaceRead],
        manifest_sha256: ZERO_SHA256.to_owned(),
    })
    .expect("read-only MCP fixture manifest")
}

#[cfg(test)]
pub(crate) fn fixture_mcp_observation(manifest: &McpManifest) -> McpProcessObservation {
    McpProcessObservation {
        package_sha256: manifest.package_sha256.clone(),
        process_sha256: manifest.process_sha256.clone(),
        endpoint_identity_sha256: manifest.transport.endpoint_identity_sha256.clone(),
        transport: manifest.transport.kind,
        descendants_contained: true,
    }
}

#[cfg(test)]
mod tests {
    use agentmage_kernel_contracts::{
        AuthorityClass, McpLimits, McpResponseClass, McpToolManifest, McpTransport,
        OperationBinding, RequiredGrantTemplate, SchemaId, SchemaReference, StateChange,
        ToolRiskLevel,
    };

    use super::*;

    struct NativeTool(ToolDefinition);

    impl Tool for NativeTool {
        fn definition(&self) -> &ToolDefinition {
            &self.0
        }
    }

    fn schema(id: &str, byte: char) -> SchemaReference {
        SchemaReference {
            schema_id: SchemaId::from_raw(id),
            schema_version: 1,
            schema_sha256: byte.to_string().repeat(64),
        }
    }

    fn tool(id: &str) -> ToolDefinition {
        ToolDefinition {
            schema_version: CONTRACT_SCHEMA_VERSION,
            tool_id: ToolId::from_raw(id),
            tool_version: "1.0.0".to_owned(),
            display_name: "Fixture MCP read".to_owned(),
            description: "Reads one exact reviewed fixture scope".to_owned(),
            input_schema: schema("mcp.fixture.input", '1'),
            output_schema: schema("mcp.fixture.output", '2'),
            risk_level: ToolRiskLevel::Low,
            declared_effects: vec![OperationBinding::new(GrantOperation::WorkspaceRead)],
            required_grant: RequiredGrantTemplate {
                operation: OperationBinding::new(GrantOperation::WorkspaceRead),
                target_scope: "mcp.fixture.scope".to_owned(),
                single_use: true,
            },
            timeout_ms: 1_000,
        }
    }

    fn manifest() -> McpManifest {
        seal_mcp_manifest(McpManifest {
            schema_version: CONTRACT_SCHEMA_VERSION,
            server_id: "fixture.mcp".to_owned(),
            server_version: "1.0.0".to_owned(),
            package_sha256: "3".repeat(64),
            process_sha256: "4".repeat(64),
            transport: McpTransport {
                kind: McpTransportKind::LocalProcessStdio,
                endpoint_identity_sha256: "5".repeat(64),
                destination: String::new(),
            },
            tools: vec![McpToolManifest {
                definition: tool("mcp.fixture.read"),
                response_class: McpResponseClass::UntrustedStructuredData,
                side_effect: StateChange::NotChanged,
            }],
            resources: Vec::new(),
            prompts: Vec::new(),
            workspace_root_sha256s: vec!["6".repeat(64)],
            network_destinations: Vec::new(),
            credential_ids: Vec::new(),
            limits: McpLimits {
                max_tools: 1,
                max_resources: 1,
                max_prompts: 1,
                max_request_bytes: 4_096,
                max_response_bytes: 8_192,
                max_response_items: 16,
                timeout_ms: 5_000,
                max_concurrency: 1,
            },
            cancellation_supported: true,
            requested_operations: vec![GrantOperation::WorkspaceRead],
            manifest_sha256: ZERO_SHA256.to_owned(),
        })
        .expect("read-only MCP manifest")
    }

    fn observation(manifest: &McpManifest) -> McpProcessObservation {
        McpProcessObservation {
            package_sha256: manifest.package_sha256.clone(),
            process_sha256: manifest.process_sha256.clone(),
            endpoint_identity_sha256: manifest.transport.endpoint_identity_sha256.clone(),
            transport: manifest.transport.kind,
            descendants_contained: true,
        }
    }

    #[test]
    fn sprint_80_manifest_and_every_transport_are_explicit_and_sealed() {
        let base = manifest();
        verify_mcp_manifest(&base).expect("manifest verifies");
        for (kind, destination, network) in [
            (McpTransportKind::InProcess, "", vec![]),
            (McpTransportKind::LocalProcessStdio, "", vec![]),
            (McpTransportKind::LocalSocket, "unix:fixture.sock", vec![]),
            (
                McpTransportKind::LoopbackTcp,
                "127.0.0.1:3100",
                vec!["127.0.0.1:3100".to_owned()],
            ),
            (
                McpTransportKind::RemoteHttps,
                "https://mcp.example.invalid",
                vec!["https://mcp.example.invalid".to_owned()],
            ),
        ] {
            let mut candidate = base.clone();
            candidate.transport.kind = kind;
            candidate.transport.destination = destination.to_owned();
            candidate.network_destinations = network;
            candidate.manifest_sha256 = ZERO_SHA256.to_owned();
            seal_mcp_manifest(candidate).expect("explicit transport seals");
        }
    }

    #[test]
    fn sprint_80_write_secret_and_uncontained_capabilities_fail_closed() {
        let mut write = manifest();
        write.tools[0].side_effect = StateChange::Changed;
        write.manifest_sha256 = ZERO_SHA256.to_owned();
        assert_eq!(
            seal_mcp_manifest(write),
            Err(McpRegistryError::WriteCapabilityDenied)
        );

        let mut privileged = manifest();
        privileged.requested_operations = vec![GrantOperation::WorkspaceWrite];
        privileged.manifest_sha256 = ZERO_SHA256.to_owned();
        assert_eq!(
            seal_mcp_manifest(privileged),
            Err(McpRegistryError::InvalidManifest)
        );

        let mut secret = manifest();
        secret.credential_ids = vec!["credential.fixture".to_owned()];
        secret.manifest_sha256 = ZERO_SHA256.to_owned();
        assert_eq!(
            seal_mcp_manifest(secret),
            Err(McpRegistryError::InvalidManifest)
        );
    }

    #[test]
    fn sprint_80_identity_change_expiry_and_containment_invalidate_connection() {
        let manifest = manifest();
        let observed = observation(&manifest);
        let connection = admit_mcp_connection(
            &manifest,
            &observed,
            "connection-fixture".to_owned(),
            1_000,
            2_000,
        )
        .expect("connection admitted");
        verify_mcp_connection(&manifest, &connection, &observed, 1_500)
            .expect("connection verifies");
        mcp_discovery(&manifest, &connection).expect("discovery seals");

        let mut changed = observed.clone();
        changed.process_sha256 = "a".repeat(64);
        assert_eq!(
            verify_mcp_connection(&manifest, &connection, &changed, 1_500),
            Err(McpRegistryError::IdentityDenied)
        );
        assert_eq!(
            verify_mcp_connection(&manifest, &connection, &observed, 2_000),
            Err(McpRegistryError::IdentityDenied)
        );
        let mut escaped = observed;
        escaped.descendants_contained = false;
        assert_eq!(
            verify_mcp_connection(&manifest, &connection, &escaped, 1_500),
            Err(McpRegistryError::IdentityDenied)
        );
    }

    #[test]
    fn sprint_80_optional_tools_cannot_shadow_or_become_native_prerequisites() {
        let mut native = ToolRegistry::new();
        native
            .register_tool(Box::new(NativeTool(tool("native.fixture.read"))))
            .expect("native tool");
        let mut registry = McpManifestRegistry::new(&native);
        registry.register(manifest()).expect("MCP manifest");

        let mut combined = ToolRegistry::new();
        combined
            .register_tool(Box::new(NativeTool(tool("native.fixture.read"))))
            .expect("native tool");
        registry
            .register_tools_into("fixture.mcp", &mut combined)
            .expect("optional MCP adapter");
        assert_eq!(combined.list_tools().len(), 2);

        registry.disable("fixture.mcp").expect("disable MCP");
        let mut native_only = ToolRegistry::new();
        native_only
            .register_tool(Box::new(NativeTool(tool("native.fixture.read"))))
            .expect("native tool remains independent");
        assert_eq!(native_only.list_tools().len(), 1);

        let mut shadow = manifest();
        shadow.server_id = "fixture.shadow".to_owned();
        shadow.tools[0].definition.tool_id = ToolId::from_raw("native.fixture.read");
        shadow.manifest_sha256 = ZERO_SHA256.to_owned();
        let shadow = seal_mcp_manifest(shadow).expect("otherwise valid shadow");
        assert_eq!(
            registry.register(shadow),
            Err(McpRegistryError::ToolShadowDenied)
        );
        assert_eq!(
            GrantOperation::WorkspaceRead.authority_class(),
            AuthorityClass::Observe
        );
    }
}
