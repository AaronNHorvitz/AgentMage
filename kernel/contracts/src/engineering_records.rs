//! Canonical schema-bound records for the complete Engineering Runtime.

use crate::{RuntimeArtifactCleanupState, RuntimeArtifactKind, RuntimeArtifactLifecycleState};

/// One inclusive-exclusive byte range in an authoritative artifact.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CanonicalByteRange {
    /// First included byte offset.
    pub start_byte: u64,
    /// First excluded byte offset.
    pub end_byte_exclusive: u64,
}

/// Immutable reference to exact artifact bytes.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CanonicalArtifactReference {
    /// Stable artifact identity.
    pub artifact_id: String,
    /// SHA-256 of the exact bytes.
    pub sha256: String,
    /// Exact byte length.
    pub byte_length: u64,
}

/// Origin of one canonical artifact.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CanonicalArtifactOrigin {
    /// Direct composer paste.
    Paste,
    /// Reference supplied in the request.
    RequestReference,
    /// Local file selected under authority.
    File,
    /// URI selected under authority.
    Uri,
    /// Directory selected under authority.
    Directory,
    /// Archive selected under authority.
    Archive,
    /// Output emitted by a tool.
    ToolOutput,
}

/// Data classification attached to an artifact.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CanonicalClassification {
    /// Public data.
    Public,
    /// Internal data.
    Internal,
    /// Confidential data.
    Confidential,
    /// Restricted data.
    Restricted,
}

/// Terminal capture state for one source.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CanonicalCaptureState {
    /// Exact bytes were captured.
    Captured,
    /// Source was unavailable.
    Unavailable,
    /// Source format is unsupported.
    Unsupported,
    /// Policy denied capture.
    Denied,
    /// Capture failed.
    Failed,
}

/// Closed provenance envelope for one source artifact.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CanonicalArtifactEnvelope {
    /// Contract schema version.
    pub schema_version: u16,
    /// Stable artifact identity.
    pub artifact_id: String,
    /// Request that introduced the artifact.
    pub request_id: String,
    /// Authority used to access the source.
    pub authority_id: String,
    /// Source origin.
    pub origin: CanonicalArtifactOrigin,
    /// Declared media type.
    pub media_type: String,
    /// Data classification.
    pub classification: CanonicalClassification,
    /// Capture state.
    pub capture_state: CanonicalCaptureState,
    /// Exact byte length only when captured.
    pub byte_length: Option<u64>,
    /// Exact SHA-256 only when captured.
    pub sha256: Option<String>,
    /// Trusted RFC 3339 collection time.
    pub collected_at: String,
}

/// Terminal ingestion disposition.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CanonicalIngestionDisposition {
    /// Source bytes were captured.
    Captured,
    /// Source was parsed completely.
    Parsed,
    /// Source was parsed partially.
    Partial,
    /// Source is unsupported.
    Unsupported,
    /// Policy denied ingestion.
    Denied,
    /// Source was unavailable.
    Unavailable,
    /// Ingestion failed.
    Failed,
    /// Source was explicitly omitted.
    Omitted,
}

/// Terminal result of ingesting one artifact.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CanonicalArtifactIngestionResult {
    /// Contract schema version.
    pub schema_version: u16,
    /// Stable ingestion identity.
    pub ingestion_id: String,
    /// Source artifact identity.
    pub artifact_id: String,
    /// Terminal disposition.
    pub disposition: CanonicalIngestionDisposition,
    /// Exact source reference when captured.
    pub source: Option<CanonicalArtifactReference>,
    /// Ordered derivative transformation identities.
    pub transformation_ids: Vec<String>,
    /// Bounded visible warnings.
    pub warnings: Vec<String>,
    /// Stable terminal error code.
    pub error_code: Option<String>,
    /// Must be true for a published result.
    pub terminal: bool,
}

/// Reproducible derivative transformation over exact source bytes.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CanonicalArtifactTransformation {
    /// Contract schema version.
    pub schema_version: u16,
    /// Stable transformation identity.
    pub transformation_id: String,
    /// Source artifact identity.
    pub artifact_id: String,
    /// Transformer implementation identity.
    pub transformer_id: String,
    /// Transformer implementation version.
    pub transformer_version: String,
    /// SHA-256 of the exact input.
    pub input_sha256: String,
    /// SHA-256 of the exact output when one exists.
    pub output_sha256: Option<String>,
    /// Exact source ranges examined.
    pub source_ranges: Vec<CanonicalByteRange>,
    /// Bounded visible warnings.
    pub warnings: Vec<String>,
    /// Whether identical inputs and implementation reproduce the output.
    pub reproducible: bool,
}

/// Disposition of an artifact in one model context.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CanonicalContextDisposition {
    /// Exact source was included.
    Included,
    /// A declared derivative summary was included.
    Summarized,
    /// Only a declared prefix or ranges were included.
    Truncated,
    /// Content was already represented by another item.
    Duplicate,
    /// Source was stale.
    Stale,
    /// Source format was unsupported.
    Unsupported,
    /// Source was unavailable.
    Unavailable,
    /// Policy restricted source delivery.
    Restricted,
    /// Source was deliberately omitted.
    Omitted,
}

/// One artifact's exact contribution to a context manifest.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CanonicalContextItem {
    /// Source artifact identity.
    pub artifact_id: String,
    /// Delivery disposition.
    pub disposition: CanonicalContextDisposition,
    /// Exact admitted source ranges.
    pub ranges: Vec<CanonicalByteRange>,
    /// Token count attributed to this item.
    pub token_count: u64,
    /// Stable deterministic code required for every non-complete disposition; null when included.
    #[serde(deserialize_with = "crate::serialization::deserialize_required_option")]
    pub reason_code: Option<String>,
    /// Visible reason for a non-complete disposition.
    #[serde(deserialize_with = "crate::serialization::deserialize_required_option")]
    pub reason: Option<String>,
}

