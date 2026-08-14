#![deny(missing_docs)]
#![forbid(unsafe_code)]
//! Declarative contracts for bounded read-only workspace capabilities.

use agentmage_kernel_contracts::ToolDefinition;

mod catalog;
mod git;
mod protocol;
mod worker;

pub use catalog::{
    READ_ONLY_INPUT_SCHEMA_ID, READ_ONLY_INPUT_SCHEMA_JSON, READ_ONLY_OUTPUT_SCHEMA_ID,
    READ_ONLY_OUTPUT_SCHEMA_JSON, READ_ONLY_TOOL_VERSION, ReadOnlyToolKind,
    read_only_tool_definition, read_only_tool_definitions, read_only_tool_kind,
};
pub use git::{
    GitCommandPlan, GitInspectionError, GitInspectionOperation, GitInspectionOutcome,
    GitInspectionRequest, GitInspectionResult, GitRecord, parse_git_inspection,
    plan_git_inspection,
};
pub use protocol::{
    MAX_READ_ONLY_CALL_DEPTH, MAX_READ_ONLY_DEPTH, MAX_READ_ONLY_FILES, MAX_READ_ONLY_INPUT_BYTES,
    MAX_READ_ONLY_MATCHES, MAX_READ_ONLY_OUTPUT_BYTES, NeverCancelled, ReadOnlyCancellation,
    ReadOnlyEncoding, ReadOnlyItem, ReadOnlyLimits, ReadOnlyOutcome, ReadOnlyRequest,
    ReadOnlyRequestError, ReadOnlyResult, SnapshotEntry, SnapshotEntryKind, WorkspaceSnapshot,
    execute_read_only, validate_read_only_request,
};
pub use worker::{ReadOnlyWorkerError, execute_worker_payload};

/// Stable component identity used by diagnostics and build verification.
pub const COMPONENT_ID: &str = "capability-read-only";

/// Stable identity of the one-file workspace read contract.
pub const WORKSPACE_FILE_READ_TOOL_ID: &str = "agentmage.workspace.read-file";

/// Immutable version of the one-file workspace read contract.
pub const WORKSPACE_FILE_READ_TOOL_VERSION: &str = READ_ONLY_TOOL_VERSION;

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
    read_only_tool_definition(ReadOnlyToolKind::ReadText)
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
