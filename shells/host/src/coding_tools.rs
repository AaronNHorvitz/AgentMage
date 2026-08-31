//! Native coding-tool definitions composed from existing kernel contracts.

use std::fmt::Write;

use agentmage_capability_read_only::{
    ARTIFACT_INPUT_SCHEMA_ID, ARTIFACT_INPUT_SCHEMA_JSON, GIT_INSPECTION_INPUT_SCHEMA_ID,
    GIT_INSPECTION_INPUT_SCHEMA_JSON, READ_ONLY_INPUT_SCHEMA_ID, READ_ONLY_INPUT_SCHEMA_JSON,
};
use agentmage_kernel_contracts::{
    CONTRACT_SCHEMA_VERSION, GrantOperation, OperationBinding, RequiredGrantTemplate, SchemaId,
    SchemaReference, ToolDefinition, ToolId, ToolRiskLevel, ValidationIssue, ValidationSeverity,
};
use agentmage_kernel_engine::{
    command_runner::{CommandRegistry, CommandRequest, prepare_command},
    tooling::{Tool, ToolRegistry},
    validation_template::ValidationTemplateRegistry,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{
    coding_changes::{
        CONTROLLED_CREATE_INPUT_SCHEMA_ID, CONTROLLED_CREATE_INPUT_SCHEMA_JSON, CodingWriteScope,
        STRUCTURED_PATCH_INPUT_SCHEMA_ID, STRUCTURED_PATCH_INPUT_SCHEMA_JSON,
        register_controlled_change_runtime_tools,
    },
    runtime_tools::read_only_runtime_registry,
};

/// Stable native identity for exact registered command execution.
pub const BOUNDED_COMMAND_TOOL_ID: &str = "agentmage.command.run-template";

/// Immutable contract version for exact registered command execution.
pub const BOUNDED_COMMAND_TOOL_VERSION: &str = "1.0.0";

/// Closed input-schema identity for one exact command-template selection.
pub const BOUNDED_COMMAND_INPUT_SCHEMA_ID: &str = "agentmage.command.request";

/// Closed output-schema identity for one bounded command receipt.
pub const BOUNDED_COMMAND_OUTPUT_SCHEMA_ID: &str = "agentmage.command.receipt";

/// Canonical closed JSON Schema for one exact registered command request.
pub const BOUNDED_COMMAND_INPUT_SCHEMA_JSON: &str = r#"{"$schema":"https://json-schema.org/draft/2020-12/schema","$id":"agentmage.command.request","type":"object","additionalProperties":false,"required":["schema_version","command_attempt_id","template_id","template_version","spec_sha256"],"properties":{"schema_version":{"const":1},"command_attempt_id":{"type":"string","pattern":"^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$"},"template_id":{"type":"string","pattern":"^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$"},"template_version":{"type":"string","pattern":"^[0-9]+\\.[0-9]+\\.[0-9]+$","maxLength":64},"spec_sha256":{"type":"string","pattern":"^[0-9a-f]{64}$"}}}"#;

/// Published closed JSON Schema for one bounded command receipt.
pub const BOUNDED_COMMAND_OUTPUT_SCHEMA_JSON: &str =
    include_str!("../../../schemas/runtime/command-receipt.schema.json");

/// Stable native identity for one exact registered validation template.
pub const TARGETED_VALIDATION_TOOL_ID: &str = "agentmage.validation.run-template";

/// Immutable contract version for exact registered validation execution.
pub const TARGETED_VALIDATION_TOOL_VERSION: &str = "1.0.0";

/// Closed input-schema identity for one exact validation-template selection.
pub const TARGETED_VALIDATION_INPUT_SCHEMA_ID: &str = "agentmage.validation.request";

/// Closed output-schema identity for one trusted validation receipt.
pub const TARGETED_VALIDATION_OUTPUT_SCHEMA_ID: &str = "agentmage.validation.receipt";

/// Canonical closed JSON Schema for one exact registered validation request.
pub const TARGETED_VALIDATION_INPUT_SCHEMA_JSON: &str = r#"{"$schema":"https://json-schema.org/draft/2020-12/schema","$id":"agentmage.validation.request","type":"object","additionalProperties":false,"required":["schema_version","validation_attempt_id","validation_id","template_sha256"],"properties":{"schema_version":{"const":1},"validation_attempt_id":{"type":"string","pattern":"^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$"},"validation_id":{"type":"string","pattern":"^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$"},"template_sha256":{"type":"string","pattern":"^[0-9a-f]{64}$"}}}"#;

/// Published closed JSON Schema for one trusted validation receipt.
pub const TARGETED_VALIDATION_OUTPUT_SCHEMA_JSON: &str =
    include_str!("../../../schemas/runtime/validation-receipt.schema.json");

/// Exact model-facing selector for a pre-registered validation template.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TargetedValidationRequest {
    /// Closed request schema version.
    pub schema_version: u16,
    /// Non-replayable validation-attempt identity.
    pub validation_attempt_id: String,
    /// Exact registered validation identity.
    pub validation_id: String,
    /// Exact immutable validation-template digest.
    pub template_sha256: String,
}