/// Sealed declaration of the context assembled for one model turn.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CanonicalContextManifest {
    /// Contract schema version.
    pub schema_version: u16,
    /// Stable manifest identity.
    pub context_manifest_id: String,
    /// Owning session.
    pub session_id: String,
    /// Owning turn.
    pub turn_id: String,
    /// Selected model profile.
    pub model_profile_id: String,
    /// Number of source artifacts considered.
    pub source_artifact_count: u64,
    /// Ordered artifact decisions.
    pub items: Vec<CanonicalContextItem>,
    /// Total admitted input tokens.
    pub total_input_tokens: u64,
    /// Output tokens reserved from the context window.
    pub reserved_output_tokens: u64,
    /// Safety margin reserved from the context window.
    pub safety_margin_tokens: u64,
    /// Digest of the canonical manifest with this field zeroed.
    pub manifest_sha256: String,
}

/// Exact delivered range for one artifact.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CanonicalDeliveredArtifact {
    /// Artifact identity.
    pub artifact_id: String,
    /// Digest of the complete authoritative source.
    pub sha256: String,
    /// Exact delivered byte range.
    pub range: CanonicalByteRange,
    /// Tokens attributed to this range.
    pub token_count: u64,
}

/// Terminal context-delivery state.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CanonicalDeliveryOutcome {
    /// All required context was delivered.
    Delivered,
    /// Required context was not delivered.
    Blocked,
}

/// Sealed proof of the exact context delivered to a model request.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CanonicalContextDeliveryReceipt {
    /// Contract schema version.
    pub schema_version: u16,
    /// Stable receipt identity.
    pub receipt_id: String,
    /// Context manifest identity.
    pub context_manifest_id: String,
    /// Digest of that manifest.
    pub context_manifest_sha256: String,
    /// Exact model request identity.
    pub model_request_id: String,
    /// Route decision identity.
    pub route_decision_id: String,
    /// Ordered exact delivered ranges.
    pub delivered: Vec<CanonicalDeliveredArtifact>,
    /// Required artifacts the model could not observe.
    pub required_unseen_artifact_ids: Vec<String>,
    /// Terminal delivery outcome.
    pub outcome: CanonicalDeliveryOutcome,
    /// Digest of the canonical receipt with this field zeroed.
    pub receipt_sha256: String,
}

/// Reference to one closed input or output schema.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CanonicalSchemaBinding {
    /// Stable schema identity.
    pub schema_id: String,
    /// Schema version.
    pub schema_version: u64,
    /// Digest of the exact schema.
    pub schema_sha256: String,
}

/// Closed execution budgets shared by workflows and capabilities.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CanonicalExecutionBudgets {
    /// Maximum model turns.
    pub turns: u64,
    /// Maximum model tokens.
    pub tokens: u64,
    /// Maximum wall duration in milliseconds.
    pub duration_ms: u64,
    /// Maximum tool calls.
    pub tool_calls: u64,
    /// Maximum attempts.
    pub attempts: u64,
    /// Maximum no-progress events.
    pub no_progress_events: u64,
    /// Maximum generated output bytes.
    pub output_bytes: u64,
    /// Maximum memory bytes.
    pub memory_bytes: u64,
    /// Abstract policy-defined minor-unit ceiling; never a credential or payment authority.
    pub cost_minor_units: u64,
}

/// Effect class of one workflow step.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CanonicalEffectClass {
    /// Read-only operation.
    ReadOnly,
    /// Idempotent write operation.
    IdempotentWrite,
    /// Effect depends on verified preconditions.
    Conditional,
    /// Non-idempotent operation.
    NonIdempotent,
    /// Destructive operation.
    Destructive,
    /// External effect.
    External,
    /// Effect class is unknown and must fail closed.
    Unknown,
}

/// Retry policy for one workflow step.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CanonicalRetryClass {
    /// Never retry automatically.
    Never,
    /// A read can start a fresh attempt.
    RecoverableRead,
    /// Retry only after state reconciliation.
    ConditionalAfterReconciliation,
    /// A user decision is required.
    UserDecisionRequired,
}

/// One closed workflow step.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CanonicalWorkflowStep {
    /// Stable step identity.
    pub step_id: String,
    /// Dependency step identities.
    pub depends_on: Vec<String>,
    /// Optional model role.
    pub model_role: Option<String>,
    /// Optional tool identity.
    pub tool_id: Option<String>,
    /// Effect class.
    pub effect_class: CanonicalEffectClass,
    /// Retry class.
    pub retry_class: CanonicalRetryClass,
    /// Required verifier identities.
    pub verifier_ids: Vec<String>,
    /// Exact step budgets.
    pub budgets: CanonicalExecutionBudgets,
}

/// Immutable workflow definition.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CanonicalWorkflowDefinition {
    /// Contract schema version.
    pub schema_version: u16,
    /// Stable workflow identity.
    pub workflow_id: String,
    /// Workflow version.
    pub workflow_version: u64,
    /// Input schema binding.
    pub input_schema: CanonicalSchemaBinding,
    /// Output schema binding.
    pub output_schema: CanonicalSchemaBinding,
    /// Closed ordered step set.
    pub steps: Vec<CanonicalWorkflowStep>,
    /// Initial dependency-ready steps.
    pub entry_step_ids: Vec<String>,
    /// Digest of this definition with this field zeroed.
    pub definition_sha256: String,
}

