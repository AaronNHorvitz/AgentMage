//! Tool definition, call, and result contracts.

use crate::{
    ActionId, ContractError, ContractPayload, CorrelationId, EvidenceReference, SchemaReference,
    ToolCallId, ToolId, ValidationIssue,
};

/// Terminal outcome shared by tool results and receipts.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OperationOutcome {
    /// The attempted operation satisfied its declared postconditions.
    Succeeded,
    /// Current policy denied the attempt before execution.
    Denied,
    /// The attempt failed with a typed error.
    Failed,
    /// The attempt was cancelled.
    Cancelled,
    /// The attempt exceeded its deadline.
    TimedOut,
    /// Completion or side effects cannot be established safely.
    Uncertain,
}

/// Versioned description of one tool available for registration review.
///
/// A tool definition describes shape and expected effects. It cannot grant its own use.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ToolDefinition {
    /// Contract schema version.
    pub schema_version: u16,
    /// Stable tool identity.
    pub tool_id: ToolId,
    /// Immutable tool-contract version.
    pub tool_version: String,
    /// Short display name.
    pub display_name: String,
    /// Bounded human-readable behavior description.
    pub description: String,
    /// Closed input schema identity.
    pub input_schema: SchemaReference,
    /// Closed output schema identity.
    pub output_schema: SchemaReference,
    /// Declared effect classes used for policy and review.
    pub declared_effects: Vec<String>,
}

/// Versioned request to invoke one exact tool contract.
///
/// A tool call is a request, not authority. The dispatcher must reject it unless a
/// separate exact grant is current and valid immediately before execution.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ToolCall {
    /// Contract schema version.
    pub schema_version: u16,
    /// Stable identity for this attempt.
    pub tool_call_id: ToolCallId,
    /// Correlation identity shared with result and receipt records.
    pub correlation_id: CorrelationId,
    /// Action that proposed the call.
    pub action_id: ActionId,
    /// Exact requested tool identity.
    pub tool_id: ToolId,
    /// Exact requested tool-contract version.
    pub tool_version: String,
    /// Schema-bound opaque argument payload.
    pub arguments: ContractPayload,
}

/// Versioned terminal result of one tool-call attempt.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ToolResult {
    /// Contract schema version.
    pub schema_version: u16,
    /// Tool-call identity this result closes.
    pub tool_call_id: ToolCallId,
    /// Correlation identity from the request.
    pub correlation_id: CorrelationId,
    /// Exact terminal outcome.
    pub outcome: OperationOutcome,
    /// Optional schema-bound output payload.
    pub output: Option<ContractPayload>,
    /// Validation findings retained without partial success.
    pub validation_issues: Vec<ValidationIssue>,
    /// Evidence records produced by the attempt.
    pub evidence: Vec<EvidenceReference>,
    /// Optional typed terminal error.
    pub error: Option<ContractError>,
}

#[cfg(test)]
mod tests {
    use super::{OperationOutcome, ToolCall};
    use crate::{ActionId, ContractPayload, CorrelationId, ToolCallId, ToolId};

    #[test]
    fn call_binds_exact_tool_version_and_correlation_identity() {
        let call = ToolCall {
            schema_version: 1,
            tool_call_id: ToolCallId::from_raw("call-0001"),
            correlation_id: CorrelationId::from_raw("correlation-0001"),
            action_id: ActionId::from_raw("action-0001"),
            tool_id: ToolId::from_raw("fixture.read"),
            tool_version: "1.0.0".to_owned(),
            arguments: ContractPayload {
                media_type: "application/json".to_owned(),
                bytes: b"{}".to_vec(),
                sha256: "0".repeat(64),
            },
        };
        assert_eq!(call.tool_id.as_str(), "fixture.read");
        assert_eq!(call.tool_version, "1.0.0");
        assert_eq!(OperationOutcome::Denied, OperationOutcome::Denied);
    }
}