/// Stable failure while composing a native coding-tool catalog.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CodingToolCatalogError {
    /// One exact definition could not enter the common registry.
    RegistrationDenied,
    /// A model-visible schema was absent or did not match the registered schema digest.
    SchemaDenied,
}

impl CodingToolCatalogError {
    /// Returns one stable content-free diagnostic code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::RegistrationDenied => "runtime.coding-tool.registration-denied",
            Self::SchemaDenied => "runtime.coding-tool.schema-denied",
        }
    }
}

/// Inert model-visible projection of one exact native tool and its validated input schema.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CodingModelToolContract {
    /// Exact declarative definition used by the common registry and dispatcher.
    pub definition: ToolDefinition,
    /// Parsed JSON Schema whose bytes match the definition's input-schema digest.
    pub input_schema: serde_json::Value,
}

struct RegisteredCommandTool {
    definition: ToolDefinition,
    commands: CommandRegistry,
}

struct RegisteredValidationTool {
    definition: ToolDefinition,
    commands: CommandRegistry,
    validations: ValidationTemplateRegistry,
}

impl Tool for RegisteredValidationTool {
    fn definition(&self) -> &ToolDefinition {
        &self.definition
    }

    fn validate_arguments(&self, arguments: &[u8]) -> Vec<ValidationIssue> {
        let valid = serde_json::from_slice::<TargetedValidationRequest>(arguments)
            .ok()
            .and_then(|request| {
                if request.schema_version != 1 {
                    return None;
                }
                let template = self
                    .validations
                    .resolve(&request.validation_id, &request.template_sha256)?;
                let command_request =
                    CommandRequest::new(request.validation_attempt_id, &template.command);
                prepare_command(&self.commands, command_request).ok()
            })
            .is_some();
        if valid {
            Vec::new()
        } else {
            vec![ValidationIssue {
                code: "validation.request.invalid".to_owned(),
                severity: ValidationSeverity::Error,
                field_path: vec!["arguments".to_owned()],
                message: "Validation request did not select one exact registered template"
                    .to_owned(),
            }]
        }
    }
}

impl Tool for RegisteredCommandTool {
    fn definition(&self) -> &ToolDefinition {
        &self.definition
    }

    fn validate_arguments(&self, arguments: &[u8]) -> Vec<ValidationIssue> {
        let request = serde_json::from_slice::<CommandRequest>(arguments);
        match request.and_then(|request| {
            prepare_command(&self.commands, request).map_err(serde_command_error)
        }) {
            Ok(_) => Vec::new(),
            Err(_) => vec![ValidationIssue {
                code: "command.request.invalid".to_owned(),
                severity: ValidationSeverity::Error,
                field_path: vec!["arguments".to_owned()],
                message: "Command request did not select one exact registered template".to_owned(),
            }],
        }
    }
}

/// Returns the declarative definition for exact registered command execution.
#[must_use]
pub fn bounded_command_tool_definition() -> ToolDefinition {
    let operation = OperationBinding::new(GrantOperation::CommandExecute);
    ToolDefinition {
        schema_version: CONTRACT_SCHEMA_VERSION,
        tool_id: ToolId::from_raw(BOUNDED_COMMAND_TOOL_ID),
        tool_version: BOUNDED_COMMAND_TOOL_VERSION.to_owned(),
        display_name: "Run registered command".to_owned(),
        description: "Selects one frozen noninteractive offline command template by exact digest"
            .to_owned(),
        input_schema: schema(
            BOUNDED_COMMAND_INPUT_SCHEMA_ID,
            BOUNDED_COMMAND_INPUT_SCHEMA_JSON.as_bytes(),
        ),
        output_schema: schema(
            BOUNDED_COMMAND_OUTPUT_SCHEMA_ID,
            BOUNDED_COMMAND_OUTPUT_SCHEMA_JSON.as_bytes(),
        ),
        risk_level: ToolRiskLevel::Moderate,
        declared_effects: vec![operation],
        required_grant: RequiredGrantTemplate {
            operation,
            target_scope: "one-exact-registered-command-template".to_owned(),
            single_use: true,
        },
        timeout_ms: 300_000,
    }
}