/// Durable workflow lifecycle state.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CanonicalWorkflowLifecycle {
    /// Workflow was created.
    Created,
    /// Inputs are being validated.
    Validating,
    /// Workflow is ready.
    Ready,
    /// Workflow is running.
    Running,
    /// Postconditions are being verified.
    Verifying,
    /// A dependency is unavailable.
    WaitingForDependency,
    /// Approval is required.
    WaitingForApproval,
    /// Workflow is paused.
    Paused,
    /// Potential effects are being reconciled.
    Reconciling,
    /// Workflow is recovering from interruption.
    Recovering,
    /// Verified success.
    Succeeded,
    /// Verified no-op.
    NoOp,
    /// Workflow is blocked.
    Blocked,
    /// Workflow failed.
    Failed,
    /// Workflow was cancelled and cleaned up.
    Cancelled,
    /// Workflow timed out.
    TimedOut,
    /// Resource budget was exhausted.
    ResourceExhausted,
    /// Effect state is uncertain.
    Uncertain,
}

/// Current durable state of one workflow.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CanonicalWorkflowState {
    /// Contract schema version.
    pub schema_version: u16,
    /// Stable workflow identity.
    pub workflow_id: String,
    /// Workflow definition version.
    pub workflow_version: u64,
    /// Monotonic journal sequence.
    pub sequence: u64,
    /// Current lifecycle state.
    pub state: CanonicalWorkflowLifecycle,
    /// Active step, if any.
    pub active_step_id: Option<String>,
    /// Completed steps.
    pub completed_step_ids: Vec<String>,
    /// Attempt identities.
    pub attempt_ids: Vec<String>,
    /// Digest of consumed budgets.
    pub consumed_budget_sha256: String,
    /// Terminal result identity only after terminal transition.
    pub terminal_result_id: Option<String>,
}

/// Durable restart checkpoint for one workflow.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CanonicalWorkflowCheckpoint {
    /// Contract schema version.
    pub schema_version: u16,
    /// Stable checkpoint identity.
    pub checkpoint_id: String,
    /// Owning workflow.
    pub workflow_id: String,
    /// Digest of exact workflow state.
    pub state_sha256: String,
    /// Bound journal sequence.
    pub journal_sequence: u64,
    /// Digest of source manifest.
    pub source_manifest_sha256: String,
    /// Digest of policy snapshot.
    pub policy_sha256: String,
    /// Digest of environment snapshot.
    pub environment_sha256: String,
    /// Digest of route decision.
    pub route_sha256: String,
    /// Digest of tool catalog.
    pub tool_catalog_sha256: String,
    /// Bound receipt digests.
    pub receipt_sha256s: Vec<String>,
    /// Bound consumed-grant digests.
    pub consumed_grant_sha256s: Vec<String>,
    /// Trusted RFC 3339 creation time.
    pub created_at: String,
}

/// Endpoint deployment class.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CanonicalEndpointClass {
    /// Current-device endpoint.
    StrictLocal,
    /// Private local-network endpoint.
    LocalNetworkPrivate,
    /// Private remote endpoint.
    RemotePrivate,
    /// Managed remote endpoint.
    RemoteManaged,
}

/// TLS requirement for an endpoint.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CanonicalTlsPolicy {
    /// Not applicable to a private local transport.
    NotApplicableLocal,
    /// Server TLS must verify.
    VerifiedTls,
    /// Mutual TLS must verify.
    MutualTls,
}

/// Disabled-by-default endpoint profile.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CanonicalModelEndpointProfile {
    /// Contract schema version.
    pub schema_version: u16,
    /// Stable endpoint profile identity.
    pub endpoint_profile_id: String,
    /// Deployment class.
    pub profile_class: CanonicalEndpointClass,
    /// Operator identity.
    pub operator_id: String,
    /// Non-secret endpoint reference.
    pub endpoint_reference: String,
    /// Protocol codec identity.
    pub protocol_codec_id: String,
    /// TLS policy.
    pub tls_policy: CanonicalTlsPolicy,
    /// Digest of host allow policy.
    pub host_policy_sha256: String,
    /// Brokered credential reference, never a secret.
    pub credential_reference: Option<String>,
    /// Required region.
    pub region: Option<String>,
    /// Retention policy description.
    pub retention_policy: String,
    /// Logging policy description.
    pub logging_policy: String,
    /// Training-use policy description.
    pub training_use_policy: String,
    /// Digest of quota policy.
    pub quota_policy_sha256: String,
    /// Digest of abstract resource policy.
    pub cost_policy_sha256: String,
    /// Current qualification digest.
    pub qualification_sha256: Option<String>,
    /// Must remain false until a separate activation record exists.
    pub enabled: bool,
    /// Must remain false; fallback is an explicit decision.
    pub automatic_fallback: bool,
}

/// Disclosure class for a model route.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CanonicalDisclosureClass {
    /// No disclosure boundary is crossed.
    None,
    /// Data enters a private network endpoint.
    PrivateNetwork,
    /// Data enters a private remote endpoint.
    PrivateRemote,
    /// Data enters a managed remote endpoint.
    ManagedRemote,
}

/// One route considered by deterministic policy.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CanonicalConsideredRoute {
    /// Route identity.
    pub route_id: String,
    /// Whether current evidence qualifies the route.
    pub qualified: bool,
    /// Whether deterministic policy admits the route.
    pub admitted: bool,
    /// Visible deterministic reason.
    pub reason: String,
}

