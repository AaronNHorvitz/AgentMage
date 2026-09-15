//! Canonical schema-bound records for the complete Engineering Runtime.

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

/// Supported schema version for the source-artifact retention contract.
pub const SOURCE_ARTIFACT_RETENTION_SCHEMA_VERSION: u16 = 1;

/// Retention semantic for one logical source-artifact ownership record.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CanonicalSourceArtifactRetentionClass {
    /// Keep only for the current in-memory run.
    Ephemeral,
    /// Retain under the owning session lifecycle.
    Session,
    /// Retain until the exact policy-selected expiration.
    UntilExpiration,
    /// Retain until an explicit user release.
    UserHold,
}

/// Current lifecycle state exposed by one logical source-artifact retention record.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CanonicalSourceArtifactRetentionLifecycleState {
    /// Retention is current and, when persistence permits, the payload may be opened
    /// under policy.
    Active,
    /// Retention was released and any backend payload is eligible for collection.
    Released,
    /// Retention is quarantined pending operator review.
    Quarantined,
    /// Retention was already deleted; only metadata remains.
    Deleted,
}

/// Explicit backend-persistence disposition for one logical source-artifact
/// retention record. This decouples logical source ownership from the runtime
/// artifact backend so that memory-only sources can be represented without
/// forcing a persisted payload.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CanonicalSourceArtifactPersistenceState {
    /// No backend payload was ever persisted; bytes exist only for the current run.
    MemoryOnly,
    /// Exactly one immutable payload is present in the shared encrypted backend.
    Persisted,
    /// A prior backend payload was released and only bounded metadata remains.
    Released,
    /// A prior backend payload was deleted and only bounded metadata remains.
    Deleted,
}

/// Opaque reference to one exact payload already present in the shared encrypted
/// content-addressed runtime artifact backend. The reference carries no semantic
/// family for the source artifact and never widens `RuntimeArtifactKind`.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CanonicalSourceArtifactPayloadReference {
    /// Existing content-addressed backend payload identity; never a native path.
    pub payload_artifact_id: String,
    /// SHA-256 content address of the exact backend payload.
    pub payload_sha256: String,
}

/// Logical source-artifact ownership and retention projected over the existing
/// encrypted content-addressed runtime artifact backend.
///
/// The record binds one source artifact to its owning request, authority, session,
/// task, and run. When `persistence_state` is `Persisted`, `payload_reference`
/// carries exactly one opaque reference to a payload already present in the
/// shared backend; the record intentionally carries no `RuntimeArtifactKind`
/// because source inputs do not belong to that runtime-generated taxonomy. When
/// `persistence_state` is `MemoryOnly`, no backend payload exists and the record
/// describes an in-memory source. `Released` and `Deleted` persistence states
/// preserve bounded historical metadata without asserting that payload bytes still
/// exist.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct CanonicalSourceArtifactRetention {
    /// Contract schema version.
    pub schema_version: u16,
    /// Stable retention record identity.
    pub retention_id: String,
    /// Owning source-artifact identity.
    pub source_artifact_id: String,
    /// Request that introduced the source artifact.
    pub request_id: String,
    /// Authority under which the source was captured.
    pub authority_id: String,
    /// Owning session.
    pub session_id: String,
    /// Owning task.
    pub task_id: String,
    /// Runtime run responsible for admission.
    pub run_id: String,
    /// Retention class.
    pub retention_class: CanonicalSourceArtifactRetentionClass,
    /// RFC 3339 expiration present only when `retention_class` is `UntilExpiration`.
    pub expires_at: Option<String>,
    /// Stable code present only when `retention_class` is `UserHold`.
    pub hold_reason_code: Option<String>,
    /// Current retention lifecycle state.
    pub lifecycle_state: CanonicalSourceArtifactRetentionLifecycleState,
    /// Explicit backend-persistence disposition.
    pub persistence_state: CanonicalSourceArtifactPersistenceState,
    /// Opaque backend payload reference required only when `persistence_state` is
    /// `Persisted`; every other state requires this field to be null.
    pub payload_reference: Option<CanonicalSourceArtifactPayloadReference>,
    /// Stable current-state reason code.
    pub reason_code: String,
    /// Trusted RFC 3339 assignment time.
    pub assigned_at: String,
    /// Digest of this record with this field zeroed.
    pub retention_sha256: String,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct RawSourceArtifactRetention {
    schema_version: u16,
    retention_id: String,
    source_artifact_id: String,
    request_id: String,
    authority_id: String,
    session_id: String,
    task_id: String,
    run_id: String,
    retention_class: CanonicalSourceArtifactRetentionClass,
    #[serde(deserialize_with = "crate::serialization::deserialize_required_option")]
    expires_at: Option<String>,
    #[serde(deserialize_with = "crate::serialization::deserialize_required_option")]
    hold_reason_code: Option<String>,
    lifecycle_state: CanonicalSourceArtifactRetentionLifecycleState,
    persistence_state: CanonicalSourceArtifactPersistenceState,
    #[serde(deserialize_with = "crate::serialization::deserialize_required_option")]
    payload_reference: Option<CanonicalSourceArtifactPayloadReference>,
    reason_code: String,
    assigned_at: String,
    retention_sha256: String,
}