/// Returns the declarative definition for exact registered validation execution.
#[must_use]
pub fn targeted_validation_tool_definition() -> ToolDefinition {
    let operation = OperationBinding::new(GrantOperation::CommandExecute);
    ToolDefinition {
        schema_version: CONTRACT_SCHEMA_VERSION,
        tool_id: ToolId::from_raw(TARGETED_VALIDATION_TOOL_ID),
        tool_version: TARGETED_VALIDATION_TOOL_VERSION.to_owned(),
        display_name: "Run targeted validation".to_owned(),
        description: "Selects one trusted validation template and its frozen command by digest"
            .to_owned(),
        input_schema: schema(
            TARGETED_VALIDATION_INPUT_SCHEMA_ID,
            TARGETED_VALIDATION_INPUT_SCHEMA_JSON.as_bytes(),
        ),
        output_schema: schema(
            TARGETED_VALIDATION_OUTPUT_SCHEMA_ID,
            TARGETED_VALIDATION_OUTPUT_SCHEMA_JSON.as_bytes(),
        ),
        risk_level: ToolRiskLevel::Moderate,
        declared_effects: vec![operation],
        required_grant: RequiredGrantTemplate {
            operation,
            target_scope: "one-exact-registered-validation-template".to_owned(),
            single_use: true,
        },
        timeout_ms: 300_000,
    }
}

/// Registers exact command selection through the common non-executing tool registry.
///
/// Registration consumes no grant and exposes no executable path or argument override.
pub fn register_bounded_command_runtime_tool(
    registry: &mut ToolRegistry,
    commands: CommandRegistry,
) -> Result<(), CodingToolCatalogError> {
    registry
        .register_tool(Box::new(RegisteredCommandTool {
            definition: bounded_command_tool_definition(),
            commands,
        }))
        .map_err(|_| CodingToolCatalogError::RegistrationDenied)
}

/// Registers targeted validation through the common non-executing tool registry.
///
/// Every validation command must already be present in the supplied command registry.
pub fn register_targeted_validation_runtime_tool(
    registry: &mut ToolRegistry,
    commands: CommandRegistry,
    validations: ValidationTemplateRegistry,
) -> Result<(), CodingToolCatalogError> {
    if !validations.verify()
        || validations.templates.iter().any(|template| {
            prepare_command(
                &commands,
                CommandRequest::new("validation-registration-check", &template.command),
            )
            .is_err()
        })
    {
        return Err(CodingToolCatalogError::RegistrationDenied);
    }
    registry
        .register_tool(Box::new(RegisteredValidationTool {
            definition: targeted_validation_tool_definition(),
            commands,
            validations,
        }))
        .map_err(|_| CodingToolCatalogError::RegistrationDenied)
}

/// Builds the exact native tool catalog for one bounded coding session.
///
/// The catalog composes existing first-party contracts directly through the kernel registry.
/// It contains no MCP translation, network operation, remote Git action, or publication tool.
pub fn native_coding_runtime_registry(
    write_scope: CodingWriteScope,
    commands: CommandRegistry,
    validations: ValidationTemplateRegistry,
) -> Result<ToolRegistry, CodingToolCatalogError> {
    let mut registry =
        read_only_runtime_registry().map_err(|_| CodingToolCatalogError::RegistrationDenied)?;
    register_controlled_change_runtime_tools(&mut registry, write_scope)
        .map_err(|_| CodingToolCatalogError::RegistrationDenied)?;
    register_bounded_command_runtime_tool(&mut registry, commands.clone())?;
    register_targeted_validation_runtime_tool(&mut registry, commands, validations)?;
    Ok(registry)
}