/// Sealed deterministic model-route decision.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CanonicalModelRouteDecision {
    /// Contract schema version.
    pub schema_version: u16,
    /// Stable decision identity.
    pub route_decision_id: String,
    /// Owning request.
    pub request_id: String,
    /// Digest of route policy.
    pub policy_sha256: String,
    /// Disclosure class.
    pub disclosure_class: CanonicalDisclosureClass,
    /// Ordered considered routes.
    pub considered_routes: Vec<CanonicalConsideredRoute>,
    /// Selected route, or none when blocked.
    pub selected_route_id: Option<String>,
    /// Whether explicit fallback was used.
    pub fallback_used: bool,
    /// Explicit fallback policy digest when used.
    pub fallback_policy_sha256: Option<String>,
    /// Visible deterministic reason.
    pub reason: String,
    /// Digest of this decision with this field zeroed.
    pub decision_sha256: String,
}

/// Terminal tool outcome.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CanonicalToolOutcome {
    /// Tool succeeded.
    Succeeded,
    /// Authority denied dispatch.
    Denied,
    /// Tool failed.
    Failed,
    /// Tool was cancelled and cleaned up.
    Cancelled,
    /// Tool timed out.
    TimedOut,
    /// Resource budget was exhausted.
    ResourceExhausted,
    /// Effect state is uncertain.
    Uncertain,
}

/// Observed state-change classification.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CanonicalStateChange {
    /// Verified unchanged.
    NotChanged,
    /// Verified changed.
    Changed,
    /// State could not be reconciled.
    Uncertain,
}

/// Retry disposition after a tool attempt.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CanonicalRetryDisposition {
    /// Retry is not eligible.
    NotEligible,
    /// A fresh attempt is eligible.
    EligibleFreshAttempt,
    /// Reconciliation is required first.
    ReconcileFirst,
    /// A user decision is required.
    UserDecisionRequired,
}

/// Complete terminal observation of one tool attempt.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CanonicalToolObservation {
    /// Contract schema version.
    pub schema_version: u16,
    /// Stable observation identity.
    pub observation_id: String,
    /// Tool call identity.
    pub tool_call_id: String,
    /// Attempt identity.
    pub attempt_id: String,
    /// Task identity.
    pub task_id: String,
    /// Workflow step identity.
    pub step_id: String,
    /// Tool identity.
    pub tool_id: String,
    /// Tool version.
    pub tool_version: String,
    /// Digest of the tool schema.
    pub tool_schema_sha256: String,
    /// Digest of validated arguments.
    pub arguments_sha256: String,
    /// Consumed authority identity.
    pub authority_id: String,
    /// Trusted RFC 3339 start time.
    pub started_at: String,
    /// Trusted RFC 3339 completion time.
    pub completed_at: String,
    /// Terminal outcome.
    pub outcome: CanonicalToolOutcome,
    /// Process exit code when applicable.
    pub exit_code: Option<i32>,
    /// Process signal description when applicable.
    pub signal: Option<String>,
    /// Exact stdout artifact when present.
    pub stdout: Option<CanonicalArtifactReference>,
    /// Exact stderr artifact when present.
    pub stderr: Option<CanonicalArtifactReference>,
    /// Bounded stdout display excerpt.
    pub stdout_excerpt: String,
    /// Bounded stderr display excerpt.
    pub stderr_excerpt: String,
    /// Whether stdout display was truncated.
    pub stdout_truncated: bool,
    /// Whether stderr display was truncated.
    pub stderr_truncated: bool,
    /// Generated artifact identities.
    pub generated_artifact_ids: Vec<String>,
    /// Verified state-change classification.
    pub state_change: CanonicalStateChange,
    /// Whether all owned descendants were cleaned.
    pub descendants_cleaned: bool,
    /// Digest of resource usage.
    pub resource_usage_sha256: String,
    /// Retry disposition.
    pub retry_disposition: CanonicalRetryDisposition,
    /// Digest of this receipt with this field zeroed.
    pub receipt_sha256: String,
    /// Must be true for a published observation.
    pub terminal: bool,
}

/// Capability lifecycle state.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CanonicalCapabilityLifecycle {
    /// Draft capability.
    Draft,
    /// Capability passed admission.
    Admitted,
    /// Capability is enabled.
    Enabled,
    /// Capability is degraded.
    Degraded,
    /// Capability is disabled.
    Disabled,
    /// Capability is quarantined.
    Quarantined,
    /// Capability is retired.
    Retired,
}

/// Immutable schema-bound capability manifest.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CanonicalCapabilityManifest {
    /// Contract schema version.
    pub schema_version: u16,
    /// Stable capability identity.
    pub capability_id: String,
    /// Capability version.
    pub capability_version: u64,
    /// Publisher identity.
    pub publisher_id: String,
    /// User-visible title.
    pub title: String,
    /// Bounded purpose.
    pub purpose: String,
    /// Lifecycle state.
    pub lifecycle: CanonicalCapabilityLifecycle,
    /// Input schema.
    pub input_schema: CanonicalSchemaBinding,
    /// Output schema.
    pub output_schema: CanonicalSchemaBinding,
    /// Digest of workflow definition.
    pub workflow_definition_sha256: String,
    /// Required tool identities.
    pub required_tool_ids: Vec<String>,
    /// Required model roles.
    pub required_model_roles: Vec<String>,
    /// Requested authority identities.
    pub requested_authority_ids: Vec<String>,
    /// Explicitly prohibited authority identities.
    pub prohibited_authority_ids: Vec<String>,
    /// Capability budgets.
    pub budgets: CanonicalExecutionBudgets,
    /// Required verifier identities.
    pub verifier_ids: Vec<String>,
    /// Digest of qualification fixtures.
    pub fixture_manifest_sha256: String,
    /// Digest of migration policy.
    pub migration_policy_sha256: String,
    /// Digest of removal policy.
    pub removal_policy_sha256: String,
    /// Digest of this manifest with this field zeroed.
    pub manifest_sha256: String,
}

/// Verification outcome.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CanonicalVerificationOutcome {
    /// Verification passed on current evidence.
    Passed,
    /// Verification failed.
    Failed,
    /// Verification was blocked.
    Blocked,
    /// Verification could not determine state.
    Uncertain,
}

