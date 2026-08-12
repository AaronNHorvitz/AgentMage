#![deny(missing_docs)]
#![forbid(unsafe_code)]
//! Declarative contracts for bounded read-only workspace capabilities.

use agentmage_kernel_contracts::{
    CONTRACT_SCHEMA_VERSION, GrantOperation, OperationBinding, RequiredGrantTemplate, SchemaId,
    SchemaReference, ToolDefinition, ToolId, ToolRiskLevel,
};

/// Stable component identity used by diagnostics and build verification.
pub const COMPONENT_ID: &str = "capability-read-only";

/// Stable identity of the one-file workspace read contract.
pub const WORKSPACE_FILE_READ_TOOL_ID: &str = "agentmage.workspace.read-file";

/// Immutable version of the one-file workspace read contract.
pub const WORKSPACE_FILE_READ_TOOL_VERSION: &str = "1.0.0";

const INPUT_SCHEMA_SHA256: &str =
    "a492b4044138d545ccb2c646de8841158d190875f3dd31c26d6c67e50fbe4ac3";
const OUTPUT_SCHEMA_SHA256: &str =
    "5525279aae7ba5d90dfea8880ecdcaf5d2d1615c37bd9a5e936a84259ba272a2";

/// Returns the identity of the contracts implemented by this capability pack.
#[must_use]
pub const fn contract_component_id() -> &'static str {
    agentmage_kernel_contracts::COMPONENT_ID
}

/// Returns the exact declarative definition for one bounded UTF-8 file read.
///
/// The definition contains no executor and cannot authorize a call. The host
/// registers it with the kernel, which still requires one exact consumed grant
/// before the Linux effect driver can run.
#[must_use]
pub fn workspace_file_read_definition() -> ToolDefinition {
    let operation = OperationBinding::new(GrantOperation::WorkspaceRead);
    ToolDefinition {
        schema_version: CONTRACT_SCHEMA_VERSION,
        tool_id: ToolId::from_raw(WORKSPACE_FILE_READ_TOOL_ID),
        tool_version: WORKSPACE_FILE_READ_TOOL_VERSION.to_owned(),
        display_name: "Read workspace file".to_owned(),
        description: "Reads one exact approved workspace file as bounded UTF-8 text".to_owned(),
        input_schema: SchemaReference {
            schema_id: SchemaId::from_raw("agentmage.workspace.read-file.input"),
            schema_version: 1,
            schema_sha256: INPUT_SCHEMA_SHA256.to_owned(),
        },
        output_schema: SchemaReference {
            schema_id: SchemaId::from_raw("agentmage.workspace.read-file.output"),
            schema_version: 1,
            schema_sha256: OUTPUT_SCHEMA_SHA256.to_owned(),
        },
        risk_level: ToolRiskLevel::Low,
        declared_effects: vec![operation],
        required_grant: RequiredGrantTemplate {
            operation,
            target_scope: "one-exact-held-workspace-file".to_owned(),
            single_use: true,
        },
        timeout_ms: 15_000,
    }
}

#[cfg(test)]
mod tests {
    use agentmage_kernel_contracts::{GrantOperation, ToolRiskLevel};

    use super::{
        COMPONENT_ID, WORKSPACE_FILE_READ_TOOL_ID, WORKSPACE_FILE_READ_TOOL_VERSION,
        contract_component_id, workspace_file_read_definition,
    };

    #[test]
    fn capability_depends_only_on_contracts() {
        assert_eq!(COMPONENT_ID, "capability-read-only");
        assert_eq!(contract_component_id(), "kernel-contracts");
    }

    #[test]
    fn one_file_read_contract_is_exact_and_non_executable() {
        let definition = workspace_file_read_definition();
        assert_eq!(definition.tool_id.as_str(), WORKSPACE_FILE_READ_TOOL_ID);
        assert_eq!(definition.tool_version, WORKSPACE_FILE_READ_TOOL_VERSION);
        assert_eq!(definition.risk_level, ToolRiskLevel::Low);
        assert_eq!(definition.declared_effects.len(), 1);
        assert_eq!(
            definition.declared_effects[0].operation(),
            GrantOperation::WorkspaceRead
        );
        assert!(definition.required_grant.single_use);
        assert_eq!(definition.timeout_ms, 15_000);
    }
}
