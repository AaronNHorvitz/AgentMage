//! Native capability registration for the shared runtime tool dispatcher.

use agentmage_capability_read_only::{
    GitInspectionError, ReadOnlyToolKind, git_inspection_tool_definition,
    read_only_tool_definition, validate_git_inspection_request, validate_read_only_request,
};
use agentmage_kernel_contracts::{ToolDefinition, ValidationIssue, ValidationSeverity};
use agentmage_kernel_engine::tooling::{Tool, ToolRegistry};

/// Stable failure while constructing an exact native runtime catalog.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NativeToolCatalogError {
    /// One built-in definition or exact identity could not enter the common registry.
    RegistrationDenied,
}

impl NativeToolCatalogError {
    /// Returns one stable content-free diagnostic code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::RegistrationDenied => "runtime.native-tool.registration-denied",
        }
    }
}

struct RegisteredReadOnlyTool {
    definition: ToolDefinition,
    kind: ReadOnlyToolKind,
}

struct RegisteredGitInspectionTool {
    definition: ToolDefinition,
}

impl Tool for RegisteredGitInspectionTool {
    fn definition(&self) -> &ToolDefinition {
        &self.definition
    }

    fn validate_arguments(&self, arguments: &[u8]) -> Vec<ValidationIssue> {
        validate_git_inspection_request(arguments)
            .map_or_else(|error| vec![git_validation_issue(error)], |_| Vec::new())
    }
}

impl Tool for RegisteredReadOnlyTool {
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
                    message: "Read-only runtime arguments failed closed validation".to_owned(),
                }]
            },
            |_| Vec::new(),
        )
    }
}

/// Registers every exact built-in read-only capability with one common registry.
///
/// Registration exposes definitions and validation only. It does not supply a workspace,
/// grant, approval, execution callback, or model authority.
pub fn register_read_only_runtime_tools(
    registry: &mut ToolRegistry,
) -> Result<(), NativeToolCatalogError> {
    for kind in ReadOnlyToolKind::ALL {
        registry
            .register_tool(Box::new(RegisteredReadOnlyTool {
                definition: read_only_tool_definition(kind),
                kind,
            }))
            .map_err(|_| NativeToolCatalogError::RegistrationDenied)?;
    }
    registry
        .register_tool(Box::new(RegisteredGitInspectionTool {
            definition: git_inspection_tool_definition(),
        }))
        .map_err(|_| NativeToolCatalogError::RegistrationDenied)?;
    Ok(())
}

fn git_validation_issue(error: GitInspectionError) -> ValidationIssue {
    ValidationIssue {
        code: error.code().to_owned(),
        severity: ValidationSeverity::Error,
        field_path: vec!["arguments".to_owned()],
        message: "Git inspection runtime arguments failed closed validation".to_owned(),
    }
}

/// Builds the exact first-party read-only catalog used by reusable runtime clients.
pub fn read_only_runtime_registry() -> Result<ToolRegistry, NativeToolCatalogError> {
    let mut registry = ToolRegistry::new();
    register_read_only_runtime_tools(&mut registry)?;
    Ok(registry)
}

#[cfg(test)]
mod tests {
    use agentmage_capability_read_only::{
        GIT_INSPECTION_INPUT_SCHEMA_ID, GIT_INSPECTION_TOOL_ID, GIT_INSPECTION_TOOL_VERSION,
        GitInspectionOperation, GitInspectionRequest, READ_ONLY_INPUT_SCHEMA_ID,
        READ_ONLY_TOOL_VERSION, ReadOnlyEncoding, ReadOnlyLimits, ReadOnlyRequest,
        ReadOnlyToolKind,
    };
    use agentmage_kernel_contracts::{
        ActionId, ContractPayload, CorrelationId, GrantOperation, ToolCall, ToolCallId, ToolId,
    };
    use agentmage_kernel_engine::runtime_loop::runtime_tool_references;
    use sha2::{Digest, Sha256};

    use super::{read_only_runtime_registry, register_read_only_runtime_tools};

    #[test]
    fn story_23_4_all_existing_read_only_tools_share_the_common_registry() {
        let registry = read_only_runtime_registry().expect("read-only catalog");
        let definitions = registry.list_tools();
        assert_eq!(definitions.len(), ReadOnlyToolKind::ALL.len() + 1);
        assert_eq!(definitions.len(), 11);
        for definition in &definitions {
            assert_eq!(definition.tool_version, READ_ONLY_TOOL_VERSION);
            assert_eq!(definition.declared_effects.len(), 1);
            assert_eq!(
                definition.declared_effects[0].operation(),
                GrantOperation::WorkspaceRead
            );
            assert_eq!(
                definition.required_grant.operation.operation(),
                GrantOperation::WorkspaceRead
            );
            assert!(definition.required_grant.single_use);
        }
        let references = runtime_tool_references(&registry).expect("runtime references");
        assert_eq!(references.len(), definitions.len());
        assert!(
            references
                .windows(2)
                .all(|pair| pair[0].tool_id < pair[1].tool_id)
        );
    }