/// Sealed deterministic verification result.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CanonicalVerificationResult {
    /// Contract schema version.
    pub schema_version: u16,
    /// Stable result identity.
    pub verification_result_id: String,
    /// Owning workflow.
    pub workflow_id: String,
    /// Optional owning step.
    pub step_id: Option<String>,
    /// Verifier identity.
    pub verifier_id: String,
    /// Verifier version.
    pub verifier_version: String,
    /// Digest of verified subject.
    pub subject_sha256: String,
    /// Digests of observed evidence.
    pub observed_evidence_sha256s: Vec<String>,
    /// Preserved invariant identities.
    pub preserved_invariants: Vec<String>,
    /// Prohibited effects actually observed.
    pub prohibited_effects_observed: Vec<String>,
    /// Verification outcome.
    pub outcome: CanonicalVerificationOutcome,
    /// Whether evidence is current.
    pub current: bool,
    /// Digest of this result with this field zeroed.
    pub result_sha256: String,
}

/// Runtime-established terminal outcome.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CanonicalTerminalOutcome {
    /// Deterministically verified success.
    VerifiedSuccess,
    /// Deterministically verified no-op.
    VerifiedNoOp,
    /// A prerequisite blocked completion.
    Blocked,
    /// Workflow failed.
    Failed,
    /// Workflow was cancelled and cleaned.
    Cancelled,
    /// Workflow timed out.
    TimedOut,
    /// Resource budget was exhausted.
    ResourceExhausted,
    /// Effect state is uncertain.
    Uncertain,
}

/// Current closed schema version of the source-artifact ownership record.
pub const SOURCE_ARTIFACT_SCHEMA_VERSION: u16 = 2;

/// Sole physical payload store admitted for retained source bytes.
///
/// Source retention reuses the existing encrypted content-addressed runtime
/// payload backend. No second physical store exists for source artifacts.
pub const SOURCE_PAYLOAD_STORE_ID: &str = "agentmage-runtime-payload-store-v1";

/// Sole existing [`RuntimeArtifactKind`] admitted for a retained source payload.
///
/// Source retention never widens the closed runtime artifact family.
pub const SOURCE_PAYLOAD_ARTIFACT_KIND: RuntimeArtifactKind = RuntimeArtifactKind::GeneratedFile;

/// Logical owner scope of one source artifact.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CanonicalSourceOwnerScope {
    /// The owning local session releases the source.
    Session,
    /// The owning task releases the source.
    Task,
    /// The originating request releases the source.
    Request,
}

/// Closed retention class of one source artifact.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CanonicalSourceRetentionClass {
    /// Retained only in process memory for the current run.
    Ephemeral,
    /// Retained under the owning session lifecycle.
    Session,
    /// Retained until an exact policy-selected expiration.
    UntilExpiration,
    /// Retained until an explicit user release decision.
    UserHold,
}

/// Whether source bytes reach the existing runtime payload backend.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CanonicalSourcePayloadBinding {
    /// No durable payload exists; only metadata and provenance persist.
    MemoryOnly,
    /// Bytes persist in the existing encrypted content-addressed payload store.
    RuntimePayloadStore,
}

/// Logical ownership and retention of one source artifact.
///
/// The record carries logical identity only. It grants no path authority, names
/// no native location, and creates no store beyond
/// [`SOURCE_PAYLOAD_STORE_ID`].
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CanonicalSourceRetention {
    /// Contract schema version.
    pub schema_version: u16,
    /// Owned source artifact identity.
    pub source_artifact_id: String,
    /// Owning local session; this identity is not read authority.
    pub session_id: String,
    /// Owning task.
    pub task_id: String,
    /// Logical owner scope.
    pub owner_scope: CanonicalSourceOwnerScope,
    /// Closed retention class.
    pub retention_class: CanonicalSourceRetentionClass,
    /// Trusted RFC 3339 expiration for `UntilExpiration`; absent otherwise.
    #[serde(deserialize_with = "crate::serialization::deserialize_required_option")]
    pub expires_at: Option<String>,
    /// Whether durable bytes exist in the existing payload backend.
    pub payload_binding: CanonicalSourcePayloadBinding,
    /// Exact physical store identity when bytes are retained.
    #[serde(deserialize_with = "crate::serialization::deserialize_required_option")]
    pub payload_store_id: Option<String>,
    /// Existing runtime artifact identity when bytes are retained.
    #[serde(deserialize_with = "crate::serialization::deserialize_required_option")]
    pub runtime_artifact_id: Option<String>,
    /// Existing runtime artifact kind when bytes are retained.
    #[serde(deserialize_with = "crate::serialization::deserialize_required_option")]
    pub runtime_artifact_kind: Option<RuntimeArtifactKind>,
    /// Content-free reason required for memory-only ownership and user holds.
    #[serde(deserialize_with = "crate::serialization::deserialize_required_option")]
    pub retention_reason_code: Option<String>,
    /// Current metadata lifecycle state owned by the operational store.
    pub lifecycle_state: RuntimeArtifactLifecycleState,
    /// Content-free cleanup disposition derived from lifecycle and references.
    pub cleanup_state: RuntimeArtifactCleanupState,
    /// Monotonic one-based lifecycle revision.
    pub lifecycle_revision: u64,
    /// Current checkpoints that root this source as a retention root.
    pub checkpoint_reference_count: u32,
    /// Active logical references to this source.
    pub active_reference_count: u32,
}