impl TryFrom<RawSourceArtifactRetention> for CanonicalSourceArtifactRetention {
    type Error = String;

    fn try_from(raw: RawSourceArtifactRetention) -> Result<Self, Self::Error> {
        let record = Self {
            schema_version: raw.schema_version,
            retention_id: raw.retention_id,
            source_artifact_id: raw.source_artifact_id,
            request_id: raw.request_id,
            authority_id: raw.authority_id,
            session_id: raw.session_id,
            task_id: raw.task_id,
            run_id: raw.run_id,
            retention_class: raw.retention_class,
            expires_at: raw.expires_at,
            hold_reason_code: raw.hold_reason_code,
            lifecycle_state: raw.lifecycle_state,
            persistence_state: raw.persistence_state,
            payload_reference: raw.payload_reference,
            reason_code: raw.reason_code,
            assigned_at: raw.assigned_at,
            retention_sha256: raw.retention_sha256,
        };
        record.validate()?;
        Ok(record)
    }
}

impl<'de> serde::Deserialize<'de> for CanonicalSourceArtifactRetention {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let raw = RawSourceArtifactRetention::deserialize(deserializer)?;
        Self::try_from(raw).map_err(serde::de::Error::custom)
    }
}

impl CanonicalSourceArtifactRetention {
    /// Validates every closed invariant of one canonical retention record.
    ///
    /// The check enforces the supported schema version, identifier and digest
    /// formats, RFC 3339 timestamps, class-dependent optional fields, the
    /// persistence-state / lifecycle-state / payload-reference agreement, and
    /// that any `expires_at` is strictly later than `assigned_at`.
    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != SOURCE_ARTIFACT_RETENTION_SCHEMA_VERSION {
            return Err(format!(
                "unsupported schema_version {}; expected {SOURCE_ARTIFACT_RETENTION_SCHEMA_VERSION}",
                self.schema_version
            ));
        }
        for (name, value) in [
            ("retention_id", &self.retention_id),
            ("source_artifact_id", &self.source_artifact_id),
            ("request_id", &self.request_id),
            ("authority_id", &self.authority_id),
            ("session_id", &self.session_id),
            ("task_id", &self.task_id),
            ("run_id", &self.run_id),
            ("reason_code", &self.reason_code),
        ] {
            if !is_canonical_identifier(value) {
                return Err(format!("{name} is not a canonical identifier"));
            }
        }
        if !is_lowercase_hex_digest(&self.retention_sha256) {
            return Err("retention_sha256 is not a 64-character lowercase hex digest".to_string());
        }
        if !is_rfc3339_datetime(&self.assigned_at) {
            return Err("assigned_at is not an RFC 3339 timestamp".to_string());
        }
        match self.retention_class {
            CanonicalSourceArtifactRetentionClass::UntilExpiration => {
                let expiration = self.expires_at.as_deref().ok_or_else(|| {
                    "retention_class until_expiration requires expires_at".to_string()
                })?;
                if !is_rfc3339_datetime(expiration) {
                    return Err("expires_at is not an RFC 3339 timestamp".to_string());
                }
                if self.hold_reason_code.is_some() {
                    return Err(
                        "retention_class until_expiration must not carry hold_reason_code"
                            .to_string(),
                    );
                }
                if compare_rfc3339(expiration, &self.assigned_at)
                    .ok_or_else(|| "malformed timestamps for comparison".to_string())?
                    != core::cmp::Ordering::Greater
                {
                    return Err("expires_at must be strictly later than assigned_at".to_string());
                }
            }
            CanonicalSourceArtifactRetentionClass::UserHold => {
                let reason = self.hold_reason_code.as_deref().ok_or_else(|| {
                    "retention_class user_hold requires hold_reason_code".to_string()
                })?;
                if !is_canonical_identifier(reason) {
                    return Err("hold_reason_code is not a canonical identifier".to_string());
                }
                if self.expires_at.is_some() {
                    return Err(
                        "retention_class user_hold must not carry expires_at".to_string()
                    );
                }
            }
            CanonicalSourceArtifactRetentionClass::Ephemeral
            | CanonicalSourceArtifactRetentionClass::Session => {
                if self.expires_at.is_some() {
                    return Err(format!(
                        "retention_class {:?} must not carry expires_at",
                        self.retention_class
                    ));
                }
                if self.hold_reason_code.is_some() {
                    return Err(format!(
                        "retention_class {:?} must not carry hold_reason_code",
                        self.retention_class
                    ));
                }
            }
        }
        self.validate_persistence()?;
        Ok(())
    }

    fn validate_persistence(&self) -> Result<(), String> {
        use CanonicalSourceArtifactPersistenceState as Persistence;
        use CanonicalSourceArtifactRetentionClass as Class;
        use CanonicalSourceArtifactRetentionLifecycleState as Lifecycle;
        match (self.lifecycle_state, self.persistence_state) {
            (Lifecycle::Active, Persistence::MemoryOnly) => {
                if !matches!(self.retention_class, Class::Ephemeral) {
                    return Err(
                        "only ephemeral active retention may declare memory_only persistence"
                            .to_string(),
                    );
                }
            }
            (Lifecycle::Active | Lifecycle::Quarantined, Persistence::Persisted) => {}
            (Lifecycle::Released, Persistence::Released) => {}
            (Lifecycle::Deleted, Persistence::Deleted) => {}
            (lifecycle, persistence) => {
                return Err(format!(
                    "lifecycle_state {lifecycle:?} is incompatible with persistence_state {persistence:?}"
                ));
            }
        }
        match self.persistence_state {
            Persistence::Persisted => {
                let reference = self
                    .payload_reference
                    .as_ref()
                    .ok_or_else(|| "persisted retention requires payload_reference".to_string())?;
                if !is_canonical_identifier(&reference.payload_artifact_id) {
                    return Err("payload_artifact_id is not a canonical identifier".to_string());
                }
                if !is_lowercase_hex_digest(&reference.payload_sha256) {
                    return Err(
                        "payload_sha256 is not a 64-character lowercase hex digest".to_string(),
                    );
                }
            }
            Persistence::MemoryOnly | Persistence::Released | Persistence::Deleted => {
                if self.payload_reference.is_some() {
                    return Err(format!(
                        "persistence_state {:?} must not carry payload_reference",
                        self.persistence_state
                    ));
                }
            }
        }
        Ok(())
    }
}

