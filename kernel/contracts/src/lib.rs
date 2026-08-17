#![deny(missing_docs)]
#![forbid(unsafe_code)]
//! Interface-independent contracts shared across AgentMage components.

mod agent;
mod agent_proposal;
mod agent_restart;
mod agent_state;
mod agent_verifier;
mod approval;
mod boundary;
mod claim;
mod classification;
mod common;
mod context;
mod conversation;
mod diagnostics;
mod display_link;
mod document_control;
mod evidence;
mod executive;
mod frontier;
mod frontier_import;
mod grant;
mod handoff;
mod ids;
mod meeting;
mod model;
mod model_discovery;
mod network;
mod operation;
mod path;
mod platform;
mod platform_path;
mod prompt;
mod reasoning;
mod runtime_event;
mod serialization;
mod task;
mod tool;
mod transaction;
mod workspace_snapshot;

pub use agent::{
    AgentFinalResponse, AgentFinalState, AgentProgressEvent, AgentProgressKind, AgentStatusKind,
    AgentStatusResponse, UserMessageDisposition, UserMessageIntent,
};
pub use agent_proposal::AgentProposal;
pub use agent_restart::AgentRestartSnapshot;
pub use agent_state::{AgentStateKind, AgentStateTransition};
pub use agent_verifier::{
    PostconditionResult, VerifierCandidate, VerifierDisposition, VerifierSource,
};
pub use approval::ApprovalRequest;
pub use boundary::{
    BoundaryFailure, BoundaryKind, BoundaryOutcomeKind, CancellationReason, CancellationSignal,
};
pub use claim::{
    ClaimBoundFinalResponse, ClaimEvidence, ClaimEvidenceRole, DerivedClaimProvenance,
    DeterministicMethodIdentity, InferenceRuntimeProvenance, InferredClaimProvenance,
    MaterialClaim, MaterialClaimEvidenceAssignment, MaterialClaimEvidenceState,
    MaterialClaimEvidenceStateKind, MaterialClaimKind, ObservedClaimProvenance,
    UnknownBlockedClaimProvenance, UnknownBlockedReason, VerifiedMaterialClaim,
};
pub use classification::{
    ActionRisk, ActionRiskAssessment, AdvisoryClassifierDisposition, AdvisoryClassifierResult,
    AdvisoryClassifierStatus, AutonomyLevel, BudgetState, ClassificationBoundary, CredentialClass,
    DataSensitivityAssessment, DeterministicPolicyFacts, DisclosureClass, ExactAuthorityState,
    ModelCapabilityAssessment, ModelCapabilityRole, ModelCapabilityStatus, NetworkRequirement,
    PathScopeState, PolicyDestinationClass, PolicySourceClass, ReclassificationContentKind,
    ReclassificationRequest, RepositoryState, StaticPolicyCheck, StaticPolicyCheckKind,
    StaticPolicyCheckState,
};
pub use common::{
    CONTRACT_SCHEMA_VERSION, ContractError, ContractPayload, ErrorCategory, RetryDisposition,
    SchemaReference, ValidationIssue, ValidationSeverity,
};
pub use context::{
    CheckedContextSummary, CheckedSummaryState, CheckpointFileIdentity, ComposedContextPacket,
    ContextAdmission, ContextItemAccounting, ContextItemCandidate, ContextItemKind,
    ContextOmissionReason, ContextSensitivity, ResumeDriftDecision, ResumeDriftDimension,
    SessionCheckpoint,
};
pub use conversation::{
    ConversationAttachmentReference, ConversationCompactionRecord, ConversationRecord,
    ConversationRetention, ConversationRetentionKind, ConversationStatus, ConversationTurn,
    ConversationTurnRole,
};
pub use diagnostics::{
    DiagnosticComponent, DiagnosticItem, DiagnosticObservation, DiagnosticState, DoctorReport,
};
pub use display_link::{
    DisplayFileLink, DisplayLinkError, DisplayLinkErrorKind, MAX_DISPLAY_FILE_URI_BYTES,
};
pub use document_control::{
    DocumentActionApproval, DocumentActionKind, DocumentActionPreview, DocumentActionReview,
    DocumentAttachmentReviewState, DocumentControlFinding, DocumentControlFindingKind,
    DocumentLifecycleState, DocumentNamedParty, DocumentRegister, DocumentRegisterAttachment,
    DocumentRegisterEntry, DocumentRegisterKind, DocumentRegisterStatement, DocumentStatementClass,
    DocumentWorkflowKind, DocumentWorkflowReport,
};
pub use evidence::{EvidenceKind, EvidenceReference, Receipt};
pub use executive::{
    ExecutiveCorrespondenceDraft, ExecutiveCorrespondenceIssue, ExecutiveCorrespondenceIssueKind,
    ExecutiveCorrespondenceKind, ExecutiveCorrespondenceReview, ExecutiveDraftClaim,
    ExecutiveDueWindow, ExecutiveEvidenceState, ExecutiveField, ExecutiveLocalMessage,
    ExecutiveMessageTriageClass, ExecutiveMessageTriageEntry, ExecutivePortfolioSnapshot,
    ExecutivePriorityComponent, ExecutivePriorityComponentKind, ExecutivePriorityEntry,
    ExecutivePriorityRanking, ExecutivePrivacyClass, ExecutivePrivacyDecision,
    ExecutivePrivacyOperation, ExecutivePrivacyRequest, ExecutiveRecord, ExecutiveRecordKind,
    ExecutiveRecordStatus, ExecutiveSourceReference, ExecutiveSourceStore, ExecutiveTracker,
    ExecutiveTrackerEntry, ExecutiveTrackerKind, ExecutiveView, ExecutiveViewItem,
    ExecutiveViewKind,
};
pub use frontier::{
    FrontierAcceptanceState, FrontierClarificationClass, FrontierRecommendationReceipt,
    FrontierRecommendationTrigger, FrontierTaskTier, FrontierTierDecision, FrontierTierEvidence,
};
pub use frontier_import::{
    FrontierDisagreement, FrontierImportDisposition, FrontierImportedClaimState,
    FrontierLocalFlowRequirements, FrontierReturnArtifactDeclaration, FrontierReturnArtifactKind,
    FrontierReturnCitationClaim, FrontierReturnInput, FrontierReturnKind, FrontierReturnManifest,
    FrontierReturnedStep, FrontierReturnedStepKind, FrontierReturnedStepOutcome,
    FrontierRoundTripReceipt,
};
pub use grant::{
    CapabilityGrant, GrantClass, GrantPreimage, GrantSideEffect, GrantStatus, GrantTarget,
    GrantTargetError,
};
pub use handoff::{
    HandoffDestinationClass, HandoffDisclosureEntry, HandoffDraft, HandoffEntryDisposition,
    HandoffEntryKind, HandoffPacketManifest, HandoffProhibitedAction, HandoffReview,
    HandoffSensitivity, LocalHandoffOutcome, LocalHandoffReceipt, RenderedHandoff,
};
pub use ids::{
    ActionId, ActorId, AdapterInstanceId, ApprovalId, AuthorityTransactionId, CancellationId,
    ContextPacketId, ContextSummaryId, ConversationCompactionId, ConversationId,
    ConversationTurnId, CorrelationId, ErrorId, EvidenceId, GrantId, GrantNonce, ModelAdapterId,
    ModelCodecId, ModelManifestId, ModelMessageId, ModelProfileId, ModelRunId, ModelStreamId,
    OperationAttemptId, PlanId, PlanStepId, PolicyId, PostconditionId, PromptId, ProposalId,
    ReceiptId, RepositorySnapshotId, RuntimeArtifactId, RuntimeEventId, RuntimeOperationId,
    RuntimeRunId, RuntimeTurnId, SchemaId, SessionCheckpointId, SessionId, TaskId, ToolCallId,
    ToolCatalogId, ToolId, VerifierId, VerifierRecordId, WorkPacketId, WorkspaceAuthorizationId,
    WorkspaceId,
};
pub use meeting::{
    MeetingAttendanceState, MeetingAttendee, MeetingCloseout, MeetingContinuityItem,
    MeetingContinuityRecord, MeetingContinuityState, MeetingContinuityUpdate, MeetingDraftKind,
    MeetingFieldState, MeetingInvitationState, MeetingMinutes, MeetingMinutesItem,
    MeetingMinutesItemKind, MeetingPlanDraft, MeetingPlanItem, MeetingProjectionPrecondition,
    MeetingTextSourceKind, MeetingTranscriptCleanup, MeetingTranscriptSegment,
    MeetingUnclearMarker,
};
pub use model::{
    ClosedModelProposal, ContextBudget, DecodingProfile, EncodedModelContext, ExactModelProfile,
    FamilyCodecIdentity, HardwareEnvelope, LocalModelRuntime, ModelArtifact,
    ModelCancellationProbe, ModelCapability, ModelCapabilityState, ModelClientSchemas,
    ModelContextPacket, ModelFamilyCodec, ModelHealth, ModelHealthState, ModelLifecycleState,
    ModelLoadReceipt, ModelManifestObservation, ModelMessage, ModelMessageRole, ModelModality,
    ModelProposalKind, ModelProposalWireCandidate, ModelResourceReport, ModelRole, ModelRunRequest,
    ModelRunResult, ModelRunTerminalState, ModelRuntimeFailure, ModelRuntimeIdentity,
    ModelRuntimeKind, ModelStreamSink, ModelToolCallCandidate, ModelToolCallWireCandidate,
    ModelTransformation, ModelUnloadReceipt, RuntimeIsolationObservation, StreamedModelFragment,
    TokenCountResult,
};
pub use model_discovery::{
    ModelActivationState, ModelCompatibilityState, ModelPickerCapability, ModelPickerDisposition,
    ModelPickerEntry, ModelPickerSnapshot, ModelSelectionRevalidation, ModelSupportState,
};
pub use network::{
    CloudSynchronizationMarker, LocalEndpointIdentity, LocalTransport, NetworkComponent,
    NetworkDestinationClass, NetworkEndpointError, NetworkObservation, StorageFilesystemClass,
    StrictLocalStorageObservation, classify_ip_destination,
};
pub use operation::{
    AuthorityClass, GrantOperation, OPERATION_TAXONOMY_VERSION, OperationBinding,
    OperationTaxonomyError,
};
pub use path::{
    MAX_WORKSPACE_PATH_COMPONENT_BYTES, MAX_WORKSPACE_PATH_COMPONENTS, WorkspacePath,
    WorkspacePathComponent, WorkspacePathError, WorkspacePathErrorKind, WorkspaceScopePath,
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
pub use reasoning::{
    AssumptionRecord, AssumptionRisk, AssumptionStatus, ClaimAssertion, ClaimStatus,
    ClarificationImpact, ClarificationQuestion, ClarificationState, ContradictionRecord,
    HypothesisRecord, HypothesisStatus, IndependentVerificationRequest,
    IndependentVerificationResult, ProblemFact, ProblemFrame, VerificationDisposition,
};
pub use runtime_event::{
    RuntimeEvent, RuntimeEventKind, RuntimeEventPersistenceClass, RuntimeEventRetention,
    RuntimeEventRetentionKind, RuntimePayloadReference, RuntimePermissionDisposition,
};
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
pub use transaction::{AuthorityTransactionRecord, AuthorityTransactionState};
pub use workspace_snapshot::{SnapshotEntry, SnapshotEntryKind, WorkspaceSnapshot};

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