/// Deterministic reason one ownership and retention record is refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CanonicalSourceRetentionRejection {
    /// The record declares an unsupported contract version.
    UnsupportedSchemaVersion,
    /// A durable binding named a store other than the single payload backend.
    ForeignPayloadStore,
    /// A durable binding named an artifact kind outside the admitted kind.
    ForeignArtifactKind,
    /// A durable binding omitted its exact runtime artifact identity.
    MissingPayloadBinding,
    /// Memory-only ownership carried durable payload identity.
    UnexpectedPayloadBinding,
    /// Memory-only ownership claimed a payload-integrity lifecycle state.
    UnexpectedPayloadLifecycle,
    /// Ephemeral retention claimed durable bytes or a checkpoint root.
    EphemeralRetentionIsNotDurable,
    /// An expiring class carried no expiration, or another class carried one.
    ExpirationMismatch,
    /// A required content-free retention reason code was absent or unexpected.
    RetentionReasonMismatch,
    /// The lifecycle revision is not one-based and monotonic.
    InvalidLifecycleRevision,
    /// An active lifecycle carried no active logical reference.
    MissingActiveReference,
    /// A deleted lifecycle still carried a live logical reference.
    LiveReferenceAfterDeletion,
    /// The declared cleanup state contradicts the deterministic projection.
    CleanupProjectionMismatch,
}

impl CanonicalSourceRetentionRejection {
    /// Stable content-free reason code for diagnostics and events.
    pub fn reason_code(self) -> &'static str {
        match self {
            Self::UnsupportedSchemaVersion => "source-retention-unsupported-version",
            Self::ForeignPayloadStore => "source-retention-foreign-store",
            Self::ForeignArtifactKind => "source-retention-foreign-artifact-kind",
            Self::MissingPayloadBinding => "source-retention-missing-payload-binding",
            Self::UnexpectedPayloadBinding => "source-retention-unexpected-payload-binding",
            Self::UnexpectedPayloadLifecycle => "source-retention-unexpected-payload-lifecycle",
            Self::EphemeralRetentionIsNotDurable => "source-retention-ephemeral-not-durable",
            Self::ExpirationMismatch => "source-retention-expiration-mismatch",
            Self::RetentionReasonMismatch => "source-retention-reason-mismatch",
            Self::InvalidLifecycleRevision => "source-retention-invalid-revision",
            Self::MissingActiveReference => "source-retention-missing-active-reference",
            Self::LiveReferenceAfterDeletion => "source-retention-live-reference-after-deletion",
            Self::CleanupProjectionMismatch => "source-retention-cleanup-mismatch",
        }
    }
}

/// Deterministic cleanup projection for one owned source artifact.
///
/// A current checkpoint or any remaining active logical reference keeps a
/// released source retained; integrity loss blocks collection.
pub fn canonical_source_cleanup_state(
    lifecycle_state: RuntimeArtifactLifecycleState,
    checkpoint_reference_count: u32,
    active_reference_count: u32,
) -> RuntimeArtifactCleanupState {
    match lifecycle_state {
        RuntimeArtifactLifecycleState::Active => RuntimeArtifactCleanupState::Retained,
        RuntimeArtifactLifecycleState::Quarantined => RuntimeArtifactCleanupState::Blocked,
        RuntimeArtifactLifecycleState::Deleted => RuntimeArtifactCleanupState::Completed,
        RuntimeArtifactLifecycleState::Released => {
            if checkpoint_reference_count == 0 && active_reference_count == 0 {
                RuntimeArtifactCleanupState::Eligible
            } else {
                RuntimeArtifactCleanupState::Retained
            }
        }
    }
}

/// Admit one closed ownership and retention record or refuse it deterministically.
pub fn admit_canonical_source_retention(
    record: &CanonicalSourceRetention,
) -> Result<(), CanonicalSourceRetentionRejection> {
    use CanonicalSourceRetentionRejection as Rejection;

    if record.schema_version != SOURCE_ARTIFACT_SCHEMA_VERSION {
        return Err(Rejection::UnsupportedSchemaVersion);
    }
    if record.lifecycle_revision == 0 {
        return Err(Rejection::InvalidLifecycleRevision);
    }

    let durable = record.payload_binding == CanonicalSourcePayloadBinding::RuntimePayloadStore;
    if durable {
        if record.payload_store_id.as_deref() != Some(SOURCE_PAYLOAD_STORE_ID) {
            return Err(Rejection::ForeignPayloadStore);
        }
        if record.runtime_artifact_id.is_none() {
            return Err(Rejection::MissingPayloadBinding);
        }
        if record.runtime_artifact_kind != Some(SOURCE_PAYLOAD_ARTIFACT_KIND) {
            return Err(Rejection::ForeignArtifactKind);
        }
    } else {
        if record.payload_store_id.is_some()
            || record.runtime_artifact_id.is_some()
            || record.runtime_artifact_kind.is_some()
        {
            return Err(Rejection::UnexpectedPayloadBinding);
        }
        if record.lifecycle_state == RuntimeArtifactLifecycleState::Quarantined {
            return Err(Rejection::UnexpectedPayloadLifecycle);
        }
    }

    if record.retention_class == CanonicalSourceRetentionClass::Ephemeral
        && (durable || record.checkpoint_reference_count > 0)
    {
        return Err(Rejection::EphemeralRetentionIsNotDurable);
    }

    let expiring = record.retention_class == CanonicalSourceRetentionClass::UntilExpiration;
    if expiring != record.expires_at.is_some() {
        return Err(Rejection::ExpirationMismatch);
    }

    let reason_required =
        !durable || record.retention_class == CanonicalSourceRetentionClass::UserHold;
    if reason_required != record.retention_reason_code.is_some() {
        return Err(Rejection::RetentionReasonMismatch);
    }

    if record.lifecycle_state == RuntimeArtifactLifecycleState::Active
        && record.active_reference_count == 0
    {
        return Err(Rejection::MissingActiveReference);
    }
    if record.lifecycle_state == RuntimeArtifactLifecycleState::Deleted
        && (record.active_reference_count > 0 || record.checkpoint_reference_count > 0)
    {
        return Err(Rejection::LiveReferenceAfterDeletion);
    }

    let projected = canonical_source_cleanup_state(
        record.lifecycle_state,
        record.checkpoint_reference_count,
        record.active_reference_count,
    );
    if record.cleanup_state != projected {
        return Err(Rejection::CleanupProjectionMismatch);
    }
    Ok(())
}