fn is_canonical_identifier(value: &str) -> bool {
    if value.is_empty() || value.len() > 128 {
        return false;
    }
    let bytes = value.as_bytes();
    let head = bytes[0];
    if !head.is_ascii_alphanumeric() {
        return false;
    }
    bytes.iter().skip(1).copied().all(|byte| {
        byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b':' | b'-')
    })
}

fn is_lowercase_hex_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn is_rfc3339_datetime(value: &str) -> bool {
    parse_rfc3339_epoch(value).is_some()
}

fn compare_rfc3339(left: &str, right: &str) -> Option<core::cmp::Ordering> {
    let left_epoch = parse_rfc3339_epoch(left)?;
    let right_epoch = parse_rfc3339_epoch(right)?;
    Some(left_epoch.cmp(&right_epoch))
}

/// Returns an epoch tuple `(seconds, nanoseconds)` after normalizing to UTC.
///
/// The parser accepts the subset of RFC 3339 required by canonical records:
/// `YYYY-MM-DDTHH:MM:SS[.fraction]{Z|+HH:MM|-HH:MM}`. Fractional seconds are
/// bounded to nanosecond precision.
fn parse_rfc3339_epoch(value: &str) -> Option<(i64, u32)> {
    let bytes = value.as_bytes();
    if bytes.len() < 20 {
        return None;
    }
    let year: i32 = value.get(0..4)?.parse().ok()?;
    if bytes[4] != b'-' {
        return None;
    }
    let month: u32 = value.get(5..7)?.parse().ok()?;
    if bytes[7] != b'-' {
        return None;
    }
    let day: u32 = value.get(8..10)?.parse().ok()?;
    if bytes[10] != b'T' && bytes[10] != b't' {
        return None;
    }
    let hour: u32 = value.get(11..13)?.parse().ok()?;
    if bytes[13] != b':' {
        return None;
    }
    let minute: u32 = value.get(14..16)?.parse().ok()?;
    if bytes[16] != b':' {
        return None;
    }
    let second: u32 = value.get(17..19)?.parse().ok()?;
    let mut cursor = 19;
    let mut nanoseconds: u32 = 0;
    if cursor < bytes.len() && bytes[cursor] == b'.' {
        cursor += 1;
        let start = cursor;
        while cursor < bytes.len() && bytes[cursor].is_ascii_digit() {
            cursor += 1;
        }
        if cursor == start || cursor - start > 9 {
            return None;
        }
        let mut fraction = 0u32;
        for &byte in &bytes[start..cursor] {
            fraction = fraction * 10 + u32::from(byte - b'0');
        }
        for _ in 0..(9 - (cursor - start)) {
            fraction *= 10;
        }
        nanoseconds = fraction;
    }
    if cursor >= bytes.len() {
        return None;
    }
    let offset_seconds: i64 = match bytes[cursor] {
        b'Z' | b'z' => {
            if cursor + 1 != bytes.len() {
                return None;
            }
            0
        }
        b'+' | b'-' => {
            let sign: i64 = if bytes[cursor] == b'+' { 1 } else { -1 };
            let hh: i64 = value.get(cursor + 1..cursor + 3)?.parse().ok()?;
            if bytes.get(cursor + 3) != Some(&b':') {
                return None;
            }
            let mm: i64 = value.get(cursor + 4..cursor + 6)?.parse().ok()?;
            if cursor + 6 != bytes.len() {
                return None;
            }
            if !(0..24).contains(&hh) || !(0..60).contains(&mm) {
                return None;
            }
            sign * (hh * 3600 + mm * 60)
        }
        _ => return None,
    };
    if !(1..=12).contains(&month) || day == 0 || day > days_in_month(year, month) {
        return None;
    }
    if hour > 23 || minute > 59 || second > 60 {
        return None;
    }
    let mut days = days_from_civil(year, month, day);
    let mut seconds = i64::from(hour) * 3600 + i64::from(minute) * 60 + i64::from(second);
    seconds -= offset_seconds;
    while seconds < 0 {
        seconds += 86_400;
        days -= 1;
    }
    while seconds >= 86_400 {
        seconds -= 86_400;
        days += 1;
    }
    let total = days * 86_400 + seconds;
    Some((total, nanoseconds))
}

fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if is_leap_year(year) {
                29
            } else {
                28
            }
        }
        _ => 0,
    }
}

fn days_from_civil(year: i32, month: u32, day: u32) -> i64 {
    let year = if month <= 2 { year - 1 } else { year };
    let month = i64::from(if month <= 2 { month + 12 } else { month });
    let era = if year >= 0 { year } else { year - 399 } / 400;
    let yoe = i64::from(year - era * 400);
    let doy = (153 * (month - 3) + 2) / 5 + i64::from(day) - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    i64::from(era) * 146_097 + doe - 719_468
}

#[cfg(test)]
mod source_artifact_retention_tests {
    use super::{
        CanonicalSourceArtifactPayloadReference, CanonicalSourceArtifactPersistenceState,
        CanonicalSourceArtifactRetention, CanonicalSourceArtifactRetentionClass,
        CanonicalSourceArtifactRetentionLifecycleState, SOURCE_ARTIFACT_RETENTION_SCHEMA_VERSION,
    };

    const DIGEST: &str = "a1b2c3d4e5f60718293a4b5c6d7e8f900112233445566778899aabbccddeeff0";

    fn persisted_record() -> CanonicalSourceArtifactRetention {
        CanonicalSourceArtifactRetention {
            schema_version: SOURCE_ARTIFACT_RETENTION_SCHEMA_VERSION,
            retention_id: "retention-1".to_string(),
            source_artifact_id: "source-1".to_string(),
            request_id: "request-1".to_string(),
            authority_id: "authority-1".to_string(),
            session_id: "session-1".to_string(),
            task_id: "task-1".to_string(),
            run_id: "run-1".to_string(),
            retention_class: CanonicalSourceArtifactRetentionClass::Session,
            expires_at: None,
            hold_reason_code: None,
            lifecycle_state: CanonicalSourceArtifactRetentionLifecycleState::Active,
            persistence_state: CanonicalSourceArtifactPersistenceState::Persisted,
            payload_reference: Some(CanonicalSourceArtifactPayloadReference {
                payload_artifact_id: "artifact-1".to_string(),
                payload_sha256: DIGEST.to_string(),
            }),
            reason_code: "admitted".to_string(),
            assigned_at: "2026-08-25T12:00:00Z".to_string(),
            retention_sha256: DIGEST.to_string(),
        }
    }

