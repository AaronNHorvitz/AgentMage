//! Authority-free frontier-return and local round-trip contracts.

use crate::{FrontierTaskTier, OperationBinding};

/// Closed top-level shape of a user-provided frontier result.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum FrontierReturnKind {
    /// A proposed decision or recommendation.
    Decision,
    /// One or more proposed artifacts for local review.
    Artifact,
}

/// Closed artifact class declared by an untrusted return manifest.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum FrontierReturnArtifactKind {
    /// Explanatory plain text.
    Text,
    /// A proposed source or document patch.
    Patch,
    /// A complete proposed file.
    File,
    /// A proposed plan with no execution authority.
    Plan,
    /// A proposed material-claim set.
    ClaimSet,
    /// A proposed decision record.
    Decision,
}

/// Closed semantic class for one returned proposal step.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum FrontierReturnedStepKind {
    /// A proposed decision requiring local review.
    Decision,
    /// A proposed claim requiring local evidence assignment.
    Claim,
    /// A proposed file change requiring the normal controlled-write flow.
    FileProposal,
    /// A proposed command requiring the normal command flow.
    CommandProposal,
    /// A proposed tool call requiring registration, classification, and a fresh grant.
    ToolProposal,
    /// A link retained as inert text pending local resolution.
    LinkReference,
    /// A claimed test result requiring trusted local validation.
    TestResult,
}

/// One exact input identity represented to the external consultation.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FrontierReturnInput {
    /// Stable input identity.
    pub input_id: String,
    /// Lowercase SHA-256 digest of the exact input bytes.
    pub input_sha256: String,
}

/// Metadata for one separately supplied untrusted artifact.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FrontierReturnArtifactDeclaration {
    /// Stable artifact identity.
    pub artifact_id: String,
    /// Closed artifact class.
    pub kind: FrontierReturnArtifactKind,
    /// Exact lowercase media type expected by the manifest.
    pub media_type: String,
    /// Display-only portable relative path or logical label.
    pub display_path: String,
    /// Declared byte length.
    pub byte_length: u64,
    /// Lowercase SHA-256 digest of the exact separately supplied bytes.
    pub content_sha256: String,
}

/// One untrusted citation claim that must be resolved again from a local source.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FrontierReturnCitationClaim {
    /// Stable citation identity used by returned steps.
    pub citation_id: String,
    /// Claimed source identity without access authority.
    pub source_id: String,
    /// Claimed object identity within the source.
    pub object_id: String,
    /// Optional claimed fragment or range.
    #[serde(deserialize_with = "crate::serialization::deserialize_required_option")]
    pub fragment: Option<String>,
    /// Claimed lowercase SHA-256 source digest.
    pub claimed_content_sha256: String,
}

/// One proposal-only returned step.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FrontierReturnedStep {
    /// Stable step identity.
    pub step_id: String,
    /// Closed proposal class.
    pub kind: FrontierReturnedStepKind,
    /// Content-minimized rationale.
    pub rationale: String,
    /// Ordered artifact identities consumed by this step.
    pub artifact_ids: Vec<String>,
    /// Ordered citation identities claimed by this step.
    pub citation_ids: Vec<String>,
    /// Ordered exact acceptance checks proposed for this step.
    pub acceptance_checks: Vec<String>,
    /// Proposed canonical operation, if any; this field carries no grant.
    #[serde(deserialize_with = "crate::serialization::deserialize_required_option")]
    pub proposed_operation: Option<OperationBinding>,
    /// Ordered operations for which fresh local approval would be required.
    pub approval_requirements: Vec<OperationBinding>,
}

/// Closed user-provided return manifest. Deserialization never executes represented content.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FrontierReturnManifest {
    /// Contract schema version.
    pub schema_version: u16,
    /// Stable import identity.
    pub import_id: String,
    /// Exact reviewed request-packet digest to which this result responds.
    pub request_packet_sha256: String,
    /// Workspace-state digest represented by the original request.
    pub base_workspace_state_sha256: String,
    /// Model-state digest represented by the original request.
    pub base_model_state_sha256: String,
    /// Policy digest represented by the original request.
    pub base_policy_sha256: String,
    /// Tier represented by this manual consultation result.
    pub tier: FrontierTaskTier,
    /// Top-level return shape.
    pub result_kind: FrontierReturnKind,
    /// Content-minimized result rationale.
    pub rationale: String,
    /// Ordered request-input identities.
    pub inputs: Vec<FrontierReturnInput>,
    /// Ordered separately supplied artifact declarations.
    pub artifacts: Vec<FrontierReturnArtifactDeclaration>,
    /// Ordered untrusted citation claims requiring fresh local resolution.
    pub citations: Vec<FrontierReturnCitationClaim>,
    /// Ordered proposal-only steps.
    pub steps: Vec<FrontierReturnedStep>,
    /// Ordered exact acceptance checks for the returned proposal.
    pub acceptance_checks: Vec<String>,
    /// Ordered proposal-level operations requiring fresh local approval.
    pub approval_requirements: Vec<OperationBinding>,
    /// Ordered remaining work that the result does not claim to complete.
    pub remaining_steps: Vec<String>,
    /// Fixed true marker for the complete imported result.
    pub external_content_untrusted: bool,
    /// Fixed false marker: a return manifest cannot grant authority.
    pub authority_granted: bool,
    /// Fixed false marker: import cannot establish completion.
    pub completion_credit: bool,
    /// Fixed false marker: local import needs no outbound network.
    pub outbound_network_required: bool,
    /// Digest of the canonical manifest with this field empty.
    pub manifest_sha256: String,
}