/// Final runtime-verifier result for one workflow.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CanonicalTerminalResult {
    /// Contract schema version.
    pub schema_version: u16,
    /// Stable terminal-result identity.
    pub terminal_result_id: String,
    /// Owning workflow.
    pub workflow_id: String,
    /// Terminal outcome.
    pub outcome: CanonicalTerminalOutcome,
    /// Supporting verification result identities.
    pub verification_result_ids: Vec<String>,
    /// Digest of last verified state.
    pub last_verified_state_sha256: String,
    /// Stable diagnostic code on non-success.
    pub diagnostic_code: Option<String>,
    /// Safe visible next action on non-success.
    pub safe_next_action: Option<String>,
    /// Must equal `agentmage-runtime-verifier`.
    pub established_by: String,
    /// Digest of this result with this field zeroed.
    pub result_sha256: String,
}

#[cfg(test)]
mod source_retention_tests {
    use super::{
        CanonicalSourceOwnerScope, CanonicalSourcePayloadBinding, CanonicalSourceRetention,
        CanonicalSourceRetentionClass, CanonicalSourceRetentionRejection,
        RuntimeArtifactCleanupState, RuntimeArtifactKind, RuntimeArtifactLifecycleState,
        SOURCE_ARTIFACT_SCHEMA_VERSION, SOURCE_PAYLOAD_ARTIFACT_KIND, SOURCE_PAYLOAD_STORE_ID,
        admit_canonical_source_retention, canonical_source_cleanup_state,
    };

    type Rejection = CanonicalSourceRetentionRejection;
    type Cleanup = RuntimeArtifactCleanupState;
    type Lifecycle = RuntimeArtifactLifecycleState;
    type Class = CanonicalSourceRetentionClass;

    fn rejection(record: &CanonicalSourceRetention) -> Option<Rejection> {
        admit_canonical_source_retention(record).err()
    }

    fn retained() -> CanonicalSourceRetention {
        CanonicalSourceRetention {
            schema_version: SOURCE_ARTIFACT_SCHEMA_VERSION,
            source_artifact_id: "source-1".to_owned(),
            session_id: "session-1".to_owned(),
            task_id: "task-1".to_owned(),
            owner_scope: CanonicalSourceOwnerScope::Session,
            retention_class: Class::Session,
            expires_at: None,
            payload_binding: CanonicalSourcePayloadBinding::RuntimePayloadStore,
            payload_store_id: Some(SOURCE_PAYLOAD_STORE_ID.to_owned()),
            runtime_artifact_id: Some("artifact-1".to_owned()),
            runtime_artifact_kind: Some(SOURCE_PAYLOAD_ARTIFACT_KIND),
            retention_reason_code: None,
            lifecycle_state: Lifecycle::Active,
            cleanup_state: Cleanup::Retained,
            lifecycle_revision: 1,
            checkpoint_reference_count: 0,
            active_reference_count: 1,
        }
    }

    fn memory_only() -> CanonicalSourceRetention {
        CanonicalSourceRetention {
            payload_binding: CanonicalSourcePayloadBinding::MemoryOnly,
            payload_store_id: None,
            runtime_artifact_id: None,
            runtime_artifact_kind: None,
            retention_reason_code: Some("retention-memory-only".to_owned()),
            ..retained()
        }
    }

    #[test]
    fn retained_and_memory_only_ownership_is_admitted() {
        assert_eq!(rejection(&retained()), None);
        assert_eq!(rejection(&memory_only()), None);
    }

    #[test]
    fn durable_binding_admits_one_store_and_one_existing_artifact_kind() {
        assert_eq!(SOURCE_PAYLOAD_ARTIFACT_KIND, RuntimeArtifactKind::GeneratedFile);

        let mut foreign_store = retained();
        foreign_store.payload_store_id = Some("agentmage-source-payload-store-v1".to_owned());
        assert_eq!(rejection(&foreign_store), Some(Rejection::ForeignPayloadStore));

        let mut absent_store = retained();
        absent_store.payload_store_id = None;
        assert_eq!(rejection(&absent_store), Some(Rejection::ForeignPayloadStore));

        let mut absent_identity = retained();
        absent_identity.runtime_artifact_id = None;
        assert_eq!(rejection(&absent_identity), Some(Rejection::MissingPayloadBinding));

        for kind in [
            RuntimeArtifactKind::Patch,
            RuntimeArtifactKind::StandardOutput,
            RuntimeArtifactKind::StandardError,
            RuntimeArtifactKind::TestLog,
            RuntimeArtifactKind::Report,
            RuntimeArtifactKind::ModelOutput,
        ] {
            let mut record = retained();
            record.runtime_artifact_kind = Some(kind);
            assert_eq!(rejection(&record), Some(Rejection::ForeignArtifactKind), "{kind:?}");
        }
    }