    fn ephemeral_memory_only_record() -> CanonicalSourceArtifactRetention {
        CanonicalSourceArtifactRetention {
            retention_class: CanonicalSourceArtifactRetentionClass::Ephemeral,
            persistence_state: CanonicalSourceArtifactPersistenceState::MemoryOnly,
            payload_reference: None,
            ..persisted_record()
        }
    }

    #[test]
    fn ephemeral_source_without_backend_reference_validates_and_omits_runtime_artifact_kind() {
        let record = ephemeral_memory_only_record();
        record.validate().expect("valid ephemeral memory-only record");
        let json = serde_json::to_string(&record).expect("serialize");
        assert!(!json.contains("payload_kind"));
        assert!(!json.contains("generated_file"));
        let decoded: CanonicalSourceArtifactRetention =
            serde_json::from_str(&json).expect("deserialize");
        assert_eq!(decoded, record);
    }

    #[test]
    fn persisted_active_record_round_trips_through_opaque_backend_reference() {
        let record = persisted_record();
        record.validate().expect("valid persisted record");
        let json = serde_json::to_string(&record).expect("serialize");
        assert!(json.contains("\"payload_artifact_id\":\"artifact-1\""));
        assert!(!json.contains("payload_kind"));
        let decoded: CanonicalSourceArtifactRetention =
            serde_json::from_str(&json).expect("deserialize");
        assert_eq!(decoded, record);
    }

    #[test]
    fn persisted_retention_without_backend_reference_fails_closed() {
        let mut record = persisted_record();
        record.payload_reference = None;
        let error = record.validate().expect_err("must reject missing reference");
        assert!(error.contains("payload_reference"));
        let mut broken = serde_json::to_value(&persisted_record()).expect("serialize");
        broken["payload_reference"] = serde_json::Value::Null;
        let error = serde_json::from_value::<CanonicalSourceArtifactRetention>(broken)
            .expect_err("must reject at deserialization");
        assert!(error.to_string().contains("payload_reference"));
    }

    #[test]
    fn active_session_class_requires_persisted_backend_reference() {
        let mut record = ephemeral_memory_only_record();
        record.retention_class = CanonicalSourceArtifactRetentionClass::Session;
        let error = record.validate().expect_err("session must persist");
        assert!(error.contains("memory_only"));
    }

    #[test]
    fn deleted_retention_preserves_metadata_without_asserting_payload_bytes() {
        let mut record = persisted_record();
        record.lifecycle_state = CanonicalSourceArtifactRetentionLifecycleState::Deleted;
        record.persistence_state = CanonicalSourceArtifactPersistenceState::Deleted;
        record.payload_reference = None;
        record.reason_code = "user-delete".to_string();
        record.validate().expect("deleted metadata-only record must validate");
        let json = serde_json::to_string(&record).expect("serialize");
        assert!(!json.contains("payload_artifact_id"));
    }

    #[test]
    fn retention_class_and_dependent_fields_fail_closed_at_deserialization() {
        let mut with_missing_expiration = serde_json::to_value(&persisted_record()).expect("json");
        with_missing_expiration["retention_class"] = serde_json::json!("until_expiration");
        assert!(
            serde_json::from_value::<CanonicalSourceArtifactRetention>(
                with_missing_expiration.clone()
            )
            .is_err()
        );
        let mut simultaneous = with_missing_expiration.clone();
        simultaneous["expires_at"] = serde_json::json!("2026-08-26T00:00:00Z");
        simultaneous["hold_reason_code"] = serde_json::json!("user-hold");
        assert!(
            serde_json::from_value::<CanonicalSourceArtifactRetention>(simultaneous).is_err()
        );
        let mut hold_missing_reason = serde_json::to_value(&persisted_record()).expect("json");
        hold_missing_reason["retention_class"] = serde_json::json!("user_hold");
        assert!(
            serde_json::from_value::<CanonicalSourceArtifactRetention>(hold_missing_reason)
                .is_err()
        );
    }

