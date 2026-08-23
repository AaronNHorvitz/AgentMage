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
    /// Visible reason for a non-complete disposition.
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