/// Projects the registered native catalog into inert model-visible definitions.
///
/// This projection contains no implementation, dispatcher, grant, path handle, or effect API.
pub fn model_visible_coding_tools(
    registry: &ToolRegistry,
) -> Result<Vec<CodingModelToolContract>, CodingToolCatalogError> {
    registry
        .list_tools()
        .into_iter()
        .map(|definition| {
            let schema_json = coding_input_schema_json(definition.input_schema.schema_id.as_str())
                .ok_or(CodingToolCatalogError::SchemaDenied)?;
            if definition.input_schema.schema_sha256 != sha256_hex(schema_json.as_bytes()) {
                return Err(CodingToolCatalogError::SchemaDenied);
            }
            let input_schema = serde_json::from_str(schema_json)
                .map_err(|_| CodingToolCatalogError::SchemaDenied)?;
            Ok(CodingModelToolContract {
                definition: definition.clone(),
                input_schema,
            })
        })
        .collect()
}

fn coding_input_schema_json(schema_id: &str) -> Option<&'static str> {
    match schema_id {
        ARTIFACT_INPUT_SCHEMA_ID => Some(ARTIFACT_INPUT_SCHEMA_JSON),
        READ_ONLY_INPUT_SCHEMA_ID => Some(READ_ONLY_INPUT_SCHEMA_JSON),
        GIT_INSPECTION_INPUT_SCHEMA_ID => Some(GIT_INSPECTION_INPUT_SCHEMA_JSON),
        STRUCTURED_PATCH_INPUT_SCHEMA_ID => Some(STRUCTURED_PATCH_INPUT_SCHEMA_JSON),
        CONTROLLED_CREATE_INPUT_SCHEMA_ID => Some(CONTROLLED_CREATE_INPUT_SCHEMA_JSON),
        BOUNDED_COMMAND_INPUT_SCHEMA_ID => Some(BOUNDED_COMMAND_INPUT_SCHEMA_JSON),
        TARGETED_VALIDATION_INPUT_SCHEMA_ID => Some(TARGETED_VALIDATION_INPUT_SCHEMA_JSON),
        _ => None,
    }
}

fn serde_command_error(
    _: agentmage_kernel_engine::command_runner::CommandError,
) -> serde_json::Error {
    serde_json::Error::io(std::io::Error::new(
        std::io::ErrorKind::InvalidData,
        "command request failed closed validation",
    ))
}