    #[test]
    fn malformed_timestamps_digests_and_versions_fail_closed() {
        let mut malformed_time = serde_json::to_value(&persisted_record()).expect("json");
        malformed_time["assigned_at"] = serde_json::json!("2026/08/25 12:00:00");
        assert!(
            serde_json::from_value::<CanonicalSourceArtifactRetention>(malformed_time).is_err()
        );
        let mut malformed_digest = serde_json::to_value(&persisted_record()).expect("json");
        malformed_digest["retention_sha256"] = serde_json::json!("not-a-digest");
        assert!(
            serde_json::from_value::<CanonicalSourceArtifactRetention>(malformed_digest).is_err()
        );
        let mut malformed_payload_digest =
            serde_json::to_value(&persisted_record()).expect("json");
        malformed_payload_digest["payload_reference"]["payload_sha256"] =
            serde_json::json!("XYZ");
        assert!(
            serde_json::from_value::<CanonicalSourceArtifactRetention>(malformed_payload_digest)
                .is_err()
        );
        let mut unsupported_version = serde_json::to_value(&persisted_record()).expect("json");
        unsupported_version["schema_version"] = serde_json::json!(2);
        assert!(
            serde_json::from_value::<CanonicalSourceArtifactRetention>(unsupported_version)
                .is_err()
        );
    }

    #[test]
    fn until_expiration_requires_expires_at_strictly_later_than_assigned_at() {
        let mut record = persisted_record();
        record.retention_class = CanonicalSourceArtifactRetentionClass::UntilExpiration;
        record.expires_at = Some(record.assigned_at.clone());
        assert!(record.validate().is_err());
        record.expires_at = Some("2026-08-25T11:59:59Z".to_string());
        assert!(record.validate().is_err());
        record.expires_at = Some("2026-08-26T00:00:00Z".to_string());
        record.validate().expect("later expires_at must pass");
    }

    #[test]
    fn unknown_and_missing_fields_fail_closed() {
        let record = persisted_record();
        let mut value = serde_json::to_value(&record).expect("json");
        value["unknown_field"] = serde_json::json!(true);
        assert!(serde_json::from_value::<CanonicalSourceArtifactRetention>(value).is_err());
        let mut missing = serde_json::to_value(&persisted_record()).expect("json");
        missing.as_object_mut().unwrap().remove("task_id");
        assert!(serde_json::from_value::<CanonicalSourceArtifactRetention>(missing).is_err());
    }

    #[test]
    fn required_optional_fields_must_be_present_as_explicit_null() {
        let mut value = serde_json::to_value(&ephemeral_memory_only_record()).expect("json");
        value.as_object_mut().unwrap().remove("expires_at");
        assert!(serde_json::from_value::<CanonicalSourceArtifactRetention>(value).is_err());
    }

    #[test]
    fn lifecycle_states_and_persistence_states_serialize_snake_case() {
        for (lifecycle, persistence, expected_lifecycle, expected_persistence) in [
            (
                CanonicalSourceArtifactRetentionLifecycleState::Released,
                CanonicalSourceArtifactPersistenceState::Released,
                "released",
                "released",
            ),
            (
                CanonicalSourceArtifactRetentionLifecycleState::Quarantined,
                CanonicalSourceArtifactPersistenceState::Persisted,
                "quarantined",
                "persisted",
            ),
        ] {
            let mut record = persisted_record();
            record.lifecycle_state = lifecycle;
            record.persistence_state = persistence;
            if matches!(persistence, CanonicalSourceArtifactPersistenceState::Released) {
                record.payload_reference = None;
            }
            record.validate().expect("state pair must validate");
            let json = serde_json::to_string(&record).expect("serialize");
            assert!(json.contains(&format!("\"lifecycle_state\":\"{expected_lifecycle}\"")));
            assert!(json.contains(&format!("\"persistence_state\":\"{expected_persistence}\"")));
        }
    }
}