    #[test]
    fn story_23_4_git_inspection_uses_its_existing_fixed_planner() {
        let registry = read_only_runtime_registry().expect("read-only catalog");
        let definition = registry
            .get_tool(
                &ToolId::from_raw(GIT_INSPECTION_TOOL_ID),
                GIT_INSPECTION_TOOL_VERSION,
            )
            .expect("Git inspection definition")
            .clone();
        let request = GitInspectionRequest {
            schema_version: 1,
            operation: GitInspectionOperation::Status,
            revision: None,
            object_id: None,
            pathspecs: Vec::new(),
            max_records: 100,
            max_output_bytes: 64 * 1024,
        };
        let bytes = serde_json::to_vec(&request).expect("Git request bytes");
        let call = ToolCall {
            schema_version: agentmage_kernel_contracts::CONTRACT_SCHEMA_VERSION,
            tool_call_id: ToolCallId::from_raw("git-call-0001"),
            correlation_id: CorrelationId::from_raw("correlation-git-0001"),
            action_id: ActionId::from_raw("action-git-0001"),
            tool_id: definition.tool_id,
            tool_version: definition.tool_version,
            arguments: ContractPayload {
                schema: definition.input_schema,
                media_type: "application/json".to_owned(),
                sha256: sha256(&bytes),
                bytes,
            },
        };
        assert!(registry.validate_arguments(&call).is_ok());

        let mut unsafe_revision = call;
        unsafe_revision.arguments.bytes = br#"{"schema_version":1,"operation":"show","revision":"--exec-path=/tmp","object_id":null,"pathspecs":[],"max_records":100,"max_output_bytes":65536}"#.to_vec();
        unsafe_revision.arguments.sha256 = sha256(&unsafe_revision.arguments.bytes);
        assert!(registry.validate_arguments(&unsafe_revision).is_err());
        assert_eq!(
            unsafe_revision.arguments.schema.schema_id.as_str(),
            GIT_INSPECTION_INPUT_SCHEMA_ID
        );
    }

    #[test]
    fn story_23_4_registry_runs_the_existing_closed_argument_validator() {
        let registry = read_only_runtime_registry().expect("read-only catalog");
        let kind = ReadOnlyToolKind::ReadText;
        let definition = registry
            .get_tool(&ToolId::from_raw(kind.id()), READ_ONLY_TOOL_VERSION)
            .expect("read definition")
            .clone();
        let request = ReadOnlyRequest {
            schema_version: 1,
            paths: vec![vec!["src".to_owned(), "lib.rs".to_owned()]],
            query: None,
            byte_offset: Some(0),
            byte_count: Some(256),
            encoding: ReadOnlyEncoding::Utf8,
            limits: ReadOnlyLimits::default(),
            call_depth: 0,
        };
        let bytes = serde_json::to_vec(&request).expect("request bytes");
        let call = ToolCall {
            schema_version: agentmage_kernel_contracts::CONTRACT_SCHEMA_VERSION,
            tool_call_id: ToolCallId::from_raw("read-call-0001"),
            correlation_id: CorrelationId::from_raw("correlation-0001"),
            action_id: ActionId::from_raw("action-0001"),
            tool_id: definition.tool_id,
            tool_version: definition.tool_version,
            arguments: ContractPayload {
                schema: definition.input_schema,
                media_type: "application/json".to_owned(),
                sha256: sha256(&bytes),
                bytes,
            },
        };
        assert!(registry.validate_arguments(&call).is_ok());

        let mut malformed = call;
        malformed.arguments.bytes = br#"{"schema_version":1}"#.to_vec();
        malformed.arguments.sha256 = sha256(&malformed.arguments.bytes);
        assert!(registry.validate_arguments(&malformed).is_err());
        assert_eq!(
            malformed.arguments.schema.schema_id.as_str(),
            READ_ONLY_INPUT_SCHEMA_ID
        );
    }

    #[test]
    fn story_23_4_duplicate_pack_registration_fails_without_replacing_tools() {
        let mut registry = read_only_runtime_registry().expect("read-only catalog");
        let before = runtime_tool_references(&registry).expect("before");
        assert!(register_read_only_runtime_tools(&mut registry).is_err());
        assert_eq!(runtime_tool_references(&registry).expect("after"), before);
    }

    fn sha256(bytes: &[u8]) -> String {
        Sha256::digest(bytes)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect()
    }
}