    #[test]
    fn memory_only_ownership_carries_no_payload_identity_or_quarantine() {
        let mut bound = memory_only();
        bound.runtime_artifact_id = Some("artifact-1".to_owned());
        assert_eq!(rejection(&bound), Some(Rejection::UnexpectedPayloadBinding));

        let mut quarantined = memory_only();
        quarantined.lifecycle_state = Lifecycle::Quarantined;
        quarantined.cleanup_state = Cleanup::Blocked;
        assert_eq!(rejection(&quarantined), Some(Rejection::UnexpectedPayloadLifecycle));

        let mut without_reason = memory_only();
        without_reason.retention_reason_code = None;
        assert_eq!(rejection(&without_reason), Some(Rejection::RetentionReasonMismatch));
    }

    #[test]
    fn retention_classes_bind_expiration_holds_and_ephemeral_limits() {
        let mut expiring = retained();
        expiring.retention_class = Class::UntilExpiration;
        assert_eq!(rejection(&expiring), Some(Rejection::ExpirationMismatch));
        expiring.expires_at = Some("2026-09-25T12:00:00Z".to_owned());
        assert_eq!(rejection(&expiring), None);

        let mut unexpected_expiration = retained();
        unexpected_expiration.expires_at = Some("2026-09-25T12:00:00Z".to_owned());
        assert_eq!(rejection(&unexpected_expiration), Some(Rejection::ExpirationMismatch));

        let mut hold = retained();
        hold.retention_class = Class::UserHold;
        assert_eq!(rejection(&hold), Some(Rejection::RetentionReasonMismatch));
        hold.retention_reason_code = Some("user-hold".to_owned());
        assert_eq!(rejection(&hold), None);

        let mut durable_ephemeral = retained();
        durable_ephemeral.retention_class = Class::Ephemeral;
        let durable_reason = Some(Rejection::EphemeralRetentionIsNotDurable);
        assert_eq!(rejection(&durable_ephemeral), durable_reason);

        let mut rooted_ephemeral = memory_only();
        rooted_ephemeral.retention_class = Class::Ephemeral;
        rooted_ephemeral.checkpoint_reference_count = 1;
        assert_eq!(rejection(&rooted_ephemeral), durable_reason);
        rooted_ephemeral.checkpoint_reference_count = 0;
        assert_eq!(rejection(&rooted_ephemeral), None);
    }

    #[test]
    fn checkpoint_and_reference_roots_drive_the_cleanup_projection() {
        assert_eq!(canonical_source_cleanup_state(Lifecycle::Released, 0, 0), Cleanup::Eligible);
        assert_eq!(canonical_source_cleanup_state(Lifecycle::Released, 1, 0), Cleanup::Retained);
        assert_eq!(canonical_source_cleanup_state(Lifecycle::Released, 0, 2), Cleanup::Retained);
        assert_eq!(canonical_source_cleanup_state(Lifecycle::Quarantined, 0, 0), Cleanup::Blocked);
        assert_eq!(canonical_source_cleanup_state(Lifecycle::Deleted, 0, 0), Cleanup::Completed);
        assert_eq!(canonical_source_cleanup_state(Lifecycle::Active, 0, 1), Cleanup::Retained);

        let mut released = retained();
        released.lifecycle_state = Lifecycle::Released;
        released.cleanup_state = Cleanup::Eligible;
        released.active_reference_count = 0;
        assert_eq!(rejection(&released), None);

        let mut rooted = released.clone();
        rooted.checkpoint_reference_count = 1;
        assert_eq!(rejection(&rooted), Some(Rejection::CleanupProjectionMismatch));
        rooted.cleanup_state = Cleanup::Retained;
        assert_eq!(rejection(&rooted), None);

        let mut inactive = retained();
        inactive.active_reference_count = 0;
        assert_eq!(rejection(&inactive), Some(Rejection::MissingActiveReference));

        let mut deleted = retained();
        deleted.lifecycle_state = Lifecycle::Deleted;
        deleted.cleanup_state = Cleanup::Completed;
        assert_eq!(rejection(&deleted), Some(Rejection::LiveReferenceAfterDeletion));
        deleted.active_reference_count = 0;
        assert_eq!(rejection(&deleted), None);
    }

    #[test]
    fn unsupported_versions_revisions_and_reason_codes_fail_closed() {
        for version in [0, 1, SOURCE_ARTIFACT_SCHEMA_VERSION + 1] {
            let mut record = retained();
            record.schema_version = version;
            assert_eq!(rejection(&record), Some(Rejection::UnsupportedSchemaVersion));
        }

        let mut zero_revision = retained();
        zero_revision.lifecycle_revision = 0;
        assert_eq!(rejection(&zero_revision), Some(Rejection::InvalidLifecycleRevision));

        let mut unexpected_reason = retained();
        unexpected_reason.retention_reason_code = Some("unexpected".to_owned());
        assert_eq!(rejection(&unexpected_reason), Some(Rejection::RetentionReasonMismatch));

        let code = Rejection::ForeignArtifactKind.reason_code();
        assert_eq!(code, "source-retention-foreign-artifact-kind");
    }

    #[test]
    fn ownership_serialization_is_closed_and_path_free() {
        let encoded = serde_json::to_string(&retained()).expect("serialize");
        assert!(encoded.contains("\"payload_store_id\":\"agentmage-runtime-payload-store-v1\""));
        assert!(encoded.contains("\"runtime_artifact_kind\":\"generated_file\""));
        assert!(!encoded.contains('/'));

        let decoded: CanonicalSourceRetention = serde_json::from_str(&encoded).expect("decode");
        assert_eq!(decoded, retained());

        let unknown = encoded.replace("{\"schema_version\"", "{\"path\":\"x\",\"schema_version\"");
        assert!(serde_json::from_str::<CanonicalSourceRetention>(&unknown).is_err());

        let absent = encoded.replace(",\"expires_at\":null", "");
        assert!(serde_json::from_str::<CanonicalSourceRetention>(&absent).is_err());
    }
}