fn schema(id: &str, bytes: &[u8]) -> SchemaReference {
    SchemaReference {
        schema_id: SchemaId::from_raw(id),
        schema_version: 1,
        schema_sha256: sha256_hex(bytes),
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(64);
    for byte in Sha256::digest(bytes) {
        write!(&mut output, "{byte:02x}").expect("writing to String cannot fail");
    }
    output
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use agentmage_capability_read_only::{
        ArtifactToolKind, GIT_INSPECTION_TOOL_ID, ReadOnlyToolKind,
    };
    use agentmage_kernel_contracts::{
        ActionId, ContractPayload, CorrelationId, GrantOperation, ToolCall, ToolCallId, ToolId,
        ToolRiskLevel, WorkspaceId, WorkspacePath,
    };
    use agentmage_kernel_engine::command_runner::{
        CommandBounds, CommandRequest, CommandRisk, CommandSpec, CommandWorkingDirectory,
    };
    use agentmage_kernel_engine::validation_template::{
        ValidationKind, ValidationParserKind, ValidationTemplateInput, ValidationTemplateSource,
        seal_validation_template,
    };

    use super::*;
    use crate::coding_changes::{
        CONTROLLED_CREATE_TOOL_ID, CodingWriteScope, STRUCTURED_PATCH_TOOL_ID,
    };

    fn command() -> CommandSpec {
        CommandSpec::seal(
            "fixture.true",
            "1.0.0",
            "/usr/bin/true",
            "a".repeat(64),
            Vec::new(),
            CommandWorkingDirectory::EmptyScratch,
            BTreeMap::from([
                ("LANG".to_owned(), "C".to_owned()),
                ("TZ".to_owned(), "UTC".to_owned()),
            ]),
            CommandRisk::Low,
            CommandBounds::new(1_000, 1_024, 1_024, 32 * 1024 * 1024, 4, 100).expect("bounds"),
        )
        .expect("command")
    }

    fn call(definition: &ToolDefinition, bytes: Vec<u8>) -> ToolCall {
        ToolCall {
            schema_version: CONTRACT_SCHEMA_VERSION,
            tool_call_id: ToolCallId::from_raw("command-call-0001"),
            correlation_id: CorrelationId::from_raw("correlation-command-0001"),
            action_id: ActionId::from_raw("action-command-0001"),
            tool_id: definition.tool_id.clone(),
            tool_version: definition.tool_version.clone(),
            arguments: ContractPayload {
                schema: definition.input_schema.clone(),
                media_type: "application/json".to_owned(),
                sha256: sha256_hex(&bytes),
                bytes,
            },
        }
    }

    fn validations(command: CommandSpec) -> ValidationTemplateRegistry {
        let template = seal_validation_template(ValidationTemplateInput {
            validation_id: "validation-unit".to_owned(),
            kind: ValidationKind::Unit,
            command,
            source: ValidationTemplateSource::TrustedProjectConfiguration,
            source_sha256: "b".repeat(64),
            source_path: Some(
                WorkspacePath::new(WorkspaceId::from_raw("workspace-coding"), ["Cargo.toml"])
                    .expect("source path"),
            ),
            user_input_approval_sha256: None,
            execution_scope_sha256: "c".repeat(64),
            parser: ValidationParserKind::AgentMageJsonV1,
            parser_version: "1.0.0".to_owned(),
            parser_sha256: "d".repeat(64),
            minimum_test_count: 1,
            focused: true,
            fail_fast: true,
            failed_test_rerun_allowed: true,
            setup_idempotent: true,
            expected_artifacts: Vec::new(),
        })
        .expect("validation template");
        ValidationTemplateRegistry::build(vec![template]).expect("validation registry")
    }

    fn write_scope() -> CodingWriteScope {
        CodingWriteScope::new(
            WorkspaceId::from_raw("workspace-coding"),
            vec![vec!["src".to_owned()], vec!["tests".to_owned()]],
        )
        .expect("write scope")
    }

    #[test]
    fn story_48_2_command_tool_selects_only_an_exact_frozen_template() {
        let command = command();
        let commands = CommandRegistry::build(vec![command.clone()]).expect("registry");
        let mut tools = ToolRegistry::new();
        register_bounded_command_runtime_tool(&mut tools, commands).expect("tool registration");
        let definition = tools
            .get_tool(
                &ToolId::from_raw(BOUNDED_COMMAND_TOOL_ID),
                BOUNDED_COMMAND_TOOL_VERSION,
            )
            .expect("command tool")
            .clone();
        assert_eq!(definition.risk_level, ToolRiskLevel::Moderate);
        assert_eq!(
            definition.declared_effects[0].operation(),
            GrantOperation::CommandExecute
        );
        assert!(definition.required_grant.single_use);

        let request = CommandRequest::new("command-attempt-0001", &command);
        let valid = call(
            &definition,
            serde_json::to_vec(&request).expect("request bytes"),
        );
        assert!(tools.validate_arguments(&valid).is_ok());

        let mut unknown = request;
        unknown.template_id = "fixture.unknown".to_owned();
        assert!(
            tools
                .validate_arguments(&call(
                    &definition,
                    serde_json::to_vec(&unknown).expect("unknown request bytes"),
                ))
                .is_err()
        );
    }

    #[test]
    fn story_48_2_command_tool_rejects_overrides_and_duplicate_registration() {
        let commands = CommandRegistry::build(vec![command()]).expect("registry");
        let mut tools = ToolRegistry::new();
        register_bounded_command_runtime_tool(&mut tools, commands.clone())
            .expect("first registration");
        assert!(register_bounded_command_runtime_tool(&mut tools, commands).is_err());
        let definition = bounded_command_tool_definition();
        let override_request = br#"{"schema_version":1,"command_attempt_id":"command-attempt-0001","template_id":"fixture.true","template_version":"1.0.0","spec_sha256":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","arguments":["--help"]}"#.to_vec();
        assert!(
            tools
                .validate_arguments(&call(&definition, override_request))
                .is_err()
        );
    }

    #[test]
    fn story_48_2_validation_tool_reuses_the_exact_registered_command() {
        let command = command();
        let commands = CommandRegistry::build(vec![command.clone()]).expect("command registry");
        let validations = validations(command);
        let template = validations.templates[0].clone();
        let mut tools = ToolRegistry::new();
        register_targeted_validation_runtime_tool(
            &mut tools,
            commands.clone(),
            validations.clone(),
        )
        .expect("validation registration");
        let definition = tools
            .get_tool(
                &ToolId::from_raw(TARGETED_VALIDATION_TOOL_ID),
                TARGETED_VALIDATION_TOOL_VERSION,
            )
            .expect("validation tool")
            .clone();
        assert_eq!(
            definition.declared_effects[0].operation(),
            GrantOperation::CommandExecute
        );

        let request = TargetedValidationRequest {
            schema_version: 1,
            validation_attempt_id: "validation-attempt-0001".to_owned(),
            validation_id: template.validation_id.clone(),
            template_sha256: template.template_sha256.clone(),
        };
        assert!(
            tools
                .validate_arguments(&call(
                    &definition,
                    serde_json::to_vec(&request).expect("validation request"),
                ))
                .is_ok()
        );

        let mut stale = request;
        stale.template_sha256 = "e".repeat(64);
        assert!(
            tools
                .validate_arguments(&call(
                    &definition,
                    serde_json::to_vec(&stale).expect("stale validation request"),
                ))
                .is_err()
        );

        let missing_commands = CommandRegistry::build(vec![
            CommandSpec::seal(
                "fixture.other",
                "1.0.0",
                "/usr/bin/false",
                "f".repeat(64),
                Vec::new(),
                CommandWorkingDirectory::EmptyScratch,
                BTreeMap::new(),
                CommandRisk::Low,
                CommandBounds::new(1_000, 1_024, 1_024, 32 * 1024 * 1024, 4, 100).expect("bounds"),
            )
            .expect("other command"),
        ])
        .expect("other command registry");
        let mut other_tools = ToolRegistry::new();
        assert!(
            register_targeted_validation_runtime_tool(
                &mut other_tools,
                missing_commands,
                validations,
            )
            .is_err()
        );
        assert!(
            other_tools
                .get_tool(
                    &ToolId::from_raw(TARGETED_VALIDATION_TOOL_ID),
                    TARGETED_VALIDATION_TOOL_VERSION,
                )
                .is_none()
        );
    }

    #[test]
    fn story_48_2_native_catalog_is_exact_local_and_mcp_independent() {
        let command = command();
        let commands = CommandRegistry::build(vec![command.clone()]).expect("command registry");
        let registry =
            native_coding_runtime_registry(write_scope(), commands, validations(command))
                .expect("native coding registry");
        let definitions = registry.list_tools();
        assert_eq!(
            definitions.len(),
            ReadOnlyToolKind::ALL.len() + ArtifactToolKind::ALL.len() + 5
        );

        let ids = definitions
            .iter()
            .map(|definition| definition.tool_id.as_str())
            .collect::<std::collections::BTreeSet<_>>();
        for required in [
            GIT_INSPECTION_TOOL_ID,
            STRUCTURED_PATCH_TOOL_ID,
            CONTROLLED_CREATE_TOOL_ID,
            BOUNDED_COMMAND_TOOL_ID,
            TARGETED_VALIDATION_TOOL_ID,
        ] {
            assert!(ids.contains(required), "missing {required}");
        }
        for kind in ReadOnlyToolKind::ALL {
            assert!(ids.contains(kind.id()), "missing {}", kind.id());
        }
        for kind in ArtifactToolKind::ALL {
            assert!(ids.contains(kind.id()), "missing {}", kind.id());
        }
        assert!(ids.iter().all(|id| !id.contains("mcp")));
        assert!(definitions.iter().all(|definition| {
            matches!(
                definition.declared_effects[0].operation(),
                GrantOperation::WorkspaceRead
                    | GrantOperation::WorkspaceWrite
                    | GrantOperation::CommandExecute
            )
        }));
        assert!(definitions.iter().all(|definition| {
            !matches!(
                definition.declared_effects[0].operation(),
                GrantOperation::NetworkAccess
                    | GrantOperation::GitClone
                    | GrantOperation::GitFetch
                    | GrantOperation::GitCommit
                    | GrantOperation::GitPush
                    | GrantOperation::Publish
                    | GrantOperation::Deploy
            )
        }));

        let model_tools = model_visible_coding_tools(&registry).expect("model-visible projection");
        assert_eq!(model_tools.len(), definitions.len());
        assert!(model_tools.iter().all(|tool| {
            tool.input_schema["$id"].as_str()
                == Some(tool.definition.input_schema.schema_id.as_str())
                && tool.input_schema["type"] == "object"
                && tool.input_schema["additionalProperties"] == false
        }));
    }
}
