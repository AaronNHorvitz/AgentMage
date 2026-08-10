#![deny(missing_docs)]
#![forbid(unsafe_code)]
//! Interface-independent contracts shared across AgentMage components.

mod common;
mod evidence;
mod ids;
mod serialization;
mod task;
mod tool;

pub use common::{
    CONTRACT_SCHEMA_VERSION, ContractError, ContractPayload, ErrorCategory, RetryDisposition,
    SchemaReference, ValidationIssue, ValidationSeverity,
};
pub use evidence::{EvidenceKind, EvidenceReference, Receipt};
pub use ids::{
    ActionId, CorrelationId, ErrorId, EvidenceId, PlanId, PlanStepId, ReceiptId, SchemaId,
    SessionId, TaskId, ToolCallId, ToolId, WorkPacketId,
};
pub use serialization::{
    ContractResult, MAX_CONTRACT_JSON_BYTES, VersionedContract, from_json, to_canonical_json,
};
pub use task::{
    Action, ActionKind, ActionState, BudgetLimit, BudgetResource, CompletionEvidence,
    DataSensitivity, Plan, PlanState, PlanStep, PlanStepState, RollbackPlan, StopCondition,
    StopConditionKind, Task, TaskStatus, WorkPacket, WorkPacketState,
};
pub use tool::{OperationOutcome, ToolCall, ToolDefinition, ToolResult};

/// Stable component identity used by diagnostics and build verification.
pub const COMPONENT_ID: &str = "kernel-contracts";

#[cfg(test)]
mod tests {
    use super::COMPONENT_ID;

    #[test]
    fn component_identity_is_stable() {
        assert_eq!(COMPONENT_ID, "kernel-contracts");
    }
}
