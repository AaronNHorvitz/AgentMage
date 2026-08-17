//! Native coding-tool definitions composed from existing kernel contracts.

use std::fmt::Write;

use agentmage_kernel_contracts::{
    CONTRACT_SCHEMA_VERSION, GrantOperation, OperationBinding, RequiredGrantTemplate, SchemaId,
    SchemaReference, ToolDefinition, ToolId, ToolRiskLevel, ValidationIssue, ValidationSeverity,
};
use agentmage_kernel_engine::{
    command_runner::{CommandRegistry, CommandRequest, prepare_command},
    tooling::{Tool, ToolRegistry},
};
use sha2::{Digest, Sha256};

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

/// Stable failure while composing a native coding-tool catalog.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CodingToolCatalogError {
    /// One exact definition could not enter the common registry.
    RegistrationDenied,
}

impl CodingToolCatalogError {
    /// Returns one stable content-free diagnostic code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::RegistrationDenied => "runtime.coding-tool.registration-denied",
        }
    }
}

struct RegisteredCommandTool {
    definition: ToolDefinition,
    commands: CommandRegistry,
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

    use agentmage_kernel_contracts::{
        ActionId, ContractPayload, CorrelationId, GrantOperation, ToolCall, ToolCallId, ToolId,
        ToolRiskLevel,
    };
    use agentmage_kernel_engine::command_runner::{
        CommandBounds, CommandRequest, CommandRisk, CommandSpec, CommandWorkingDirectory,
    };

    use super::*;

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
}
