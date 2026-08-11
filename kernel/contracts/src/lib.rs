#![deny(missing_docs)]
#![forbid(unsafe_code)]
//! Interface-independent contracts shared across AgentMage components.

mod approval;
mod boundary;
mod common;
mod display_link;
mod evidence;
mod grant;
mod ids;
mod path;
mod platform;
mod platform_path;
mod prompt;
mod serialization;
mod task;
mod tool;

pub use approval::ApprovalRequest;
pub use boundary::{
    BoundaryFailure, BoundaryKind, BoundaryOutcomeKind, CancellationReason, CancellationSignal,
};
pub use common::{
    CONTRACT_SCHEMA_VERSION, ContractError, ContractPayload, ErrorCategory, RetryDisposition,
    SchemaReference, ValidationIssue, ValidationSeverity,
};
pub use display_link::{
    DisplayFileLink, DisplayLinkError, DisplayLinkErrorKind, MAX_DISPLAY_FILE_URI_BYTES,
};
pub use evidence::{EvidenceKind, EvidenceReference, Receipt};
pub use grant::{
    CapabilityGrant, GrantClass, GrantOperation, GrantPreimage, GrantSideEffect, GrantStatus,
    GrantTarget,
};
pub use ids::{
    ActionId, ActorId, AdapterInstanceId, CancellationId, CorrelationId, ErrorId, EvidenceId,
    GrantId, GrantNonce, PlanId, PlanStepId, PromptId, ReceiptId, SchemaId, SessionId, TaskId,
    ToolCallId, ToolId, WorkPacketId, WorkspaceAuthorizationId, WorkspaceId,
};
pub use path::{
    MAX_WORKSPACE_PATH_COMPONENT_BYTES, MAX_WORKSPACE_PATH_COMPONENTS, WorkspacePath,
    WorkspacePathComponent, WorkspacePathError, WorkspacePathErrorKind,
};
pub use platform::{
    PLATFORM_ADAPTER_API_VERSION, PlatformAdapter, PlatformArchitecture, PlatformCapability,
    PlatformCapabilityObservation, PlatformCapabilityStatus, PlatformFamily,
    PlatformManifestIdentity, PlatformRuntimeIdentity, PlatformStartupError,
    PlatformStartupErrorKind, REQUIRED_PLATFORM_CAPABILITIES,
};
pub use platform_path::{
    AuthorizedWorkspaceHandle, FilePreimage, HeldWorkspaceObject, PathAdapterError,
    PathAdapterErrorKind, PathPlatform, PathResolutionIntent, PlatformPathAdapter,
    WorkspaceObjectIdentity, WorkspaceObjectKind,
};
pub use prompt::{Prompt, PromptMessage, PromptRole};
pub use serialization::{
    ContractResult, MAX_CONTRACT_JSON_BYTES, VersionedContract, from_json, to_canonical_json,
};
pub use task::{
    Action, ActionKind, ActionState, BudgetLimit, BudgetResource, CompletionEvidence,
    DataSensitivity, Plan, PlanState, PlanStep, PlanStepState, RollbackPlan, StopCondition,
    StopConditionKind, Task, TaskStatus, WorkPacket, WorkPacketState,
};
pub use tool::{
    OperationOutcome, RequiredGrantTemplate, StateChange, ToolCall, ToolDefinition, ToolResult,
    ToolRiskLevel,
};

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