/// Closed local disposition for one imported proposal step.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum FrontierImportDisposition {
    /// The step is structurally invalid or violates a hard local boundary.
    Rejected,
    /// The step is retained without trust pending missing or conflicting evidence.
    Quarantined,
    /// The step may enter a normal local proposal flow but carries no authority.
    ProposalEligible,
}

/// Evidence state allowed for a returned claim before a normal local proof flow.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum FrontierImportedClaimState {
    /// The imported interpretation has current locally resolved supporting citations.
    Inferred,
    /// Current local evidence cannot support even an inferred disposition.
    UnknownBlocked,
}

/// Local authority requirements attached to a proposal-eligible returned step.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FrontierLocalFlowRequirements {
    /// Fresh local task classification is always required.
    pub fresh_task_classification_required: bool,
    /// Fresh local grant evaluation is required for any operation.
    pub fresh_grant_required: bool,
    /// Registered-tool validation is required for a proposed tool call.
    pub registered_tool_validation_required: bool,
    /// Exact-preimage write preview is required for a proposed file change.
    pub exact_write_preview_required: bool,
    /// Trusted local validation is required for claimed test or completion results.
    pub trusted_validation_required: bool,
    /// Local evidence-state assignment is required for returned claims.
    pub evidence_assignment_required: bool,
    /// Explicit user approval is required before any represented state change.
    pub user_approval_required: bool,
}

/// Content-free local result for one imported proposal step.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FrontierReturnedStepOutcome {
    /// Exact returned step identity.
    pub step_id: String,
    /// Local disposition.
    pub disposition: FrontierImportDisposition,
    /// Optional evidence state for a returned claim.
    #[serde(deserialize_with = "crate::serialization::deserialize_required_option")]
    pub claim_state: Option<FrontierImportedClaimState>,
    /// Ordered stable local reason codes.
    pub reason_codes: Vec<String>,
    /// Normal local flow requirements that remain after import.
    pub local_requirements: FrontierLocalFlowRequirements,
    /// Fixed false marker: this outcome cannot issue a grant.
    pub grant_issued: bool,
    /// Fixed false marker: this outcome cannot invoke a tool.
    pub tool_called: bool,
    /// Fixed false marker: this outcome cannot write a file.
    pub file_written: bool,
    /// Fixed false marker: this outcome cannot establish completion.
    pub completion_credited: bool,
}

/// One visible disagreement between imported content and current local evidence.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FrontierDisagreement {
    /// Stable disagreement identity.
    pub disagreement_id: String,
    /// Digest of the exact imported assertion or proposal.
    pub imported_sha256: String,
    /// Optional digest of conflicting current local evidence.
    #[serde(deserialize_with = "crate::serialization::deserialize_required_option")]
    pub local_evidence_sha256: Option<String>,
    /// Stable content-free disagreement reason.
    pub reason_code: String,
    /// Fixed true marker requiring evidence or user direction.
    pub unresolved: bool,
}

/// Content-free receipt for one complete local import and revalidation round trip.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FrontierRoundTripReceipt {
    /// Contract schema version.
    pub schema_version: u16,
    /// Stable receipt identity.
    pub receipt_id: String,
    /// Exact reviewed request-packet digest.
    pub request_packet_sha256: String,
    /// Exact imported manifest digest.
    pub return_manifest_sha256: String,
    /// Digest of current local state and all step outcomes.
    pub local_revalidation_sha256: String,
    /// Ordered content-free step outcomes.
    pub step_outcomes: Vec<FrontierReturnedStepOutcome>,
    /// Ordered preserved disagreements.
    pub disagreements: Vec<FrontierDisagreement>,
    /// Optional stable reason to recommend another manual consultation.
    #[serde(deserialize_with = "crate::serialization::deserialize_required_option")]
    pub re_escalation_reason: Option<String>,
    /// Ordered content-free capability feedback codes.
    pub capability_feedback: Vec<String>,
    /// Fixed zero marker: import applies no effect.
    pub applied_effect_count: u32,
    /// Fixed zero marker: import cannot duplicate an effect.
    pub duplicate_effect_count: u32,
    /// Fixed false marker: import requires no outbound network.
    pub outbound_network_used: bool,
    /// Digest of the canonical receipt with this field empty.
    pub receipt_sha256: String,
}
